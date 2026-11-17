//! Lịch sử ảnh chụp: vẽ bằng GDI (danh sách, khung chi tiết, thanh trên, trạng thái đặc biệt).
//! Chỉ vẽ; trạng thái và bố cục do `history_window` và `history_layout` lo.

use crate::history_layout::{
    empty_state_rects, empty_text_width, expires_text, fit_rect, row_subtitle, Layout, META_ROW,
};
use crate::theme::{self, ButtonKind, ControlState};
use crate::winutil::scale;
use darkshot_core::history::{format_dims, format_full, format_size, format_when, is_expired};
use darkshot_core::history::{Entry, Kind};
use darkshot_core::i18n::{Lang, Msg};
use darkshot_core::naming::LocalTime;
use darkshot_core::theme::{
    Rgb, ACCENT, ACCENT_BUTTON, BG, BUTTON_BORDER, DANGER, INPUT, LINE, SUCCESS, SURFACE,
    TAB_HOVER, TEXT, TEXT_DISABLED, TEXT_MUTED, WHITE,
};
use std::ffi::c_void;
use windows::Win32::Foundation::{LRESULT, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC,
    SelectObject, SetBrushOrgEx, SetStretchBltMode, StretchBlt, BITMAPINFO, BITMAPINFOHEADER,
    BI_RGB, DIB_RGB_COLORS, DRAW_TEXT_FORMAT, DT_CALCRECT, DT_CENTER, DT_END_ELLIPSIS, DT_LEFT,
    DT_NOPREFIX, DT_PATH_ELLIPSIS, DT_SINGLELINE, DT_VCENTER, DT_WORDBREAK, HALFTONE, HBITMAP, HDC,
    HFONT, HGDIOBJ, SRCCOPY,
};
use windows::Win32::UI::Controls::{CDDS_PREPAINT, CDRF_DODEFAULT, CDRF_SKIPDEFAULT, NMCUSTOMDRAW};

/// Phông của cửa sổ: thường, đậm (nút chính), tiêu đề (130%, đậm).
#[derive(Clone, Copy, Debug, Default)]
pub struct Fonts {
    pub normal: Option<HFONT>,
    pub bold: Option<HFONT>,
    pub title: Option<HFONT>,
}

/// Bitmap DIB 32 bit (đục) do cửa sổ giữ; tự `DeleteObject` khi bị bỏ.
#[derive(Debug, PartialEq, Eq)]
pub struct Bmp {
    pub handle: HBITMAP,
    pub w: i32,
    pub h: i32,
}

impl Drop for Bmp {
    fn drop(&mut self) {
        // SAFETY: bitmap do `bmp_from_pixels` tạo, chỉ `Bmp` này giữ; không còn chọn vào DC nào.
        unsafe {
            let _ = DeleteObject(HGDIOBJ(self.handle.0));
        }
    }
}

/// Tạo DIB `w` x `h` từ điểm ảnh `0x00RRGGBB` (hàng trên trước).
pub fn bmp_from_pixels(w: u32, h: u32, px: &[u32]) -> Option<Bmp> {
    let (wi, hi) = (i32::try_from(w).ok()?, i32::try_from(h).ok()?);
    if wi <= 0 || hi <= 0 || px.len() != (w as usize) * (h as usize) {
        return None;
    }
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: wi,
            biHeight: -hi, // âm: từ trên xuống
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bits: *mut c_void = std::ptr::null_mut();
    // SAFETY: DIB mới có đúng w*h ô 4 byte; chép đúng px.len() == w*h phần tử. Lỗi thì xóa bitmap.
    unsafe {
        let bmp = CreateDIBSection(None, &bmi, DIB_RGB_COLORS, &mut bits, None, 0).ok()?;
        if bits.is_null() {
            let _ = DeleteObject(HGDIOBJ(bmp.0));
            return None;
        }
        std::ptr::copy_nonoverlapping(px.as_ptr(), bits as *mut u32, px.len());
        Some(Bmp {
            handle: bmp,
            w: wi,
            h: hi,
        })
    }
}

/// Chép `bmp` vào `dst` (thu nhỏ bằng HALFTONE khi khác cỡ).
pub fn blit(dc: HDC, bmp: &Bmp, dst: &RECT) {
    let (dw, dh) = (dst.right - dst.left, dst.bottom - dst.top);
    if dw <= 0 || dh <= 0 {
        return;
    }
    // SAFETY: DC tạm tạo ở đây được trả bitmap cũ và xóa trên mọi đường đi; bitmap thuộc `bmp`.
    unsafe {
        let mem = CreateCompatibleDC(Some(dc));
        if mem.is_invalid() {
            return;
        }
        let old = SelectObject(mem, HGDIOBJ(bmp.handle.0));
        if dw == bmp.w && dh == bmp.h {
            let _ = BitBlt(dc, dst.left, dst.top, dw, dh, Some(mem), 0, 0, SRCCOPY);
        } else {
            let prev = SetStretchBltMode(dc, HALFTONE);
            let _ = SetBrushOrgEx(dc, 0, 0, None);
            let _ = StretchBlt(
                dc,
                dst.left,
                dst.top,
                dw,
                dh,
                Some(mem),
                0,
                0,
                bmp.w,
                bmp.h,
                SRCCOPY,
            );
            SetStretchBltMode(dc, windows::Win32::Graphics::Gdi::STRETCH_BLT_MODE(prev));
        }
        SelectObject(mem, old);
        let _ = DeleteDC(mem);
    }
}

/// Chuỗi UTF-16 không có số 0 cuối, cho `theme::draw_text`.
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

/// Đo chữ một dòng bằng phông `font` (pixel): (rộng, cao).
pub fn measure(dc: HDC, font: Option<HFONT>, text: &str) -> (i32, i32) {
    let r = theme::draw_text(
        dc,
        &wide(text),
        &RECT::default(),
        TEXT,
        font,
        DT_CALCRECT | DT_SINGLELINE | DT_NOPREFIX,
    );
    (r.right - r.left, r.bottom - r.top)
}

/// Chiều cao đoạn chữ xuống dòng trong bề rộng `width`.
pub fn measure_wrapped(dc: HDC, font: Option<HFONT>, text: &str, width: i32) -> i32 {
    let r = theme::draw_text(
        dc,
        &wide(text),
        &RECT {
            left: 0,
            top: 0,
            right: width.max(1),
            bottom: 0,
        },
        TEXT,
        font,
        DT_CALCRECT | DT_WORDBREAK | DT_NOPREFIX | DT_CENTER,
    );
    r.bottom - r.top
}

/// Gọi `f` với DC của màn hình (để đo chữ ngoài lúc vẽ), trả DC sau đó.
pub fn with_screen_dc<R>(f: impl FnOnce(HDC) -> R) -> R {
    // SAFETY: DC màn hình được trả bằng ReleaseDC ngay sau khi dùng.
    unsafe {
        let dc = GetDC(None);
        let r = f(dc);
        ReleaseDC(None, dc);
        r
    }
}

/// Màu chấm của nhãn loại.
pub fn kind_color(k: Kind) -> Rgb {
    match k {
        Kind::Saved => TEXT_MUTED,
        Kind::Uploaded => ACCENT,
        Kind::Copied => SUCCESS,
        Kind::UploadFallback => DANGER,
    }
}

fn inset(r: &RECT, d: i32) -> RECT {
    RECT {
        left: r.left + d,
        top: r.top + d,
        right: r.right - d,
        bottom: r.bottom - d,
    }
}

fn text(dc: HDC, s: &str, r: &RECT, c: Rgb, font: Option<HFONT>, flags: DRAW_TEXT_FORMAT) -> RECT {
    theme::draw_text(dc, &wide(s), r, c, font, flags | DT_NOPREFIX)
}

const ONE_LINE: DRAW_TEXT_FORMAT =
    DRAW_TEXT_FORMAT(DT_SINGLELINE.0 | DT_VCENTER.0 | DT_END_ELLIPSIS.0);

/// Chấm tròn đường kính `d` căn giữa dọc trong `line`, mép trái ở `x`.
fn dot(dc: HDC, x: i32, line: &RECT, d: i32, c: Rgb) {
    let top = line.top + (line.bottom - line.top - d) / 2;
    let r = RECT {
        left: x,
        top,
        right: x + d,
        bottom: top + d,
    };
    theme::ellipse(dc, &r, c, c, 0);
}

/// Biểu tượng khung ảnh (núi và mặt trời) trong `r`, nét `c` trên nền `bg`.
pub fn picture_icon(dc: HDC, r: &RECT, c: Rgb, bg: Rgb, dpi: u32) {
    let bw = scale(2, dpi).max(1);
    theme::round_rect(dc, r, scale(6, dpi), bg, c, bw);
    let (w, h) = (r.right - r.left, r.bottom - r.top);
    let p = |fx: i32, fy: i32| POINT {
        x: r.left + w * fx / 100,
        y: r.top + h * fy / 100,
    };
    theme::polyline(
        dc,
        &[p(14, 78), p(38, 46), p(56, 66), p(68, 54), p(86, 78)],
        c,
        bw,
    );
    let d = (h * 18 / 100).max(3);
    let sun = RECT {
        left: r.left + w * 66 / 100,
        top: r.top + h * 20 / 100,
        right: r.left + w * 66 / 100 + d,
        bottom: r.top + h * 20 / 100 + d,
    };
    theme::ellipse(dc, &sun, c, c, 0);
}

/// Biểu tượng ảnh vỡ trong `r`: khung ảnh bị một vết nứt chéo cắt đôi.
pub fn broken_icon(dc: HDC, r: &RECT, c: Rgb, bg: Rgb, dpi: u32) {
    picture_icon(dc, r, c, bg, dpi);
    let (w, h) = (r.right - r.left, r.bottom - r.top);
    let p = |fx: i32, fy: i32| POINT {
        x: r.left + w * fx / 100,
        y: r.top + h * fy / 100,
    };
    // Vết nứt màu nền (xóa nét khung), rồi viền nứt màu nét.
    let crack = [p(58, -4), p(46, 30), p(60, 52), p(44, 104)];
    theme::polyline(dc, &crack, bg, scale(5, dpi).max(3));
    theme::polyline(dc, &crack, c, scale(1, dpi).max(1));
}

/// Dữ liệu một dòng danh sách lúc vẽ.
pub struct RowView<'a> {
    pub entry: &'a Entry,
    pub exists: bool,
    pub selected: bool,
    pub hover: bool,
    pub focused: bool,
    pub thumb: ThumbView<'a>,
}

/// Ảnh nhỏ của một dòng lúc vẽ.
pub enum ThumbView<'a> {
    Pending,
    Ready(&'a Bmp),
    Broken,
}

/// Vẽ một dòng danh sách vào `r` (tọa độ client danh sách).
pub fn draw_row(
    dc: HDC,
    r: &RECT,
    v: &RowView,
    fonts: &Fonts,
    lang: Lang,
    now: &LocalTime,
    dpi: u32,
) {
    let s = |x: i32| scale(x, dpi);
    let bg = if v.selected {
        SURFACE
    } else if v.hover {
        TAB_HOVER
    } else {
        BG
    };
    theme::fill(dc, r, bg);
    if v.selected {
        let bar = RECT {
            right: r.left + s(3),
            ..*r
        };
        theme::fill(dc, &bar, ACCENT);
    }
    // Ô ảnh nhỏ.
    let (tw, th) = (s(88), s(56));
    let cell = RECT {
        left: r.left + s(14),
        top: r.top + (r.bottom - r.top - th) / 2,
        right: r.left + s(14) + tw,
        bottom: r.top + (r.bottom - r.top - th) / 2 + th,
    };
    theme::round_rect(dc, &cell, s(4), INPUT, INPUT, 0);
    match (&v.thumb, v.exists) {
        (ThumbView::Ready(b), _) => blit(dc, b, &fit_rect(b.w as u32, b.h as u32, &cell)),
        (ThumbView::Broken, _) | (ThumbView::Pending, false) => {
            let ic = RECT {
                left: cell.left + (tw - s(28)) / 2,
                top: cell.top + (th - s(22)) / 2,
                right: cell.left + (tw - s(28)) / 2 + s(28),
                bottom: cell.top + (th - s(22)) / 2 + s(22),
            };
            broken_icon(dc, &ic, TEXT_DISABLED, INPUT, dpi);
        }
        (ThumbView::Pending, true) => {}
    }
    // Chữ: dòng chính (ngày giờ + nhãn loại), dòng phụ (cỡ, dung lượng hoặc tệp mất).
    let left = cell.right + s(12);
    let right = r.right - s(12);
    let line1 = RECT {
        left,
        top: r.top + s(14),
        right,
        bottom: r.top + s(34),
    };
    let line2 = RECT {
        left,
        top: r.top + s(38),
        right,
        bottom: r.top + s(58),
    };
    // Ngày giờ được ưu tiên đủ chữ; nhãn loại lấy phần còn lại (cắt bằng "…" khi cột hẹp, quá hẹp
    // thì chỉ còn chấm màu).
    let label = v.entry.kind.label(lang);
    let when = format_when(lang, &v.entry.local, now);
    let (date_w, _) = measure(dc, fonts.normal, &when);
    let (label_w, _) = measure(dc, fonts.normal, label);
    let d = s(8);
    let room = right - left - date_w - s(8) - d - s(6);
    let lw = label_w.min(room);
    let lw = if lw < label_w && lw < s(36) {
        0
    } else {
        lw.max(0)
    };
    let label_rc = RECT {
        left: right - lw,
        ..line1
    };
    if lw > 0 {
        text(
            dc,
            label,
            &label_rc,
            TEXT_MUTED,
            fonts.normal,
            ONE_LINE | DT_LEFT,
        );
    }
    let dot_x = if lw > 0 {
        label_rc.left - s(6) - d
    } else {
        right - d
    };
    dot(dc, dot_x, &line1, d, kind_color(v.entry.kind));
    let date_rc = RECT {
        right: dot_x - s(8),
        ..line1
    };
    text(dc, &when, &date_rc, TEXT, fonts.normal, ONE_LINE | DT_LEFT);
    if v.exists {
        let sub = row_subtitle(lang, v.entry);
        text(
            dc,
            &sub,
            &line2,
            TEXT_MUTED,
            fonts.normal,
            ONE_LINE | DT_LEFT,
        );
    } else {
        let msg = darkshot_core::i18n::t(lang, Msg::HistoryFileMissing);
        text(dc, msg, &line2, DANGER, fonts.normal, ONE_LINE | DT_LEFT);
    }
    if v.focused && v.selected {
        theme::focus_frame(dc, &inset(r, s(2)), dpi);
    }
}

/// Thanh trên ở trạng thái tĩnh: ô tìm (chữ gợi ý), bốn viên thuốc (Tất cả đang chọn), nút xóa lịch sử.
pub fn draw_header(dc: HDC, l: &Layout, fonts: &Fonts, lang: Lang, has_entries: bool) {
    let dpi = l.dpi;
    let s = |x: i32| scale(x, dpi);
    let t = |m: Msg| darkshot_core::i18n::t(lang, m);
    // Ô tìm.
    theme::draw_field(dc, &l.search, dpi, theme::edit_border(false, true));
    let cy = (l.search.top + l.search.bottom) / 2;
    let d = s(12);
    let lens = RECT {
        left: l.search.left + s(10),
        top: cy - d / 2 - s(1),
        right: l.search.left + s(10) + d,
        bottom: cy + d / 2 - s(1),
    };
    let bw = s(2).max(1) * 3 / 4;
    theme::ellipse(dc, &lens, INPUT, TEXT_MUTED, bw.max(1));
    theme::polyline(
        dc,
        &[
            POINT {
                x: lens.right - s(2),
                y: lens.bottom - s(2),
            },
            POINT {
                x: lens.right + s(3),
                y: lens.bottom + s(3),
            },
        ],
        TEXT_MUTED,
        bw.max(1) + 1,
    );
    let hint = RECT {
        left: lens.right + s(10),
        right: l.search.right - s(10),
        ..l.search
    };
    text(
        dc,
        t(Msg::HistorySearchPlaceholder),
        &hint,
        TEXT_MUTED,
        fonts.normal,
        ONE_LINE | DT_LEFT,
    );
    // Viên thuốc lọc.
    let labels = [
        Msg::HistoryFilterAll,
        Msg::HistoryFilterSaved,
        Msg::HistoryFilterUploaded,
        Msg::HistoryFilterCopied,
    ];
    for (k, chip) in l.chips.iter().enumerate() {
        let radius = (chip.bottom - chip.top) / 2;
        let (fill, border, fg) = if k == 0 {
            (ACCENT_BUTTON, ACCENT_BUTTON, WHITE)
        } else {
            (BG, BUTTON_BORDER, TEXT_MUTED)
        };
        theme::round_rect(dc, chip, radius, fill, border, s(1).max(1));
        let inner = RECT {
            left: chip.left + s(6),
            right: chip.right - s(6),
            ..*chip
        };
        text(
            dc,
            t(labels[k]),
            &inner,
            fg,
            fonts.normal,
            ONE_LINE | DT_CENTER,
        );
    }
    // Nút "Xóa lịch sử".
    let wtext = wide(t(Msg::HistoryClear));
    let state = ControlState {
        hot: false,
        pressed: false,
        focused: false,
        enabled: has_entries,
        checked: false,
    };
    theme::draw_button(
        dc,
        &l.clear_btn,
        &wtext,
        ButtonKind::Normal,
        state,
        dpi,
        fonts.normal,
        (false, false),
    );
}

/// Đường kẻ dưới thanh trên và giữa danh sách với khung chi tiết.
pub fn draw_separators(dc: HDC, l: &Layout, with_list: bool) {
    let h = RECT {
        left: 0,
        top: l.body.top - 1,
        right: l.body.right,
        bottom: l.body.top,
    };
    theme::fill(dc, &h, LINE);
    if with_list {
        let v = RECT {
            left: l.detail.left,
            top: l.body.top,
            right: l.detail.left + 1,
            bottom: l.body.bottom,
        };
        theme::fill(dc, &v, LINE);
    }
}

/// Dải "Lịch sử đang tắt" (nút "Mở Tùy chọn" là cửa sổ con, vẽ riêng).
pub fn draw_banner(dc: HDC, l: &Layout, fonts: &Fonts, lang: Lang) {
    let dpi = l.dpi;
    let s = |x: i32| scale(x, dpi);
    theme::round_rect(dc, &l.banner, s(6), SURFACE, SURFACE, 0);
    let cy = (l.banner.top + l.banner.bottom) / 2;
    let d = s(18);
    let ic = RECT {
        left: l.banner.left + s(12),
        top: cy - d / 2,
        right: l.banner.left + s(12) + d,
        bottom: cy - d / 2 + d,
    };
    theme::ellipse(dc, &ic, SURFACE, ACCENT, s(2).max(1) * 3 / 4 + 1);
    // Chữ "i" vẽ bằng hai khối: chấm và thân.
    let cx = (ic.left + ic.right) / 2;
    let stem_w = s(2).max(2);
    let dot_rc = RECT {
        left: cx - stem_w / 2,
        top: ic.top + d * 26 / 100,
        right: cx - stem_w / 2 + stem_w,
        bottom: ic.top + d * 26 / 100 + stem_w,
    };
    theme::fill(dc, &dot_rc, ACCENT);
    let stem = RECT {
        left: dot_rc.left,
        top: ic.top + d * 42 / 100,
        right: dot_rc.right,
        bottom: ic.top + d * 76 / 100,
    };
    theme::fill(dc, &stem, ACCENT);
    let tr = RECT {
        left: ic.right + s(10),
        top: l.banner.top,
        right: l.banner_btn.left - s(12),
        bottom: l.banner.bottom,
    };
    text(
        dc,
        darkshot_core::i18n::t(lang, Msg::HistoryDisabledBanner),
        &tr,
        TEXT,
        fonts.normal,
        ONE_LINE | DT_LEFT,
    );
}

/// Trạng thái đặc biệt giữa thân: biểu tượng, tiêu đề, đoạn giải thích (nút là cửa sổ con).
#[allow(clippy::too_many_arguments)]
pub fn draw_empty(
    dc: HDC,
    l: &Layout,
    fonts: &Fonts,
    title: &str,
    body: &str,
    button_w: Option<i32>,
) {
    let dpi = l.dpi;
    let tw = empty_text_width(&l.body, dpi);
    let th = measure_wrapped(dc, fonts.normal, body, tw);
    let e = empty_state_rects(&l.body, dpi, th, button_w);
    picture_icon(dc, &e.icon, TEXT_DISABLED, BG, dpi);
    text(dc, title, &e.title, TEXT, fonts.title, ONE_LINE | DT_CENTER);
    text(
        dc,
        body,
        &e.body,
        TEXT_MUTED,
        fonts.normal,
        DT_WORDBREAK | DT_CENTER,
    );
}

/// Bản xem trước lúc vẽ.
pub enum PreviewView<'a> {
    Idle,
    Loading,
    Ready(&'a Bmp),
    Missing,
    Unreadable,
}

/// Khung chi tiết của mục `e` (chưa chọn: lời nhắc chọn ảnh).
#[allow(clippy::too_many_arguments)]
pub fn draw_detail(
    dc: HDC,
    l: &Layout,
    e: Option<(&Entry, bool)>,
    preview: PreviewView,
    fonts: &Fonts,
    lang: Lang,
    today: &LocalTime,
) {
    let dpi = l.dpi;
    let s = |x: i32| scale(x, dpi);
    let t = |m: Msg| darkshot_core::i18n::t(lang, m);
    let Some((e, exists)) = e else {
        let area = RECT {
            left: l.detail.left + s(16),
            right: l.detail.right - s(16),
            ..l.detail
        };
        let cy = (area.top + area.bottom) / 2 - s(24);
        let ic = RECT {
            left: (area.left + area.right) / 2 - s(32),
            top: cy - s(48),
            right: (area.left + area.right) / 2 + s(32),
            bottom: cy,
        };
        picture_icon(dc, &ic, TEXT_DISABLED, BG, dpi);
        let tr = RECT {
            top: cy + s(14),
            bottom: cy + s(38),
            ..area
        };
        text(
            dc,
            t(Msg::HistorySelectPrompt),
            &tr,
            TEXT_MUTED,
            fonts.normal,
            ONE_LINE | DT_CENTER,
        );
        return;
    };
    // Vùng xem trước.
    theme::round_rect(dc, &l.preview_area, s(6), INPUT, INPUT, 0);
    let inner = inset(&l.preview_area, s(16));
    let center_msg = |dc: HDC, icon: bool, title: &str, hint: Option<&str>| {
        let tw = (inner.right - inner.left).clamp(1, s(380));
        let hint_h = hint.map_or(0, |h| measure_wrapped(dc, fonts.normal, h, tw));
        let ih = if icon { s(36) + s(12) } else { 0 };
        let total = ih + s(22) + if hint.is_some() { s(6) + hint_h } else { 0 };
        let top = inner.top + ((inner.bottom - inner.top) - total).max(0) / 2;
        let cx = (inner.left + inner.right) / 2;
        if icon {
            let ic = RECT {
                left: cx - s(24),
                top,
                right: cx + s(24),
                bottom: top + s(36),
            };
            broken_icon(dc, &ic, TEXT_MUTED, INPUT, dpi);
        }
        let tr = RECT {
            left: cx - tw / 2,
            top: top + ih,
            right: cx - tw / 2 + tw,
            bottom: top + ih + s(22),
        };
        let c = if icon { TEXT } else { TEXT_MUTED };
        text(dc, title, &tr, c, fonts.normal, ONE_LINE | DT_CENTER);
        if let Some(h) = hint {
            let hr = RECT {
                top: tr.bottom + s(6),
                bottom: (tr.bottom + s(6) + hint_h).min(inner.bottom),
                ..tr
            };
            text(
                dc,
                h,
                &hr,
                TEXT_MUTED,
                fonts.normal,
                DT_WORDBREAK | DT_CENTER,
            );
        }
    };
    match preview {
        PreviewView::Ready(b) => {
            // Bitmap đã thu nhỏ vừa khung lúc giải mã; khung nhỏ đi sau đó thì thu tiếp khi vẽ.
            let dst = fit_rect(b.w as u32, b.h as u32, &inner);
            blit(dc, b, &dst);
        }
        PreviewView::Loading => center_msg(dc, false, t(Msg::HistoryPreviewLoading), None),
        PreviewView::Missing => center_msg(
            dc,
            true,
            t(Msg::HistoryFileMissing),
            Some(t(Msg::HistoryFileMissingHint)),
        ),
        PreviewView::Unreadable => center_msg(dc, true, t(Msg::HistoryFileUnreadable), None),
        PreviewView::Idle => {}
    }
    // Tiêu đề và lưới nhãn–giá trị.
    text(
        dc,
        &format_full(lang, &e.local),
        &l.title,
        TEXT,
        fonts.title,
        ONE_LINE | DT_LEFT,
    );
    let mut rows: Vec<(Msg, String, Rgb, DRAW_TEXT_FORMAT, bool)> = Vec::new();
    rows.push((
        Msg::HistoryLabelType,
        e.kind.label(lang).to_string(),
        TEXT,
        DT_END_ELLIPSIS,
        true,
    ));
    rows.push((
        Msg::HistoryLabelDimensions,
        format_dims(e.width, e.height),
        TEXT,
        DT_END_ELLIPSIS,
        false,
    ));
    if let Some(b) = e.bytes {
        rows.push((
            Msg::HistoryLabelFileSize,
            format_size(lang, b),
            TEXT,
            DT_END_ELLIPSIS,
            false,
        ));
    }
    if let Some(p) = &e.path {
        let c = if exists { TEXT } else { DANGER };
        rows.push((
            Msg::HistoryLabelFile,
            p.to_string_lossy().into_owned(),
            c,
            DT_PATH_ELLIPSIS,
            false,
        ));
    }
    if let Some(u) = &e.url {
        rows.push((
            Msg::HistoryLabelLink,
            u.clone(),
            TEXT,
            DT_PATH_ELLIPSIS,
            false,
        ));
    }
    if e.kind == Kind::Uploaded {
        let (v, c) = match &e.expires {
            Some(d) if is_expired(d, today) => (t(Msg::HistoryExpired).to_string(), DANGER),
            Some(d) => (expires_text(lang, d), TEXT),
            None => (t(Msg::HistoryNeverExpires).to_string(), TEXT),
        };
        rows.push((Msg::HistoryLabelExpires, v, c, DT_END_ELLIPSIS, false));
    }
    let label_w = rows
        .iter()
        .map(|r| measure(dc, fonts.normal, t(r.0)).0)
        .max()
        .unwrap_or(0)
        + s(16);
    let mut y = l.title.bottom + s(6);
    for (label, value, c, mode, with_dot) in rows.iter().take(6) {
        let line = RECT {
            left: l.meta_area.left,
            top: y,
            right: l.meta_area.right,
            bottom: y + s(META_ROW),
        };
        let lr = RECT {
            right: line.left + label_w,
            ..line
        };
        text(
            dc,
            t(*label),
            &lr,
            TEXT_MUTED,
            fonts.normal,
            ONE_LINE | DT_LEFT,
        );
        let mut vr = RECT {
            left: lr.right,
            ..line
        };
        if *with_dot {
            dot(dc, vr.left, &line, s(8), kind_color(e.kind));
            vr.left += s(8) + s(8);
        }
        // `mode`: DT_PATH_ELLIPSIS (đường dẫn, liên kết: cắt giữa) hoặc DT_END_ELLIPSIS.
        let flags = DT_SINGLELINE | DT_VCENTER | DT_LEFT | *mode;
        text(dc, value, &vr, *c, fonts.normal, flags);
        y += s(META_ROW);
    }
}

/// Vẽ nút "Xóa khỏi lịch sử": dáng nút thường, chữ `DANGER` (nguy hiểm nhẹ). Như
/// `theme::custom_draw_button`: vẽ hết ở `CDDS_PREPAINT`, điều khiển gốc lo bàn phím và trợ năng.
pub fn custom_draw_danger(nmcd: &NMCUSTOMDRAW, bg: Rgb, dpi: u32) -> LRESULT {
    if nmcd.dwDrawStage != CDDS_PREPAINT {
        return LRESULT(CDRF_DODEFAULT as isize);
    }
    let hwnd = nmcd.hdr.hwndFrom;
    let rc = nmcd.rc;
    let state = theme::button_state(hwnd);
    let label = theme::window_text(hwnd);
    let cues = theme::ui_cues(hwnd);
    let font = theme::control_font(hwnd);
    let c = theme::button_colors(ButtonKind::Normal, state);
    let fg = if state.enabled { DANGER } else { c.text };
    theme::buffered(nmcd.hdc, &rc, |dc| {
        theme::fill(dc, &rc, bg);
        theme::round_rect(
            dc,
            &rc,
            scale(theme::RADIUS, dpi),
            c.fill,
            c.border,
            scale(1, dpi).max(1),
        );
        theme::draw_text(
            dc,
            &label,
            &rc,
            fg,
            font,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | theme::prefix_flag(cues.1),
        );
        if state.focused && cues.0 {
            theme::focus_frame(dc, &inset(&rc, scale(3, dpi)), dpi);
        }
    });
    LRESULT(CDRF_SKIPDEFAULT as isize)
}
