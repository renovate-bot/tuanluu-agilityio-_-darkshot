//! Bảng màu giao diện tối và công thức tương phản WCAG 2.x. Chỉ là số, không phụ thuộc hệ điều hành.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    /// `0xRRGGBB`.
    pub const fn hex(v: u32) -> Rgb {
        let _ = v;
        Rgb(0, 0, 0)
    }

    /// `0x00BBGGRR` cho GDI.
    pub const fn colorref(self) -> u32 {
        0
    }
}

fn linear(c: u8) -> f64 {
    let s = f64::from(c) / 255.0;
    if s <= 0.03928 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

/// Độ sáng tương đối theo WCAG 2.x.
pub fn relative_luminance(c: Rgb) -> f64 {
    0.2126 * linear(c.0) + 0.7152 * linear(c.1) + 0.0722 * linear(c.2)
}

/// Tỉ lệ tương phản `(L1 + 0.05) / (L2 + 0.05)` với `L1 >= L2`.
pub fn contrast_ratio(a: Rgb, b: Rgb) -> f64 {
    let _ = (a, b);
    todo!()
}

pub const WHITE: Rgb = Rgb::hex(0xFFFFFF);
pub const BG: Rgb = Rgb::hex(0x1C1E22);
pub const SURFACE: Rgb = Rgb::hex(0x24272C);
pub const INPUT: Rgb = Rgb::hex(0x15171A);
pub const LINE: Rgb = Rgb::hex(0x383C43);
pub const INPUT_BORDER: Rgb = Rgb::hex(0x4A4F57);
pub const BUTTON: Rgb = Rgb::hex(0x33373E);
pub const BUTTON_HOVER: Rgb = Rgb::hex(0x3C4048);
pub const BUTTON_PRESSED: Rgb = Rgb::hex(0x2B2E33);
pub const BUTTON_BORDER: Rgb = Rgb::hex(0x454A52);
pub const TAB_HOVER: Rgb = Rgb::hex(0x2E3238);
pub const CHECK_BORDER: Rgb = Rgb::hex(0x8A9099);
pub const TRACK: Rgb = Rgb::hex(0x454A52);
pub const TEXT: Rgb = Rgb::hex(0xE7E9EC);
pub const TEXT_MUTED: Rgb = Rgb::hex(0xA3A9B2);
pub const TEXT_DISABLED: Rgb = Rgb::hex(0x7C828B);
pub const ACCENT: Rgb = Rgb::hex(0x4C8DFF);
pub const ACCENT_BUTTON: Rgb = Rgb::hex(0x2F6FE4);
pub const ACCENT_BUTTON_HOVER: Rgb = Rgb::hex(0x2563EB);
pub const ACCENT_BUTTON_PRESSED: Rgb = Rgb::hex(0x1D4ED8);
pub const DANGER: Rgb = Rgb::hex(0xFF7B72);
pub const SUCCESS: Rgb = Rgb::hex(0x4ADE80);
pub const OVERLAY_BORDER: Rgb = Rgb::hex(0x3F434A);
pub const OVERLAY_ICON: Rgb = Rgb::hex(0xD5D9DF);
pub const OVERLAY_HOVER: Rgb = Rgb::hex(0x363A41);
pub const OVERLAY_LABEL: Rgb = Rgb::hex(0x16181B);
pub const OVERLAY_TIP: Rgb = Rgb::hex(0x2B2D31);

/// (chữ, nền) đang dùng; mỗi cặp phải đạt 4.5:1.
pub const TEXT_PAIRS: &[(Rgb, Rgb)] = &[
    (TEXT, BG),
    (TEXT, SURFACE),
    (TEXT, INPUT),
    (TEXT, BUTTON),
    (TEXT, BUTTON_HOVER),
    (TEXT, BUTTON_PRESSED),
    (TEXT, TAB_HOVER),
    (TEXT, OVERLAY_LABEL),
    (TEXT, OVERLAY_TIP),
    (TEXT_MUTED, BG),
    (TEXT_MUTED, SURFACE),
    (TEXT_MUTED, INPUT),
    (DANGER, BG),
    (DANGER, SURFACE),
    (SUCCESS, BG),
    (SUCCESS, SURFACE),
    (WHITE, ACCENT_BUTTON),
    (WHITE, ACCENT_BUTTON_HOVER),
    (WHITE, ACCENT_BUTTON_PRESSED),
];

/// (thành phần, nền) đang dùng; mỗi cặp phải đạt 3:1.
pub const UI_PAIRS: &[(Rgb, Rgb)] = &[
    (ACCENT, BG),
    (ACCENT, SURFACE),
    (ACCENT, INPUT),
    (ACCENT, OVERLAY_HOVER),
    (CHECK_BORDER, SURFACE),
    (OVERLAY_ICON, SURFACE),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contrast_matches_wcag_examples() {
        assert!((contrast_ratio(WHITE, Rgb(0, 0, 0)) - 21.0).abs() < 0.01);
        assert!((contrast_ratio(Rgb::hex(0x777777), WHITE) - 4.48).abs() < 0.01);
        let (a, b) = (Rgb::hex(0x123456), Rgb::hex(0xABCDEF));
        assert_eq!(contrast_ratio(a, b), contrast_ratio(b, a));
    }

    #[test]
    fn palette_values_match_plan() {
        assert_eq!(TEXT, Rgb::hex(0xE7E9EC));
        assert_eq!(BG, Rgb::hex(0x1C1E22));
        assert_eq!(ACCENT, Rgb::hex(0x4C8DFF));
        assert_eq!(ACCENT_BUTTON_HOVER, Rgb::hex(0x2563EB));
        assert_eq!(DANGER, Rgb::hex(0xFF7B72));
    }

    #[test]
    fn colorref_is_bgr() {
        assert_eq!(Rgb::hex(0x112233).colorref(), 0x0033_2211);
    }

    #[test]
    fn text_pairs_meet_aa() {
        for (fg, bg) in TEXT_PAIRS {
            let r = contrast_ratio(*fg, *bg);
            assert!(r >= 4.5, "{fg:?} on {bg:?} = {r:.2}");
        }
    }

    #[test]
    fn ui_pairs_meet_non_text_contrast() {
        for (fg, bg) in UI_PAIRS {
            let r = contrast_ratio(*fg, *bg);
            assert!(r >= 3.0, "{fg:?} on {bg:?} = {r:.2}");
        }
    }
}
