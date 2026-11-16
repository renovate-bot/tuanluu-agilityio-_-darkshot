//! Ngôn ngữ hiện hành của lớp Windows; đọc được từ mọi luồng.

use shotme_core::i18n::{Lang, Msg};

pub fn current() -> Lang {
    todo!()
}

pub fn set(_lang: Lang) {
    todo!()
}

pub fn tr(_m: Msg) -> &'static str {
    todo!()
}

/// Ngôn ngữ chính (`langid & 0x3FF`) bằng 0x2A là tiếng Việt; còn lại là English.
pub fn lang_from_langid(_langid: u16) -> Lang {
    todo!()
}

pub fn os_default() -> Lang {
    todo!()
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
