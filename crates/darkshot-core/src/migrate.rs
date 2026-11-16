//! Chuyển dữ liệu từ tên cũ của ứng dụng: quyết định chép cấu hình, `save_dir` nào, giá trị Run nào.
//!
//! Tên cũ chỉ được nằm trong file này và `darkshot-windows/src/legacy.rs`.

use serde_json::Value;
use std::path::Path;

/// Thư mục cũ: `%APPDATA%\shotme` và `Pictures\shotme`.
pub const LEGACY_APP_DIR: &str = "shotme";
/// Tên giá trị cũ trong khóa Run.
pub const LEGACY_RUN_VALUE: &str = "shotme";
/// Tên exe cũ (trong danh sách loại trừ của WER).
pub const LEGACY_EXE: &str = "shotme.exe";

#[derive(Debug, PartialEq, Eq)]
pub enum MigrateError {
    /// File cũ không phải một JSON object.
    Corrupt,
}

/// Chỉ chép khi chưa có cấu hình mới và có cấu hình cũ.
pub fn should_copy_config(new_exists: bool, old_exists: bool) -> bool {
    !new_exists && old_exists
}

/// Nội dung file mới: mọi trường của file cũ giữ nguyên giá trị; nếu thiếu `save_dir` (hoặc là null)
/// thì thêm `save_dir` = legacy_default_save_dir (chuỗi đường dẫn). Không thêm hay đổi trường nào khác.
pub fn migrated_config(
    old_json: &[u8],
    legacy_default_save_dir: &Path,
) -> Result<Vec<u8>, MigrateError> {
    let mut value: Value = serde_json::from_slice(old_json).map_err(|_| MigrateError::Corrupt)?;
    let obj = value.as_object_mut().ok_or(MigrateError::Corrupt)?;
    if obj.get("save_dir").is_some_and(|v| !v.is_null()) {
        // Không cần đổi gì: giữ nguyên từng byte của file cũ.
        return Ok(old_json.to_vec());
    }
    // Ghi đường dẫn như `config::save` (serde của PathBuf); đường dẫn không phải UTF-8 thì không chép.
    let dir = serde_json::to_value(legacy_default_save_dir).map_err(|_| MigrateError::Corrupt)?;
    obj.insert("save_dir".to_string(), dir);
    serde_json::to_vec_pretty(&value).map_err(|_| MigrateError::Corrupt)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunAction {
    Write,
    Delete,
    Nothing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunPlan {
    pub delete_legacy: bool,
    pub darkshot: RunAction,
}

/// delete_legacy = legacy_present; darkshot = Write nếu autostart (luôn ghi lại để cập nhật đường dẫn exe),
/// Delete nếu !autostart && darkshot_present, còn lại Nothing.
pub fn plan_run(legacy_present: bool, darkshot_present: bool, autostart: bool) -> RunPlan {
    let darkshot = if autostart {
        RunAction::Write
    } else if darkshot_present {
        RunAction::Delete
    } else {
        RunAction::Nothing
    };
    RunPlan {
        delete_legacy: legacy_present,
        darkshot,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        self, Config, Expiry, Hotkey, HotkeySlot, R2Config, SecretStore, UploadFormat,
    };
    use crate::i18n::Lang;
    use serde_json::Value;
    use std::path::PathBuf;

    fn temp_dir() -> PathBuf {
        let mut b = [0u8; 8];
        getrandom::fill(&mut b).unwrap();
        let name: String = b.iter().map(|x| format!("{x:02x}")).collect();
        let d = std::env::temp_dir().join(format!("darkshot_migrate_test_{name}"));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// DPAPI giả: XOR, đủ để biết khóa đi qua được mà không lộ chữ rõ.
    struct XorStore;
    impl SecretStore for XorStore {
        fn protect(&self, plain: &[u8]) -> Result<Vec<u8>, String> {
            Ok(plain.iter().map(|b| b ^ 0x5A).collect())
        }
        fn unprotect(&self, cipher: &[u8]) -> Result<Vec<u8>, String> {
            Ok(cipher.iter().map(|b| b ^ 0x5A).collect())
        }
    }

    /// Cấu hình có mọi trường khác mặc định.
    fn full_config() -> Config {
        let mut c = Config::defaults(Path::new("C:/old/pics"), Lang::Vi);
        c.r2 = R2Config {
            account_id: "acc".into(),
            access_key_id: "kid".into(),
            secret_access_key: "TOP-SECRET-VALUE".into(),
            bucket: "bucket".into(),
            public_base_url: "https://img.example.com".into(),
        };
        c.expiry = Expiry::D7;
        c.hotkeys.region = HotkeySlot {
            enabled: false,
            key: Hotkey {
                ctrl: true,
                alt: true,
                shift: false,
                win: false,
                vk: 0x70,
            },
        };
        c.hotkeys.save_full = HotkeySlot {
            enabled: true,
            key: Hotkey {
                win: true,
                vk: 0x71,
                ..Hotkey::default()
            },
        };
        c.hotkeys.upload_full = HotkeySlot {
            enabled: true,
            key: Hotkey {
                alt: true,
                vk: 0x72,
                ..Hotkey::default()
            },
        };
        c.save_dir = PathBuf::from(r"D:\My Shots");
        c.autostart = true;
        c.language = Lang::En;
        c.show_success_notices = false;
        c.keep_selection = true;
        c.include_cursor = true;
        c.upload_format = UploadFormat::Jpeg;
        c.jpeg_quality = 75;
        c
    }

    fn legacy_pics() -> PathBuf {
        PathBuf::from(r"C:\Users\u\Pictures").join(LEGACY_APP_DIR)
    }

    fn json(bytes: &[u8]) -> Value {
        serde_json::from_slice(bytes).unwrap()
    }

    #[test]
    fn copy_only_when_new_missing_and_old_present() {
        assert!(!should_copy_config(false, false));
        assert!(should_copy_config(false, true));
        assert!(!should_copy_config(true, false));
        assert!(!should_copy_config(true, true));
    }

    #[test]
    fn copied_config_keeps_every_field() {
        let path = temp_dir().join("config.json");
        config::save(&path, &full_config(), &XorStore).unwrap();
        let old = std::fs::read(&path).unwrap();
        let new = migrated_config(&old, &legacy_pics()).unwrap();
        let (old_v, new_v) = (json(&old), json(&new));
        assert!(old_v.as_object().unwrap().len() > 5);
        assert_eq!(new_v, old_v);
        assert_eq!(new_v["save_dir"], r"D:\My Shots");
        assert_eq!(new_v["autostart"], true);
        assert_eq!(new_v["upload_format"], "jpeg");
    }

    #[test]
    fn missing_save_dir_points_to_legacy_folder() {
        let want = legacy_pics().to_string_lossy().into_owned();

        let v =
            json(&migrated_config(br#"{"autostart":true,"expiry":"d7"}"#, &legacy_pics()).unwrap());
        assert_eq!(v["save_dir"], want.as_str());
        assert_eq!(v["autostart"], true);
        assert_eq!(v["expiry"], "d7");
        assert_eq!(v.as_object().unwrap().len(), 3);

        let v = json(
            &migrated_config(br#"{"save_dir":null,"autostart":false}"#, &legacy_pics()).unwrap(),
        );
        assert_eq!(v["save_dir"], want.as_str());
        assert_eq!(v["autostart"], false);
        assert_eq!(v.as_object().unwrap().len(), 2);

        let v = json(&migrated_config(br#"{"save_dir":"E:\\shots"}"#, &legacy_pics()).unwrap());
        assert_eq!(v["save_dir"], r"E:\shots");
        assert_eq!(v.as_object().unwrap().len(), 1);

        let v = json(&migrated_config(b"{}", &legacy_pics()).unwrap());
        assert_eq!(v["save_dir"], want.as_str());
    }

    #[test]
    fn copied_config_loads_with_secret() {
        let dir = temp_dir();
        let old_path = dir.join("old.json");
        let mut c = full_config();
        config::save(&old_path, &c, &XorStore).unwrap();
        let new = migrated_config(&std::fs::read(&old_path).unwrap(), &legacy_pics()).unwrap();
        let new_path = dir.join("config.json");
        std::fs::write(&new_path, new).unwrap();
        let l = config::load(&new_path, &XorStore, Path::new("C:/new/pics"), Lang::Vi).unwrap();
        assert!(!l.secret_lost);
        assert_eq!(l.config.r2.secret_access_key, "TOP-SECRET-VALUE");
        assert_eq!(l.config, c);

        // Không có save_dir: tải lên thành thư mục mặc định cũ, không phải mặc định mới.
        let text = r#"{"language":"vi"}"#;
        let new = migrated_config(text.as_bytes(), &legacy_pics()).unwrap();
        std::fs::write(&new_path, new).unwrap();
        let l = config::load(&new_path, &XorStore, Path::new("C:/new/pics"), Lang::Vi).unwrap();
        assert_eq!(l.config.save_dir, legacy_pics());
        c = Config::defaults(&legacy_pics(), Lang::Vi);
        assert_eq!(l.config, c);
    }

    #[test]
    fn corrupt_old_config_is_not_copied() {
        for bad in [
            &b"not json"[..],
            b"[]",
            b"",
            b"42",
            b"\"text\"",
            b"null",
            b"{",
        ] {
            assert_eq!(
                migrated_config(bad, &legacy_pics()),
                Err(MigrateError::Corrupt),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn plan_run_legacy_does_not_enable() {
        assert_eq!(
            plan_run(true, false, false),
            RunPlan {
                delete_legacy: true,
                darkshot: RunAction::Nothing
            }
        );
    }

    #[test]
    fn plan_run_all_combinations() {
        use RunAction::*;
        let cases = [
            ((false, false, false), (false, Nothing)),
            ((false, false, true), (false, Write)),
            ((false, true, false), (false, Delete)),
            ((false, true, true), (false, Write)),
            ((true, false, false), (true, Nothing)),
            ((true, false, true), (true, Write)),
            ((true, true, false), (true, Delete)),
            ((true, true, true), (true, Write)),
        ];
        for ((legacy, dark, auto), (del, action)) in cases {
            assert_eq!(
                plan_run(legacy, dark, auto),
                RunPlan {
                    delete_legacy: del,
                    darkshot: action
                },
                "{legacy} {dark} {auto}"
            );
        }
    }

    #[test]
    fn plan_run_never_writes_without_autostart() {
        for legacy in [false, true] {
            for dark in [false, true] {
                assert_ne!(plan_run(legacy, dark, false).darkshot, RunAction::Write);
                assert_eq!(plan_run(legacy, dark, true).darkshot, RunAction::Write);
            }
        }
    }
}
