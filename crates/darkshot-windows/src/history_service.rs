//! Dịch vụ ghi lịch sử ảnh chụp: ánh xạ kết quả hành động thành mục lịch sử và ghi trên luồng nền.
//!
//! Lịch sử không bao giờ làm hỏng hành động chụp: mọi lỗi và panic trên đường ghi bị nuốt và
//! chỉ tăng bộ đếm trong bộ nhớ (không ghi nhật ký). Tắt thì không làm gì cả.

use crate::paths;
use darkshot_core::actions::Outcome;
use darkshot_core::config::Expiry;
use darkshot_core::history::{expires_on, Kind};
use darkshot_core::history_store::{NewEntry, Store};
use darkshot_core::image::Image;
use darkshot_core::naming::LocalTime;
use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering};
use std::thread::JoinHandle;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_APP};

/// Gửi tới cửa sổ Lịch sử (nếu đang mở) sau khi một mục được ghi xong; Task 56 xử lý.
pub const WM_HISTORY_CHANGED: u32 = WM_APP + 6;

static ENABLED: AtomicBool = AtomicBool::new(false);
static FAILURES: AtomicU32 = AtomicU32::new(0);
static WINDOW: AtomicIsize = AtomicIsize::new(0);

/// Bật/tắt lịch sử cho cả tiến trình (lúc khởi động và sau khi Tùy chọn lưu xong).
pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::SeqCst);
}

pub fn is_enabled() -> bool {
    ENABLED.load(Ordering::SeqCst)
}

/// HWND của cửa sổ Lịch sử đang mở (0 khi không có); cửa sổ tự đặt và tự xóa.
/// Cửa sổ thật có từ Task 55/56; tới lúc đó chưa ai gọi hàm này.
#[allow(dead_code)]
pub fn set_window(hwnd: isize) {
    WINDOW.store(hwnd, Ordering::SeqCst);
}

/// Số lần ghi lịch sử thất bại trong tiến trình này (cho kiểm thử, không ghi nhật ký).
#[allow(dead_code)]
pub fn failure_count() -> u32 {
    FAILURES.load(Ordering::SeqCst)
}

fn fail() {
    FAILURES.fetch_add(1, Ordering::SeqCst);
}

/// Ánh xạ thuần: kết quả hành động → mục mới. `Failed`/`Cancelled` không ghi.
pub fn plan(
    outcome: &Outcome,
    expiry: Expiry,
    now: &LocalTime,
    unix_millis: u64,
) -> Option<NewEntry> {
    let base = |kind: Kind| NewEntry {
        kind,
        unix_millis,
        local: *now,
        path: None,
        url: None,
        expires: None,
    };
    match outcome {
        Outcome::Saved(path) => Some(NewEntry {
            path: Some(path.clone()),
            ..base(Kind::Saved)
        }),
        Outcome::Copied => Some(base(Kind::Copied)),
        Outcome::Uploaded(link) => Some(NewEntry {
            url: Some(link.clone()),
            expires: expires_on(now, expiry),
            ..base(Kind::Uploaded)
        }),
        Outcome::UploadFailedSaved { path, .. } => Some(NewEntry {
            path: Some(path.clone()),
            ..base(Kind::UploadFallback)
        }),
        Outcome::Failed | Outcome::Cancelled => None,
    }
}

/// Một lần Lưu thay cho lượt tải lên (lưu dự phòng): `Saved` thành `UploadFailedSaved`.
pub fn as_fallback(outcome: Outcome) -> Outcome {
    match outcome {
        Outcome::Saved(path) => Outcome::UploadFailedSaved {
            path,
            needs_settings: false,
        },
        other => other,
    }
}

fn unix_millis_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

fn notify_window() {
    let h = WINDOW.load(Ordering::SeqCst);
    if h == 0 {
        return;
    }
    // SAFETY: PostMessageW an toàn từ luồng bất kỳ; cửa sổ đã hủy thì lỗi và ta bỏ qua.
    unsafe {
        let _ = PostMessageW(
            Some(HWND(h as *mut c_void)),
            WM_HISTORY_CHANGED,
            WPARAM(0),
            LPARAM(0),
        );
    }
}

/// Ghi một mục (đồng bộ) và nuốt mọi lỗi/panic.
fn write_entry(root: PathBuf, new: NewEntry, img: &Image) {
    let r = catch_unwind(AssertUnwindSafe(|| Store::new(root).record(new, img)));
    match r {
        Ok(Ok(_)) => notify_window(),
        Ok(Err(_)) | Err(_) => fail(),
    }
}

/// Như `record_outcome` nhưng với thư mục gốc và cờ bật truyền vào; trả luồng đã tạo (nếu có).
fn record_with(
    enabled: bool,
    root: PathBuf,
    outcome: &Outcome,
    img: &Image,
    expiry: Expiry,
    now: LocalTime,
) -> Option<JoinHandle<()>> {
    if !enabled {
        return None;
    }
    let new = plan(outcome, expiry, &now, unix_millis_now())?;
    // Bản sao duy nhất của ảnh, giao cho luồng ghi.
    let img = img.clone();
    match std::thread::Builder::new()
        .name("darkshot-history".into())
        .spawn(move || write_entry(root, new, &img))
    {
        Ok(h) => Some(h),
        Err(_) => {
            fail();
            None
        }
    }
}

/// Như `record_outcome_blocking` nhưng với thư mục gốc và cờ bật truyền vào.
fn record_blocking_with(
    enabled: bool,
    root: PathBuf,
    outcome: &Outcome,
    img: &Image,
    expiry: Expiry,
    now: LocalTime,
) {
    if !enabled {
        return;
    }
    if let Some(new) = plan(outcome, expiry, &now, unix_millis_now()) {
        write_entry(root, new, img);
    }
}

/// Ghi kết quả vào lịch sử trên luồng nền `darkshot-history`; không chặn, không panic.
/// Tắt thì không làm gì (không sao ảnh, không tạo thư mục, không tạo luồng).
pub fn record_outcome(outcome: &Outcome, img: &Image, expiry: Expiry, now: LocalTime) {
    if !is_enabled() {
        return;
    }
    let _ = record_with(true, paths::history_dir(), outcome, img, expiry, now);
}

/// Như `record_outcome` nhưng ghi xong mới trả về (dùng khi thoát: lưu dự phòng).
pub fn record_outcome_blocking(outcome: &Outcome, img: &Image, expiry: Expiry, now: LocalTime) {
    if !is_enabled() {
        return;
    }
    record_blocking_with(true, paths::history_dir(), outcome, img, expiry, now);
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkshot_core::actions::Outcome;
    use darkshot_core::config::Expiry;
    use darkshot_core::history::Kind;
    use darkshot_core::history_store::Store;
    use darkshot_core::image::Image;
    use darkshot_core::naming::LocalTime;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn now() -> LocalTime {
        LocalTime {
            year: 2026,
            month: 11,
            day: 16,
            hour: 1,
            minute: 2,
            second: 3,
        }
    }

    fn img() -> Image {
        Image {
            width: 4,
            height: 3,
            bgra: vec![200; 4 * 3 * 4],
        }
    }

    static COUNTER: AtomicU32 = AtomicU32::new(0);
    fn temp_dir() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let d =
            std::env::temp_dir().join(format!("darkshot_histsvc_{}_{t}_{n}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn plan_saved() {
        let p = plan(
            &Outcome::Saved(PathBuf::from("C:/pics/a.png")),
            Expiry::D7,
            &now(),
            1234,
        )
        .unwrap();
        assert_eq!(p.kind, Kind::Saved);
        assert_eq!(p.path, Some(PathBuf::from("C:/pics/a.png")));
        assert_eq!(p.url, None);
        assert_eq!(p.expires, None);
        assert_eq!(p.unix_millis, 1234);
        assert_eq!(p.local, now());
    }

    #[test]
    fn plan_copied() {
        let p = plan(&Outcome::Copied, Expiry::D7, &now(), 5).unwrap();
        assert_eq!(p.kind, Kind::Copied);
        assert_eq!((p.path, p.url, p.expires), (None, None, None));
        assert_eq!(p.unix_millis, 5);
    }

    #[test]
    fn plan_uploaded_with_expiry() {
        for (e, want) in [
            (Expiry::D1, Some("2026-11-17")),
            (Expiry::D7, Some("2026-11-23")),
            (Expiry::D30, Some("2026-12-16")),
            (Expiry::Keep, None),
        ] {
            let p = plan(
                &Outcome::Uploaded("https://img.example.com/a.png".into()),
                e,
                &now(),
                9,
            )
            .unwrap();
            assert_eq!(p.kind, Kind::Uploaded);
            assert_eq!(p.url.as_deref(), Some("https://img.example.com/a.png"));
            assert_eq!(p.path, None);
            assert_eq!(p.expires.as_deref(), want, "{e:?}");
        }
    }

    #[test]
    fn plan_upload_fallback() {
        for needs_settings in [false, true] {
            let p = plan(
                &Outcome::UploadFailedSaved {
                    path: PathBuf::from("C:/pics/b.png"),
                    needs_settings,
                },
                Expiry::D30,
                &now(),
                7,
            )
            .unwrap();
            assert_eq!(p.kind, Kind::UploadFallback);
            assert_eq!(p.path, Some(PathBuf::from("C:/pics/b.png")));
            assert_eq!((p.url, p.expires), (None, None));
        }
    }

    #[test]
    fn plan_skips_failed_and_cancelled() {
        assert_eq!(plan(&Outcome::Failed, Expiry::D7, &now(), 1), None);
        assert_eq!(plan(&Outcome::Cancelled, Expiry::D7, &now(), 1), None);
    }

    #[test]
    fn as_fallback_turns_saved_into_upload_fallback() {
        let p = PathBuf::from("C:/x.png");
        assert_eq!(
            as_fallback(Outcome::Saved(p.clone())),
            Outcome::UploadFailedSaved {
                path: p,
                needs_settings: false
            }
        );
        assert_eq!(as_fallback(Outcome::Failed), Outcome::Failed);
        assert_eq!(as_fallback(Outcome::Copied), Outcome::Copied);
    }

    #[test]
    fn disabled_does_nothing() {
        let root = temp_dir().join("history");
        let h = record_with(
            false,
            root.clone(),
            &Outcome::Copied,
            &img(),
            Expiry::D7,
            now(),
        );
        assert!(h.is_none());
        assert!(!root.exists());
        record_blocking_with(
            false,
            root.clone(),
            &Outcome::Copied,
            &img(),
            Expiry::D7,
            now(),
        );
        assert!(!root.exists());
    }

    #[test]
    fn useless_outcome_spawns_nothing() {
        let root = temp_dir().join("history");
        assert!(record_with(
            true,
            root.clone(),
            &Outcome::Failed,
            &img(),
            Expiry::D7,
            now()
        )
        .is_none());
        assert!(!root.exists());
    }

    #[test]
    fn enabled_records_on_background_thread() {
        let root = temp_dir().join("history");
        let h = record_with(
            true,
            root.clone(),
            &Outcome::Copied,
            &img(),
            Expiry::D7,
            now(),
        )
        .expect("thread");
        assert_eq!(h.thread().name(), Some("darkshot-history"));
        h.join().unwrap();
        let l = Store::new(root).load();
        assert_eq!(l.entries.len(), 1);
        assert_eq!(l.entries[0].kind, Kind::Copied);
        assert_eq!((l.entries[0].width, l.entries[0].height), (4, 3));
    }

    #[test]
    fn blocking_records_before_returning() {
        let root = temp_dir().join("history");
        let saved = temp_dir().join("shot.png");
        std::fs::write(&saved, b"png").unwrap();
        record_blocking_with(
            true,
            root.clone(),
            &Outcome::UploadFailedSaved {
                path: saved.clone(),
                needs_settings: false,
            },
            &img(),
            Expiry::D7,
            now(),
        );
        let l = Store::new(root).load();
        assert_eq!(l.entries.len(), 1);
        assert_eq!(l.entries[0].kind, Kind::UploadFallback);
        assert_eq!(l.entries[0].path.as_deref(), Some(saved.as_path()));
        // Tệp của người dùng còn nguyên.
        assert_eq!(std::fs::read(&saved).unwrap(), b"png");
    }

    #[test]
    fn unwritable_root_counts_failure_without_panic() {
        // Gốc là một TỆP: không tạo được thư mục con.
        let root = temp_dir().join("not_a_dir");
        std::fs::write(&root, b"x").unwrap();
        let before = failure_count();
        let h = record_with(
            true,
            root.clone(),
            &Outcome::Copied,
            &img(),
            Expiry::D7,
            now(),
        )
        .expect("thread");
        assert!(h.join().is_ok(), "luồng ghi không được panic");
        assert!(failure_count() > before);
        let before = failure_count();
        record_blocking_with(
            true,
            root.clone(),
            &Outcome::Copied,
            &img(),
            Expiry::D7,
            now(),
        );
        assert!(failure_count() > before);
        assert_eq!(std::fs::read(&root).unwrap(), b"x");
    }

    #[test]
    fn invalid_image_counts_failure_without_panic() {
        let root = temp_dir().join("history");
        let bad = Image {
            width: 5,
            height: 5,
            bgra: vec![0; 3],
        };
        let before = failure_count();
        let h = record_with(
            true,
            root.clone(),
            &Outcome::Copied,
            &bad,
            Expiry::D7,
            now(),
        )
        .expect("thread");
        assert!(h.join().is_ok());
        assert!(failure_count() > before);
    }

    #[test]
    fn concurrent_uploads_make_two_entries() {
        let root = temp_dir().join("history");
        let a = record_with(
            true,
            root.clone(),
            &Outcome::Uploaded("https://x/a.png".into()),
            &img(),
            Expiry::D30,
            now(),
        )
        .unwrap();
        let b = record_with(
            true,
            root.clone(),
            &Outcome::Uploaded("https://x/b.png".into()),
            &img(),
            Expiry::D30,
            now(),
        )
        .unwrap();
        a.join().unwrap();
        b.join().unwrap();
        let l = Store::new(root).load();
        assert_eq!(l.entries.len(), 2);
        let mut urls: Vec<_> = l.entries.iter().filter_map(|e| e.url.clone()).collect();
        urls.sort();
        assert_eq!(urls, vec!["https://x/a.png", "https://x/b.png"]);
        assert!(l
            .entries
            .iter()
            .all(|e| e.expires.as_deref() == Some("2026-12-16")));
        assert_ne!(l.entries[0].id, l.entries[1].id);
    }

    #[test]
    fn enabled_flag_round_trip() {
        // Chỉ kiểm thử này đổi cờ toàn tiến trình.
        let old = is_enabled();
        set_enabled(true);
        assert!(is_enabled());
        set_enabled(false);
        assert!(!is_enabled());
        set_enabled(old);
    }

    #[test]
    fn message_ids() {
        use windows::Win32::UI::WindowsAndMessaging::WM_APP;
        assert_eq!(WM_HISTORY_CHANGED, WM_APP + 6);
    }
}
