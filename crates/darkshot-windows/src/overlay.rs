//! Lớp phủ toàn màn hình để chọn vùng và đánh dấu.
//!
//! Cửa sổ `WS_POPUP` phủ đúng màn hình ảo, nên tọa độ client trùng tọa độ ảnh.
//! Mọi trạng thái kéo thả nằm trong `geometry::Selection`; hình đánh dấu trong
//! `annotate::Annotations`. Ở đây chỉ vẽ và nhận sự kiện.

use crate::annotate_draw::{caret_rect, draw_shapes, shape_bounds};
use crate::capture::Screenshot;
use crate::gdiplus::{GdiPlus, Gp};
use crate::lang;
use darkshot_core::actions::Action;
use darkshot_core::annotate::{
    add_point, constrain_45, constrain_square, dip_to_px, press_mode, sanitize_paste, too_short,
    Annotations, PressMode, Pt, Shape, TextEdit, Tool, DEFAULT_COLOR, MARKER_DIP, PALETTE,
    PALETTE_COLS, STROKE_DIP, TEXT_DIP,
};
use darkshot_core::geometry::{
    monitor_containing, place_side_toolbar, place_toolbar, Handle, Hit, Point, Rect, Selection,
};
use darkshot_core::i18n::Msg;
use darkshot_core::theme::{
    Rgb, ACCENT, OVERLAY_BORDER, OVERLAY_HOVER, OVERLAY_ICON, OVERLAY_LABEL, OVERLAY_TIP, SURFACE,
    TEXT, TEXT_DISABLED, WHITE,
};
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateDIBSection,
    CreateFontIndirectW, CreatePen, CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, EndPaint,
    FillRect, GetStockObject, GetTextExtentPoint32W, IntersectClipRect, InvalidateRect,
    MonitorFromPoint, Rectangle, RoundRect, SelectClipRgn, SelectObject, SetBkMode, SetTextColor,
    StretchDIBits, UpdateWindow, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, DT_CENTER,
    DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, FW_BOLD, HBITMAP, HDC, HFONT, HGDIOBJ, LOGFONTW,
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
use windows::Win32::UI::Input::Ime::{
    ImmGetCompositionStringW, ImmGetContext, ImmNotifyIME, ImmReleaseContext,
    ImmSetCompositionFontW, ImmSetCompositionWindow, CFS_DEFAULT, CFS_POINT, COMPOSITIONFORM,
    CPS_CANCEL, GCS_COMPSTR, NI_COMPOSITIONSTR,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, RegisterHotKey, ReleaseCapture, SetCapture, SetFocus, UnregisterHotKey,
    MOD_NOREPEAT, VIRTUAL_KEY, VK_CONTROL, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_HOME, VK_LEFT,
    VK_MENU, VK_RIGHT, VK_SHIFT, VK_UP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    GetCaretBlinkTime, GetClientRect, GetCursorPos, GetForegroundWindow, GetMessageW,
    GetWindowRect, GetWindowThreadProcessId, KillTimer, LoadCursorW, PostQuitMessage,
    RegisterClassExW, SetCursor, SetForegroundWindow, SetTimer, SetWindowPos, ShowWindow,
    TranslateMessage, HTCLIENT, IDC_ARROW, IDC_CROSS, IDC_IBEAM, IDC_SIZEALL, IDC_SIZENESW,
    IDC_SIZENS, IDC_SIZENWSE, IDC_SIZEWE, MSG, NONCLIENTMETRICSW, SPI_GETNONCLIENTMETRICS,
    SWP_NOACTIVATE, SWP_NOZORDER, SW_SHOW, WM_ACTIVATE, WM_CAPTURECHANGED, WM_CHAR, WM_CLOSE,
    WM_DESTROY, WM_DPICHANGED, WM_ERASEBKGND, WM_HOTKEY, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN,
    WM_LBUTTONUP, WM_MOUSEMOVE, WM_PAINT, WM_SETCURSOR, WM_TIMER, WNDCLASSEXW, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_POPUP,
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
const CELL: i32 = 20;
/// Khe và lề của bảng màu.
const PAL_GAP: i32 = 4;
/// Ô vuông màu hiện tại trên nút Màu.
const SWATCH: i32 = 16;
/// Vạch đánh dấu công cụ đang chọn ở mép trái nút, và viền ô màu đang chọn.
const MARK: i32 = 2;

/// ID phím tắt Esc tạm (khác ID phím chụp của cửa sổ chính).
const ESC_HOTKEY_ID: i32 = 0x0E5C;
const FOCUS_TIMER_ID: usize = 1;
const FOCUS_RETRY_MS: u32 = 100;
const MAX_FOCUS_RETRIES: u32 = 5;
/// Bộ hẹn giờ nháy con trỏ của ô chữ.
const CARET_TIMER_ID: usize = 2;
/// Khung nét đứt quanh ô chữ: cách chữ, rộng tối thiểu khi ô rỗng, nét và khe (DIP).
const TEXT_FRAME_PAD: i32 = 3;
const TEXT_EMPTY_W: i32 = 40;
const TEXT_DASH: i32 = 4;
const TEXT_DASH_GAP: i32 = 3;

// Màu lấy từ `darkshot_core::theme`; biểu tượng GDI+ dùng ARGB (0xFF + RGB).
const ICON_ARGB: u32 = argb(OVERLAY_ICON);
const ICON_HOVER_ARGB: u32 = argb(ACCENT);
const ICON_DISABLED_ARGB: u32 = argb(TEXT_DISABLED);

const fn argb(c: Rgb) -> u32 {
    0xFF00_0000 | ((c.0 as u32) << 16) | ((c.1 as u32) << 8) | c.2 as u32
}

fn cr(c: Rgb) -> COLORREF {
    crate::theme::colorref(c)
}

thread_local! {
    /// Màu nét nhớ qua các lần mở lớp phủ trong tiến trình.
    static LAST_COLOR: Cell<Rgb> = const { Cell::new(DEFAULT_COLOR) };
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

impl SideItem {
    fn tip(self) -> Msg {
        match self {
            SideItem::Tool(Tool::Pen) => Msg::ToolPen,
            SideItem::Tool(Tool::Line) => Msg::ToolLine,
            SideItem::Tool(Tool::Arrow) => Msg::ToolArrow,
            SideItem::Tool(Tool::Rect) => Msg::ToolRect,
            SideItem::Tool(Tool::Marker) => Msg::ToolMarker,
            SideItem::Tool(Tool::Text) => Msg::ToolText,
            SideItem::Color => Msg::ToolColor,
            SideItem::Undo => Msg::ToolUndo,
        }
    }
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
/// Chỉ số của nút Màu trong `SIDE_ITEMS`.
const COLOR_INDEX: usize = 6;

/// Số vạch ngăn nằm phía trên mục thứ `i`.
fn seps_before(i: usize) -> i32 {
    match i {
        0..=5 => 0,
        6 => 1,
        _ => 2,
    }
}

/// Cỡ thanh dọc với nút `b` và ô vạch ngăn cao `sep` (gồm lề).
fn side_bar_size(b: i32, sep: i32) -> (i32, i32) {
    (b, b * SIDE_ITEMS.len() as i32 + 2 * sep)
}

/// Ô của mục thứ `i` trên thanh dọc `bar` (nút vuông cạnh `bar.w`).
fn side_item_rect(bar: Rect, sep: i32, i: usize) -> Rect {
    let b = bar.w;
    Rect {
        x: bar.x,
        y: bar.y + b * i as i32 + sep * seps_before(i),
        w: b,
        h: b,
    }
}

/// Mục của thanh dọc dưới điểm `p`; vạch ngăn và lề của nó trả `None`.
fn side_item_at(bar: Rect, sep: i32, p: Point) -> Option<SideItem> {
    if bar.w <= 0 || !bar.contains(p) {
        return None;
    }
    SIDE_ITEMS
        .iter()
        .enumerate()
        .find(|(i, _)| side_item_rect(bar, sep, *i).contains(p))
        .map(|(_, item)| *item)
}

/// Cỡ bảng màu 8 cột × 6 hàng ô cạnh `cell`, khe và lề `gap`.
fn palette_size(cell: i32, gap: i32) -> (i32, i32) {
    let (cols, rows) = (PALETTE_COLS as i32, (PALETTE.len() / PALETTE_COLS) as i32);
    (
        cols * cell + (cols + 1) * gap,
        rows * cell + (rows + 1) * gap,
    )
}

/// Ô màu thứ `i` (theo hàng, 8 ô mỗi hàng) của bảng `pal`.
fn palette_cell_rect(pal: Rect, cell: i32, gap: i32, i: usize) -> Rect {
    let (col, row) = ((i % PALETTE_COLS) as i32, (i / PALETTE_COLS) as i32);
    Rect {
        x: pal.x + gap + col * (cell + gap),
        y: pal.y + gap + row * (cell + gap),
        w: cell,
        h: cell,
    }
}

/// Ô màu dưới điểm `p`; khe và lề trả `None`.
fn palette_cell_at(pal: Rect, cell: i32, gap: i32, p: Point) -> Option<usize> {
    if !pal.contains(p) {
        return None;
    }
    (0..PALETTE.len()).find(|i| palette_cell_rect(pal, cell, gap, *i).contains(p))
}

/// Thanh bốn nút và thanh dọc cho vùng chọn `sel`, màn hình chứa góc dưới phải `mon` ở `dpi`.
fn bars_for(sel: Rect, mon: Rect, dpi: u32) -> (Rect, Rect) {
    let b = scaled(BUTTON, dpi);
    let gap = scaled(GAP, dpi);
    let bar = place_toolbar(sel, (b * ACTIONS.len() as i32, b), gap, mon);
    let size = side_bar_size(b, scaled(SIDE_SEP, dpi));
    let side = place_side_toolbar(sel, size, gap, mon, Some(bar));
    (bar, side)
}

/// Thanh dọc nằm ở nửa phải vùng chọn (ngoài bên phải, hoặc trong vùng sát mép phải).
fn side_on_right(side: Rect, sel: Rect) -> bool {
    2 * side.x + side.w > 2 * sel.x + sel.w
}

/// Đặt `r` vào trong `m` theo cả hai chiều.
fn clamp_into(mut r: Rect, m: Rect) -> Rect {
    r.x = r.x.min(m.right() - r.w).max(m.x);
    r.y = r.y.min(m.bottom() - r.h).max(m.y);
    r
}

/// Bảng màu cạnh nút Màu của thanh dọc `side`: bên trái thanh nếu thanh nằm nửa phải vùng chọn,
/// ngược lại bên phải; giữa theo chiều dọc nút Màu; kẹp vào màn hình `mon`.
fn palette_rect_for(side: Rect, sel: Rect, mon: Rect, dpi: u32) -> Rect {
    let gap = scaled(GAP, dpi);
    let (w, h) = palette_size(scaled(CELL, dpi), scaled(PAL_GAP, dpi));
    let btn = side_item_rect(side, scaled(SIDE_SEP, dpi), COLOR_INDEX);
    let x = if side_on_right(side, sel) {
        side.x - gap - w
    } else {
        side.right() + gap
    };
    clamp_into(
        Rect {
            x,
            y: btn.y + btn.h / 2 - h / 2,
            w,
            h,
        },
        mon,
    )
}

/// Phím bổ trợ giả cho kiểm thử gửi thông điệp (bit 0: Ctrl, bit 1: Shift).
#[cfg(test)]
static TEST_MODS: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// Phím `vk` đang được giữ (theo trạng thái bàn phím của luồng này).
fn key_held(vk: VIRTUAL_KEY) -> bool {
    #[cfg(test)]
    {
        let m = TEST_MODS.load(std::sync::atomic::Ordering::SeqCst);
        if (vk == VK_CONTROL && m & 1 != 0) || (vk == VK_SHIFT && m & 2 != 0) {
            return true;
        }
    }
    // SAFETY: đọc trạng thái phím của luồng này.
    unsafe { GetKeyState(i32::from(vk.0)) < 0 }
}

/// `WM_KEYDOWN` là lần lặp của phím đang giữ (bit 30 của `lParam`: phím đã xuống trước đó).
fn key_repeat(lp: LPARAM) -> bool {
    lp.0 & (1 << 30) != 0
}

/// Nét đang vẽ: công cụ, điểm đầu (tọa độ ảnh chụp) và hình hiện tại.
#[derive(Clone, Debug, PartialEq)]
struct Stroke {
    tool: Tool,
    start: Pt,
    shape: Shape,
    /// Điểm chuột cuối (để nhấn hoặc nhả Shift cập nhật nét mà không cần di chuột).
    cursor: Pt,
}

/// Bắt đầu nét tại `p` với độ dày theo `dpi` của màn hình chứa `p`; `None` với Chữ.
fn begin_stroke(tool: Tool, p: Pt, dpi: u32, color: Rgb) -> Option<Stroke> {
    let dip = if tool == Tool::Marker {
        MARKER_DIP
    } else {
        STROKE_DIP
    };
    let width = dip_to_px(dip, dpi);
    let shape = match tool {
        Tool::Pen => Shape::Pen {
            points: vec![p],
            width,
            color,
        },
        Tool::Marker => Shape::Marker {
            points: vec![p],
            width,
            color,
        },
        Tool::Line => Shape::Line {
            a: p,
            b: p,
            width,
            color,
        },
        Tool::Arrow => Shape::Arrow {
            a: p,
            b: p,
            width,
            color,
        },
        Tool::Rect => Shape::Rect {
            a: p,
            b: p,
            width,
            color,
        },
        Tool::Text => return None,
    };
    Some(Stroke {
        tool,
        start: p,
        shape,
        cursor: p,
    })
}

/// Hộp bao đoạn `p`–`q` nới `pad` mỗi phía, làm tròn ra ngoài.
fn segment_dirty(p: Pt, q: Pt, pad: f32) -> Rect {
    let x0 = (p.x.min(q.x) - pad).floor() as i32;
    let y0 = (p.y.min(q.y) - pad).floor() as i32;
    let x1 = (p.x.max(q.x) + pad).ceil() as i32;
    let y1 = (p.y.max(q.y) + pad).ceil() as i32;
    Rect {
        x: x0,
        y: y0,
        w: x1.saturating_sub(x0),
        h: y1.saturating_sub(y0),
    }
}

fn union(a: Option<Rect>, b: Option<Rect>) -> Option<Rect> {
    match (a, b) {
        (Some(a), Some(b)) => {
            let x0 = a.x.min(b.x);
            let y0 = a.y.min(b.y);
            let x1 = a.right().max(b.right());
            let y1 = a.bottom().max(b.bottom());
            Some(Rect {
                x: x0,
                y: y0,
                w: x1 - x0,
                h: y1 - y0,
            })
        }
        (a, None) => a,
        (None, b) => b,
    }
}

/// Kéo nét tới `p` (`shift`: khóa góc 45° hoặc hình vuông). Trả vùng cần vẽ lại, `None` khi không đổi.
/// Nét tự do chỉ làm mới hộp bao đoạn mới (nới nửa độ dày, đầu vuông của bút dạ quang thì nhân
/// căn 2, cộng 2 px khử răng cưa), không phụ thuộc độ dài nét.
fn extend_stroke(s: &mut Stroke, p: Pt, shift: bool) -> Option<Rect> {
    s.cursor = p;
    let marker = s.tool == Tool::Marker;
    match &mut s.shape {
        Shape::Pen { points, width, .. } | Shape::Marker { points, width, .. } => {
            let last = *points.last()?;
            let n = points.len();
            add_point(points, p);
            if points.len() == n {
                return None;
            }
            let half = *width / 2.0;
            let reach = if marker {
                half * std::f32::consts::SQRT_2
            } else {
                half
            };
            Some(segment_dirty(last, p, reach + 2.0))
        }
        _ => {
            let end = match (s.tool, shift) {
                (Tool::Line | Tool::Arrow, true) => constrain_45(s.start, p),
                (Tool::Rect, true) => constrain_square(s.start, p),
                _ => p,
            };
            let before = s.shape.bounds();
            match &mut s.shape {
                Shape::Line { b, .. } | Shape::Arrow { b, .. } | Shape::Rect { b, .. } => {
                    if *b == end {
                        return None;
                    }
                    *b = end;
                }
                _ => return None,
            }
            union(before, s.shape.bounds())
        }
    }
}

/// Hình chốt từ nét vừa vẽ; `None` khi Đường thẳng, Mũi tên, Hình chữ nhật quá ngắn.
fn end_stroke(s: Stroke) -> Option<Shape> {
    match &s.shape {
        Shape::Line { a, b, .. } | Shape::Arrow { a, b, .. } | Shape::Rect { a, b, .. }
            if too_short(s.tool, *a, *b) =>
        {
            None
        }
        _ => Some(s.shape),
    }
}

const NO_RECT: Rect = Rect {
    x: 0,
    y: 0,
    w: 0,
    h: 0,
};

/// Ô chữ đang gõ cùng hình học đã đo (đo lại mỗi lần ô đổi, không đo khi vẽ hay khi nháy).
struct TextBox {
    edit: TextEdit,
    /// Điểm bấm tạo ô (tọa độ ảnh chụp).
    at: Point,
    /// DPI của màn hình chứa `at`.
    dpi: u32,
    /// Con trỏ nháy (tọa độ ảnh chụp).
    caret: Rect,
    /// Khung nét đứt (tọa độ ảnh chụp).
    frame: Rect,
}

impl TextBox {
    fn new(at: Point, dpi: u32, color: Rgb) -> TextBox {
        let origin = Pt {
            x: at.x as f32,
            y: at.y as f32,
        };
        let edit = TextEdit::new(origin, dip_to_px(TEXT_DIP, dpi), color);
        let mut b = TextBox {
            edit,
            at,
            dpi,
            caret: NO_RECT,
            frame: NO_RECT,
        };
        b.measure();
        b
    }

    /// Đo lại con trỏ và khung. Khung gồm phần mực của chữ và con trỏ (phần mực không gồm khoảng
    /// trắng cuối dòng), nới `TEXT_FRAME_PAD`; ô chỉ có khoảng trắng thì tính từ điểm đặt ô, cao
    /// một dòng, rộng tối thiểu `TEXT_EMPTY_W` (không gọi `shape_bounds` cho ô rỗng).
    fn measure(&mut self) {
        self.caret = caret_rect(&self.edit);
        let base = match self.edit.to_shape() {
            Some(shape) => union(Some(shape_bounds(&shape)), Some(self.caret)),
            None => union(
                Some(Rect {
                    x: self.at.x,
                    y: self.at.y,
                    w: scaled(TEXT_EMPTY_W, self.dpi),
                    h: self.caret.h,
                }),
                Some(self.caret),
            ),
        }
        .unwrap_or(self.caret);
        self.frame = inflate(base, scaled(TEXT_FRAME_PAD, self.dpi));
    }

    /// Vùng màn hình ô chiếm (chữ, khung, con trỏ).
    fn dirty(&self) -> Rect {
        union(Some(inflate(self.frame, 1)), Some(self.caret)).unwrap_or(self.frame)
    }
}

/// Bộ gõ đang có chuỗi soạn dở trên cửa sổ `hwnd`.
fn ime_composing(hwnd: HWND) -> bool {
    // SAFETY: ngữ cảnh IME của cửa sổ luồng này, trả lại ngay bằng ImmReleaseContext.
    unsafe {
        let himc = ImmGetContext(hwnd);
        if himc.is_invalid() {
            return false;
        }
        let n = ImmGetCompositionStringW(himc, GCS_COMPSTR, None, 0);
        let _ = ImmReleaseContext(hwnd, himc);
        n > 0
    }
}

/// Kết thúc chuỗi soạn dở của bộ gõ trên `hwnd` khi ô chữ đóng. `keep`: trả chuỗi đang soạn để
/// chèn thẳng vào ô cũ (hoàn tất); không thì bỏ. Chuỗi soạn luôn được xóa bằng `CPS_CANCEL`, nên bộ
/// gõ không gửi thêm WM_CHAR sau đó (sẽ rơi vào ô mới hoặc mất). Đặt lại cửa sổ soạn về
/// `CFS_DEFAULT` để khi không có ô chữ bộ gõ hành xử như trước.
fn ime_end_composition(hwnd: HWND, keep: bool) -> Option<String> {
    let form = COMPOSITIONFORM {
        dwStyle: CFS_DEFAULT,
        ..Default::default()
    };
    // SAFETY: ngữ cảnh IME của cửa sổ luồng này, trả lại ở cuối khối trên mọi đường đi; bộ đệm
    // đọc chuỗi soạn đủ `bytes` byte theo kích thước bộ gõ vừa báo.
    unsafe {
        let himc = ImmGetContext(hwnd);
        if himc.is_invalid() {
            return None;
        }
        let bytes = ImmGetCompositionStringW(himc, GCS_COMPSTR, None, 0);
        let mut pending = None;
        if bytes > 0 {
            if keep {
                let mut buf = vec![0u16; (bytes as usize).div_ceil(2)];
                let got = ImmGetCompositionStringW(
                    himc,
                    GCS_COMPSTR,
                    Some(buf.as_mut_ptr() as *mut c_void),
                    (buf.len() * 2) as u32,
                );
                if got > 0 {
                    let n = (got as usize / 2).min(buf.len());
                    pending = buf.get(..n).map(String::from_utf16_lossy);
                }
            }
            let _ = ImmNotifyIME(himc, NI_COMPOSITIONSTR, CPS_CANCEL, 0);
        }
        let _ = ImmSetCompositionWindow(himc, &form);
        let _ = ImmReleaseContext(hwnd, himc);
        pending.filter(|s| !s.is_empty())
    }
}

/// Đặt cửa sổ soạn của bộ gõ tại `caret` (tọa độ client) với phông cỡ gần chữ đánh dấu.
fn place_ime(hwnd: HWND, caret: Rect, size_px: f32) {
    let form = COMPOSITIONFORM {
        dwStyle: CFS_POINT,
        ptCurrentPos: POINT {
            x: caret.x,
            y: caret.y,
        },
        ..Default::default()
    };
    let mut lf = LOGFONTW {
        lfHeight: -(size_px.round() as i32).max(1),
        lfWeight: FW_BOLD.0 as i32,
        ..Default::default()
    };
    for (slot, u) in lf.lfFaceName.iter_mut().zip(wide("Segoe UI")) {
        *slot = u;
    }
    // SAFETY: ngữ cảnh IME của cửa sổ luồng này, trả lại ngay; hai cấu trúc sống suốt lời gọi.
    unsafe {
        let himc = ImmGetContext(hwnd);
        if himc.is_invalid() {
            return;
        }
        let _ = ImmSetCompositionFontW(himc, &lf);
        let _ = ImmSetCompositionWindow(himc, &form);
        let _ = ImmReleaseContext(hwnd, himc);
    }
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
    /// Công cụ vẽ đang chọn; `None` thì lớp phủ chạy như khi chưa có công cụ vẽ.
    tool: Option<Tool>,
    /// Các hình đã chốt (tọa độ ảnh chụp).
    marks: Annotations,
    /// Hộp bao (tọa độ ảnh chụp) của từng hình trong `marks`, cùng thứ tự; tính lúc chốt.
    mark_bounds: Vec<Rect>,
    /// Ô chữ đang gõ.
    text: Option<TextBox>,
    /// Con trỏ nháy của ô chữ đang hiện.
    caret_on: bool,
    /// Nửa đầu của cặp UTF-16 (ký tự ngoài BMP) chờ nửa sau qua WM_CHAR.
    high_surrogate: Option<u16>,
    /// Phím tắt Esc toàn cục đang được lớp phủ đăng ký.
    esc_registered: bool,
    /// Màu cho hình vẽ sau đó.
    color: Rgb,
    /// Nét đang kéo (chưa chốt vào `marks`).
    stroke: Option<Stroke>,
    side_hover: Option<SideItem>,
    side_pressed: Option<SideItem>,
    /// Bảng màu đang mở.
    palette: bool,
    cell_pressed: Option<usize>,
    result: Option<Option<Picked>>,
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

/// Vị trí các thanh cho vùng chọn hiện tại.
struct Bars {
    sel: Rect,
    /// Màn hình chứa góc dưới phải vùng chọn.
    mon: Rect,
    dpi: u32,
    /// Thanh bốn nút.
    bar: Rect,
    /// Thanh công cụ vẽ (dọc).
    side: Rect,
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

    /// Vị trí hai thanh; `None` khi chưa có vùng chọn hoặc đang kéo khung mới.
    fn bars(&self) -> Option<Bars> {
        let sel = self.sel.rect()?;
        if self.sel.is_dragging() && self.new_drag {
            return None;
        }
        let corner = Point {
            x: sel.right() - 1,
            y: sel.bottom() - 1,
        };
        let mon = monitor_containing(corner, &self.monitors);
        let dpi = self.monitor_dpi(mon);
        let (bar, side) = bars_for(sel, mon, dpi);
        Some(Bars {
            sel,
            mon,
            dpi,
            bar,
            side,
        })
    }

    fn toolbar(&self) -> Option<(Rect, u32)> {
        self.bars().map(|b| (b.bar, b.dpi))
    }

    fn side_bar(&self) -> Option<(Rect, u32)> {
        self.bars().map(|b| (b.side, b.dpi))
    }

    fn side_item_at(&self, p: Point) -> Option<SideItem> {
        let (side, dpi) = self.side_bar()?;
        side_item_at(side, scaled(SIDE_SEP, dpi), p)
    }

    /// Bảng màu khi đang mở.
    fn palette_rect(&self) -> Option<(Rect, u32)> {
        if !self.palette {
            return None;
        }
        let b = self.bars()?;
        Some((palette_rect_for(b.side, b.sel, b.mon, b.dpi), b.dpi))
    }

    fn palette_cell_at(&self, p: Point) -> Option<usize> {
        let (pal, dpi) = self.palette_rect()?;
        palette_cell_at(pal, scaled(CELL, dpi), scaled(PAL_GAP, dpi), p)
    }

    /// Đang kéo (vùng chọn hoặc nét vẽ).
    fn dragging(&self) -> bool {
        self.sel.is_dragging() || self.stroke.is_some()
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

    /// Chú thích của nút đang rê chuột (thanh bốn nút hoặc thanh dọc): vị trí, chữ, DPI.
    fn tooltip(&mut self) -> Option<(Rect, Vec<u16>, u32)> {
        if self.side_hover.is_some() {
            return self.side_tooltip();
        }
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

    /// Chú thích của mục thanh dọc: cạnh thanh, phía xa vùng chọn (không đủ chỗ thì phía kia),
    /// giữa theo chiều dọc nút.
    fn side_tooltip(&mut self) -> Option<(Rect, Vec<u16>, u32)> {
        let item = self.side_hover?;
        let b = self.bars()?;
        let dpi = b.dpi;
        let i = SIDE_ITEMS.iter().position(|x| *x == item)?;
        let btn = side_item_rect(b.side, scaled(SIDE_SEP, dpi), i);
        let text = wide(lang::tr(item.tip()));
        let (tw, th) = self.text_size(&text, dpi);
        let (w, h) = (tw + 2 * scaled(8, dpi), th + 2 * scaled(4, dpi));
        let gap = scaled(7, dpi);
        let right_x = b.side.right() + gap;
        let left_x = b.side.x - gap - w;
        let fits_right = right_x + w <= b.mon.right();
        let fits_left = left_x >= b.mon.x;
        let x = if side_on_right(b.side, b.sel) {
            if fits_right || !fits_left {
                right_x
            } else {
                left_x
            }
        } else if fits_left || !fits_right {
            left_x
        } else {
            right_x
        };
        let rect = clamp_into(
            Rect {
                x,
                y: btn.y + btn.h / 2 - h / 2,
                w,
                h,
            },
            b.mon,
        );
        Some((rect, text, dpi))
    }

    /// Thanh dọc, bảng màu và chú thích: các vùng đổi khi chọn công cụ, màu, rê chuột trên thanh.
    fn bar_chrome(&mut self) -> Vec<Rect> {
        let mut v = Vec::new();
        if let Some((r, _)) = self.side_bar() {
            v.push(inflate(r, 1));
        }
        if let Some((r, _)) = self.palette_rect() {
            v.push(inflate(r, 1));
        }
        if let Some((r, _, _)) = self.tooltip() {
            v.push(r);
        }
        v
    }

    /// Mọi vùng có vẽ thêm ngoài nền tối (khung, tay nắm, nhãn, các thanh, bảng màu, chú thích).
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
        v.extend(self.bar_chrome());
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

    /// Như `update` nhưng chỉ cho thay đổi trên thanh dọc, bảng màu, chú thích (không làm mới vùng chọn).
    fn update_bars(&mut self, f: impl FnOnce(&mut State)) {
        let before = self.bar_chrome();
        f(self);
        let after = self.bar_chrome();
        self.invalidate(&before);
        self.invalidate(&after);
    }

    fn cursor_at(&self, p: Point) -> PCWSTR {
        if self.dragging() {
            return self.drag_cursor;
        }
        if self.palette {
            // Bảng màu mở: cú bấm nào ngoài bảng cũng chỉ đóng bảng.
            return IDC_ARROW;
        }
        let on_bar = [self.toolbar(), self.side_bar(), self.palette_rect()]
            .iter()
            .flatten()
            .any(|(r, _)| r.contains(p));
        if on_bar {
            return IDC_ARROW;
        }
        let hit = self.sel.hit_test(p, self.hit_radius(p));
        match press_mode(self.tool, !self.marks.is_empty(), hit) {
            PressMode::Draw if self.tool == Some(Tool::Text) => IDC_IBEAM,
            PressMode::Draw => IDC_CROSS,
            PressMode::Ignore => IDC_ARROW,
            PressMode::Selection => cursor_for_hit(hit),
        }
    }

    fn finish(&mut self, a: Action) {
        self.close_text(true);
        if a == Action::Cancel {
            self.result = Some(None);
        } else if let Some(r) = self.sel.rect() {
            self.result = Some(Some(Picked {
                action: a,
                rect: r,
                shapes: std::mem::take(&mut self.marks).into_shapes(),
            }));
            self.mark_bounds.clear();
        }
    }

    fn toggle_tool(&mut self, t: Tool) {
        self.close_text(true);
        self.update_bars(|s| s.tool = if s.tool == Some(t) { None } else { Some(t) });
    }

    /// Hoàn tác. Đang gõ: ô có chữ thì chỉ xóa ô đó; ô rỗng thì hủy ô rồi hoàn tác hình cuối.
    fn undo(&mut self) {
        if let Some(b) = &self.text {
            let had_text = !b.edit.text().is_empty();
            self.close_text(false);
            if had_text {
                return;
            }
        }
        let mut removed = None;
        self.update_bars(|s| removed = s.pop_mark());
        if let Some(r) = removed {
            self.invalidate(&[r]);
        }
    }

    /// Chốt hình vào `marks`, lưu sẵn hộp bao để khi vẽ bỏ qua hình không giao vùng cần vẽ.
    fn push_mark(&mut self, shape: Shape) {
        self.mark_bounds.push(shape_bounds(&shape));
        self.marks.push(shape);
    }

    /// Bỏ hình cuối; trả hộp bao của nó.
    fn pop_mark(&mut self) -> Option<Rect> {
        let shape = self.marks.undo()?;
        Some(
            self.mark_bounds
                .pop()
                .unwrap_or_else(|| shape_bounds(&shape)),
        )
    }

    /// Đăng ký (`on`) hoặc gỡ phím tắt Esc toàn cục của lớp phủ. Gỡ khi ô chữ mở để Esc tới được
    /// bộ gõ (hủy chuỗi soạn) và tới lớp phủ qua WM_KEYDOWN; đăng ký lại khi ô đóng.
    fn set_esc_hotkey(&mut self, on: bool) {
        if on == self.esc_registered {
            return;
        }
        // SAFETY: phím tắt gắn với cửa sổ lớp phủ của luồng này; EscHotkey gỡ lần cuối khi đóng.
        unsafe {
            if on {
                self.esc_registered = RegisterHotKey(
                    Some(self.hwnd),
                    ESC_HOTKEY_ID,
                    MOD_NOREPEAT,
                    u32::from(VK_ESCAPE.0),
                )
                .is_ok();
            } else {
                let _ = UnregisterHotKey(Some(self.hwnd), ESC_HOTKEY_ID);
                self.esc_registered = false;
            }
        }
    }

    /// (Đặt lại) bộ hẹn giờ nháy con trỏ theo `GetCaretBlinkTime`; con trỏ hiện ngay.
    /// Hệ thống tắt nháy (INFINITE hoặc 0) thì con trỏ đứng yên.
    fn restart_blink(&mut self) {
        self.caret_on = true;
        // SAFETY: chỉ đọc cài đặt hệ thống và đặt bộ hẹn giờ của chính cửa sổ lớp phủ
        // (mất theo cửa sổ; close_text hủy sớm hơn).
        unsafe {
            let ms = GetCaretBlinkTime();
            if ms == 0 || ms == u32::MAX {
                let _ = KillTimer(Some(self.hwnd), CARET_TIMER_ID);
            } else {
                SetTimer(Some(self.hwnd), CARET_TIMER_ID, ms, None);
            }
        }
    }

    /// Nhịp nháy: đảo con trỏ, chỉ làm mới ô con trỏ (không đo lại chữ).
    fn blink(&mut self) {
        let Some(caret) = self.text.as_ref().map(|b| b.caret) else {
            return;
        };
        self.caret_on = !self.caret_on;
        self.invalidate(&[caret]);
    }

    /// Mở ô chữ tại `p` (đã chốt ô cũ).
    fn open_text(&mut self, p: Point) {
        let b = TextBox::new(p, self.dpi_at(p), self.color);
        self.invalidate(&[b.dirty()]);
        place_ime(self.hwnd, b.caret, dip_to_px(TEXT_DIP, b.dpi));
        self.text = Some(b);
        self.high_surrogate = None;
        self.set_esc_hotkey(false);
        self.restart_blink();
        // Nút Hoàn tác sáng lên khi có ô chữ.
        let bars = self.bar_chrome();
        self.invalidate(&bars);
    }

    /// Đóng ô chữ: `commit` thì chốt vào `marks` (bỏ nếu chỉ có khoảng trắng), không thì hủy.
    fn close_text(&mut self, commit: bool) {
        if self.text.is_none() {
            return;
        }
        // Chuỗi bộ gõ đang soạn: chốt thì hoàn tất vào chính ô này, hủy thì bỏ.
        let pending = ime_end_composition(self.hwnd, commit);
        let Some(mut b) = self.text.take() else {
            return;
        };
        self.invalidate(&[b.dirty()]);
        if let Some(s) = pending {
            b.edit.insert(&s);
        }
        self.high_surrogate = None;
        // SAFETY: hủy bộ hẹn giờ nháy của chính cửa sổ lớp phủ.
        unsafe {
            let _ = KillTimer(Some(self.hwnd), CARET_TIMER_ID);
        }
        self.set_esc_hotkey(true);
        if commit {
            if let Some(shape) = b.edit.to_shape() {
                self.push_mark(shape);
                // Chữ hoàn tất từ bộ gõ có thể vượt khung cũ.
                let added: Vec<Rect> = self.mark_bounds.last().copied().into_iter().collect();
                self.invalidate(&added);
            }
        }
        // Nút Hoàn tác có thể đổi trạng thái (mờ, sáng).
        let bars = self.bar_chrome();
        self.invalidate(&bars);
    }

    /// Sửa ô chữ đang gõ qua `f`, đo lại, làm mới vùng cũ và mới, dời cửa sổ soạn của bộ gõ.
    fn edit_text(&mut self, f: impl FnOnce(&mut TextEdit)) {
        let Some(b) = self.text.as_mut() else {
            return;
        };
        let before = b.dirty();
        f(&mut b.edit);
        b.measure();
        let (after, caret, size) = (b.dirty(), b.caret, dip_to_px(TEXT_DIP, b.dpi));
        self.invalidate(&[before, after]);
        place_ime(self.hwnd, caret, size);
        self.restart_blink();
    }

    /// WM_CHAR khi đang gõ; `false` khi không có ô chữ (để xử lý mặc định).
    fn on_char(&mut self, unit: u16) -> bool {
        if self.text.is_none() {
            return false;
        }
        if (0xD800..0xDC00).contains(&unit) {
            self.high_surrogate = Some(unit);
            return true;
        }
        let s = if (0xDC00..0xE000).contains(&unit) {
            match self.high_surrogate.take() {
                Some(h) => String::from_utf16_lossy(&[h, unit]),
                None => return true,
            }
        } else {
            self.high_surrogate = None;
            match unit {
                0x08 => {
                    self.edit_text(|e| e.backspace());
                    return true;
                }
                0x0D => "\n".to_string(),
                // Ký tự điều khiển khác (Ctrl+V, Ctrl+Z, Tab, Ctrl+Enter...) không vào ô.
                u if u < 0x20 || u == 0x7F => return true,
                u => String::from_utf16_lossy(&[u]),
            }
        };
        self.edit_text(|e| e.insert(&s));
        true
    }

    /// Phím khi đang gõ (WM_KEYDOWN, trừ Esc): sửa, di con trỏ, Ctrl+Z/V/D/S/C.
    fn text_key(&mut self, vk: u16) {
        if key_held(VK_CONTROL) {
            match vk {
                0x5A => self.undo(),  // Z
                0x56 => self.paste(), // V
                0x44 => self.finish(Action::Upload),
                0x53 => self.finish(Action::Save),
                0x43 => self.finish(Action::Copy),
                _ => {}
            }
            return;
        }
        let mv: fn(&mut TextEdit) = match VIRTUAL_KEY(vk) {
            VK_DELETE => TextEdit::delete,
            VK_LEFT => TextEdit::left,
            VK_RIGHT => TextEdit::right,
            VK_UP => TextEdit::up,
            VK_DOWN => TextEdit::down,
            VK_HOME => TextEdit::home,
            VK_END => TextEdit::end,
            _ => return,
        };
        self.edit_text(mv);
    }

    /// Ctrl+V: dán chữ thuần của clipboard vào ô chữ.
    fn paste(&mut self) {
        if self.text.is_none() {
            return;
        }
        let Some(raw) = crate::clipboard::paste_text(self.hwnd) else {
            return;
        };
        let clean = sanitize_paste(&raw);
        if !clean.is_empty() {
            self.edit_text(|e| e.insert(&clean));
        }
    }

    /// Nhấn hoặc nhả Shift khi đang kéo Đường thẳng, Mũi tên, Hình chữ nhật: cập nhật nét ngay
    /// theo điểm chuột cuối.
    fn shift_changed(&mut self, shift: bool) {
        let Some(st) = self.stroke.as_mut() else {
            return;
        };
        if !matches!(st.tool, Tool::Line | Tool::Arrow | Tool::Rect) {
            return;
        }
        let at = st.cursor;
        if let Some(r) = extend_stroke(st, at, shift) {
            self.invalidate(&[r]);
        }
    }

    fn pick_color(&mut self, i: usize) {
        let Some(c) = PALETTE.get(i).copied() else {
            return;
        };
        LAST_COLOR.with(|l| l.set(c));
        self.update_bars(|s| {
            s.color = c;
            s.palette = false;
        });
        self.edit_text(|e| e.set_color(c));
    }

    fn activate(&mut self, item: SideItem) {
        match item {
            SideItem::Tool(t) => self.toggle_tool(t),
            SideItem::Color => self.update_bars(|s| s.palette = !s.palette),
            SideItem::Undo => self.undo(),
        }
    }

    /// Esc (phím hoặc phím tắt toàn cục): bảng màu đang mở thì chỉ đóng bảng; đang gõ thì hủy ô
    /// chữ (trừ khi bộ gõ đang soạn: Esc thuộc về bộ gõ); còn lại đóng lớp phủ.
    fn escape(&mut self) {
        if self.palette {
            self.update_bars(|s| {
                s.palette = false;
                s.cell_pressed = None;
            });
        } else if self.text.is_some() {
            if !ime_composing(self.hwnd) {
                self.close_text(false);
            }
        } else {
            self.result = Some(None);
        }
    }

    /// Bấm trong vùng chọn khi công cụ Chữ đang chọn: chốt ô đang gõ (nếu có), mở ô mới tại `p`.
    fn text_press(&mut self, p: Point) -> Effects {
        self.close_text(true);
        self.open_text(p);
        Effects::default()
    }

    /// Chốt nét đang vẽ vào `marks` (hoặc bỏ nếu quá ngắn).
    fn commit_stroke(&mut self) {
        let Some(s) = self.stroke.take() else {
            return;
        };
        let drawn = s.shape.bounds();
        match end_stroke(s) {
            Some(shape) => self.update_bars(|st| st.push_mark(shape)),
            None => self.invalidate(&drawn.into_iter().collect::<Vec<_>>()),
        }
    }

    /// Mất chuột giữa chừng: chốt nét, kết thúc lần kéo vùng.
    fn lost_capture(&mut self) {
        if self.stroke.is_some() {
            self.commit_stroke();
        }
        if self.sel.is_dragging() {
            self.update(|s| s.sel.release());
        }
    }

    fn on_down(&mut self, p: Point) -> Effects {
        if self.dragging() {
            return Effects::default();
        }
        // Bấm ra chỗ khác thì chốt ô chữ; trên thanh dọc (Màu, Hoàn tác, công cụ tự chốt khi cần)
        // và khi bảng màu mở (để đổi màu ô đang gõ) thì chưa.
        let on_side = self.side_bar().is_some_and(|(r, _)| r.contains(p));
        if self.text.is_some() && !self.palette && !on_side {
            self.close_text(true);
        }
        if self.palette {
            if self.palette_rect().is_some_and(|(r, _)| r.contains(p)) {
                self.cell_pressed = self.palette_cell_at(p);
            } else {
                // Bấm ngoài bảng màu chỉ đóng bảng (kể cả trên nút Màu), không làm gì khác.
                self.update_bars(|s| {
                    s.palette = false;
                    s.cell_pressed = None;
                });
            }
            return Effects::default();
        }
        if let Some((bar, _)) = self.toolbar() {
            if bar.contains(p) {
                self.pressed = button_at(bar, p);
                return Effects::default();
            }
        }
        if let Some((side, _)) = self.side_bar() {
            if side.contains(p) {
                self.side_pressed = self.side_item_at(p);
                return Effects::default();
            }
        }
        let rad = self.hit_radius(p);
        let hit = self.sel.hit_test(p, rad);
        match press_mode(self.tool, !self.marks.is_empty(), hit) {
            PressMode::Ignore => Effects::default(),
            PressMode::Draw => {
                let Some(tool) = self.tool else {
                    return Effects::default();
                };
                if tool == Tool::Text {
                    return self.text_press(p);
                }
                let at = Pt {
                    x: p.x as f32,
                    y: p.y as f32,
                };
                let Some(stroke) = begin_stroke(tool, at, self.dpi_at(p), self.color) else {
                    return Effects::default();
                };
                let dirty = stroke.shape.bounds();
                self.update_bars(|s| {
                    s.hover = None;
                    s.side_hover = None;
                    s.pressed = None;
                    s.side_pressed = None;
                    s.drag_cursor = IDC_CROSS;
                    s.stroke = Some(stroke);
                });
                self.invalidate(&dirty.into_iter().collect::<Vec<_>>());
                Effects {
                    capture: true,
                    ..Default::default()
                }
            }
            PressMode::Selection => {
                self.update(|s| {
                    s.new_drag = hit == Hit::Outside;
                    s.drag_cursor = cursor_for_hit(hit);
                    s.hover = None;
                    s.side_hover = None;
                    s.pressed = None;
                    s.side_pressed = None;
                    s.sel.press(p, rad);
                });
                Effects {
                    capture: true,
                    ..Default::default()
                }
            }
        }
    }

    fn on_move(&mut self, p: Point) {
        if let Some(stroke) = self.stroke.as_mut() {
            let at = Pt {
                x: p.x as f32,
                y: p.y as f32,
            };
            if let Some(r) = extend_stroke(stroke, at, key_held(VK_SHIFT)) {
                self.invalidate(&[r]);
            }
        } else if self.sel.is_dragging() {
            self.update(|s| s.sel.drag_to(p));
        } else {
            let hover = self.toolbar().and_then(|(bar, _)| button_at(bar, p));
            if hover != self.hover {
                self.update(|s| s.hover = hover);
            }
            let side_hover = self.side_item_at(p);
            if side_hover != self.side_hover {
                self.update_bars(|s| s.side_hover = side_hover);
            }
        }
        set_cursor(self.cursor_at(p));
    }

    fn on_up(&mut self, p: Point) -> Effects {
        if let Some(stroke) = self.stroke.as_mut() {
            let at = Pt {
                x: p.x as f32,
                y: p.y as f32,
            };
            if let Some(r) = extend_stroke(stroke, at, key_held(VK_SHIFT)) {
                self.invalidate(&[r]);
            }
            self.commit_stroke();
            return Effects {
                release: true,
                ..Default::default()
            };
        }
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
        if let Some(i) = self.cell_pressed.take() {
            if self.palette_cell_at(p) == Some(i) {
                self.pick_color(i);
            }
            return Effects::default();
        }
        if let Some(item) = self.side_pressed.take() {
            if self.side_item_at(p) == Some(item) {
                self.activate(item);
            }
            return Effects::default();
        }
        if let Some(a) = self.pressed.take() {
            if self.toolbar().and_then(|(bar, _)| button_at(bar, p)) == Some(a) {
                self.finish(a);
            }
        }
        Effects::default()
    }

    /// WM_KEYDOWN; `repeat`: lần lặp của phím đang giữ.
    fn on_key(&mut self, vk: u16, repeat: bool) {
        if vk == VK_ESCAPE.0 {
            self.escape();
            return;
        }
        if vk == VK_SHIFT.0 {
            self.shift_changed(true);
            return;
        }
        if self.text.is_some() {
            self.text_key(vk);
            return;
        }
        if self.dragging() || self.sel.rect().is_none() {
            return;
        }
        let ctrl = key_held(VK_CONTROL);
        if !ctrl {
            if !key_held(VK_MENU) && !repeat {
                if let Some(t) = Tool::from_key(vk) {
                    self.toggle_tool(t);
                }
            }
            return;
        }
        let a = match vk {
            0x5A => {
                // Z
                self.undo();
                return;
            }
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
        // Hình đánh dấu: sau ảnh sáng, trước khung và các thanh; cắt theo vùng chọn.
        if let (Some(clip), true) = (sel, self.gdiplus) {
            // Chỉ vẽ các hình có hộp bao giao vùng cần vẽ; giữ thứ tự bằng cách vẽ từng dải liền.
            let shapes = self.marks.shapes();
            let visible = |i: usize| {
                self.mark_bounds
                    .get(i)
                    .is_none_or(|r| intersect(*r, clip).is_some())
            };
            let mut i = 0;
            while i < shapes.len() {
                if !visible(i) {
                    i += 1;
                    continue;
                }
                let start = i;
                while i < shapes.len() && visible(i) {
                    i += 1;
                }
                if let Some(run) = shapes.get(start..i) {
                    draw_shapes(p.dc, run, p.ox, p.oy, clip);
                }
            }
            if let Some(s) = &self.stroke {
                draw_shapes(p.dc, std::slice::from_ref(&s.shape), p.ox, p.oy, clip);
            }
            let typing = self
                .text
                .as_ref()
                .filter(|b| intersect(b.frame, clip).is_some())
                .and_then(|b| b.edit.to_shape());
            if let Some(shape) = typing {
                draw_shapes(p.dc, std::slice::from_ref(&shape), p.ox, p.oy, clip);
            }
        }
        // Khung nét đứt và con trỏ của ô đang gõ: chỉ trên màn hình, không in ra ảnh.
        if let Some(b) = &self.text {
            p.dashed_frame(b.frame, b.dpi);
            if self.caret_on {
                p.fill(b.caret, ACCENT);
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
        if let Some((side, dpi)) = self.side_bar() {
            let look = SideLook {
                tool: self.tool,
                hover: self.side_hover,
                color: self.color,
                // Đang gõ thì Hoàn tác xóa ô chữ (hoặc hủy ô rỗng rồi hoàn tác hình cuối).
                can_undo: !self.marks.is_empty() || self.text.is_some(),
            };
            p.side_toolbar(side, dpi, &look, self.gdiplus);
        }
        if let Some((pal, dpi)) = self.palette_rect() {
            p.palette(pal, dpi, self.color);
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

/// Trạng thái hiển thị của thanh công cụ vẽ.
struct SideLook {
    tool: Option<Tool>,
    hover: Option<SideItem>,
    color: Rgb,
    can_undo: bool,
}

impl SideLook {
    fn selected(&self, item: SideItem) -> bool {
        matches!(item, SideItem::Tool(t) if Some(t) == self.tool)
    }

    fn enabled(&self, item: SideItem) -> bool {
        item != SideItem::Undo || self.can_undo
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

    /// Khung nét đứt mảnh `ACCENT` (1 px; nét `TEXT_DASH`, khe `TEXT_DASH_GAP` DIP), khe trong suốt.
    /// Vẽ từng nét bằng ô tô màu để độ dài nét đúng pixel (bút kiểu GDI tính theo "style unit").
    fn dashed_frame(&self, r: Rect, dpi: u32) {
        let (dash, gap) = (scaled(TEXT_DASH, dpi), scaled(TEXT_DASH_GAP, dpi));
        let mut segs = Vec::new();
        let mut x = r.x;
        while x < r.right() {
            let w = dash.min(r.right() - x);
            segs.push(Rect { x, y: r.y, w, h: 1 });
            segs.push(Rect {
                x,
                y: r.bottom() - 1,
                w,
                h: 1,
            });
            x += dash + gap;
        }
        let mut y = r.y;
        while y < r.bottom() {
            let h = dash.min(r.bottom() - y);
            segs.push(Rect { x: r.x, y, w: 1, h });
            segs.push(Rect {
                x: r.right() - 1,
                y,
                w: 1,
                h,
            });
            y += dash + gap;
        }
        // SAFETY: chổi tạo và xóa trong khối, không được chọn vào DC.
        unsafe {
            let brush = CreateSolidBrush(cr(ACCENT));
            if brush.is_invalid() {
                return;
            }
            for seg in segs {
                FillRect(self.dc, &self.rc(seg), brush);
            }
            let _ = DeleteObject(brush.into());
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

    /// Thanh công cụ vẽ (dọc): sáu công cụ, vạch ngăn, Màu, vạch ngăn, Hoàn tác.
    fn side_toolbar(&self, bar: Rect, dpi: u32, look: &SideLook, gdiplus: bool) {
        let b = bar.w;
        let sep = scaled(SIDE_SEP, dpi);
        self.round_rect(bar, scaled(6, dpi), SURFACE, OVERLAY_BORDER);
        let mark = scaled(MARK, dpi);
        let mark_inset = scaled(6, dpi);
        for (i, item) in SIDE_ITEMS.iter().enumerate() {
            let r = side_item_rect(bar, sep, i);
            let selected = look.selected(*item);
            if selected || (look.enabled(*item) && look.hover == Some(*item)) {
                self.fill(
                    Rect {
                        x: r.x + 1,
                        y: r.y + 1,
                        w: r.w - 2,
                        h: r.h - 2,
                    },
                    OVERLAY_HOVER,
                );
            }
            if selected {
                self.fill(
                    Rect {
                        x: r.x + 1,
                        y: r.y + mark_inset,
                        w: mark,
                        h: r.h - 2 * mark_inset,
                    },
                    ACCENT,
                );
            }
        }
        // Vạch ngăn 1 px giữa ô vạch ngăn, có lề hai bên.
        let margin = scaled(GAP, dpi);
        for y in [bar.y + 6 * b + sep / 2, bar.y + 7 * b + sep + sep / 2] {
            self.fill(
                Rect {
                    x: bar.x + margin,
                    y,
                    w: b - 2 * margin,
                    h: 1,
                },
                OVERLAY_BORDER,
            );
        }
        // Nút Màu: ô vuông tô màu hiện tại, viền OVERLAY_BORDER.
        let btn = side_item_rect(bar, sep, COLOR_INDEX);
        let sw = scaled(SWATCH, dpi);
        self.round_rect(
            Rect {
                x: btn.x + (btn.w - sw) / 2,
                y: btn.y + (btn.h - sw) / 2,
                w: sw,
                h: sw,
            },
            scaled(2, dpi),
            look.color,
            OVERLAY_BORDER,
        );
        if !gdiplus {
            return;
        }
        let Some(g) = Gp::new(self.dc) else { return };
        let icon = scaled(ICON, dpi) as f32;
        for (i, item) in SIDE_ITEMS.iter().enumerate() {
            if *item == SideItem::Color {
                continue;
            }
            let r = side_item_rect(bar, sep, i);
            let cx = (r.x - self.ox) as f32 + r.w as f32 * 0.5;
            let cy = (r.y - self.oy) as f32 + r.h as f32 * 0.5;
            let color = if !look.enabled(*item) {
                ICON_DISABLED_ARGB
            } else if look.selected(*item) || look.hover == Some(*item) {
                ICON_HOVER_ARGB
            } else {
                ICON_ARGB
            };
            g.side_icon(*item, cx, cy, icon, color);
        }
    }

    /// Bảng màu 8 × 6: nền SURFACE, viền OVERLAY_BORDER; ô đang chọn có viền TEXT.
    fn palette(&self, pal: Rect, dpi: u32, current: Rgb) {
        self.round_rect(pal, scaled(6, dpi), SURFACE, OVERLAY_BORDER);
        let (cell, gap) = (scaled(CELL, dpi), scaled(PAL_GAP, dpi));
        let ring = scaled(MARK, dpi);
        for (i, c) in PALETTE.iter().enumerate() {
            let r = palette_cell_rect(pal, cell, gap, i);
            if *c == current {
                // Viền TEXT dày `ring`, cách ô 1 px nền để nổi cả trên ô trắng.
                self.fill(inflate(r, ring + 1), TEXT);
                self.fill(inflate(r, 1), SURFACE);
            }
            self.round_rect(r, 0, *c, OVERLAY_BORDER);
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

/// Đường GDI+ đang dựng cho một biểu tượng: tọa độ trong lưới 24×24, gốc (`ox`, `oy`), hệ số `k`.
struct IconPath {
    path: *mut GpPath,
    ox: f32,
    oy: f32,
    k: f32,
}

impl IconPath {
    fn arc(&self, x: f32, y: f32, r: f32, start: f32, sweep: f32) {
        let k = self.k;
        // SAFETY: path hợp lệ suốt đời của IconPath (Gp::stroke_icon tạo và xóa).
        unsafe {
            GdipAddPathArc(
                self.path,
                self.ox + (x - r) * k,
                self.oy + (y - r) * k,
                2.0 * r * k,
                2.0 * r * k,
                start,
                sweep,
            );
        }
    }

    fn line(&self, x1: f32, y1: f32, x2: f32, y2: f32) {
        let (ox, oy, k) = (self.ox, self.oy, self.k);
        // SAFETY: như `arc`.
        unsafe {
            GdipAddPathLine(
                self.path,
                ox + x1 * k,
                oy + y1 * k,
                ox + x2 * k,
                oy + y2 * k,
            );
        }
    }

    fn start(&self) {
        // SAFETY: như `arc`.
        unsafe {
            GdipStartPathFigure(self.path);
        }
    }

    fn close(&self) {
        // SAFETY: như `arc`.
        unsafe {
            GdipClosePathFigure(self.path);
        }
    }

    /// Đường gấp khúc mở, bắt đầu hình mới.
    fn poly(&self, pts: &[(f32, f32)]) {
        self.start();
        for s in pts.windows(2) {
            let ((x1, y1), (x2, y2)) = (s[0], s[1]);
            self.line(x1, y1, x2, y2);
        }
    }
}

impl Gp {
    /// Vẽ nét mảnh (1,8 trong lưới 24×24, đầu và chỗ nối tròn) của đường do `build` dựng,
    /// cỡ `size` pixel, tâm (`cx`, `cy`).
    fn stroke_icon(&self, cx: f32, cy: f32, size: f32, argb: u32, build: impl FnOnce(&IconPath)) {
        let k = size / 24.0;
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
        }
        build(&IconPath {
            path,
            ox: cx - size / 2.0,
            oy: cy - size / 2.0,
            k,
        });
        // SAFETY: như trên; xóa đúng một lần.
        unsafe {
            GdipDrawPath(self.raw(), pen, path);
            GdipDeletePath(path);
            GdipDeletePen(pen);
        }
    }

    /// Vẽ biểu tượng trong lưới 24×24 (theo hình mẫu), cỡ `size` pixel, tâm (`cx`, `cy`).
    fn icon(&self, a: Action, cx: f32, cy: f32, size: f32, argb: u32) {
        self.stroke_icon(cx, cy, size, argb, |p| match a {
            Action::Upload => {
                // Đám mây: ba cung tròn (đổi từ đường SVG của hình mẫu), mở ở đáy.
                p.arc(6.989, 14.0, 4.0, 89.84, 173.14);
                p.arc(12.328, 11.4565, 6.0, 193.75, 147.22);
                p.arc(17.75, 13.75, 4.2574, 273.37, 180.0);
                // Mũi tên lên.
                p.poly(&[(12.0, 20.0), (12.0, 12.0)]);
                p.poly(&[(9.0, 15.0), (12.0, 12.0), (15.0, 15.0)]);
            }
            Action::Save => {
                p.poly(&[
                    (5.0, 3.0),
                    (16.0, 3.0),
                    (19.0, 6.0),
                    (19.0, 21.0),
                    (5.0, 21.0),
                ]);
                p.close();
                p.poly(&[(8.0, 3.0), (8.0, 9.0), (15.0, 9.0), (15.0, 3.0)]);
                p.poly(&[(8.0, 21.0), (8.0, 14.0), (16.0, 14.0), (16.0, 21.0)]);
            }
            Action::Copy => {
                // Ô trước: hình vuông bo góc (9,9)-(20,20), bán kính 2.
                p.start();
                p.arc(11.0, 11.0, 2.0, 180.0, 90.0);
                p.arc(18.0, 11.0, 2.0, 270.0, 90.0);
                p.arc(18.0, 18.0, 2.0, 0.0, 90.0);
                p.arc(11.0, 18.0, 2.0, 90.0, 90.0);
                p.close();
                // Ô sau: chỉ phần lộ ra (cạnh trái và cạnh trên).
                p.poly(&[(5.0, 15.0), (5.0, 5.0)]);
                p.arc(7.0, 5.0, 2.0, 180.0, 90.0);
                p.line(7.0, 3.0, 15.0, 3.0);
            }
            Action::Cancel => {
                p.poly(&[(6.0, 6.0), (18.0, 18.0)]);
                p.poly(&[(18.0, 6.0), (6.0, 18.0)]);
            }
        });
    }

    /// Biểu tượng của thanh công cụ vẽ, cùng lưới và nét với `icon` (tự vẽ, hình học đơn giản).
    fn side_icon(&self, item: SideItem, cx: f32, cy: f32, size: f32, argb: u32) {
        self.stroke_icon(cx, cy, size, argb, |p| match item {
            SideItem::Tool(Tool::Pen) => {
                // Bút chì chéo: thân, đầu nhọn ở dưới trái, vạch nối đầu bút.
                p.poly(&[
                    (4.0, 20.0),
                    (5.0, 15.5),
                    (15.5, 5.0),
                    (19.0, 8.5),
                    (8.5, 19.0),
                ]);
                p.close();
                p.poly(&[(13.0, 7.5), (16.5, 11.0)]);
            }
            SideItem::Tool(Tool::Line) => p.poly(&[(5.0, 19.0), (19.0, 5.0)]),
            SideItem::Tool(Tool::Arrow) => {
                p.poly(&[(5.0, 19.0), (18.5, 5.5)]);
                p.poly(&[(10.0, 5.0), (19.0, 5.0), (19.0, 14.0)]);
            }
            SideItem::Tool(Tool::Rect) => {
                p.poly(&[(4.0, 6.0), (20.0, 6.0), (20.0, 18.0), (4.0, 18.0)]);
                p.close();
            }
            SideItem::Tool(Tool::Marker) => {
                // Bút dạ quang: thân chéo, đầu vát, vạch tô bên dưới.
                p.poly(&[(9.0, 13.0), (15.0, 7.0), (19.0, 11.0), (13.0, 17.0)]);
                p.close();
                p.poly(&[(9.0, 13.0), (6.0, 18.0), (8.0, 20.0), (13.0, 17.0)]);
                p.poly(&[(12.0, 21.0), (20.0, 21.0)]);
            }
            SideItem::Tool(Tool::Text) => {
                p.poly(&[(5.0, 7.0), (5.0, 5.0), (19.0, 5.0), (19.0, 7.0)]);
                p.poly(&[(12.0, 5.0), (12.0, 19.0)]);
                p.poly(&[(9.0, 19.0), (15.0, 19.0)]);
            }
            SideItem::Undo => {
                // Mũi tên quay lại: đầu nhọn bên trái, thân vòng nửa tròn bên phải.
                p.poly(&[(8.0, 5.0), (4.0, 9.0), (8.0, 13.0)]);
                p.start();
                p.line(4.0, 9.0, 14.5, 9.0);
                p.arc(14.5, 14.5, 5.5, 270.0, 180.0);
                p.line(14.5, 20.0, 9.0, 20.0);
            }
            SideItem::Color => {}
        });
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
            // Mất chuột giữa chừng (cửa sổ khác chiếm): chốt nét đang vẽ, kết thúc lần kéo.
            with_state(|s| s.lost_capture());
            LRESULT(0)
        }
        WM_KEYDOWN => {
            with_state(|s| s.on_key(wp.0 as u16, key_repeat(lp)));
            LRESULT(0)
        }
        WM_KEYUP => {
            if wp.0 == usize::from(VK_SHIFT.0) {
                with_state(|s| s.shift_changed(false));
            }
            // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
            unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
        }
        WM_CHAR => {
            if with_state(|s| s.on_char(wp.0 as u16)) == Some(true) {
                return LRESULT(0);
            }
            // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
            unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
        }
        WM_TIMER if wp.0 == CARET_TIMER_ID => {
            with_state(|s| s.blink());
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
            with_state(|s| s.escape());
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
/// tiêu điểm. `State::set_esc_hotkey` đăng ký (và gỡ tạm khi ô chữ mở); guard này gỡ khi
/// `pick_region` kết thúc trên mọi đường đi, kể cả khi lớp phủ đóng giữa lúc đang gõ
/// (gỡ một phím chưa đăng ký chỉ trả lỗi, bỏ qua).
struct EscHotkey(HWND);

impl Drop for EscHotkey {
    fn drop(&mut self) {
        with_state(|s| s.esc_registered = false);
        // SAFETY: phím tắt của cửa sổ lớp phủ, gỡ khi cửa sổ còn sống (guard này thả trước WindowGuard).
        unsafe {
            let _ = UnregisterHotKey(Some(self.0), ESC_HOTKEY_ID);
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
        tool: None,
        marks: Annotations::default(),
        mark_bounds: Vec::new(),
        text: None,
        caret_on: false,
        high_surrogate: None,
        esc_registered: false,
        color: LAST_COLOR.with(|c| c.get()),
        stroke: None,
        side_hover: None,
        side_pressed: None,
        palette: false,
        cell_pressed: None,
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
    let _esc = EscHotkey(hwnd);
    with_state(|s| s.set_esc_hotkey(true));

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
        if let Some(Some(r)) = with_state(|s| s.result.take()) {
            return Ok(r);
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
        // Vòng có chữ: mở ô, gõ, bấm chỗ khác (chốt, mở ô mới), gõ, Ctrl+S.
        let mut steps = vec![live::Step::Key(u16::from(b'T'))];
        steps.extend(live::click(inside(30, 40)));
        steps.extend(live::chars("ab"));
        steps.extend(live::click(inside(60, 120)));
        steps.extend(live::chars("c"));
        steps.push(live::Step::Ctrl(u16::from(b'S')));
        let (r, ok) = live::drive(&shot, Some(sel), steps);
        assert!(ok);
        let p = r.unwrap().unwrap();
        assert_eq!(p.shapes.len(), 2);
        assert!(p.shapes.iter().all(|s| matches!(s, Shape::Text { .. })));
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
        let (cell, gap) = (20, 4);
        let (w, h) = palette_size(cell, gap);
        assert_eq!((w, h), (196, 148));
        let pal = Rect {
            x: 200,
            y: 300,
            w,
            h,
        };
        for i in 0..48usize {
            let (col, row) = ((i % 8) as i32, (i / 8) as i32);
            let c = Point {
                x: 200 + 4 + col * 24 + 10,
                y: 300 + 4 + row * 24 + 10,
            };
            assert_eq!(palette_cell_at(pal, cell, gap, c), Some(i), "ô {i}");
            let r = palette_cell_rect(pal, cell, gap, i);
            assert_eq!(
                r,
                Rect {
                    x: c.x - 10,
                    y: c.y - 10,
                    w: 20,
                    h: 20
                }
            );
        }
        let at = |x, y| palette_cell_at(pal, cell, gap, Point { x, y });
        // Ô 7 góc trên phải, ô 40 góc dưới trái, ô 47 góc dưới phải.
        assert_eq!(at(200 + w - 4 - 10, 300 + 14), Some(7));
        assert_eq!(at(200 + 14, 300 + h - 4 - 10), Some(40));
        assert_eq!(at(200 + w - 14, 300 + h - 14), Some(47));
        // Khe ngang, khe dọc, lề, ngoài bảng.
        assert_eq!(at(200 + 4 + 20 + 2, 300 + 14), None);
        assert_eq!(at(200 + 14, 300 + 4 + 20 + 2), None);
        assert_eq!(at(201, 301), None);
        assert_eq!(at(200 + w - 2, 300 + 14), None);
        assert_eq!(at(199, 314), None);
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
            /// WM_KEYDOWN lặp (phím đang giữ: bit 30 của lParam).
            KeyRepeat(u16),
            /// WM_CHAR với một đơn vị UTF-16.
            Char(u16),
            /// Gửi WM_CLOSE (đóng lớp phủ như khi hệ thống yêu cầu).
            Close,
            /// Phím Esc toàn cục đang trống (`true`: không ai giữ, kể cả lớp phủ) hay đang bị giữ.
            /// Kiểm bằng `RegisterHotKey` ở luồng phụ, gỡ ngay nếu đăng ký được.
            EscFree(bool),
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
                let (cell, gap) = (scaled(CELL, self.dpi), scaled(PAL_GAP, self.dpi));
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
                        Step::KeyRepeat(vk) => post(WM_KEYDOWN, usize::from(vk), LPARAM(1 << 30)),
                        Step::Char(c) => post(WM_CHAR, usize::from(c), LPARAM(1)),
                        Step::Close => post(WM_CLOSE, 0, LPARAM(0)),
                        Step::EscFree(want) => {
                            std::thread::sleep(Duration::from_millis(300));
                            ok &= esc_free() == want;
                        }
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

        /// Thử đăng ký Esc toàn cục ở luồng này; được thì gỡ ngay và trả `true`.
        fn esc_free() -> bool {
            const PROBE_ID: i32 = 0x0E5D;
            // SAFETY: phím tắt gắn với luồng này (không cửa sổ); gỡ ngay khi đăng ký được.
            unsafe {
                let free =
                    RegisterHotKey(None, PROBE_ID, MOD_NOREPEAT, u32::from(VK_ESCAPE.0)).is_ok();
                if free {
                    let _ = UnregisterHotKey(None, PROBE_ID);
                }
                free
            }
        }

        /// Các bước gửi chuỗi `s` bằng WM_CHAR (mỗi đơn vị UTF-16 một bước).
        pub fn chars(s: &str) -> Vec<Step> {
            s.encode_utf16().map(Step::Char).collect()
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
        // Chọn màu xanh lam (ô 5, #0080FF) qua bảng màu, chọn Đường thẳng, kéo, Ctrl+S.
        let mut steps = live::click(l.item(6));
        steps.extend(live::click(l.cell(5)));
        steps.push(live::Step::Key(u16::from(b'L')));
        steps.extend(live::drag(a, b));
        steps.push(live::Step::Ctrl(u16::from(b'S')));
        let (r, ok) = live::drive(&shot, Some(sel), steps);
        assert!(ok);
        let p = r.unwrap().unwrap();
        assert_eq!((p.action, p.rect), (Action::Save, sel));
        assert_eq!(p.shapes, vec![line(PALETTE[5])]);
        // Lần mở sau nhớ màu; công cụ thì không (L chọn lại).
        let mut steps = vec![live::Step::Key(u16::from(b'L'))];
        steps.extend(live::drag(a, b));
        steps.push(live::Step::Ctrl(u16::from(b'S')));
        let (r, ok) = live::drive(&shot, Some(sel), steps);
        assert!(ok);
        assert_eq!(r.unwrap().unwrap().shapes, vec![line(PALETTE[5])]);
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

    #[test]
    fn key_repeat_bit_is_read() {
        assert!(!key_repeat(LPARAM(0)));
        assert!(!key_repeat(LPARAM(1)));
        assert!(key_repeat(LPARAM(1 << 30)));
        assert!(key_repeat(LPARAM((1 << 30) | 1)));
    }

    /// Chữ (origin, nội dung) của các hình Text trong `shapes`; hình khác làm kiểm thử thất bại.
    fn texts(shapes: &[Shape]) -> Vec<(Pt, String)> {
        shapes
            .iter()
            .map(|s| match s {
                Shape::Text { origin, text, .. } => (*origin, text.clone()),
                other => panic!("không phải Text: {other:?}"),
            })
            .collect()
    }

    #[test]
    #[ignore = "mở lớp phủ thật; chạy tay: cargo test -p darkshot-windows -- --ignored held_tool_key_does_not_toggle --test-threads=1"]
    fn held_tool_key_does_not_toggle() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        let shot = live_setup();
        let sel = live::test_sel(&shot);
        let l = live::layout(&shot, sel);
        let at = |dx, dy| Point {
            x: sel.x + dx,
            y: sel.y + dy,
        };
        // Giữ P: một WM_KEYDOWN rồi các lần lặp; công cụ phải vẫn là Bút vẽ.
        let mut steps = vec![
            live::Step::Key(u16::from(b'P')),
            live::Step::KeyRepeat(u16::from(b'P')),
            live::Step::KeyRepeat(u16::from(b'P')),
            live::Step::KeyRepeat(u16::from(b'P')),
        ];
        steps.extend(live::drag(at(30, 40), at(150, 90)));
        steps.extend(live::click(l.save()));
        let (r, ok) = live::drive(&shot, Some(sel), steps);
        assert!(ok);
        let p = r.unwrap().unwrap();
        assert_eq!(p.shapes.len(), 1);
        assert!(matches!(p.shapes[0], Shape::Pen { .. }));
    }

    #[test]
    #[ignore = "mở lớp phủ thật; chạy tay: cargo test -p darkshot-windows -- --ignored wm_char_sequence_builds_text --test-threads=1"]
    fn wm_char_sequence_builds_text() {
        use darkshot_core::annotate::{dip_to_px, TEXT_DIP};
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        let shot = live_setup();
        let sel = live::test_sel(&shot);
        let at = Point {
            x: sel.x + 40,
            y: sel.y + 50,
        };
        const BS: u16 = 0x08;
        let mut steps = vec![live::Step::Key(u16::from(b'T'))];
        steps.extend(live::click(at));
        // Unikey: gõ "Viet" kiểu Telex, bộ gõ xóa rồi gửi lại ký tự dựng sẵn.
        steps.extend(live::chars("Vie"));
        steps.push(live::Step::Char(BS));
        steps.extend(live::chars("\u{ea}t"));
        steps.push(live::Step::Char(BS));
        steps.push(live::Step::Char(BS));
        steps.extend(live::chars("\u{1ec7}t"));
        // Enter rồi "2".
        steps.extend(live::chars("\r2"));
        steps.push(live::Step::Ctrl(u16::from(b'S')));
        let (r, ok) = live::drive(&shot, Some(sel), steps);
        assert!(ok);
        let p = r.unwrap().unwrap();
        assert_eq!((p.action, p.rect), (Action::Save, sel));
        assert_eq!(
            texts(&p.shapes),
            vec![(live::pt(at), "Vi\u{1ec7}t\n2".to_string())]
        );
        let Shape::Text { size, .. } = &p.shapes[0] else {
            unreachable!()
        };
        assert_eq!(*size, dip_to_px(TEXT_DIP, live::dpi_at(&shot, at)));
    }

    #[test]
    #[ignore = "mở lớp phủ thật; chạy tay: cargo test -p darkshot-windows -- --ignored esc_cancels_text_not_overlay --test-threads=1"]
    fn esc_cancels_text_not_overlay() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        let shot = live_setup();
        let sel = live::test_sel(&shot);
        let at = Point {
            x: sel.x + 40,
            y: sel.y + 50,
        };
        let typing = || {
            let mut steps = vec![live::Step::Key(u16::from(b'T'))];
            steps.extend(live::click(at));
            steps.extend(live::chars("abc"));
            steps
        };
        // Esc bằng WM_KEYDOWN: lần đầu hủy ô, lần sau đóng lớp phủ.
        let mut steps = typing();
        steps.push(live::Step::Key(VK_ESCAPE.0));
        steps.push(live::Step::Alive);
        steps.push(live::Step::Key(VK_ESCAPE.0));
        let (r, ok) = live::drive(&shot, Some(sel), steps);
        assert!(ok, "Esc thứ nhất phải chỉ hủy ô chữ");
        assert_eq!(r, Ok(None));
        // Esc qua phím tắt toàn cục.
        let mut steps = typing();
        steps.push(live::Step::Hotkey);
        steps.push(live::Step::Alive);
        steps.push(live::Step::Hotkey);
        let (r, ok) = live::drive(&shot, Some(sel), steps);
        assert!(ok, "phím tắt Esc thứ nhất phải chỉ hủy ô chữ");
        assert_eq!(r, Ok(None));
        // Ô bị hủy không được chốt.
        let mut steps = typing();
        steps.push(live::Step::Key(VK_ESCAPE.0));
        steps.push(live::Step::Alive);
        steps.push(live::Step::Ctrl(u16::from(b'S')));
        let (r, ok) = live::drive(&shot, Some(sel), steps);
        assert!(ok);
        assert_eq!(r.unwrap().unwrap().shapes, Vec::new());
    }

    #[test]
    #[ignore = "mở lớp phủ thật; chạy tay: cargo test -p darkshot-windows -- --ignored click_elsewhere_commits --test-threads=1"]
    fn click_elsewhere_commits() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        let shot = live_setup();
        let sel = live::test_sel(&shot);
        let l = live::layout(&shot, sel);
        let (a, b) = (
            Point {
                x: sel.x + 30,
                y: sel.y + 40,
            },
            Point {
                x: sel.x + 120,
                y: sel.y + 130,
            },
        );
        let mut steps = vec![live::Step::Key(u16::from(b'T'))];
        steps.extend(live::click(a));
        steps.extend(live::chars("A"));
        steps.extend(live::click(b));
        steps.extend(live::chars("B"));
        steps.extend(live::click(l.save()));
        let (r, ok) = live::drive(&shot, Some(sel), steps);
        assert!(ok);
        let p = r.unwrap().unwrap();
        assert_eq!(p.action, Action::Save);
        assert_eq!(
            texts(&p.shapes),
            vec![
                (live::pt(a), "A".to_string()),
                (live::pt(b), "B".to_string())
            ]
        );
    }

    #[test]
    #[ignore = "mở lớp phủ thật; chạy tay: cargo test -p darkshot-windows -- --ignored empty_text_is_dropped --test-threads=1"]
    fn empty_text_is_dropped() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        let shot = live_setup();
        let sel = live::test_sel(&shot);
        let l = live::layout(&shot, sel);
        let mut steps = vec![live::Step::Key(u16::from(b'T'))];
        steps.extend(live::click(Point {
            x: sel.x + 30,
            y: sel.y + 40,
        }));
        // Đổi công cụ bằng nút Bút vẽ trên thanh dọc (phím P lúc này đi vào ô chữ).
        steps.extend(live::click(l.item(0)));
        steps.extend(live::click(l.save()));
        let (r, ok) = live::drive(&shot, Some(sel), steps);
        assert!(ok);
        let p = r.unwrap().unwrap();
        assert_eq!((p.action, p.shapes), (Action::Save, Vec::new()));
    }

    #[test]
    #[ignore = "mở lớp phủ thật; chạy tay: cargo test -p darkshot-windows -- --ignored ctrl_z_while_typing --test-threads=1"]
    fn ctrl_z_while_typing() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        let shot = live_setup();
        let sel = live::test_sel(&shot);
        let at = |dx, dy| Point {
            x: sel.x + dx,
            y: sel.y + dy,
        };
        let setup = || {
            let mut steps = vec![live::Step::Key(u16::from(b'P'))];
            steps.extend(live::drag(at(20, 20), at(120, 80)));
            steps.push(live::Step::Key(u16::from(b'T')));
            steps.extend(live::click(at(40, 140)));
            steps.extend(live::chars("x"));
            steps
        };
        // Ctrl+Z một lần: xóa ô đang gõ, nét Pen còn.
        let mut steps = setup();
        steps.push(live::Step::Ctrl(u16::from(b'Z')));
        steps.push(live::Step::Ctrl(u16::from(b'S')));
        let (r, ok) = live::drive(&shot, Some(sel), steps);
        assert!(ok);
        let p = r.unwrap().unwrap();
        assert_eq!(p.shapes.len(), 1);
        assert!(matches!(p.shapes[0], Shape::Pen { .. }));
        // Ctrl+Z lần nữa: hoàn tác nét Pen.
        let mut steps = setup();
        steps.push(live::Step::Ctrl(u16::from(b'Z')));
        steps.push(live::Step::Ctrl(u16::from(b'Z')));
        steps.push(live::Step::Ctrl(u16::from(b'S')));
        let (r, ok) = live::drive(&shot, Some(sel), steps);
        assert!(ok);
        assert_eq!(r.unwrap().unwrap().shapes, Vec::new());
    }

    #[test]
    #[ignore = "mở lớp phủ thật; chạy tay: cargo test -p darkshot-windows -- --ignored tool_letters_go_into_text --test-threads=1"]
    fn tool_letters_go_into_text() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        let shot = live_setup();
        let sel = live::test_sel(&shot);
        let (a, b) = (
            Point {
                x: sel.x + 30,
                y: sel.y + 40,
            },
            Point {
                x: sel.x + 120,
                y: sel.y + 130,
            },
        );
        let mut steps = vec![live::Step::Key(u16::from(b'T'))];
        steps.extend(live::click(a));
        // Như bàn phím thật: chỉ WM_KEYDOWN 'P'; TranslateMessage của vòng lặp lớp phủ sinh
        // WM_CHAR 'p'. Phím không đổi công cụ, ký tự vào ô. Cần Caps Lock tắt (bật thì ra "P").
        steps.push(live::Step::Key(u16::from(b'P')));
        // Công cụ vẫn là Chữ: bấm chỗ khác mở ô mới (không vẽ chấm Bút vẽ).
        steps.extend(live::click(b));
        steps.extend(live::chars("q"));
        steps.push(live::Step::Ctrl(u16::from(b'S')));
        let (r, ok) = live::drive(&shot, Some(sel), steps);
        assert!(ok);
        assert_eq!(
            texts(&r.unwrap().unwrap().shapes),
            vec![
                (live::pt(a), "p".to_string()),
                (live::pt(b), "q".to_string())
            ]
        );
    }

    #[test]
    #[ignore = "mở lớp phủ thật; cần Esc toàn cục không bị ứng dụng khác giữ; chạy tay: cargo test -p darkshot-windows -- --ignored escape_hotkey_restored --test-threads=1"]
    fn escape_hotkey_restored() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        let shot = live_setup();
        let sel = live::test_sel(&shot);
        let at = Point {
            x: sel.x + 40,
            y: sel.y + 50,
        };
        // Lớp phủ giữ Esc toàn cục; nhả khi ô chữ mở; giữ lại khi ô đóng.
        let mut steps = vec![live::Step::EscFree(false), live::Step::Key(u16::from(b'T'))];
        steps.extend(live::click(at));
        steps.extend(live::chars("a"));
        steps.push(live::Step::EscFree(true));
        steps.push(live::Step::Key(VK_ESCAPE.0));
        steps.push(live::Step::EscFree(false));
        // Mở ô lần nữa rồi đóng lớp phủ giữa lúc đang gõ.
        steps.extend(live::click(at));
        steps.extend(live::chars("b"));
        steps.push(live::Step::EscFree(true));
        steps.push(live::Step::Close);
        let (r, ok) = live::drive(&shot, Some(sel), steps);
        assert!(ok);
        assert_eq!(r, Ok(None));
        // Lần mở sau: Esc toàn cục được giữ lại và đóng lớp phủ.
        let steps = vec![live::Step::EscFree(false), live::Step::Hotkey];
        let (r, ok) = live::drive(&shot, Some(sel), steps);
        assert!(ok);
        assert_eq!(r, Ok(None));
    }
}
