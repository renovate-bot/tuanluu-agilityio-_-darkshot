//! Cấu hình ứng dụng và lưu trữ.

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub r2: R2Config,
    pub expiry: Expiry,
    pub hotkey: Hotkey,
    pub save_dir: PathBuf,
    pub autostart: bool,
}

impl Config {
    pub fn defaults(default_save_dir: &Path) -> Config {
        Config {
            r2: R2Config::default(),
            expiry: Expiry::default(),
            hotkey: Hotkey::default(),
            save_dir: default_save_dir.to_path_buf(),
            autostart: false,
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

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct FileConfig {
    r2: FileR2,
    expiry: String,
    hotkey: FileHotkey,
    save_dir: Option<PathBuf>,
    autostart: bool,
}

impl Default for FileConfig {
    fn default() -> Self {
        FileConfig {
            r2: FileR2::default(),
            expiry: Expiry::default().prefix().to_string(),
            hotkey: FileHotkey::default(),
            save_dir: None,
            autostart: false,
        }
    }
}

pub fn load(
    path: &Path,
    store: &dyn SecretStore,
    default_save_dir: &Path,
) -> Result<Loaded, ConfigError> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Loaded {
                config: Config::defaults(default_save_dir),
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
        hotkey: Hotkey {
            ctrl: f.hotkey.ctrl,
            alt: f.hotkey.alt,
            shift: f.hotkey.shift,
            win: f.hotkey.win,
            vk: f.hotkey.vk,
        },
        save_dir: f.save_dir.unwrap_or_else(|| default_save_dir.to_path_buf()),
        autostart: f.autostart,
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
            ctrl: cfg.hotkey.ctrl,
            alt: cfg.hotkey.alt,
            shift: cfg.hotkey.shift,
            win: cfg.hotkey.win,
            vk: cfg.hotkey.vk,
        },
        save_dir: Some(cfg.save_dir.clone()),
        autostart: cfg.autostart,
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

pub fn normalize_public_base_url(input: &str) -> Result<String, String> {
    let s = input.trim().trim_end_matches('/');
    let Some(rest) = s.strip_prefix("https://") else {
        return Err("Địa chỉ công khai phải bắt đầu bằng https://".into());
    };
    if rest.is_empty() {
        return Err("Địa chỉ công khai thiếu tên miền sau https://".into());
    }
    if s.chars().any(char::is_whitespace) {
        return Err("Địa chỉ công khai không được chứa khoảng trắng".into());
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
        let d = std::env::temp_dir().join(format!("shotme_test_{name}"));
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
        let mut c = Config::defaults(Path::new("C:/pics"));
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
        )
        .unwrap();
        assert_eq!(l.config, Config::defaults(Path::new("C:/pics")));
        assert_eq!(l.config.hotkey.vk, 0x2C);
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
            load(&path, &XorStore, Path::new("C:/pics")).unwrap().config,
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
            load(&path, &XorStore, Path::new("C:/pics")),
            Err(ConfigError::Corrupt(_))
        ));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{not json");
    }

    #[test]
    fn undecryptable_secret_keeps_other_fields() {
        let path = temp_dir().join("config.json");
        save(&path, &sample(), &XorStore).unwrap();
        let l = load(&path, &FailStore, Path::new("C:/pics")).unwrap();
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
        assert!(normalize_public_base_url("http://img.example.com").is_err());
        assert!(normalize_public_base_url("img.example.com").is_err());
        assert!(normalize_public_base_url("https://").is_err());
        assert!(normalize_public_base_url("https://img.exa mple.com").is_err());
    }
}
