//! Cấu hình ứng dụng và lưu trữ.

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
        todo!()
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
        todo!()
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
        todo!()
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
    pub fn defaults(_default_save_dir: &Path) -> Config {
        todo!()
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

pub fn load(
    _path: &Path,
    _store: &dyn SecretStore,
    _default_save_dir: &Path,
) -> Result<Loaded, ConfigError> {
    todo!()
}

pub fn save(_path: &Path, _cfg: &Config, _store: &dyn SecretStore) -> Result<(), ConfigError> {
    todo!()
}

pub fn normalize_public_base_url(_input: &str) -> Result<String, String> {
    todo!()
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
        let l = load(&temp_dir().join("config.json"), &XorStore, Path::new("C:/pics")).unwrap();
        assert_eq!(l.config, Config::defaults(Path::new("C:/pics")));
        assert_eq!(l.config.hotkey.vk, 0x2C);
        assert!(!l.config.r2.is_complete() && !l.secret_lost && !l.config.autostart);
    }

    #[test]
    fn round_trip_and_secret_not_in_plaintext() {
        let path = temp_dir().join("sub").join("config.json");
        let c = sample();
        save(&path, &c, &XorStore).unwrap();
        assert!(!std::fs::read_to_string(&path).unwrap().contains("TOP-SECRET-VALUE"));
        assert_eq!(load(&path, &XorStore, Path::new("C:/pics")).unwrap().config, c);
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
