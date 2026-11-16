//! Ngôn ngữ hiện hành của lớp Windows; đọc được từ mọi luồng.

use shotme_core::i18n::{t, with_error_code, Lang, Msg};
use std::sync::atomic::{AtomicU8, Ordering};

static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn current() -> Lang {
    match CURRENT.load(Ordering::Relaxed) {
        1 => Lang::En,
        _ => Lang::Vi,
    }
}

pub fn set(lang: Lang) {
    let v = match lang {
        Lang::Vi => 0,
        Lang::En => 1,
    };
    CURRENT.store(v, Ordering::Relaxed);
}

pub fn tr(m: Msg) -> &'static str {
    t(current(), m)
}

/// Thông báo kèm mã lỗi hệ điều hành, theo ngôn ngữ hiện hành.
pub fn tr_code(m: Msg, code: &str) -> String {
    with_error_code(current(), tr(m), code)
}

/// Ngôn ngữ chính (`langid & 0x3FF`) bằng 0x2A là tiếng Việt; còn lại là English.
pub fn lang_from_langid(langid: u16) -> Lang {
    if langid & 0x3FF == 0x2A {
        Lang::Vi
    } else {
        Lang::En
    }
}

pub fn os_default() -> Lang {
    // SAFETY: hàm Win32 không tham số, không có điều kiện tiên quyết.
    let id = unsafe { windows::Win32::Globalization::GetUserDefaultUILanguage() };
    lang_from_langid(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_langid_maps_to_lang() {
        assert_eq!(lang_from_langid(0x042A), Lang::Vi);
        assert_eq!(lang_from_langid(0x002A), Lang::Vi);
        assert_eq!(lang_from_langid(0x0409), Lang::En);
        assert_eq!(lang_from_langid(0x0411), Lang::En);
        assert_eq!(lang_from_langid(0), Lang::En);
    }

    // Kiểm thử duy nhất đụng biến toàn cục; đặt lại Vi ở cuối.
    #[test]
    fn tr_follows_current_language() {
        set(Lang::En);
        assert_eq!(current(), Lang::En);
        assert_eq!(tr(Msg::TrayCapture), "Take screenshot");
        set(Lang::Vi);
        assert_eq!(tr(Msg::TrayCapture), "Chụp màn hình");
    }
}
