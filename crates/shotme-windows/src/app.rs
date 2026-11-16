//! Trạng thái ứng dụng và luồng tải lên. Phần lớn là hàm thuần để kiểm thử được.

use crate::lang;
use shotme_core::actions::{run_action, Action, Clipboard, Ctx, Notifier, Outcome};
use shotme_core::config::{Config, ConfigError, SecretStore, UploadFormat, JPEG_QUALITY_DEFAULT};
use shotme_core::i18n::{t, Lang, Msg};
use shotme_core::image::Image;
use shotme_core::naming::LocalTime;
use shotme_core::upload::Uploader;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigNotice {
    Corrupt,
    SecretLost,
    Unreadable,
}

impl ConfigNotice {
    pub fn message(self, lang: Lang) -> String {
        let m = match self {
            ConfigNotice::Corrupt => Msg::NoticeConfigCorrupt,
            ConfigNotice::SecretLost => Msg::NoticeSecretLost,
            ConfigNotice::Unreadable => Msg::NoticeConfigUnreadable,
        };
        t(lang, m).to_string()
    }
}

pub struct AppState {
    pub config: Config,
    pub config_notice: Option<ConfigNotice>,
    pub overlay_open: bool,
}

/// Đọc cấu hình lúc khởi động. Không bao giờ ghi file.
pub fn startup(
    path: &Path,
    store: &dyn SecretStore,
    default_save_dir: &Path,
    os_lang: Lang,
) -> AppState {
    let defaults = || Config::defaults(default_save_dir, os_lang);
    let (config, config_notice) =
        match shotme_core::config::load(path, store, default_save_dir, os_lang) {
            Ok(l) if l.secret_lost => (l.config, Some(ConfigNotice::SecretLost)),
            Ok(l) => (l.config, None),
            Err(ConfigError::Corrupt(_)) => (defaults(), Some(ConfigNotice::Corrupt)),
            Err(_) => (defaults(), Some(ConfigNotice::Unreadable)),
        };
    AppState {
        config,
        config_notice,
        overlay_open: false,
    }
}

pub fn needs_settings_on_start(s: &AppState, config_file_exists: bool) -> bool {
    !config_file_exists || s.config_notice.is_some()
}

/// Một lượt tải lên chưa xong: ảnh và thư mục lưu của chính lượt đó.
pub struct PendingUpload {
    pub image: Arc<Image>,
    pub save_dir: PathBuf,
}

/// Sổ các lượt tải lên đang chạy, để lưu ảnh vào máy nếu ứng dụng thoát giữa chừng.
/// Dùng được từ nhiều luồng; `Mutex` bị poison vẫn dùng tiếp, không panic.
pub struct PendingUploads {
    inner: Mutex<PendingInner>,
}

struct PendingInner {
    next_id: u64,
    items: Vec<(u64, PendingUpload)>,
}

impl Default for PendingUploads {
    fn default() -> Self {
        Self::new()
    }
}

impl PendingUploads {
    pub const fn new() -> Self {
        PendingUploads {
            inner: Mutex::new(PendingInner {
                next_id: 0,
                items: Vec::new(),
            }),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, PendingInner> {
        // Sổ chỉ là danh sách; một luồng panic giữa chừng không làm nó sai, nên dùng tiếp.
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Ghi một lượt vào sổ; trả mã để rút ra sau.
    pub fn add(&self, image: Arc<Image>, save_dir: PathBuf) -> u64 {
        let mut g = self.lock();
        let id = g.next_id;
        g.next_id = g.next_id.wrapping_add(1);
        g.items.push((id, PendingUpload { image, save_dir }));
        id
    }

    /// Rút một lượt khỏi sổ; `false` nếu nó không còn trong sổ.
    pub fn remove(&self, id: u64) -> bool {
        let mut g = self.lock();
        match g.items.iter().position(|(i, _)| *i == id) {
            Some(pos) => {
                g.items.remove(pos);
                true
            }
            None => false,
        }
    }

    /// Lấy ra mọi lượt còn trong sổ (theo thứ tự thêm vào), sổ trở thành rỗng.
    pub fn drain(&self) -> Vec<PendingUpload> {
        std::mem::take(&mut self.lock().items)
            .into_iter()
            .map(|(_, p)| p)
            .collect()
    }
}

/// Bỏ qua mọi thông báo: lúc thoát hoặc tắt máy không còn ai để báo.
struct SilentNotifier;
impl Notifier for SilentNotifier {
    fn notify(&self, _: shotme_core::actions::NoticeKind, _: &str, _: &str) {}
}

/// Không đụng clipboard của người dùng khi lưu lúc thoát.
struct NoClipboard;
impl Clipboard for NoClipboard {
    fn set_text(&self, _: &str) -> Result<(), String> {
        Err(lang::tr(Msg::ClipboardUnusedOnExit).into())
    }
    fn set_image(&self, _: &Image) -> Result<(), String> {
        Err(lang::tr(Msg::ClipboardUnusedOnExit).into())
    }
}

/// Lưu PNG của các lượt chưa xong vào thư mục lưu của từng lượt. Chỉ ghi file, không thông báo,
/// không chạm clipboard, không mạng. Trả số ảnh đã lưu được.
pub fn save_pending(items: Vec<PendingUpload>, now: LocalTime) -> usize {
    items
        .iter()
        .filter(|p| {
            let ctx = Ctx {
                save_dir: &p.save_dir,
                expiry: shotme_core::config::Expiry::Keep,
                now,
                uploader: None,
                clipboard: &NoClipboard,
                notifier: &SilentNotifier,
                lang: lang::current(),
                upload_format: UploadFormat::Png,
                jpeg_quality: JPEG_QUALITY_DEFAULT,
            };
            matches!(run_action(&ctx, Action::Save, &p.image), Outcome::Saved(_))
        })
        .count()
}

/// Mọi thứ một lượt tải lên cần; đã là bản chụp riêng, không tham chiếu trạng thái luồng chính.
pub struct UploadJob {
    pub save_dir: PathBuf,
    pub expiry: shotme_core::config::Expiry,
    pub uploader: Option<Arc<dyn Uploader>>,
    pub clipboard: Arc<dyn Clipboard>,
    pub notifier: Arc<dyn Notifier>,
    pub image: Arc<Image>,
    pub clock: fn() -> LocalTime,
    /// Gọi khi `UploadFailedSaved { needs_settings: true }`.
    pub on_needs_settings: Box<dyn FnOnce() + Send>,
    /// Sổ và mã của lượt này; luồng tải lên rút nó khỏi sổ khi kết thúc.
    pub pending: Option<(Arc<PendingUploads>, u64)>,
    /// Ngôn ngữ của thông báo, chốt lúc tạo lượt.
    pub lang: Lang,
    pub upload_format: UploadFormat,
    pub jpeg_quality: u8,
}

/// Chạy một lượt tải lên trên luồng riêng. Lỗi tạo luồng trả về `Err` (không panic).
pub fn spawn_upload(job: UploadJob) -> Result<JoinHandle<()>, String> {
    std::thread::Builder::new()
        .name("shotme-upload".into())
        .spawn(move || {
            let ctx = Ctx {
                save_dir: &job.save_dir,
                expiry: job.expiry,
                now: (job.clock)(),
                uploader: job.uploader.as_deref(),
                clipboard: job.clipboard.as_ref(),
                notifier: job.notifier.as_ref(),
                lang: job.lang,
                upload_format: job.upload_format,
                jpeg_quality: job.jpeg_quality,
            };
            let outcome = run_action(&ctx, Action::Upload, &job.image);
            // Lượt đã kết thúc (lên kho, đã lưu dự phòng, hoặc đã báo lỗi): rút khỏi sổ.
            if let Some((reg, id)) = &job.pending {
                reg.remove(*id);
            }
            if let Outcome::UploadFailedSaved {
                needs_settings: true,
                ..
            } = outcome
            {
                (job.on_needs_settings)();
            }
        })
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use shotme_core::actions::NoticeKind;
    use shotme_core::config::{Expiry, R2Config};
    use shotme_core::upload::UploadError;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::sync::Mutex;
    use std::time::Duration;

    struct XorStore;
    impl SecretStore for XorStore {
        fn protect(&self, plain: &[u8]) -> Result<Vec<u8>, String> {
            Ok(plain.iter().map(|b| b ^ 0x5A).collect())
        }
        fn unprotect(&self, cipher: &[u8]) -> Result<Vec<u8>, String> {
            Ok(cipher.iter().map(|b| b ^ 0x5A).collect())
        }
    }
    struct FailStore;
    impl SecretStore for FailStore {
        fn protect(&self, _: &[u8]) -> Result<Vec<u8>, String> {
            Err("fail".into())
        }
        fn unprotect(&self, _: &[u8]) -> Result<Vec<u8>, String> {
            Err("fail".into())
        }
    }

    static COUNTER: AtomicU32 = AtomicU32::new(0);
    fn temp_dir() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let d = std::env::temp_dir().join(format!("shotme_app_{}_{t}_{n}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn complete_config() -> Config {
        let mut c = Config::defaults(Path::new("C:/pics"), Lang::Vi);
        c.r2 = R2Config {
            account_id: "acc123".into(),
            access_key_id: "AKIDEXAMPLE".into(),
            secret_access_key: "TOP-SECRET-VALUE".into(),
            bucket: "bucket".into(),
            public_base_url: "https://img.example.com".into(),
        };
        c
    }

    #[test]
    fn first_run_opens_settings() {
        let missing = temp_dir().join("config.json");
        let s = startup(&missing, &XorStore, Path::new("C:/pics"), Lang::Vi);
        assert!(needs_settings_on_start(&s, false) && s.config_notice.is_none());
        assert!(!missing.exists());
    }

    #[test]
    fn corrupt_config_uses_defaults_and_does_not_overwrite() {
        let path = temp_dir().join("config.json");
        std::fs::write(&path, "{not json").unwrap();
        let s = startup(&path, &XorStore, Path::new("C:/pics"), Lang::Vi);
        assert_eq!(s.config, Config::defaults(Path::new("C:/pics"), Lang::Vi));
        assert_eq!(s.config_notice, Some(ConfigNotice::Corrupt));
        assert!(needs_settings_on_start(&s, true));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{not json");
    }

    #[test]
    fn lost_secret_opens_settings_with_notice() {
        let path = temp_dir().join("config.json");
        shotme_core::config::save(&path, &complete_config(), &XorStore).unwrap();
        let s = startup(&path, &FailStore, Path::new("C:/pics"), Lang::Vi);
        assert_eq!(s.config_notice, Some(ConfigNotice::SecretLost));
        assert!(s
            .config_notice
            .unwrap()
            .message(Lang::Vi)
            .contains("Secret Access Key"));
        assert!(needs_settings_on_start(&s, true));
        assert!(s.config.r2.secret_access_key.is_empty());
    }

    #[test]
    fn corrupt_or_missing_config_uses_os_language() {
        let path = temp_dir().join("config.json");
        std::fs::write(&path, "{not json").unwrap();
        let s = startup(&path, &XorStore, Path::new("C:/pics"), Lang::En);
        assert_eq!(s.config.language, Lang::En);
        assert_eq!(s.config_notice, Some(ConfigNotice::Corrupt));

        let missing = temp_dir().join("config.json");
        let s = startup(&missing, &XorStore, Path::new("C:/pics"), Lang::En);
        assert_eq!(s.config.language, Lang::En);
        assert_eq!(s.config_notice, None);

        let path = temp_dir().join("config.json");
        std::fs::write(&path, "{}").unwrap();
        let s = startup(&path, &XorStore, Path::new("C:/pics"), Lang::En);
        assert_eq!(s.config.language, Lang::Vi);
        assert_eq!(s.config_notice, None);
    }

    #[test]
    fn config_notice_messages_differ_by_language() {
        for n in [
            ConfigNotice::Corrupt,
            ConfigNotice::SecretLost,
            ConfigNotice::Unreadable,
        ] {
            assert_ne!(n.message(Lang::Vi), n.message(Lang::En));
        }
    }

    #[test]
    fn complete_config_starts_quietly() {
        let path = temp_dir().join("config.json");
        shotme_core::config::save(&path, &complete_config(), &XorStore).unwrap();
        let s = startup(&path, &XorStore, Path::new("C:/pics"), Lang::Vi);
        assert_eq!(s.config, complete_config());
        assert!(!needs_settings_on_start(&s, true));
        assert!(!s.overlay_open);
    }

    #[derive(Default)]
    struct FakeNotifier {
        notes: Mutex<Vec<(NoticeKind, String, String)>>,
    }
    impl Notifier for FakeNotifier {
        fn notify(&self, kind: NoticeKind, title: &str, body: &str) {
            self.notes
                .lock()
                .unwrap()
                .push((kind, title.into(), body.into()));
        }
    }
    #[derive(Default)]
    struct FakeClipboard;
    impl Clipboard for FakeClipboard {
        fn set_text(&self, _: &str) -> Result<(), String> {
            Ok(())
        }
        fn set_image(&self, _: &Image) -> Result<(), String> {
            Ok(())
        }
    }
    struct SlowUploader(String);
    impl Uploader for SlowUploader {
        fn upload(&self, _key: &str, _body: &[u8], _ct: &str) -> Result<String, UploadError> {
            std::thread::sleep(Duration::from_millis(200));
            Ok(self.0.clone())
        }
    }
    fn fixed_now() -> LocalTime {
        LocalTime {
            year: 2026,
            month: 11,
            day: 16,
            hour: 1,
            minute: 2,
            second: 3,
        }
    }
    fn img() -> Arc<Image> {
        Arc::new(Image {
            width: 2,
            height: 2,
            bgra: vec![255; 16],
        })
    }
    fn job(
        dir: PathBuf,
        up: Option<Arc<dyn Uploader>>,
        n: Arc<FakeNotifier>,
        cb: Box<dyn FnOnce() + Send>,
    ) -> UploadJob {
        UploadJob {
            save_dir: dir,
            expiry: Expiry::D30,
            uploader: up,
            clipboard: Arc::new(FakeClipboard),
            notifier: n,
            image: img(),
            clock: fixed_now,
            on_needs_settings: cb,
            pending: None,
            lang: Lang::Vi,
            upload_format: UploadFormat::Png,
            jpeg_quality: JPEG_QUALITY_DEFAULT,
        }
    }

    fn pixel(v: u8) -> Arc<Image> {
        Arc::new(Image {
            width: 1,
            height: 1,
            bgra: vec![v, v, v, 255],
        })
    }

    #[test]
    fn pending_add_remove_drain() {
        let reg = PendingUploads::new();
        let (a, b, c) = (pixel(1), pixel(2), pixel(3));
        let ia = reg.add(a.clone(), PathBuf::from("C:/a"));
        let ib = reg.add(b.clone(), PathBuf::from("C:/b"));
        let ic = reg.add(c.clone(), PathBuf::from("C:/c"));
        assert!(ia != ib && ib != ic && ia != ic);
        assert!(reg.remove(ib));
        // Rút hai lần: lần sau báo không còn.
        assert!(!reg.remove(ib));
        let left = reg.drain();
        assert_eq!(left.len(), 2);
        assert!(Arc::ptr_eq(&left[0].image, &a) && left[0].save_dir == Path::new("C:/a"));
        assert!(Arc::ptr_eq(&left[1].image, &c) && left[1].save_dir == Path::new("C:/c"));
        // Sau drain sổ rỗng; rút một lượt đã bị drain không làm gì.
        assert!(reg.drain().is_empty());
        assert!(!reg.remove(ia));
        // Mã mới không trùng mã cũ.
        let id = reg.add(pixel(4), PathBuf::from("C:/d"));
        assert!(id != ia && id != ib && id != ic);
        assert_eq!(reg.drain().len(), 1);
    }

    #[test]
    fn pending_survives_poisoned_mutex() {
        let reg = Arc::new(PendingUploads::new());
        let id = reg.add(pixel(1), PathBuf::from("C:/a"));
        let r2 = reg.clone();
        let _ = std::thread::spawn(move || {
            let _g = r2.inner.lock().unwrap();
            panic!("poison");
        })
        .join();
        assert!(reg.inner.is_poisoned());
        let id2 = reg.add(pixel(2), PathBuf::from("C:/b"));
        assert!(reg.remove(id));
        assert!(!reg.remove(id));
        let left = reg.drain();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].save_dir, Path::new("C:/b"));
        assert!(!reg.remove(id2));
    }

    #[test]
    fn save_pending_writes_each_png_to_its_dir() {
        let (d1, d2) = (temp_dir(), temp_dir().join("sub"));
        let items = vec![
            PendingUpload {
                image: pixel(1),
                save_dir: d1.clone(),
            },
            PendingUpload {
                image: pixel(2),
                save_dir: d2.clone(),
            },
            PendingUpload {
                image: pixel(3),
                save_dir: d1.clone(),
            },
        ];
        assert_eq!(save_pending(items, fixed_now()), 3);
        assert!(d1.join("shotme_2026-11-16_01-02-03.png").exists());
        assert!(d1.join("shotme_2026-11-16_01-02-03_2.png").exists());
        assert!(d2.join("shotme_2026-11-16_01-02-03.png").exists());
        assert_eq!(std::fs::read_dir(&d1).unwrap().count(), 2);
    }

    #[test]
    fn finished_upload_leaves_the_registry() {
        let reg = Arc::new(PendingUploads::new());
        let other = reg.add(pixel(9), PathBuf::from("C:/other"));
        for up in [
            Some(Arc::new(SlowUploader("https://x/a".into())) as Arc<dyn Uploader>),
            None, // lỗi cấu hình: đã tự lưu dự phòng
        ] {
            let n = Arc::new(FakeNotifier::default());
            let mut j = job(temp_dir(), up, n, Box::new(|| {}));
            let id = reg.add(j.image.clone(), j.save_dir.clone());
            j.pending = Some((reg.clone(), id));
            spawn_upload(j).unwrap().join().unwrap();
            assert!(!reg.remove(id), "lượt đã xong phải tự rút khỏi sổ");
        }
        // Lượt không liên quan vẫn còn.
        assert!(reg.remove(other));
    }

    #[test]
    fn concurrent_uploads_each_notify() {
        let n = Arc::new(FakeNotifier::default());
        let dir = temp_dir();
        let mk = |link: &str| {
            let up: Arc<dyn Uploader> = Arc::new(SlowUploader(link.into()));
            job(dir.clone(), Some(up), n.clone(), Box::new(|| {}))
        };
        let h1 = spawn_upload(mk("https://x/a")).unwrap();
        let h2 = spawn_upload(mk("https://x/b")).unwrap();
        h1.join().unwrap();
        h2.join().unwrap();
        let notes = n.notes.lock().unwrap();
        assert_eq!(notes.len(), 2);
        assert!(notes.iter().all(|(_, t, _)| t == "Đã chép link"));
        let mut links: Vec<_> = notes.iter().map(|(_, _, b)| b.clone()).collect();
        links.sort();
        assert_eq!(links, vec!["https://x/a", "https://x/b"]);
    }

    #[test]
    fn unconfigured_upload_saves_and_requests_settings() {
        let n = Arc::new(FakeNotifier::default());
        let dir = temp_dir();
        let called = Arc::new(AtomicBool::new(false));
        let c2 = called.clone();
        let h = spawn_upload(job(
            dir.clone(),
            None,
            n.clone(),
            Box::new(move || c2.store(true, Ordering::SeqCst)),
        ))
        .unwrap();
        h.join().unwrap();
        assert!(called.load(Ordering::SeqCst));
        assert!(dir.join("shotme_2026-11-16_01-02-03.png").exists());
    }

    #[test]
    fn upload_success_does_not_request_settings() {
        let n = Arc::new(FakeNotifier::default());
        let called = Arc::new(AtomicBool::new(false));
        let c2 = called.clone();
        let up: Arc<dyn Uploader> = Arc::new(SlowUploader("https://x/a".into()));
        let h = spawn_upload(job(
            temp_dir(),
            Some(up),
            n,
            Box::new(move || c2.store(true, Ordering::SeqCst)),
        ))
        .unwrap();
        h.join().unwrap();
        assert!(!called.load(Ordering::SeqCst));
    }
}
