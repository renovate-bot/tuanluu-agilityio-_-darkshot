use shotme_core::config::Hotkey;
use windows::Win32::Foundation::HWND;

pub const HOTKEY_ID: i32 = 1;

/// RegisterHotKey với MOD_NOREPEAT.
pub fn register(_hwnd: HWND, _hk: &Hotkey) -> bool {
    todo!()
}

pub fn unregister(_hwnd: HWND) {
    todo!()
}

pub fn describe(_hk: &Hotkey) -> String {
    todo!()
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
