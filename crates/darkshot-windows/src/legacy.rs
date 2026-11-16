//! Chuyển dữ liệu của bản cũ (shotme) sang darkshot lúc khởi động: cấu hình, khóa Run, WER.
//!
//! Tên cũ chỉ được nằm ở file này và `darkshot-core/src/migrate.rs`. Không bao giờ xóa hay sửa
//! gì trong thư mục cũ, không bao giờ ghi đè cấu hình darkshot đã có.

use std::path::Path;

/// Bước 1–2 của thiết kế mục 6 với thư mục gốc truyền vào (để kiểm thử bằng thư mục tạm).
/// Ok(true) = đã chép; Ok(false) = không cần chép; Err = có bước lỗi (đã bỏ qua).
pub fn migrate_config(appdata: &Path, pictures: &Path) -> Result<bool, String> {
    todo!("{} {}", appdata.display(), pictures.display())
}

/// Gọi lúc khởi động, trước app::startup: migrate_config với thư mục thật, rồi
/// WerRemoveExcludedApplication(LEGACY_EXE, FALSE) (lỗi "không có" không tính là lỗi).
pub fn run_before_startup() -> bool {
    todo!()
}

/// Gọi sau app::startup: đọc hai giá trị Run, plan_run(..., config.autostart), áp dụng.
pub fn sync_autostart(autostart: bool) -> bool {
    todo!("{autostart}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkshot_core::config::{self, SecretStore};
    use darkshot_core::i18n::Lang;
    use darkshot_core::migrate::LEGACY_APP_DIR;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    /// Thư mục tạm riêng cho từng kiểm thử, giả làm thư mục gốc `%APPDATA%` và `Pictures`.
    fn temp_root() -> (PathBuf, PathBuf) {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let d =
            std::env::temp_dir().join(format!("darkshot_legacy_{}_{t}_{n}", std::process::id()));
        let (appdata, pictures) = (d.join("appdata"), d.join("pictures"));
        std::fs::create_dir_all(&appdata).unwrap();
        std::fs::create_dir_all(&pictures).unwrap();
        (appdata, pictures)
    }

    /// DPAPI giả: trả nguyên byte, để đọc lại khóa trong file mẫu mà không đụng DPAPI thật.
    struct PlainStore;
    impl SecretStore for PlainStore {
        fn protect(&self, plain: &[u8]) -> Result<Vec<u8>, String> {
            Ok(plain.to_vec())
        }
        fn unprotect(&self, cipher: &[u8]) -> Result<Vec<u8>, String> {
            Ok(cipher.to_vec())
        }
    }

    fn write_old(appdata: &Path, bytes: &[u8]) -> PathBuf {
        let dir = appdata.join(LEGACY_APP_DIR);
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("config.json");
        std::fs::write(&p, bytes).unwrap();
        p
    }

    fn new_path(appdata: &Path) -> PathBuf {
        appdata.join("darkshot").join("config.json")
    }

    const OLD: &[u8] = br#"{
  "r2": {"account_id": "acc", "secret_access_key_protected": "AAEC"},
  "autostart": true,
  "language": "en"
}"#;

    #[test]
    fn migration_round_trip_in_temp_dir() {
        let (appdata, pictures) = temp_root();
        let old = write_old(&appdata, OLD);
        assert_eq!(migrate_config(&appdata, &pictures), Ok(true));
        assert_eq!(std::fs::read(&old).unwrap(), OLD);
        let l = config::load(
            &new_path(&appdata),
            &PlainStore,
            Path::new(r"C:\new\pics"),
            Lang::Vi,
        )
        .unwrap();
        assert!(!l.secret_lost);
        assert!(l.config.autostart);
        assert_eq!(l.config.language, Lang::En);
        assert_eq!(l.config.r2.account_id, "acc");
        assert_eq!(l.config.r2.secret_access_key, "\u{0}\u{1}\u{2}");
        assert_eq!(l.config.save_dir, pictures.join(LEGACY_APP_DIR));
        // Không để lại file tạm cạnh file mới.
        let names: Vec<_> = std::fs::read_dir(appdata.join("darkshot"))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["config.json".to_string()]);
        // Chạy lần nữa: đã có file mới thì không làm gì.
        assert_eq!(migrate_config(&appdata, &pictures), Ok(false));
        assert_eq!(std::fs::read(&old).unwrap(), OLD);
    }

    #[test]
    fn existing_new_config_is_never_overwritten() {
        let (appdata, pictures) = temp_root();
        let old = write_old(&appdata, OLD);
        let new = new_path(&appdata);
        std::fs::create_dir_all(new.parent().unwrap()).unwrap();
        let mine = br#"{"autostart":false}"#;
        std::fs::write(&new, mine).unwrap();
        assert_eq!(migrate_config(&appdata, &pictures), Ok(false));
        assert_eq!(std::fs::read(&new).unwrap(), mine);
        assert_eq!(std::fs::read(&old).unwrap(), OLD);
    }

    #[test]
    fn no_legacy_config_does_nothing() {
        let (appdata, pictures) = temp_root();
        assert_eq!(migrate_config(&appdata, &pictures), Ok(false));
        assert!(!appdata.join("darkshot").exists());
        // Thư mục cũ có mà không có file cấu hình: cũng không làm gì.
        std::fs::create_dir_all(appdata.join(LEGACY_APP_DIR)).unwrap();
        assert_eq!(migrate_config(&appdata, &pictures), Ok(false));
        assert!(!appdata.join("darkshot").exists());
    }

    #[test]
    fn corrupt_legacy_config_reports_error() {
        let (appdata, pictures) = temp_root();
        let old = write_old(&appdata, b"not json");
        assert!(migrate_config(&appdata, &pictures).is_err());
        assert!(!new_path(&appdata).exists());
        assert_eq!(std::fs::read(&old).unwrap(), b"not json");
    }
}
