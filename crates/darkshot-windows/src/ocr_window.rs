//! Cửa sổ "Chữ nhận dạng": tối, không chặn, đổi cỡ được; tối đa `MAX_WINDOWS` cửa sổ. Hai khung:
//! trái là ảnh đã nhận dạng (khung dòng, bấm để chép, Ctrl+bấm để gom), phải là ô chữ, ba công tắc
//! và hàng nút. Mở ra là hiện "Đang nhận dạng chữ…" và giao ảnh cho `ocr::start` (luồng nền); kết
//! quả về bằng `WM_OCR_RESULT`. Thông điệp do vòng lặp của ứng dụng phân phối; `pre_translate` lo
//! bàn phím.
//!
//! Sở hữu kết quả: mỗi `WM_OCR_RESULT` mang một `Box` (LPARAM) được lấy lại đúng một lần, kể cả khi
//! token không khớp, khi cửa sổ đã có kết quả, hay khi cửa sổ bị hủy trước khi lấy thông điệp
//! (vét hàng đợi ở `WM_DESTROY`/`WM_NCDESTROY`, và lưới an toàn trong `pre_translate`).
//! Không bao giờ gọi `ocr::recognize` trên luồng giao diện. Clipboard chỉ bị ghi khi người dùng bấm
//! hoặc gõ phím, hoặc khi "Tự chép khi xong" đang bật lúc kết quả về.

use crate::clipboard::WinClipboard;
use crate::history_paint::Bmp;
use crate::lang;
use crate::ocr::{self, OcrError, OcrResult};
use crate::ocr_view::{self, Geom, Pick, PickInput, PickOutcome};
use crate::theme;
use crate::winutil::{self, scale};
use darkshot_core::actions::{Clipboard, NoticeKind};
use darkshot_core::geometry::Rect;
use darkshot_core::i18n::{self, ocr_status, t, Lang, Msg};
use darkshot_core::image::Image;
use darkshot_core::ocr_text::{compose_all, line_at, line_count, OcrLine, TextOptions};
use darkshot_core::theme::{ACCENT, BG, INPUT, LINE, SURFACE, TEXT, TEXT_MUTED};
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleDC, CreateFontIndirectW, DeleteDC, DeleteObject, EndPaint,
    GetDC, GetMonitorInfoW, GetTextExtentPoint32W, InvalidateRect, MonitorFromPoint,
    MonitorFromRect, ReleaseDC, ScreenToClient, SelectObject, DT_CENTER, DT_NOPREFIX,
    DT_SINGLELINE, DT_VCENTER, HDC, HFONT, HGDIOBJ, HMONITOR, MONITORINFO,
    MONITOR_DEFAULTTONEAREST, PAINTSTRUCT, SRCCOPY,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{
    SetScrollInfo, EM_SETLIMITTEXT, EM_SETREADONLY, EM_SETSEL, NMHDR, NM_CUSTOMDRAW, WM_MOUSELEAVE,
};
use windows::Win32::UI::HiDpi::{
    AdjustWindowRectExForDpi, GetDpiForMonitor, GetDpiForWindow, SystemParametersInfoForDpi,
    MDT_EFFECTIVE_DPI,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    EnableWindow, GetActiveWindow, GetFocus, GetKeyState, IsWindowEnabled, SetFocus,
    TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT, VK_CONTROL, VK_DOWN, VK_ESCAPE, VK_RETURN,
    VK_SHIFT, VK_SPACE, VK_TAB, VK_UP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetCursorPos, GetNextDlgTabItem,
    GetScrollInfo, GetWindowLongPtrW, IsChild, IsDialogMessageW, IsWindow, KillTimer, LoadCursorW,
    PeekMessageW, RegisterClassW, SendMessageW, SetCursor, SetForegroundWindow, SetTimer,
    SetWindowPos, SetWindowTextW, ShowWindow, BM_GETCHECK, BM_SETCHECK, BS_AUTOCHECKBOX,
    BS_PUSHBUTTON, DLGC_WANTARROWS, DLGC_WANTCHARS, EN_KILLFOCUS, EN_SETFOCUS, ES_AUTOVSCROLL,
    ES_MULTILINE, ES_NOHIDESEL, ES_WANTRETURN, GWL_EXSTYLE, HMENU, HTCLIENT, HWND_NOTOPMOST,
    IDC_ARROW, IDC_HAND, MINMAXINFO, MSG, NONCLIENTMETRICSW, PM_REMOVE, SB_BOTTOM, SB_LINEDOWN,
    SB_LINEUP, SB_PAGEDOWN, SB_PAGEUP, SB_THUMBPOSITION, SB_THUMBTRACK, SB_TOP, SB_VERT,
    SCROLLBAR_COMMAND, SCROLLINFO, SIF_DISABLENOSCROLL, SIF_PAGE, SIF_POS, SIF_RANGE, SIF_TRACKPOS,
    SPI_GETNONCLIENTMETRICS, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SW_SHOW,
    WA_INACTIVE, WINDOW_EX_STYLE, WINDOW_STYLE, WM_ACTIVATE, WM_APP, WM_CLOSE, WM_COMMAND,
    WM_CTLCOLOREDIT, WM_CTLCOLORSTATIC, WM_DESTROY, WM_DPICHANGED, WM_ERASEBKGND, WM_GETDLGCODE,
    WM_GETMINMAXINFO, WM_KEYDOWN, WM_KILLFOCUS, WM_LBUTTONDOWN, WM_MOUSEMOVE, WM_MOUSEWHEEL,
    WM_NCDESTROY, WM_NOTIFY, WM_PAINT, WM_SETCURSOR, WM_SETFOCUS, WM_SETFONT, WM_SIZE, WM_TIMER,
    WM_VSCROLL, WNDCLASSW, WS_CAPTION, WS_CHILD, WS_CLIPCHILDREN, WS_DISABLED, WS_EX_APPWINDOW,
    WS_EX_CONTROLPARENT, WS_EX_TOPMOST, WS_GROUP, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_SYSMENU,
    WS_TABSTOP, WS_THICKFRAME, WS_VISIBLE, WS_VSCROLL,
};

/// Kết quả nhận dạng gửi về cửa sổ: WPARAM = token, LPARAM = `Box` của `ocr::start`.
pub const WM_OCR_RESULT: u32 = WM_APP + 12;
/// Vùng client tối thiểu và mặc định (DIP).
const MIN_W: i32 = 640;
const MIN_H: i32 = 400;
const DEFAULT_W: i32 = 960;
const DEFAULT_H: i32 = 600;
/// Bề rộng tối thiểu của khung phải (DIP) và phần trăm bề rộng dành cho khung ảnh.
const RIGHT_MIN: i32 = 280;
const IMAGE_SHARE: i32 = 58;
/// Mở cửa sổ thứ chín thì đóng cái cũ nhất.
const MAX_WINDOWS: usize = 8;

const CLASS: PCWSTR = w!("darkshot-ocr");
const VIEW_CLASS: PCWSTR = w!("darkshot-ocr-view");
/// Chép tất cả mang ID của IDOK, Đóng mang ID của IDCANCEL.
const ID_COPY: i32 = 1;
const ID_CLOSE: i32 = 2;
const ID_EDIT: i32 = 100;
const ID_STATUS: i32 = 101;
const ID_VIEW: i32 = 102;
const ID_HINT: i32 = 103;
/// Ba công tắc: Gộp thành một dòng, Tách đoạn, Tự chép khi xong.
const ID_CHECKS: [i32; 3] = [110, 111, 112];
/// Hẹn giờ ẩn nhãn "Đã chép" (trên khung ảnh).
const TIP_TIMER: usize = 1;
const TIP_MS: u32 = 1200;

const STYLE: WINDOW_STYLE = WINDOW_STYLE(
    WS_CAPTION.0
        | WS_SYSMENU.0
        | WS_THICKFRAME.0
        | WS_MINIMIZEBOX.0
        | WS_MAXIMIZEBOX.0
        | WS_CLIPCHILDREN.0,
);
const EX_STYLE: WINDOW_EX_STYLE = WINDOW_EX_STYLE(WS_EX_APPWINDOW.0 | WS_EX_CONTROLPARENT.0);
/// Kiểu STATIC của dòng trạng thái: một dòng, căn giữa dọc, cắt bằng dấu ba chấm, không tiền tố &.
const SS_STATUS: WINDOW_STYLE = WINDOW_STYLE(0x0200 | 0x4000 | 0x0080);
/// Kiểu STATIC của dòng gợi ý: trái, tự xuống dòng theo từ, không tiền tố &.
const SS_HINT: WINDOW_STYLE = WINDOW_STYLE(0x0080);

// Bố cục (DIP): lề, khoảng giữa các phần, nút, dòng trạng thái, đệm của ô chữ trong khung, công
// tắc, dòng gợi ý (hai dòng chữ), đệm khung ảnh trong khung viền, đệm ảnh trong khung ảnh.
const MARGIN: i32 = 12;
const GAP: i32 = 8;
const ROW_GAP: i32 = 4;
const BTN_H: i32 = 28;
const BTN_MIN_W: i32 = 88;
const BTN_PAD: i32 = 16;
const STATUS_H: i32 = 20;
const EDIT_PAD: i32 = 6;
const CHECK_H: i32 = 24;
const CHECK_GAP: i32 = 2;
const HINT_H: i32 = 36;
const VIEW_INSET: i32 = 3;
const VIEW_PAD: i32 = 8;
/// Mỗi nấc bánh xe cuộn ảnh bao nhiêu DIP.
const WHEEL_DIP: i32 = 48;
/// Độ đục của lớp phủ dòng đang chọn (~30%).
const SELECT_ALPHA: u8 = 77;

/// Bề rộng chữ (pixel) dùng để đặt cỡ: nút Chép tất cả, nút Đóng (phông đậm), công tắc dài nhất.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Texts {
    copy: i32,
    close: i32,
    check: i32,
}

/// Bố cục hai khung.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PaneLayout {
    /// Khung tự vẽ quanh khung ảnh (viền, gạch dưới khi có tiêu điểm).
    view_field: Rect,
    /// Cửa sổ con khung ảnh, nằm trong `view_field`.
    view: Rect,
    hint: Rect,
    status: Rect,
    field: Rect,
    edit: Rect,
    checks: [Rect; 3],
    copy: Rect,
    close: Rect,
}

/// Bề rộng nút cho chữ rộng `text_px` pixel ở `dpi`: tối thiểu `BTN_MIN_W`, đủ chỗ cho chữ và đệm.
fn button_width(text_px: i32, dpi: u32) -> i32 {
    scale(BTN_MIN_W, dpi).max(text_px.max(0) + 2 * scale(BTN_PAD, dpi))
}

fn inset(r: Rect, d: i32) -> Rect {
    Rect {
        x: r.x + d,
        y: r.y + d,
        w: (r.w - 2 * d).max(0),
        h: (r.h - 2 * d).max(0),
    }
}

/// Bố cục trong vùng client `cw`×`ch` pixel: trái là khung ảnh (~58% bề rộng) và dòng gợi ý dưới
/// nó; phải (tối thiểu 280 DIP) là dòng trạng thái, ô chữ, ba công tắc và hàng nút Chép tất cả |
/// Đóng sát góc dưới phải (Chép tất cả lấy phần còn lại của hàng). Cỡ nhỏ hơn tối thiểu thì các phần
/// co về 0 chứ không âm.
fn pane_layout(cw: i32, ch: i32, dpi: u32, texts: Texts) -> PaneLayout {
    let s = |v| scale(v, dpi);
    let (m, g, bh) = (s(MARGIN), s(GAP), s(BTN_H));
    let inner = (cw - 2 * m - g).max(0);
    let mut rw = inner - inner * IMAGE_SHARE / 100;
    if rw < s(RIGHT_MIN) {
        rw = s(RIGHT_MIN).min(inner);
    }
    let lw = inner - rw;
    let rx = m + lw + g;
    let btn_y = ch - m - bh;
    let close_w = button_width(texts.close, dpi);
    let close = Rect {
        x: rx + rw - close_w,
        y: btn_y,
        w: close_w,
        h: bh,
    };
    let copy = Rect {
        x: rx,
        y: btn_y,
        w: (close.x - g - rx).max(0),
        h: bh,
    };
    let check_h = s(CHECK_H);
    let c2 = Rect {
        x: rx,
        y: btn_y - g - check_h,
        w: rw,
        h: check_h,
    };
    let c1 = Rect {
        y: c2.y - s(CHECK_GAP) - check_h,
        ..c2
    };
    let c0 = Rect {
        y: c1.y - s(CHECK_GAP) - check_h,
        ..c2
    };
    let status = Rect {
        x: rx,
        y: m,
        w: rw,
        h: s(STATUS_H),
    };
    let field_y = status.bottom() + s(ROW_GAP);
    let field = Rect {
        x: rx,
        y: field_y,
        w: rw,
        h: (c0.y - g - field_y).max(0),
    };
    let hint = Rect {
        x: m,
        y: ch - m - s(HINT_H),
        w: lw,
        h: s(HINT_H),
    };
    let view_field = Rect {
        x: m,
        y: m,
        w: lw,
        h: (hint.y - s(ROW_GAP) - m).max(0),
    };
    PaneLayout {
        view_field,
        view: inset(view_field, s(VIEW_INSET)),
        hint,
        status,
        field,
        edit: inset(field, s(EDIT_PAD)),
        checks: [c0, c1, c2],
        copy,
        close,
    }
}

/// Ba công tắc của cửa sổ (lưu trong cấu hình).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Switches {
    pub single_line: bool,
    pub paragraphs: bool,
    pub auto_copy: bool,
}

impl Switches {
    fn text_options(self) -> TextOptions {
        TextOptions {
            single_line: self.single_line,
            paragraphs: self.paragraphs && !self.single_line,
        }
    }
}

/// "Tách đoạn" dùng được không (không khi "Gộp thành một dòng" bật).
fn paragraphs_enabled(s: Switches) -> bool {
    !s.single_line
}

/// Chữ của kết quả có chữ theo `o`: ghép từ các dòng (kết quả không có dòng thì chữ thô).
fn result_text(r: &OcrResult, o: TextOptions) -> String {
    if r.lines_detail.is_empty() {
        to_crlf(&r.text)
    } else {
        compose_all(&r.lines_detail, o)
    }
}

/// Chữ cần tự chép khi kết quả `res` về với công tắc `s`; `None` = không chép.
fn auto_copy_text(s: Switches, res: &Result<OcrResult, OcrError>) -> Option<String> {
    match res {
        Ok(r) if s.auto_copy && !r.text.trim().is_empty() => Some(result_text(r, s.text_options())),
        _ => None,
    }
}

/// Kết quả mang token `got` có được nhận không: chỉ khi cửa sổ đang chờ (`expected` = `Some`) đúng
/// token đó. Sau khi nhận, cửa sổ đặt `None` nên kết quả thứ hai bị bỏ.
fn token_matches(expected: Option<usize>, got: usize) -> bool {
    expected == Some(got)
}

/// Những gì cửa sổ hiện cho một trạng thái.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct View {
    /// Chữ trong ô (dòng nối `\r\n`).
    text: String,
    status: String,
    /// Ô chữ sửa được (không chỉ đọc).
    editable: bool,
    can_copy: bool,
    /// Chữ ô màu phụ (đang nhận dạng).
    muted: bool,
}

fn busy_view(lang: Lang) -> View {
    View {
        text: t(lang, Msg::OcrBusy).to_string(),
        muted: true,
        ..View::default()
    }
}

/// Cửa sổ sau khi có kết quả; chữ ghép theo `o`. Lỗi kỹ thuật (`Failed`) không bao giờ hiện cho
/// người dùng.
fn view_of(lang: Lang, res: &Result<OcrResult, OcrError>, o: TextOptions) -> View {
    match res {
        Ok(r) if r.text.trim().is_empty() => View {
            text: t(lang, Msg::OcrEmpty).to_string(),
            status: ocr_status(lang, &lang_label(&r.lang_tag, &r.lang_name), 0),
            ..View::default()
        },
        Ok(r) => View {
            text: result_text(r, o),
            status: ocr_status(lang, &lang_label(&r.lang_tag, &r.lang_name), r.lines),
            editable: true,
            can_copy: true,
            muted: false,
        },
        Err(OcrError::NoLanguage) => View {
            text: t(lang, Msg::OcrNoLanguage).to_string(),
            ..View::default()
        },
        Err(OcrError::Failed(_)) => View {
            text: t(lang, Msg::OcrFailed).to_string(),
            ..View::default()
        },
    }
}

/// Tên ngôn ngữ cho dòng trạng thái: tên hiển thị của Windows kèm thẻ BCP-47.
fn lang_label(tag: &str, name: &str) -> String {
    match (tag.is_empty(), name.is_empty()) {
        (_, true) => tag.to_string(),
        (true, false) => name.to_string(),
        (false, false) => format!("{name}, {tag}"),
    }
}

/// Mọi kiểu xuống dòng thành `\r\n` (ô EDIT nhiều dòng chỉ hiểu `\r\n`).
fn to_crlf(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '\r' => {
                if it.peek() == Some(&'\n') {
                    it.next();
                }
                out.push_str("\r\n");
            }
            '\n' => out.push_str("\r\n"),
            c => out.push(c),
        }
    }
    out
}

/// Một cửa sổ đang mở. Không bao giờ giữ mượn sổ `WINS` qua lời gọi Win32 có thể gửi thông điệp
/// (vẽ GDI thì được).
struct Win {
    hwnd: HWND,
    /// Token của lượt nhận dạng (cố định).
    token: usize,
    /// `Some(token)` khi còn chờ kết quả; `None` sau khi đã nhận.
    waiting: Option<usize>,
    edit: HWND,
    status: HWND,
    copy: HWND,
    close: HWND,
    view: HWND,
    hint: HWND,
    checks: [HWND; 3],
    dpi: u32,
    font: Option<HFONT>,
    bold: Option<HFONT>,
    edit_font: Option<HFONT>,
    muted: bool,
    /// Tên ngôn ngữ của kết quả (cho thông báo khi chép).
    label: String,
    /// Điều khiển có tiêu điểm khi cửa sổ mất kích hoạt (trả lại khi kích hoạt lại).
    focus: HWND,
    field: Rect,
    view_field: Rect,
    /// Thứ đang hiện (để `relabel` dựng lại chữ khi đổi ngôn ngữ).
    shown: Shown,
    switches: Switches,
    /// Các dòng của kết quả (khung ở px ảnh gốc); rỗng khi chưa có kết quả hay lỗi.
    lines: Vec<OcrLine>,
    /// Cỡ ảnh gốc và bản giữ (DIB, cạnh dài tối đa `KEEP_MAX`).
    orig: (u32, u32),
    kept_size: (u32, u32),
    kept: Option<Bmp>,
    /// Bản đã co cho khung hiện tại (`None` khi hiển thị 1:1 bản giữ).
    disp: Option<Bmp>,
    geom: Geom,
    scroll: i32,
    pick: Pick,
    hover: Option<usize>,
    tracking: bool,
    /// Dòng đang hiện nhãn "Đã chép".
    tip: Option<usize>,
}

type Notify = fn(NoticeKind, &str, &str);
type Starter = fn(Image, HWND, u32, usize) -> Result<(), String>;
type LoadSwitches = fn() -> Switches;
type SaveSwitches = fn(Switches);

thread_local! {
    /// Cửa sổ đang mở, cũ nhất trước.
    static WINS: RefCell<Vec<Win>> = const { RefCell::new(Vec::new()) };
    static NEXT_TOKEN: Cell<usize> = const { Cell::new(1) };
    static NOTIFY: Cell<Option<Notify>> = const { Cell::new(None) };
    static STORE: Cell<Option<(LoadSwitches, SaveSwitches)>> = const { Cell::new(None) };
}

#[cfg(test)]
thread_local! {
    /// Số kết quả đã lấy lại quyền sở hữu (kiểm thử đếm để thấy không rò).
    static FREED: Cell<usize> = const { Cell::new(0) };
}

fn with_win<R>(hwnd: HWND, f: impl FnOnce(&mut Win) -> R) -> Option<R> {
    WINS.with(|v| {
        let mut v = v.try_borrow_mut().ok()?;
        v.iter_mut().find(|w| w.hwnd == hwnd).map(f)
    })
}

/// Cửa sổ kết quả chứa khung ảnh `view`.
fn owner_of_view(view: HWND) -> Option<HWND> {
    WINS.with(|v| {
        let v = v.try_borrow().ok()?;
        v.iter().find(|w| w.view == view).map(|w| w.hwnd)
    })
}

/// Hàm `main` đăng ký để báo Đã chép / lỗi clipboard qua khay (lọc thông báo Success ở `main`).
pub fn set_notifier(f: Notify) {
    NOTIFY.with(|n| n.set(Some(f)));
}

/// Hàm `main` đăng ký để đọc giá trị đầu của ba công tắc và lưu chúng (đường lưu cấu hình hiện có).
pub fn set_switch_store(load: LoadSwitches, save: SaveSwitches) {
    STORE.with(|s| s.set(Some((load, save))));
}

fn load_switches() -> Switches {
    STORE
        .with(|s| s.get())
        .map(|(load, _)| load())
        .unwrap_or_default()
}

fn save_switches(sw: Switches) {
    if let Some((_, save)) = STORE.with(|s| s.get()) {
        save(sw);
    }
}

fn notify(kind: NoticeKind, title: &str, body: &str) {
    if let Some(f) = NOTIFY.with(|n| n.get()) {
        f(kind, title, body);
    }
}

/// Lấy lại `Box` kết quả của một `WM_OCR_RESULT` (đúng một lần cho mỗi thông điệp).
fn consume(lp: LPARAM) -> Box<Result<OcrResult, OcrError>> {
    #[cfg(test)]
    FREED.with(|f| f.set(f.get() + 1));
    // SAFETY: chỉ gọi cho WM_OCR_RESULT vừa lấy khỏi hàng đợi (GetMessage/PeekMessage PM_REMOVE),
    // mỗi thông điệp đúng một lần; LPARAM là con trỏ Box do `ocr::start` tạo.
    unsafe { ocr::take_result(lp) }
}

/// Vét mọi kết quả còn trong hàng đợi của `hwnd` và giải phóng chúng.
fn drain(hwnd: HWND) {
    let mut m = MSG::default();
    // SAFETY: chỉ lấy WM_OCR_RESULT của cửa sổ này khỏi hàng đợi của luồng này.
    while unsafe { PeekMessageW(&mut m, Some(hwnd), WM_OCR_RESULT, WM_OCR_RESULT, PM_REMOVE) }
        .as_bool()
    {
        drop(consume(m.lParam));
    }
}

/// Mở cửa sổ kết quả cho `image` và bắt đầu nhận dạng trên luồng nền.
pub fn open(image: Image) -> Result<(), String> {
    open_avoiding(image, None)
}

/// Như `open`, nhưng đặt cửa sổ tránh khung `avoid` (tọa độ màn hình, ví dụ ảnh ghim luôn ở trên
/// cùng) trên màn hình chứa khung đó.
pub fn open_avoiding(image: Image, avoid: Option<Rect>) -> Result<(), String> {
    open_inner(image, avoid, ocr::start)
}

#[cfg(test)]
fn open_with(image: Image, start: Starter) -> Result<(), String> {
    open_inner(image, None, start)
}

/// Bản giữ để hiển thị của `image`: DIB cùng cỡ, hay bản co về cạnh dài `KEEP_MAX` (7b.7).
fn kept_bitmap(image: &Image) -> (Option<Bmp>, (u32, u32)) {
    let (w, h) = (image.width, image.height);
    let (kw, kh) = ocr_view::kept_size(w, h);
    if !image.is_valid() {
        return (None, (kw, kh));
    }
    let bmp = if (kw, kh) == (w, h) {
        ocr_view::bmp_from_bgra(w, h, &image.bgra)
    } else {
        ocr_view::bmp_from_bgra(kw, kh, &ocr::scale_bgra(&image.bgra, w, h, kw, kh))
    };
    (bmp, (kw, kh))
}

fn open_inner(image: Image, avoid: Option<Rect>, start: Starter) -> Result<(), String> {
    close_oldest_over(MAX_WINDOWS - 1);
    let lang = lang::current();
    let (kept, kept_size) = kept_bitmap(&image);
    let orig = (image.width, image.height);
    let hwnd = create(lang, avoid, kept, orig, kept_size)
        .ok_or_else(|| "CreateWindowExW failed".to_string())?;
    let token = WINS.with(|v| v.borrow().iter().find(|w| w.hwnd == hwnd).map(|w| w.token));
    let Some(token) = token else {
        destroy(hwnd);
        return Err("window not registered".into());
    };
    // Ảnh gốc chuyển cho luồng nhận dạng; cửa sổ chỉ giữ bản DIB của nó.
    if let Err(e) = start(image, hwnd, WM_OCR_RESULT, token) {
        destroy(hwnd);
        return Err(e);
    }
    Ok(())
}

/// Đóng cửa sổ cũ nhất cho tới khi còn tối đa `keep` cái.
fn close_oldest_over(keep: usize) {
    loop {
        prune();
        let oldest = WINS.with(|v| {
            let v = v.borrow();
            (v.len() > keep).then(|| v[0].hwnd)
        });
        let Some(h) = oldest else {
            return;
        };
        destroy(h);
        // Lưới an toàn: hủy không gỡ được khỏi sổ thì bỏ thẳng, để vòng lặp luôn dừng.
        WINS.with(|v| {
            if let Ok(mut v) = v.try_borrow_mut() {
                v.retain(|w| w.hwnd != h);
            }
        });
    }
}

/// Bỏ khỏi sổ các handle đã chết (WM_NCDESTROY đã gỡ trong trường hợp thường).
fn prune() {
    WINS.with(|v| {
        if let Ok(mut v) = v.try_borrow_mut() {
            // SAFETY: IsWindow chỉ hỏi tính hợp lệ của handle.
            v.retain(|w| unsafe { IsWindow(Some(w.hwnd)) }.as_bool());
        }
    });
}

fn destroy(hwnd: HWND) {
    // SAFETY: cửa sổ kết quả của luồng này; dọn dẹp ở WM_DESTROY/WM_NCDESTROY.
    unsafe {
        let _ = DestroyWindow(hwnd);
    }
}

/// Đóng mọi cửa sổ kết quả (thoát ứng dụng). Kết quả đang chờ bị bỏ và giải phóng.
pub fn close_all() {
    let all: Vec<HWND> = WINS.with(|v| v.borrow().iter().map(|w| w.hwnd).collect());
    for h in all {
        destroy(h);
    }
    prune();
}

/// Bỏ `WS_EX_TOPMOST` của mọi cửa sổ kết quả đang topmost (gọi khi ảnh ghim cuối cùng đóng: không
/// còn gì để nằm trên). Không dời, không đổi cỡ, không kích hoạt cửa sổ.
pub fn drop_topmost() {
    let all: Vec<HWND> = WINS.with(|v| {
        v.try_borrow()
            .map(|v| v.iter().map(|w| w.hwnd).collect())
            .unwrap_or_default()
    });
    for h in all {
        // SAFETY: cửa sổ kết quả của luồng này; chỉ đọc kiểu và đổi thứ tự Z, ngoài lúc mượn sổ.
        unsafe {
            if GetWindowLongPtrW(h, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 != 0 {
                let _ = SetWindowPos(
                    h,
                    Some(HWND_NOTOPMOST),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
        }
    }
}

fn key_down(vk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY) -> bool {
    // SAFETY: chỉ đọc trạng thái phím của luồng này.
    unsafe { GetKeyState(i32::from(vk.0)) < 0 }
}

/// Phím trên khung ảnh (đang có tiêu điểm) → thao tác chọn dòng.
fn view_key(vk: u16, ctrl: bool) -> Option<PickInput> {
    match vk {
        v if v == VK_UP.0 => Some(PickInput::Up),
        v if v == VK_DOWN.0 => Some(PickInput::Down),
        v if v == VK_RETURN.0 && !ctrl => Some(PickInput::Enter),
        v if v == VK_SPACE.0 => Some(PickInput::Space),
        v if v == u16::from(b'A') && ctrl => Some(PickInput::SelectAll),
        _ => None,
    }
}

/// `true`: thông điệp đã được xử lý, vòng lặp bỏ qua. Chỉ đụng thông điệp của cửa sổ kết quả (và
/// con của nó), cộng `WM_OCR_RESULT` tới handle không còn là cửa sổ kết quả (giải phóng rồi bỏ).
pub fn pre_translate(msg: &MSG) -> bool {
    if msg.message == WM_OCR_RESULT {
        // Để thủ tục cửa sổ lấy lại Box. Không bao giờ đụng LPARAM của thông điệp gửi tới cửa sổ
        // khác: thành phần khác trên luồng có thể dùng cùng mã WM_APP + 12. Kết quả tới cửa sổ đã
        // hủy được vét ở WM_DESTROY/WM_NCDESTROY; nếu luồng nền đăng đúng lúc sau lần vét cuối mà
        // handle chưa kịp chết thì một Box nhỏ bị rò (chấp nhận được, rất hiếm).
        return false;
    }
    let owner = WINS.with(|v| {
        v.try_borrow().ok().and_then(|v| {
            v.iter()
                .find(|w| winutil::belongs_to(w.hwnd, msg))
                .map(|w| (w.hwnd, w.edit, w.close, w.view))
        })
    });
    let Some((hwnd, edit, close, view)) = owner else {
        return false;
    };
    if msg.message == WM_KEYDOWN {
        let vk = msg.wParam.0 as u16;
        let ctrl = key_down(VK_CONTROL);
        // SAFETY: chỉ hỏi điều khiển có tiêu điểm của luồng này.
        let focus = unsafe { GetFocus() };
        if vk == VK_ESCAPE.0 {
            // Có vùng chọn trên ảnh thì bỏ chọn, không thì đóng.
            view_input(hwnd, PickInput::Escape);
            return true;
        }
        if msg.hwnd == view {
            if let Some(input) = view_key(vk, ctrl) {
                view_input(hwnd, input);
                return true;
            }
        }
        if vk == VK_RETURN.0 {
            if ctrl {
                copy_all(hwnd);
                return true;
            }
            if focus == edit {
                // Xuống dòng trong ô chữ.
                return false;
            }
            if focus == close {
                destroy(hwnd);
            } else {
                copy_all(hwnd);
            }
            return true;
        }
        if vk == VK_TAB.0 && !ctrl {
            let back = key_down(VK_SHIFT);
            // SAFETY: đi giữa các điều khiển có WS_TABSTOP của cửa sổ này.
            unsafe {
                if let Ok(next) = GetNextDlgTabItem(hwnd, Some(focus), back) {
                    let _ = SetFocus(Some(next));
                }
            }
            return true;
        }
        if vk == u16::from(b'A') && ctrl && focus == edit {
            // SAFETY: chọn hết chữ trong ô chữ của cửa sổ này.
            unsafe {
                SendMessageW(edit, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
            }
            return true;
        }
    }
    // SAFETY: xử lý phím điều hướng còn lại (Space trên nút, gợi ý bàn phím) của cửa sổ này.
    unsafe { IsDialogMessageW(hwnd, msg).as_bool() }
}

/// DPI hiệu dụng của màn hình `mon`.
fn monitor_dpi(mon: HMONITOR) -> u32 {
    let (mut x, mut y) = (0u32, 0u32);
    // SAFETY: các tham số ra là biến cục bộ.
    let ok = unsafe { GetDpiForMonitor(mon, MDT_EFFECTIVE_DPI, &mut x, &mut y).is_ok() };
    if ok && x > 0 {
        x
    } else {
        96
    }
}

/// Vùng làm việc của màn hình `mon` (tọa độ màn hình).
fn work_area(mon: HMONITOR) -> Option<Rect> {
    let mut mi = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: mi là biến cục bộ có cbSize đúng.
    if !unsafe { GetMonitorInfoW(mon, &mut mi) }.as_bool() {
        return None;
    }
    let a = mi.rcWork;
    Some(Rect {
        x: a.left,
        y: a.top,
        w: a.right - a.left,
        h: a.bottom - a.top,
    })
}

/// Cỡ cửa sổ (pixel) cho vùng client `w`×`h` DIP ở `dpi`.
fn window_size(w: i32, h: i32, dpi: u32) -> (i32, i32) {
    let mut rc = RECT {
        left: 0,
        top: 0,
        right: scale(w, dpi),
        bottom: scale(h, dpi),
    };
    // SAFETY: rc là biến cục bộ.
    unsafe {
        let _ = AdjustWindowRectExForDpi(&mut rc, STYLE, false, EX_STYLE, dpi);
    }
    (rc.right - rc.left, rc.bottom - rc.top)
}

/// Phông hệ thống 10 pt ở `dpi` cho ô chữ. Người gọi phải `DeleteObject`.
fn edit_font(dpi: u32) -> Option<HFONT> {
    let mut ncm = NONCLIENTMETRICSW {
        cbSize: std::mem::size_of::<NONCLIENTMETRICSW>() as u32,
        ..Default::default()
    };
    // SAFETY: ncm có cbSize đúng; hệ thống điền lfMessageFont.
    unsafe {
        SystemParametersInfoForDpi(
            SPI_GETNONCLIENTMETRICS.0,
            ncm.cbSize,
            Some(&mut ncm as *mut _ as *mut c_void),
            0,
            dpi,
        )
        .ok()?;
        let mut lf = ncm.lfMessageFont;
        lf.lfHeight = -((10 * dpi as i32 + 36) / 72);
        let f = CreateFontIndirectW(&lf);
        (!f.is_invalid()).then_some(f)
    }
}

fn delete_font(f: Option<HFONT>) {
    if let Some(f) = f {
        // SAFETY: phông do CreateFontIndirectW tạo; không điều khiển nào còn dùng.
        unsafe {
            let _ = DeleteObject(HGDIOBJ(f.0));
        }
    }
}

fn set_font(hwnd: HWND, f: Option<HFONT>) {
    if let (Some(f), false) = (f, hwnd.is_invalid()) {
        // SAFETY: phông sống tới khi cửa sổ kết quả đổi phông hoặc bị hủy.
        unsafe {
            SendMessageW(
                hwnd,
                WM_SETFONT,
                Some(WPARAM(f.0 as usize)),
                Some(LPARAM(1)),
            );
        }
    }
}

fn child(parent: HWND, class: PCWSTR, text: &str, style: WINDOW_STYLE, id: i32) -> HWND {
    // SAFETY: tạo điều khiển con của cửa sổ kết quả ở luồng chính; bố cục đặt sau.
    unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class,
            &HSTRING::from(text),
            WS_CHILD | WS_VISIBLE | style,
            0,
            0,
            0,
            0,
            Some(parent),
            Some(HMENU(id as usize as *mut c_void)),
            None,
            None,
        )
        .unwrap_or_default()
    }
}

fn check_msgs() -> [Msg; 3] {
    [Msg::OcrSingleLine, Msg::OcrParagraphs, Msg::OcrAutoCopy]
}

fn is_checked(h: HWND) -> bool {
    // SAFETY: chỉ hỏi trạng thái của ô tích thuộc cửa sổ này.
    unsafe { SendMessageW(h, BM_GETCHECK, None, None).0 == 1 }
}

fn set_checked(h: HWND, on: bool) {
    // SAFETY: đặt trạng thái của ô tích thuộc cửa sổ này.
    unsafe {
        SendMessageW(h, BM_SETCHECK, Some(WPARAM(usize::from(on))), None);
    }
}

/// Tạo, đăng ký vào sổ và hiện cửa sổ ở trạng thái "đang nhận dạng". Lỗi thì không còn gì sót
/// (bản giữ `kept` được giải phóng cùng cửa sổ hoặc ngay khi không tạo được).
fn create(
    lang: Lang,
    avoid: Option<Rect>,
    kept: Option<Bmp>,
    orig: (u32, u32),
    kept_size: (u32, u32),
) -> Option<HWND> {
    // Màn hình chứa khung cần tránh, không có thì màn hình có con trỏ.
    // SAFETY: tham số ra là biến cục bộ; MonitorFromRect/Point chỉ đọc.
    let mon = unsafe {
        match avoid {
            Some(a) => MonitorFromRect(&rect_of(a), MONITOR_DEFAULTTONEAREST),
            None => {
                let mut pt = POINT::default();
                let _ = GetCursorPos(&mut pt);
                MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST)
            }
        }
    };
    // Tạo thẳng trên màn hình đó, theo DPI của nó: không có WM_DPICHANGED lúc mở.
    let dpi = monitor_dpi(mon);
    let (ww, wh) = window_size(DEFAULT_W, DEFAULT_H, dpi);
    let (x, y, covered) = match work_area(mon) {
        Some(work) => place_avoiding(work, ww, wh, avoid, scale(GAP, dpi)),
        None => (100, 100, avoid.is_some()),
    };
    let title = HSTRING::from(t(lang, Msg::OcrWindowTitle));
    // SAFETY: đăng ký lớp và tạo cửa sổ ở luồng chính; đăng ký lại lần hai báo lỗi vô hại.
    let hwnd = unsafe {
        let hinst = GetModuleHandleW(None).ok()?;
        let arrow = LoadCursorW(None, IDC_ARROW).unwrap_or_default();
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: hinst.into(),
            lpszClassName: CLASS,
            hbrBackground: theme::brush(BG),
            hCursor: arrow,
            ..Default::default()
        };
        RegisterClassW(&wc);
        let vc = WNDCLASSW {
            lpfnWndProc: Some(view_proc),
            hInstance: hinst.into(),
            lpszClassName: VIEW_CLASS,
            hCursor: arrow,
            ..Default::default()
        };
        RegisterClassW(&vc);
        CreateWindowExW(
            ex_style(covered),
            CLASS,
            &title,
            STYLE,
            x,
            y,
            ww,
            wh,
            None,
            None,
            Some(hinst.into()),
            None,
        )
        .ok()?
    };
    // SAFETY: hwnd vừa tạo ở luồng này.
    let dpi = match unsafe { GetDpiForWindow(hwnd) } {
        0 => dpi,
        d => d,
    };
    let font = winutil::system_font(dpi);
    let bold = winutil::system_font_styled(dpi, 100, true);
    let efont = edit_font(dpi);
    // Thứ tự tạo = thứ tự Tab: khung ảnh → ô chữ → ba công tắc → Chép tất cả → Đóng.
    let tab = WINDOW_STYLE(WS_TABSTOP.0 | WS_GROUP.0);
    let view = child(hwnd, VIEW_CLASS, "", tab | WS_VSCROLL, ID_VIEW);
    let edit_style = WINDOW_STYLE(
        tab.0
            | WS_VSCROLL.0
            | ES_MULTILINE as u32
            | ES_AUTOVSCROLL as u32
            | ES_WANTRETURN as u32
            | ES_NOHIDESEL as u32,
    );
    let edit = child(hwnd, w!("EDIT"), "", edit_style, ID_EDIT);
    let msgs = check_msgs();
    let checks: [HWND; 3] = std::array::from_fn(|i| {
        child(
            hwnd,
            w!("BUTTON"),
            t(lang, msgs[i]),
            tab | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
            ID_CHECKS[i],
        )
    });
    let push = WINDOW_STYLE(tab.0 | BS_PUSHBUTTON as u32);
    let copy = child(
        hwnd,
        w!("BUTTON"),
        t(lang, Msg::OcrCopyAll),
        push | WS_DISABLED,
        ID_COPY,
    );
    let close = child(hwnd, w!("BUTTON"), t(lang, Msg::OcrClose), push, ID_CLOSE);
    let status = child(hwnd, w!("STATIC"), "", SS_STATUS, ID_STATUS);
    let hint = child(
        hwnd,
        w!("STATIC"),
        t(lang, Msg::OcrPickHint),
        SS_HINT,
        ID_HINT,
    );
    set_font(edit, efont.or(font));
    for h in [status, hint, copy, close].into_iter().chain(checks) {
        set_font(h, font);
    }
    let switches = load_switches();
    let token = NEXT_TOKEN.with(|n| {
        let v = n.get();
        n.set(v.wrapping_add(1).max(1));
        v
    });
    let zero = Rect {
        x: 0,
        y: 0,
        w: 0,
        h: 0,
    };
    WINS.with(|v| {
        v.borrow_mut().push(Win {
            hwnd,
            token,
            waiting: Some(token),
            edit,
            status,
            copy,
            close,
            view,
            hint,
            checks,
            dpi,
            font,
            bold,
            edit_font: efont,
            muted: true,
            label: String::new(),
            focus: HWND::default(),
            shown: Shown::Busy,
            field: zero,
            view_field: zero,
            switches,
            lines: Vec::new(),
            orig,
            kept_size,
            kept,
            disp: None,
            geom: ocr_view::geom(orig, kept_size, 0, 0, 0),
            scroll: 0,
            pick: Pick::default(),
            hover: None,
            tracking: false,
            tip: None,
        })
    });
    let all = [edit, status, copy, close, view, hint];
    if all.iter().chain(checks.iter()).any(|h| h.is_invalid()) {
        // WM_NCDESTROY gỡ khỏi sổ, xóa phông và bitmap.
        destroy(hwnd);
        return None;
    }
    theme::set_dark_control_theme(edit, theme::DarkClass::Explorer);
    theme::set_dark_control_theme(view, theme::DarkClass::Explorer);
    // SAFETY: ô chữ của cửa sổ này; 0 = giới hạn tối đa (chữ dài tùy ý).
    unsafe {
        SendMessageW(edit, EM_SETLIMITTEXT, Some(WPARAM(0)), None);
    }
    for (h, on) in checks.iter().zip([
        switches.single_line,
        switches.paragraphs,
        switches.auto_copy,
    ]) {
        set_checked(*h, on);
    }
    // SAFETY: ô tích của cửa sổ này.
    unsafe {
        let _ = EnableWindow(checks[1], paragraphs_enabled(switches));
    }
    // Thanh cuộn luôn hiện (mờ khi không cần) để bề rộng khung ảnh không đổi theo nội dung.
    update_scrollbar(hwnd);
    apply_view(hwnd, &busy_view(lang));
    relayout(hwnd);
    theme::apply_dark_title_bar(hwnd);
    winutil::set_app_icons(hwnd);
    // SAFETY: hwnd là cửa sổ của luồng này.
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
        let _ = SetFocus(Some(close));
    }
    Some(hwnd)
}

/// Đưa `view` lên cửa sổ: chữ ô, chỉ đọc hay không, dòng trạng thái, nút Chép tất cả.
fn apply_view(hwnd: HWND, view: &View) {
    let Some((edit, status, copy, close)) = with_win(hwnd, |w| {
        w.muted = view.muted;
        (w.edit, w.status, w.copy, w.close)
    }) else {
        return;
    };
    // SAFETY: điều khiển con của cửa sổ này; chuỗi sống suốt lời gọi.
    unsafe {
        let _ = SetWindowTextW(edit, &HSTRING::from(view.text.as_str()));
        SendMessageW(
            edit,
            EM_SETREADONLY,
            Some(WPARAM(usize::from(!view.editable))),
            None,
        );
        let _ = SetWindowTextW(status, &HSTRING::from(view.status.as_str()));
        if !view.can_copy && GetFocus() == copy {
            let _ = SetFocus(Some(close));
        }
        let _ = EnableWindow(copy, view.can_copy);
        let _ = InvalidateRect(Some(edit), None, true);
    }
}

/// Nhận một `WM_OCR_RESULT`: lấy lại Box (luôn), dùng chỉ khi token khớp và còn chờ. Khung dòng
/// tính lại theo hình học hiện tại của khung ảnh mỗi lần vẽ hay dò, nên kết quả đến sau khi đổi cỡ,
/// đổi DPI hay đang cuộn vẫn khớp chữ.
fn on_result(hwnd: HWND, wp: WPARAM, lp: LPARAM) {
    let res = consume(lp);
    let accepted = with_win(hwnd, |w| {
        if token_matches(w.waiting, wp.0) {
            w.waiting = None;
            if let Ok(r) = res.as_ref() {
                w.label = lang_label(&r.lang_tag, &r.lang_name);
                w.lines = r.lines_detail.clone();
            }
            w.shown = shown_of(&res);
            w.pick = Pick::default();
            w.hover = None;
            w.tip = None;
            Some((w.switches, w.view))
        } else {
            None
        }
    })
    .flatten();
    let Some((switches, view_hwnd)) = accepted else {
        return;
    };
    let view = view_of(lang::current(), &res, switches.text_options());
    let auto = auto_copy_text(switches, &res);
    drop(res);
    apply_view(hwnd, &view);
    invalidate(view_hwnd);
    if view.editable {
        if let Some(edit) = with_win(hwnd, |w| w.edit) {
            // SAFETY: ô chữ của cửa sổ này; chỉ chuyển tiêu điểm trong cửa sổ đang kích hoạt.
            unsafe {
                SendMessageW(edit, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
                if GetActiveWindow() == hwnd {
                    let _ = SetFocus(Some(edit));
                } else {
                    with_win(hwnd, |w| w.focus = edit);
                }
            }
        }
    }
    // Tự chép theo công tắc lúc kết quả về (7b.6); báo như nút Chép tất cả, cửa sổ ở lại.
    if let Some(text) = auto {
        copy_and_notify(hwnd, &text);
    }
}

/// Chép `text` và báo qua khay như nút Chép tất cả (Đã chép chữ / lỗi). Cửa sổ ở lại.
fn copy_and_notify(hwnd: HWND, text: &str) {
    let label = with_win(hwnd, |w| w.label.clone()).unwrap_or_default();
    let lang = lang::current();
    match WinClipboard.set_text(text) {
        Ok(()) => notify(
            NoticeKind::Success,
            t(lang, Msg::OcrCopiedTitle),
            &ocr_status(lang, &label, line_count(text)),
        ),
        Err(e) => notify(
            NoticeKind::Error,
            t(lang, Msg::OcrCopyFailedTitle),
            &i18n::ocr_copy_failed_body(lang, &e),
        ),
    }
}

/// Chép chữ hiện có trong ô (kể cả đã sửa); cửa sổ ở lại để người dùng còn chọn tiếp trên ảnh.
fn copy_all(hwnd: HWND) {
    let Some((edit, copy)) = with_win(hwnd, |w| (w.edit, w.copy)) else {
        return;
    };
    // SAFETY: chỉ hỏi trạng thái bật của nút.
    if !unsafe { IsWindowEnabled(copy) }.as_bool() {
        return;
    }
    let text = String::from_utf16_lossy(&theme::window_text(edit));
    copy_and_notify(hwnd, &text);
}

/// Chép chữ của các dòng vừa bấm; thành công thì hiện "Đã chép" cạnh dòng `at` 1,2 s, lỗi thì báo.
fn copy_lines(hwnd: HWND, text: &str, at: usize) {
    let lang = lang::current();
    match WinClipboard.set_text(text) {
        Ok(()) => {
            if let Some(view) = with_win(hwnd, |w| {
                w.tip = Some(at);
                w.view
            }) {
                // SAFETY: hẹn giờ của khung ảnh thuộc cửa sổ này (đặt lại nếu đang chạy); hủy cùng
                // cửa sổ.
                unsafe {
                    SetTimer(Some(view), TIP_TIMER, TIP_MS, None);
                }
            }
        }
        Err(e) => notify(
            NoticeKind::Error,
            t(lang, Msg::OcrCopyFailedTitle),
            &i18n::ocr_copy_failed_body(lang, &e),
        ),
    }
}

/// Áp một thao tác chọn dòng (chuột hay phím) lên khung ảnh của `hwnd`.
fn view_input(hwnd: HWND, input: PickInput) {
    let Some((out, view, cursor_rect)) = with_win(hwnd, |w| {
        let before = w.pick.cursor;
        let out = ocr_view::pick(&mut w.pick, &w.lines, w.switches.text_options(), input);
        // Phím ↑/↓: cuộn cho dòng đang trỏ hiện ra.
        let moved = w.pick.cursor != before && matches!(input, PickInput::Up | PickInput::Down);
        let rect = w
            .pick
            .cursor
            .filter(|_| moved)
            .and_then(|i| w.lines.get(i))
            .map(|l| w.geom.line_rect(l.rect));
        (out, w.view, rect)
    }) else {
        return;
    };
    if let Some(r) = cursor_rect {
        let target = with_win(hwnd, |w| {
            w.geom.scroll_to_show(w.scroll, r, scale(VIEW_PAD, w.dpi))
        });
        if let Some(t) = target {
            scroll_to(hwnd, t);
        }
    }
    match out {
        PickOutcome::Copy { text, at } => copy_lines(hwnd, &text, at),
        PickOutcome::Close => {
            destroy(hwnd);
            return;
        }
        PickOutcome::Nothing => {}
    }
    invalidate(view);
}

/// Dòng dưới điểm (tọa độ client của khung ảnh) của cửa sổ `hwnd`.
fn hit_line(hwnd: HWND, x: i32, y: i32) -> Option<usize> {
    with_win(hwnd, |w| {
        let (px, py) = w.geom.to_image(x, y + w.scroll)?;
        line_at(&w.lines, px, py)
    })
    .flatten()
}

/// Đổi một công tắc: đọc lại ba ô tích, lưu cấu hình, dựng lại ô chữ khi đã có chữ. Không chép lại
/// vùng chọn đang có (clipboard không bị ghi khi người dùng chưa bấm khung).
fn on_switch(hwnd: HWND) {
    let Some(checks) = with_win(hwnd, |w| w.checks) else {
        return;
    };
    let sw = Switches {
        single_line: is_checked(checks[0]),
        paragraphs: is_checked(checks[1]),
        auto_copy: is_checked(checks[2]),
    };
    let Some((changed, edit, text)) = with_win(hwnd, |w| {
        let changed = w.switches != sw;
        w.switches = sw;
        let text = (matches!(w.shown, Shown::Text { .. }) && !w.lines.is_empty())
            .then(|| compose_all(&w.lines, sw.text_options()));
        (changed, w.edit, text)
    }) else {
        return;
    };
    // SAFETY: ô tích "Tách đoạn" của cửa sổ này.
    unsafe {
        let _ = EnableWindow(checks[1], paragraphs_enabled(sw));
    }
    if !changed {
        return;
    }
    save_switches(sw);
    if let Some(text) = text {
        // SAFETY: ô chữ của cửa sổ này; chuỗi sống suốt lời gọi. Sửa tay trong ô bị thay (như thiết kế).
        unsafe {
            let _ = SetWindowTextW(edit, &HSTRING::from(text.as_str()));
        }
    }
}

/// Bề rộng chữ `text` với phông `font` (pixel).
fn text_width(hwnd: HWND, font: Option<HFONT>, text: &str) -> i32 {
    let s: Vec<u16> = text.encode_utf16().collect();
    let mut sz = SIZE::default();
    // SAFETY: DC lấy và trả trong khối; phông cũ được chọn lại trước khi trả DC.
    unsafe {
        let dc = GetDC(Some(hwnd));
        if dc.is_invalid() {
            return 0;
        }
        let old = font.map(|f| SelectObject(dc, HGDIOBJ(f.0)));
        let _ = GetTextExtentPoint32W(dc, &s, &mut sz);
        if let Some(o) = old {
            SelectObject(dc, o);
        }
        ReleaseDC(Some(hwnd), dc);
    }
    sz.cx
}

fn rect_of(r: Rect) -> RECT {
    RECT {
        left: r.x,
        top: r.y,
        right: r.right(),
        bottom: r.bottom(),
    }
}

fn invalidate(h: HWND) {
    if !h.is_invalid() {
        // SAFETY: chỉ đánh dấu vẽ lại cửa sổ của luồng này.
        unsafe {
            let _ = InvalidateRect(Some(h), None, false);
        }
    }
}

/// Tính lại bố cục theo vùng client hiện tại và DPI của cửa sổ.
fn relayout(hwnd: HWND) {
    let Some((dpi, bold, font, edit, status, copy, close, view, hint, checks)) =
        with_win(hwnd, |w| {
            (
                w.dpi, w.bold, w.font, w.edit, w.status, w.copy, w.close, w.view, w.hint, w.checks,
            )
        })
    else {
        return;
    };
    let mut rc = RECT::default();
    // SAFETY: rc là biến cục bộ; hwnd là cửa sổ của luồng này.
    unsafe {
        let _ = GetClientRect(hwnd, &mut rc);
    }
    let lang = lang::current();
    let f = bold.or(font);
    let texts = Texts {
        copy: text_width(hwnd, f, t(lang, Msg::OcrCopyAll)),
        close: text_width(hwnd, f, t(lang, Msg::OcrClose)),
        check: check_msgs()
            .iter()
            .map(|m| text_width(hwnd, font, t(lang, *m)))
            .max()
            .unwrap_or(0),
    };
    let l = pane_layout(rc.right - rc.left, rc.bottom - rc.top, dpi, texts);
    with_win(hwnd, |w| {
        w.field = l.field;
        w.view_field = l.view_field;
    });
    let placed = [
        (view, l.view),
        (edit, l.edit),
        (status, l.status),
        (hint, l.hint),
        (copy, l.copy),
        (close, l.close),
        (checks[0], l.checks[0]),
        (checks[1], l.checks[1]),
        (checks[2], l.checks[2]),
    ];
    for (h, r) in placed {
        // SAFETY: điều khiển con của cửa sổ này.
        unsafe {
            let _ = SetWindowPos(h, None, r.x, r.y, r.w, r.h, SWP_NOZORDER | SWP_NOACTIVATE);
        }
    }
    refresh_view(hwnd);
    // SAFETY: vẽ lại cả cửa sổ (khung ô chữ đổi chỗ).
    unsafe {
        let _ = InvalidateRect(Some(hwnd), None, true);
    }
}

/// Tính lại hình học khung ảnh theo cỡ client hiện tại của nó; dựng lại bản đã co khi cỡ hiển thị
/// đổi (không co lại ở mỗi `WM_PAINT`); kẹp vị trí cuộn; cập nhật thanh cuộn.
fn refresh_view(hwnd: HWND) {
    let Some(view) = with_win(hwnd, |w| w.view) else {
        return;
    };
    let mut rc = RECT::default();
    // SAFETY: rc là biến cục bộ; khung ảnh là con của cửa sổ này.
    unsafe {
        let _ = GetClientRect(view, &mut rc);
    }
    with_win(hwnd, |w| {
        let g = ocr_view::geom(
            w.orig,
            w.kept_size,
            rc.right - rc.left,
            rc.bottom - rc.top,
            scale(VIEW_PAD, w.dpi),
        );
        let (dw, dh) = (g.img.w, g.img.h);
        let kept_px = (w.kept_size.0 as i32, w.kept_size.1 as i32);
        let disp_ok = match &w.disp {
            Some(d) => (d.w, d.h) == (dw, dh),
            None => (dw, dh) == kept_px,
        };
        if !disp_ok {
            // Bỏ bản cũ trước khi tạo bản mới (chỉ một bản đã co sống cùng bản giữ).
            w.disp = None;
            if (dw, dh) != kept_px {
                w.disp = w.kept.as_ref().and_then(|k| ocr_view::scaled(k, dw, dh));
            }
        }
        w.geom = g;
        w.scroll = w.scroll.clamp(0, g.max_scroll());
    });
    update_scrollbar(hwnd);
    invalidate(view);
}

fn update_scrollbar(hwnd: HWND) {
    let Some((view, content, page, pos)) = with_win(hwnd, |w| {
        (w.view, w.geom.content_h, w.geom.view_h, w.scroll)
    }) else {
        return;
    };
    let si = SCROLLINFO {
        cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
        fMask: SIF_RANGE | SIF_PAGE | SIF_POS | SIF_DISABLENOSCROLL,
        nMin: 0,
        nMax: (content - 1).max(0),
        nPage: page.max(0) as u32,
        nPos: pos,
        nTrackPos: 0,
    };
    // SAFETY: si là biến cục bộ đủ cbSize; khung ảnh là cửa sổ con của luồng này.
    unsafe {
        SetScrollInfo(view, SB_VERT, &si, true);
    }
}

/// Cuộn khung ảnh tới `target` pixel (đã kẹp).
fn scroll_to(hwnd: HWND, target: i32) {
    let Some((changed, view)) = with_win(hwnd, |w| {
        let t = target.clamp(0, w.geom.max_scroll());
        let changed = t != w.scroll;
        w.scroll = t;
        (changed, w.view)
    }) else {
        return;
    };
    if changed {
        update_scrollbar(hwnd);
        invalidate(view);
    }
}

/// Đổi DPI: phông mới cho mọi điều khiển, xóa phông cũ sau khi không ai dùng.
fn on_dpi_changed(hwnd: HWND, dpi: u32) {
    let font = winutil::system_font(dpi);
    let bold = winutil::system_font_styled(dpi, 100, true);
    let efont = edit_font(dpi);
    let Some((old, ctrls)) = with_win(hwnd, |w| {
        w.dpi = dpi;
        let old = [
            std::mem::replace(&mut w.font, font),
            std::mem::replace(&mut w.bold, bold),
            std::mem::replace(&mut w.edit_font, efont),
        ];
        (old, (w.edit, [w.status, w.hint, w.copy, w.close], w.checks))
    }) else {
        for f in [font, bold, efont] {
            delete_font(f);
        }
        return;
    };
    let (edit, others, checks) = ctrls;
    set_font(edit, efont.or(font));
    for h in others.into_iter().chain(checks) {
        set_font(h, font);
    }
    for f in old {
        delete_font(f);
    }
}

fn paint(hwnd: HWND) {
    let mut ps = PAINTSTRUCT::default();
    // SAFETY: BeginPaint/EndPaint cân bằng trong khối.
    unsafe {
        let hdc = BeginPaint(hwnd, &mut ps);
        if !hdc.is_invalid() {
            if let Some((field, view_field, dpi, edit, view)) =
                with_win(hwnd, |w| (w.field, w.view_field, w.dpi, w.edit, w.view))
            {
                let focus = GetFocus();
                for (f, ctl) in [(field, edit), (view_field, view)] {
                    if f.w > 0 && f.h > 0 {
                        let rc = rect_of(f);
                        theme::buffered(hdc, &rc, |m| {
                            theme::fill(m, &rc, BG);
                            theme::draw_field(m, &rc, dpi, theme::edit_border(focus == ctl, true));
                        });
                    }
                }
            }
        }
        let _ = EndPaint(hwnd, &ps);
    }
}

/// Chép phần nhìn thấy được của `bmp` (cỡ `img`, đặt tại `img.x`, `img.y - scroll`) lên `dc`.
fn blit_visible(dc: HDC, bmp: &Bmp, img: Rect, scroll: i32, client: &RECT) {
    let top = img.y - scroll;
    let y0 = top.max(client.top);
    let y1 = (top + img.h).min(client.bottom);
    let x0 = img.x.max(client.left);
    let x1 = (img.x + img.w).min(client.right);
    if y1 <= y0 || x1 <= x0 {
        return;
    }
    if (bmp.w, bmp.h) != (img.w, img.h) {
        // Không có bản đã co (thiếu bộ nhớ): co thẳng từ bản giữ.
        crate::history_paint::blit(
            dc,
            bmp,
            &RECT {
                left: img.x,
                top,
                right: img.x + img.w,
                bottom: top + img.h,
            },
        );
        return;
    }
    // SAFETY: DC tạm tạo ở đây được trả bitmap cũ và xóa trên mọi đường đi; bitmap thuộc `bmp`.
    unsafe {
        let mem = CreateCompatibleDC(Some(dc));
        if mem.is_invalid() {
            return;
        }
        let old = SelectObject(mem, HGDIOBJ(bmp.handle.0));
        let _ = BitBlt(
            dc,
            x0,
            y0,
            x1 - x0,
            y1 - y0,
            Some(mem),
            x0 - img.x,
            y0 - top,
            SRCCOPY,
        );
        SelectObject(mem, old);
        let _ = DeleteDC(mem);
    }
}

fn inflate(r: &RECT, d: i32) -> RECT {
    RECT {
        left: r.left - d,
        top: r.top - d,
        right: r.right + d,
        bottom: r.bottom + d,
    }
}

/// Vẽ khung ảnh: nền `INPUT`, ảnh, khung dòng (thường 1 px, rê chuột 2 px, chọn: phủ `ACCENT` ~30%
/// và 2 px), viền nét đứt quanh dòng đang trỏ khi có tiêu điểm, nhãn "Đã chép".
fn paint_view(view: HWND) {
    let mut ps = PAINTSTRUCT::default();
    // SAFETY: BeginPaint/EndPaint cân bằng trong khối; hỏi tiêu điểm và client của chính khung ảnh.
    unsafe {
        let hdc = BeginPaint(view, &mut ps);
        if !hdc.is_invalid() {
            let mut client = RECT::default();
            let _ = GetClientRect(view, &mut client);
            let focused = GetFocus() == view;
            let tip_text: Vec<u16> = lang::tr(Msg::OcrCopiedTip).encode_utf16().collect();
            if let Some(hwnd) = owner_of_view(view) {
                // Chỉ vẽ GDI trong lúc mượn sổ (không gửi thông điệp).
                with_win(hwnd, |w| {
                    theme::buffered(hdc, &client, |dc| {
                        draw_view(dc, &client, w, focused, &tip_text);
                    });
                });
            }
        }
        let _ = EndPaint(view, &ps);
    }
}

fn draw_view(dc: HDC, client: &RECT, w: &Win, focused: bool, tip_text: &[u16]) {
    theme::fill(dc, client, INPUT);
    let g = w.geom;
    if let Some(bmp) = w.disp.as_ref().or(w.kept.as_ref()) {
        blit_visible(dc, bmp, g.img, w.scroll, client);
    }
    let s = |v| scale(v, w.dpi);
    let visible = |r: &RECT| r.bottom > client.top && r.top < client.bottom;
    let shown = |i: usize| -> Option<RECT> {
        let l = w.lines.get(i)?;
        // Khung suy biến (mọi từ có khung rỗng): không vẽ.
        if l.rect.w <= 0 || l.rect.h <= 0 {
            return None;
        }
        let d = g.line_rect(l.rect);
        let r = RECT {
            left: d.x,
            top: d.y - w.scroll,
            right: d.right(),
            bottom: d.bottom() - w.scroll,
        };
        Some(inflate(&r, 1))
    };
    for i in 0..w.lines.len() {
        let Some(r) = shown(i).filter(|r| visible(r)) else {
            continue;
        };
        let selected = w.pick.is_selected(i);
        if selected {
            ocr_view::tint(dc, &r, ACCENT, SELECT_ALPHA);
        }
        let thick = if selected || w.hover == Some(i) { 2 } else { 1 };
        ocr_view::frame(dc, &r, ACCENT, s(thick).max(thick));
    }
    if focused {
        if let Some(r) = w.pick.cursor.and_then(&shown) {
            ocr_view::dotted(dc, &inflate(&r, s(2)), TEXT);
        }
    }
    if let Some(r) = w.tip.and_then(shown) {
        let m = theme::draw_text(
            dc,
            tip_text,
            &RECT::default(),
            TEXT,
            w.font,
            DT_SINGLELINE | DT_NOPREFIX | windows::Win32::Graphics::Gdi::DT_CALCRECT,
        );
        let (tw, th) = (m.right - m.left + 2 * s(6), m.bottom - m.top + 2 * s(2));
        let gap = s(4);
        let mut y = r.top - gap - th;
        if y < client.top {
            y = r.bottom + gap;
        }
        let x = r.left.min(client.right - tw).max(client.left);
        let rc = RECT {
            left: x,
            top: y,
            right: x + tw,
            bottom: y + th,
        };
        theme::round_rect(dc, &rc, s(theme::RADIUS), SURFACE, LINE, 1);
        theme::draw_text(
            dc,
            tip_text,
            &rc,
            TEXT,
            w.font,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
    }
}

fn lo_word(v: usize) -> u32 {
    (v & 0xFFFF) as u32
}

/// Tọa độ client (có dấu) trong LPARAM của thông điệp chuột.
fn point_of(lp: LPARAM) -> (i32, i32) {
    (
        i32::from((lp.0 & 0xFFFF) as u16 as i16),
        i32::from(((lp.0 >> 16) & 0xFFFF) as u16 as i16),
    )
}

/// Điều khiển nhận tiêu điểm khi cửa sổ được kích hoạt: cái đã nhớ, không thì ô chữ khi đã có chữ
/// để sửa, còn lại nút Đóng.
fn restore_focus(hwnd: HWND) {
    let Some((saved, edit, copy, close)) = with_win(hwnd, |w| (w.focus, w.edit, w.copy, w.close))
    else {
        return;
    };
    // SAFETY: chỉ hỏi quan hệ/trạng thái điều khiển của cửa sổ này rồi đặt tiêu điểm.
    unsafe {
        let target = if !saved.is_invalid()
            && IsChild(hwnd, saved).as_bool()
            && IsWindowEnabled(saved).as_bool()
        {
            saved
        } else if IsWindowEnabled(copy).as_bool() {
            edit
        } else {
            close
        };
        let _ = SetFocus(Some(target));
    }
}

/// Thủ tục của khung ảnh (cửa sổ con tự vẽ, có thanh cuộn dọc).
unsafe extern "system" fn view_proc(view: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let owner = owner_of_view(view);
    match (msg, owner) {
        (WM_PAINT, _) => {
            paint_view(view);
            LRESULT(0)
        }
        (WM_ERASEBKGND, _) => LRESULT(1),
        (WM_GETDLGCODE, _) => LRESULT((DLGC_WANTARROWS | DLGC_WANTCHARS) as isize),
        (WM_SETFOCUS | WM_KILLFOCUS, Some(hwnd)) => {
            // Viền nét đứt của dòng đang trỏ và gạch dưới của khung đổi theo tiêu điểm.
            invalidate(view);
            if let Some(f) = with_win(hwnd, |w| w.view_field) {
                let rc = rect_of(f);
                // SAFETY: vẽ lại vùng khung của cửa sổ cha.
                unsafe {
                    let _ = InvalidateRect(Some(hwnd), Some(&rc), false);
                }
            }
            LRESULT(0)
        }
        (WM_LBUTTONDOWN, Some(hwnd)) => {
            // SAFETY: đưa tiêu điểm vào khung ảnh (không giữ mượn sổ).
            unsafe {
                let _ = SetFocus(Some(view));
            }
            let (x, y) = point_of(lp);
            let hit = hit_line(hwnd, x, y);
            // Ctrl lấy từ cờ của chính thông điệp (MK_CONTROL), không đọc bàn phím.
            let input = if wp.0 & 0x0008 != 0 {
                PickInput::CtrlClick(hit)
            } else {
                PickInput::Click(hit)
            };
            view_input(hwnd, input);
            LRESULT(0)
        }
        (WM_MOUSEMOVE, Some(hwnd)) => {
            let (x, y) = point_of(lp);
            let hit = hit_line(hwnd, x, y);
            let change = with_win(hwnd, |w| {
                let old = std::mem::replace(&mut w.hover, hit);
                let track = !w.tracking;
                w.tracking = true;
                (old != hit, track)
            });
            if let Some((changed, track)) = change {
                if track {
                    let mut tme = TRACKMOUSEEVENT {
                        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE,
                        hwndTrack: view,
                        dwHoverTime: 0,
                    };
                    // SAFETY: tme là biến cục bộ đủ cbSize; theo dõi chính khung ảnh.
                    unsafe {
                        let _ = TrackMouseEvent(&mut tme);
                    }
                }
                if changed {
                    invalidate(view);
                }
            }
            LRESULT(0)
        }
        (WM_MOUSELEAVE, Some(hwnd)) => {
            let had = with_win(hwnd, |w| {
                w.tracking = false;
                w.hover.take().is_some()
            });
            if had == Some(true) {
                invalidate(view);
            }
            LRESULT(0)
        }
        (WM_SETCURSOR, Some(hwnd)) if lo_word(lp.0 as usize) == HTCLIENT => {
            let mut pt = POINT::default();
            // SAFETY: pt là biến cục bộ; đổi sang tọa độ client của chính khung ảnh.
            unsafe {
                let _ = GetCursorPos(&mut pt);
                let _ = ScreenToClient(view, &mut pt);
            }
            if hit_line(hwnd, pt.x, pt.y).is_some() {
                // SAFETY: con trỏ hệ thống dùng chung, không phải hủy.
                unsafe {
                    if let Ok(c) = LoadCursorW(None, IDC_HAND) {
                        SetCursor(Some(c));
                    }
                }
                return LRESULT(1);
            }
            // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
            unsafe { DefWindowProcW(view, msg, wp, lp) }
        }
        (WM_MOUSEWHEEL, Some(hwnd)) => {
            let delta = i32::from(((wp.0 >> 16) & 0xFFFF) as u16 as i16);
            if let Some((scroll, step)) = with_win(hwnd, |w| (w.scroll, scale(WHEEL_DIP, w.dpi))) {
                scroll_to(hwnd, scroll - delta * step / 120);
            }
            LRESULT(0)
        }
        (WM_VSCROLL, Some(hwnd)) => {
            let code = SCROLLBAR_COMMAND(lo_word(wp.0) as i32);
            let Some((scroll, line, page, max)) = with_win(hwnd, |w| {
                let line = scale(WHEEL_DIP / 2, w.dpi);
                (
                    w.scroll,
                    line,
                    (w.geom.view_h - line).max(1),
                    w.geom.max_scroll(),
                )
            }) else {
                return LRESULT(0);
            };
            let target = match code {
                SB_LINEUP => scroll - line,
                SB_LINEDOWN => scroll + line,
                SB_PAGEUP => scroll - page,
                SB_PAGEDOWN => scroll + page,
                SB_TOP => 0,
                SB_BOTTOM => max,
                SB_THUMBTRACK | SB_THUMBPOSITION => {
                    let mut si = SCROLLINFO {
                        cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
                        fMask: SIF_TRACKPOS,
                        ..Default::default()
                    };
                    // SAFETY: si là biến cục bộ đủ cbSize; hỏi thanh cuộn của chính khung ảnh.
                    unsafe {
                        let _ = GetScrollInfo(view, SB_VERT, &mut si);
                    }
                    si.nTrackPos
                }
                _ => scroll,
            };
            scroll_to(hwnd, target);
            LRESULT(0)
        }
        (WM_TIMER, Some(hwnd)) if wp.0 == TIP_TIMER => {
            // SAFETY: hẹn giờ của chính khung ảnh.
            unsafe {
                let _ = KillTimer(Some(view), TIP_TIMER);
            }
            with_win(hwnd, |w| w.tip = None);
            invalidate(view);
            LRESULT(0)
        }
        // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
        _ => unsafe { DefWindowProcW(view, msg, wp, lp) },
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_OCR_RESULT => {
            on_result(hwnd, wp, lp);
            LRESULT(0)
        }
        WM_COMMAND => {
            let (id, code) = ((wp.0 & 0xFFFF) as i32, ((wp.0 >> 16) & 0xFFFF) as u32);
            match (id, code) {
                (ID_COPY, 0) => copy_all(hwnd),
                (ID_CLOSE, 0) => destroy(hwnd),
                (id, 0) if ID_CHECKS.contains(&id) => on_switch(hwnd),
                (ID_EDIT, c) if c == EN_SETFOCUS || c == EN_KILLFOCUS => {
                    // Gạch dưới của khung đổi theo tiêu điểm.
                    if let Some(f) = with_win(hwnd, |w| w.field) {
                        let rc = rect_of(f);
                        // SAFETY: vẽ lại vùng khung của cửa sổ này.
                        unsafe {
                            let _ = InvalidateRect(Some(hwnd), Some(&rc), false);
                        }
                    }
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            destroy(hwnd);
            LRESULT(0)
        }
        WM_CTLCOLOREDIT | WM_CTLCOLORSTATIC => {
            let ctl = HWND(lp.0 as *mut c_void);
            let hdc = HDC(wp.0 as *mut c_void);
            // Ô chữ (kể cả khi chỉ đọc, gửi WM_CTLCOLORSTATIC): nền INPUT; trạng thái, gợi ý: nền BG.
            match with_win(hwnd, |w| (w.edit == ctl, w.muted)) {
                Some((true, muted)) => {
                    theme::ctl_color(hdc, if muted { TEXT_MUTED } else { TEXT }, INPUT)
                }
                _ => theme::ctl_color(hdc, TEXT_MUTED, BG),
            }
        }
        WM_NOTIFY => {
            // SAFETY: lp của WM_NOTIFY trỏ tới NMHDR hợp lệ suốt lời gọi.
            let hdr = unsafe { (lp.0 as *const NMHDR).as_ref() };
            let kind = match hdr.filter(|h| h.code == NM_CUSTOMDRAW).map(|h| h.idFrom) {
                Some(id) if id == ID_COPY as usize => Some(theme::ButtonKind::Primary),
                Some(id) if id == ID_CLOSE as usize => Some(theme::ButtonKind::Normal),
                // Ô tích: `custom_draw_button` nhận ra kiểu BS_AUTOCHECKBOX và vẽ như Tùy chọn.
                Some(id) if ID_CHECKS.iter().any(|c| *c as usize == id) => {
                    Some(theme::ButtonKind::Normal)
                }
                _ => None,
            };
            if let Some(kind) = kind {
                // SAFETY: mã NM_CUSTOMDRAW từ nút nên lp trỏ tới NMCUSTOMDRAW.
                if let Some(cd) = unsafe { theme::nmcd_of(lp) } {
                    let (dpi, bold) = with_win(hwnd, |w| (w.dpi, w.bold)).unwrap_or((96, None));
                    return theme::custom_draw_button(cd, kind, BG, dpi, bold);
                }
            }
            // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
            unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
        }
        WM_PAINT => {
            paint(hwnd);
            LRESULT(0)
        }
        WM_SIZE => {
            relayout(hwnd);
            LRESULT(0)
        }
        WM_GETMINMAXINFO => {
            // SAFETY: hwnd là cửa sổ này (có thể đang được tạo); chỉ hỏi DPI.
            let dpi = match unsafe { GetDpiForWindow(hwnd) } {
                0 => 96,
                d => d,
            };
            let (mw, mh) = window_size(MIN_W, MIN_H, dpi);
            // SAFETY: với WM_GETMINMAXINFO, lp trỏ tới MINMAXINFO hợp lệ suốt lời gọi.
            if let Some(mmi) = unsafe { (lp.0 as *mut MINMAXINFO).as_mut() } {
                mmi.ptMinTrackSize.x = mw;
                mmi.ptMinTrackSize.y = mh;
            }
            LRESULT(0)
        }
        WM_DPICHANGED => {
            let dpi = match lo_word(wp.0) {
                0 => 96,
                d => d,
            };
            on_dpi_changed(hwnd, dpi);
            // Tài liệu WM_DPICHANGED: dùng đúng RECT đề nghị trong lp.
            // SAFETY: lp trỏ tới RECT hợp lệ suốt lời gọi.
            if let Some(r) = unsafe { (lp.0 as *const RECT).as_ref() } {
                // SAFETY: đổi vị trí và cỡ chính cửa sổ này (WM_SIZE tính lại bố cục).
                unsafe {
                    let _ = SetWindowPos(
                        hwnd,
                        None,
                        r.left,
                        r.top,
                        r.right - r.left,
                        r.bottom - r.top,
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                }
            }
            relayout(hwnd);
            LRESULT(0)
        }
        WM_ACTIVATE => {
            if lo_word(wp.0) == WA_INACTIVE {
                // SAFETY: chỉ hỏi tiêu điểm và quan hệ cửa sổ của luồng này.
                let f = unsafe { GetFocus() };
                if !f.is_invalid() && unsafe { IsChild(hwnd, f) }.as_bool() {
                    with_win(hwnd, |w| w.focus = f);
                }
            } else {
                restore_focus(hwnd);
            }
            LRESULT(0)
        }
        WM_SETFOCUS => {
            restore_focus(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            // Kết quả đã đăng mà chưa lấy: giải phóng trước khi handle chết.
            drain(hwnd);
            LRESULT(0)
        }
        WM_NCDESTROY => {
            drain(hwnd);
            let removed = WINS.with(|v| {
                let mut v = v.try_borrow_mut().ok()?;
                let i = v.iter().position(|w| w.hwnd == hwnd)?;
                Some(v.remove(i))
            });
            if let Some(w) = removed {
                // Mọi điều khiển đã bị hủy: phông không còn ai dùng. Bitmap tự xóa khi `w` bị bỏ.
                for f in [w.font, w.bold, w.edit_font] {
                    delete_font(f);
                }
            }
            // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
            unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
        }
        // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
        _ => unsafe { DefWindowProcW(hwnd, msg, wp, lp) },
    }
}

/// Kiểu mở rộng của cửa sổ kết quả: chồng lên khung cần tránh (ảnh ghim, luôn ở trên cùng) thì
/// thêm `WS_EX_TOPMOST` để cửa sổ nằm trên ảnh ghim; không thì như thường.
fn ex_style(overlaps: bool) -> WINDOW_EX_STYLE {
    if overlaps {
        WINDOW_EX_STYLE(EX_STYLE.0 | WS_EX_TOPMOST.0)
    } else {
        EX_STYLE
    }
}

/// Góc trên trái cho cửa sổ `w`×`h` trong vùng làm việc `work` (tọa độ màn hình), tránh khung
/// `avoid` (ảnh ghim, luôn ở trên cùng): thử bên phải, rồi trái, dưới, trên khung, cách khung `gap`;
/// không chỗ nào đủ (hay không có khung) thì giữa vùng làm việc. Luôn kẹp góc vào vùng làm việc.
/// Phần tử thứ ba (`overlaps`): cửa sổ ở vị trí trả về chồng lên `avoid`.
fn place_avoiding(work: Rect, w: i32, h: i32, avoid: Option<Rect>, gap: i32) -> (i32, i32, bool) {
    let clamp = |v: i32, lo: i32, hi: i32| v.min(hi).max(lo);
    let center = (
        (work.x + (work.w - w) / 2).max(work.x),
        (work.y + (work.h - h) / 2).max(work.y),
    );
    let Some(a) = avoid else {
        return (center.0, center.1, false);
    };
    // Phép so trên i64: khung ảnh ghim có thể ở tọa độ rất lớn.
    let hit = |x: i32, y: i32| {
        let (x, y, w, h) = (x as i64, y as i64, w as i64, h as i64);
        let (ax, ay, aw, ah) = (a.x as i64, a.y as i64, a.w as i64, a.h as i64);
        x < ax + aw && ax < x + w && y < ay + ah && ay < y + h
    };
    let (fits_w, fits_h) = (w <= work.w, h <= work.h);
    let y_beside = clamp(a.y, work.y, work.bottom() - h);
    let x_beside = clamp(a.x, work.x, work.right() - w);
    if fits_w && fits_h {
        let right = a.right() + gap;
        if right >= work.x && right + w <= work.right() {
            return (right, y_beside, false);
        }
        let left = a.x - gap - w;
        if left >= work.x && left + w <= work.right() {
            return (left, y_beside, false);
        }
        let below = a.bottom() + gap;
        if below >= work.y && below + h <= work.bottom() {
            return (x_beside, below, false);
        }
        let above = a.y - gap - h;
        if above >= work.y && above + h <= work.bottom() {
            return (x_beside, above, false);
        }
    }
    (center.0, center.1, hit(center.0, center.1))
}

/// Thứ cửa sổ đang hiện, đủ để dựng lại chữ giao diện khi đổi ngôn ngữ.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Shown {
    Busy,
    /// Chữ nhận dạng (người dùng có thể đang sửa): ngôn ngữ OCR và số dòng nhận được.
    Text {
        label: String,
        lines: usize,
    },
    Empty {
        label: String,
    },
    NoLanguage,
    Failed,
}

fn shown_of(res: &Result<OcrResult, OcrError>) -> Shown {
    match res {
        Ok(r) if r.text.trim().is_empty() => Shown::Empty {
            label: lang_label(&r.lang_tag, &r.lang_name),
        },
        Ok(r) => Shown::Text {
            label: lang_label(&r.lang_tag, &r.lang_name),
            lines: r.lines,
        },
        Err(OcrError::NoLanguage) => Shown::NoLanguage,
        Err(OcrError::Failed(_)) => Shown::Failed,
    }
}

/// Chữ ô (chỉ khi ô đang hiện câu thông báo chỉ đọc; `None` = giữ chữ người dùng) và dòng trạng thái
/// của `shown` ở ngôn ngữ `lang`. Khớp `busy_view`/`view_of`.
fn relabel_parts(lang: Lang, shown: &Shown) -> (Option<String>, String) {
    let msg = |m| Some(t(lang, m).to_string());
    match shown {
        Shown::Busy => (msg(Msg::OcrBusy), String::new()),
        Shown::Text { label, lines } => (None, ocr_status(lang, label, *lines)),
        Shown::Empty { label } => (msg(Msg::OcrEmpty), ocr_status(lang, label, 0)),
        Shown::NoLanguage => (msg(Msg::OcrNoLanguage), String::new()),
        Shown::Failed => (msg(Msg::OcrFailed), String::new()),
    }
}

/// Đổi ngôn ngữ giao diện khi cửa sổ đang mở: tiêu đề, chữ nút, dòng trạng thái, câu thông báo
/// trong ô chỉ đọc, rồi tính lại bố cục. Không đụng chữ người dùng đang sửa.
pub fn relabel() {
    let lang = lang::current();
    type Row = (HWND, HWND, HWND, HWND, HWND, HWND, [HWND; 3], Shown);
    let all: Vec<Row> = WINS.with(|v| {
        v.borrow()
            .iter()
            .map(|w| {
                (
                    w.hwnd,
                    w.edit,
                    w.status,
                    w.copy,
                    w.close,
                    w.hint,
                    w.checks,
                    w.shown.clone(),
                )
            })
            .collect()
    });
    for (hwnd, edit, status, copy, close, hint, checks, shown) in all {
        let (text, line) = relabel_parts(lang, &shown);
        let set = |h: HWND, s: &str| {
            // SAFETY: cửa sổ kết quả hoặc điều khiển con của nó; chuỗi sống suốt lời gọi.
            unsafe {
                let _ = SetWindowTextW(h, &HSTRING::from(s));
            }
        };
        set(hwnd, t(lang, Msg::OcrWindowTitle));
        set(copy, t(lang, Msg::OcrCopyAll));
        set(close, t(lang, Msg::OcrClose));
        set(hint, t(lang, Msg::OcrPickHint));
        for (h, m) in checks.iter().zip(check_msgs()) {
            set(*h, t(lang, m));
        }
        set(status, &line);
        if let Some(text) = text {
            set(edit, &text);
        }
        relayout(hwnd);
    }
}

#[cfg(test)]
fn test_windows() -> Vec<(HWND, usize)> {
    WINS.with(|v| v.borrow().iter().map(|w| (w.hwnd, w.token)).collect())
}

#[cfg(test)]
fn busy(hwnd: HWND) -> bool {
    with_win(hwnd, |w| w.waiting.is_some()).unwrap_or(false)
}

#[cfg(test)]
fn edit_of(hwnd: HWND) -> HWND {
    with_win(hwnd, |w| w.edit).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::winutil::scale;
    use darkshot_core::i18n::{ocr_status, t, Msg};
    use std::time::{Duration, Instant};
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::System::Threading::{
        GetCurrentProcess, GetGuiResources, GR_GDIOBJECTS, GR_USEROBJECTS,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetWindowTextLengthW, GetWindowTextW, IsWindow, PeekMessageW,
        PostMessageW, TranslateMessage, PM_NOREMOVE, PM_REMOVE,
    };

    const LANGS: [Lang; 2] = [Lang::Vi, Lang::En];
    const DPIS: [u32; 5] = [96, 120, 144, 168, 192];

    /// Bề rộng chữ bi quan (mọi ký tự rộng 10 DIP, phông đậm) để kiểm thử không cắt nút.
    fn text_px(s: &str, dpi: u32) -> i32 {
        s.chars().count() as i32 * scale(10, dpi)
    }

    fn overlaps(a: Rect, b: Rect) -> bool {
        a.x < b.right() && b.x < a.right() && a.y < b.bottom() && b.y < a.bottom()
    }

    fn within(inner: Rect, outer: Rect) -> bool {
        inner.x >= outer.x
            && inner.y >= outer.y
            && inner.right() <= outer.right()
            && inner.bottom() <= outer.bottom()
    }

    #[test]
    fn sizes_match_design() {
        assert_eq!((MIN_W, MIN_H), (640, 400));
        assert_eq!((DEFAULT_W, DEFAULT_H), (960, 600));
        assert_eq!(MAX_WINDOWS, 8);
        assert_eq!((RIGHT_MIN, IMAGE_SHARE), (280, 58));
    }

    #[test]
    fn button_width_fits_label() {
        for dpi in DPIS {
            assert!(button_width(0, dpi) >= scale(80, dpi));
            for px in [0, 30, 80, 200] {
                let w = button_width(px, dpi);
                assert!(w >= px + 2 * scale(12, dpi), "dpi {dpi} px {px}: {w}");
            }
        }
    }

    /// Bề rộng chữ phông thường bi quan (8 DIP mỗi ký tự) cho ô tích và dòng gợi ý.
    fn regular_px(s: &str, dpi: u32) -> i32 {
        s.chars().count() as i32 * scale(8, dpi)
    }

    /// Ở cỡ tối thiểu, mặc định và lớn, mọi DPI 96–192 và hai ngôn ngữ: khung ảnh bên trái (~58%),
    /// dòng gợi ý dưới nó; bên phải trạng thái, ô chữ, ba công tắc, hàng nút; mọi phần nằm trong vùng
    /// client, không chồng nhau, không cắt chữ; khung phải ≥ 280 DIP.
    #[test]
    fn pane_layout_fits_without_overlap() {
        for lang in LANGS {
            for dpi in DPIS {
                let texts = Texts {
                    copy: text_px(t(lang, Msg::OcrCopyAll), dpi),
                    close: text_px(t(lang, Msg::OcrClose), dpi),
                    check: [Msg::OcrSingleLine, Msg::OcrParagraphs, Msg::OcrAutoCopy]
                        .iter()
                        .map(|m| regular_px(t(lang, *m), dpi))
                        .max()
                        .unwrap_or(0),
                };
                for (w, h) in [(MIN_W, MIN_H), (DEFAULT_W, DEFAULT_H), (1600, 1000)] {
                    let (cw, ch) = (scale(w, dpi), scale(h, dpi));
                    let client = Rect {
                        x: 0,
                        y: 0,
                        w: cw,
                        h: ch,
                    };
                    let l = pane_layout(cw, ch, dpi, texts);
                    let ctx = format!("{lang:?} dpi {dpi} {w}x{h}: {l:?}");
                    let parts = [
                        l.view_field,
                        l.hint,
                        l.status,
                        l.field,
                        l.checks[0],
                        l.checks[1],
                        l.checks[2],
                        l.copy,
                        l.close,
                    ];
                    for (i, a) in parts.iter().enumerate() {
                        assert!(a.w > 0 && a.h > 0, "{ctx}");
                        assert!(within(*a, client), "{ctx}");
                        for b in &parts[i + 1..] {
                            assert!(!overlaps(*a, *b), "{ctx}");
                        }
                    }
                    assert!(
                        within(l.view, l.view_field) && l.view != l.view_field,
                        "{ctx}"
                    );
                    assert!(within(l.edit, l.field) && l.edit != l.field, "{ctx}");
                    // Khung ảnh bên trái, khung phải bên phải.
                    let right_x = l.status.x;
                    assert!(l.view_field.right() < right_x, "{ctx}");
                    for p in [l.field, l.checks[0], l.checks[2], l.copy] {
                        assert_eq!(p.x, right_x, "{ctx}");
                    }
                    let right_w = l.close.right() - right_x;
                    assert!(right_w >= scale(RIGHT_MIN, dpi), "{ctx}");
                    assert_eq!(l.close.right(), cw - scale(12, dpi), "{ctx}");
                    let inner = cw - 2 * scale(12, dpi) - scale(8, dpi);
                    if right_w > scale(RIGHT_MIN, dpi) {
                        let share = l.view_field.w * 100 / inner;
                        assert!((57..=58).contains(&share), "{ctx}: {share}%");
                    }
                    // Gợi ý dưới khung ảnh, đủ hai dòng cho chữ dài nhất.
                    assert!(l.hint.y >= l.view_field.bottom(), "{ctx}");
                    assert_eq!(l.hint.x, l.view_field.x, "{ctx}");
                    assert!(l.hint.h >= 2 * scale(16, dpi), "{ctx}");
                    let hint = regular_px(t(lang, Msg::OcrPickHint), dpi);
                    assert!(hint <= 2 * l.hint.w - scale(60, dpi), "{ctx}: hint {hint}");
                    // Thứ tự dọc bên phải: trạng thái, ô chữ, ba công tắc, hàng nút.
                    assert!(l.status.bottom() <= l.field.y, "{ctx}");
                    assert!(l.field.bottom() <= l.checks[0].y, "{ctx}");
                    assert!(l.checks[0].bottom() <= l.checks[1].y, "{ctx}");
                    assert!(l.checks[1].bottom() <= l.checks[2].y, "{ctx}");
                    assert!(l.checks[2].bottom() <= l.copy.y, "{ctx}");
                    assert!(l.status.h >= scale(16, dpi), "{ctx}");
                    assert!(l.edit.h >= scale(100, dpi), "{ctx}");
                    assert!(l.view.h >= scale(200, dpi), "{ctx}");
                    for c in l.checks {
                        assert!(c.h >= scale(20, dpi), "{ctx}");
                        assert!(c.w >= scale(18 + 8, dpi) + texts.check, "{ctx}");
                    }
                    // Chép tất cả bên trái Đóng, cùng hàng; cả hai đủ rộng cho chữ.
                    assert!(l.copy.right() <= l.close.x, "{ctx}");
                    assert_eq!(l.copy.y, l.close.y, "{ctx}");
                    assert!(l.copy.w >= texts.copy + 2 * scale(12, dpi), "{ctx}");
                    assert!(l.close.w >= texts.close + 2 * scale(12, dpi), "{ctx}");
                }
            }
        }
    }

    /// Chỉ nhận kết quả mang đúng token của cửa sổ, và chỉ một lần (sau đó token là `None`).
    #[test]
    fn token_mismatch_rejected() {
        assert!(token_matches(Some(5), 5));
        assert!(!token_matches(Some(5), 4));
        assert!(!token_matches(Some(5), 6));
        assert!(!token_matches(None, 5));
        assert!(!token_matches(None, 0));
    }

    #[test]
    fn lang_label_has_name_and_tag() {
        assert_eq!(
            lang_label("en-US", "English (United States)"),
            "English (United States), en-US"
        );
        assert_eq!(lang_label("vi", ""), "vi");
        assert_eq!(lang_label("", "Tiếng Việt"), "Tiếng Việt");
    }

    fn r(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    /// Màn hình bên trái (tọa độ âm): phải, rồi trái, dưới, trên khung cần tránh; không chỗ nào đủ
    /// thì giữa vùng làm việc; luôn kẹp trong vùng làm việc.
    #[test]
    fn placement_avoids_rect() {
        let work = r(-1920, 0, 1920, 1040);
        let (w, h, g) = (600, 400, 8);
        let place = |avoid| {
            let (x, y, _) = place_avoiding(work, w, h, avoid, g);
            (x, y)
        };
        let center = (-1920 + (1920 - 600) / 2, (1040 - 400) / 2);
        assert_eq!(place(None), center);
        // Đủ chỗ bên phải.
        assert_eq!(place(Some(r(-1800, 100, 400, 300))), (-1392, 100));
        // Sát mép phải: sang trái.
        assert_eq!(place(Some(r(-700, 100, 600, 300))), (-1308, 100));
        // Chiếm cả bề ngang ở trên: xuống dưới, kẹp mép trái.
        assert_eq!(place(Some(r(-1920, 0, 1920, 300))), (-1920, 308));
        // Chiếm cả bề ngang ở dưới: lên trên.
        assert_eq!(place(Some(r(-1920, 700, 1920, 340))), (-1920, 292));
        // Che gần hết màn hình: giữa như cũ.
        assert_eq!(place(Some(r(-1900, 20, 1880, 1000))), center);
        // Khung sát mép dưới: bên phải, y kẹp để cửa sổ không tràn khỏi vùng làm việc.
        assert_eq!(place(Some(r(-1800, 900, 200, 100))), (-1592, 640));
        // Khung tràn ra ngoài vùng làm việc (sát mép trái trên): vẫn bên phải, trong vùng.
        assert_eq!(place(Some(r(-2000, -50, 300, 200))), (-1692, 0));
        // Cửa sổ lớn hơn vùng làm việc: góc trên trái của vùng.
        assert_eq!(place_avoiding(work, 2000, 1200, None, g), (-1920, 0, false));
        assert_eq!(
            place_avoiding(work, 2000, 1200, Some(r(-1800, 100, 100, 100)), g),
            (-1920, 0, true)
        );
        // Kết quả nằm trong vùng làm việc và không chồng khung cần tránh.
        for avoid in [
            r(-1900, 500, 50, 50),
            r(-60, 1000, 60, 40),
            r(-1920, 0, 10, 10),
        ] {
            let (x, y) = place(Some(avoid));
            assert!(x >= work.x && x + w <= work.right(), "{avoid:?}");
            assert!(y >= work.y && y + h <= work.bottom(), "{avoid:?}");
            assert!(!overlaps(r(x, y, w, h), avoid), "{avoid:?}");
        }
    }

    /// Cờ `overlaps`: có chỗ không chồng → false; không chỗ → true (vị trí giữa chồng lên khung);
    /// không có khung → false. Chỉ khi chồng thì cửa sổ là topmost.
    #[test]
    fn placement_reports_overlap_and_topmost() {
        let work = r(-1920, 0, 1920, 1040);
        let (w, h, g) = (600, 400, 8);
        let flag = |avoid| place_avoiding(work, w, h, avoid, g).2;
        assert!(!flag(None));
        assert!(!flag(Some(r(-1800, 100, 400, 300))));
        assert!(!flag(Some(r(-700, 100, 600, 300))));
        assert!(!flag(Some(r(-1920, 0, 1920, 300))));
        assert!(!flag(Some(r(-1920, 700, 1920, 340))));
        // Che gần hết hoặc phủ kín vùng làm việc: không chỗ, chồng.
        assert!(flag(Some(r(-1900, 20, 1880, 1000))));
        assert!(flag(Some(work)));
        assert!(flag(Some(r(-5000, -5000, 10000, 10000))));
        // Ngay cả giữa vùng cũng không chồng (cửa sổ lớn hơn vùng, khung nằm ngoài cửa sổ).
        assert!(!place_avoiding(work, 2000, 1200, Some(r(500, 2000, 10, 10)), g).2);

        assert_eq!(ex_style(false), EX_STYLE);
        assert_eq!(
            ex_style(true),
            WINDOW_EX_STYLE(EX_STYLE.0 | WS_EX_TOPMOST.0)
        );
    }

    /// `WM_OCR_RESULT` gửi tới cửa sổ không phải cửa sổ kết quả: `pre_translate` không đụng LPARAM.
    #[test]
    fn pre_translate_ignores_foreign_result_message() {
        // Tạo đối tượng GDI: không chen vào kiểm thử đếm GDI của cả tiến trình.
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        use windows::core::w;
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, HWND_MESSAGE, WINDOW_EX_STYLE, WINDOW_STYLE,
        };
        // SAFETY: cửa sổ chỉ-thông-điệp tạm của kiểm thử, hủy ở cuối.
        let h = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                w!("darkshot-ocr-foreign"),
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                None,
                None,
            )
        }
        .expect("message-only window");
        FREED.with(|f| f.set(0));
        post_fake(h, 1, "x");
        let mut m = MSG::default();
        // SAFETY: lấy thông điệp vừa gửi của cửa sổ do luồng này tạo.
        let got = unsafe { PeekMessageW(&mut m, Some(h), WM_OCR_RESULT, WM_OCR_RESULT, PM_REMOVE) };
        assert!(got.as_bool());
        let handled = pre_translate(&m);
        let freed = FREED.with(|f| f.get());
        if freed == 0 {
            // SAFETY: Box do `post_fake` tạo, chưa ai lấy lại.
            drop(unsafe { ocr::take_result(m.lParam) });
        }
        // SAFETY: hủy cửa sổ tạm của kiểm thử.
        unsafe {
            let _ = DestroyWindow(h);
        }
        assert!(!handled);
        assert_eq!(freed, 0);
    }

    /// Dòng nhận dạng tổng hợp: chữ và khung (px ảnh gốc).
    fn ocr_line(text: &str, x: i32, y: i32, w: i32, h: i32) -> OcrLine {
        OcrLine {
            text: text.into(),
            rect: Rect { x, y, w, h },
        }
    }

    /// Kết quả có chữ ghép từ `lines` như `ocr.rs` (chữ = `compose_all` mặc định).
    fn ok_lines(lines: Vec<OcrLine>) -> Result<OcrResult, OcrError> {
        Ok(OcrResult {
            text: compose_all(&lines, TextOptions::default()),
            lang_tag: "en-US".into(),
            lang_name: "English".into(),
            lines: lines.len(),
            lines_detail: lines,
        })
    }

    fn results() -> Vec<Result<OcrResult, OcrError>> {
        vec![
            ok_lines(vec![
                ocr_line("Hello", 10, 10, 80, 20),
                ocr_line("world", 10, 34, 80, 20),
            ]),
            ok_lines(Vec::new()),
            Err(OcrError::NoLanguage),
            Err(OcrError::Failed("0x80004005".into())),
        ]
    }

    /// Đổi ngôn ngữ: chữ giao diện dựng lại khớp `view_of`/`busy_view` ở ngôn ngữ mới; với kết quả có
    /// chữ thì không trả chữ ô (giữ nguyên chữ người dùng đang sửa).
    #[test]
    fn relabel_parts_match_views() {
        for lang in LANGS {
            let (text, status) = relabel_parts(lang, &Shown::Busy);
            let busy = busy_view(lang);
            assert_eq!(
                (text.as_deref(), status.as_str()),
                (Some(busy.text.as_str()), "")
            );
            for res in results() {
                let shown = shown_of(&res);
                let v = view_of(lang, &res, TextOptions::default());
                let (text, status) = relabel_parts(lang, &shown);
                assert_eq!(status, v.status, "{lang:?} {shown:?}");
                if v.editable {
                    assert_eq!(text, None, "{lang:?} {shown:?}");
                } else {
                    assert_eq!(text, Some(v.text.clone()), "{lang:?} {shown:?}");
                }
            }
        }
        let [ok, empty, nolang, failed]: [Result<OcrResult, OcrError>; 4] =
            results().try_into().unwrap();
        assert_eq!(
            shown_of(&ok),
            Shown::Text {
                label: "English, en-US".into(),
                lines: 2
            }
        );
        assert_eq!(
            shown_of(&empty),
            Shown::Empty {
                label: "English, en-US".into()
            }
        );
        assert_eq!(shown_of(&nolang), Shown::NoLanguage);
        assert_eq!(shown_of(&failed), Shown::Failed);
    }

    fn title_of(h: HWND) -> String {
        String::from_utf16_lossy(&crate::theme::window_text(h))
    }

    /// Đổi ngôn ngữ khi cửa sổ đang mở: tiêu đề, nút, dòng trạng thái theo ngôn ngữ mới; chữ người
    /// dùng đã sửa trong ô giữ nguyên. Cửa sổ còn bận: câu "đang nhận dạng" đổi ngôn ngữ.
    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_relabel_keeps_user_text() {
        use windows::Win32::UI::WindowsAndMessaging::{GetDlgItem, SetWindowTextW};
        close_all();
        let before = crate::lang::current();
        crate::lang::set(Lang::Vi);
        open_with(tiny(), no_ocr).expect("open");
        open_with(tiny(), no_ocr).expect("open");
        let wins = test_windows();
        let (h, token) = wins[0];
        let busy_win = wins[1].0;
        post_fake(h, token, "Hello");
        assert!(pump(2_000, || !busy(h)));
        let edit = edit_of(h);
        // Người dùng sửa chữ.
        // SAFETY: ô chữ của cửa sổ do luồng này tạo; chuỗi sống suốt lời gọi.
        unsafe {
            let _ = SetWindowTextW(edit, &windows::core::HSTRING::from("Hello edited"));
        }
        assert_eq!(title_of(h), t(Lang::Vi, Msg::OcrWindowTitle));
        crate::lang::set(Lang::En);
        relabel();
        pump(100, || false);
        assert_eq!(title_of(h), t(Lang::En, Msg::OcrWindowTitle));
        assert_eq!(text_of(edit), "Hello edited");
        // SAFETY: điều khiển con của cửa sổ này theo ID.
        let (copy, close, status) = unsafe {
            (
                GetDlgItem(Some(h), ID_COPY).unwrap_or_default(),
                GetDlgItem(Some(h), ID_CLOSE).unwrap_or_default(),
                GetDlgItem(Some(h), ID_STATUS).unwrap_or_default(),
            )
        };
        assert_eq!(title_of(copy), t(Lang::En, Msg::OcrCopyAll));
        assert_eq!(title_of(close), t(Lang::En, Msg::OcrClose));
        assert_eq!(title_of(status), ocr_status(Lang::En, "English, en-US", 1));
        assert_eq!(text_of(edit_of(busy_win)), t(Lang::En, Msg::OcrBusy));
        assert_eq!(title_of(busy_win), t(Lang::En, Msg::OcrWindowTitle));
        close_all();
        crate::lang::set(before);
    }

    #[test]
    fn crlf_normalized() {
        assert_eq!(to_crlf("a\nb\r\nc\rd"), "a\r\nb\r\nc\r\nd");
        assert_eq!(to_crlf(""), "");
        assert_eq!(to_crlf("x"), "x");
    }

    #[test]
    fn views_for_each_result() {
        let plain = TextOptions::default();
        for lang in LANGS {
            let busy = busy_view(lang);
            assert_eq!(busy.text, t(lang, Msg::OcrBusy));
            assert!(!busy.editable && !busy.can_copy && busy.muted);
            assert_eq!(busy.status, "");

            let ok = ok_lines(vec![
                ocr_line("Hello", 10, 10, 80, 20),
                ocr_line(" world ", 10, 34, 80, 20),
            ]);
            let v = view_of(lang, &ok, plain);
            assert_eq!(v.text, "Hello\r\nworld");
            assert!(v.editable && v.can_copy && !v.muted);
            assert_eq!(v.status, ocr_status(lang, "English, en-US", 2));
            // Công tắc áp dụng cho ô chữ.
            let one = TextOptions {
                single_line: true,
                paragraphs: false,
            };
            assert_eq!(view_of(lang, &ok, one).text, "Hello world");
            let gap = ok_lines(vec![
                ocr_line("a", 10, 10, 80, 20),
                ocr_line("b", 10, 34, 80, 20),
                ocr_line("c", 10, 120, 80, 20),
            ]);
            let para = TextOptions {
                single_line: false,
                paragraphs: true,
            };
            assert_eq!(view_of(lang, &gap, para).text, "a\r\nb\r\n\r\nc");
            assert_eq!(view_of(lang, &gap, plain).text, "a\r\nb\r\nc");

            let empty = ok_lines(Vec::new());
            let v = view_of(lang, &empty, plain);
            assert_eq!(v.text, t(lang, Msg::OcrEmpty));
            assert!(!v.editable && !v.can_copy);
            assert_eq!(v.status, ocr_status(lang, "English, en-US", 0));

            let v = view_of(lang, &Err(OcrError::NoLanguage), one);
            assert_eq!(v.text, t(lang, Msg::OcrNoLanguage));
            assert!(!v.editable && !v.can_copy);
            assert_eq!(v.status, "");

            let v = view_of(
                lang,
                &Err(OcrError::Failed("RecognizeAsync: 0x80004005".into())),
                plain,
            );
            assert_eq!(v.text, t(lang, Msg::OcrFailed));
            assert!(!v.text.contains("0x8") && !v.status.contains("0x8"));
            assert!(!v.editable && !v.can_copy);
        }
    }

    /// Ba công tắc → cách ghép chữ: "Tách đoạn" mất tác dụng (và bị vô hiệu) khi "Gộp một dòng" bật.
    #[test]
    fn switches_map_to_text_options() {
        let s = |single_line, paragraphs, auto_copy| Switches {
            single_line,
            paragraphs,
            auto_copy,
        };
        assert_eq!(
            s(false, false, false).text_options(),
            TextOptions::default()
        );
        assert_eq!(
            s(false, true, false).text_options(),
            TextOptions {
                single_line: false,
                paragraphs: true
            }
        );
        assert_eq!(
            s(true, true, true).text_options(),
            TextOptions {
                single_line: true,
                paragraphs: false
            }
        );
        assert!(paragraphs_enabled(s(false, true, false)));
        assert!(!paragraphs_enabled(s(true, false, false)));
    }

    /// Tự chép khi xong: chỉ khi công tắc bật lúc kết quả về và kết quả có chữ; chữ theo công tắc.
    #[test]
    fn auto_copy_only_when_on_and_text() {
        let on = Switches {
            single_line: true,
            paragraphs: false,
            auto_copy: true,
        };
        let off = Switches {
            auto_copy: false,
            ..on
        };
        let [ok, empty, nolang, failed]: [Result<OcrResult, OcrError>; 4] =
            results().try_into().unwrap();
        assert_eq!(auto_copy_text(on, &ok).as_deref(), Some("Hello world"));
        assert_eq!(auto_copy_text(off, &ok), None);
        for r in [&empty, &nolang, &failed] {
            assert_eq!(auto_copy_text(on, r), None);
        }
    }

    /// Khung dòng suy biến (`w == 0` hoặc `h == 0`) từ `lines_detail`: chữ vẫn ghép, không panic.
    #[test]
    fn degenerate_line_boxes_keep_text() {
        let res = ok_lines(vec![
            ocr_line("a", 0, 0, 0, 0),
            ocr_line("b", 5, 5, 0, 10),
            ocr_line("c", 5, 30, 10, 0),
        ]);
        let para = TextOptions {
            single_line: false,
            paragraphs: true,
        };
        let v = view_of(Lang::En, &res, para);
        assert!(v.text.contains('a') && v.text.contains('b') && v.text.contains('c'));
    }

    // ---- Chạy tay (mở cửa sổ thật, nhỏ, không chặn):
    // cargo test -p darkshot-windows ocr_window::tests::manual -- --ignored --test-threads=1

    fn pump(ms: u64, done: impl Fn() -> bool) -> bool {
        let deadline = Instant::now() + Duration::from_millis(ms);
        let mut m = MSG::default();
        loop {
            // SAFETY: vòng lặp thông điệp của luồng kiểm thử.
            unsafe {
                while PeekMessageW(&mut m, None, 0, 0, PM_REMOVE).as_bool() {
                    if !pre_translate(&m) {
                        let _ = TranslateMessage(&m);
                        DispatchMessageW(&m);
                    }
                }
            }
            if done() {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(15));
        }
    }

    fn text_of(h: HWND) -> String {
        // SAFETY: đọc chữ của điều khiển thuộc luồng này vào bộ đệm cục bộ.
        unsafe {
            let n = GetWindowTextLengthW(h).max(0) as usize;
            let mut buf = vec![0u16; n + 1];
            let got = GetWindowTextW(h, &mut buf).max(0) as usize;
            String::from_utf16_lossy(&buf[..got])
        }
    }

    fn post_fake(h: HWND, token: usize, text: &str) {
        let res: Result<OcrResult, OcrError> = Ok(OcrResult {
            text: text.into(),
            lang_tag: "en-US".into(),
            lang_name: "English".into(),
            lines: 1,
            lines_detail: Vec::new(),
        });
        let p = Box::into_raw(Box::new(res));
        // SAFETY: gửi tới cửa sổ của luồng này; gửi lỗi thì lấy lại Box.
        unsafe {
            if PostMessageW(Some(h), WM_OCR_RESULT, WPARAM(token), LPARAM(p as isize)).is_err() {
                drop(Box::from_raw(p));
                panic!("PostMessageW failed");
            }
        }
    }

    fn no_ocr(_: Image, _: HWND, _: u32, _: usize) -> Result<(), String> {
        Ok(())
    }

    fn tiny() -> Image {
        Image {
            width: 8,
            height: 8,
            bgra: vec![255; 8 * 8 * 4],
        }
    }

    fn gui_counts() -> (u32, u32) {
        // SAFETY: chỉ đọc bộ đếm của tiến trình hiện tại.
        unsafe {
            (
                GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS),
                GetGuiResources(GetCurrentProcess(), GR_USEROBJECTS),
            )
        }
    }

    fn lang_now() -> Lang {
        crate::lang::current()
    }

    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_recognizes_hello() {
        close_all();
        let img = crate::ocr::tests::text_image(600, 120, Some("Hello darkshot 123"));
        open(img).expect("open");
        let wins = test_windows();
        assert_eq!(wins.len(), 1);
        let h = wins[0].0;
        let edit = edit_of(h);
        assert!(!edit.is_invalid());
        assert!(busy(h), "must start busy");
        assert!(pump(30_000, || !busy(h)), "no result within 30 s");
        let text = text_of(edit);
        if text == t(lang_now(), Msg::OcrNoLanguage) {
            println!("NoLanguage: may khong co goi OCR");
        } else {
            assert!(text.contains("Hello"), "{text:?}");
        }
        close_all();
        assert!(test_windows().is_empty());
        // SAFETY: chỉ hỏi tính hợp lệ của handle.
        assert!(!unsafe { IsWindow(Some(h)) }.as_bool());
    }

    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_open_close_no_leak() {
        close_all();
        for _ in 0..2 {
            open_with(tiny(), no_ocr).expect("open");
            close_all();
        }
        pump(200, || false);
        let before = gui_counts();
        for _ in 0..20 {
            open(tiny()).expect("open");
            pump(20, || false);
            close_all();
            assert!(test_windows().is_empty());
        }
        // Kết quả của 20 lượt nhận dạng tới cửa sổ đã đóng: bị bỏ, không cửa sổ nào còn.
        pump(5_000, || false);
        let after = gui_counts();
        assert!(
            after.0 <= before.0 && after.1 <= before.1,
            "GDI/USER grew: {before:?} -> {after:?}"
        );
    }

    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_close_while_recognizing() {
        close_all();
        let img = crate::ocr::tests::text_image(1920, 1080, Some("Hello darkshot 123"));
        open(img).expect("open");
        let (h, _) = test_windows()[0];
        close_all();
        // SAFETY: chỉ hỏi tính hợp lệ của handle.
        assert!(!unsafe { IsWindow(Some(h)) }.as_bool());
        pump(8_000, || false);
        let mut m = MSG::default();
        // SAFETY: chỉ nhìn hàng đợi của luồng này.
        let left = unsafe { PeekMessageW(&mut m, None, WM_OCR_RESULT, WM_OCR_RESULT, PM_NOREMOVE) };
        assert!(!left.as_bool(), "a result is left in the queue");
        assert!(test_windows().is_empty());
    }

    /// Kết quả token cũ bị bỏ (và giải phóng); kết quả đúng được dùng đúng một lần; kết quả tới sau
    /// khi đóng (còn trong hàng đợi) được vét và giải phóng khi hủy cửa sổ.
    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_late_and_stale_results_freed() {
        close_all();
        FREED.with(|f| f.set(0));
        open_with(tiny(), no_ocr).expect("open");
        let (h, token) = test_windows()[0];
        let edit = edit_of(h);
        post_fake(h, token.wrapping_add(1000), "stale");
        pump(300, || false);
        assert_eq!(FREED.with(|f| f.get()), 1);
        assert!(busy(h));
        assert_eq!(text_of(edit), t(lang_now(), Msg::OcrBusy));

        post_fake(h, token, "Hello");
        assert!(pump(2_000, || !busy(h)));
        assert_eq!(FREED.with(|f| f.get()), 2);
        assert_eq!(text_of(edit), "Hello");

        post_fake(h, token, "again");
        pump(300, || false);
        assert_eq!(FREED.with(|f| f.get()), 3);
        assert_eq!(text_of(edit), "Hello");

        post_fake(h, token, "late 1");
        post_fake(h, token.wrapping_add(7), "late 2");
        close_all();
        assert_eq!(FREED.with(|f| f.get()), 5);
        assert!(test_windows().is_empty());
    }

    /// Mở từ ảnh ghim: khung cần tránh phủ kín vùng làm việc → cửa sổ topmost (nằm trên ảnh ghim);
    /// khung nhỏ (còn chỗ bên cạnh) → không topmost. Không dời chuột, không gửi phím.
    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_topmost_only_when_covered() {
        use windows::Win32::Foundation::POINT;
        use windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTOPRIMARY;
        use windows::Win32::UI::WindowsAndMessaging::{GetWindowLongPtrW, GWL_EXSTYLE};
        close_all();
        // SAFETY: chỉ hỏi màn hình chính.
        let mon = unsafe { MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY) };
        let work = work_area(mon).expect("work area");
        let topmost = |h: HWND| {
            // SAFETY: đọc kiểu mở rộng của cửa sổ do luồng này tạo.
            let ex = unsafe { GetWindowLongPtrW(h, GWL_EXSTYLE) } as u32;
            ex & WS_EX_TOPMOST.0 != 0
        };

        open_inner(tiny(), Some(work), no_ocr).expect("open covered");
        let (covered, _) = test_windows()[0];
        assert!(topmost(covered), "covered: must be topmost");
        close_all();

        let small = Rect {
            x: work.x,
            y: work.y,
            w: 40,
            h: 40,
        };
        open_inner(tiny(), Some(small), no_ocr).expect("open small");
        let (free, _) = test_windows()[0];
        assert!(!topmost(free), "room beside: not topmost");
        close_all();
        assert!(test_windows().is_empty());
    }

    /// Ảnh ghim cuối cùng đóng: `drop_topmost` bỏ topmost của cửa sổ kết quả, cửa sổ vẫn mở.
    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_drop_topmost_clears_flag() {
        use windows::Win32::Foundation::POINT;
        use windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTOPRIMARY;
        use windows::Win32::UI::WindowsAndMessaging::{GetWindowLongPtrW, GWL_EXSTYLE};
        close_all();
        // SAFETY: chỉ hỏi màn hình chính.
        let mon = unsafe { MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY) };
        let work = work_area(mon).expect("work area");
        let topmost = |h: HWND| {
            // SAFETY: đọc kiểu mở rộng của cửa sổ do luồng này tạo.
            let ex = unsafe { GetWindowLongPtrW(h, GWL_EXSTYLE) } as u32;
            ex & WS_EX_TOPMOST.0 != 0
        };
        open_inner(tiny(), Some(work), no_ocr).expect("open covered");
        let (h, _) = test_windows()[0];
        assert!(topmost(h));
        drop_topmost();
        assert!(!topmost(h), "topmost must be dropped");
        // SAFETY: chỉ hỏi tính hợp lệ của handle.
        assert!(unsafe { IsWindow(Some(h)) }.as_bool());
        // Gọi lần nữa (không còn cửa sổ topmost) vô hại.
        drop_topmost();
        close_all();
        assert!(test_windows().is_empty());
    }

    /// Quá 8 cửa sổ: cái cũ nhất đóng.
    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_at_most_eight_windows() {
        close_all();
        open_with(tiny(), no_ocr).expect("open");
        let first = test_windows()[0].0;
        for _ in 0..8 {
            open_with(tiny(), no_ocr).expect("open");
        }
        let wins = test_windows();
        assert_eq!(wins.len(), 8);
        assert!(wins.iter().all(|(h, _)| *h != first));
        // SAFETY: chỉ hỏi tính hợp lệ của handle.
        assert!(!unsafe { IsWindow(Some(first)) }.as_bool());
        close_all();
        assert!(test_windows().is_empty());
    }

    // ---- Chạy tay: khung ảnh, chọn dòng, công tắc. Chỉ gửi thông điệp thẳng tới HWND của chính cửa
    // sổ darkshot (không giả lập chuột/phím thật, quyết định 111). Clipboard được lưu ở đầu và trả
    // lại ở cuối; chỉ ghi chữ tổng hợp.

    const MK_CONTROL: usize = 0x0008;

    /// Giữ chữ đang có trên clipboard của người dùng và trả lại khi bị bỏ (kể cả khi kiểm thử hỏng).
    struct ClipGuard(Option<String>);

    impl ClipGuard {
        fn save(owner: HWND) -> ClipGuard {
            ClipGuard(crate::clipboard::paste_text(owner))
        }
    }

    impl Drop for ClipGuard {
        fn drop(&mut self) {
            let _ = WinClipboard.set_text(self.0.as_deref().unwrap_or(""));
        }
    }

    fn clip(owner: HWND) -> Option<String> {
        crate::clipboard::paste_text(owner)
    }

    fn set_clip(s: &str) {
        WinClipboard.set_text(s).expect("set clipboard");
    }

    fn view_hwnd(h: HWND) -> HWND {
        with_win(h, |w| w.view).unwrap_or_default()
    }

    /// Tâm khung dòng `i` trong tọa độ client của khung ảnh (đã trừ cuộn).
    fn line_point(h: HWND, i: usize) -> Option<(i32, i32)> {
        with_win(h, |w| {
            let l = w.lines.get(i)?;
            let d = w.geom.line_rect(l.rect);
            Some((d.x + d.w / 2, d.y + d.h / 2 - w.scroll))
        })
        .flatten()
    }

    fn picked(h: HWND) -> Vec<usize> {
        with_win(h, |w| w.pick.sel.clone()).unwrap_or_default()
    }

    fn scroll_state(h: HWND) -> (i32, i32, i32) {
        with_win(h, |w| (w.scroll, w.geom.max_scroll(), w.geom.view_h)).unwrap_or_default()
    }

    fn line_texts(h: HWND) -> Vec<String> {
        with_win(h, |w| {
            w.lines.iter().map(|l| l.text.trim().to_string()).collect()
        })
        .unwrap_or_default()
    }

    fn lparam_xy(x: i32, y: i32) -> LPARAM {
        LPARAM((((y as u16 as u32) << 16) | (x as u16 as u32)) as isize)
    }

    /// Bấm (gửi thẳng `WM_LBUTTONDOWN`/`UP` tới khung ảnh) tại `(x, y)` client, có hay không Ctrl.
    fn click(h: HWND, x: i32, y: i32, ctrl: bool) {
        let view = view_hwnd(h);
        let mk = 0x0001 | if ctrl { MK_CONTROL } else { 0 };
        // SAFETY: gửi vào hàng đợi khung ảnh của cửa sổ do luồng này tạo.
        unsafe {
            PostMessageW(Some(view), WM_LBUTTONDOWN, WPARAM(mk), lparam_xy(x, y)).unwrap();
            PostMessageW(
                Some(view),
                windows::Win32::UI::WindowsAndMessaging::WM_LBUTTONUP,
                WPARAM(mk & !0x0001),
                lparam_xy(x, y),
            )
            .unwrap();
        }
        pump(60, || false);
    }

    fn click_line(h: HWND, i: usize, ctrl: bool) {
        let (x, y) = line_point(h, i).expect("line box");
        click(h, x, y, ctrl);
    }

    /// Gõ phím (gửi thẳng `WM_KEYDOWN` vào hàng đợi của khung ảnh; đi qua `pre_translate`).
    fn key(h: HWND, vk: u16) {
        let view = view_hwnd(h);
        // SAFETY: gửi vào hàng đợi khung ảnh của cửa sổ do luồng này tạo.
        unsafe {
            PostMessageW(Some(view), WM_KEYDOWN, WPARAM(usize::from(vk)), LPARAM(1)).unwrap();
        }
        pump(60, || false);
    }

    fn post_lines(h: HWND, token: usize, lines: Vec<OcrLine>) {
        let res = ok_lines(lines);
        let p = Box::into_raw(Box::new(res));
        // SAFETY: gửi tới cửa sổ của luồng này; gửi lỗi thì lấy lại Box.
        unsafe {
            if PostMessageW(Some(h), WM_OCR_RESULT, WPARAM(token), LPARAM(p as isize)).is_err() {
                drop(Box::from_raw(p));
                panic!("PostMessageW failed");
            }
        }
    }

    fn white(w: u32, h: u32) -> Image {
        Image {
            width: w,
            height: h,
            bgra: vec![255; (w as usize) * (h as usize) * 4],
        }
    }

    fn bm_click(b: HWND) {
        // SAFETY: bấm nút của cửa sổ do luồng này tạo (thông điệp tới chính nút).
        unsafe {
            SendMessageW(
                b,
                windows::Win32::UI::WindowsAndMessaging::BM_CLICK,
                None,
                None,
            );
        }
        pump(30, || false);
    }

    thread_local! {
        static TEST_SWITCHES: Cell<Switches> = const { Cell::new(Switches {
            single_line: false,
            paragraphs: false,
            auto_copy: false,
        }) };
        static SAVED: RefCell<Vec<Switches>> = const { RefCell::new(Vec::new()) };
        static NOTICES: RefCell<Vec<(NoticeKind, String)>> = const { RefCell::new(Vec::new()) };
    }

    fn test_load() -> Switches {
        TEST_SWITCHES.with(|s| s.get())
    }

    fn test_save(s: Switches) {
        SAVED.with(|v| v.borrow_mut().push(s));
    }

    fn test_notify(kind: NoticeKind, title: &str, _: &str) {
        NOTICES.with(|v| v.borrow_mut().push((kind, title.to_string())));
    }

    fn reset_hooks(start: Switches) {
        TEST_SWITCHES.with(|s| s.set(start));
        SAVED.with(|v| v.borrow_mut().clear());
        NOTICES.with(|v| v.borrow_mut().clear());
        set_switch_store(test_load, test_save);
        set_notifier(test_notify);
    }

    /// Chữ Anh tổng hợp ba dòng, nhận dạng thật (luồng nền): bấm dòng 2 → clipboard dòng 2;
    /// Ctrl+bấm dòng 1 rồi 3 → ba dòng theo thứ tự đọc; bấm chỗ trống bỏ chọn, không ghi; Ctrl+bấm
    /// bỏ dòng cuối → clipboard giữ "x"; ↑/↓/Enter/Space/Ctrl+A/Esc trên khung ảnh.
    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_pick_lines_after_real_ocr() {
        close_all();
        reset_hooks(Switches::default());
        let wanted = [
            ("Alpha line one", 50),
            ("Bravo line two", 150),
            ("Charlie line three", 250),
        ];
        let (img, _) = {
            let _gdi = crate::capture::GDI_TEST_LOCK.lock();
            crate::ocr::tests::lines_image(700, 300, 32, &wanted)
        };
        open(img).expect("open");
        let (h, _) = test_windows()[0];
        let _clip = ClipGuard::save(h);
        assert!(pump(30_000, || !busy(h)), "no result within 30 s");
        let texts = line_texts(h);
        if text_of(edit_of(h)) == t(lang_now(), Msg::OcrNoLanguage) {
            println!("NoLanguage: may khong co goi OCR");
            close_all();
            return;
        }
        assert_eq!(texts.len(), 3, "{texts:?}");
        let (l1, l2, l3) = (&texts[0], &texts[1], &texts[2]);
        assert!(l2.to_lowercase().contains("bravo"), "{texts:?}");

        click_line(h, 1, false);
        assert_eq!(clip(h).as_deref(), Some(l2.as_str()));
        assert_eq!(picked(h), vec![1]);
        click_line(h, 0, true);
        click_line(h, 2, true);
        assert_eq!(clip(h), Some(format!("{l1}\r\n{l2}\r\n{l3}")));
        assert_eq!(picked(h), vec![0, 1, 2]);

        // Bấm chỗ trống (đệm trên cùng của khung ảnh): bỏ chọn, clipboard không đổi.
        set_clip("x");
        click(h, 2, 2, false);
        assert!(picked(h).is_empty());
        assert_eq!(clip(h).as_deref(), Some("x"));

        // Ctrl+bấm chọn rồi bỏ dòng cuối cùng: lần bỏ không ghi clipboard.
        click_line(h, 0, true);
        assert_eq!(clip(h).as_deref(), Some(l1.as_str()));
        set_clip("x");
        click_line(h, 0, true);
        assert!(picked(h).is_empty());
        assert_eq!(clip(h).as_deref(), Some("x"));

        // Bàn phím trên khung ảnh.
        key(h, VK_DOWN.0);
        key(h, VK_DOWN.0);
        key(h, VK_DOWN.0);
        key(h, VK_UP.0);
        assert_eq!(with_win(h, |w| w.pick.cursor).flatten(), Some(1));
        assert_eq!(clip(h).as_deref(), Some("x"), "arrows must not copy");
        key(h, VK_RETURN.0);
        assert_eq!(clip(h).as_deref(), Some(l2.as_str()));
        set_clip("x");
        key(h, VK_SPACE.0);
        assert!(picked(h).is_empty());
        assert_eq!(clip(h).as_deref(), Some("x"));
        key(h, VK_SPACE.0);
        assert_eq!(clip(h).as_deref(), Some(l2.as_str()));
        // Ctrl+A: Ctrl không giả lập được bằng thông điệp; gọi thẳng thao tác của phím.
        assert_eq!(view_key(u16::from(b'A'), true), Some(PickInput::SelectAll));
        view_input(h, PickInput::SelectAll);
        assert_eq!(clip(h), Some(format!("{l1}\r\n{l2}\r\n{l3}")));
        // Esc: bỏ chọn trước, lần sau mới đóng.
        key(h, VK_ESCAPE.0);
        assert!(picked(h).is_empty());
        // SAFETY: chỉ hỏi tính hợp lệ của handle.
        assert!(unsafe { IsWindow(Some(h)) }.as_bool());
        key(h, VK_ESCAPE.0);
        // SAFETY: chỉ hỏi tính hợp lệ của handle.
        assert!(!unsafe { IsWindow(Some(h)) }.as_bool());
        // Bấm dòng không phát thông báo khay (chỉ nhãn "Đã chép").
        assert!(NOTICES.with(|v| v.borrow().is_empty()));
        close_all();
    }

    /// Review Focus 1: ảnh chụp cuộn 800×12000 (co giữ về 273×4096): khung dòng cuối chỉ hiện sau
    /// khi cuộn; cuộn tới cuối bằng thanh cuộn, bấm trúng đúng dòng cuối; bánh xe cuộn ngược.
    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_tall_image_scrolls_to_last_line() {
        close_all();
        reset_hooks(Switches::default());
        open_with(white(800, 12_000), no_ocr).expect("open");
        let (h, token) = test_windows()[0];
        let _clip = ClipGuard::save(h);
        assert_eq!(with_win(h, |w| w.kept_size), Some((273, 4096)));
        post_lines(
            h,
            token,
            vec![
                ocr_line("first", 20, 100, 300, 40),
                ocr_line("middle", 20, 6000, 300, 40),
                ocr_line("last", 20, 11_900, 300, 40),
            ],
        );
        assert!(pump(2_000, || !busy(h)));
        let (scroll, max, view_h) = scroll_state(h);
        assert_eq!(scroll, 0);
        assert!(max > 0, "tall image must scroll");
        let (_, y) = line_point(h, 2).unwrap();
        assert!(y > view_h, "last line starts hidden: {y} > {view_h}");
        let view = view_hwnd(h);
        // SAFETY: thông điệp thanh cuộn gửi thẳng tới khung ảnh của cửa sổ này.
        unsafe {
            SendMessageW(view, WM_VSCROLL, Some(WPARAM(SB_BOTTOM.0 as usize)), None);
        }
        let (scroll, max, view_h) = scroll_state(h);
        assert_eq!(scroll, max);
        let (x, y) = line_point(h, 2).unwrap();
        assert!(
            (0..view_h).contains(&y),
            "last line visible after scroll: {y}"
        );
        assert_eq!(hit_line(h, x, y), Some(2));
        click(h, x, y, false);
        assert_eq!(clip(h).as_deref(), Some("last"));
        // Bánh xe lên một nấc: cuộn ngược.
        // SAFETY: thông điệp gửi thẳng tới khung ảnh của cửa sổ này.
        unsafe {
            SendMessageW(view, WM_MOUSEWHEEL, Some(WPARAM(120 << 16)), None);
        }
        assert!(scroll_state(h).0 < max);
        // ↑ tới dòng đầu: cuộn cho nó hiện ra (đủ khung, cách mép trên một đệm).
        for _ in 0..3 {
            key(h, VK_UP.0);
        }
        let (scroll, _, view_h) = scroll_state(h);
        let (_, y) = line_point(h, 0).unwrap();
        assert!((0..view_h).contains(&y), "first line visible: {y}");
        assert!(scroll < max);
        close_all();
    }

    /// Review Focus 2: kết quả đến sau khi cửa sổ đã đổi cỡ và đang cuộn; rồi đổi cỡ và DPI lần nữa:
    /// khung dòng đặt lại đúng, bấm trúng đúng dòng.
    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_result_after_resize_and_dpi_change() {
        close_all();
        reset_hooks(Switches::default());
        open_with(white(1600, 3000), no_ocr).expect("open");
        let (h, token) = test_windows()[0];
        let _clip = ClipGuard::save(h);
        let resize = |w: i32, hh: i32| {
            // SAFETY: đổi cỡ chính cửa sổ do luồng này tạo.
            unsafe {
                let _ = SetWindowPos(
                    h,
                    None,
                    0,
                    0,
                    w,
                    hh,
                    SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
            pump(50, || false);
        };
        resize(1300, 900);
        let view = view_hwnd(h);
        // SAFETY: thông điệp thanh cuộn gửi thẳng tới khung ảnh của cửa sổ này.
        unsafe {
            SendMessageW(view, WM_VSCROLL, Some(WPARAM(SB_PAGEDOWN.0 as usize)), None);
        }
        assert!(scroll_state(h).0 > 0);
        let lines: Vec<OcrLine> = (0..10)
            .map(|i| ocr_line(&format!("row {i}"), 100, 100 + i * 290, 900, 60))
            .collect();
        post_lines(h, token, lines);
        assert!(pump(2_000, || !busy(h)));
        let check_all = |ctx: &str| {
            let (_, _, view_h) = scroll_state(h);
            let mut hits = 0;
            for i in 0..10 {
                let (x, y) = line_point(h, i).unwrap();
                if !(0..view_h).contains(&y) {
                    continue;
                }
                hits += 1;
                set_clip("x");
                click(h, x, y, false);
                assert_eq!(clip(h), Some(format!("row {i}")), "{ctx} line {i}");
            }
            assert!(hits > 0, "{ctx}: no visible line");
        };
        check_all("after resize + scroll");
        resize(900, 500);
        check_all("smaller");
        let mut rc = RECT::default();
        // SAFETY: rc là biến cục bộ; hỏi khung của chính cửa sổ này.
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::GetWindowRect(h, &mut rc);
        }
        let big = RECT {
            right: rc.left + (rc.right - rc.left) * 3 / 2,
            bottom: rc.top + (rc.bottom - rc.top) * 3 / 2,
            ..rc
        };
        // SAFETY: thông điệp gửi thẳng tới cửa sổ của luồng này; RECT sống suốt lời gọi.
        unsafe {
            SendMessageW(
                h,
                WM_DPICHANGED,
                Some(WPARAM((144 << 16) | 144)),
                Some(LPARAM(&big as *const RECT as isize)),
            );
        }
        pump(50, || false);
        assert_eq!(with_win(h, |w| w.dpi), Some(144));
        check_all("dpi 144");
        // SAFETY: như trên, trả về 96 DPI.
        unsafe {
            SendMessageW(
                h,
                WM_DPICHANGED,
                Some(WPARAM((96 << 16) | 96)),
                Some(LPARAM(&rc as *const RECT as isize)),
            );
        }
        pump(50, || false);
        check_all("dpi 96");
        close_all();
    }

    /// Review Focus 4 và công tắc: đổi khi đang nhận dạng hay khi lỗi → không panic, ô chữ giữ thông
    /// báo; khi có chữ → ô chữ dựng lại, cấu hình lưu mỗi lần đổi; "Tách đoạn" vô hiệu khi "Gộp một
    /// dòng" bật; đổi công tắc không ghi clipboard.
    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_switches_rebuild_text_and_save() {
        close_all();
        reset_hooks(Switches::default());
        open_with(tiny(), no_ocr).expect("open");
        let (h, token) = test_windows()[0];
        let _clip = ClipGuard::save(h);
        set_clip("x");
        let checks = with_win(h, |w| w.checks).unwrap();
        let edit = edit_of(h);
        let enabled = |b: HWND| {
            // SAFETY: chỉ hỏi trạng thái bật của nút.
            unsafe { IsWindowEnabled(b) }.as_bool()
        };
        // Đang nhận dạng.
        bm_click(checks[0]);
        assert_eq!(text_of(edit), t(lang_now(), Msg::OcrBusy));
        assert!(!enabled(checks[1]), "paragraphs disabled with single line");
        assert_eq!(
            SAVED.with(|v| v.borrow().clone()),
            vec![Switches {
                single_line: true,
                paragraphs: false,
                auto_copy: false
            }]
        );
        // Lỗi.
        let p = Box::into_raw(Box::new(Err::<OcrResult, OcrError>(OcrError::NoLanguage)));
        // SAFETY: gửi tới cửa sổ của luồng này.
        unsafe {
            PostMessageW(Some(h), WM_OCR_RESULT, WPARAM(token), LPARAM(p as isize)).unwrap();
        }
        assert!(pump(2_000, || !busy(h)));
        bm_click(checks[0]);
        assert_eq!(text_of(edit), t(lang_now(), Msg::OcrNoLanguage));
        assert!(enabled(checks[1]));
        close_all();

        // Có chữ: cửa sổ mới đọc giá trị đầu từ cấu hình (hàm do main đưa vào).
        reset_hooks(Switches {
            single_line: true,
            paragraphs: true,
            auto_copy: false,
        });
        open_with(tiny(), no_ocr).expect("open");
        let (h, token) = test_windows()[0];
        let checks = with_win(h, |w| w.checks).unwrap();
        assert!(is_checked(checks[0]) && is_checked(checks[1]) && !is_checked(checks[2]));
        assert!(!enabled(checks[1]));
        post_lines(
            h,
            token,
            vec![
                ocr_line("a", 10, 10, 80, 20),
                ocr_line("b", 10, 34, 80, 20),
                ocr_line("c", 10, 120, 80, 20),
            ],
        );
        assert!(pump(2_000, || !busy(h)));
        let edit = edit_of(h);
        assert_eq!(text_of(edit), "a b c");
        bm_click(checks[0]);
        assert_eq!(text_of(edit), "a\r\nb\r\n\r\nc");
        assert!(enabled(checks[1]));
        bm_click(checks[1]);
        assert_eq!(text_of(edit), "a\r\nb\r\nc");
        bm_click(checks[2]);
        assert_eq!(
            SAVED.with(|v| v.borrow().clone()),
            vec![
                Switches {
                    single_line: false,
                    paragraphs: true,
                    auto_copy: false
                },
                Switches {
                    single_line: false,
                    paragraphs: false,
                    auto_copy: false
                },
                Switches {
                    single_line: false,
                    paragraphs: false,
                    auto_copy: true
                },
            ]
        );
        // Bật "Tự chép" sau khi kết quả đã về: không chép bù; đổi công tắc không ghi clipboard.
        assert_eq!(clip(h).as_deref(), Some("x"));
        // Chọn dòng rồi đổi công tắc: không chép lại vùng chọn.
        click_line(h, 0, false);
        set_clip("x");
        bm_click(checks[0]);
        assert_eq!(clip(h).as_deref(), Some("x"));
        close_all();
    }

    /// Tự chép khi xong: bật lúc kết quả về → chép chữ theo công tắc, báo như nút, cửa sổ ở lại;
    /// tắt → clipboard giữ nguyên. Chép tất cả: chép ô chữ (kể cả đã sửa), cửa sổ ở lại.
    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_auto_copy_and_copy_all_keep_window() {
        close_all();
        let lines = || {
            vec![
                ocr_line("one", 10, 10, 80, 20),
                ocr_line("two", 10, 34, 80, 20),
            ]
        };
        reset_hooks(Switches {
            single_line: true,
            paragraphs: false,
            auto_copy: true,
        });
        open_with(tiny(), no_ocr).expect("open");
        let (h, token) = test_windows()[0];
        let _clip = ClipGuard::save(h);
        set_clip("x");
        post_lines(h, token, lines());
        assert!(pump(2_000, || !busy(h)));
        assert_eq!(clip(h).as_deref(), Some("one two"));
        assert_eq!(
            NOTICES.with(|v| v.borrow().clone()),
            vec![(
                NoticeKind::Success,
                t(lang_now(), Msg::OcrCopiedTitle).to_string()
            )]
        );
        // SAFETY: chỉ hỏi tính hợp lệ của handle.
        assert!(unsafe { IsWindow(Some(h)) }.as_bool());
        close_all();

        reset_hooks(Switches::default());
        open_with(tiny(), no_ocr).expect("open");
        let (h, token) = test_windows()[0];
        set_clip("x");
        post_lines(h, token, lines());
        assert!(pump(2_000, || !busy(h)));
        assert_eq!(clip(h).as_deref(), Some("x"));
        // Chép tất cả lấy chữ đã sửa trong ô; cửa sổ ở lại.
        // SAFETY: ô chữ của cửa sổ do luồng này tạo; chuỗi sống suốt lời gọi.
        unsafe {
            let _ = SetWindowTextW(edit_of(h), &HSTRING::from("edited"));
        }
        let copy = with_win(h, |w| w.copy).unwrap();
        bm_click(copy);
        assert_eq!(clip(h).as_deref(), Some("edited"));
        // SAFETY: chỉ hỏi tính hợp lệ của handle.
        assert!(unsafe { IsWindow(Some(h)) }.as_bool());
        assert_eq!(NOTICES.with(|v| v.borrow().len()), 1);
        close_all();
    }

    /// Khung dòng suy biến (`w == 0`/`h == 0`): vẽ không panic, chỉ trúng trong vùng nới 2 px.
    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_degenerate_boxes_paint_and_hit() {
        close_all();
        reset_hooks(Switches::default());
        open_with(white(400, 200), no_ocr).expect("open");
        let (h, token) = test_windows()[0];
        post_lines(
            h,
            token,
            vec![
                ocr_line("zero", 100, 100, 0, 0),
                ocr_line("flat", 200, 50, 50, 0),
            ],
        );
        assert!(pump(2_000, || !busy(h)));
        let view = view_hwnd(h);
        // SAFETY: vẽ lại ngay khung ảnh của cửa sổ này.
        unsafe {
            let _ = windows::Win32::Graphics::Gdi::RedrawWindow(
                Some(view),
                None,
                None,
                windows::Win32::Graphics::Gdi::RDW_INVALIDATE
                    | windows::Win32::Graphics::Gdi::RDW_UPDATENOW,
            );
        }
        let g = with_win(h, |w| w.geom).unwrap();
        // Ảnh 1:1: px ảnh = điểm khung trừ chỗ ảnh.
        let at = |px: i32, py: i32| hit_line(h, g.img.x + px, g.img.y + py);
        assert_eq!(at(100, 100), Some(0));
        assert_eq!(at(101, 101), Some(0));
        assert_eq!(at(103, 100), None);
        assert_eq!(at(225, 50), Some(1));
        assert_eq!(at(225, 53), None);
        close_all();
    }

    /// Mở/đóng 20 lần với ảnh lớn, kết quả có dòng, bấm và cuộn: không rò GDI/USER.
    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_pick_windows_no_leak() {
        close_all();
        reset_hooks(Switches::default());
        let img = white(2400, 1600);
        let lines = || {
            (0..5)
                .map(|i| ocr_line(&format!("r{i}"), 100, 100 + i * 200, 900, 60))
                .collect::<Vec<_>>()
        };
        let mut guard = None;
        let round = || {
            open_with(img.clone(), no_ocr).expect("open");
            let (h, token) = test_windows()[0];
            post_lines(h, token, lines());
            assert!(pump(2_000, || !busy(h)));
            click_line(h, 1, false);
            click_line(h, 2, true);
            // SAFETY: đổi cỡ chính cửa sổ do luồng này tạo (dựng lại bản đã co).
            unsafe {
                let _ = SetWindowPos(
                    h,
                    None,
                    0,
                    0,
                    1100,
                    700,
                    SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
            pump(80, || false);
            h
        };
        for _ in 0..2 {
            let h = round();
            guard.get_or_insert_with(|| ClipGuard::save(h));
            close_all();
        }
        pump(200, || false);
        let before = gui_counts();
        for _ in 0..20 {
            round();
            close_all();
            assert!(test_windows().is_empty());
        }
        pump(300, || false);
        let after = gui_counts();
        assert!(
            after.0 <= before.0 && after.1 <= before.1,
            "GDI/USER grew: {before:?} -> {after:?}"
        );
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn PrintWindow(hwnd: HWND, hdc: HDC, flags: u32) -> i32;
    }

    /// Ảnh của cửa sổ `h` bằng PrintWindow (PW_RENDERFULLCONTENT), không đọc màn hình.
    fn print_window(h: HWND) -> Image {
        use windows::Win32::Graphics::Gdi::{
            CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GdiFlush, SelectObject,
            BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        };
        let mut rc = RECT::default();
        // SAFETY: rc là biến cục bộ; hỏi khung của chính cửa sổ này.
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::GetWindowRect(h, &mut rc);
        }
        let (w, hh) = (rc.right - rc.left, rc.bottom - rc.top);
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -hh,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let len = (w as usize) * (hh as usize) * 4;
        let mut out = vec![0u8; len];
        let mut bits: *mut c_void = std::ptr::null_mut();
        // SAFETY: DC và DIB chỉ dùng trong khối và đều được giải phóng; chép đúng `len` byte.
        unsafe {
            let dc = CreateCompatibleDC(None);
            let bmp =
                CreateDIBSection(Some(dc), &bmi, DIB_RGB_COLORS, &mut bits, None, 0).expect("dib");
            let old = SelectObject(dc, bmp.into());
            assert_ne!(PrintWindow(h, dc, 2), 0, "PrintWindow");
            let _ = GdiFlush();
            std::ptr::copy_nonoverlapping(bits.cast::<u8>(), out.as_mut_ptr(), len);
            SelectObject(dc, old);
            let _ = DeleteObject(bmp.into());
            let _ = DeleteDC(dc);
        }
        for p in out.chunks_mut(4) {
            p[3] = 255;
        }
        Image {
            width: w as u32,
            height: hh as u32,
            bgra: out,
        }
    }

    /// Ảnh PrintWindow của cửa sổ có kết quả (chữ Anh tổng hợp, nhận dạng thật) và hai dòng đã chọn.
    /// Biến môi trường `DARKSHOT_OCR_SHOT` = đường dẫn PNG để lưu (ngoài repo).
    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_print_window_with_selection() {
        close_all();
        reset_hooks(Switches::default());
        let wanted = [
            ("Quarterly report summary", 60),
            ("Revenue grew 12 percent", 120),
            ("Costs stayed flat", 180),
            ("Next review in March", 300),
            ("Contact the finance team", 360),
        ];
        let (img, _) = {
            let _gdi = crate::capture::GDI_TEST_LOCK.lock();
            crate::ocr::tests::lines_image(720, 420, 30, &wanted)
        };
        open(img).expect("open");
        let (h, _) = test_windows()[0];
        let _clip = ClipGuard::save(h);
        assert!(pump(30_000, || !busy(h)), "no result within 30 s");
        if line_texts(h).len() >= 3 {
            click_line(h, 1, false);
            click_line(h, 2, true);
        }
        pump(100, || false);
        let shot = print_window(h);
        assert!(shot.bgra.chunks(4).any(|p| p[..3] != shot.bgra[..3]));
        if let Ok(path) = std::env::var("DARKSHOT_OCR_SHOT") {
            let png = darkshot_core::image::encode_png(&shot).expect("png");
            std::fs::write(&path, png).expect("write shot");
            println!("saved {path}: lines {:?}", line_texts(h));
        }
        close_all();
    }
}
