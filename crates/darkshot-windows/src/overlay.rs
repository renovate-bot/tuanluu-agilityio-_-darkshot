//! Lớp phủ toàn màn hình để chọn vùng.
//!
//! Cửa sổ `WS_POPUP` phủ đúng màn hình ảo, nên tọa độ client trùng tọa độ ảnh.
//! Mọi trạng thái kéo thả nằm trong `geometry::Selection`; ở đây chỉ vẽ và nhận sự kiện.

use crate::capture::Screenshot;
use crate::gdiplus::{GdiPlus, Gp};
use crate::lang;
use darkshot_core::actions::Action;
use darkshot_core::annotate::{Pt, Shape, Tool};
use darkshot_core::geometry::{
    monitor_containing, place_toolbar, Handle, Hit, Point, Rect, Selection,
};
use darkshot_core::i18n::Msg;
use darkshot_core::theme::{
    Rgb, ACCENT, OVERLAY_BORDER, OVERLAY_HOVER, OVERLAY_ICON, OVERLAY_LABEL, OVERLAY_TIP, SURFACE,
    TEXT, WHITE,
};
use std::cell::RefCell;
use std::ffi::c_void;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateDIBSection,
    CreateFontIndirectW, CreatePen, CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, EndPaint,
    FillRect, GetStockObject, GetTextExtentPoint32W, IntersectClipRect, InvalidateRect,
    MonitorFromPoint, Rectangle, RoundRect, SelectClipRgn, SelectObject, SetBkMode, SetTextColor,
    StretchDIBits, UpdateWindow, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, DT_CENTER,
    DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, HBITMAP, HDC, HFONT, HGDIOBJ, LOGFONTW,
    MONITOR_DEFAULTTONEAREST, NULL_BRUSH, OPAQUE, PAINTSTRUCT, PS_SOLID, SRCCOPY, TRANSPARENT,
};
use windows::Win32::Graphics::GdiPlus::{
    DashCapFlat, FillModeAlternate, GdipAddPathArc, GdipAddPathLine, GdipClosePathFigure,
    GdipCreatePath, GdipCreatePen1, GdipDeletePath, GdipDeletePen, GdipDrawPath,
    GdipSetPenLineCap197819, GdipSetPenLineJoin, GdipStartPathFigure, GpPath, LineCapRound,
    LineJoinRound, UnitPixel,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, SystemParametersInfoForDpi, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, RegisterHotKey, ReleaseCapture, SetCapture, SetFocus, UnregisterHotKey,
    MOD_NOREPEAT, VK_CONTROL, VK_ESCAPE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    GetClientRect, GetCursorPos, GetForegroundWindow, GetMessageW, GetWindowRect,
    GetWindowThreadProcessId, KillTimer, LoadCursorW, PostQuitMessage, RegisterClassExW, SetCursor,
    SetForegroundWindow, SetTimer, SetWindowPos, ShowWindow, TranslateMessage, HTCLIENT, IDC_ARROW,
    IDC_CROSS, IDC_SIZEALL, IDC_SIZENESW, IDC_SIZENS, IDC_SIZENWSE, IDC_SIZEWE, MSG,
    NONCLIENTMETRICSW, SPI_GETNONCLIENTMETRICS, SWP_NOACTIVATE, SWP_NOZORDER, SW_SHOW, WM_ACTIVATE,
    WM_CAPTURECHANGED, WM_CLOSE, WM_DESTROY, WM_DPICHANGED, WM_ERASEBKGND, WM_HOTKEY, WM_KEYDOWN,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_PAINT, WM_SETCURSOR, WM_TIMER, WNDCLASSEXW,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

const CLASS: PCWSTR = w!("darkshot-overlay");
const ACTIONS: [Action; 4] = [Action::Upload, Action::Save, Action::Copy, Action::Cancel];
const TIPS: [Msg; 4] = [
    Msg::OverlayUpload,
    Msg::OverlaySave,
    Msg::OverlayCopy,
    Msg::OverlayCancel,
];

// Kích thước ở 96 DPI (100%); nhân theo DPI của màn hình chứa phần tử.
const BUTTON: i32 = 36;
const GAP: i32 = 6;
const HANDLE_SIZE: i32 = 9;
const HANDLE_HIT: i32 = 6;
const ICON: i32 = 20;
/// Ô vạch ngăn của thanh dọc (vạch 1 px ở giữa, phần còn lại là lề).
const SIDE_SEP: i32 = 9;
/// Ô màu của bảng màu.
const CELL: i32 = 24;

/// ID phím tắt Esc tạm (khác ID phím chụp của cửa sổ chính).
const ESC_HOTKEY_ID: i32 = 0x0E5C;
const FOCUS_TIMER_ID: usize = 1;
const FOCUS_RETRY_MS: u32 = 100;
const MAX_FOCUS_RETRIES: u32 = 5;

// Màu lấy từ `darkshot_core::theme`; biểu tượng GDI+ dùng ARGB (0xFF + RGB).
const ICON_ARGB: u32 = argb(OVERLAY_ICON);
const ICON_HOVER_ARGB: u32 = argb(ACCENT);

const fn argb(c: Rgb) -> u32 {
    0xFF00_0000 | ((c.0 as u32) << 16) | ((c.1 as u32) << 8) | c.2 as u32
}

fn cr(c: Rgb) -> COLORREF {
    crate::theme::colorref(c)
}

/// Kết quả chọn vùng: hành động, vùng (tọa độ ảnh chụp) và các hình đã vẽ (tọa độ ảnh chụp).
#[derive(Clone, Debug, PartialEq)]
pub struct Picked {
    pub action: Action,
    pub rect: Rect,
    pub shapes: Vec<Shape>,
}

/// Mục của thanh công cụ vẽ (thanh dọc).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SideItem {
    Tool(Tool),
    Color,
    Undo,
}

/// Thứ tự trên thanh dọc; vạch ngăn nằm trước Màu và trước Hoàn tác.
const SIDE_ITEMS: [SideItem; 8] = [
    SideItem::Tool(Tool::Pen),
    SideItem::Tool(Tool::Line),
    SideItem::Tool(Tool::Arrow),
    SideItem::Tool(Tool::Rect),
    SideItem::Tool(Tool::Marker),
    SideItem::Tool(Tool::Text),
    SideItem::Color,
    SideItem::Undo,
];

/// Cỡ thanh dọc với nút `b` và ô vạch ngăn cao `sep` (gồm lề).
fn side_bar_size(b: i32, sep: i32) -> (i32, i32) {
    let _ = (b, sep);
    todo!()
}

/// Ô của mục thứ `i` trên thanh dọc `bar` (nút vuông cạnh `bar.w`).
fn side_item_rect(bar: Rect, sep: i32, i: usize) -> Rect {
    let _ = (bar, sep, i);
    todo!()
}

/// Mục của thanh dọc dưới điểm `p`; vạch ngăn và lề của nó trả `None`.
fn side_item_at(bar: Rect, sep: i32, p: Point) -> Option<SideItem> {
    let _ = (bar, sep, p);
    todo!()
}

/// Cỡ bảng màu 2×4 ô cạnh `cell`, khe và lề `gap`.
fn palette_size(cell: i32, gap: i32) -> (i32, i32) {
    let _ = (cell, gap);
    todo!()
}

/// Ô màu thứ `i` (hàng trên 0..4, hàng dưới 4..8) của bảng `pal`.
fn palette_cell_rect(pal: Rect, cell: i32, gap: i32, i: usize) -> Rect {
    let _ = (pal, cell, gap, i);
    todo!()
}

/// Ô màu dưới điểm `p`; khe và lề trả `None`.
fn palette_cell_at(pal: Rect, cell: i32, gap: i32, p: Point) -> Option<usize> {
    let _ = (pal, cell, gap, p);
    todo!()
}

/// Thanh bốn nút và thanh dọc cho vùng chọn `sel`, màn hình chứa góc dưới phải `mon` ở `dpi`.
fn bars_for(sel: Rect, mon: Rect, dpi: u32) -> (Rect, Rect) {
    let _ = (sel, mon, dpi);
    todo!()
}

/// Bảng màu cạnh nút Màu của thanh dọc `side`: bên trái thanh nếu thanh nằm nửa phải vùng chọn,
/// ngược lại bên phải; kẹp vào màn hình `mon`.
fn palette_rect_for(side: Rect, sel: Rect, mon: Rect, dpi: u32) -> Rect {
    let _ = (side, sel, mon, dpi);
    todo!()
}

/// Phím bổ trợ giả cho kiểm thử gửi thông điệp (bit 0: Ctrl, bit 1: Shift).
#[cfg(test)]
static TEST_MODS: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// Nét đang vẽ: công cụ, điểm đầu (tọa độ ảnh chụp) và hình hiện tại.
#[derive(Clone, Debug, PartialEq)]
struct Stroke {
    tool: Tool,
    start: Pt,
    shape: Shape,
}

/// Bắt đầu nét tại `p` với độ dày theo `dpi` của màn hình chứa `p`; `None` với Chữ.
fn begin_stroke(tool: Tool, p: Pt, dpi: u32, color: Rgb) -> Option<Stroke> {
    let _ = (tool, p, dpi, color);
    todo!()
}

/// Kéo nét tới `p` (`shift`: khóa góc 45° hoặc hình vuông). Trả vùng cần vẽ lại, `None` khi không đổi.
fn extend_stroke(s: &mut Stroke, p: Pt, shift: bool) -> Option<Rect> {
    let _ = (s, p, shift);
    todo!()
}

/// Nút của thanh công cụ dưới điểm `p`; bốn nút vuông xếp ngang theo thứ tự tải lên, lưu, chép, hủy.
pub fn button_at(toolbar: Rect, p: Point) -> Option<Action> {
    if toolbar.w <= 0 || !toolbar.contains(p) {
        return None;
    }
    let i = (i64::from(p.x - toolbar.x) * ACTIONS.len() as i64 / i64::from(toolbar.w)) as usize;
    ACTIONS.get(i).copied()
}

/// Vùng `r` nằm trọn trong ảnh BGRA `img_w`×`img_h` và bộ đệm dài `len` đủ chứa ảnh.
fn blit_in_bounds(r: Rect, img_w: i32, img_h: i32, len: usize) -> bool {
    let (x, y, w, h) = (
        i64::from(r.x),
        i64::from(r.y),
        i64::from(r.w),
        i64::from(r.h),
    );
    let (iw, ih) = (i64::from(img_w), i64::from(img_h));
    let need = iw.checked_mul(ih).and_then(|n| n.checked_mul(4));
    w > 0
        && h > 0
        && x >= 0
        && y >= 0
        && x + w <= iw
        && y + h <= ih
        && need.is_some_and(|n| n <= i64::try_from(len).unwrap_or(i64::MAX))
}

/// `v` pixel ở 96 DPI đổi sang pixel vật lý ở `dpi`, làm tròn, tối thiểu 1.
fn scaled(v: i32, dpi: u32) -> i32 {
    let dpi = i64::from(dpi.max(1));
    ((i64::from(v) * dpi + 48) / 96).clamp(1, i64::from(i32::MAX)) as i32
}

fn action_index(a: Action) -> usize {
    ACTIONS.iter().position(|x| *x == a).unwrap_or(0)
}

fn to_rect(r: Rect) -> RECT {
    RECT {
        left: r.x,
        top: r.y,
        right: r.right(),
        bottom: r.bottom(),
    }
}

fn intersect(a: Rect, b: Rect) -> Option<Rect> {
    let x0 = a.x.max(b.x);
    let y0 = a.y.max(b.y);
    let x1 = a.right().min(b.right());
    let y1 = a.bottom().min(b.bottom());
    (x1 > x0 && y1 > y0).then_some(Rect {
        x: x0,
        y: y0,
        w: x1 - x0,
        h: y1 - y0,
    })
}

fn inflate(r: Rect, d: i32) -> Rect {
    Rect {
        x: r.x - d,
        y: r.y - d,
        w: r.w + 2 * d,
        h: r.h + 2 * d,
    }
}

/// Đặt `r` vào trong `m` theo chiều ngang.
fn clamp_x(mut r: Rect, m: Rect) -> Rect {
    r.x = r.x.min(m.right() - r.w).max(m.x);
    r
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

fn point_from_lparam(lp: LPARAM) -> Point {
    Point {
        x: (lp.0 & 0xFFFF) as u16 as i16 as i32,
        y: ((lp.0 >> 16) & 0xFFFF) as u16 as i16 as i32,
    }
}

fn cursor_for_hit(hit: Hit) -> PCWSTR {
    match hit {
        Hit::Outside => IDC_CROSS,
        Hit::Inside => IDC_SIZEALL,
        Hit::Handle(Handle::N | Handle::S) => IDC_SIZENS,
        Hit::Handle(Handle::E | Handle::W) => IDC_SIZEWE,
        Hit::Handle(Handle::NW | Handle::SE) => IDC_SIZENWSE,
        Hit::Handle(Handle::NE | Handle::SW) => IDC_SIZENESW,
    }
}

fn set_cursor(id: PCWSTR) {
    // SAFETY: con trỏ chuẩn của hệ thống, không cần giải phóng.
    unsafe {
        if let Ok(c) = LoadCursorW(None, id) {
            SetCursor(Some(c));
        }
    }
}

/// Việc phải làm sau khi đã trả mượn trạng thái (các API này có thể gửi thông điệp đồng bộ).
#[derive(Default)]
struct Effects {
    capture: bool,
    release: bool,
}

struct State {
    hwnd: HWND,
    w: i32,
    h: i32,
    /// Ảnh gốc (`Screenshot::image.bgra`), sống suốt `pick_region`; chỉ đọc.
    bright: *const u8,
    bright_len: usize,
    dark: Vec<u8>,
    /// Bộ đệm vẽ cố định cỡ màn hình ảo; `None` thì mỗi lần vẽ tạo bitmap tạm.
    back: Option<BackBuffer>,
    /// Số lần đã thử lại giành tiêu điểm.
    focus_retries: u32,
    monitors: Vec<Rect>,
    dpis: Vec<u32>,
    sel: Selection,
    /// Lần kéo hiện tại là kéo khung mới (ẩn thanh công cụ).
    new_drag: bool,
    drag_cursor: PCWSTR,
    pressed: Option<Action>,
    hover: Option<Action>,
    result: Option<Option<(Action, Rect)>>,
    fonts: Vec<(u32, HFONT)>,
    /// DC bộ nhớ dùng để đo chữ.
    measure: HDC,
    gdiplus: bool,
}

impl Drop for State {
    fn drop(&mut self) {
        // SAFETY: font và DC do State tạo, xóa đúng một lần; không còn được chọn vào DC nào.
        unsafe {
            for (_, f) in self.fonts.drain(..) {
                let _ = DeleteObject(f.into());
            }
            if !self.measure.is_invalid() {
                let _ = DeleteDC(self.measure);
            }
        }
    }
}

/// DC bộ nhớ có DIB 32 bit cố định được chọn sẵn; giải phóng khi drop.
struct BackBuffer {
    dc: HDC,
    bmp: HBITMAP,
    old: HGDIOBJ,
}

impl BackBuffer {
    fn new(w: i32, h: i32) -> Option<BackBuffer> {
        let bmi = dib_info(w, h);
        let mut bits: *mut c_void = std::ptr::null_mut();
        // SAFETY: DC và DIB vừa tạo; mọi đường lỗi giải phóng phần đã tạo.
        unsafe {
            let dc = CreateCompatibleDC(None);
            if dc.is_invalid() {
                return None;
            }
            let Ok(bmp) = CreateDIBSection(Some(dc), &bmi, DIB_RGB_COLORS, &mut bits, None, 0)
            else {
                let _ = DeleteDC(dc);
                return None;
            };
            let old = SelectObject(dc, bmp.into());
            Some(BackBuffer { dc, bmp, old })
        }
    }
}

impl Drop for BackBuffer {
    fn drop(&mut self) {
        // SAFETY: bỏ chọn rồi xóa bitmap và DC do BackBuffer tạo, mỗi thứ đúng một lần.
        unsafe {
            SelectObject(self.dc, self.old);
            let _ = DeleteObject(self.bmp.into());
            let _ = DeleteDC(self.dc);
        }
    }
}

/// BITMAPINFO cho ảnh BGRA 32 bit từ trên xuống.
fn dib_info(w: i32, h: i32) -> BITMAPINFO {
    BITMAPINFO {
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
    }
}

thread_local! {
    static OVERLAY: RefCell<Option<State>> = const { RefCell::new(None) };
}

/// Chạy `f` với trạng thái lớp phủ; `None` nếu chưa có hoặc đang được mượn (gọi lồng).
fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    OVERLAY.with(|o| {
        let mut b = o.try_borrow_mut().ok()?;
        b.as_mut().map(f)
    })
}

impl State {
    fn monitor_dpi(&self, m: Rect) -> u32 {
        self.monitors
            .iter()
            .position(|x| *x == m)
            .and_then(|i| self.dpis.get(i).copied())
            .unwrap_or(96)
    }

    fn dpi_at(&self, p: Point) -> u32 {
        self.monitor_dpi(monitor_containing(p, &self.monitors))
    }

    fn max_dpi(&self) -> u32 {
        self.dpis.iter().copied().max().unwrap_or(96)
    }

    fn hit_radius(&self, p: Point) -> i32 {
        scaled(HANDLE_HIT, self.dpi_at(p))
    }

    fn toolbar(&self) -> Option<(Rect, u32)> {
        let r = self.sel.rect()?;
        if self.sel.is_dragging() && self.new_drag {
            return None;
        }
        let corner = Point {
            x: r.right() - 1,
            y: r.bottom() - 1,
        };
        let mon = monitor_containing(corner, &self.monitors);
        let dpi = self.monitor_dpi(mon);
        let b = scaled(BUTTON, dpi);
        let bar = place_toolbar(r, (b * ACTIONS.len() as i32, b), scaled(GAP, dpi), mon);
        Some((bar, dpi))
    }

    fn font(&mut self, dpi: u32) -> HFONT {
        if let Some((_, f)) = self.fonts.iter().find(|(d, _)| *d == dpi) {
            return *f;
        }
        let mut ncm = NONCLIENTMETRICSW {
            cbSize: std::mem::size_of::<NONCLIENTMETRICSW>() as u32,
            ..Default::default()
        };
        // SAFETY: ncm có cbSize đúng; hàm chỉ ghi vào ncm.
        let ok = unsafe {
            SystemParametersInfoForDpi(
                SPI_GETNONCLIENTMETRICS.0,
                ncm.cbSize,
                Some(&mut ncm as *mut NONCLIENTMETRICSW as *mut c_void),
                0,
                dpi,
            )
            .is_ok()
        };
        let lf = if ok {
            ncm.lfMessageFont
        } else {
            let mut lf = LOGFONTW {
                lfHeight: -scaled(12, dpi),
                ..Default::default()
            };
            for (slot, u) in lf.lfFaceName.iter_mut().zip(wide("Segoe UI")) {
                *slot = u;
            }
            lf
        };
        // SAFETY: lf hợp lệ; font được lưu và xóa trong Drop của State.
        let f = unsafe { CreateFontIndirectW(&lf) };
        if !f.is_invalid() {
            self.fonts.push((dpi, f));
        }
        f
    }

    fn text_size(&mut self, text: &[u16], dpi: u32) -> (i32, i32) {
        let font = self.font(dpi);
        let mut sz = SIZE::default();
        // SAFETY: DC đo chữ còn sống; font được chọn rồi trả lại ngay.
        unsafe {
            let old = SelectObject(self.measure, font.into());
            let _ = GetTextExtentPoint32W(self.measure, text, &mut sz);
            SelectObject(self.measure, old);
        }
        (sz.cx, sz.cy)
    }

    /// Nhãn kích thước: vị trí, chữ, DPI.
    fn label(&mut self) -> Option<(Rect, Vec<u16>, u32)> {
        let r = self.sel.rect()?;
        let corner = Point { x: r.x, y: r.y };
        let mon = monitor_containing(corner, &self.monitors);
        let dpi = self.monitor_dpi(mon);
        let text = wide(&format!("{} \u{d7} {}", r.w, r.h));
        let (tw, th) = self.text_size(&text, dpi);
        let (lw, lh) = (tw + 2 * scaled(7, dpi), th + 2 * scaled(2, dpi));
        let gap = scaled(GAP, dpi);
        let above = r.y - gap - lh;
        let y = if above >= mon.y { above } else { r.y + gap };
        let x = if above >= mon.y { r.x } else { r.x + gap };
        let rect = clamp_x(Rect { x, y, w: lw, h: lh }, mon);
        Some((rect, text, dpi))
    }

    /// Chú thích của nút đang rê chuột: vị trí, chữ, DPI.
    fn tooltip(&mut self) -> Option<(Rect, Vec<u16>, u32)> {
        let a = self.hover?;
        let (bar, dpi) = self.toolbar()?;
        let i = action_index(a);
        let text = wide(lang::tr(*TIPS.get(i)?));
        let (tw, th) = self.text_size(&text, dpi);
        let (w, h) = (tw + 2 * scaled(8, dpi), th + 2 * scaled(4, dpi));
        let b = bar.h;
        let btn_right = bar.x + b * (i as i32 + 1);
        let mon = monitor_containing(
            Point {
                x: btn_right - 1,
                y: bar.y,
            },
            &self.monitors,
        );
        let gap = scaled(7, dpi);
        let above = bar.y - gap - h;
        let y = if above >= mon.y {
            above
        } else {
            bar.bottom() + gap
        };
        let rect = clamp_x(
            Rect {
                x: btn_right - w,
                y,
                w,
                h,
            },
            mon,
        );
        Some((rect, text, dpi))
    }

    /// Mọi vùng có vẽ thêm ngoài nền tối (khung, tay nắm, nhãn, thanh công cụ, chú thích).
    fn chrome(&mut self) -> Vec<Rect> {
        let mut v = Vec::new();
        if let Some(r) = self.sel.rect() {
            v.push(inflate(r, scaled(HANDLE_SIZE, self.max_dpi()) / 2 + 3));
        }
        if let Some((r, _, _)) = self.label() {
            v.push(r);
        }
        if let Some((r, _)) = self.toolbar() {
            v.push(inflate(r, 1));
        }
        if let Some((r, _, _)) = self.tooltip() {
            v.push(r);
        }
        v
    }

    fn invalidate(&self, rects: &[Rect]) {
        for r in rects {
            let rc = to_rect(*r);
            // SAFETY: hwnd là cửa sổ lớp phủ của luồng này; chỉ đánh dấu vùng cần vẽ lại.
            unsafe {
                let _ = InvalidateRect(Some(self.hwnd), Some(&rc), false);
            }
        }
    }

    /// Thay đổi trạng thái qua `f`, rồi đánh dấu vẽ lại vùng cũ và vùng mới.
    fn update(&mut self, f: impl FnOnce(&mut State)) {
        let before = self.chrome();
        f(self);
        let after = self.chrome();
        self.invalidate(&before);
        self.invalidate(&after);
    }

    fn cursor_at(&self, p: Point) -> PCWSTR {
        if self.sel.is_dragging() {
            return self.drag_cursor;
        }
        if let Some((bar, _)) = self.toolbar() {
            if bar.contains(p) {
                return IDC_ARROW;
            }
        }
        cursor_for_hit(self.sel.hit_test(p, self.hit_radius(p)))
    }

    fn finish(&mut self, a: Action) {
        if a == Action::Cancel {
            self.result = Some(None);
        } else if let Some(r) = self.sel.rect() {
            self.result = Some(Some((a, r)));
        }
    }

    fn on_down(&mut self, p: Point) -> Effects {
        if let Some((bar, _)) = self.toolbar() {
            if bar.contains(p) {
                self.pressed = button_at(bar, p);
                return Effects::default();
            }
        }
        let rad = self.hit_radius(p);
        let hit = self.sel.hit_test(p, rad);
        self.update(|s| {
            s.new_drag = hit == Hit::Outside;
            s.drag_cursor = cursor_for_hit(hit);
            s.hover = None;
            s.pressed = None;
            s.sel.press(p, rad);
        });
        Effects {
            capture: true,
            ..Default::default()
        }
    }

    fn on_move(&mut self, p: Point) {
        if self.sel.is_dragging() {
            self.update(|s| s.sel.drag_to(p));
        } else {
            let hover = self.toolbar().and_then(|(bar, _)| button_at(bar, p));
            if hover != self.hover {
                self.update(|s| s.hover = hover);
            }
        }
        set_cursor(self.cursor_at(p));
    }

    fn on_up(&mut self, p: Point) -> Effects {
        if self.sel.is_dragging() {
            self.update(|s| {
                s.sel.drag_to(p);
                s.sel.release();
                s.hover = s.toolbar().and_then(|(bar, _)| button_at(bar, p));
            });
            return Effects {
                release: true,
                ..Default::default()
            };
        }
        if let Some(a) = self.pressed.take() {
            if self.toolbar().and_then(|(bar, _)| button_at(bar, p)) == Some(a) {
                self.finish(a);
            }
        }
        Effects::default()
    }

    fn on_key(&mut self, vk: u16) {
        if vk == VK_ESCAPE.0 {
            self.result = Some(None);
            return;
        }
        // SAFETY: đọc trạng thái phím của luồng này.
        let ctrl = unsafe { GetKeyState(i32::from(VK_CONTROL.0)) } < 0;
        if !ctrl || self.sel.is_dragging() || self.sel.rect().is_none() {
            return;
        }
        let a = match vk {
            0x44 => Action::Upload, // D
            0x53 => Action::Save,   // S
            0x43 => Action::Copy,   // C
            _ => return,
        };
        self.finish(a);
    }

    fn paint(&mut self, target: HDC, area: Rect) {
        let Some(area) = intersect(
            area,
            Rect {
                x: 0,
                y: 0,
                w: self.w,
                h: self.h,
            },
        ) else {
            return;
        };
        if let Some(dc) = self.back.as_ref().map(|b| b.dc) {
            // Bộ đệm cố định cùng tọa độ với client: chỉ vẽ trong `area` rồi chép đúng vùng đó.
            // SAFETY: DC của BackBuffer còn sống; vùng cắt được bỏ ngay sau khi vẽ.
            unsafe {
                IntersectClipRect(dc, area.x, area.y, area.right(), area.bottom());
            }
            self.draw(&Painter { dc, ox: 0, oy: 0 }, area);
            // SAFETY: như trên; target là DC của BeginPaint còn hiệu lực.
            unsafe {
                SelectClipRgn(dc, None);
                let _ = BitBlt(
                    target,
                    area.x,
                    area.y,
                    area.w,
                    area.h,
                    Some(dc),
                    area.x,
                    area.y,
                    SRCCOPY,
                );
            }
            return;
        }
        // Dự phòng khi không tạo được bộ đệm cố định: bitmap tạm cỡ vùng cần vẽ.
        // SAFETY: DC và bitmap bộ nhớ chỉ sống trong khối này và được giải phóng ở cuối,
        // sau khi bỏ chọn bitmap.
        unsafe {
            let mem = CreateCompatibleDC(Some(target));
            if mem.is_invalid() {
                return;
            }
            let bmp = CreateCompatibleBitmap(target, area.w, area.h);
            if bmp.is_invalid() {
                let _ = DeleteDC(mem);
                return;
            }
            let old = SelectObject(mem, bmp.into());
            let p = Painter {
                dc: mem,
                ox: area.x,
                oy: area.y,
            };
            self.draw(&p, area);
            let _ = BitBlt(
                target,
                area.x,
                area.y,
                area.w,
                area.h,
                Some(mem),
                0,
                0,
                SRCCOPY,
            );
            SelectObject(mem, old);
            let _ = DeleteObject(bmp.into());
            let _ = DeleteDC(mem);
        }
    }

    fn draw(&mut self, p: &Painter, area: Rect) {
        let sel = self.sel.rect().and_then(|r| intersect(r, area));
        let (w, h) = (self.w, self.h);
        // SAFETY: `bright`/`bright_len` là bộ đệm của Screenshot::image, sống suốt pick_region
        // (State bị hủy trước khi pick_region trả về) và không bị ai ghi trong lúc này.
        let bright = unsafe { std::slice::from_raw_parts(self.bright, self.bright_len) };
        let dark = self.dark.as_slice();
        // Nền tối chỉ vẽ ngoài khung, ảnh gốc chỉ vẽ trong khung: không vẽ chồng.
        match sel {
            None => p.image(dark, w, h, area),
            Some(s) => {
                for strip in [
                    Rect {
                        h: s.y - area.y,
                        ..area
                    },
                    Rect {
                        y: s.bottom(),
                        h: area.bottom() - s.bottom(),
                        ..area
                    },
                    Rect {
                        y: s.y,
                        w: s.x - area.x,
                        h: s.h,
                        ..area
                    },
                    Rect {
                        x: s.right(),
                        y: s.y,
                        w: area.right() - s.right(),
                        h: s.h,
                    },
                ] {
                    if strip.w > 0 && strip.h > 0 {
                        p.image(dark, w, h, strip);
                    }
                }
                p.image(bright, w, h, s);
            }
        }
        let Some(r) = self.sel.rect() else { return };
        p.border(r);
        for (hx, hy) in [
            (r.x, r.y),
            (r.x + r.w / 2, r.y),
            (r.right(), r.y),
            (r.right(), r.y + r.h / 2),
            (r.right(), r.bottom()),
            (r.x + r.w / 2, r.bottom()),
            (r.x, r.bottom()),
            (r.x, r.y + r.h / 2),
        ] {
            let size = scaled(HANDLE_SIZE, self.dpi_at(Point { x: hx, y: hy }));
            p.handle(hx, hy, size);
        }
        if let Some((rect, mut text, dpi)) = self.label() {
            let font = self.font(dpi);
            p.bubble(
                rect,
                scaled(4, dpi),
                &mut text,
                font,
                OVERLAY_LABEL,
                OVERLAY_LABEL,
            );
        }
        if let Some((bar, dpi)) = self.toolbar() {
            p.toolbar(bar, dpi, self.hover, self.gdiplus);
        }
        if let Some((rect, mut text, dpi)) = self.tooltip() {
            let font = self.font(dpi);
            p.bubble(
                rect,
                scaled(4, dpi),
                &mut text,
                font,
                OVERLAY_TIP,
                OVERLAY_BORDER,
            );
        }
    }
}

/// Vẽ vào DC bộ nhớ có gốc tại (`ox`, `oy`) của tọa độ client.
struct Painter {
    dc: HDC,
    ox: i32,
    oy: i32,
}

impl Painter {
    fn rc(&self, r: Rect) -> RECT {
        to_rect(Rect {
            x: r.x - self.ox,
            y: r.y - self.oy,
            ..r
        })
    }

    /// Chép vùng `r` (tọa độ ảnh) của ảnh BGRA `img_w`×`img_h` từ `bits`; sai biên thì bỏ qua.
    fn image(&self, bits: &[u8], img_w: i32, img_h: i32, r: Rect) {
        if !blit_in_bounds(r, img_w, img_h, bits.len()) {
            return;
        }
        // Đưa con trỏ tới hàng đầu của vùng và khai báo DIB đúng `r.h` hàng, nên nguồn
        // luôn là trọn chiều cao DIB: không phụ thuộc cách hiểu ySrc của DIB từ trên xuống.
        let offset = r.y as usize * img_w as usize * 4;
        let Some(rows) = bits.get(offset..) else {
            return;
        };
        let bmi = dib_info(img_w, r.h);
        // SAFETY: blit_in_bounds bảo đảm `rows` (từ hàng r.y) chứa đủ r.h hàng rộng img_w
        // và cột r.x..r.right() nằm trong mỗi hàng; bộ đệm sống suốt lời gọi.
        unsafe {
            StretchDIBits(
                self.dc,
                r.x - self.ox,
                r.y - self.oy,
                r.w,
                r.h,
                r.x,
                0,
                r.w,
                r.h,
                Some(rows.as_ptr() as *const c_void),
                &bmi,
                DIB_RGB_COLORS,
                SRCCOPY,
            );
        }
    }

    fn border(&self, r: Rect) {
        let rc = self.rc(r);
        // SAFETY: bút tạo và xóa trong khối; đối tượng cũ được chọn lại trước khi xóa.
        unsafe {
            let pen = CreatePen(PS_SOLID, 1, cr(ACCENT));
            let outer = CreatePen(PS_SOLID, 1, cr(OVERLAY_LABEL));
            let old_pen = SelectObject(self.dc, outer.into());
            let old_brush = SelectObject(self.dc, GetStockObject(NULL_BRUSH));
            SetBkMode(self.dc, OPAQUE);
            // Nét tối ngoài 1px để viền xanh nổi trên nền xanh.
            let _ = Rectangle(
                self.dc,
                rc.left - 2,
                rc.top - 2,
                rc.right + 2,
                rc.bottom + 2,
            );
            SelectObject(self.dc, pen.into());
            let _ = Rectangle(
                self.dc,
                rc.left - 1,
                rc.top - 1,
                rc.right + 1,
                rc.bottom + 1,
            );
            SelectObject(self.dc, old_brush);
            SelectObject(self.dc, old_pen);
            let _ = DeleteObject(pen.into());
            let _ = DeleteObject(outer.into());
        }
    }

    fn handle(&self, x: i32, y: i32, size: i32) {
        let (x, y) = (x - self.ox, y - self.oy);
        let half = size / 2;
        // SAFETY: như `border`.
        unsafe {
            let pen = CreatePen(PS_SOLID, 1, cr(ACCENT));
            let brush = CreateSolidBrush(cr(WHITE));
            let old_pen = SelectObject(self.dc, pen.into());
            let old_brush = SelectObject(self.dc, brush.into());
            let _ = Rectangle(
                self.dc,
                x - half,
                y - half,
                x - half + size,
                y - half + size,
            );
            SelectObject(self.dc, old_brush);
            SelectObject(self.dc, old_pen);
            let _ = DeleteObject(brush.into());
            let _ = DeleteObject(pen.into());
        }
    }

    fn round_rect(&self, r: Rect, radius: i32, fill: Rgb, edge: Rgb) {
        let rc = self.rc(r);
        // SAFETY: như `border`.
        unsafe {
            let pen = CreatePen(PS_SOLID, 1, cr(edge));
            let brush = CreateSolidBrush(cr(fill));
            let old_pen = SelectObject(self.dc, pen.into());
            let old_brush = SelectObject(self.dc, brush.into());
            let _ = RoundRect(
                self.dc,
                rc.left,
                rc.top,
                rc.right,
                rc.bottom,
                2 * radius,
                2 * radius,
            );
            SelectObject(self.dc, old_brush);
            SelectObject(self.dc, old_pen);
            let _ = DeleteObject(brush.into());
            let _ = DeleteObject(pen.into());
        }
    }

    fn fill(&self, r: Rect, color: Rgb) {
        let rc = self.rc(r);
        // SAFETY: chổi tạo và xóa trong khối, không được chọn vào DC.
        unsafe {
            let brush = CreateSolidBrush(cr(color));
            FillRect(self.dc, &rc, brush);
            let _ = DeleteObject(brush.into());
        }
    }

    /// Ô chữ sáng trên nền tối bo góc (nhãn kích thước, chú thích) với nền `fill` và viền `edge`.
    fn bubble(&self, r: Rect, radius: i32, text: &mut [u16], font: HFONT, fill: Rgb, edge: Rgb) {
        self.round_rect(r, radius, fill, edge);
        let mut rc = self.rc(r);
        // SAFETY: font còn sống (State giữ); đối tượng cũ được chọn lại.
        unsafe {
            let old = SelectObject(self.dc, font.into());
            SetBkMode(self.dc, TRANSPARENT);
            SetTextColor(self.dc, cr(TEXT));
            DrawTextW(
                self.dc,
                text,
                &mut rc,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );
            SelectObject(self.dc, old);
        }
    }

    fn toolbar(&self, bar: Rect, dpi: u32, hover: Option<Action>, gdiplus: bool) {
        let b = bar.h;
        self.round_rect(bar, scaled(6, dpi), SURFACE, OVERLAY_BORDER);
        if let Some(a) = hover {
            let i = action_index(a) as i32;
            self.fill(
                Rect {
                    x: bar.x + b * i + 1,
                    y: bar.y + 1,
                    w: b - 2,
                    h: b - 2,
                },
                OVERLAY_HOVER,
            );
        }
        for i in 1..ACTIONS.len() as i32 {
            self.fill(
                Rect {
                    x: bar.x + b * i,
                    y: bar.y + 1,
                    w: 1,
                    h: b - 2,
                },
                OVERLAY_BORDER,
            );
        }
        if !gdiplus {
            return;
        }
        let Some(g) = Gp::new(self.dc) else { return };
        let icon = scaled(ICON, dpi) as f32;
        for (i, a) in ACTIONS.iter().enumerate() {
            let cx = (bar.x - self.ox) as f32 + b as f32 * (i as f32 + 0.5);
            let cy = (bar.y - self.oy) as f32 + b as f32 * 0.5;
            let color = if hover == Some(*a) {
                ICON_HOVER_ARGB
            } else {
                ICON_ARGB
            };
            g.icon(*a, cx, cy, icon, color);
        }
    }
}

impl Gp {
    /// Vẽ biểu tượng trong lưới 24×24 (theo hình mẫu), cỡ `size` pixel, tâm (`cx`, `cy`).
    fn icon(&self, a: Action, cx: f32, cy: f32, size: f32, argb: u32) {
        let k = size / 24.0;
        let (ox, oy) = (cx - size / 2.0, cy - size / 2.0);
        let mut pen = std::ptr::null_mut();
        let mut path: *mut GpPath = std::ptr::null_mut();
        // SAFETY: bút và đường được tạo, dùng và xóa trong khối; kiểm null trước khi dùng.
        unsafe {
            if GdipCreatePen1(argb, 1.8 * k, UnitPixel, &mut pen).0 != 0 || pen.is_null() {
                return;
            }
            GdipSetPenLineCap197819(pen, LineCapRound, LineCapRound, DashCapFlat);
            GdipSetPenLineJoin(pen, LineJoinRound);
            if GdipCreatePath(FillModeAlternate, &mut path).0 != 0 || path.is_null() {
                GdipDeletePen(pen);
                return;
            }
            let arc = |x: f32, y: f32, r: f32, start: f32, sweep: f32| {
                GdipAddPathArc(
                    path,
                    ox + (x - r) * k,
                    oy + (y - r) * k,
                    2.0 * r * k,
                    2.0 * r * k,
                    start,
                    sweep,
                );
            };
            let poly = |pts: &[(f32, f32)]| {
                GdipStartPathFigure(path);
                for s in pts.windows(2) {
                    let ((x1, y1), (x2, y2)) = (s[0], s[1]);
                    GdipAddPathLine(path, ox + x1 * k, oy + y1 * k, ox + x2 * k, oy + y2 * k);
                }
            };
            match a {
                Action::Upload => {
                    // Đám mây: ba cung tròn (đổi từ đường SVG của hình mẫu), mở ở đáy.
                    arc(6.989, 14.0, 4.0, 89.84, 173.14);
                    arc(12.328, 11.4565, 6.0, 193.75, 147.22);
                    arc(17.75, 13.75, 4.2574, 273.37, 180.0);
                    // Mũi tên lên.
                    poly(&[(12.0, 20.0), (12.0, 12.0)]);
                    poly(&[(9.0, 15.0), (12.0, 12.0), (15.0, 15.0)]);
                }
                Action::Save => {
                    poly(&[
                        (5.0, 3.0),
                        (16.0, 3.0),
                        (19.0, 6.0),
                        (19.0, 21.0),
                        (5.0, 21.0),
                    ]);
                    GdipClosePathFigure(path);
                    poly(&[(8.0, 3.0), (8.0, 9.0), (15.0, 9.0), (15.0, 3.0)]);
                    poly(&[(8.0, 21.0), (8.0, 14.0), (16.0, 14.0), (16.0, 21.0)]);
                }
                Action::Copy => {
                    // Ô trước: hình vuông bo góc (9,9)-(20,20), bán kính 2.
                    GdipStartPathFigure(path);
                    arc(11.0, 11.0, 2.0, 180.0, 90.0);
                    arc(18.0, 11.0, 2.0, 270.0, 90.0);
                    arc(18.0, 18.0, 2.0, 0.0, 90.0);
                    arc(11.0, 18.0, 2.0, 90.0, 90.0);
                    GdipClosePathFigure(path);
                    // Ô sau: chỉ phần lộ ra (cạnh trái và cạnh trên).
                    poly(&[(5.0, 15.0), (5.0, 5.0)]);
                    arc(7.0, 5.0, 2.0, 180.0, 90.0);
                    GdipAddPathLine(
                        path,
                        ox + 7.0 * k,
                        oy + 3.0 * k,
                        ox + 15.0 * k,
                        oy + 3.0 * k,
                    );
                }
                Action::Cancel => {
                    poly(&[(6.0, 6.0), (18.0, 18.0)]);
                    poly(&[(18.0, 6.0), (6.0, 18.0)]);
                }
            }
            GdipDrawPath(self.raw(), pen, path);
            GdipDeletePath(path);
            GdipDeletePen(pen);
        }
    }
}

/// Lấy lại trạng thái khi `pick_region` kết thúc trên mọi đường đi (font, DC được giải phóng).
struct StateGuard;
impl Drop for StateGuard {
    fn drop(&mut self) {
        let s = OVERLAY.with(|o| o.try_borrow_mut().ok().and_then(|mut b| b.take()));
        drop(s);
    }
}

struct WindowGuard(HWND);
impl Drop for WindowGuard {
    fn drop(&mut self) {
        // SAFETY: cửa sổ lớp phủ của luồng này; hủy một lần, kể cả khi đã bị hủy trước (lỗi bỏ qua).
        unsafe {
            let _ = ReleaseCapture();
            let _ = DestroyWindow(self.0);
        }
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        // Vẽ nền ở WM_PAINT; xóa nền ở đây sẽ gây nháy.
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            // SAFETY: cặp BeginPaint/EndPaint chuẩn trên cửa sổ của luồng này.
            let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
            if !hdc.is_invalid() {
                let rc = ps.rcPaint;
                let area = Rect {
                    x: rc.left,
                    y: rc.top,
                    w: rc.right - rc.left,
                    h: rc.bottom - rc.top,
                };
                with_state(|s| s.paint(hdc, area));
            }
            // SAFETY: kết thúc lượt vẽ đã mở ở trên.
            unsafe {
                let _ = EndPaint(hwnd, &ps);
            }
            LRESULT(0)
        }
        WM_SETCURSOR if (lp.0 & 0xFFFF) as u32 == HTCLIENT => {
            let mut pt = POINT::default();
            // SAFETY: chỉ đọc vị trí chuột.
            if unsafe { GetCursorPos(&mut pt) }.is_ok() {
                let mut wr = RECT::default();
                // SAFETY: đọc vị trí cửa sổ của ta để đổi sang tọa độ client.
                let _ = unsafe { GetWindowRect(hwnd, &mut wr) };
                let p = Point {
                    x: pt.x - wr.left,
                    y: pt.y - wr.top,
                };
                if let Some(id) = with_state(|s| s.cursor_at(p)) {
                    set_cursor(id);
                    return LRESULT(1);
                }
            }
            // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
            unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
        }
        WM_LBUTTONDOWN => {
            let p = point_from_lparam(lp);
            if let Some(fx) = with_state(|s| s.on_down(p)) {
                if fx.capture {
                    // SAFETY: giữ chuột cho cửa sổ của ta trong lúc kéo.
                    unsafe {
                        SetCapture(hwnd);
                    }
                }
            }
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let p = point_from_lparam(lp);
            with_state(|s| s.on_move(p));
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let p = point_from_lparam(lp);
            if let Some(fx) = with_state(|s| s.on_up(p)) {
                if fx.release {
                    // SAFETY: trả chuột sau khi kéo xong; WM_CAPTURECHANGED đến khi đã trả mượn.
                    unsafe {
                        let _ = ReleaseCapture();
                    }
                }
            }
            LRESULT(0)
        }
        WM_CAPTURECHANGED => {
            // Mất chuột giữa chừng (cửa sổ khác chiếm): kết thúc lần kéo.
            with_state(|s| {
                if s.sel.is_dragging() {
                    s.update(|s| s.sel.release());
                }
            });
            LRESULT(0)
        }
        WM_KEYDOWN => {
            with_state(|s| s.on_key(wp.0 as u16));
            LRESULT(0)
        }
        WM_ACTIVATE => {
            // WA_INACTIVE = 0: mất tiêu điểm (Alt+Tab, Win...) thì hủy.
            if wp.0 & 0xFFFF == 0 {
                with_state(|s| {
                    if s.result.is_none() {
                        s.result = Some(None);
                    }
                });
            }
            // Xử lý mặc định đặt tiêu điểm bàn phím vào cửa sổ khi được kích hoạt.
            // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
            unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
        }
        // Lớp phủ đã phủ đúng pixel vật lý; không đổi kích thước theo DPI.
        WM_DPICHANGED => LRESULT(0),
        WM_HOTKEY if wp.0 == ESC_HOTKEY_ID as usize => {
            with_state(|s| s.result = Some(None));
            LRESULT(0)
        }
        WM_TIMER if wp.0 == FOCUS_TIMER_ID => {
            let tries = with_state(|s| {
                s.focus_retries += 1;
                s.focus_retries
            })
            .unwrap_or(MAX_FOCUS_RETRIES);
            // Gọi ngoài lúc mượn trạng thái: SetForegroundWindow gửi WM_ACTIVATE đồng bộ.
            if take_focus(hwnd) || tries >= MAX_FOCUS_RETRIES {
                // SAFETY: hủy bộ hẹn giờ của chính cửa sổ này.
                unsafe {
                    let _ = KillTimer(Some(hwnd), FOCUS_TIMER_ID);
                }
            }
            LRESULT(0)
        }
        WM_CLOSE | WM_DESTROY => {
            with_state(|s| {
                if s.result.is_none() {
                    s.result = Some(None);
                }
            });
            if msg == WM_CLOSE {
                return LRESULT(0);
            }
            // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
            unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
        }
        // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
        _ => unsafe { DefWindowProcW(hwnd, msg, wp, lp) },
    }
}

fn monitor_dpi_at(p: POINT) -> u32 {
    let (mut x, mut y) = (96u32, 96u32);
    // SAFETY: MonitorFromPoint với DEFAULTTONEAREST luôn trả về một màn hình; chỉ đọc DPI.
    let ok = unsafe {
        let m = MonitorFromPoint(p, MONITOR_DEFAULTTONEAREST);
        GetDpiForMonitor(m, MDT_EFFECTIVE_DPI, &mut x, &mut y).is_ok()
    };
    if ok && x > 0 {
        x
    } else {
        96
    }
}

fn create_window(origin: Point, w: i32, h: i32) -> Option<HWND> {
    // SAFETY: đăng ký lớp (lần sau trả lỗi "đã có", bỏ qua) và tạo cửa sổ ở luồng chính.
    unsafe {
        let hinst = GetModuleHandleW(None).ok()?;
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(wndproc),
            hInstance: hinst.into(),
            lpszClassName: CLASS,
            ..Default::default()
        };
        RegisterClassExW(&wc);
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            CLASS,
            w!("darkshot"),
            WS_POPUP,
            origin.x,
            origin.y,
            w,
            h,
            None,
            None,
            Some(hinst.into()),
            None,
        )
        .ok()?;
        // Bảo đảm client đúng bằng màn hình ảo (tọa độ client = tọa độ ảnh).
        let mut rc = RECT::default();
        if GetClientRect(hwnd, &mut rc).is_ok() && (rc.right != w || rc.bottom != h) {
            let _ = SetWindowPos(
                hwnd,
                None,
                origin.x,
                origin.y,
                w,
                h,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
        Some(hwnd)
    }
}

/// Đưa lớp phủ lên trước và nhận bàn phím. Windows có thể từ chối SetForegroundWindow
/// (quy tắc chống giành tiêu điểm); khi đó gắn tạm hàng đợi nhập với luồng đang ở trước rồi thử lại.
/// Trả về `true` khi lớp phủ đã ở trước.
fn take_focus(hwnd: HWND) -> bool {
    // SAFETY: các API chỉ thao tác trên cửa sổ của ta và luồng đang ở trước; gắn rồi gỡ ngay.
    unsafe {
        let _ = SetForegroundWindow(hwnd);
        let fg = GetForegroundWindow();
        if fg != hwnd {
            let me = GetCurrentThreadId();
            let other = GetWindowThreadProcessId(fg, None);
            if other != 0 && other != me && AttachThreadInput(me, other, true).as_bool() {
                let _ = BringWindowToTop(hwnd);
                let _ = SetForegroundWindow(hwnd);
                let _ = AttachThreadInput(me, other, false);
            }
        }
        let _ = SetFocus(Some(hwnd));
        GetForegroundWindow() == hwnd
    }
}

/// Phím Esc toàn cục trong lúc lớp phủ mở, để không bao giờ kẹt khi chưa giành được
/// tiêu điểm. Gỡ khi drop; đăng ký thất bại (phím đã bị chiếm) thì không làm gì.
struct EscHotkey {
    hwnd: HWND,
    registered: bool,
}

impl EscHotkey {
    fn register(hwnd: HWND) -> EscHotkey {
        // SAFETY: phím tắt gắn với cửa sổ lớp phủ của luồng này; gỡ trong Drop.
        let registered = unsafe {
            RegisterHotKey(
                Some(hwnd),
                ESC_HOTKEY_ID,
                MOD_NOREPEAT,
                u32::from(VK_ESCAPE.0),
            )
        }
        .is_ok();
        EscHotkey { hwnd, registered }
    }
}

impl Drop for EscHotkey {
    fn drop(&mut self) {
        if self.registered {
            // SAFETY: gỡ đúng phím đã đăng ký, khi cửa sổ còn sống (guard này thả trước WindowGuard).
            unsafe {
                let _ = UnregisterHotKey(Some(self.hwnd), ESC_HOTKEY_ID);
            }
        }
    }
}

/// Hiện lớp phủ trên ảnh `shot` và chờ người dùng chọn. `Rect` trong tọa độ ảnh.
/// `initial` (tọa độ ảnh): khung chọn sẵn, lớp phủ mở ở trạng thái đã chọn xong; khung không
/// nằm trọn trong ảnh thì như `None`. `Ok(None)` khi người dùng hủy; `Err` khi không mở được.
pub fn pick_region(shot: &Screenshot, initial: Option<Rect>) -> Result<Option<Picked>, String> {
    let img = &shot.image;
    let size = (i32::try_from(img.width), i32::try_from(img.height));
    let (Ok(w), Ok(h)) = size else {
        return Err(lang::tr(Msg::OverlayErrTooLarge).to_string());
    };
    if !img.is_valid() {
        return Err(lang::tr(Msg::OverlayErrInvalid).to_string());
    }
    // Không mở lồng hai lớp phủ.
    let busy = OVERLAY.with(|o| o.try_borrow().map(|s| s.is_some()).unwrap_or(true));
    if busy {
        return Err(lang::tr(Msg::OverlayErrAlreadyOpen).to_string());
    }

    let mut dark = Vec::new();
    if dark.try_reserve_exact(img.bgra.len()).is_err() {
        return Err(lang::tr(Msg::OverlayErrMemory).to_string());
    }
    dark.extend(img.bgra.iter().map(|b| b >> 1));

    let gdiplus = GdiPlus::start();
    let dpis = shot
        .monitors
        .iter()
        .map(|m| {
            monitor_dpi_at(POINT {
                x: shot.origin.x + m.x + m.w / 2,
                y: shot.origin.y + m.y + m.h / 2,
            })
        })
        .collect();
    // SAFETY: DC bộ nhớ để đo chữ; State xóa nó khi drop.
    let measure = unsafe { CreateCompatibleDC(None) };
    let state = State {
        hwnd: HWND::default(),
        w,
        h,
        bright: img.bgra.as_ptr(),
        bright_len: img.bgra.len(),
        dark,
        back: BackBuffer::new(w, h),
        focus_retries: 0,
        monitors: shot.monitors.clone(),
        dpis,
        sel: Selection::with_rect(Rect { x: 0, y: 0, w, h }, initial),
        new_drag: false,
        drag_cursor: IDC_CROSS,
        pressed: None,
        hover: None,
        result: None,
        fonts: Vec::new(),
        measure,
        gdiplus: gdiplus.ok(),
    };
    let placed = OVERLAY.with(|o| match o.try_borrow_mut() {
        Ok(mut b) => {
            *b = Some(state);
            true
        }
        Err(_) => false,
    });
    if !placed {
        return Err(lang::tr(Msg::OverlayErrAlreadyOpen).to_string());
    }
    let _state = StateGuard;
    let Some(hwnd) = create_window(shot.origin, w, h) else {
        return Err(lang::tr(Msg::OverlayErrWindow).to_string());
    };
    // Thả trước _state: cửa sổ bị hủy khi trạng thái (và ảnh tối) còn sống.
    let _window = WindowGuard(hwnd);
    with_state(|s| s.hwnd = hwnd);
    // Thả trước _window: gỡ phím Esc khi cửa sổ còn sống.
    let _esc = EscHotkey::register(hwnd);

    // SAFETY: hiện và vẽ ngay cửa sổ vừa tạo của luồng này.
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = UpdateWindow(hwnd);
    }
    if !take_focus(hwnd) {
        // SAFETY: bộ hẹn giờ của cửa sổ lớp phủ; tự mất khi cửa sổ bị hủy.
        unsafe {
            SetTimer(Some(hwnd), FOCUS_TIMER_ID, FOCUS_RETRY_MS, None);
        }
    }
    if initial.is_some() && with_state(|s| s.sel.rect().is_some()) == Some(true) {
        // Mở với khung có sẵn: con trỏ chuột và nút đang rê theo chỗ chuột đứng ngay,
        // không đợi lần di chuột đầu tiên.
        let mut pt = POINT::default();
        // SAFETY: chỉ đọc vị trí chuột.
        if unsafe { GetCursorPos(&mut pt) }.is_ok() {
            let p = Point {
                x: pt.x - shot.origin.x,
                y: pt.y - shot.origin.y,
            };
            with_state(|s| s.on_move(p));
        }
    }

    let mut msg = MSG::default();
    loop {
        if let Some(Some(r)) = with_state(|s| s.result) {
            return Ok(r.map(|(action, rect)| Picked {
                action,
                rect,
                shapes: Vec::new(),
            }));
        }
        // SAFETY: vòng lặp thông điệp chuẩn của luồng này.
        let got = unsafe { GetMessageW(&mut msg, None, 0, 0) }.0;
        if got == 0 {
            // WM_QUIT (ứng dụng đang thoát): hủy lớp phủ và trả WM_QUIT cho vòng lặp chính.
            // SAFETY: đăng lại thông điệp thoát cho luồng này.
            unsafe { PostQuitMessage(msg.wParam.0 as i32) };
            return Ok(None);
        }
        if got < 0 {
            return Err(lang::tr(Msg::OverlayErrLoop).to_string());
        }
        // SAFETY: phân phát thông điệp vừa nhận.
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toolbar_button_hit_testing() {
        let bar = Rect {
            x: 100,
            y: 100,
            w: 144,
            h: 36,
        };
        let at = |x, y| button_at(bar, Point { x, y });
        assert_eq!(at(110, 110), Some(Action::Upload));
        assert_eq!(at(150, 110), Some(Action::Save));
        assert_eq!(at(190, 110), Some(Action::Copy));
        assert_eq!(at(230, 110), Some(Action::Cancel));
        assert_eq!(at(300, 110), None);
    }

    #[test]
    fn blit_bounds_are_checked() {
        let r = |x, y, w, h| Rect { x, y, w, h };
        let full = 100 * 50 * 4;
        assert!(blit_in_bounds(r(0, 0, 100, 50), 100, 50, full));
        assert!(blit_in_bounds(r(10, 20, 90, 30), 100, 50, full));
        // vượt phải, vượt dưới
        assert!(!blit_in_bounds(r(10, 0, 91, 10), 100, 50, full));
        assert!(!blit_in_bounds(r(0, 45, 10, 6), 100, 50, full));
        // âm
        assert!(!blit_in_bounds(r(-1, 0, 10, 10), 100, 50, full));
        assert!(!blit_in_bounds(r(0, -1, 10, 10), 100, 50, full));
        assert!(!blit_in_bounds(r(0, 0, -5, 10), 100, 50, full));
        // rỗng
        assert!(!blit_in_bounds(r(0, 0, 0, 10), 100, 50, full));
        assert!(!blit_in_bounds(r(0, 0, 10, 0), 100, 50, full));
        // bộ đệm ngắn
        assert!(!blit_in_bounds(r(0, 0, 10, 10), 100, 50, full - 1));
        // tràn số khi cộng
        assert!(!blit_in_bounds(r(i32::MAX, 0, 10, 10), 100, 50, full));
    }

    /// Mở lớp phủ thật, giả lập kéo khung rồi bấm nút Lưu (hoặc Esc), kiểm kết quả và
    /// số đối tượng GDI không tăng sau nhiều lần mở. Bỏ qua mặc định vì lớp phủ hiện
    /// lên màn hình của người đang dùng máy.
    #[test]
    #[ignore = "mở lớp phủ thật; chạy tay: cargo test -p darkshot-windows -- --ignored"]
    fn overlay_round_trip_without_gdi_leaks() {
        use crate::capture::{capture_virtual_screen, GDI_TEST_LOCK};
        use std::time::Duration;
        use windows::Win32::System::Threading::{
            GetCurrentProcess, GetGuiResources, GR_GDIOBJECTS,
        };
        use windows::Win32::UI::HiDpi::{
            SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        };
        use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, PostMessageW};

        let _gdi = GDI_TEST_LOCK.lock();
        // SAFETY: như ứng dụng thật (manifest PerMonitorV2), áp cho luồng kiểm thử.
        unsafe {
            SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
        let shot = capture_virtual_screen(false).unwrap();
        let m0 = shot.monitors[0];
        let (a, b) = (
            Point {
                x: m0.x + 100,
                y: m0.y + 100,
            },
            Point {
                x: m0.x + 400,
                y: m0.y + 300,
            },
        );
        let sel = Rect::from_points(a, b);
        // Vị trí nút Lưu, tính như State::toolbar.
        let corner = Point {
            x: sel.right() - 1,
            y: sel.bottom() - 1,
        };
        let mon = monitor_containing(corner, &shot.monitors);
        let dpi = monitor_dpi_at(POINT {
            x: shot.origin.x + mon.x + mon.w / 2,
            y: shot.origin.y + mon.y + mon.h / 2,
        });
        let btn = scaled(BUTTON, dpi);
        let bar = place_toolbar(sel, (4 * btn, btn), scaled(GAP, dpi), mon);
        let save = Point {
            x: bar.x + btn + btn / 2,
            y: bar.y + btn / 2,
        };

        // 0: bấm nút Lưu, 1: phím Esc (WM_KEYDOWN), 2: phím tắt Esc toàn cục (WM_HOTKEY).
        let run = |finish: u8| {
            let driver = std::thread::spawn(move || {
                let lp =
                    |p: Point| LPARAM(((p.y as isize & 0xFFFF) << 16) | (p.x as isize & 0xFFFF));
                for _ in 0..50 {
                    std::thread::sleep(Duration::from_millis(100));
                    // SAFETY: tìm cửa sổ lớp phủ theo tên lớp, chỉ đăng thông điệp vào nó.
                    let Ok(h) = (unsafe { FindWindowW(CLASS, None) }) else {
                        continue;
                    };
                    let post = |m: u32, wp: usize, l: LPARAM| {
                        // SAFETY: đăng thông điệp vào hàng đợi của cửa sổ còn sống.
                        let _ = unsafe { PostMessageW(Some(h), m, WPARAM(wp), l) };
                    };
                    post(WM_LBUTTONDOWN, 1, lp(a));
                    post(WM_MOUSEMOVE, 1, lp(b));
                    post(WM_LBUTTONUP, 0, lp(b));
                    std::thread::sleep(Duration::from_millis(300));
                    post(WM_MOUSEMOVE, 0, lp(save));
                    std::thread::sleep(Duration::from_millis(300));
                    match finish {
                        0 => {
                            post(WM_LBUTTONDOWN, 1, lp(save));
                            post(WM_LBUTTONUP, 0, lp(save));
                        }
                        1 => post(WM_KEYDOWN, usize::from(VK_ESCAPE.0), LPARAM(0)),
                        _ => post(WM_HOTKEY, ESC_HOTKEY_ID as usize, LPARAM(0)),
                    }
                    return true;
                }
                false
            });
            let r = pick_region(&shot, None);
            assert!(driver.join().unwrap(), "không thấy cửa sổ lớp phủ");
            r
        };
        // SAFETY: chỉ đọc số đối tượng GDI của tiến trình này.
        let gdi = || unsafe { GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS) };

        let saved = Ok(Some(Picked {
            action: Action::Save,
            rect: sel,
            shapes: Vec::new(),
        }));
        assert_eq!(run(0), saved);
        let after_first = gdi();
        assert_eq!(run(1), Ok(None));
        assert_eq!(run(2), Ok(None));
        // Vòng có vẽ: bút, hình chữ nhật, hoàn tác, mở bảng màu, đóng bằng Esc, rồi Lưu.
        let l = live::layout(&shot, sel);
        let inside = |dx, dy| Point {
            x: sel.x + dx,
            y: sel.y + dy,
        };
        let mut steps = vec![live::Step::Key(u16::from(b'P'))];
        steps.extend(live::drag(inside(20, 20), inside(120, 80)));
        steps.push(live::Step::Key(u16::from(b'R')));
        steps.extend(live::drag(inside(30, 30), inside(150, 120)));
        steps.push(live::Step::Ctrl(u16::from(b'Z')));
        steps.extend(live::click(l.item(6)));
        steps.push(live::Step::Key(VK_ESCAPE.0));
        steps.extend(live::click(l.save()));
        let (r, ok) = live::drive(&shot, Some(sel), steps);
        assert!(ok);
        let p = r.unwrap().unwrap();
        assert_eq!((p.action, p.rect, p.shapes.len()), (Action::Save, sel, 1));
        assert!(matches!(p.shapes[0], Shape::Pen { .. }));
        assert_eq!(run(0), saved);
        let after_third = gdi();
        assert!(
            after_third <= after_first,
            "GDI tăng từ {after_first} lên {after_third}"
        );
    }

    #[test]
    fn side_bar_button_hit_testing() {
        let (b, sep) = (36, 9);
        let (w, h) = side_bar_size(b, sep);
        assert_eq!((w, h), (36, 8 * 36 + 2 * 9));
        let bar = Rect {
            x: 500,
            y: 100,
            w,
            h,
        };
        for (i, item) in SIDE_ITEMS.iter().enumerate() {
            let r = side_item_rect(bar, sep, i);
            assert_eq!((r.x, r.w, r.h), (500, 36, 36));
            let c = Point {
                x: r.x + r.w / 2,
                y: r.y + r.h / 2,
            };
            assert_eq!(side_item_at(bar, sep, c), Some(*item), "mục {i}");
        }
        let at = |y| side_item_at(bar, sep, Point { x: 518, y });
        assert_eq!(at(100 + 18), Some(SideItem::Tool(Tool::Pen)));
        assert_eq!(at(100 + 36 + 18), Some(SideItem::Tool(Tool::Line)));
        assert_eq!(at(100 + 2 * 36 + 18), Some(SideItem::Tool(Tool::Arrow)));
        assert_eq!(at(100 + 3 * 36 + 18), Some(SideItem::Tool(Tool::Rect)));
        assert_eq!(at(100 + 4 * 36 + 18), Some(SideItem::Tool(Tool::Marker)));
        assert_eq!(at(100 + 5 * 36 + 18), Some(SideItem::Tool(Tool::Text)));
        // Vạch ngăn thứ nhất cùng lề của nó.
        for y in 100 + 6 * 36..100 + 6 * 36 + 9 {
            assert_eq!(at(y), None, "y = {y}");
        }
        assert_eq!(at(100 + 6 * 36 + 9), Some(SideItem::Color));
        assert_eq!(at(100 + 6 * 36 + 9 + 18), Some(SideItem::Color));
        // Vạch ngăn thứ hai.
        for y in 100 + 7 * 36 + 9..100 + 7 * 36 + 18 {
            assert_eq!(at(y), None, "y = {y}");
        }
        assert_eq!(at(100 + 7 * 36 + 18 + 18), Some(SideItem::Undo));
        assert_eq!(at(100 + h - 1), Some(SideItem::Undo));
        // Ngoài thanh.
        assert_eq!(at(99), None);
        assert_eq!(at(100 + h), None);
        assert_eq!(side_item_at(bar, sep, Point { x: 499, y: 118 }), None);
        assert_eq!(side_item_at(bar, sep, Point { x: 536, y: 118 }), None);
    }

    #[test]
    fn palette_hit_testing() {
        let (cell, gap) = (24, 6);
        let (w, h) = palette_size(cell, gap);
        assert_eq!((w, h), (4 * 24 + 5 * 6, 2 * 24 + 3 * 6));
        let pal = Rect {
            x: 200,
            y: 300,
            w,
            h,
        };
        for i in 0..8usize {
            let (col, row) = ((i % 4) as i32, (i / 4) as i32);
            let c = Point {
                x: 200 + 6 + col * 30 + 12,
                y: 300 + 6 + row * 30 + 12,
            };
            assert_eq!(palette_cell_at(pal, cell, gap, c), Some(i), "ô {i}");
            let r = palette_cell_rect(pal, cell, gap, i);
            assert_eq!(
                r,
                Rect {
                    x: c.x - 12,
                    y: c.y - 12,
                    w: 24,
                    h: 24
                }
            );
        }
        let at = |x, y| palette_cell_at(pal, cell, gap, Point { x, y });
        // Khe ngang, khe dọc, lề, ngoài bảng.
        assert_eq!(at(200 + 6 + 24 + 3, 300 + 18), None);
        assert_eq!(at(200 + 18, 300 + 6 + 24 + 3), None);
        assert_eq!(at(202, 302), None);
        assert_eq!(at(200 + w - 2, 300 + 18), None);
        assert_eq!(at(199, 318), None);
    }

    #[test]
    fn segment_invalidation_is_local() {
        use darkshot_core::annotate::{dip_to_px, DEFAULT_COLOR, STROKE_DIP};
        let dpi = 144;
        let width = dip_to_px(STROKE_DIP, dpi);
        let mut s = begin_stroke(Tool::Pen, Pt { x: 100.0, y: 100.0 }, dpi, DEFAULT_COLOR).unwrap();
        // Nét dài trước đó: hộp làm mới không phụ thuộc độ dài nét.
        for i in 1..=2000 {
            let p = Pt {
                x: 100.0 + i as f32 * 1.5,
                y: 100.0 + (i % 7) as f32,
            };
            assert!(extend_stroke(&mut s, p, false).is_some());
        }
        let Shape::Pen { points, .. } = &s.shape else {
            panic!("không phải Pen");
        };
        let p = *points.last().unwrap();
        let q = Pt {
            x: p.x + 5.0,
            y: p.y - 3.0,
        };
        let r = extend_stroke(&mut s, q, false).unwrap();
        let pad = width / 2.0 + 2.0;
        let (x0, x1) = (p.x.min(q.x) - pad, p.x.max(q.x) + pad);
        let (y0, y1) = (p.y.min(q.y) - pad, p.y.max(q.y) + pad);
        // Chứa hộp bao đoạn pq nới nửa độ dày + 2 px...
        assert!(r.x as f32 <= x0 && r.y as f32 <= y0, "{r:?}");
        assert!(r.right() as f32 >= x1 && r.bottom() as f32 >= y1, "{r:?}");
        // ...và không lớn hơn (chỉ làm tròn ra ngoài, dưới 1 px mỗi phía).
        assert!(r.x as f32 > x0 - 1.0 && r.y as f32 > y0 - 1.0, "{r:?}");
        assert!(
            (r.right() as f32) < x1 + 1.0 && (r.bottom() as f32) < y1 + 1.0,
            "{r:?}"
        );
        // Dịch dưới 1 px: không thêm điểm, không làm mới.
        let tiny = Pt {
            x: q.x + 0.4,
            y: q.y,
        };
        assert_eq!(extend_stroke(&mut s, tiny, false), None);
    }

    /// Lớp phủ thật được điều khiển bằng thông điệp giả từ một luồng phụ.
    mod live {
        use super::*;
        use crate::capture::Screenshot;
        use std::sync::atomic::Ordering;
        use std::time::Duration;
        use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, IsWindow, PostMessageW};

        pub enum Step {
            Down(Point),
            Move(Point),
            Up(Point),
            /// WM_KEYDOWN không phím bổ trợ.
            Key(u16),
            /// WM_KEYDOWN khi Ctrl đang giữ.
            Ctrl(u16),
            /// Phím tắt Esc toàn cục.
            Hotkey,
            /// Lớp phủ vẫn mở sau bước này.
            Alive,
        }

        pub fn drag(a: Point, b: Point) -> Vec<Step> {
            let mid = Point {
                x: (a.x + b.x) / 2,
                y: (a.y + b.y) / 2,
            };
            vec![Step::Down(a), Step::Move(mid), Step::Move(b), Step::Up(b)]
        }

        pub fn click(p: Point) -> Vec<Step> {
            vec![Step::Move(p), Step::Down(p), Step::Up(p)]
        }

        pub struct Layout {
            pub bar: Rect,
            pub side: Rect,
            pub palette: Rect,
            pub dpi: u32,
        }

        impl Layout {
            fn center(r: Rect) -> Point {
                Point {
                    x: r.x + r.w / 2,
                    y: r.y + r.h / 2,
                }
            }

            /// Giữa mục thứ `i` của thanh dọc.
            pub fn item(&self, i: usize) -> Point {
                Self::center(side_item_rect(self.side, scaled(SIDE_SEP, self.dpi), i))
            }

            /// Giữa ô màu thứ `i`.
            pub fn cell(&self, i: usize) -> Point {
                let (cell, gap) = (scaled(CELL, self.dpi), scaled(GAP, self.dpi));
                Self::center(palette_cell_rect(self.palette, cell, gap, i))
            }

            /// Giữa nút Lưu của thanh bốn nút.
            pub fn save(&self) -> Point {
                let b = self.bar.h;
                Point {
                    x: self.bar.x + b + b / 2,
                    y: self.bar.y + b / 2,
                }
            }
        }

        fn dpi_of(shot: &Screenshot, mon: Rect) -> u32 {
            monitor_dpi_at(POINT {
                x: shot.origin.x + mon.x + mon.w / 2,
                y: shot.origin.y + mon.y + mon.h / 2,
            })
        }

        /// Vị trí các thanh, tính như `State`.
        pub fn layout(shot: &Screenshot, sel: Rect) -> Layout {
            let corner = Point {
                x: sel.right() - 1,
                y: sel.bottom() - 1,
            };
            let mon = monitor_containing(corner, &shot.monitors);
            let dpi = dpi_of(shot, mon);
            let (bar, side) = bars_for(sel, mon, dpi);
            let palette = palette_rect_for(side, sel, mon, dpi);
            Layout {
                bar,
                side,
                palette,
                dpi,
            }
        }

        /// DPI của màn hình chứa điểm `p` (tọa độ ảnh chụp).
        pub fn dpi_at(shot: &Screenshot, p: Point) -> u32 {
            dpi_of(shot, monitor_containing(p, &shot.monitors))
        }

        /// Vùng chọn thử trên màn hình đầu tiên.
        pub fn test_sel(shot: &Screenshot) -> Rect {
            let m0 = shot.monitors[0];
            Rect {
                x: m0.x + 100,
                y: m0.y + 100,
                w: 300,
                h: 200,
            }
        }

        /// Mở lớp phủ, chạy `steps` từ luồng phụ. Trả kết quả và `false` nếu một bước `Alive`
        /// thấy lớp phủ đã đóng, hoặc lớp phủ không tự đóng sau bước cuối (khi đó gửi WM_CLOSE).
        pub fn drive(
            shot: &Screenshot,
            initial: Option<Rect>,
            steps: Vec<Step>,
        ) -> (Result<Option<Picked>, String>, bool) {
            let driver = std::thread::spawn(move || {
                let lp =
                    |p: Point| LPARAM(((p.y as isize & 0xFFFF) << 16) | (p.x as isize & 0xFFFF));
                let mut found = None;
                for _ in 0..50 {
                    std::thread::sleep(Duration::from_millis(100));
                    // SAFETY: tìm cửa sổ lớp phủ theo tên lớp.
                    if let Ok(h) = unsafe { FindWindowW(CLASS, None) } {
                        found = Some(h);
                        break;
                    }
                }
                let Some(h) = found else { return false };
                std::thread::sleep(Duration::from_millis(300));
                let post = |m: u32, wp: usize, l: LPARAM| {
                    // SAFETY: đăng thông điệp vào hàng đợi của cửa sổ lớp phủ.
                    let _ = unsafe { PostMessageW(Some(h), m, WPARAM(wp), l) };
                };
                // SAFETY: chỉ hỏi cửa sổ còn sống không.
                let alive = || unsafe { IsWindow(Some(h)) }.as_bool();
                let mut ok = true;
                for step in steps {
                    match step {
                        Step::Down(p) => post(WM_LBUTTONDOWN, 1, lp(p)),
                        Step::Move(p) => post(WM_MOUSEMOVE, 1, lp(p)),
                        Step::Up(p) => post(WM_LBUTTONUP, 0, lp(p)),
                        Step::Key(vk) => post(WM_KEYDOWN, usize::from(vk), LPARAM(0)),
                        Step::Ctrl(vk) => {
                            TEST_MODS.store(1, Ordering::SeqCst);
                            post(WM_KEYDOWN, usize::from(vk), LPARAM(0));
                            std::thread::sleep(Duration::from_millis(200));
                            TEST_MODS.store(0, Ordering::SeqCst);
                        }
                        Step::Hotkey => post(WM_HOTKEY, ESC_HOTKEY_ID as usize, LPARAM(0)),
                        Step::Alive => {
                            std::thread::sleep(Duration::from_millis(300));
                            ok &= alive();
                        }
                    }
                    std::thread::sleep(Duration::from_millis(40));
                }
                for _ in 0..30 {
                    if !alive() {
                        return ok;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
                post(WM_CLOSE, 0, LPARAM(0));
                false
            });
            let r = pick_region(shot, initial);
            let ok = driver.join().unwrap();
            (r, ok)
        }

        pub fn pt(p: Point) -> Pt {
            Pt {
                x: p.x as f32,
                y: p.y as f32,
            }
        }
    }

    fn live_setup() -> crate::capture::Screenshot {
        use crate::capture::capture_virtual_screen;
        use windows::Win32::UI::HiDpi::{
            SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        };
        // SAFETY: như ứng dụng thật (manifest PerMonitorV2), áp cho luồng kiểm thử.
        unsafe {
            SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
        capture_virtual_screen(false).unwrap()
    }

    #[test]
    #[ignore = "mở lớp phủ thật; chạy tay: cargo test -p darkshot-windows -- --ignored drag_outside_with_shapes_keeps_everything --test-threads=1"]
    fn drag_outside_with_shapes_keeps_everything() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        let shot = live_setup();
        let sel = live::test_sel(&shot);
        let l = live::layout(&shot, sel);
        let at = |dx, dy| Point {
            x: sel.x + dx,
            y: sel.y + dy,
        };
        let mut steps = vec![live::Step::Key(u16::from(b'P'))];
        steps.extend(live::drag(at(30, 40), at(150, 90)));
        steps.push(live::Step::Key(u16::from(b'P')));
        // Kéo ngoài vùng (trên trái vùng chọn) khi đã có hình, không công cụ.
        steps.extend(live::drag(at(-80, -80), at(-20, -30)));
        steps.extend(live::click(l.save()));
        let (r, ok) = live::drive(&shot, Some(sel), steps);
        assert!(ok);
        let p = r.unwrap().unwrap();
        assert_eq!((p.action, p.rect), (Action::Save, sel));
        assert_eq!(p.shapes.len(), 1);
        assert!(matches!(p.shapes[0], Shape::Pen { .. }));
    }

    #[test]
    #[ignore = "mở lớp phủ thật; chạy tay: cargo test -p darkshot-windows -- --ignored draw_line_and_save_returns_shape --test-threads=1"]
    fn draw_line_and_save_returns_shape() {
        use darkshot_core::annotate::{dip_to_px, PALETTE};
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        let shot = live_setup();
        let sel = live::test_sel(&shot);
        let l = live::layout(&shot, sel);
        let (a, b) = (
            Point {
                x: sel.x + 40,
                y: sel.y + 50,
            },
            Point {
                x: sel.x + 210,
                y: sel.y + 130,
            },
        );
        let width = dip_to_px(3.0, live::dpi_at(&shot, a));
        let line = |color| Shape::Line {
            a: live::pt(a),
            b: live::pt(b),
            width,
            color,
        };
        // Chọn màu xanh lam qua bảng màu, chọn Đường thẳng, kéo, Ctrl+S.
        let mut steps = live::click(l.item(6));
        steps.extend(live::click(l.cell(4)));
        steps.push(live::Step::Key(u16::from(b'L')));
        steps.extend(live::drag(a, b));
        steps.push(live::Step::Ctrl(u16::from(b'S')));
        let (r, ok) = live::drive(&shot, Some(sel), steps);
        assert!(ok);
        let p = r.unwrap().unwrap();
        assert_eq!((p.action, p.rect), (Action::Save, sel));
        assert_eq!(p.shapes, vec![line(PALETTE[4])]);
        // Lần mở sau nhớ màu; công cụ thì không (L chọn lại).
        let mut steps = vec![live::Step::Key(u16::from(b'L'))];
        steps.extend(live::drag(a, b));
        steps.push(live::Step::Ctrl(u16::from(b'S')));
        let (r, ok) = live::drive(&shot, Some(sel), steps);
        assert!(ok);
        assert_eq!(r.unwrap().unwrap().shapes, vec![line(PALETTE[4])]);
    }

    #[test]
    #[ignore = "mở lớp phủ thật; chạy tay: cargo test -p darkshot-windows -- --ignored undo_removes_last_shape --test-threads=1"]
    fn undo_removes_last_shape() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        let shot = live_setup();
        let sel = live::test_sel(&shot);
        let l = live::layout(&shot, sel);
        let at = |dx, dy| Point {
            x: sel.x + dx,
            y: sel.y + dy,
        };
        let mut steps = vec![live::Step::Key(u16::from(b'R'))];
        steps.extend(live::drag(at(20, 20), at(120, 90)));
        steps.push(live::Step::Key(u16::from(b'H')));
        steps.extend(live::drag(at(30, 150), at(250, 160)));
        steps.push(live::Step::Ctrl(u16::from(b'Z')));
        steps.extend(live::click(l.save()));
        let (r, ok) = live::drive(&shot, Some(sel), steps);
        assert!(ok);
        let p = r.unwrap().unwrap();
        assert_eq!(p.shapes.len(), 1);
        assert!(matches!(p.shapes[0], Shape::Rect { .. }));
    }

    #[test]
    #[ignore = "mở lớp phủ thật; chạy tay: cargo test -p darkshot-windows -- --ignored no_tool_flow_unchanged --test-threads=1"]
    fn no_tool_flow_unchanged() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        let shot = live_setup();
        let sel = live::test_sel(&shot);
        let l = live::layout(&shot, sel);
        let mut steps = live::drag(
            Point { x: sel.x, y: sel.y },
            Point {
                x: sel.right(),
                y: sel.bottom(),
            },
        );
        steps.extend(live::click(l.save()));
        let (r, ok) = live::drive(&shot, None, steps);
        assert!(ok);
        assert_eq!(
            r,
            Ok(Some(Picked {
                action: Action::Save,
                rect: sel,
                shapes: Vec::new(),
            }))
        );
    }

    #[test]
    #[ignore = "mở lớp phủ thật; chạy tay: cargo test -p darkshot-windows -- --ignored esc_closes_palette_not_overlay --test-threads=1"]
    fn esc_closes_palette_not_overlay() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        let shot = live_setup();
        let sel = live::test_sel(&shot);
        let l = live::layout(&shot, sel);
        // Esc bằng WM_KEYDOWN.
        let mut steps = live::click(l.item(6));
        steps.push(live::Step::Key(VK_ESCAPE.0));
        steps.push(live::Step::Alive);
        steps.push(live::Step::Key(VK_ESCAPE.0));
        let (r, ok) = live::drive(&shot, Some(sel), steps);
        assert!(
            ok,
            "Esc thứ nhất phải chỉ đóng bảng màu, Esc thứ hai đóng lớp phủ"
        );
        assert_eq!(r, Ok(None));
        // Esc qua phím tắt toàn cục.
        let mut steps = live::click(l.item(6));
        steps.push(live::Step::Hotkey);
        steps.push(live::Step::Alive);
        steps.push(live::Step::Hotkey);
        let (r, ok) = live::drive(&shot, Some(sel), steps);
        assert!(ok, "phím tắt Esc thứ nhất phải chỉ đóng bảng màu");
        assert_eq!(r, Ok(None));
    }
}
