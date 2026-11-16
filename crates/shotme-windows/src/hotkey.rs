//! Đăng ký phím tắt toàn cục.

use shotme_core::config::{Hotkey, HotkeyRole, Hotkeys};
use shotme_core::i18n::HelpKeys;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT,
    MOD_SHIFT, MOD_WIN,
};

pub const HOTKEY_ID: i32 = 1;

/// Mã đăng ký với Windows của một dòng: Region = 1, SaveFull = 2, UploadFull = 3.
pub fn id_of(role: HotkeyRole) -> i32 {
    let _ = role;
    todo!()
}

/// Dòng ứng với mã `id` của `WM_HOTKEY`.
pub fn role_of(id: i32) -> Option<HotkeyRole> {
    let _ = id;
    todo!()
}

/// Hủy đăng ký cả ba dòng.
#[allow(dead_code)]
pub fn unregister_all(hwnd: HWND) {
    let _ = hwnd;
    todo!()
}

/// Đăng ký các dòng đang bật theo `HotkeyRole::ALL`; trả về các dòng không đăng ký được.
#[allow(dead_code)]
pub fn register_enabled(hwnd: HWND, keys: &Hotkeys) -> Vec<HotkeyRole> {
    let _ = (hwnd, keys);
    todo!()
}

/// Tổ hợp của các dòng đang bật cho Trợ giúp.
pub fn help_keys(keys: &Hotkeys) -> HelpKeys {
    let _ = keys;
    todo!()
}

/// Tên tổ hợp của các dòng `failed`, theo thứ tự `HotkeyRole::ALL`.
pub fn taken_combos(keys: &Hotkeys, failed: &[HotkeyRole]) -> Vec<String> {
    let _ = (keys, failed);
    todo!()
}

/// RegisterHotKey với MOD_NOREPEAT. Trả về `false` khi phím đang bị ứng dụng khác chiếm.
pub fn register(hwnd: HWND, hk: &Hotkey) -> bool {
    let mut mods: HOT_KEY_MODIFIERS = MOD_NOREPEAT;
    if hk.ctrl {
        mods |= MOD_CONTROL;
    }
    if hk.alt {
        mods |= MOD_ALT;
    }
    if hk.shift {
        mods |= MOD_SHIFT;
    }
    if hk.win {
        mods |= MOD_WIN;
    }
    // SAFETY: hwnd là cửa sổ của luồng gọi; các tham số là giá trị thuần.
    unsafe { RegisterHotKey(Some(hwnd), HOTKEY_ID, mods, hk.vk).is_ok() }
}

pub fn unregister(hwnd: HWND) {
    // SAFETY: hủy phím đã đăng ký cho cửa sổ này; lỗi (chưa đăng ký) được bỏ qua.
    unsafe {
        let _ = UnregisterHotKey(Some(hwnd), HOTKEY_ID);
    }
}

fn key_name(vk: u32) -> String {
    match vk {
        0x2C => "PrintScreen".into(),
        0x08 => "Backspace".into(),
        0x09 => "Tab".into(),
        0x0D => "Enter".into(),
        0x13 => "Pause".into(),
        0x1B => "Esc".into(),
        0x20 => "Space".into(),
        0x21 => "PageUp".into(),
        0x22 => "PageDown".into(),
        0x23 => "End".into(),
        0x24 => "Home".into(),
        0x25 => "Left".into(),
        0x26 => "Up".into(),
        0x27 => "Right".into(),
        0x28 => "Down".into(),
        0x2D => "Insert".into(),
        0x2E => "Delete".into(),
        0x30..=0x39 | 0x41..=0x5A => char::from(vk as u8).to_string(),
        0x70..=0x87 => format!("F{}", vk - 0x70 + 1),
        _ => format!("0x{vk:02X}"),
    }
}

/// "PrintScreen", "Ctrl+Shift+S"...
pub fn describe(hk: &Hotkey) -> String {
    let mut s = String::new();
    for (on, name) in [
        (hk.ctrl, "Ctrl"),
        (hk.alt, "Alt"),
        (hk.shift, "Shift"),
        (hk.win, "Win"),
    ] {
        if on {
            s.push_str(name);
            s.push('+');
        }
    }
    s.push_str(&key_name(hk.vk));
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describe_hotkey() {
        assert_eq!(
            describe(&Hotkey {
                ctrl: false,
                alt: false,
                shift: false,
                win: false,
                vk: 0x2C
            }),
            "PrintScreen"
        );
        assert_eq!(
            describe(&Hotkey {
                ctrl: true,
                alt: false,
                shift: true,
                win: false,
                vk: 0x53
            }),
            "Ctrl+Shift+S"
        );
        assert_eq!(
            describe(&Hotkey {
                ctrl: false,
                alt: true,
                shift: false,
                win: false,
                vk: 0x71
            }),
            "Alt+F2"
        );
    }

    #[test]
    fn ids_round_trip() {
        assert_eq!(id_of(HotkeyRole::Region), 1);
        let ids: Vec<i32> = HotkeyRole::ALL.iter().map(|r| id_of(*r)).collect();
        assert_eq!(ids, vec![1, 2, 3]);
        for r in HotkeyRole::ALL {
            assert_eq!(role_of(id_of(r)), Some(r));
        }
        assert_eq!(role_of(0), None);
        assert_eq!(role_of(4), None);
    }

    #[test]
    fn help_keys_skip_disabled_rows() {
        let mut h = Hotkeys::default();
        assert_eq!(
            help_keys(&h),
            HelpKeys {
                region: Some("PrintScreen".into()),
                save_full: None,
                upload_full: None
            }
        );
        h.upload_full.enabled = true;
        h.region.enabled = false;
        assert_eq!(
            help_keys(&h),
            HelpKeys {
                region: None,
                save_full: None,
                upload_full: Some("Ctrl+PrintScreen".into())
            }
        );
    }

    #[test]
    fn taken_combos_follow_row_order() {
        let h = Hotkeys::default();
        assert_eq!(
            taken_combos(&h, &[HotkeyRole::UploadFull, HotkeyRole::Region]),
            vec!["PrintScreen".to_string(), "Ctrl+PrintScreen".to_string()]
        );
        assert!(taken_combos(&h, &[]).is_empty());
    }
}
