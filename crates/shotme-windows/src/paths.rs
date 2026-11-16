use shotme_core::naming::LocalTime;
use std::path::PathBuf;

/// `%APPDATA%\shotme\config.json`
pub fn config_path() -> PathBuf {
    todo!()
}

/// `Pictures\shotme`
pub fn default_save_dir() -> PathBuf {
    todo!()
}

pub fn local_now() -> LocalTime {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_end_correctly() {
        assert!(config_path().ends_with("shotme/config.json"));
        assert!(default_save_dir().ends_with("shotme"));
    }

    #[test]
    fn local_now_is_plausible() {
        let t = local_now();
        assert!(t.year >= 2026 && (1..=12).contains(&t.month));
    }
}
