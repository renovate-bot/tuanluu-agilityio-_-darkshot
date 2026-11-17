//! Giao diện tối dùng chung: thanh tiêu đề, menu, điều khiển chuẩn, chổi và màu vẽ.

use crate::winutil::scale;
use darkshot_core::theme::{
    Rgb, ACCENT, ACCENT_BUTTON, ACCENT_BUTTON_HOVER, ACCENT_BUTTON_PRESSED, BG, BUTTON,
    BUTTON_BORDER, BUTTON_HOVER, BUTTON_PRESSED, CHECK_BORDER, INPUT_BORDER, LINE, SURFACE,
    TAB_HOVER, TEXT, TEXT_DISABLED, TEXT_MUTED, TRACK, WHITE,
};
use std::ffi::c_void;
use std::sync::Mutex;
use windows::core::{w, PCSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT};
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWINDOWATTRIBUTE};
use windows::Win32::Graphics::Gdi::{
    AlphaBlend, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateDIBSection, CreatePen,
    CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, Ellipse, FillRect, FrameRect,
    IntersectClipRect, Polyline, RestoreDC, RoundRect, SaveDC, SelectObject, SetBkColor, SetBkMode,
    SetTextColor, SetWindowOrgEx, AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    BLENDFUNCTION, DIB_RGB_COLORS, DRAW_TEXT_FORMAT, DT_CALCRECT, DT_CENTER, DT_END_ELLIPSIS,
    DT_HIDEPREFIX, DT_LEFT, DT_SINGLELINE, DT_VCENTER, HBRUSH, HDC, HFONT, HGDIOBJ, OPAQUE,
    PS_INSIDEFRAME, PS_SOLID, SRCCOPY, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW};
use windows::Win32::UI::Controls::{
    SetWindowTheme, BST_CHECKED, BST_HOT, CDDS_PREPAINT, CDRF_DODEFAULT, CDRF_SKIPDEFAULT,
    NMCUSTOMDRAW, TBM_GETCHANNELRECT, TBM_GETTHUMBRECT,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetFocus, IsWindowEnabled};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW, SendMessageW,
    BM_GETSTATE, BST_PUSHED, GWL_STYLE, UISF_HIDEACCEL, UISF_HIDEFOCUS, WM_GETFONT,
    WM_QUERYUISTATE,
};

/// Thanh tiêu đề tối. Thuộc tính 20 (Windows 10 20H1 trở lên, 11), lỗi thì thử 19 (bản cũ hơn); không có thì bỏ qua.
pub fn apply_dark_title_bar(hwnd: HWND) {
    let on: i32 = 1;
    for attr in [20, 19] {
        // SAFETY: on là biến cục bộ 4 byte sống suốt lời gọi; hwnd là cửa sổ của luồng này.
        let r = unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWINDOWATTRIBUTE(attr),
                &on as *const i32 as *const c_void,
                4,
            )
        };
        if r.is_ok() {
            return;
        }
    }
}

/// Menu của tiến trình theo chế độ tối (uxtheme ordinal 135 `SetPreferredAppMode`, 136 `FlushMenuThemes`).
/// Thiếu hàm thì bỏ qua. Gọi một lần trước khi tạo cửa sổ.
pub fn enable_dark_menus() {
    // SAFETY: nạp uxtheme.dll của hệ thống; hàm theo số thứ tự được kiểm None trước khi gọi,
    // và chữ ký (int) -> int / () -> void khớp với hàm không công khai tương ứng.
    unsafe {
        let module = match GetModuleHandleW(w!("uxtheme.dll")) {
            Ok(m) => m,
            Err(_) => match LoadLibraryW(w!("uxtheme.dll")) {
                Ok(m) => m,
                Err(_) => return,
            },
        };
        let ordinal = |n: usize| PCSTR(std::ptr::without_provenance::<u8>(n));
        let Some(set_mode) = GetProcAddress(module, ordinal(135)) else {
            return;
        };
        let set_mode: unsafe extern "system" fn(i32) -> i32 = std::mem::transmute(set_mode);
        // 2 = ForceDark.
        set_mode(2);
        if let Some(flush) = GetProcAddress(module, ordinal(136)) {
            let flush: unsafe extern "system" fn() = std::mem::transmute(flush);
            flush();
        }
    }
}

#[derive(Clone, Copy)]
pub enum DarkClass {
    /// `DarkMode_Explorer`: nút, ô chữ, thanh cuộn.
    Explorer,
    /// `DarkMode_CFD`: hộp chọn.
    Cfd,
}

/// Cho điều khiển chuẩn dùng giao diện tối của hệ thống; không có thì không đổi gì.
pub fn set_dark_control_theme(hwnd: HWND, class: DarkClass) {
    let name = match class {
        DarkClass::Explorer => w!("DarkMode_Explorer"),
        DarkClass::Cfd => w!("DarkMode_CFD"),
    };
    // SAFETY: hwnd là cửa sổ của luồng này; tên là chuỗi hằng kết thúc bằng 0.
    unsafe {
        let _ = SetWindowTheme(hwnd, name, None);
    }
}

/// Chổi đặc dùng chung, tạo lười một lần cho mỗi màu và sống suốt tiến trình (không bao giờ xóa).
static BRUSHES: Mutex<Vec<(u32, isize)>> = Mutex::new(Vec::new());

pub fn brush(c: Rgb) -> HBRUSH {
    let key = c.colorref();
    let Ok(mut cache) = BRUSHES.lock() else {
        // SAFETY: tạo chổi mới; chỉ xảy ra khi khóa bị hỏng, chấp nhận rò một chổi.
        return unsafe { CreateSolidBrush(COLORREF(key)) };
    };
    if let Some((_, h)) = cache.iter().find(|(k, _)| *k == key) {
        return HBRUSH(*h as *mut c_void);
    }
    // SAFETY: tạo chổi đặc; lưu lại để dùng mãi.
    let h = unsafe { CreateSolidBrush(COLORREF(key)) };
    cache.push((key, h.0 as isize));
    h
}

/// Loại nút bấm: thường (nền xám) hoặc chính (OK, Đóng: nền màu nhấn, chữ trắng đậm).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonKind {
    Normal,
    Primary,
}

/// Trạng thái của một điều khiển lúc vẽ.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControlState {
    pub hot: bool,
    pub pressed: bool,
    pub focused: bool,
    pub enabled: bool,
    pub checked: bool,
}

/// Bộ màu vẽ: nền, viền, chữ.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Colors {
    pub fill: Rgb,
    pub border: Rgb,
    pub text: Rgb,
}

/// Màu nút bấm theo loại và trạng thái.
pub fn button_colors(kind: ButtonKind, s: ControlState) -> Colors {
    if !s.enabled {
        return Colors {
            fill: BUTTON,
            border: BUTTON_BORDER,
            text: TEXT_DISABLED,
        };
    }
    match kind {
        ButtonKind::Normal => Colors {
            fill: if s.pressed {
                BUTTON_PRESSED
            } else if s.hot {
                BUTTON_HOVER
            } else {
                BUTTON
            },
            border: BUTTON_BORDER,
            text: TEXT,
        },
        ButtonKind::Primary => {
            let fill = if s.pressed {
                ACCENT_BUTTON_PRESSED
            } else if s.hot {
                ACCENT_BUTTON_HOVER
            } else {
                ACCENT_BUTTON
            };
            Colors {
                fill,
                border: fill,
                text: WHITE,
            }
        }
    }
}

/// Hộp ô tích: tắt viền `CHECK_BORDER`, bật nền và viền `ACCENT_BUTTON`; chữ (nằm trên nền khung) `TEXT`
/// hoặc `TEXT_DISABLED`.
pub fn check_colors(s: ControlState) -> Colors {
    let text = if s.enabled { TEXT } else { TEXT_DISABLED };
    match (s.checked, s.enabled) {
        (true, true) => Colors {
            fill: ACCENT_BUTTON,
            border: ACCENT_BUTTON,
            text,
        },
        (true, false) => Colors {
            fill: TEXT_DISABLED,
            border: TEXT_DISABLED,
            text,
        },
        (false, true) => Colors {
            fill: SURFACE,
            border: CHECK_BORDER,
            text,
        },
        (false, false) => Colors {
            fill: SURFACE,
            border: TEXT_DISABLED,
            text,
        },
    }
}

/// Vòng nút chọn: bật thì viền và chấm (`fill`) `ACCENT`; tắt viền `CHECK_BORDER`.
pub fn radio_colors(s: ControlState) -> Colors {
    let text = if s.enabled { TEXT } else { TEXT_DISABLED };
    let mark = match (s.checked, s.enabled) {
        (true, true) => ACCENT,
        (true, false) | (false, false) => TEXT_DISABLED,
        (false, true) => CHECK_BORDER,
    };
    Colors {
        fill: if s.checked { mark } else { SURFACE },
        border: mark,
        text,
    }
}

/// Tab: chọn chữ `TEXT`, gạch dưới (`border`) `ACCENT`; không chọn chữ `TEXT_MUTED`; khi trỏ nền
/// `TAB_HOVER` và chữ `TEXT`.
pub fn tab_colors(selected: bool, hot: bool) -> Colors {
    Colors {
        fill: if hot { TAB_HOVER } else { BG },
        border: if selected { ACCENT } else { LINE },
        text: if selected || hot { TEXT } else { TEXT_MUTED },
    }
}

/// Màu viền ô nhập và độ dày (DIP) của gạch dưới: 2 khi có tiêu điểm, 1 khi không.
pub fn edit_border(focused: bool, enabled: bool) -> (Rgb, u32) {
    if focused && enabled {
        (ACCENT, 2)
    } else {
        (INPUT_BORDER, 1)
    }
}

pub fn colorref(c: Rgb) -> COLORREF {
    COLORREF(c.colorref())
}

/// Xử lý `WM_CTLCOLOR*`: đặt màu chữ và nền cho `hdc`, trả chổi nền dùng chung.
pub fn ctl_color(hdc: HDC, fg: Rgb, bg: Rgb) -> LRESULT {
    // SAFETY: hdc do hệ thống cấp cho lần vẽ này.
    unsafe {
        SetTextColor(hdc, colorref(fg));
        SetBkColor(hdc, colorref(bg));
        SetBkMode(hdc, OPAQUE);
    }
    LRESULT(brush(bg).0 as isize)
}

// ---------------------------------------------------------------------------
// Vẽ tay: hình cơ bản, nút bấm, ô tích, nút chọn, thanh trượt
// ---------------------------------------------------------------------------

/// Bán kính bo góc nút bấm và hộp ô tích (DIP).
pub const RADIUS: i32 = 4;
/// Cạnh hộp ô tích và vòng nút chọn (DIP).
const BOX: i32 = 18;
/// Khoảng từ hộp tới chữ (DIP). Hộp + khoảng = 24 DIP, đúng phần `HK_BOX` mà bố cục Tùy chọn dành sẵn.
/// Hình mẫu dùng 10, nhưng 10 làm cắt nhãn dài nhất của tab Phím tắt ("Tải lên nhanh (toàn màn hình)").
const BOX_GAP: i32 = 6;
/// Đường kính chấm của nút chọn bật (DIP).
const DOT: i32 = 8;
/// Viền hộp ô tích và vòng nút chọn; bề dày nét dấu tích (DIP).
const BORDER_DIP: f32 = 1.5;
const TICK_DIP: f32 = 2.0;
/// Núm thanh trượt, độ dày viền núm và độ cao rãnh (DIP).
const KNOB: i32 = 18;
const KNOB_BORDER: i32 = 4;
const TRACK_H: i32 = 4;

/// Gợi ý bàn phím của cửa sổ theo `WM_QUERYUISTATE`: (hiện khung tiêu điểm, hiện chữ gạch dưới).
pub fn ui_cues(hwnd: HWND) -> (bool, bool) {
    // SAFETY: hỏi trạng thái giao diện của cửa sổ thuộc luồng này; không có tham số con trỏ.
    let st = unsafe { SendMessageW(hwnd, WM_QUERYUISTATE, None, None).0 } as u32;
    (st & UISF_HIDEFOCUS == 0, st & UISF_HIDEACCEL == 0)
}

/// Cờ `DrawTextW` ẩn chữ gạch dưới khi chưa dùng bàn phím (như Windows).
pub fn prefix_flag(show_accel: bool) -> DRAW_TEXT_FORMAT {
    if show_accel {
        DRAW_TEXT_FORMAT(0)
    } else {
        DT_HIDEPREFIX
    }
}

/// Tô `rc` bằng chổi dùng chung màu `c`.
pub fn fill(hdc: HDC, rc: &RECT, c: Rgb) {
    // SAFETY: hdc hợp lệ trong lần vẽ của bên gọi; chổi dùng chung không bị xóa.
    unsafe {
        FillRect(hdc, rc, brush(c));
    }
}

/// Chọn bút viền (tạo rồi xóa ngay) và chổi nền dùng chung, gọi `draw`, trả lại đối tượng cũ.
fn shape(hdc: HDC, fill_c: Rgb, border: Rgb, bw: i32, draw: impl FnOnce(HDC)) {
    let (pen_c, width) = if bw > 0 { (border, bw) } else { (fill_c, 1) };
    // SAFETY: bút tạo ở đây được bỏ chọn rồi xóa trước khi trả; chổi dùng chung không bị xóa.
    unsafe {
        let pen = CreatePen(PS_INSIDEFRAME, width, colorref(pen_c));
        if pen.is_invalid() {
            return;
        }
        let old_pen = SelectObject(hdc, HGDIOBJ(pen.0));
        let old_brush = SelectObject(hdc, HGDIOBJ(brush(fill_c).0));
        draw(hdc);
        SelectObject(hdc, old_brush);
        SelectObject(hdc, old_pen);
        let _ = DeleteObject(HGDIOBJ(pen.0));
    }
}

/// Hình chữ nhật bo góc `radius` px, nền `fill_c`, viền `border` dày `bw` px (0 = không viền), vẽ trong `rc`.
pub fn round_rect(hdc: HDC, rc: &RECT, radius: i32, fill_c: Rgb, border: Rgb, bw: i32) {
    shape(hdc, fill_c, border, bw, |dc| {
        // SAFETY: dc có bút và chổi đã chọn; tọa độ là số nguyên.
        unsafe {
            let _ = RoundRect(
                dc,
                rc.left,
                rc.top,
                rc.right,
                rc.bottom,
                radius * 2,
                radius * 2,
            );
        }
    });
}

/// Hình tròn (elip) nội tiếp `rc`, nền `fill_c`, viền `border` dày `bw` px (0 = không viền).
pub fn ellipse(hdc: HDC, rc: &RECT, fill_c: Rgb, border: Rgb, bw: i32) {
    shape(hdc, fill_c, border, bw, |dc| {
        // SAFETY: dc có bút và chổi đã chọn.
        unsafe {
            let _ = Ellipse(dc, rc.left, rc.top, rc.right, rc.bottom);
        }
    });
}

/// Hình chữ nhật màu `c` chỉ bo hai góc trên: vẽ hình bo cao hơn `rc` rồi cắt phần dưới.
pub fn round_top_rect(hdc: HDC, rc: &RECT, radius: i32, c: Rgb) {
    // SAFETY: lưu và trả trạng thái DC (vùng cắt) quanh lần vẽ.
    unsafe {
        let saved = SaveDC(hdc);
        IntersectClipRect(hdc, rc.left, rc.top, rc.right, rc.bottom);
        let tall = RECT {
            bottom: rc.bottom + radius * 2,
            ..*rc
        };
        round_rect(hdc, &tall, radius, c, c, 0);
        let _ = RestoreDC(hdc, saved);
    }
}

/// Khung tiêu điểm: viền `TEXT` dày 1 DIP, vẽ phía trong `rc`.
pub fn focus_frame(hdc: HDC, rc: &RECT, dpi: u32) {
    let mut r = *rc;
    for _ in 0..scale(1, dpi).max(1) {
        if r.right - r.left < 2 || r.bottom - r.top < 2 {
            return;
        }
        // SAFETY: hdc hợp lệ; chổi dùng chung.
        unsafe {
            FrameRect(hdc, &r, brush(TEXT));
        }
        r.left += 1;
        r.top += 1;
        r.right -= 1;
        r.bottom -= 1;
    }
}

/// Đoạn gấp khúc qua `pts`, bút màu `c` dày `width` px (tạo rồi xóa ngay).
pub fn polyline(hdc: HDC, pts: &[POINT], c: Rgb, width: i32) {
    // SAFETY: bút tạo ở đây được bỏ chọn rồi xóa trước khi trả.
    unsafe {
        let pen = CreatePen(PS_SOLID, width.max(1), colorref(c));
        if pen.is_invalid() {
            return;
        }
        let old = SelectObject(hdc, HGDIOBJ(pen.0));
        let _ = Polyline(hdc, pts);
        SelectObject(hdc, old);
        let _ = DeleteObject(HGDIOBJ(pen.0));
    }
}

/// Viết `text` (UTF-16, không có số 0 cuối) trong `rc` bằng phông `font`, màu `c`, nền trong suốt.
/// Có `DT_CALCRECT` trong `flags` thì chỉ đo. Trả khung chữ (sau khi đo) hoặc `rc`.
pub fn draw_text(
    hdc: HDC,
    text: &[u16],
    rc: &RECT,
    c: Rgb,
    font: Option<HFONT>,
    flags: DRAW_TEXT_FORMAT,
) -> RECT {
    let mut r = *rc;
    if text.is_empty() {
        return RECT {
            right: r.left,
            bottom: r.top,
            ..r
        };
    }
    let mut buf = text.to_vec();
    // SAFETY: phông cũ được chọn lại trước khi trả; bộ đệm chữ sống suốt lời gọi.
    unsafe {
        let old = font.map(|f| SelectObject(hdc, HGDIOBJ(f.0)));
        SetBkMode(hdc, TRANSPARENT);
        SetTextColor(hdc, colorref(c));
        DrawTextW(hdc, &mut buf, &mut r, flags);
        if let Some(o) = old {
            SelectObject(hdc, o);
        }
    }
    r
}

/// Chữ của cửa sổ `hwnd` dạng UTF-16 (không có số 0 cuối).
pub fn window_text(hwnd: HWND) -> Vec<u16> {
    // SAFETY: bộ đệm đủ chỗ cho chuỗi cộng ký tự kết thúc.
    unsafe {
        let n = GetWindowTextLengthW(hwnd);
        if n <= 0 {
            return Vec::new();
        }
        let mut buf = vec![0u16; n as usize + 1];
        let got = GetWindowTextW(hwnd, &mut buf).max(0) as usize;
        buf.truncate(got.min(n as usize));
        buf
    }
}

/// Phông đang đặt cho điều khiển `hwnd` (`WM_GETFONT`), không có thì `None`.
pub fn control_font(hwnd: HWND) -> Option<HFONT> {
    // SAFETY: chỉ hỏi phông của điều khiển; phông vẫn thuộc về cửa sổ chủ.
    let f = unsafe { SendMessageW(hwnd, WM_GETFONT, None, None).0 };
    (f != 0).then_some(HFONT(f as *mut c_void))
}

/// Vẽ vào bitmap tạm có cùng tọa độ với `rc` rồi chép một lần lên `hdc` (không nháy).
/// Không tạo được bitmap thì vẽ thẳng lên `hdc`.
pub fn buffered(hdc: HDC, rc: &RECT, draw: impl FnOnce(HDC)) {
    let (w, h) = (rc.right - rc.left, rc.bottom - rc.top);
    if w <= 0 || h <= 0 {
        return;
    }
    // SAFETY: DC và bitmap tạm tạo ở đây được bỏ chọn và xóa trên mọi đường đi.
    unsafe {
        let mem = CreateCompatibleDC(Some(hdc));
        if mem.is_invalid() {
            draw(hdc);
            return;
        }
        let bmp = CreateCompatibleBitmap(hdc, w, h);
        if bmp.is_invalid() {
            let _ = DeleteDC(mem);
            draw(hdc);
            return;
        }
        let old = SelectObject(mem, HGDIOBJ(bmp.0));
        let _ = SetWindowOrgEx(mem, rc.left, rc.top, None);
        draw(mem);
        let _ = BitBlt(
            hdc,
            rc.left,
            rc.top,
            w,
            h,
            Some(mem),
            rc.left,
            rc.top,
            SRCCOPY,
        );
        SelectObject(mem, old);
        let _ = DeleteObject(HGDIOBJ(bmp.0));
        let _ = DeleteDC(mem);
    }
}

/// Trạng thái của nút `hwnd` theo `BM_GETSTATE`, bật/tắt và tiêu điểm.
pub fn button_state(hwnd: HWND) -> ControlState {
    // SAFETY: chỉ đọc trạng thái của nút thuộc luồng này.
    let (st, enabled, focus) = unsafe {
        (
            SendMessageW(hwnd, BM_GETSTATE, None, None).0 as u32,
            IsWindowEnabled(hwnd).as_bool(),
            GetFocus(),
        )
    };
    ControlState {
        hot: st & BST_HOT != 0,
        pressed: st & BST_PUSHED != 0,
        focused: focus == hwnd,
        enabled,
        checked: st & BST_CHECKED.0 != 0,
    }
}

/// Kiểu vẽ của một `BUTTON` theo kiểu cửa sổ của nó.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Look {
    Push,
    Check,
    Radio,
}

fn look_of(hwnd: HWND) -> Look {
    // SAFETY: chỉ đọc kiểu cửa sổ.
    let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) } as u32 & 0xF;
    match style {
        // BS_CHECKBOX, BS_AUTOCHECKBOX, BS_3STATE, BS_AUTO3STATE.
        0x2 | 0x3 | 0x5 | 0x6 => Look::Check,
        // BS_RADIOBUTTON, BS_AUTORADIOBUTTON.
        0x4 | 0x9 => Look::Radio,
        _ => Look::Push,
    }
}

/// Vẽ nút bấm `kind` vào `rc`: nền và viền bo 4 DIP, chữ giữa, khung tiêu điểm phía trong.
#[allow(clippy::too_many_arguments)]
pub fn draw_button(
    hdc: HDC,
    rc: &RECT,
    text: &[u16],
    kind: ButtonKind,
    state: ControlState,
    dpi: u32,
    font: Option<HFONT>,
    cues: (bool, bool),
) {
    let c = button_colors(kind, state);
    let bw = if kind == ButtonKind::Primary {
        0
    } else {
        scale(1, dpi).max(1)
    };
    round_rect(hdc, rc, scale(RADIUS, dpi), c.fill, c.border, bw);
    draw_text(
        hdc,
        text,
        rc,
        c.text,
        font,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE | prefix_flag(cues.1),
    );
    if state.focused && cues.0 {
        let inset = scale(3, dpi);
        let fr = RECT {
            left: rc.left + inset,
            top: rc.top + inset,
            right: rc.right - inset,
            bottom: rc.bottom - inset,
        };
        focus_frame(hdc, &fr, dpi);
    }
}

// ---------------------------------------------------------------------------
// Hình mịn: độ phủ của pixel tính từ khoảng cách có dấu tới cạnh hình, trộn lên DC bằng `AlphaBlend`
// ---------------------------------------------------------------------------

/// Khoảng cách có dấu (âm là bên trong) từ (`x`, `y`) tới đường tròn tâm (`cx`, `cy`) bán kính `r`.
fn sd_circle(x: f32, y: f32, cx: f32, cy: f32, r: f32) -> f32 {
    (x - cx).hypot(y - cy) - r
}

/// Khoảng cách có dấu tới hình vuông tâm (`cx`, `cy`), nửa cạnh `half`, bo góc `r`.
fn sd_round_rect(x: f32, y: f32, cx: f32, cy: f32, half: f32, r: f32) -> f32 {
    let r = r.clamp(0.0, half.max(0.0));
    let qx = (x - cx).abs() - half + r;
    let qy = (y - cy).abs() - half + r;
    qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - r
}

/// Khoảng cách có dấu tới nét thẳng đầu bằng từ `a` tới `b`, nửa bề dày `h`.
fn sd_bar(x: f32, y: f32, a: (f32, f32), b: (f32, f32), h: f32) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len = dx.hypot(dy);
    let (px, py) = (x - (a.0 + b.0) / 2.0, y - (a.1 + b.1) / 2.0);
    if len <= f32::EPSILON {
        return px.hypot(py) - h;
    }
    let (ux, uy) = (dx / len, dy / len);
    let u = (px * ux + py * uy).abs() - len / 2.0;
    let v = (py * ux - px * uy).abs() - h;
    u.max(0.0).hypot(v.max(0.0)) + u.max(v).min(0.0)
}

/// Hai nét của dấu tích (nét ngắn bên trái, nét dài bên phải; mỗi nét từ đầu tự do tới mép ngoài của
/// góc) trong hộp cạnh `s` px. Theo CSS của hình mẫu ở hộp 18: chữ L 5×9 có viền phải và dưới 2 px,
/// xoay 45° sau khi dời (-1, -1), đặt giữa hộp.
fn tick_arms(s: f32) -> [((f32, f32), (f32, f32)); 2] {
    let k = s / 18.0;
    let c = s / 2.0;
    let f = std::f32::consts::FRAC_1_SQRT_2;
    // (x, y) là tọa độ trên đường giữa của viền, gốc ở tâm chữ L chưa xoay.
    let p = |x: f32, y: f32| {
        let (x, y) = (x - 1.0, y - 1.0);
        (c + (x - y) * f * k, c + (x + y) * f * k)
    };
    [(p(-3.5, 4.5), p(3.5, 4.5)), (p(2.5, -5.5), p(2.5, 5.5))]
}

/// Lớp vẽ vuông cạnh `n` px, mỗi pixel là (r, g, b, a) nhân trước alpha trong [0, 1].
struct Layer {
    n: usize,
    px: Vec<[f32; 4]>,
}

impl Layer {
    fn new(n: usize) -> Layer {
        Layer {
            n,
            px: vec![[0.0; 4]; n * n],
        }
    }

    /// Phủ màu `c` lên phần có `sdf < 0`; pixel ở cạnh phủ theo khoảng cách từ tâm pixel tới cạnh.
    fn paint(&mut self, c: Rgb, sdf: impl Fn(f32, f32) -> f32) {
        let col = [c.0, c.1, c.2].map(|v| v as f32 / 255.0);
        for y in 0..self.n {
            for x in 0..self.n {
                let cov = (0.5 - sdf(x as f32 + 0.5, y as f32 + 0.5)).clamp(0.0, 1.0);
                if cov <= 0.0 {
                    continue;
                }
                let p = &mut self.px[y * self.n + x];
                let keep = 1.0 - cov;
                for i in 0..3 {
                    p[i] = col[i] * cov + p[i] * keep;
                }
                p[3] = cov + p[3] * keep;
            }
        }
    }

    /// Pixel `0xAARRGGBB` (BGRA trong bộ nhớ) nhân trước alpha, đúng định dạng `AlphaBlend` cần.
    fn into_bgra(self) -> Vec<u32> {
        let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
        self.px
            .into_iter()
            .map(|[r, g, b, a]| {
                let a = q(a);
                (a << 24) | (q(r).min(a) << 16) | (q(g).min(a) << 8) | q(b).min(a)
            })
            .collect()
    }
}

/// Ô tích cạnh `n` px ở tỉ lệ `k` px/DIP: hộp bo `RADIUS`, viền 1,5 DIP màu `c.border`, lòng `c.fill`;
/// bật thì thêm dấu tích trắng nét 2 DIP.
fn check_mark(n: usize, k: f32, c: Colors, checked: bool) -> Vec<u32> {
    let mut l = Layer::new(n);
    let (m, half) = (n as f32 / 2.0, n as f32 / 2.0);
    let (r, bw) = (RADIUS as f32 * k, BORDER_DIP * k);
    l.paint(c.border, |x, y| sd_round_rect(x, y, m, m, half, r));
    if c.fill != c.border {
        l.paint(c.fill, |x, y| sd_round_rect(x, y, m, m, half - bw, r - bw));
    }
    if checked {
        let [a, b] = tick_arms(n as f32);
        let h = TICK_DIP * k / 2.0;
        l.paint(WHITE, |x, y| {
            sd_bar(x, y, a.0, a.1, h).min(sd_bar(x, y, b.0, b.1, h))
        });
    }
    l.into_bgra()
}

/// Nút chọn cạnh `n` px ở tỉ lệ `k`: vòng viền 1,5 DIP màu `c.border`, lòng trong suốt; bật thì thêm chấm
/// `DOT` DIP màu `c.fill` ở giữa.
fn radio_mark(n: usize, k: f32, c: Colors, checked: bool) -> Vec<u32> {
    let mut l = Layer::new(n);
    let (m, r) = (n as f32 / 2.0, n as f32 / 2.0);
    let bw = BORDER_DIP * k;
    l.paint(c.border, |x, y| {
        sd_circle(x, y, m, m, r).max(-sd_circle(x, y, m, m, r - bw))
    });
    if checked {
        let d = DOT as f32 * k / 2.0;
        l.paint(c.fill, |x, y| sd_circle(x, y, m, m, d));
    }
    l.into_bgra()
}

/// Núm tròn cạnh `n` px: viền `bw_dip` DIP màu `border`, lòng `fill`.
fn knob_mark(n: usize, k: f32, fill: Rgb, border: Rgb, bw_dip: f32) -> Vec<u32> {
    let mut l = Layer::new(n);
    let m = n as f32 / 2.0;
    l.paint(border, |x, y| sd_circle(x, y, m, m, m));
    l.paint(fill, |x, y| sd_circle(x, y, m, m, m - bw_dip * k));
    l.into_bgra()
}

/// Trộn bộ đệm `px` (cạnh `n`, BGRA nhân trước alpha) lên `hdc` tại (`x`, `y`) bằng `AlphaBlend`.
/// Trả `false` khi không làm được (bên gọi vẽ lại bằng GDI thường); mọi đối tượng tạm đều được xóa.
fn blend(hdc: HDC, x: i32, y: i32, n: usize, px: &[u32]) -> bool {
    let Ok(side) = i32::try_from(n) else {
        return false;
    };
    if n == 0 || px.len() != n * n {
        return false;
    }
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: side,
            biHeight: -side, // âm: từ trên xuống
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    // SAFETY: DC và DIB tạm tạo ở đây được bỏ chọn và xóa trên mọi đường đi; chép đúng n*n pixel vào
    // vùng nhớ n*n*4 byte của DIB vừa tạo.
    unsafe {
        let mem = CreateCompatibleDC(Some(hdc));
        if mem.is_invalid() {
            return false;
        }
        let mut bits: *mut c_void = std::ptr::null_mut();
        let bmp = match CreateDIBSection(Some(mem), &bmi, DIB_RGB_COLORS, &mut bits, None, 0) {
            Ok(b) => b,
            Err(_) => {
                let _ = DeleteDC(mem);
                return false;
            }
        };
        if bits.is_null() {
            let _ = DeleteObject(HGDIOBJ(bmp.0));
            let _ = DeleteDC(mem);
            return false;
        }
        std::ptr::copy_nonoverlapping(px.as_ptr(), bits as *mut u32, n * n);
        let old = SelectObject(mem, HGDIOBJ(bmp.0));
        let bf = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        let ok = AlphaBlend(hdc, x, y, side, side, mem, 0, 0, side, side, bf).as_bool();
        SelectObject(mem, old);
        let _ = DeleteObject(HGDIOBJ(bmp.0));
        let _ = DeleteDC(mem);
        ok
    }
}

/// Cách vẽ GDI thường (răng cưa), chỉ dùng khi `blend` không làm được.
fn draw_check_gdi(hdc: HDC, bx: &RECT, radio: bool, c: Colors, checked: bool, dpi: u32) {
    let s = bx.right - bx.left;
    let bw = (scale(3, dpi) / 2).max(1);
    if radio {
        ellipse(hdc, bx, SURFACE, c.border, bw);
        if checked {
            let d = scale(DOT, dpi);
            let off = (s - d) / 2;
            let dot = RECT {
                left: bx.left + off,
                top: bx.top + off,
                right: bx.left + off + d,
                bottom: bx.top + off + d,
            };
            ellipse(hdc, &dot, c.fill, c.fill, 0);
        }
    } else {
        round_rect(hdc, bx, scale(RADIUS, dpi), c.fill, c.border, bw);
        if checked {
            let p = |fx: i32, fy: i32| POINT {
                x: bx.left + s * fx / 100,
                y: bx.top + s * fy / 100,
            };
            polyline(
                hdc,
                &[p(27, 50), p(43, 66), p(73, 35)],
                WHITE,
                scale(2, dpi),
            );
        }
    }
}

/// Vẽ ô tích (`radio == false`) hoặc nút chọn vào `rc`: hộp 18 DIP bên trái (khử răng cưa), chữ bên
/// phải, khung tiêu điểm quanh chữ.
#[allow(clippy::too_many_arguments)]
pub fn draw_check(
    hdc: HDC,
    rc: &RECT,
    text: &[u16],
    radio: bool,
    state: ControlState,
    dpi: u32,
    font: Option<HFONT>,
    cues: (bool, bool),
) {
    let c = if radio {
        radio_colors(state)
    } else {
        check_colors(state)
    };
    let s = scale(BOX, dpi);
    let top = rc.top + (rc.bottom - rc.top - s) / 2;
    let bx = RECT {
        left: rc.left,
        top,
        right: rc.left + s,
        bottom: top + s,
    };
    let n = s.max(0) as usize;
    let k = dpi as f32 / 96.0;
    let px = if radio {
        radio_mark(n, k, c, state.checked)
    } else {
        check_mark(n, k, c, state.checked)
    };
    if !blend(hdc, bx.left, bx.top, n, &px) {
        draw_check_gdi(hdc, &bx, radio, c, state.checked, dpi);
    }
    let tr = RECT {
        left: bx.right + scale(BOX_GAP, dpi),
        ..*rc
    };
    let flags = DT_LEFT | DT_VCENTER | DT_SINGLELINE | prefix_flag(cues.1);
    draw_text(hdc, text, &tr, c.text, font, flags | DT_END_ELLIPSIS);
    if state.focused && cues.0 && !text.is_empty() {
        let m = draw_text(hdc, text, &tr, c.text, font, flags | DT_CALCRECT);
        let h = m.bottom - m.top;
        let pad = scale(2, dpi);
        let t = rc.top + (rc.bottom - rc.top - h) / 2;
        let fr = RECT {
            left: m.left - pad,
            top: (t - pad).max(rc.top),
            right: (m.right + pad).min(rc.right),
            bottom: (t + h + pad).min(rc.bottom),
        };
        focus_frame(hdc, &fr, dpi);
    }
}

/// Xử lý `NM_CUSTOMDRAW` của một `BUTTON` (nút bấm, ô tích, nút chọn; Common Controls v6): vẽ hết ở
/// `CDDS_PREPAINT` rồi trả `CDRF_SKIPDEFAULT`, nên điều khiển gốc vẫn lo bàn phím, tiêu điểm và trợ năng.
/// `kind` chỉ dùng cho nút bấm; `bg` là nền cửa sổ cha; `bold` là phông đậm của nút chính (thiếu thì
/// dùng phông của nút).
pub fn custom_draw_button(
    nmcd: &NMCUSTOMDRAW,
    kind: ButtonKind,
    bg: Rgb,
    dpi: u32,
    bold: Option<HFONT>,
) -> LRESULT {
    if nmcd.dwDrawStage != CDDS_PREPAINT {
        return LRESULT(CDRF_DODEFAULT as isize);
    }
    let hwnd = nmcd.hdr.hwndFrom;
    let rc = nmcd.rc;
    let state = button_state(hwnd);
    let text = window_text(hwnd);
    let cues = ui_cues(hwnd);
    let font = control_font(hwnd);
    let look = look_of(hwnd);
    buffered(nmcd.hdc, &rc, |dc| {
        fill(dc, &rc, bg);
        match look {
            Look::Push => {
                let f = if kind == ButtonKind::Primary {
                    bold.or(font)
                } else {
                    font
                };
                draw_button(dc, &rc, &text, kind, state, dpi, f, cues);
            }
            Look::Check => draw_check(dc, &rc, &text, false, state, dpi, font, cues),
            Look::Radio => draw_check(dc, &rc, &text, true, state, dpi, font, cues),
        }
    });
    LRESULT(CDRF_SKIPDEFAULT as isize)
}

/// Xử lý `NM_CUSTOMDRAW` của thanh trượt ngang: rãnh `TRACK`, phần đã qua `ACCENT`, núm tròn `TEXT`
/// viền `ACCENT`; mờ khi vô hiệu; khung tiêu điểm khi đi bằng bàn phím. Vẽ hết ở `CDDS_PREPAINT`.
pub fn custom_draw_trackbar(nmcd: &NMCUSTOMDRAW, bg: Rgb, dpi: u32) -> LRESULT {
    if nmcd.dwDrawStage != CDDS_PREPAINT {
        return LRESULT(CDRF_DODEFAULT as isize);
    }
    let hwnd = nmcd.hdr.hwndFrom;
    let mut client = RECT::default();
    let (mut channel, mut thumb) = (RECT::default(), RECT::default());
    // SAFETY: các RECT là biến cục bộ, thanh trượt chỉ ghi vào chúng; còn lại chỉ đọc trạng thái.
    let (enabled, focused) = unsafe {
        let _ = GetClientRect(hwnd, &mut client);
        SendMessageW(
            hwnd,
            TBM_GETCHANNELRECT,
            None,
            Some(LPARAM(&mut channel as *mut RECT as isize)),
        );
        SendMessageW(
            hwnd,
            TBM_GETTHUMBRECT,
            None,
            Some(LPARAM(&mut thumb as *mut RECT as isize)),
        );
        (IsWindowEnabled(hwnd).as_bool(), GetFocus() == hwnd)
    };
    let (show_focus, _) = ui_cues(hwnd);
    let (done_c, knob_fill, knob_border) = if enabled {
        (ACCENT, TEXT, ACCENT)
    } else {
        (TEXT_DISABLED, TEXT_DISABLED, TRACK)
    };
    buffered(nmcd.hdc, &client, |dc| {
        fill(dc, &client, bg);
        let cy = (channel.top + channel.bottom) / 2;
        let th = scale(TRACK_H, dpi).max(2);
        let r = scale(KNOB, dpi) / 2;
        let lo = client.left + r;
        let hi = (client.right - r).max(lo);
        let knob_x = ((thumb.left + thumb.right) / 2).clamp(lo, hi);
        let track = RECT {
            left: channel.left,
            top: cy - th / 2,
            right: channel.right,
            bottom: cy - th / 2 + th,
        };
        round_rect(dc, &track, th / 2, TRACK, TRACK, 0);
        let done = RECT {
            right: knob_x.min(track.right),
            ..track
        };
        if done.right > done.left {
            round_rect(dc, &done, th / 2, done_c, done_c, 0);
        }
        let knob = RECT {
            left: knob_x - r,
            top: cy - r,
            right: knob_x + r,
            bottom: cy + r,
        };
        let px = knob_mark(
            (2 * r).max(0) as usize,
            dpi as f32 / 96.0,
            knob_fill,
            knob_border,
            KNOB_BORDER as f32,
        );
        if !blend(dc, knob.left, knob.top, (2 * r).max(0) as usize, &px) {
            ellipse(dc, &knob, knob_fill, knob_border, scale(KNOB_BORDER, dpi));
        }
        if focused && show_focus {
            focus_frame(dc, &client, dpi);
        }
    });
    LRESULT(CDRF_SKIPDEFAULT as isize)
}

/// `NMCUSTOMDRAW` của một `WM_NOTIFY` mang `NM_CUSTOMDRAW`.
///
/// # Safety
/// `lp` phải là lParam của một `WM_NOTIFY` có mã `NM_CUSTOMDRAW` đang được xử lý.
pub unsafe fn nmcd_of<'a>(lp: LPARAM) -> Option<&'a NMCUSTOMDRAW> {
    // SAFETY: theo hợp đồng của hàm, lp trỏ tới NMCUSTOMDRAW hợp lệ suốt lời gọi WM_NOTIFY.
    unsafe { (lp.0 as *const NMCUSTOMDRAW).as_ref() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkshot_core::i18n::HelpKeys;
    use darkshot_core::theme::{BG, TEXT};
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::System::Threading::{GetCurrentProcess, GetGuiResources, GR_GDIOBJECTS};
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, PeekMessageW,
        RegisterClassW, TranslateMessage, MSG, PM_REMOVE, WINDOW_EX_STYLE, WNDCLASSW, WS_CAPTION,
    };

    use darkshot_core::theme::{
        ACCENT, ACCENT_BUTTON, ACCENT_BUTTON_HOVER, ACCENT_BUTTON_PRESSED, BUTTON, BUTTON_BORDER,
        BUTTON_HOVER, BUTTON_PRESSED, CHECK_BORDER, INPUT_BORDER, SURFACE, TAB_HOVER,
        TEXT_DISABLED, TEXT_MUTED, TEXT_PAIRS, WHITE,
    };

    fn st(hot: bool, pressed: bool, focused: bool, enabled: bool, checked: bool) -> ControlState {
        ControlState {
            hot,
            pressed,
            focused,
            enabled,
            checked,
        }
    }

    const NORMAL: ControlState = ControlState {
        hot: false,
        pressed: false,
        focused: false,
        enabled: true,
        checked: false,
    };

    /// Mọi tổ hợp trạng thái.
    fn all_states() -> Vec<ControlState> {
        let mut v = Vec::new();
        for bits in 0..32u8 {
            let b = |i: u8| bits & (1 << i) != 0;
            v.push(st(b(0), b(1), b(2), b(3), b(4)));
        }
        v
    }

    #[test]
    fn primary_button_colors() {
        let c = button_colors(ButtonKind::Primary, NORMAL);
        assert_eq!((c.fill, c.text), (ACCENT_BUTTON, WHITE));
        let hot = button_colors(
            ButtonKind::Primary,
            ControlState {
                hot: true,
                ..NORMAL
            },
        );
        assert_eq!(hot.fill, ACCENT_BUTTON_HOVER);
        let pressed = button_colors(
            ButtonKind::Primary,
            ControlState {
                hot: true,
                pressed: true,
                ..NORMAL
            },
        );
        assert_eq!(pressed.fill, ACCENT_BUTTON_PRESSED);
        let off = button_colors(
            ButtonKind::Primary,
            ControlState {
                enabled: false,
                ..NORMAL
            },
        );
        assert_eq!(off.text, TEXT_DISABLED);
    }

    #[test]
    fn normal_button_colors() {
        assert_eq!(
            button_colors(ButtonKind::Normal, NORMAL),
            Colors {
                fill: BUTTON,
                border: BUTTON_BORDER,
                text: TEXT
            }
        );
        let hot = button_colors(
            ButtonKind::Normal,
            ControlState {
                hot: true,
                ..NORMAL
            },
        );
        assert_eq!(hot.fill, BUTTON_HOVER);
        let pressed = button_colors(
            ButtonKind::Normal,
            ControlState {
                hot: true,
                pressed: true,
                ..NORMAL
            },
        );
        assert_eq!(pressed.fill, BUTTON_PRESSED);
        let off = button_colors(
            ButtonKind::Normal,
            ControlState {
                enabled: false,
                ..NORMAL
            },
        );
        assert_eq!(off.text, TEXT_DISABLED);
    }

    #[test]
    fn check_and_radio_colors() {
        // Ô tích: tắt viền CHECK_BORDER, bật nền ACCENT_BUTTON; chữ TEXT hoặc TEXT_DISABLED.
        let off = check_colors(NORMAL);
        assert_eq!((off.border, off.text), (CHECK_BORDER, TEXT));
        let on = check_colors(ControlState {
            checked: true,
            ..NORMAL
        });
        assert_eq!((on.fill, on.text), (ACCENT_BUTTON, TEXT));
        for checked in [false, true] {
            let dis = check_colors(ControlState {
                checked,
                enabled: false,
                ..NORMAL
            });
            assert_eq!(dis.text, TEXT_DISABLED, "checked={checked}");
            assert_ne!(dis.fill, ACCENT_BUTTON, "checked={checked}");
        }
        // Nút chọn: bật viền và chấm ACCENT; tắt viền CHECK_BORDER.
        let off = radio_colors(NORMAL);
        assert_eq!((off.border, off.text), (CHECK_BORDER, TEXT));
        let on = radio_colors(ControlState {
            checked: true,
            ..NORMAL
        });
        assert_eq!((on.border, on.fill, on.text), (ACCENT, ACCENT, TEXT));
        for checked in [false, true] {
            let dis = radio_colors(ControlState {
                checked,
                enabled: false,
                ..NORMAL
            });
            assert_eq!(dis.text, TEXT_DISABLED, "checked={checked}");
            assert_ne!(dis.border, ACCENT, "checked={checked}");
        }
    }

    #[test]
    fn tab_colors_selected_and_hot() {
        let sel = tab_colors(true, false);
        assert_eq!((sel.text, sel.border, sel.fill), (TEXT, ACCENT, BG));
        let other = tab_colors(false, false);
        assert_eq!((other.text, other.fill), (TEXT_MUTED, BG));
        assert_eq!(tab_colors(false, true).fill, TAB_HOVER);
        assert_eq!(tab_colors(true, true).fill, TAB_HOVER);
        assert_eq!(tab_colors(true, true).border, ACCENT);
    }

    #[test]
    fn edit_border_focus() {
        assert_eq!(edit_border(true, true), (ACCENT, 2));
        assert_eq!(edit_border(false, true), (INPUT_BORDER, 1));
        assert_eq!(edit_border(false, false).1, 1);
    }

    /// Pixel BGRA nhân trước alpha (`0xAARRGGBB`) thành (r, g, b, a).
    fn unpack(p: u32) -> (u8, u8, u8, u8) {
        ((p >> 16) as u8, (p >> 8) as u8, p as u8, (p >> 24) as u8)
    }

    fn opaque(c: Rgb) -> (u8, u8, u8, u8) {
        (c.0, c.1, c.2, 255)
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }

    fn pt_close(a: (f32, f32), b: (f32, f32)) -> bool {
        close(a.0, b.0) && close(a.1, b.1)
    }

    /// Pixel (`x`, `y`) của bộ đệm cạnh `n`.
    fn at(px: &[u32], n: usize, x: usize, y: usize) -> (u8, u8, u8, u8) {
        unpack(px[y * n + x])
    }

    /// Mọi pixel nhân trước alpha (kênh màu không vượt alpha) và có ít nhất một pixel phủ một phần
    /// (cạnh được khử răng cưa).
    fn assert_premultiplied_and_smooth(px: &[u32]) {
        for &p in px {
            let (r, g, b, a) = unpack(p);
            assert!(r <= a && g <= a && b <= a, "{p:08X}");
        }
        assert!(px.iter().any(|&p| {
            let a = p >> 24;
            a > 0 && a < 255
        }));
    }

    /// Mỗi pixel bằng pixel đối xứng qua trục dọc và trục ngang của bộ đệm cạnh `n`.
    fn assert_symmetric(px: &[u32], n: usize) {
        for y in 0..n {
            for x in 0..n {
                assert_eq!(px[y * n + x], px[y * n + (n - 1 - x)], "({x},{y})");
                assert_eq!(px[y * n + x], px[(n - 1 - y) * n + x], "({x},{y})");
            }
        }
    }

    #[test]
    fn tick_arms_follow_prototype() {
        // CSS của hình mẫu: chữ L 5×9, viền phải và dưới 2 px, xoay 45° và dời (-1, -1), giữa hộp 18.
        let [short, long] = tick_arms(18.0);
        assert!(pt_close(short.0, (3.343, 8.293)), "{short:?}");
        assert!(pt_close(short.1, (8.293, 13.243)), "{short:?}");
        assert!(pt_close(long.0, (14.657, 5.464)), "{long:?}");
        assert!(pt_close(long.1, (6.879, 13.243)), "{long:?}");
        // Nét ngắn dài 7, nét dài 11 (tính cả phần góc vuông).
        let len =
            |(a, b): ((f32, f32), (f32, f32))| ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
        assert!(close(len(short), 7.0) && close(len(long), 11.0));
        // Tỉ lệ theo cạnh hộp.
        let [s2, l2] = tick_arms(36.0);
        assert!(pt_close(s2.0, (short.0 .0 * 2.0, short.0 .1 * 2.0)));
        assert!(pt_close(l2.1, (long.1 .0 * 2.0, long.1 .1 * 2.0)));
    }

    #[test]
    fn signed_distances() {
        assert!(close(sd_circle(9.0, 9.0, 9.0, 9.0, 4.0), -4.0));
        assert!(close(sd_circle(15.0, 9.0, 9.0, 9.0, 4.0), 2.0));
        // Hình vuông tâm (9, 9), nửa cạnh 9, bo 4.
        assert!(close(sd_round_rect(9.0, 0.0, 9.0, 9.0, 9.0, 4.0), 0.0));
        assert!(close(sd_round_rect(9.0, 9.0, 9.0, 9.0, 9.0, 4.0), -9.0));
        // Góc: tâm cung (4, 4), điểm (0, 0) cách cung 4√2 - 4.
        assert!(close(
            sd_round_rect(0.0, 0.0, 9.0, 9.0, 9.0, 4.0),
            32f32.sqrt() - 4.0
        ));
        // Thanh nét đầu bằng: giữa thanh là -nửa rộng, ngoài đầu thanh dương.
        assert!(close(sd_bar(5.0, 0.0, (0.0, 0.0), (10.0, 0.0), 1.0), -1.0));
        assert!(close(sd_bar(11.0, 0.0, (0.0, 0.0), (10.0, 0.0), 1.0), 1.0));
        assert!(close(sd_bar(5.0, 3.0, (0.0, 0.0), (10.0, 0.0), 1.0), 2.0));
    }

    #[test]
    fn check_mark_pixels() {
        let n = 18;
        let off = check_mark(n, 1.0, check_colors(NORMAL), false);
        assert_eq!(off.len(), n * n);
        assert_eq!(at(&off, n, 9, 9), opaque(SURFACE));
        // Viền 1,5 px phủ trọn pixel đầu của cạnh trái.
        assert_eq!(at(&off, n, 0, 9), opaque(CHECK_BORDER));
        // Góc bo: ngoài hình, trong suốt.
        assert_eq!(at(&off, n, 0, 0).3, 0);
        assert_premultiplied_and_smooth(&off);
        assert_symmetric(&off, n);

        let on_state = st(false, false, false, true, true);
        let on = check_mark(n, 1.0, check_colors(on_state), true);
        assert_eq!(at(&on, n, 4, 3), opaque(ACCENT_BUTTON));
        // Góc của dấu tích trắng.
        assert_eq!(at(&on, n, 7, 12), opaque(WHITE));
        assert_premultiplied_and_smooth(&on);

        // DPI 200%: cùng hình, gấp đôi.
        let big = check_mark(36, 2.0, check_colors(on_state), true);
        assert_eq!(at(&big, 36, 14, 24), opaque(WHITE));
        assert_eq!(at(&big, 36, 1, 18), opaque(ACCENT_BUTTON));
        assert_eq!(at(&big, 36, 0, 0).3, 0);
    }

    #[test]
    fn radio_mark_pixels() {
        let n = 18;
        let on = radio_mark(
            n,
            1.0,
            radio_colors(st(false, false, false, true, true)),
            true,
        );
        assert_eq!(on.len(), n * n);
        // Chấm ACCENT ở giữa; giữa chấm và vòng trong suốt (như hình mẫu).
        assert_eq!(at(&on, n, 9, 9), opaque(ACCENT));
        assert_eq!(at(&on, n, 14, 9).3, 0);
        assert_eq!(at(&on, n, 0, 0).3, 0);
        assert_premultiplied_and_smooth(&on);
        assert_symmetric(&on, n);

        let off = radio_mark(n, 1.0, radio_colors(NORMAL), false);
        // Tắt: không chấm, lòng vòng trong suốt.
        assert_eq!(at(&off, n, 9, 9).3, 0);
        // Vòng màu CHECK_BORDER: pixel trên vòng có màu (sau khi bỏ nhân alpha) đúng màu viền.
        let (r, g, b, a) = at(&off, n, 0, 9);
        assert!(a > 200, "{a}");
        let un = |c: u8| (c as u32 * 255 + a as u32 / 2) / a as u32;
        let want = CHECK_BORDER;
        assert!(un(r).abs_diff(want.0 as u32) <= 2 && un(g).abs_diff(want.1 as u32) <= 2);
        assert!(un(b).abs_diff(want.2 as u32) <= 2);
        assert_premultiplied_and_smooth(&off);
        assert_symmetric(&off, n);
    }

    #[test]
    fn knob_mark_pixels() {
        let n = 18;
        let px = knob_mark(n, 1.0, TEXT, ACCENT, 4.0);
        assert_eq!(at(&px, n, 9, 9), opaque(TEXT));
        assert_eq!(at(&px, n, 2, 9), opaque(ACCENT));
        assert_eq!(at(&px, n, 0, 0).3, 0);
        assert_premultiplied_and_smooth(&px);
        assert_symmetric(&px, n);
    }

    /// Mọi cặp (chữ, nền dưới chữ) có thể vẽ khi điều khiển bật đều nằm trong `TEXT_PAIRS`, nên kiểm thử
    /// tương phản của core bao luôn chúng. Chữ của ô tích và nút chọn nằm trên nền khung (`SURFACE`),
    /// không nằm trên hộp, nên cặp của chúng là (text, SURFACE).
    #[test]
    fn every_drawn_text_pair_is_listed() {
        let listed = |fg: Rgb, bg: Rgb| TEXT_PAIRS.contains(&(fg, bg));
        for s in all_states().into_iter().filter(|s| s.enabled) {
            for kind in [ButtonKind::Normal, ButtonKind::Primary] {
                let c = button_colors(kind, s);
                assert!(listed(c.text, c.fill), "{kind:?} {s:?}: {c:?}");
            }
            let c = check_colors(s);
            assert!(listed(c.text, SURFACE), "check {s:?}: {c:?}");
            let c = radio_colors(s);
            assert!(listed(c.text, SURFACE), "radio {s:?}: {c:?}");
        }
        for selected in [false, true] {
            for hot in [false, true] {
                let c = tab_colors(selected, hot);
                assert!(listed(c.text, c.fill), "tab {selected} {hot}: {c:?}");
            }
        }
    }

    #[test]
    fn theme_brush_is_cached() {
        assert_eq!(brush(BG), brush(BG));
        assert_ne!(brush(BG), brush(TEXT));
    }

    unsafe extern "system" fn proc_(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
        // SAFETY: chuyển nguyên tham số.
        unsafe { DefWindowProcW(h, m, w, l) }
    }

    fn gdi_count() -> u32 {
        // SAFETY: chỉ đọc bộ đếm của tiến trình hiện tại.
        unsafe { GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS) }
    }

    fn pump() {
        let mut msg = MSG::default();
        // SAFETY: bơm thông điệp còn lại của luồng kiểm thử, không chờ.
        unsafe {
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    /// Vẽ ô tích, nút chọn (mọi trạng thái) và thanh trượt lên DC bộ nhớ 200 lần: số đối tượng GDI sau
    /// cùng bằng lúc đầu (DC và DIB tạm của `blend` được xóa). Bỏ qua khi chạy chung vì đếm theo tiến
    /// trình, kiểm thử khác chạy song song làm lệch số đếm.
    /// Chạy tay: `cargo test -p darkshot-windows -- --ignored smooth_marks_no_gdi_leak`.
    #[test]
    #[ignore]
    fn smooth_marks_no_gdi_leak() {
        use windows::Win32::Graphics::Gdi::{GetDC, ReleaseDC};
        // SAFETY: DC màn hình, DC bộ nhớ và bitmap tạo ở đây được trả và xóa ở cuối.
        unsafe {
            let screen = GetDC(None);
            let mem = CreateCompatibleDC(Some(screen));
            let bmp = CreateCompatibleBitmap(screen, 200, 60);
            let old = SelectObject(mem, HGDIOBJ(bmp.0));
            let rc = RECT {
                left: 0,
                top: 0,
                right: 200,
                bottom: 60,
            };
            let text: Vec<u16> = "Ab".encode_utf16().collect();
            let draw_all = || {
                for s in all_states() {
                    for radio in [false, true] {
                        for dpi in [96, 144, 192] {
                            draw_check(mem, &rc, &text, radio, s, dpi, None, (true, true));
                        }
                    }
                }
            };
            draw_all();
            let before = gdi_count();
            for _ in 0..200 {
                draw_all();
            }
            let after = gdi_count();
            SelectObject(mem, old);
            let _ = DeleteObject(HGDIOBJ(bmp.0));
            let _ = DeleteDC(mem);
            ReleaseDC(None, screen);
            assert_eq!(after, before);
        }
    }

    /// Mở rồi đóng Thông tin và Trợ giúp 20 lần: số đối tượng GDI không tăng.
    #[test]
    fn about_help_no_gdi_leak() {
        // SAFETY: cửa sổ cha ẩn riêng của kiểm thử, hủy ở cuối.
        let parent = unsafe {
            let hinst = GetModuleHandleW(None).unwrap();
            let class = w!("darkshot-theme-test-parent");
            let wc = WNDCLASSW {
                lpfnWndProc: Some(proc_),
                hInstance: hinst.into(),
                lpszClassName: class,
                ..Default::default()
            };
            RegisterClassW(&wc);
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                class,
                w!("parent"),
                WS_CAPTION,
                0,
                0,
                10,
                10,
                None,
                None,
                Some(hinst.into()),
                None,
            )
            .unwrap()
        };
        let keys = HelpKeys {
            region: Some("PrintScreen".into()),
            save_full: None,
            upload_full: Some("Ctrl+PrintScreen".into()),
        };
        let mut counts = Vec::new();
        for _ in 0..20 {
            crate::about::show(parent);
            crate::help::show(parent, &keys);
            pump();
            crate::about::close();
            crate::help::close();
            pump();
            counts.push(gdi_count());
        }
        assert!(
            counts[19] <= counts[0],
            "GDI objects grew across opens: {counts:?}"
        );
        // SAFETY: dọn cửa sổ cha.
        unsafe {
            let _ = DestroyWindow(parent);
        }
    }
}
