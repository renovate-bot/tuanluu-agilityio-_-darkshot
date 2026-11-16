//! Khởi động cùng Windows qua `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.

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

fn is_enabled_as(_name: &str) -> bool {
    todo!()
}

fn set_enabled_as(_name: &str, _on: bool) -> Result<(), String> {
    todo!()
}

/// Nội dung kiểu chuỗi của một giá trị trong khóa Run, `None` nếu không có.
fn read_value(_name: &str) -> Option<String> {
    todo!()
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
