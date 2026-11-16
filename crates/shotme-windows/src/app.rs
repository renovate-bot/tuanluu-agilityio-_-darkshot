//! Trạng thái ứng dụng và luồng tải lên. Phần lớn là hàm thuần để kiểm thử được.

use shotme_core::actions::{run_action, Action, Clipboard, Ctx, Notifier};
use shotme_core::config::{Config, ConfigError, SecretStore};
use shotme_core::image::Image;
use shotme_core::naming::LocalTime;
use shotme_core::upload::Uploader;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::JoinHandle;

pub const NOTICE_CORRUPT: &str =
    "File cấu hình bị hỏng. Cài đặt mặc định đang được dùng; bấm Lưu để ghi lại.";
pub const NOTICE_SECRET_LOST: &str =
    "Không đọc được khóa bí mật đã lưu. Hãy nhập lại Secret Access Key.";
pub const NOTICE_UNREADABLE: &str =
    "Không đọc được file cấu hình. Cài đặt mặc định đang được dùng; bấm Lưu để ghi lại.";

pub struct AppState {
    pub config: Config,
    pub config_notice: Option<String>,
    pub overlay_open: bool,
}

/// Đọc cấu hình lúc khởi động. Không bao giờ ghi file.
pub fn startup(path: &Path, store: &dyn SecretStore, default_save_dir: &Path) -> AppState {
    let _ = (
        path,
        store,
        default_save_dir,
        ConfigError::Io(String::new()),
    );
    todo!()
}

pub fn needs_settings_on_start(s: &AppState, config_file_exists: bool) -> bool {
    let _ = (s, config_file_exists);
    todo!()
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
}

/// Chạy một lượt tải lên trên luồng riêng. Lỗi tạo luồng trả về `Err` (không panic).
pub fn spawn_upload(job: UploadJob) -> Result<JoinHandle<()>, String> {
    let _ = (job, run_action as fn(&Ctx, Action, &Image) -> _);
    todo!()
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
        let mut c = Config::defaults(Path::new("C:/pics"));
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
        let s = startup(&missing, &XorStore, Path::new("C:/pics"));
        assert!(needs_settings_on_start(&s, false) && s.config_notice.is_none());
        assert!(!missing.exists());
    }

    #[test]
    fn corrupt_config_uses_defaults_and_does_not_overwrite() {
        let path = temp_dir().join("config.json");
        std::fs::write(&path, "{not json").unwrap();
        let s = startup(&path, &XorStore, Path::new("C:/pics"));
        assert_eq!(s.config, Config::defaults(Path::new("C:/pics")));
        assert_eq!(s.config_notice.as_deref(), Some(NOTICE_CORRUPT));
        assert!(needs_settings_on_start(&s, true));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{not json");
    }

    #[test]
    fn lost_secret_opens_settings_with_notice() {
        let path = temp_dir().join("config.json");
        shotme_core::config::save(&path, &complete_config(), &XorStore).unwrap();
        let s = startup(&path, &FailStore, Path::new("C:/pics"));
        assert!(s
            .config_notice
            .as_deref()
            .unwrap()
            .contains("Secret Access Key"));
        assert!(needs_settings_on_start(&s, true));
        assert!(s.config.r2.secret_access_key.is_empty());
    }

    #[test]
    fn complete_config_starts_quietly() {
        let path = temp_dir().join("config.json");
        shotme_core::config::save(&path, &complete_config(), &XorStore).unwrap();
        let s = startup(&path, &XorStore, Path::new("C:/pics"));
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
        fn upload(&self, _key: &str, _png: &[u8]) -> Result<String, UploadError> {
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
        }
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
