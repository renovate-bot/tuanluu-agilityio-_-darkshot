//! Khung ảnh của cửa sổ "Chữ nhận dạng": hình học thuần (ảnh gốc → bản giữ → hiển thị, cuộn, dò
//! điểm), trạng thái chọn dòng thuần, và các hàm vẽ GDI nhỏ (DIB, khung dòng, nhãn "Đã chép").

use crate::history_paint::{self, Bmp};
use crate::theme;
use darkshot_core::geometry::Rect;
use darkshot_core::ocr_text::{compose, compose_all, ocr_scale, OcrLine, TextOptions};
use darkshot_core::theme::Rgb;
use std::ffi::c_void;
use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Gdi::{
    AlphaBlend, CreateCompatibleDC, CreateDIBSection, CreatePen, DeleteDC, DeleteObject,
    GetStockObject, Rectangle, SelectObject, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    BLENDFUNCTION, DIB_RGB_COLORS, HDC, HGDIOBJ, NULL_BRUSH, PS_DOT,
};

/// Ảnh có cạnh dài hơn mức này thì cửa sổ chỉ giữ bản đã co về cạnh dài này (thiết kế 7b.7).
pub const KEEP_MAX: u32 = 4096;

/// Cỡ bản giữ của ảnh `w`×`h`: cạnh dài > `KEEP_MAX` thì co giữ tỉ lệ, còn lại giữ nguyên.
pub fn kept_size(w: u32, h: u32) -> (u32, u32) {
    if w.max(h) > KEEP_MAX {
        ocr_scale(w, h, KEEP_MAX).unwrap_or((w, h))
    } else {
        (w, h)
    }
}

/// Hình học khung ảnh. Tọa độ "nội dung": gốc ở góc trên trái vùng cuộn (cuộn = 0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Geom {
    /// Cỡ ảnh gốc (khung dòng của OCR tính theo cỡ này).
    pub orig: (u32, u32),
    /// Cỡ bản giữ.
    pub kept: (u32, u32),
    /// Chỗ ảnh hiển thị trong tọa độ nội dung (w, h = cỡ hiển thị).
    pub img: Rect,
    /// Chiều cao nội dung (≥ chiều cao khung khi không cần cuộn).
    pub content_h: i32,
    /// Chiều cao khung nhìn.
    pub view_h: i32,
}

/// Hình học cho khung nhìn `vw`×`vh` px, đệm `pad` px: ảnh khớp bề rộng (không phóng quá 1:1),
/// căn giữa ngang; vừa khung thì căn giữa dọc, cao hơn thì đặt sát trên và cuộn dọc.
pub fn geom(orig: (u32, u32), kept: (u32, u32), vw: i32, vh: i32, pad: i32) -> Geom {
    let vh = vh.max(0);
    let (aw, ah) = ((vw - 2 * pad).max(0), (vh - 2 * pad).max(0));
    let (kw, kh) = (i64::from(kept.0), i64::from(kept.1));
    let empty = Geom {
        orig,
        kept,
        img: Rect {
            x: pad.max(0),
            y: pad.max(0),
            w: 0,
            h: 0,
        },
        content_h: vh,
        view_h: vh,
    };
    if kw == 0 || kh == 0 || aw == 0 || ah == 0 || orig.0 == 0 || orig.1 == 0 {
        return empty;
    }
    let (dw, dh) = if kw <= i64::from(aw) {
        (kw, kh)
    } else {
        let a = i64::from(aw);
        (a, ((kh * a + kw / 2) / kw).max(1))
    };
    // Bản giữ tối đa 4096 px mỗi cạnh nên vừa i32.
    let (dw, dh) = (dw as i32, dh as i32);
    let x = pad + (aw - dw) / 2;
    let (y, content_h) = if dh <= ah {
        (pad + (ah - dh) / 2, vh)
    } else {
        (pad, dh + 2 * pad)
    };
    Geom {
        img: Rect { x, y, w: dw, h: dh },
        content_h,
        ..empty
    }
}

impl Geom {
    /// Vị trí cuộn lớn nhất.
    pub fn max_scroll(&self) -> i32 {
        (self.content_h - self.view_h).max(0)
    }

    /// Khung dòng `r` (px ảnh gốc) trong tọa độ nội dung: gốc → bản giữ → hiển thị.
    pub fn line_rect(&self, r: Rect) -> Rect {
        use darkshot_core::ocr_text::map_rect;
        let (ow, oh) = self.orig;
        let (kw, kh) = self.kept;
        let k = map_rect(r, ow, oh, kw, kh);
        let (dw, dh) = (self.img.w.max(0) as u32, self.img.h.max(0) as u32);
        let d = map_rect(k, kw, kh, dw, dh);
        Rect {
            x: d.x + self.img.x,
            y: d.y + self.img.y,
            ..d
        }
    }

    /// Điểm (tọa độ nội dung) → px ảnh gốc; ngoài ảnh hiển thị thì `None`.
    pub fn to_image(self, x: i32, y: i32) -> Option<(i32, i32)> {
        let i = self.img;
        if i.w <= 0 || i.h <= 0 || x < i.x || y < i.y || x >= i.right() || y >= i.bottom() {
            return None;
        }
        let px = i64::from(x - i.x) * i64::from(self.orig.0) / i64::from(i.w);
        let py = i64::from(y - i.y) * i64::from(self.orig.1) / i64::from(i.h);
        Some((px as i32, py as i32))
    }

    /// Vị trí cuộn mới để khung `r` (tọa độ nội dung) hiện đủ, cách mép `pad`; đã kẹp.
    pub fn scroll_to_show(&self, scroll: i32, r: Rect, pad: i32) -> i32 {
        let mut s = scroll;
        if r.bottom() + pad > s + self.view_h {
            s = r.bottom() + pad - self.view_h;
        }
        if r.y - pad < s {
            s = r.y - pad;
        }
        s.clamp(0, self.max_scroll())
    }
}

/// Vùng chọn dòng trong khung ảnh.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pick {
    /// Chỉ số các dòng đang chọn (tăng dần, không trùng).
    pub sel: Vec<usize>,
    /// Dòng đang trỏ bằng bàn phím (viền nét đứt).
    pub cursor: Option<usize>,
}

/// Thao tác của người dùng trên khung ảnh (dòng trúng đã dò sẵn).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickInput {
    Click(Option<usize>),
    CtrlClick(Option<usize>),
    Up,
    Down,
    Enter,
    Space,
    SelectAll,
    Escape,
}

/// Việc cần làm sau một thao tác.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PickOutcome {
    Nothing,
    /// Chép `text`; nhãn "Đã chép" đặt cạnh dòng `at`.
    Copy {
        text: String,
        at: usize,
    },
    /// Đóng cửa sổ (Esc khi không có vùng chọn).
    Close,
}

impl Pick {
    pub fn is_selected(&self, i: usize) -> bool {
        self.sel.binary_search(&i).is_ok()
    }
}

/// Áp `input` lên vùng chọn `p` của `lines`; trả việc cần làm (chữ cần chép ghép theo `o`).
pub fn pick(p: &mut Pick, lines: &[OcrLine], o: TextOptions, input: PickInput) -> PickOutcome {
    let n = lines.len();
    let valid = |i: Option<usize>| i.filter(|&i| i < n);
    match input {
        PickInput::Escape => {
            if p.sel.is_empty() {
                PickOutcome::Close
            } else {
                p.sel.clear();
                PickOutcome::Nothing
            }
        }
        _ if n == 0 => PickOutcome::Nothing,
        PickInput::Click(hit) => match valid(hit) {
            Some(i) => {
                p.sel = vec![i];
                p.cursor = Some(i);
                PickOutcome::Copy {
                    text: compose(lines, &p.sel, o),
                    at: i,
                }
            }
            None => {
                p.sel.clear();
                PickOutcome::Nothing
            }
        },
        PickInput::CtrlClick(hit) => {
            let Some(i) = valid(hit) else {
                return PickOutcome::Nothing;
            };
            match p.sel.binary_search(&i) {
                Ok(at) => {
                    p.sel.remove(at);
                }
                Err(at) => p.sel.insert(at, i),
            }
            p.cursor = Some(i);
            if p.sel.is_empty() {
                PickOutcome::Nothing
            } else {
                PickOutcome::Copy {
                    text: compose(lines, &p.sel, o),
                    at: i,
                }
            }
        }
        PickInput::Up => {
            p.cursor = Some(match valid(p.cursor) {
                Some(i) => i.saturating_sub(1),
                None => n - 1,
            });
            PickOutcome::Nothing
        }
        PickInput::Down => {
            p.cursor = Some(match valid(p.cursor) {
                Some(i) => (i + 1).min(n - 1),
                None => 0,
            });
            PickOutcome::Nothing
        }
        PickInput::Enter => match valid(p.cursor) {
            Some(i) => pick(p, lines, o, PickInput::Click(Some(i))),
            None => PickOutcome::Nothing,
        },
        PickInput::Space => match valid(p.cursor) {
            Some(i) => pick(p, lines, o, PickInput::CtrlClick(Some(i))),
            None => PickOutcome::Nothing,
        },
        PickInput::SelectAll => {
            p.sel = (0..n).collect();
            PickOutcome::Copy {
                text: compose_all(lines, o),
                at: valid(p.cursor).unwrap_or(0),
            }
        }
    }
}

// ---- Vẽ (GDI)

/// DIB 32 bit từ ảnh BGRA `w`×`h` (hàng trên trước). Sai cỡ hay không tạo được thì `None`.
pub fn bmp_from_bgra(w: u32, h: u32, bgra: &[u8]) -> Option<Bmp> {
    let (wi, hi) = (i32::try_from(w).ok()?, i32::try_from(h).ok()?);
    let len = (w as usize).checked_mul(h as usize)?.checked_mul(4)?;
    if wi <= 0 || hi <= 0 || bgra.len() != len {
        return None;
    }
    let bmp = new_dib(wi, hi)?;
    let (handle, bits) = bmp;
    // SAFETY: DIB mới có đúng w*h ô 4 byte (`len` byte); chép đúng `len` byte từ `bgra`.
    unsafe {
        std::ptr::copy_nonoverlapping(bgra.as_ptr(), bits.cast::<u8>(), len);
    }
    Some(Bmp {
        handle,
        w: wi,
        h: hi,
    })
}

/// DIB trống `w`×`h` (32 bit, từ trên xuống) và con trỏ điểm ảnh của nó.
fn new_dib(w: i32, h: i32) -> Option<(windows::Win32::Graphics::Gdi::HBITMAP, *mut c_void)> {
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bits: *mut c_void = std::ptr::null_mut();
    // SAFETY: bmi và bits là biến cục bộ; lỗi hay con trỏ rỗng thì xóa bitmap trước khi trả.
    unsafe {
        let bmp = CreateDIBSection(None, &bmi, DIB_RGB_COLORS, &mut bits, None, 0).ok()?;
        if bits.is_null() {
            let _ = DeleteObject(HGDIOBJ(bmp.0));
            return None;
        }
        Some((bmp, bits))
    }
}

/// Bản co của `src` thành `w`×`h` (HALFTONE), làm một lần mỗi khi khung đổi cỡ.
pub fn scaled(src: &Bmp, w: i32, h: i32) -> Option<Bmp> {
    if w <= 0 || h <= 0 {
        return None;
    }
    let (handle, _) = new_dib(w, h)?;
    let out = Bmp { handle, w, h };
    // SAFETY: DC tạm tạo ở đây được trả bitmap cũ và xóa trên mọi đường đi; `out` giữ bitmap.
    unsafe {
        let mem = CreateCompatibleDC(None);
        if mem.is_invalid() {
            return None;
        }
        let old = SelectObject(mem, HGDIOBJ(out.handle.0));
        history_paint::blit(
            mem,
            src,
            &RECT {
                left: 0,
                top: 0,
                right: w,
                bottom: h,
            },
        );
        SelectObject(mem, old);
        let _ = DeleteDC(mem);
    }
    Some(out)
}

/// Phủ màu `c` độ đục `alpha`/255 lên `r` bằng `AlphaBlend` từ một điểm ảnh.
pub fn tint(dc: HDC, r: &RECT, c: Rgb, alpha: u8) {
    let (w, h) = (r.right - r.left, r.bottom - r.top);
    if w <= 0 || h <= 0 {
        return;
    }
    let px = [(u32::from(c.0) << 16) | (u32::from(c.1) << 8) | u32::from(c.2)];
    let Some(b) = history_paint::bmp_from_pixels(1, 1, &px) else {
        return;
    };
    // SAFETY: DC tạm tạo ở đây được trả bitmap cũ và xóa trên mọi đường đi; bitmap thuộc `b`.
    unsafe {
        let mem = CreateCompatibleDC(Some(dc));
        if mem.is_invalid() {
            return;
        }
        let old = SelectObject(mem, HGDIOBJ(b.handle.0));
        let bf = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: alpha,
            AlphaFormat: 0,
        };
        let _ = AlphaBlend(dc, r.left, r.top, w, h, mem, 0, 0, 1, 1, bf);
        SelectObject(mem, old);
        let _ = DeleteDC(mem);
    }
}

/// Viền màu `c` dày `px` pixel, vẽ phía trong `r`.
pub fn frame(dc: HDC, r: &RECT, c: Rgb, px: i32) {
    let mut r = *r;
    for _ in 0..px.max(1) {
        if r.right - r.left < 1 || r.bottom - r.top < 1 {
            return;
        }
        // SAFETY: dc hợp lệ trong lần vẽ; chổi dùng chung không bị xóa.
        unsafe {
            windows::Win32::Graphics::Gdi::FrameRect(dc, &r, theme::brush(c));
        }
        r.left += 1;
        r.top += 1;
        r.right -= 1;
        r.bottom -= 1;
    }
}

/// Viền nét đứt màu `c` quanh `r` (dòng đang trỏ bằng bàn phím).
pub fn dotted(dc: HDC, r: &RECT, c: Rgb) {
    if r.right - r.left < 2 || r.bottom - r.top < 2 {
        return;
    }
    // SAFETY: bút tạo ở đây được bỏ chọn rồi xóa; chổi rỗng là đối tượng kho, không xóa.
    unsafe {
        let pen = CreatePen(PS_DOT, 1, theme::colorref(c));
        if pen.is_invalid() {
            return;
        }
        let old_pen = SelectObject(dc, HGDIOBJ(pen.0));
        let old_brush = SelectObject(dc, GetStockObject(NULL_BRUSH));
        let _ = windows::Win32::Graphics::Gdi::SetBkMode(
            dc,
            windows::Win32::Graphics::Gdi::TRANSPARENT,
        );
        let _ = Rectangle(dc, r.left, r.top, r.right, r.bottom);
        SelectObject(dc, old_brush);
        SelectObject(dc, old_pen);
        let _ = DeleteObject(HGDIOBJ(pen.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkshot_core::ocr_text::line_at;

    fn r(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    fn line(text: &str, rect: Rect) -> OcrLine {
        OcrLine {
            text: text.into(),
            rect,
        }
    }

    fn three() -> Vec<OcrLine> {
        vec![
            line("one", r(10, 10, 100, 20)),
            line("two", r(10, 40, 100, 20)),
            line("three", r(10, 70, 100, 20)),
        ]
    }

    #[test]
    fn kept_size_caps_long_edge() {
        assert_eq!(kept_size(1920, 1080), (1920, 1080));
        assert_eq!(kept_size(4096, 100), (4096, 100));
        assert_eq!(kept_size(800, 12_000), (273, 4096));
        assert_eq!(kept_size(5000, 3000), (4096, 2458));
        assert_eq!(kept_size(100_000, 10), (4096, 1));
        // Ảnh nhỏ không bao giờ bị phóng (khác `ocr_scale`).
        assert_eq!(kept_size(30, 20), (30, 20));
        assert_eq!(kept_size(0, 0), (0, 0));
    }

    #[test]
    fn wide_image_fits_width_and_centres() {
        let g = geom((1920, 1080), (1920, 1080), 600, 400, 8);
        assert_eq!(g.img, r(8, 35, 584, 329));
        assert_eq!(g.content_h, 400);
        assert_eq!(g.view_h, 400);
        assert_eq!(g.max_scroll(), 0);
    }

    #[test]
    fn small_image_not_upscaled() {
        let g = geom((200, 50), (200, 50), 600, 400, 8);
        assert_eq!(g.img, r(200, 175, 200, 50));
        assert_eq!(g.max_scroll(), 0);
    }

    #[test]
    fn tall_image_fits_width_and_scrolls() {
        // Ảnh chụp cuộn 800×12000: bản giữ 273×4096, hẹp hơn khung nên giữ 1:1 và cuộn.
        let g = geom((800, 12_000), (273, 4096), 600, 400, 8);
        assert_eq!(g.img, r(163, 8, 273, 4096));
        assert_eq!(g.content_h, 4112);
        assert_eq!(g.max_scroll(), 3712);
        // Rộng hơn khung: khớp bề rộng.
        let g = geom((2000, 12_000), (683, 4096), 600, 400, 8);
        assert_eq!(g.img.w, 584);
        assert_eq!(g.img.h, 3502);
        assert_eq!(g.img.x, 8);
        assert_eq!(g.content_h, 3518);
    }

    #[test]
    fn degenerate_view_or_image_is_empty() {
        for g in [
            geom((0, 0), (0, 0), 600, 400, 8),
            geom((100, 100), (100, 100), 10, 10, 8),
            geom((100, 100), (100, 100), 0, 0, 0),
        ] {
            assert_eq!((g.img.w, g.img.h), (0, 0), "{g:?}");
            assert_eq!(g.max_scroll(), 0, "{g:?}");
            assert_eq!(g.to_image(5, 5), None);
            let lr = g.line_rect(r(10, 10, 20, 20));
            assert_eq!((lr.w, lr.h), (0, 0), "{lr:?}");
        }
    }

    #[test]
    fn line_rect_maps_through_three_layers() {
        // 1:1 cả ba lớp: chỉ dời theo chỗ ảnh.
        let g = geom((200, 50), (200, 50), 600, 400, 8);
        assert_eq!(g.line_rect(r(10, 10, 50, 20)), r(210, 185, 50, 20));
        // Co giữ (12000 → 4096) rồi hiển thị 1:1: tỉ lệ 4096/12000 theo dọc, 273/800 theo ngang.
        let g = geom((800, 12_000), (273, 4096), 600, 400, 8);
        let lr = g.line_rect(r(20, 11_900, 300, 40));
        assert_eq!(lr, r(163 + 6, 8 + 4061, 104, 15));
    }

    /// Tâm khung hiển thị của mọi dòng đổi ngược về px gốc thì trúng đúng dòng đó, ở mọi tổ hợp co
    /// giữ và co hiển thị (ba lớp tọa độ).
    #[test]
    fn display_centre_hits_same_line() {
        let cases = [
            ((1920u32, 1080u32), 600, 400),
            ((800, 12_000), 600, 400),
            ((2000, 12_000), 600, 400),
            ((5000, 3000), 900, 700),
            ((300, 80), 600, 400),
        ];
        for (orig, vw, vh) in cases {
            let (ow, oh) = (orig.0 as i32, orig.1 as i32);
            let lines: Vec<OcrLine> = (0..5)
                .map(|i| {
                    let h = (oh / 12).max(4);
                    line(&format!("l{i}"), r(ow / 10, i * oh / 5 + 1, ow / 2, h))
                })
                .collect();
            let g = geom(orig, kept_size(orig.0, orig.1), vw, vh, 8);
            for (i, l) in lines.iter().enumerate() {
                let d = g.line_rect(l.rect);
                assert!(d.w > 0 && d.h > 0, "{orig:?} {d:?}");
                let (cx, cy) = (d.x + d.w / 2, d.y + d.h / 2);
                let (px, py) = g.to_image(cx, cy).expect("inside image");
                assert_eq!(line_at(&lines, px, py), Some(i), "{orig:?} line {i} {d:?}");
            }
        }
    }

    #[test]
    fn to_image_outside_is_none_and_corners_map() {
        let g = geom((1920, 1080), (1920, 1080), 600, 400, 8);
        assert_eq!(g.to_image(7, 100), None);
        assert_eq!(g.to_image(100, 34), None);
        assert_eq!(g.to_image(8 + 584, 100), None);
        assert_eq!(g.to_image(100, 35 + 329), None);
        assert_eq!(g.to_image(8, 35), Some((0, 0)));
        let (x, y) = g.to_image(8 + 583, 35 + 328).unwrap();
        assert!(
            (1915..1920).contains(&x) && (1074..1080).contains(&y),
            "{x},{y}"
        );
    }

    /// Khung rỗng (`w == 0` hoặc `h == 0`): không panic, không chia 0.
    #[test]
    fn degenerate_line_boxes_do_not_panic() {
        let g = geom((800, 12_000), (273, 4096), 600, 400, 8);
        for b in [r(0, 0, 0, 0), r(100, 100, 0, 30), r(100, 100, 30, 0)] {
            let d = g.line_rect(b);
            assert!(d.w >= 0 && d.h >= 0);
        }
    }

    #[test]
    fn scroll_to_show_clamps() {
        let g = geom((800, 12_000), (273, 4096), 600, 400, 8);
        // Đã hiện: giữ.
        assert_eq!(g.scroll_to_show(0, r(0, 100, 10, 20), 8), 0);
        // Ở dưới: cuộn để mép dưới cách đáy 8.
        assert_eq!(
            g.scroll_to_show(0, r(0, 1000, 10, 20), 8),
            1000 + 20 + 8 - 400
        );
        // Ở trên: cuộn để mép trên cách đỉnh 8.
        assert_eq!(g.scroll_to_show(2000, r(0, 1000, 10, 20), 8), 992);
        // Kẹp trong [0, max].
        assert_eq!(g.scroll_to_show(500, r(0, 2, 10, 20), 8), 0);
        assert_eq!(g.scroll_to_show(0, r(0, 4100, 10, 20), 8), g.max_scroll());
    }

    fn opts() -> TextOptions {
        TextOptions::default()
    }

    fn copy(text: &str, at: usize) -> PickOutcome {
        PickOutcome::Copy {
            text: text.into(),
            at,
        }
    }

    #[test]
    fn click_selects_one_and_copies() {
        let ls = three();
        let mut p = Pick::default();
        assert_eq!(
            pick(&mut p, &ls, opts(), PickInput::Click(Some(1))),
            copy("two", 1)
        );
        assert_eq!(p.sel, vec![1]);
        assert_eq!(p.cursor, Some(1));
        assert_eq!(
            pick(&mut p, &ls, opts(), PickInput::Click(Some(2))),
            copy("three", 2)
        );
        assert_eq!(p.sel, vec![2]);
        // Ngoài dải: như bấm chỗ trống.
        assert_eq!(
            pick(&mut p, &ls, opts(), PickInput::Click(Some(9))),
            PickOutcome::Nothing
        );
        assert!(p.sel.is_empty());
    }

    #[test]
    fn click_blank_clears_without_copy() {
        let ls = three();
        let mut p = Pick {
            sel: vec![0, 2],
            cursor: Some(2),
        };
        assert_eq!(
            pick(&mut p, &ls, opts(), PickInput::Click(None)),
            PickOutcome::Nothing
        );
        assert!(p.sel.is_empty());
        assert_eq!(p.cursor, Some(2));
    }

    #[test]
    fn ctrl_click_toggles_and_copies_in_reading_order() {
        let ls = three();
        let mut p = Pick::default();
        assert_eq!(
            pick(&mut p, &ls, opts(), PickInput::CtrlClick(Some(2))),
            copy("three", 2)
        );
        assert_eq!(
            pick(&mut p, &ls, opts(), PickInput::CtrlClick(Some(0))),
            copy("one\r\nthree", 0)
        );
        assert_eq!(p.sel, vec![0, 2]);
        // Bỏ một dòng: chép phần còn lại.
        assert_eq!(
            pick(&mut p, &ls, opts(), PickInput::CtrlClick(Some(2))),
            copy("one", 2)
        );
        // Bỏ dòng cuối: không chép.
        assert_eq!(
            pick(&mut p, &ls, opts(), PickInput::CtrlClick(Some(0))),
            PickOutcome::Nothing
        );
        assert!(p.sel.is_empty());
        // Ctrl+bấm chỗ trống: giữ nguyên vùng chọn.
        p.sel = vec![1];
        assert_eq!(
            pick(&mut p, &ls, opts(), PickInput::CtrlClick(None)),
            PickOutcome::Nothing
        );
        assert_eq!(p.sel, vec![1]);
        // Tùy chọn ghép áp dụng cho chữ chép.
        let one = TextOptions {
            single_line: true,
            paragraphs: false,
        };
        assert_eq!(
            pick(&mut p, &ls, one, PickInput::CtrlClick(Some(2))),
            copy("two three", 2)
        );
    }

    #[test]
    fn arrows_move_cursor_without_copy() {
        let ls = three();
        let mut p = Pick::default();
        assert_eq!(
            pick(&mut p, &ls, opts(), PickInput::Down),
            PickOutcome::Nothing
        );
        assert_eq!(p.cursor, Some(0));
        pick(&mut p, &ls, opts(), PickInput::Down);
        pick(&mut p, &ls, opts(), PickInput::Down);
        pick(&mut p, &ls, opts(), PickInput::Down);
        assert_eq!(p.cursor, Some(2));
        pick(&mut p, &ls, opts(), PickInput::Up);
        assert_eq!(p.cursor, Some(1));
        let mut q = Pick::default();
        pick(&mut q, &ls, opts(), PickInput::Up);
        assert_eq!(q.cursor, Some(2));
        pick(&mut q, &ls, opts(), PickInput::Up);
        pick(&mut q, &ls, opts(), PickInput::Up);
        pick(&mut q, &ls, opts(), PickInput::Up);
        assert_eq!(q.cursor, Some(0));
        assert!(p.sel.is_empty() && q.sel.is_empty());
    }

    #[test]
    fn enter_and_space_act_on_cursor() {
        let ls = three();
        let mut p = Pick::default();
        assert_eq!(
            pick(&mut p, &ls, opts(), PickInput::Enter),
            PickOutcome::Nothing
        );
        assert_eq!(
            pick(&mut p, &ls, opts(), PickInput::Space),
            PickOutcome::Nothing
        );
        p.cursor = Some(1);
        assert_eq!(pick(&mut p, &ls, opts(), PickInput::Enter), copy("two", 1));
        p.cursor = Some(2);
        assert_eq!(
            pick(&mut p, &ls, opts(), PickInput::Space),
            copy("two\r\nthree", 2)
        );
        assert_eq!(pick(&mut p, &ls, opts(), PickInput::Space), copy("two", 2));
        p.cursor = Some(0);
        assert_eq!(pick(&mut p, &ls, opts(), PickInput::Enter), copy("one", 0));
        assert_eq!(p.sel, vec![0]);
    }

    #[test]
    fn select_all_and_escape() {
        let ls = three();
        let mut p = Pick::default();
        assert_eq!(
            pick(&mut p, &ls, opts(), PickInput::SelectAll),
            copy("one\r\ntwo\r\nthree", 0)
        );
        assert_eq!(p.sel, vec![0, 1, 2]);
        assert_eq!(
            pick(&mut p, &ls, opts(), PickInput::Escape),
            PickOutcome::Nothing
        );
        assert!(p.sel.is_empty());
        assert_eq!(
            pick(&mut p, &ls, opts(), PickInput::Escape),
            PickOutcome::Close
        );
        p.cursor = Some(2);
        assert_eq!(
            pick(&mut p, &ls, opts(), PickInput::SelectAll),
            copy("one\r\ntwo\r\nthree", 2)
        );
    }

    #[test]
    fn no_lines_only_escape_does_anything() {
        let mut p = Pick::default();
        for i in [
            PickInput::Click(Some(0)),
            PickInput::CtrlClick(Some(0)),
            PickInput::Click(None),
            PickInput::Up,
            PickInput::Down,
            PickInput::Enter,
            PickInput::Space,
            PickInput::SelectAll,
        ] {
            assert_eq!(pick(&mut p, &[], opts(), i), PickOutcome::Nothing, "{i:?}");
            assert_eq!(p, Pick::default());
        }
        assert_eq!(
            pick(&mut p, &[], opts(), PickInput::Escape),
            PickOutcome::Close
        );
    }
}
