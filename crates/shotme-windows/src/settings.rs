//! Cửa sổ Cài đặt.

use shotme_core::config::{normalize_public_base_url, Config, Expiry, Hotkey, R2Config};
use std::path::PathBuf;

/// Giá trị đọc từ các ô của cửa sổ.
#[derive(Clone)]
pub struct Form {
    pub account_id: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub bucket: String,
    pub public_base_url: String,
    pub expiry: Expiry,
    pub hotkey: Hotkey,
    pub save_dir: String,
    pub autostart: bool,
}

impl std::fmt::Debug for Form {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Form")
            .field("account_id", &self.account_id)
            .field("access_key_id", &self.access_key_id)
            .field("secret_access_key", &"<redacted>")
            .field("bucket", &self.bucket)
            .field("public_base_url", &self.public_base_url)
            .field("expiry", &self.expiry)
            .field("hotkey", &self.hotkey)
            .field("save_dir", &self.save_dir)
            .field("autostart", &self.autostart)
            .finish()
    }
}

pub fn apply_form(current: &Config, form: &Form) -> Result<Config, String> {
    let _ = (current, form, normalize_public_base_url("")); // khung tối thiểu
    let _ = (R2Config::default(), PathBuf::new());
    todo!()
}

/// Kết quả xử lý một lần nhả phím khi đang chọn phím tắt mới.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capture {
    /// Phím bổ trợ đứng một mình: chờ tiếp.
    Ignore,
    /// Phím thường cần kèm Ctrl/Alt/Shift/Win.
    NeedModifier,
    Got(Hotkey),
}

pub fn capture_hotkey(vk: u32, ctrl: bool, alt: bool, shift: bool, win: bool) -> Capture {
    let _ = (vk, ctrl, alt, shift, win);
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn base() -> Config {
        let mut c = Config::defaults(Path::new("C:/pics"));
        c.r2 = R2Config {
            account_id: "acc".into(),
            access_key_id: "kid".into(),
            secret_access_key: "OLD".into(),
            bucket: "b".into(),
            public_base_url: "https://img.example.com".into(),
        };
        c
    }

    fn form_of(c: &Config) -> Form {
        Form {
            account_id: c.r2.account_id.clone(),
            access_key_id: c.r2.access_key_id.clone(),
            secret_access_key: String::new(),
            bucket: c.r2.bucket.clone(),
            public_base_url: c.r2.public_base_url.clone(),
            expiry: c.expiry,
            hotkey: c.hotkey,
            save_dir: c.save_dir.to_string_lossy().into_owned(),
            autostart: c.autostart,
        }
    }

    #[test]
    fn empty_secret_keeps_existing() {
        let c = base();
        let mut f = form_of(&c);
        assert_eq!(apply_form(&c, &f).unwrap().r2.secret_access_key, "OLD");
        f.secret_access_key = "NEW".into();
        assert_eq!(apply_form(&c, &f).unwrap().r2.secret_access_key, "NEW");
    }

    #[test]
    fn url_is_normalized_and_bad_url_rejected() {
        let c = base();
        let mut f = form_of(&c);
        f.public_base_url = "https://img.example.com/".into();
        assert_eq!(
            apply_form(&c, &f).unwrap().r2.public_base_url,
            "https://img.example.com"
        );
        f.public_base_url = "http://x".into();
        assert!(apply_form(&c, &f).is_err());
    }

    #[test]
    fn empty_url_allowed_when_r2_not_filled() {
        let c = Config::defaults(Path::new("C:/pics"));
        let mut f = form_of(&c);
        f.save_dir = "C:/pics".into();
        let out = apply_form(&c, &f).unwrap();
        assert!(!out.r2.is_complete());
        assert_eq!(out.r2.public_base_url, "");
    }

    #[test]
    fn fields_are_trimmed() {
        let c = base();
        let mut f = form_of(&c);
        f.bucket = " bkt ".into();
        f.account_id = " acc2 ".into();
        f.access_key_id = "\tk2 ".into();
        f.save_dir = "  C:/x  ".into();
        f.secret_access_key = " S ".into();
        let out = apply_form(&c, &f).unwrap();
        assert_eq!(out.r2.bucket, "bkt");
        assert_eq!(out.r2.account_id, "acc2");
        assert_eq!(out.r2.access_key_id, "k2");
        assert_eq!(out.r2.secret_access_key, "S");
        assert_eq!(out.save_dir, PathBuf::from("C:/x"));
    }

    #[test]
    fn empty_save_dir_rejected() {
        let c = base();
        let mut f = form_of(&c);
        f.save_dir = "   ".into();
        assert!(apply_form(&c, &f).is_err());
    }

    #[test]
    fn other_fields_are_carried() {
        let c = base();
        let mut f = form_of(&c);
        f.expiry = Expiry::Keep;
        f.autostart = true;
        f.hotkey = Hotkey {
            ctrl: true,
            alt: false,
            shift: false,
            win: false,
            vk: 0x53,
        };
        let out = apply_form(&c, &f).unwrap();
        assert_eq!(out.expiry, Expiry::Keep);
        assert!(out.autostart);
        assert_eq!(out.hotkey, f.hotkey);
    }

    #[test]
    fn bad_account_or_bucket_chars_rejected() {
        let c = base();
        let mut f = form_of(&c);
        f.account_id = "a/b".into();
        assert!(apply_form(&c, &f).is_err());
        let mut f = form_of(&c);
        f.bucket = "my bucket".into();
        assert!(apply_form(&c, &f).is_err());
    }

    #[test]
    fn errors_and_debug_never_contain_secret() {
        let c = base();
        let mut f = form_of(&c);
        f.secret_access_key = "SUPER-SECRET".into();
        f.public_base_url = "http://x".into();
        let e = apply_form(&c, &f).unwrap_err();
        assert!(!e.contains("SUPER-SECRET"));
        assert!(!format!("{f:?}").contains("SUPER-SECRET"));
    }

    #[test]
    fn capture_hotkey_rules() {
        // PrintScreen đứng một mình được phép; phím chữ phải kèm phím bổ trợ.
        assert_eq!(
            capture_hotkey(0x2C, false, false, false, false),
            Capture::Got(Hotkey::default())
        );
        assert_eq!(
            capture_hotkey(0x53, false, false, false, false),
            Capture::NeedModifier
        );
        assert_eq!(
            capture_hotkey(0x53, true, false, true, false),
            Capture::Got(Hotkey {
                ctrl: true,
                alt: false,
                shift: true,
                win: false,
                vk: 0x53
            })
        );
        // Chỉ phím bổ trợ: bỏ qua.
        assert_eq!(
            capture_hotkey(0x11, true, false, false, false),
            Capture::Ignore
        );
        assert_eq!(
            capture_hotkey(0x5B, false, false, false, true),
            Capture::Ignore
        );
        // F-key đứng một mình được phép.
        assert!(matches!(
            capture_hotkey(0x71, false, false, false, false),
            Capture::Got(_)
        ));
    }
}
