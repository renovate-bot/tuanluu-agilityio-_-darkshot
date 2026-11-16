//! Chuyển dữ liệu của bản cũ (shotme) sang darkshot lúc khởi động: cấu hình, khóa Run, WER.
//!
//! Tên cũ chỉ được nằm ở file này và `darkshot-core/src/migrate.rs`. Không bao giờ xóa hay sửa
//! gì trong thư mục cũ, không bao giờ ghi đè cấu hình darkshot đã có.

use crate::{autostart, paths};
use darkshot_core::migrate::{
    migrated_config, plan_run, should_copy_config, RunAction, LEGACY_APP_DIR, LEGACY_EXE,
    LEGACY_RUN_VALUE,
};
use std::ffi::OsStr;
use std::io::Write;
use std::path::{Path, PathBuf};
use windows::core::HSTRING;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_WRITE_THROUGH};
use windows::Win32::System::ErrorReporting::WerRemoveExcludedApplication;
use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_ANY};

/// Khóa chứa danh sách loại trừ của WER cho người dùng hiện tại (`bAllUsers = FALSE`).
const WER_EXCLUDED_KEY: &str =
    r"Software\Microsoft\Windows\Windows Error Reporting\ExcludedApplications";

/// Bước 1–2 của thiết kế mục 6 với thư mục gốc truyền vào (để kiểm thử bằng thư mục tạm).
/// Ok(true) = đã chép; Ok(false) = không cần chép; Err = có bước lỗi (đã bỏ qua).
pub fn migrate_config(appdata: &Path, pictures: &Path) -> Result<bool, String> {
    let new_path = paths::config_path_in(appdata);
    let old_path = appdata.join(LEGACY_APP_DIR).join("config.json");
    // Không chắc file mới có hay không thì coi là lỗi, không chép (không bao giờ ghi đè).
    let new_exists = new_path.try_exists().map_err(|e| e.to_string())?;
    let old_exists = old_path.is_file();
    if !should_copy_config(new_exists, old_exists) {
        return Ok(false);
    }
    let old = std::fs::read(&old_path).map_err(|e| e.to_string())?;
    let data = migrated_config(&old, &pictures.join(LEGACY_APP_DIR))
        .map_err(|e| format!("legacy config: {e:?}"))?;
    write_new_file(&new_path, &data)?;
    Ok(true)
}

/// Ghi `data` thành `path` như `config::save` (file tạm cạnh đích, `sync_all`, đổi tên), nhưng
/// không bao giờ đè lên file đích đã có: đổi tên không kèm `MOVEFILE_REPLACE_EXISTING`.
fn write_new_file(path: &Path, data: &[u8]) -> Result<(), String> {
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or("no parent directory")?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut tmp_name = path.file_name().unwrap_or_default().to_os_string();
    tmp_name.push(".migrate.tmp");
    let tmp: PathBuf = dir.join(tmp_name);
    let result = write_tmp(&tmp, data).and_then(|()| {
        // Kiểm lại ngay trước khi đổi tên; MoveFileExW cũng tự thất bại nếu đích đã có.
        if path.try_exists().map_err(|e| e.to_string())? {
            return Err("target already exists".to_string());
        }
        // SAFETY: hai HSTRING kết thúc bằng 0 và sống suốt lời gọi.
        unsafe {
            MoveFileExW(
                &HSTRING::from(tmp.as_os_str()),
                &HSTRING::from(path.as_os_str()),
                MOVEFILE_WRITE_THROUGH,
            )
        }
        .map_err(|e| e.to_string())
    });
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

fn write_tmp(tmp: &Path, data: &[u8]) -> Result<(), String> {
    let mut f = std::fs::File::create(tmp).map_err(|e| e.to_string())?;
    f.write_all(data).map_err(|e| e.to_string())?;
    f.sync_all().map_err(|e| e.to_string())
}

/// Đúng khi danh sách loại trừ WER của người dùng hiện tại còn exe cũ.
fn legacy_wer_excluded() -> bool {
    let mut size = 0u32;
    // SAFETY: chỉ hỏi kích thước giá trị, không có bộ đệm; RegGetValueW tự mở và đóng khóa con.
    let r = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            &HSTRING::from(WER_EXCLUDED_KEY),
            &HSTRING::from(LEGACY_EXE),
            RRF_RT_ANY,
            None,
            None,
            Some(&mut size),
        )
    };
    r == ERROR_SUCCESS
}

/// Bỏ exe cũ khỏi danh sách loại trừ WER. Tài liệu không nêu mã lỗi khi exe không có trong
/// danh sách, nên chỉ tính là lỗi khi gọi thất bại mà giá trị vẫn còn.
fn remove_legacy_wer() -> Result<(), String> {
    // SAFETY: HSTRING kết thúc bằng 0 và sống suốt lời gọi; hàm không giữ con trỏ lại.
    let r = unsafe { WerRemoveExcludedApplication(&HSTRING::from(LEGACY_EXE), false) };
    match r {
        Err(e) if legacy_wer_excluded() => Err(e.to_string()),
        _ => Ok(()),
    }
}

/// Đúng khi tên file exe là tên exe cũ (không phân biệt hoa thường): khi đó loại trừ WER đang
/// thuộc về chính tiến trình này, không gỡ.
fn is_legacy_exe(file_name: &OsStr) -> bool {
    file_name
        .to_str()
        .is_some_and(|n| n.eq_ignore_ascii_case(LEGACY_EXE))
}

/// Gọi lúc khởi động, trước app::startup: migrate_config với thư mục thật, rồi
/// WerRemoveExcludedApplication(LEGACY_EXE, FALSE) (lỗi "không có" không tính là lỗi).
pub fn run_before_startup() -> bool {
    let config_failed = migrate_config(&paths::appdata_dir(), &paths::pictures_dir()).is_err();
    let exe = std::env::current_exe().ok();
    let running_as_legacy = exe
        .as_deref()
        .and_then(Path::file_name)
        .is_some_and(is_legacy_exe);
    let wer_failed = !running_as_legacy && remove_legacy_wer().is_err();
    config_failed || wer_failed
}

/// Gọi sau app::startup: đọc hai giá trị Run, plan_run(..., config.autostart), áp dụng.
pub fn sync_autostart(autostart: bool) -> bool {
    let plan = plan_run(
        autostart::value_exists(LEGACY_RUN_VALUE),
        autostart::is_enabled(),
        autostart,
    );
    let mut failed = false;
    if plan.delete_legacy {
        failed |= autostart::delete_value(LEGACY_RUN_VALUE).is_err();
    }
    failed |= match plan.darkshot {
        RunAction::Write => autostart::set_enabled(true).is_err(),
        RunAction::Delete => autostart::set_enabled(false).is_err(),
        RunAction::Nothing => false,
    };
    failed
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
    fn legacy_exe_name_ignores_case() {
        assert!(is_legacy_exe(OsStr::new(LEGACY_EXE)));
        assert!(is_legacy_exe(OsStr::new(&LEGACY_EXE.to_uppercase())));
        assert!(!is_legacy_exe(OsStr::new("darkshot.exe")));
        assert!(!is_legacy_exe(OsStr::new("")));
    }

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
