//! Đặt tên file local và tên đối tượng trên kho.

use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

pub fn local_file_name(t: &LocalTime) -> String {
    format!(
        "darkshot_{:04}-{:02}-{:02}_{:02}-{:02}-{:02}.png",
        t.year, t.month, t.day, t.hour, t.minute, t.second
    )
}

pub fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let first = dir.join(name);
    if !first.exists() {
        return first;
    }
    let (stem, ext) = match name.rfind('.') {
        Some(i) => (&name[..i], &name[i..]),
        None => (name, ""),
    };
    let mut n = 2u32;
    loop {
        let candidate = dir.join(format!("{stem}_{n}{ext}"));
        if !candidate.exists() {
            return candidate;
        }
        n += 1;
    }
}

const ALPHABET: &[u8; 62] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

/// 16 ký tự `[A-Za-z0-9]`; lấy mẫu loại trừ (bỏ byte >= 248) để không lệch phân phối.
///
/// Panic nếu nguồn ngẫu nhiên của hệ điều hành lỗi.
pub fn random_object_name() -> String {
    let mut out = String::with_capacity(16);
    let mut buf = [0u8; 32];
    while out.len() < 16 {
        getrandom::fill(&mut buf).expect("OS random source unavailable");
        for &b in buf.iter().filter(|&&b| b < 248) {
            if out.len() == 16 {
                break;
            }
            out.push(ALPHABET[(b % 62) as usize] as char);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn temp_dir() -> PathBuf {
        let mut b = [0u8; 8];
        getrandom::fill(&mut b).unwrap();
        let name: String = b.iter().map(|x| format!("{x:02x}")).collect();
        let d = std::env::temp_dir().join(format!("darkshot_test_{name}"));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn file_name_format() {
        let t = LocalTime {
            year: 2026,
            month: 11,
            day: 16,
            hour: 14,
            minute: 3,
            second: 5,
        };
        assert_eq!(local_file_name(&t), "darkshot_2026-11-16_14-03-05.png");
    }

    #[test]
    fn unique_path_adds_suffix() {
        let d = temp_dir();
        let n = "darkshot_2026-11-16_14-03-05.png";
        assert_eq!(unique_path(&d, n), d.join(n));
        std::fs::write(d.join(n), b"x").unwrap();
        assert_eq!(
            unique_path(&d, n),
            d.join("darkshot_2026-11-16_14-03-05_2.png")
        );
        std::fs::write(d.join("darkshot_2026-11-16_14-03-05_2.png"), b"x").unwrap();
        assert_eq!(
            unique_path(&d, n),
            d.join("darkshot_2026-11-16_14-03-05_3.png")
        );
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
