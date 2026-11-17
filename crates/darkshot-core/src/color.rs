//! Mô hình màu của bảng chọn màu tự chọn: HLS theo kiểu Windows, 16 ô màu tự chọn, mã hex
//! và bố cục bảng. Chỉ là số, không phụ thuộc hệ điều hành.

use crate::geometry::Rect;
use crate::theme::Rgb;

pub const HLS_MAX: u16 = 240;
/// Giá trị Hue lớn nhất khi hiển thị (240 trùng với 0).
pub const HUE_MAX: u16 = 239;
pub const RGB_MAX: u16 = 255;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hls {
    pub h: u16,
    pub l: u16,
    pub s: u16,
}

/// Theo `ColorRGBToHLS` của Windows (số nguyên, `HLSMAX = 240`, `RGBMAX = 255`).
pub fn rgb_to_hls(c: Rgb) -> Hls {
    const HM: i32 = HLS_MAX as i32;
    const RM: i32 = RGB_MAX as i32;
    let (r, g, b) = (i32::from(c.0), i32::from(c.1), i32::from(c.2));
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let sum = max + min;
    let diff = max - min;
    let l = (sum * HM + RM) / (2 * RM);
    let (h, s) = if diff == 0 {
        (HM * 2 / 3, 0)
    } else {
        let s = if l <= HM / 2 {
            (diff * HM + sum / 2) / sum
        } else {
            (diff * HM + (2 * RM - sum) / 2) / (2 * RM - sum)
        };
        let delta = |v: i32| ((max - v) * (HM / 6) + diff / 2) / diff;
        let (rd, gd, bd) = (delta(r), delta(g), delta(b));
        let mut h = if r == max {
            bd - gd
        } else if g == max {
            HM / 3 + rd - bd
        } else {
            HM * 2 / 3 + gd - rd
        };
        if h < 0 {
            h += HM;
        }
        if h >= HM {
            h -= HM;
        }
        (h, s)
    };
    Hls {
        h: h as u16,
        l: l as u16,
        s: s as u16,
    }
}

/// Theo `ColorHLSToRGB` của Windows.
pub fn hls_to_rgb(v: Hls) -> Rgb {
    const HM: i32 = HLS_MAX as i32;
    const RM: i32 = RGB_MAX as i32;
    let h = i32::from(v.h.min(HLS_MAX));
    let l = i32::from(v.l.min(HLS_MAX));
    let s = i32::from(v.s.min(HLS_MAX));
    if s == 0 {
        let g = (l * RM / HM) as u8;
        return Rgb(g, g, g);
    }
    let m2 = if l <= HM / 2 {
        (l * (HM + s) + HM / 2) / HM
    } else {
        l + s - (l * s + HM / 2) / HM
    };
    let m1 = 2 * l - m2;
    let hue_to = |mut hue: i32| -> i32 {
        if hue < 0 {
            hue += HM;
        }
        if hue > HM {
            hue -= HM;
        }
        if hue < HM / 6 {
            m1 + ((m2 - m1) * hue + HM / 12) / (HM / 6)
        } else if hue < HM / 2 {
            m2
        } else if hue < HM * 2 / 3 {
            m1 + ((m2 - m1) * (HM * 2 / 3 - hue) + HM / 12) / (HM / 6)
        } else {
            m1
        }
    };
    let ch = |hue: i32| ((hue_to(hue) * RM + HM / 2) / HM).clamp(0, RM) as u8;
    Rgb(ch(h + HM / 3), ch(h), ch(h - HM / 3))
}

pub const CUSTOM_SLOTS: usize = 16;

#[derive(Clone, Copy, Debug, Default)]
pub struct CustomColors {
    pub slots: [Option<Rgb>; CUSTOM_SLOTS],
    next_overwrite: usize,
}

impl PartialEq for CustomColors {
    fn eq(&self, other: &Self) -> bool {
        self.slots == other.slots
    }
}
impl Eq for CustomColors {}

impl CustomColors {
    pub fn from_slots(slots: [Option<Rgb>; CUSTOM_SLOTS]) -> CustomColors {
        CustomColors {
            slots,
            next_overwrite: 0,
        }
    }

    /// Ghi `c` vào `target` nếu có, không thì ô trống đầu tiên, hết ô trống thì ô `next_overwrite`
    /// (vòng tròn, bắt đầu 0, tăng sau mỗi lần ghi đè). Trả chỉ số đã ghi.
    pub fn add(&mut self, c: Rgb, target: Option<usize>) -> usize {
        let idx = match target {
            Some(i) if i < CUSTOM_SLOTS => i,
            _ => match self.slots.iter().position(|s| s.is_none()) {
                Some(i) => i,
                None => {
                    let i = self.next_overwrite % CUSTOM_SLOTS;
                    self.next_overwrite = (i + 1) % CUSTOM_SLOTS;
                    i
                }
            },
        };
        self.slots[idx] = Some(c);
        idx
    }
}

/// "#RRGGBB" (hoa hoặc thường), khác thì `None`.
pub fn parse_hex(s: &str) -> Option<Rgb> {
    let d = s.strip_prefix('#')?;
    if d.len() != 6 || !d.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let v = u32::from_str_radix(d, 16).ok()?;
    Some(Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

/// "#RRGGBB" chữ hoa.
pub fn to_hex(c: Rgb) -> String {
    format!("#{:02X}{:02X}{:02X}", c.0, c.1, c.2)
}

/// Bố cục bảng chọn màu (tọa độ tương đối góc trên trái bảng, pixel).
pub struct PickerLayout {
    pub size: (i32, i32),
    pub basic_label: Rect,
    pub basic: [Rect; 48],
    pub custom_label: Rect,
    pub custom: [Rect; CUSTOM_SLOTS],
    pub define_btn: Rect,
    // chỉ có nghĩa khi expanded (dạng gọn: các ô rỗng, w = h = 0):
    pub spectrum: Rect,
    pub lum_bar: Rect,
    pub lum_arrow_x: i32,
    pub preview: Rect,
    /// (nhãn, ô) theo thứ tự Hue, Sat, Lum, Red, Green, Blue.
    pub fields: [(Rect, Rect); 6],
    pub add_btn: Rect,
    pub ok_btn: Rect,
    pub cancel_btn: Rect,
}

/// `dpi` / 96 là hệ số co giãn.
pub fn picker_layout(dpi: u32, expanded: bool) -> PickerLayout {
    let s = |v: i32| (v * dpi as i32 + 48) / 96;
    let r = |x: i32, y: i32, w: i32, h: i32| Rect {
        x: s(x),
        y: s(y),
        w: s(w),
        h: s(h),
    };
    let none = Rect {
        x: 0,
        y: 0,
        w: 0,
        h: 0,
    };
    let basic = std::array::from_fn(|i| {
        let (col, row) = ((i % 8) as i32, (i / 8) as i32);
        r(8 + col * 24, 30 + row * 24, 20, 20)
    });
    let custom = std::array::from_fn(|i| {
        let (col, row) = ((i % 8) as i32, (i / 8) as i32);
        r(8 + col * 24, 200 + row * 24, 20, 20)
    });
    let mut l = PickerLayout {
        size: (s(204), s(286)),
        basic_label: r(8, 8, 188, 18),
        basic,
        custom_label: r(8, 178, 188, 18),
        custom,
        define_btn: r(8, 252, 188, 26),
        spectrum: none,
        lum_bar: none,
        lum_arrow_x: 0,
        preview: none,
        fields: [(none, none); 6],
        add_btn: none,
        ok_btn: none,
        cancel_btn: none,
    };
    if expanded {
        l.size = (s(456), s(320));
        l.spectrum = r(212, 8, 200, 180);
        l.lum_bar = r(420, 8, 14, 180);
        l.lum_arrow_x = s(434);
        l.preview = r(212, 196, 56, 40);
        for (i, f) in l.fields.iter_mut().enumerate() {
            let (col, row) = ((i / 3) as i32, (i % 3) as i32);
            let x = 276 + col * 88;
            let y = 196 + row * 26;
            *f = (r(x, y, 48, 22), r(x + 48, y, 36, 22));
        }
        l.add_btn = r(212, 286, 222, 26);
        l.ok_btn = r(8, 286, 92, 26);
        l.cancel_btn = r(104, 286, 92, 26);
    }
    l
}

/// Điểm (kẹp vào ô) -> (hue 0..=239, sat 240..=0).
pub fn spectrum_hs(spectrum: Rect, x: i32, y: i32) -> (u16, u16) {
    let w = (spectrum.w - 1).max(1);
    let h = (spectrum.h - 1).max(1);
    let dx = (x - spectrum.x).clamp(0, w);
    let dy = (y - spectrum.y).clamp(0, h);
    let hue = (dx * i32::from(HUE_MAX) + w / 2) / w;
    let sat = i32::from(HLS_MAX) - (dy * i32::from(HLS_MAX) + h / 2) / h;
    (hue as u16, sat as u16)
}

pub fn spectrum_point(spectrum: Rect, h: u16, s: u16) -> (i32, i32) {
    let w = (spectrum.w - 1).max(1);
    let hh = (spectrum.h - 1).max(1);
    let h = i32::from(h.min(HUE_MAX));
    let s = i32::from(s.min(HLS_MAX));
    let x = (h * w + i32::from(HUE_MAX) / 2) / i32::from(HUE_MAX);
    let y = ((i32::from(HLS_MAX) - s) * hh + i32::from(HLS_MAX) / 2) / i32::from(HLS_MAX);
    (spectrum.x + x, spectrum.y + y)
}

/// Trên = 240, dưới = 0, kẹp.
pub fn lum_at(bar: Rect, y: i32) -> u16 {
    let h = (bar.h - 1).max(1);
    let dy = (y - bar.y).clamp(0, h);
    (i32::from(HLS_MAX) - (dy * i32::from(HLS_MAX) + h / 2) / h) as u16
}

pub fn lum_y(bar: Rect, l: u16) -> i32 {
    let h = (bar.h - 1).max(1);
    let l = i32::from(l.min(HLS_MAX));
    bar.y + ((i32::from(HLS_MAX) - l) * h + i32::from(HLS_MAX) / 2) / i32::from(HLS_MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotate::PALETTE;

    fn hls(h: u16, l: u16, s: u16) -> Hls {
        Hls { h, l, s }
    }

    #[test]
    fn windows_hls_reference() {
        assert_eq!(rgb_to_hls(Rgb(255, 0, 0)), hls(0, 120, 240));
        assert_eq!(rgb_to_hls(Rgb(0, 255, 0)), hls(80, 120, 240));
        assert_eq!(rgb_to_hls(Rgb(0, 0, 255)), hls(160, 120, 240));
        assert_eq!(rgb_to_hls(Rgb(255, 255, 255)), hls(160, 240, 0));
        assert_eq!(rgb_to_hls(Rgb(0, 0, 0)), hls(160, 0, 0));
        let g = rgb_to_hls(Rgb(128, 128, 128));
        assert_eq!((g.h, g.s), (160, 0));
        assert!(g.l.abs_diff(120) <= 1, "{g:?}");
        assert_eq!(rgb_to_hls(Rgb(255, 255, 0)), hls(40, 120, 240));
        // (0x80,0x40,0x40) tính tay theo KB29240: cMax=128, cMin=64.
        // L = ((192*240)+255)/510 = 90; L <= 120 nên S = (64*240 + 96)/192 = 80;
        // R là cMax: H = Bdelta - Gdelta = 40 - 40 = 0.
        assert_eq!(rgb_to_hls(Rgb(0x80, 0x40, 0x40)), hls(0, 90, 80));
    }

    fn lcg(seed: &mut u64) -> u8 {
        *seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (*seed >> 33) as u8
    }

    // Hue chỉ có 240 bậc, mỗi bậc đổi một kênh tới ~6,4 đơn vị RGB (255*6/240), nên đi rồi về
    // lệch tới ~4; đo được khớp thuật toán KB29240 của Windows.
    #[test]
    fn hls_round_trip() {
        let mut colors: Vec<Rgb> = PALETTE.to_vec();
        let mut seed = 0x1234_5678_9abc_def0u64;
        for _ in 0..1000 {
            colors.push(Rgb(lcg(&mut seed), lcg(&mut seed), lcg(&mut seed)));
        }
        for c in colors {
            let b = hls_to_rgb(rgb_to_hls(c));
            assert!(c.0.abs_diff(b.0) <= 4, "{c:?} -> {b:?}");
            assert!(c.1.abs_diff(b.1) <= 4, "{c:?} -> {b:?}");
            assert!(c.2.abs_diff(b.2) <= 4, "{c:?} -> {b:?}");
        }
    }

    #[test]
    fn hls_to_rgb_reference() {
        assert_eq!(hls_to_rgb(hls(0, 120, 240)), Rgb(255, 0, 0));
        assert_eq!(hls_to_rgb(hls(160, 240, 0)), Rgb(255, 255, 255));
        let g = hls_to_rgb(hls(80, 60, 240));
        assert!(g.0 <= 1 && g.2 <= 1 && g.1.abs_diff(128) <= 1, "{g:?}");
    }

    #[test]
    fn custom_add_rules() {
        let c = |n: u8| Rgb(n, 0, 0);
        let mut cc = CustomColors::default();
        assert_eq!(cc.add(c(1), Some(5)), 5);
        assert_eq!(cc.slots[5], Some(c(1)));
        assert_eq!(cc.add(c(2), None), 0);
        assert_eq!(cc.add(c(3), None), 1);
        // Đích đang có màu thì bị ghi đè.
        assert_eq!(cc.add(c(4), Some(5)), 5);
        assert_eq!(cc.slots[5], Some(c(4)));
        // Làm đầy rồi ghi đè vòng tròn.
        let mut full = CustomColors::default();
        for i in 0..CUSTOM_SLOTS {
            assert_eq!(full.add(c(i as u8), None), i);
        }
        assert_eq!(full.add(c(100), None), 0);
        assert_eq!(full.add(c(101), None), 1);
        for k in 2..CUSTOM_SLOTS {
            assert_eq!(full.add(c(102), None), k);
        }
        assert_eq!(full.add(c(103), None), 0);
        assert_eq!(full.slots[0], Some(c(103)));
    }

    #[test]
    fn hex_parse_format() {
        assert_eq!(parse_hex("#ff3b30"), Some(Rgb(0xFF, 0x3B, 0x30)));
        assert_eq!(parse_hex("#FF3B30"), Some(Rgb(0xFF, 0x3B, 0x30)));
        for bad in ["FF3B30", "#FFF", "#GG0000", "", "#FF3B300", "#+F3B30"] {
            assert_eq!(parse_hex(bad), None, "{bad:?}");
        }
        assert_eq!(to_hex(Rgb(0xff, 0x3b, 0x0a)), "#FF3B0A");
    }

    fn overlap(a: Rect, b: Rect) -> bool {
        a.w > 0
            && a.h > 0
            && b.w > 0
            && b.h > 0
            && a.x < b.x + b.w
            && b.x < a.x + a.w
            && a.y < b.y + b.h
            && b.y < a.y + a.h
    }

    fn inside(r: Rect, size: (i32, i32)) -> bool {
        r.x >= 0 && r.y >= 0 && r.w > 0 && r.h > 0 && r.x + r.w <= size.0 && r.y + r.h <= size.1
    }

    fn all_rects(l: &PickerLayout, expanded: bool) -> Vec<Rect> {
        let mut v = vec![l.basic_label, l.custom_label, l.define_btn];
        v.extend(l.basic);
        v.extend(l.custom);
        if expanded {
            v.extend([
                l.spectrum,
                l.lum_bar,
                l.preview,
                l.add_btn,
                l.ok_btn,
                l.cancel_btn,
            ]);
            for (a, b) in l.fields {
                v.push(a);
                v.push(b);
            }
        }
        v
    }

    #[test]
    fn picker_layout_fits_and_disjoint() {
        for dpi in [96u32, 144] {
            let compact = picker_layout(dpi, false);
            let full = picker_layout(dpi, true);
            for (l, ex) in [(&compact, false), (&full, true)] {
                let v = all_rects(l, ex);
                for (i, a) in v.iter().enumerate() {
                    assert!(
                        inside(*a, l.size),
                        "dpi {dpi} ex {ex} rect {i} {a:?} {:?}",
                        l.size
                    );
                    for (j, b) in v.iter().enumerate().skip(i + 1) {
                        assert!(
                            !overlap(*a, *b),
                            "dpi {dpi} ex {ex}: {i} {a:?} vs {j} {b:?}"
                        );
                    }
                }
            }
            assert!(full.size.0 > compact.size.0);
            assert!(full.size.1 >= compact.size.1);
            assert!(full.lum_arrow_x >= full.lum_bar.x + full.lum_bar.w);
            assert!(full.lum_arrow_x < full.size.0);
        }
        let l = picker_layout(96, true);
        assert_eq!((l.spectrum.w, l.spectrum.h), (200, 180));
        assert_eq!(l.basic[0].w, 20);
    }

    #[test]
    fn spectrum_mapping() {
        let sp = Rect {
            x: 10,
            y: 20,
            w: 200,
            h: 180,
        };
        assert_eq!(spectrum_hs(sp, 10, 20), (0, 240));
        assert_eq!(spectrum_hs(sp, 209, 199), (239, 0));
        assert_eq!(spectrum_hs(sp, -50, -50), (0, 240));
        assert_eq!(spectrum_hs(sp, 900, 900), (239, 0));
        for (x, y) in [(10, 20), (50, 77), (150, 120), (209, 199), (100, 100)] {
            let (h, s) = spectrum_hs(sp, x, y);
            let (bx, by) = spectrum_point(sp, h, s);
            assert!(
                (bx - x).abs() <= 1 && (by - y).abs() <= 1,
                "{x},{y} -> {bx},{by}"
            );
        }
        let bar = Rect {
            x: 0,
            y: 30,
            w: 14,
            h: 180,
        };
        assert_eq!(lum_at(bar, 30), 240);
        assert_eq!(lum_at(bar, 209), 0);
        assert_eq!(lum_at(bar, -100), 240);
        assert_eq!(lum_at(bar, 999), 0);
        for y in [30, 31, 80, 120, 209] {
            let back = lum_y(bar, lum_at(bar, y));
            assert!((back - y).abs() <= 1, "{y} -> {back}");
        }
    }
}
