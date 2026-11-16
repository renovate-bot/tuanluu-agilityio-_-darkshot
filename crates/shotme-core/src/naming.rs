//! Đặt tên file local và tên đối tượng trên kho.

use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug)]
pub struct LocalTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

pub fn local_file_name(_t: &LocalTime) -> String {
    todo!()
}

pub fn unique_path(_dir: &Path, _name: &str) -> PathBuf {
    todo!()
}

pub fn random_object_name() -> String {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn temp_dir() -> PathBuf {
        let mut b = [0u8; 8];
        getrandom::fill(&mut b).unwrap();
        let name: String = b.iter().map(|x| format!("{x:02x}")).collect();
        let d = std::env::temp_dir().join(format!("shotme_test_{name}"));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn file_name_format() {
        let t = LocalTime { year: 2026, month: 11, day: 16, hour: 14, minute: 3, second: 5 };
        assert_eq!(local_file_name(&t), "shotme_2026-11-16_14-03-05.png");
    }

    #[test]
    fn unique_path_adds_suffix() {
        let d = temp_dir();
        let n = "shotme_2026-11-16_14-03-05.png";
        assert_eq!(unique_path(&d, n), d.join(n));
        std::fs::write(d.join(n), b"x").unwrap();
        assert_eq!(unique_path(&d, n), d.join("shotme_2026-11-16_14-03-05_2.png"));
        std::fs::write(d.join("shotme_2026-11-16_14-03-05_2.png"), b"x").unwrap();
        assert_eq!(unique_path(&d, n), d.join("shotme_2026-11-16_14-03-05_3.png"));
    }

    #[test]
    fn random_name_shape_and_uniqueness() {
        let names: std::collections::HashSet<String> =
            (0..1000).map(|_| random_object_name()).collect();
        assert_eq!(names.len(), 1000);
        assert!(names
            .iter()
            .all(|n| n.len() == 16 && n.bytes().all(|b| b.is_ascii_alphanumeric())));
    }
}
