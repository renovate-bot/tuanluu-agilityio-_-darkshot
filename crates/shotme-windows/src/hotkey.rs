//! Đăng ký phím tắt toàn cục.

use shotme_core::config::Hotkey;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT,
    MOD_SHIFT, MOD_WIN,
};

pub const HOTKEY_ID: i32 = 1;

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
}
