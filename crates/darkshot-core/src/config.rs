//! Cấu hình ứng dụng và lưu trữ.

use crate::color::CustomColors;
use crate::i18n::{t, Lang, Msg};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Expiry {
    D1,
    D7,
    #[default]
    D30,
    Keep,
}

impl Expiry {
    pub const ALL: [Expiry; 4] = [Expiry::D1, Expiry::D7, Expiry::D30, Expiry::Keep];
    pub fn prefix(self) -> &'static str {
        match self {
            Expiry::D1 => "d1",
            Expiry::D7 => "d7",
            Expiry::D30 => "d30",
            Expiry::Keep => "keep",
        }
    }

    fn from_prefix(s: &str) -> Option<Expiry> {
        Expiry::ALL.into_iter().find(|e| e.prefix() == s)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UploadFormat {
    #[default]
    Png,
    Jpeg,
}

impl UploadFormat {
    pub const ALL: [UploadFormat; 2] = [UploadFormat::Png, UploadFormat::Jpeg];

    pub fn extension(self) -> &'static str {
        match self {
            UploadFormat::Png => "png",
            UploadFormat::Jpeg => "jpg",
        }
    }

    pub fn content_type(self) -> &'static str {
        match self {
            UploadFormat::Png => "image/png",
            UploadFormat::Jpeg => "image/jpeg",
        }
    }

    pub fn label(self) -> Msg {
        match self {
            UploadFormat::Png => Msg::RadioPng,
            UploadFormat::Jpeg => Msg::RadioJpeg,
        }
    }

    fn code(self) -> &'static str {
        match self {
            UploadFormat::Png => "png",
            UploadFormat::Jpeg => "jpeg",
        }
    }

    fn from_code(s: &str) -> Option<UploadFormat> {
        UploadFormat::ALL.into_iter().find(|f| f.code() == s)
    }
}

pub const JPEG_QUALITY_MIN: u8 = 50;
pub const JPEG_QUALITY_MAX: u8 = 100;
pub const JPEG_QUALITY_DEFAULT: u8 = 90;

pub fn clamp_jpeg_quality(v: i64) -> u8 {
    let c = v.clamp(i64::from(JPEG_QUALITY_MIN), i64::from(JPEG_QUALITY_MAX));
    u8::try_from(c).unwrap_or(JPEG_QUALITY_DEFAULT)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hotkey {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub win: bool,
    pub vk: u32,
}

impl Default for Hotkey {
    fn default() -> Self {
        Hotkey {
            ctrl: false,
            alt: false,
            shift: false,
            win: false,
            vk: 0x2C,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Default)]
pub struct R2Config {
    pub account_id: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub bucket: String,
    pub public_base_url: String,
}

impl std::fmt::Debug for R2Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("R2Config")
            .field("account_id", &self.account_id)
            .field("access_key_id", &self.access_key_id)
            .field("secret_access_key", &"<redacted>")
            .field("bucket", &self.bucket)
            .field("public_base_url", &self.public_base_url)
            .finish()
    }
}

impl R2Config {
    pub fn is_complete(&self) -> bool {
        [
            &self.account_id,
            &self.access_key_id,
            &self.secret_access_key,
            &self.bucket,
            &self.public_base_url,
        ]
        .iter()
        .all(|s| !s.is_empty())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HotkeyRole {
    Region,
    SaveFull,
    UploadFull,
}

impl HotkeyRole {
    pub const ALL: [HotkeyRole; 3] = [
        HotkeyRole::Region,
        HotkeyRole::SaveFull,
        HotkeyRole::UploadFull,
    ];

    pub fn label(self) -> Msg {
        match self {
            HotkeyRole::Region => Msg::LabelHotkey,
            HotkeyRole::SaveFull => Msg::LabelHotkeySaveFull,
            HotkeyRole::UploadFull => Msg::LabelHotkeyUploadFull,
        }
    }

    pub fn default_slot(self) -> HotkeySlot {
        let d = Hotkeys::default();
        match self {
            HotkeyRole::Region => d.region,
            HotkeyRole::SaveFull => d.save_full,
            HotkeyRole::UploadFull => d.upload_full,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HotkeySlot {
    pub enabled: bool,
    pub key: Hotkey,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hotkeys {
    pub region: HotkeySlot,
    pub save_full: HotkeySlot,
    pub upload_full: HotkeySlot,
}

impl Default for Hotkeys {
    fn default() -> Self {
        let pre = Hotkey::default();
        Hotkeys {
            region: HotkeySlot {
                enabled: true,
                key: pre,
            },
            save_full: HotkeySlot {
                enabled: false,
                key: Hotkey { shift: true, ..pre },
            },
            upload_full: HotkeySlot {
                enabled: false,
                key: Hotkey { ctrl: true, ..pre },
            },
        }
    }
}

impl Hotkeys {
    pub fn get(&self, role: HotkeyRole) -> HotkeySlot {
        match role {
            HotkeyRole::Region => self.region,
            HotkeyRole::SaveFull => self.save_full,
            HotkeyRole::UploadFull => self.upload_full,
        }
    }

    pub fn set(&mut self, role: HotkeyRole, slot: HotkeySlot) {
        match role {
            HotkeyRole::Region => self.region = slot,
            HotkeyRole::SaveFull => self.save_full = slot,
            HotkeyRole::UploadFull => self.upload_full = slot,
        }
    }

    /// Cặp dòng đầu tiên (theo `HotkeyRole::ALL`) cùng bật và cùng tổ hợp.
    pub fn first_conflict(&self) -> Option<(HotkeyRole, HotkeyRole)> {
        for (i, a) in HotkeyRole::ALL.into_iter().enumerate() {
            for b in HotkeyRole::ALL.into_iter().skip(i + 1) {
                let (sa, sb) = (self.get(a), self.get(b));
                if sa.enabled && sb.enabled && sa.key == sb.key {
                    return Some((a, b));
                }
            }
        }
        None
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub r2: R2Config,
    pub expiry: Expiry,
    pub hotkeys: Hotkeys,
    pub save_dir: PathBuf,
    pub autostart: bool,
    pub language: Lang,
    pub show_success_notices: bool,
    pub keep_selection: bool,
    pub include_cursor: bool,
    pub upload_format: UploadFormat,
    pub jpeg_quality: u8,
    pub custom_colors: CustomColors,
}

impl Config {
    pub fn defaults(default_save_dir: &Path, language: Lang) -> Config {
        Config {
            r2: R2Config::default(),
            expiry: Expiry::default(),
            hotkeys: Hotkeys::default(),
            save_dir: default_save_dir.to_path_buf(),
            autostart: false,
            language,
            show_success_notices: true,
            keep_selection: false,
            include_cursor: false,
            upload_format: UploadFormat::Png,
            jpeg_quality: JPEG_QUALITY_DEFAULT,
            custom_colors: CustomColors::default(),
        }
    }
}

pub trait SecretStore: Send + Sync {
    fn protect(&self, plain: &[u8]) -> Result<Vec<u8>, String>;
    fn unprotect(&self, cipher: &[u8]) -> Result<Vec<u8>, String>;
}

#[derive(Debug)]
pub enum ConfigError {
    Io(String),
    Corrupt(String),
    Secret(String),
}

pub struct Loaded {
    pub config: Config,
    pub secret_lost: bool,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct FileR2 {
    account_id: String,
    access_key_id: String,
    secret_access_key_protected: String,
    bucket: String,
    public_base_url: String,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct FileHotkey {
    ctrl: bool,
    alt: bool,
    shift: bool,
    win: bool,
    vk: u32,
}

impl Default for FileHotkey {
    fn default() -> Self {
        let h = Hotkey::default();
        FileHotkey {
            ctrl: h.ctrl,
            alt: h.alt,
            shift: h.shift,
            win: h.win,
            vk: h.vk,
        }
    }
}

/// Một dòng phím tắt đầy đủ trong file; trường con thiếu lấy từ mặc định của dòng đó.
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct FileSlot {
    enabled: Option<bool>,
    ctrl: Option<bool>,
    alt: Option<bool>,
    shift: Option<bool>,
    win: Option<bool>,
    vk: Option<u32>,
}

impl FileSlot {
    fn from_slot(s: HotkeySlot) -> FileSlot {
        FileSlot {
            enabled: Some(s.enabled),
            ctrl: Some(s.key.ctrl),
            alt: Some(s.key.alt),
            shift: Some(s.key.shift),
            win: Some(s.key.win),
            vk: Some(s.key.vk),
        }
    }

    fn into_slot(self, default: HotkeySlot) -> HotkeySlot {
        HotkeySlot {
            enabled: self.enabled.unwrap_or(default.enabled),
            key: Hotkey {
                ctrl: self.ctrl.unwrap_or(default.key.ctrl),
                alt: self.alt.unwrap_or(default.key.alt),
                shift: self.shift.unwrap_or(default.key.shift),
                win: self.win.unwrap_or(default.key.win),
                vk: self.vk.unwrap_or(default.key.vk),
            },
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct FileConfig {
    r2: FileR2,
    expiry: String,
    hotkey: FileHotkey,
    hotkey_enabled: bool,
    hotkey_save_full: Option<FileSlot>,
    hotkey_upload_full: Option<FileSlot>,
    notify_success: bool,
    keep_selection: bool,
    include_cursor: bool,
    upload_format: String,
    jpeg_quality: i64,
    custom_colors: serde_json::Value,
    save_dir: Option<PathBuf>,
    autostart: bool,
    language: String,
}

impl Default for FileConfig {
    fn default() -> Self {
        FileConfig {
            r2: FileR2::default(),
            expiry: Expiry::default().prefix().to_string(),
            hotkey: FileHotkey::default(),
            hotkey_enabled: true,
            hotkey_save_full: None,
            hotkey_upload_full: None,
            notify_success: true,
            keep_selection: false,
            include_cursor: false,
            upload_format: UploadFormat::default().code().to_string(),
            jpeg_quality: i64::from(JPEG_QUALITY_DEFAULT),
            custom_colors: serde_json::Value::Null,
            save_dir: None,
            autostart: false,
            language: String::new(),
        }
    }
}

pub fn load(
    path: &Path,
    store: &dyn SecretStore,
    default_save_dir: &Path,
    os_lang: Lang,
) -> Result<Loaded, ConfigError> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Loaded {
                config: Config::defaults(default_save_dir, os_lang),
                secret_lost: false,
            })
        }
        Err(e) => return Err(ConfigError::Io(e.to_string())),
    };
    let f: FileConfig =
        serde_json::from_str(&text).map_err(|e| ConfigError::Corrupt(e.to_string()))?;

    let mut secret_lost = false;
    let secret = if f.r2.secret_access_key_protected.is_empty() {
        String::new()
    } else {
        let plain = BASE64
            .decode(&f.r2.secret_access_key_protected)
            .map_err(|e| e.to_string())
            .and_then(|c| store.unprotect(&c))
            .ok()
            .and_then(|p| String::from_utf8(p).ok());
        match plain {
            Some(p) => p,
            None => {
                secret_lost = true;
                String::new()
            }
        }
    };

    let config = Config {
        r2: R2Config {
            account_id: f.r2.account_id,
            access_key_id: f.r2.access_key_id,
            secret_access_key: secret,
            bucket: f.r2.bucket,
            public_base_url: f.r2.public_base_url,
        },
        expiry: Expiry::from_prefix(&f.expiry).unwrap_or_default(),
        hotkeys: Hotkeys {
            region: HotkeySlot {
                enabled: f.hotkey_enabled,
                key: Hotkey {
                    ctrl: f.hotkey.ctrl,
                    alt: f.hotkey.alt,
                    shift: f.hotkey.shift,
                    win: f.hotkey.win,
                    vk: f.hotkey.vk,
                },
            },
            save_full: f
                .hotkey_save_full
                .unwrap_or_default()
                .into_slot(HotkeyRole::SaveFull.default_slot()),
            upload_full: f
                .hotkey_upload_full
                .unwrap_or_default()
                .into_slot(HotkeyRole::UploadFull.default_slot()),
        },
        save_dir: f.save_dir.unwrap_or_else(|| default_save_dir.to_path_buf()),
        autostart: f.autostart,
        language: Lang::from_code(&f.language).unwrap_or(Lang::Vi),
        show_success_notices: f.notify_success,
        keep_selection: f.keep_selection,
        include_cursor: f.include_cursor,
        upload_format: UploadFormat::from_code(&f.upload_format).unwrap_or_default(),
        jpeg_quality: clamp_jpeg_quality(f.jpeg_quality),
        custom_colors: CustomColors::default(),
    };
    Ok(Loaded {
        config,
        secret_lost,
    })
}

pub fn save(path: &Path, cfg: &Config, store: &dyn SecretStore) -> Result<(), ConfigError> {
    let protected = if cfg.r2.secret_access_key.is_empty() {
        String::new()
    } else {
        let c = store
            .protect(cfg.r2.secret_access_key.as_bytes())
            .map_err(ConfigError::Secret)?;
        BASE64.encode(c)
    };
    let f = FileConfig {
        r2: FileR2 {
            account_id: cfg.r2.account_id.clone(),
            access_key_id: cfg.r2.access_key_id.clone(),
            secret_access_key_protected: protected,
            bucket: cfg.r2.bucket.clone(),
            public_base_url: cfg.r2.public_base_url.clone(),
        },
        expiry: cfg.expiry.prefix().to_string(),
        hotkey: FileHotkey {
            ctrl: cfg.hotkeys.region.key.ctrl,
            alt: cfg.hotkeys.region.key.alt,
            shift: cfg.hotkeys.region.key.shift,
            win: cfg.hotkeys.region.key.win,
            vk: cfg.hotkeys.region.key.vk,
        },
        hotkey_enabled: cfg.hotkeys.region.enabled,
        hotkey_save_full: Some(FileSlot::from_slot(cfg.hotkeys.save_full)),
        hotkey_upload_full: Some(FileSlot::from_slot(cfg.hotkeys.upload_full)),
        notify_success: cfg.show_success_notices,
        keep_selection: cfg.keep_selection,
        include_cursor: cfg.include_cursor,
        upload_format: cfg.upload_format.code().to_string(),
        jpeg_quality: i64::from(clamp_jpeg_quality(i64::from(cfg.jpeg_quality))),
        custom_colors: serde_json::Value::Null,
        save_dir: Some(cfg.save_dir.clone()),
        autostart: cfg.autostart,
        language: cfg.language.code().to_string(),
    };
    let json = serde_json::to_string_pretty(&f).map_err(|e| ConfigError::Io(e.to_string()))?;

    let io = |e: std::io::Error| ConfigError::Io(e.to_string());
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty());
    if let Some(d) = dir {
        std::fs::create_dir_all(d).map_err(io)?;
    }
    let mut tmp_name = path.file_name().unwrap_or_default().to_os_string();
    tmp_name.push(".tmp");
    let tmp = match dir {
        Some(d) => d.join(tmp_name),
        None => PathBuf::from(tmp_name),
    };
    let write_tmp = || -> std::io::Result<()> {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(json.as_bytes())?;
        f.sync_all()
    };
    if let Err(e) = write_tmp() {
        let _ = std::fs::remove_file(&tmp);
        return Err(io(e));
    }
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        io(e)
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UrlError {
    NotHttps,
    MissingHost,
    Whitespace,
    ForbiddenChar,
}

impl UrlError {
    pub fn message(&self, lang: Lang) -> String {
        let m = match self {
            UrlError::NotHttps => Msg::UrlNotHttps,
            UrlError::MissingHost => Msg::UrlMissingHost,
            UrlError::Whitespace => Msg::UrlWhitespace,
            UrlError::ForbiddenChar => Msg::UrlForbiddenChar,
        };
        t(lang, m).to_string()
    }
}

pub fn normalize_public_base_url(input: &str) -> Result<String, UrlError> {
    let trimmed = input.trim();
    if !trimmed.starts_with("https://") {
        return Err(UrlError::NotHttps);
    }
    // Cắt dấu `/` cuối sau khi đã xác nhận có `https://`, để "https://" trơn là thiếu tên miền.
    let s = trimmed.trim_end_matches('/');
    let rest = s.strip_prefix("https://").unwrap_or("");
    if rest.is_empty() {
        return Err(UrlError::MissingHost);
    }
    if s.chars().any(char::is_whitespace) {
        return Err(UrlError::Whitespace);
    }
    if rest.contains(['?', '#', '@']) {
        return Err(UrlError::ForbiddenChar);
    }
    let authority = rest.split('/').next().unwrap_or("");
    let host = match authority.rsplit_once(':') {
        Some((h, _port)) => h,
        None => authority,
    };
    if host.is_empty() {
        return Err(UrlError::MissingHost);
    }
    Ok(s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> PathBuf {
        let mut b = [0u8; 8];
        getrandom::fill(&mut b).unwrap();
        let name: String = b.iter().map(|x| format!("{x:02x}")).collect();
        let d = std::env::temp_dir().join(format!("darkshot_test_{name}"));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

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
        fn protect(&self, _plain: &[u8]) -> Result<Vec<u8>, String> {
            Err("fail".into())
        }
        fn unprotect(&self, _cipher: &[u8]) -> Result<Vec<u8>, String> {
            Err("fail".into())
        }
    }

    fn sample() -> Config {
        let mut c = Config::defaults(Path::new("C:/pics"), Lang::Vi);
        c.r2 = R2Config {
            account_id: "acc".into(),
            access_key_id: "kid".into(),
            secret_access_key: "TOP-SECRET-VALUE".into(),
            bucket: "b".into(),
            public_base_url: "https://img.example.com".into(),
        };
        c.expiry = Expiry::D7;
        c.autostart = true;
        c
    }

    #[test]
    fn expiry_prefixes_and_default() {
        assert_eq!(Expiry::ALL.map(Expiry::prefix), ["d1", "d7", "d30", "keep"]);
        assert_eq!(Expiry::default(), Expiry::D30);
    }

    #[test]
    fn missing_file_gives_defaults() {
        let l = load(
            &temp_dir().join("config.json"),
            &XorStore,
            Path::new("C:/pics"),
            Lang::Vi,
        )
        .unwrap();
        assert_eq!(l.config, Config::defaults(Path::new("C:/pics"), Lang::Vi));
        assert_eq!(l.config.hotkeys.region.key.vk, 0x2C);
        assert!(!l.config.r2.is_complete() && !l.secret_lost && !l.config.autostart);
    }

    #[test]
    fn round_trip_and_secret_not_in_plaintext() {
        let path = temp_dir().join("sub").join("config.json");
        let c = sample();
        save(&path, &c, &XorStore).unwrap();
        assert!(!std::fs::read_to_string(&path)
            .unwrap()
            .contains("TOP-SECRET-VALUE"));
        assert_eq!(
            load(&path, &XorStore, Path::new("C:/pics"), Lang::Vi)
                .unwrap()
                .config,
            c
        );
    }

    #[test]
    fn save_leaves_no_temp_file() {
        let dir = temp_dir();
        save(&dir.join("config.json"), &sample(), &XorStore).unwrap();
        let names: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["config.json".to_string()]);
    }

    #[test]
    fn corrupt_file_is_error_and_untouched() {
        let path = temp_dir().join("config.json");
        std::fs::write(&path, "{not json").unwrap();
        assert!(matches!(
            load(&path, &XorStore, Path::new("C:/pics"), Lang::Vi),
            Err(ConfigError::Corrupt(_))
        ));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{not json");
    }

    #[test]
    fn undecryptable_secret_keeps_other_fields() {
        let path = temp_dir().join("config.json");
        save(&path, &sample(), &XorStore).unwrap();
        let l = load(&path, &FailStore, Path::new("C:/pics"), Lang::Vi).unwrap();
        assert!(l.secret_lost);
        assert_eq!(l.config.r2.secret_access_key, "");
        assert_eq!(l.config.r2.bucket, "b");
    }

    #[test]
    fn public_base_url_normalization() {
        assert_eq!(
            normalize_public_base_url("  https://img.example.com// ").unwrap(),
            "https://img.example.com"
        );
        assert_eq!(
            normalize_public_base_url("https://pub-1a2b.r2.dev").unwrap(),
            "https://pub-1a2b.r2.dev"
        );
        let err = |s: &str| normalize_public_base_url(s).unwrap_err();
        assert_eq!(err("http://x"), UrlError::NotHttps);
        assert_eq!(err("http://img.example.com"), UrlError::NotHttps);
        assert_eq!(err("img.example.com"), UrlError::NotHttps);
        assert_eq!(err("https://"), UrlError::MissingHost);
        assert_eq!(err("https://:443"), UrlError::MissingHost);
        assert_eq!(err("https://img.exa mple.com"), UrlError::Whitespace);
        assert_eq!(err("https://a.com?x"), UrlError::ForbiddenChar);
    }

    #[test]
    fn public_base_url_rejects_query_fragment_userinfo_and_empty_host() {
        for bad in [
            "https://img.example.com/?x=1",
            "https://img.example.com?x",
            "https://img.example.com/#top",
            "https://img.example.com#top",
            "https://user@img.example.com",
            "https://evil.com@img.example.com",
            "https:///path",
            "https://:443",
            "https://:443/path",
        ] {
            let e = normalize_public_base_url(bad).unwrap_err();
            let want = if bad.starts_with("https:///") || bad.starts_with("https://:") {
                UrlError::MissingHost
            } else {
                UrlError::ForbiddenChar
            };
            assert_eq!(e, want, "{bad:?}");
        }
        // Có đường dẫn con và cổng thì vẫn hợp lệ.
        assert_eq!(
            normalize_public_base_url("https://img.example.com:8443/shots/").unwrap(),
            "https://img.example.com:8443/shots"
        );
    }

    #[test]
    fn url_error_messages_differ_between_languages() {
        let all = [
            UrlError::NotHttps,
            UrlError::MissingHost,
            UrlError::Whitespace,
            UrlError::ForbiddenChar,
        ];
        for e in all {
            let (vi, en) = (e.message(Lang::Vi), e.message(Lang::En));
            assert!(!vi.is_empty() && !en.is_empty() && vi != en, "{e:?}");
        }
    }

    #[test]
    fn missing_file_uses_os_language() {
        let l = load(
            &temp_dir().join("config.json"),
            &XorStore,
            Path::new("C:/pics"),
            Lang::En,
        )
        .unwrap();
        assert_eq!(l.config.language, Lang::En);
    }

    #[test]
    fn file_without_language_field_is_vietnamese() {
        let p = temp_dir().join("config.json");
        std::fs::write(&p, "{}").unwrap();
        assert_eq!(
            load(&p, &XorStore, Path::new("C:/pics"), Lang::En)
                .unwrap()
                .config
                .language,
            Lang::Vi
        );
    }

    #[test]
    fn unknown_language_value_is_vietnamese() {
        let p = temp_dir().join("config.json");
        std::fs::write(&p, r#"{"language":"fr"}"#).unwrap();
        assert_eq!(
            load(&p, &XorStore, Path::new("C:/pics"), Lang::En)
                .unwrap()
                .config
                .language,
            Lang::Vi
        );
    }

    #[test]
    fn language_round_trips() {
        let p = temp_dir().join("config.json");
        let mut c = sample();
        c.language = Lang::En;
        save(&p, &c, &XorStore).unwrap();
        assert!(std::fs::read_to_string(&p)
            .unwrap()
            .contains(r#""language": "en""#));
        assert_eq!(
            load(&p, &XorStore, Path::new("C:/pics"), Lang::Vi)
                .unwrap()
                .config
                .language,
            Lang::En
        );
    }

    fn load_text(text: &str) -> Loaded {
        let p = temp_dir().join("config.json");
        std::fs::write(&p, text).unwrap();
        load(&p, &XorStore, Path::new("C:/pics"), Lang::Vi).unwrap()
    }

    #[test]
    fn default_slots() {
        let h = Hotkeys::default();
        assert!(h.region.enabled && !h.save_full.enabled && !h.upload_full.enabled);
        assert_eq!(h.region.key, Hotkey::default());
        assert_eq!(
            h.save_full.key,
            Hotkey {
                shift: true,
                ..Hotkey::default()
            }
        );
        assert_eq!(
            h.upload_full.key,
            Hotkey {
                ctrl: true,
                ..Hotkey::default()
            }
        );
        for r in HotkeyRole::ALL {
            assert_eq!(h.get(r), r.default_slot());
        }
    }

    #[test]
    fn get_and_set_address_the_right_row() {
        let slot = HotkeySlot {
            enabled: true,
            key: Hotkey {
                alt: true,
                vk: 0x70,
                ..Hotkey::default()
            },
        };
        for r in HotkeyRole::ALL {
            let mut h = Hotkeys::default();
            let before = h;
            h.set(r, slot);
            assert_eq!(h.get(r), slot);
            for o in HotkeyRole::ALL.into_iter().filter(|o| *o != r) {
                assert_eq!(h.get(o), before.get(o));
            }
        }
    }

    #[test]
    fn old_file_keeps_behaviour() {
        let l = load_text(
            r#"{"hotkey":{"ctrl":true,"alt":false,"shift":true,"win":false,"vk":83},"language":"vi"}"#,
        );
        assert_eq!(
            l.config.hotkeys.region,
            HotkeySlot {
                enabled: true,
                key: Hotkey {
                    ctrl: true,
                    alt: false,
                    shift: true,
                    win: false,
                    vk: 83
                }
            }
        );
        assert_eq!(
            l.config.hotkeys.save_full,
            HotkeyRole::SaveFull.default_slot()
        );
        assert_eq!(
            l.config.hotkeys.upload_full,
            HotkeyRole::UploadFull.default_slot()
        );
        assert!(
            l.config.show_success_notices && !l.config.keep_selection && !l.config.include_cursor
        );
    }

    #[test]
    fn new_fields_round_trip() {
        let path = temp_dir().join("config.json");
        let mut c = sample();
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
        c.show_success_notices = false;
        c.keep_selection = true;
        c.include_cursor = true;
        save(&path, &c, &XorStore).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["hotkey"]["vk"], 0x70);
        assert_eq!(v["hotkey"]["ctrl"], true);
        assert_eq!(
            load(&path, &XorStore, Path::new("C:/pics"), Lang::Vi)
                .unwrap()
                .config,
            c
        );
    }

    #[test]
    fn partial_slot_uses_that_rows_defaults() {
        let l = load_text(r#"{"hotkey_save_full":{"enabled":true}}"#);
        assert_eq!(
            l.config.hotkeys.save_full,
            HotkeySlot {
                enabled: true,
                key: Hotkey {
                    shift: true,
                    ..Hotkey::default()
                }
            }
        );
        let l = load_text(r#"{"hotkey_upload_full":{"enabled":true}}"#);
        assert_eq!(
            l.config.hotkeys.upload_full,
            HotkeySlot {
                enabled: true,
                key: Hotkey {
                    ctrl: true,
                    ..Hotkey::default()
                }
            }
        );
    }

    #[test]
    fn wrong_type_is_corrupt() {
        let p = temp_dir().join("config.json");
        std::fs::write(&p, r#"{"notify_success":"yes"}"#).unwrap();
        assert!(matches!(
            load(&p, &XorStore, Path::new("C:/pics"), Lang::Vi),
            Err(ConfigError::Corrupt(_))
        ));
    }

    #[test]
    fn old_file_uploads_png() {
        let l = load_text(r#"{"language":"vi","expiry":"d7"}"#);
        assert_eq!(l.config.upload_format, UploadFormat::Png);
        assert_eq!(l.config.jpeg_quality, 90);
    }

    #[test]
    fn upload_format_round_trip() {
        let path = temp_dir().join("config.json");
        let mut c = sample();
        c.upload_format = UploadFormat::Jpeg;
        c.jpeg_quality = 75;
        save(&path, &c, &XorStore).unwrap();
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(v["upload_format"], "jpeg");
        assert_eq!(v["jpeg_quality"], 75);
        let back = load(&path, &XorStore, Path::new("C:/pics"), Lang::Vi)
            .unwrap()
            .config;
        assert_eq!(back.upload_format, UploadFormat::Jpeg);
        assert_eq!(back.jpeg_quality, 75);
        assert_eq!(back, c);
    }

    #[test]
    fn jpeg_quality_is_clamped_on_load() {
        for (input, want) in [
            (-1, 50),
            (0, 50),
            (49, 50),
            (101, 100),
            (255, 100),
            (300, 100),
            (50, 50),
            (90, 90),
            (100, 100),
        ] {
            let l = load_text(&format!(r#"{{"jpeg_quality":{input}}}"#));
            assert_eq!(l.config.jpeg_quality, want, "input {input}");
        }
    }

    #[test]
    fn unknown_format_is_png() {
        let l = load_text(r#"{"upload_format":"webp"}"#);
        assert_eq!(l.config.upload_format, UploadFormat::Png);
    }

    #[test]
    fn wrong_type_quality_is_corrupt() {
        for text in [
            r#"{"jpeg_quality":"90"}"#,
            r#"{"jpeg_quality":90.5}"#,
            r#"{"upload_format":1}"#,
        ] {
            let p = temp_dir().join("config.json");
            std::fs::write(&p, text).unwrap();
            assert!(
                matches!(
                    load(&p, &XorStore, Path::new("C:/pics"), Lang::Vi),
                    Err(ConfigError::Corrupt(_))
                ),
                "{text}"
            );
        }
    }

    #[test]
    fn format_extension_and_content_type() {
        assert_eq!(UploadFormat::Png.extension(), "png");
        assert_eq!(UploadFormat::Png.content_type(), "image/png");
        assert_eq!(UploadFormat::Jpeg.extension(), "jpg");
        assert_eq!(UploadFormat::Jpeg.content_type(), "image/jpeg");
        assert_eq!(UploadFormat::Png.label(), Msg::RadioPng);
        assert_eq!(UploadFormat::Jpeg.label(), Msg::RadioJpeg);
    }

    #[test]
    fn conflict_only_between_enabled_rows() {
        let mut h = Hotkeys::default();
        assert_eq!(h.first_conflict(), None);
        h.save_full.key = h.region.key;
        assert_eq!(h.first_conflict(), None);
        h.save_full.enabled = true;
        assert_eq!(
            h.first_conflict(),
            Some((HotkeyRole::Region, HotkeyRole::SaveFull))
        );
        h.region.enabled = false;
        assert_eq!(h.first_conflict(), None);
        h.upload_full = h.save_full;
        assert_eq!(
            h.first_conflict(),
            Some((HotkeyRole::SaveFull, HotkeyRole::UploadFull))
        );
        h.region.enabled = true;
        assert_eq!(
            h.first_conflict(),
            Some((HotkeyRole::Region, HotkeyRole::SaveFull))
        );
    }

    #[test]
    fn config_custom_colors_round_trip() {
        use crate::theme::Rgb;
        let path = temp_dir().join("config.json");
        let mut c = sample();
        c.custom_colors.add(Rgb(0xFF, 0x3B, 0x30), None);
        c.custom_colors.add(Rgb(1, 2, 3), Some(9));
        save(&path, &c, &XorStore).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["custom_colors"][0], "#FF3B30");
        assert_eq!(v["custom_colors"][9], "#010203");
        assert!(v["custom_colors"][1].is_null());
        assert_eq!(v["custom_colors"].as_array().unwrap().len(), 16);
        assert_eq!(
            load(&path, &XorStore, Path::new("C:/pics"), Lang::Vi)
                .unwrap()
                .config,
            c
        );
        // File không có trường: trống.
        let l = load_text(r#"{"language":"vi"}"#);
        assert_eq!(l.config.custom_colors, CustomColors::default());
        // Phần tử sai định dạng: ô trống; dài hơn 16: bỏ phần thừa; sai kiểu: trống.
        let mut arr = vec![serde_json::Value::from("#0A0B0C"), "xyz".into(), 5.into()];
        arr.resize(20, serde_json::Value::from("#FFFFFF"));
        let text = serde_json::json!({ "custom_colors": arr }).to_string();
        let l = load_text(&text);
        let s = l.config.custom_colors.slots;
        assert_eq!(s[0], Some(Rgb(10, 11, 12)));
        assert_eq!((s[1], s[2]), (None, None));
        assert_eq!(s[15], Some(Rgb(255, 255, 255)));
        // Mảng ngắn.
        let l = load_text(r##"{"custom_colors":["#000001"]}"##);
        assert_eq!(l.config.custom_colors.slots[0], Some(Rgb(0, 0, 1)));
        assert_eq!(l.config.custom_colors.slots[1], None);
        let l = load_text(r#"{"custom_colors":"oops"}"#);
        assert_eq!(l.config.custom_colors, CustomColors::default());
    }
}
