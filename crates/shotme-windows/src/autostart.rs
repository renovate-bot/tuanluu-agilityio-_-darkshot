//! Khởi động cùng Windows qua `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.

use crate::lang;
use shotme_core::i18n::Msg;
use std::path::Path;
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegGetValueW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
    RRF_RT_REG_SZ,
};

/// Khóa Run của người dùng hiện tại (dưới `HKEY_CURRENT_USER`).
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
/// Tên giá trị thật của ứng dụng.
const VALUE_NAME: &str = "shotme";

/// Đúng khi giá trị `shotme` có trong khóa Run.
pub fn is_enabled() -> bool {
    is_enabled_as(VALUE_NAME)
}

/// Bật: ghi đường dẫn file chạy hiện tại (trong dấu nháy kép). Tắt: xóa giá trị.
pub fn set_enabled(on: bool) -> Result<(), String> {
    set_enabled_as(VALUE_NAME, on)
}

fn is_enabled_as(name: &str) -> bool {
    read_value(name).is_some()
}

fn set_enabled_as(name: &str, on: bool) -> Result<(), String> {
    let key = RunKey::open()?;
    if on {
        let exe = std::env::current_exe()
            .map_err(|e| format!("{}: {e}", lang::tr(Msg::AutostartExePath)))?;
        let data: Vec<u8> = launch_command(&exe)
            .encode_utf16()
            .chain(std::iter::once(0))
            .flat_map(u16::to_le_bytes)
            .collect();
        // SAFETY: khóa đang mở với quyền ghi; dữ liệu REG_SZ kết thúc bằng NUL.
        let r = unsafe { RegSetValueExW(key.0, &HSTRING::from(name), None, REG_SZ, Some(&data)) };
        if r != ERROR_SUCCESS {
            return Err(lang::tr_code(Msg::AutostartEnableFailed, &r.0.to_string()));
        }
    } else {
        // SAFETY: khóa đang mở với quyền ghi.
        let r = unsafe { RegDeleteValueW(key.0, &HSTRING::from(name)) };
        if r != ERROR_SUCCESS && r != ERROR_FILE_NOT_FOUND {
            return Err(lang::tr_code(Msg::AutostartDisableFailed, &r.0.to_string()));
        }
    }
    Ok(())
}

/// Dòng lệnh ghi vào khóa Run: đường dẫn trong dấu nháy kép (đường dẫn có thể chứa dấu cách).
fn launch_command(exe: &Path) -> String {
    format!("\"{}\"", exe.display())
}

/// Nội dung kiểu chuỗi của một giá trị trong khóa Run, `None` nếu không có.
fn read_value(name: &str) -> Option<String> {
    let sub = HSTRING::from(RUN_KEY);
    let value = HSTRING::from(name);
    let mut size = 0u32;
    // SAFETY: chỉ hỏi kích thước (byte) của giá trị, không có bộ đệm.
    let r = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            &sub,
            &value,
            RRF_RT_REG_SZ,
            None,
            None,
            Some(&mut size),
        )
    };
    if r != ERROR_SUCCESS {
        return None;
    }
    // Dư một ô phòng giá trị bị đổi giữa hai lần gọi; hàm sẽ báo ERROR_MORE_DATA nếu vẫn thiếu.
    let mut buf = vec![0u16; (size as usize).div_ceil(2) + 1];
    let mut bytes = u32::try_from(buf.len() * 2).ok()?;
    // SAFETY: bộ đệm dài `bytes` byte, còn sống suốt lời gọi.
    let r = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            &sub,
            &value,
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr().cast()),
            Some(&mut bytes),
        )
    };
    if r != ERROR_SUCCESS {
        return None;
    }
    let len = buf.iter().position(|&u| u == 0).unwrap_or(buf.len());
    String::from_utf16(buf.get(..len)?).ok()
}

/// Khóa Run đã mở; đóng khi drop.
struct RunKey(HKEY);

impl RunKey {
    fn open() -> Result<Self, String> {
        let mut key = HKEY::default();
        // SAFETY: mở (hoặc tạo nếu thiếu) khóa con của HKCU; handle đóng trong Drop.
        let r = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                &HSTRING::from(RUN_KEY),
                None,
                PCWSTR::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_QUERY_VALUE | KEY_SET_VALUE,
                None,
                &mut key,
                None,
            )
        };
        if r != ERROR_SUCCESS {
            return Err(lang::tr_code(Msg::AutostartKeyFailed, &r.0.to_string()));
        }
        Ok(RunKey(key))
    }
}

impl Drop for RunKey {
    fn drop(&mut self) {
        // SAFETY: handle mở thành công trong `open`, chỉ đóng một lần.
        unsafe {
            let _ = RegCloseKey(self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::core::HSTRING;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegDeleteValueW, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE,
    };

    /// Xóa giá trị kiểm thử khi ra khỏi phạm vi, kể cả khi khẳng định thất bại.
    /// Gọi thẳng Win32 để không phụ thuộc mã đang kiểm thử.
    struct Cleanup(String);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let mut key = HKEY::default();
            // SAFETY: mở khóa Run của người dùng với quyền ghi; đóng ngay sau khi xóa giá trị.
            unsafe {
                if RegOpenKeyExW(
                    HKEY_CURRENT_USER,
                    &HSTRING::from(RUN_KEY),
                    None,
                    KEY_SET_VALUE,
                    &mut key,
                )
                .is_ok()
                {
                    let _ = RegDeleteValueW(key, &HSTRING::from(self.0.as_str()));
                    let _ = RegCloseKey(key);
                }
            }
        }
    }

    #[test]
    fn launch_command_quotes_path_with_spaces() {
        assert_eq!(
            launch_command(Path::new(r"D:\2026 - Tuan\shotme\shotme.exe")),
            "\"D:\\2026 - Tuan\\shotme\\shotme.exe\""
        );
    }

    #[test]
    fn autostart_round_trip_with_test_value_name() {
        let name = format!("shotme-test-{}", std::process::id());
        assert_ne!(name, VALUE_NAME);
        let _cleanup = Cleanup(name.clone());

        set_enabled_as(&name, true).unwrap();
        assert!(is_enabled_as(&name));
        let v = read_value(&name).expect("giá trị vừa ghi");
        assert!(v.len() > 2 && v.starts_with('"') && v.ends_with('"'), "{v}");
        let exe = std::env::current_exe().unwrap();
        assert_eq!(v, format!("\"{}\"", exe.display()));

        set_enabled_as(&name, false).unwrap();
        assert!(!is_enabled_as(&name));
        assert_eq!(read_value(&name), None);
        // Tắt lần nữa khi đã không còn giá trị vẫn thành công.
        set_enabled_as(&name, false).unwrap();
    }
}
