//! Đường dẫn và giờ địa phương. Không panic: thiếu thư mục hệ thống thì dùng phương án dự phòng.

use darkshot_core::naming::LocalTime;
use std::path::{Path, PathBuf};
use windows::core::GUID;
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::UI::Shell::{
    FOLDERID_Pictures, FOLDERID_RoamingAppData, SHGetKnownFolderPath, KF_FLAG_DEFAULT,
};

fn known_folder(id: &GUID) -> Option<PathBuf> {
    // SAFETY: SHGetKnownFolderPath trả về chuỗi do COM cấp; ta sao chép rồi giải phóng bằng CoTaskMemFree.
    unsafe {
        let p = SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None).ok()?;
        let s = p.to_string().ok();
        CoTaskMemFree(Some(p.0 as *const _));
        s.filter(|s| !s.is_empty()).map(PathBuf::from)
    }
}

/// Thư mục gốc `%APPDATA%` (Roaming).
pub fn appdata_dir() -> PathBuf {
    known_folder(&FOLDERID_RoamingAppData)
        .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
        .unwrap_or_else(std::env::temp_dir)
}

/// Thư mục gốc `Pictures` của người dùng.
pub fn pictures_dir() -> PathBuf {
    known_folder(&FOLDERID_Pictures)
        .or_else(|| std::env::var_os("USERPROFILE").map(|p| PathBuf::from(p).join("Pictures")))
        .unwrap_or_else(std::env::temp_dir)
}

/// `<appdata>\darkshot\config.json` với thư mục gốc truyền vào.
pub fn config_path_in(appdata: &Path) -> PathBuf {
    appdata.join("darkshot").join("config.json")
}

/// `%APPDATA%\darkshot\config.json`
pub fn config_path() -> PathBuf {
    config_path_in(&appdata_dir())
}

/// `Pictures\darkshot`
pub fn default_save_dir() -> PathBuf {
    pictures_dir().join("darkshot")
}

pub fn local_now() -> LocalTime {
    // SAFETY: GetLocalTime chỉ trả về giá trị.
    let t = unsafe { GetLocalTime() };
    LocalTime {
        year: t.wYear,
        month: t.wMonth as u8,
        day: t.wDay as u8,
        hour: t.wHour as u8,
        minute: t.wMinute as u8,
        second: t.wSecond as u8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_darkshot() {
        assert!(config_path().ends_with("darkshot/config.json"));
        assert!(default_save_dir().ends_with("darkshot"));
    }

    #[test]
    fn local_now_is_plausible() {
        let t = local_now();
        assert!(t.year >= 2026 && (1..=12).contains(&t.month));
    }
}
