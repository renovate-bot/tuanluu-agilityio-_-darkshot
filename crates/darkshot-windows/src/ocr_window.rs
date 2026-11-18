//! Cửa sổ "Chữ nhận dạng": tối, không chặn, đổi cỡ được; tối đa `MAX_WINDOWS` cửa sổ. Mở ra là hiện
//! "Đang nhận dạng chữ…" và giao ảnh cho `ocr::start` (luồng nền); kết quả về bằng `WM_OCR_RESULT`.
//! Thông điệp do vòng lặp của ứng dụng phân phối; `pre_translate` lo bàn phím.
//!
//! Sở hữu kết quả: mỗi `WM_OCR_RESULT` mang một `Box` (LPARAM) được lấy lại đúng một lần, kể cả khi
//! token không khớp, khi cửa sổ đã có kết quả, hay khi cửa sổ bị hủy trước khi lấy thông điệp
//! (vét hàng đợi ở `WM_DESTROY`/`WM_NCDESTROY`, và lưới an toàn trong `pre_translate`).
//! Không bao giờ gọi `ocr::recognize` trên luồng giao diện.

use crate::clipboard::WinClipboard;
use crate::lang;
use crate::ocr::{self, OcrError, OcrResult};
use crate::theme;
use crate::winutil::{self, scale};
use darkshot_core::actions::{Clipboard, NoticeKind};
use darkshot_core::geometry::Rect;
use darkshot_core::i18n::{self, ocr_status, t, Lang, Msg};
use darkshot_core::image::Image;
use darkshot_core::ocr_text::line_count;
use darkshot_core::theme::{BG, INPUT, TEXT, TEXT_MUTED};
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateFontIndirectW, DeleteObject, EndPaint, GetDC, GetMonitorInfoW,
    GetTextExtentPoint32W, InvalidateRect, MonitorFromPoint, MonitorFromRect, ReleaseDC,
    SelectObject, HDC, HFONT, HGDIOBJ, HMONITOR, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    PAINTSTRUCT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{
    EM_SETLIMITTEXT, EM_SETREADONLY, EM_SETSEL, NMHDR, NM_CUSTOMDRAW,
};
use windows::Win32::UI::HiDpi::{
    AdjustWindowRectExForDpi, GetDpiForMonitor, GetDpiForWindow, SystemParametersInfoForDpi,
    MDT_EFFECTIVE_DPI,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    EnableWindow, GetActiveWindow, GetFocus, GetKeyState, IsWindowEnabled, SetFocus, VK_CONTROL,
    VK_ESCAPE, VK_RETURN, VK_SHIFT, VK_TAB,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetCursorPos, GetNextDlgTabItem,
    IsChild, IsDialogMessageW, IsWindow, LoadCursorW, PeekMessageW, RegisterClassW, SendMessageW,
    SetForegroundWindow, SetWindowPos, SetWindowTextW, ShowWindow, BS_PUSHBUTTON, EN_KILLFOCUS,
    EN_SETFOCUS, ES_AUTOVSCROLL, ES_MULTILINE, ES_NOHIDESEL, ES_WANTRETURN, HMENU, IDC_ARROW,
    MINMAXINFO, MSG, NONCLIENTMETRICSW, PM_REMOVE, SPI_GETNONCLIENTMETRICS, SWP_NOACTIVATE,
    SWP_NOZORDER, SW_SHOW, WA_INACTIVE, WINDOW_EX_STYLE, WINDOW_STYLE, WM_ACTIVATE, WM_APP,
    WM_CLOSE, WM_COMMAND, WM_CTLCOLOREDIT, WM_CTLCOLORSTATIC, WM_DESTROY, WM_DPICHANGED,
    WM_GETMINMAXINFO, WM_KEYDOWN, WM_NCDESTROY, WM_NOTIFY, WM_PAINT, WM_SETFOCUS, WM_SETFONT,
    WM_SIZE, WNDCLASSW, WS_CAPTION, WS_CHILD, WS_CLIPCHILDREN, WS_DISABLED, WS_EX_APPWINDOW,
    WS_EX_CONTROLPARENT, WS_GROUP, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_SYSMENU, WS_TABSTOP,
    WS_THICKFRAME, WS_VISIBLE, WS_VSCROLL,
};

/// Kết quả nhận dạng gửi về cửa sổ: WPARAM = token, LPARAM = `Box` của `ocr::start`.
pub const WM_OCR_RESULT: u32 = WM_APP + 12;
/// Vùng client tối thiểu và mặc định (DIP).
const MIN_W: i32 = 420;
const MIN_H: i32 = 260;
const DEFAULT_W: i32 = 560;
const DEFAULT_H: i32 = 380;
/// Mở cửa sổ thứ chín thì đóng cái cũ nhất.
const MAX_WINDOWS: usize = 8;

const CLASS: PCWSTR = w!("darkshot-ocr");
/// Chép mang ID của IDOK, Đóng mang ID của IDCANCEL.
const ID_COPY: i32 = 1;
const ID_CLOSE: i32 = 2;
const ID_EDIT: i32 = 100;
const ID_STATUS: i32 = 101;

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

// Bố cục (DIP): lề, khoảng giữa các phần, nút, dòng trạng thái, đệm của ô chữ trong khung.
const MARGIN: i32 = 12;
const GAP: i32 = 8;
const ROW_GAP: i32 = 4;
const BTN_H: i32 = 28;
const BTN_MIN_W: i32 = 88;
const BTN_PAD: i32 = 16;
const STATUS_H: i32 = 20;
const EDIT_PAD: i32 = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Layout {
    /// Khung tự vẽ của ô chữ (nền `INPUT`, viền, gạch dưới khi có tiêu điểm).
    field: Rect,
    /// Điều khiển EDIT, nằm trong khung.
    edit: Rect,
    status: Rect,
    copy: Rect,
    close: Rect,
}

/// Bề rộng nút cho chữ rộng `text_px` pixel ở `dpi`: tối thiểu `BTN_MIN_W`, đủ chỗ cho chữ và đệm.
fn button_width(text_px: i32, dpi: u32) -> i32 {
    scale(BTN_MIN_W, dpi).max(text_px.max(0) + 2 * scale(BTN_PAD, dpi))
}

/// Bố cục trong vùng client `cw`×`ch` pixel: ô chữ trên cùng, dòng trạng thái, hàng nút Chép | Đóng
/// sát góc dưới phải. Hai nút cùng bề rộng (đủ cho chữ dài hơn, `copy_text`/`close_text` là bề rộng
/// chữ đo bằng phông đậm). Cỡ nhỏ hơn tối thiểu thì các phần co về 0 chứ không âm.
fn layout(cw: i32, ch: i32, dpi: u32, copy_text: i32, close_text: i32) -> Layout {
    let s = |v| scale(v, dpi);
    let (m, bh) = (s(MARGIN), s(BTN_H));
    let bw = button_width(copy_text, dpi).max(button_width(close_text, dpi));
    let btn_y = ch - m - bh;
    let close = Rect {
        x: cw - m - bw,
        y: btn_y,
        w: bw,
        h: bh,
    };
    let copy = Rect {
        x: close.x - s(GAP) - bw,
        ..close
    };
    let status = Rect {
        x: m,
        y: btn_y - s(ROW_GAP) - s(STATUS_H),
        w: (cw - 2 * m).max(0),
        h: s(STATUS_H),
    };
    let field = Rect {
        x: m,
        y: m,
        w: (cw - 2 * m).max(0),
        h: (status.y - s(ROW_GAP) - m).max(0),
    };
    let pad = s(EDIT_PAD);
    let edit = Rect {
        x: field.x + pad,
        y: field.y + pad,
        w: (field.w - 2 * pad).max(0),
        h: (field.h - 2 * pad).max(0),
    };
    Layout {
        field,
        edit,
        status,
        copy,
        close,
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

/// Cửa sổ sau khi có kết quả. Lỗi kỹ thuật (`Failed`) không bao giờ hiện cho người dùng.
fn view_of(lang: Lang, res: &Result<OcrResult, OcrError>) -> View {
    match res {
        Ok(r) if r.text.trim().is_empty() => View {
            text: t(lang, Msg::OcrEmpty).to_string(),
            status: ocr_status(lang, &lang_label(&r.lang_tag, &r.lang_name), 0),
            ..View::default()
        },
        Ok(r) => View {
            text: to_crlf(&r.text),
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

/// Một cửa sổ đang mở. Không bao giờ giữ mượn sổ `WINS` qua lời gọi Win32 có thể gửi thông điệp.
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
    /// Thứ đang hiện (để `relabel` dựng lại chữ khi đổi ngôn ngữ).
    shown: Shown,
}

type Notify = fn(NoticeKind, &str, &str);
type Starter = fn(Image, HWND, u32, usize) -> Result<(), String>;

thread_local! {
    /// Cửa sổ đang mở, cũ nhất trước.
    static WINS: RefCell<Vec<Win>> = const { RefCell::new(Vec::new()) };
    static NEXT_TOKEN: Cell<usize> = const { Cell::new(1) };
    static NOTIFY: Cell<Option<Notify>> = const { Cell::new(None) };
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

/// Hàm `main` đăng ký để báo Đã chép / lỗi clipboard qua khay (lọc thông báo Success ở `main`).
pub fn set_notifier(f: Notify) {
    NOTIFY.with(|n| n.set(Some(f)));
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

fn open_inner(image: Image, avoid: Option<Rect>, start: Starter) -> Result<(), String> {
    close_oldest_over(MAX_WINDOWS - 1);
    let lang = lang::current();
    let hwnd = create(lang, avoid).ok_or_else(|| "CreateWindowExW failed".to_string())?;
    let token = WINS.with(|v| v.borrow().iter().find(|w| w.hwnd == hwnd).map(|w| w.token));
    let Some(token) = token else {
        destroy(hwnd);
        return Err("window not registered".into());
    };
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

fn key_down(vk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY) -> bool {
    // SAFETY: chỉ đọc trạng thái phím của luồng này.
    unsafe { GetKeyState(i32::from(vk.0)) < 0 }
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
                .map(|w| (w.hwnd, w.edit, w.close))
        })
    });
    let Some((hwnd, edit, close)) = owner else {
        return false;
    };
    if msg.message == WM_KEYDOWN {
        let vk = msg.wParam.0 as u16;
        let ctrl = key_down(VK_CONTROL);
        // SAFETY: chỉ hỏi điều khiển có tiêu điểm của luồng này.
        let focus = unsafe { GetFocus() };
        if vk == VK_ESCAPE.0 {
            destroy(hwnd);
            return true;
        }
        if vk == VK_RETURN.0 {
            if ctrl {
                copy_and_close(hwnd);
                return true;
            }
            if focus == edit {
                // Xuống dòng trong ô chữ.
                return false;
            }
            if focus == close {
                destroy(hwnd);
            } else {
                copy_and_close(hwnd);
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

/// Tạo, đăng ký vào sổ và hiện cửa sổ ở trạng thái "đang nhận dạng". Lỗi thì không còn gì sót.
fn create(lang: Lang, avoid: Option<Rect>) -> Option<HWND> {
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
    let (x, y) = match work_area(mon) {
        Some(work) => place_avoiding(work, ww, wh, avoid, scale(GAP, dpi)),
        None => (100, 100),
    };
    let title = HSTRING::from(t(lang, Msg::OcrWindowTitle));
    // SAFETY: đăng ký lớp và tạo cửa sổ ở luồng chính; đăng ký lại lần hai báo lỗi vô hại.
    let hwnd = unsafe {
        let hinst = GetModuleHandleW(None).ok()?;
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: hinst.into(),
            lpszClassName: CLASS,
            hbrBackground: theme::brush(BG),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            ..Default::default()
        };
        RegisterClassW(&wc);
        CreateWindowExW(
            EX_STYLE,
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
    let edit_style = WINDOW_STYLE(
        WS_TABSTOP.0
            | WS_GROUP.0
            | WS_VSCROLL.0
            | ES_MULTILINE as u32
            | ES_AUTOVSCROLL as u32
            | ES_WANTRETURN as u32
            | ES_NOHIDESEL as u32,
    );
    let edit = child(hwnd, w!("EDIT"), "", edit_style, ID_EDIT);
    let status = child(hwnd, w!("STATIC"), "", SS_STATUS, ID_STATUS);
    let push = WINDOW_STYLE(WS_TABSTOP.0 | WS_GROUP.0 | BS_PUSHBUTTON as u32);
    let copy = child(
        hwnd,
        w!("BUTTON"),
        t(lang, Msg::OcrCopy),
        push | WS_DISABLED,
        ID_COPY,
    );
    let close = child(hwnd, w!("BUTTON"), t(lang, Msg::OcrClose), push, ID_CLOSE);
    set_font(edit, efont.or(font));
    set_font(status, font);
    set_font(copy, font);
    set_font(close, font);
    let token = NEXT_TOKEN.with(|n| {
        let v = n.get();
        n.set(v.wrapping_add(1).max(1));
        v
    });
    WINS.with(|v| {
        v.borrow_mut().push(Win {
            hwnd,
            token,
            waiting: Some(token),
            edit,
            status,
            copy,
            close,
            dpi,
            font,
            bold,
            edit_font: efont,
            muted: true,
            label: String::new(),
            focus: HWND::default(),
            shown: Shown::Busy,
            field: Rect {
                x: 0,
                y: 0,
                w: 0,
                h: 0,
            },
        })
    });
    if [edit, status, copy, close].iter().any(|h| h.is_invalid()) {
        // WM_NCDESTROY gỡ khỏi sổ và xóa phông.
        destroy(hwnd);
        return None;
    }
    if !edit.is_invalid() {
        theme::set_dark_control_theme(edit, theme::DarkClass::Explorer);
        // SAFETY: ô chữ của cửa sổ này; 0 = giới hạn tối đa (chữ dài tùy ý).
        unsafe {
            SendMessageW(edit, EM_SETLIMITTEXT, Some(WPARAM(0)), None);
        }
    }
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

/// Đưa `view` lên cửa sổ: chữ ô, chỉ đọc hay không, dòng trạng thái, nút Chép.
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

/// Nhận một `WM_OCR_RESULT`: lấy lại Box (luôn), dùng chỉ khi token khớp và còn chờ.
fn on_result(hwnd: HWND, wp: WPARAM, lp: LPARAM) {
    let res = consume(lp);
    let accepted = with_win(hwnd, |w| {
        if token_matches(w.waiting, wp.0) {
            w.waiting = None;
            if let Ok(r) = res.as_ref() {
                w.label = lang_label(&r.lang_tag, &r.lang_name);
            }
            w.shown = shown_of(&res);
            true
        } else {
            false
        }
    })
    .unwrap_or(false);
    if !accepted {
        return;
    }
    let view = view_of(lang::current(), &res);
    drop(res);
    apply_view(hwnd, &view);
    if !view.editable {
        return;
    }
    let Some(edit) = with_win(hwnd, |w| w.edit) else {
        return;
    };
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

/// Chép chữ hiện có trong ô (kể cả đã sửa) rồi đóng; lỗi clipboard thì báo và giữ cửa sổ.
fn copy_and_close(hwnd: HWND) {
    let Some((edit, copy, label)) = with_win(hwnd, |w| (w.edit, w.copy, w.label.clone())) else {
        return;
    };
    // SAFETY: chỉ hỏi trạng thái bật của nút.
    if !unsafe { IsWindowEnabled(copy) }.as_bool() {
        return;
    }
    let text = String::from_utf16_lossy(&theme::window_text(edit));
    let lang = lang::current();
    match WinClipboard.set_text(&text) {
        Ok(()) => {
            notify(
                NoticeKind::Success,
                t(lang, Msg::OcrCopiedTitle),
                &ocr_status(lang, &label, line_count(&text)),
            );
            destroy(hwnd);
        }
        Err(e) => notify(
            NoticeKind::Error,
            t(lang, Msg::OcrCopyFailedTitle),
            &i18n::ocr_copy_failed_body(lang, &e),
        ),
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

/// Tính lại bố cục theo vùng client hiện tại và DPI của cửa sổ.
fn relayout(hwnd: HWND) {
    let Some((dpi, bold, font, edit, status, copy, close)) = with_win(hwnd, |w| {
        (w.dpi, w.bold, w.font, w.edit, w.status, w.copy, w.close)
    }) else {
        return;
    };
    let mut rc = RECT::default();
    // SAFETY: rc là biến cục bộ; hwnd là cửa sổ của luồng này.
    unsafe {
        let _ = GetClientRect(hwnd, &mut rc);
    }
    let lang = lang::current();
    let f = bold.or(font);
    let l = layout(
        rc.right - rc.left,
        rc.bottom - rc.top,
        dpi,
        text_width(hwnd, f, t(lang, Msg::OcrCopy)),
        text_width(hwnd, f, t(lang, Msg::OcrClose)),
    );
    with_win(hwnd, |w| w.field = l.field);
    for (h, r) in [
        (edit, l.edit),
        (status, l.status),
        (copy, l.copy),
        (close, l.close),
    ] {
        // SAFETY: điều khiển con của cửa sổ này.
        unsafe {
            let _ = SetWindowPos(h, None, r.x, r.y, r.w, r.h, SWP_NOZORDER | SWP_NOACTIVATE);
        }
    }
    // SAFETY: vẽ lại cả cửa sổ (khung ô chữ đổi chỗ).
    unsafe {
        let _ = InvalidateRect(Some(hwnd), None, true);
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
        (old, (w.edit, w.status, w.copy, w.close))
    }) else {
        for f in [font, bold, efont] {
            delete_font(f);
        }
        return;
    };
    let (edit, status, copy, close) = ctrls;
    set_font(edit, efont.or(font));
    set_font(status, font);
    set_font(copy, font);
    set_font(close, font);
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
            if let Some((field, dpi, edit)) = with_win(hwnd, |w| (w.field, w.dpi, w.edit)) {
                if field.w > 0 && field.h > 0 {
                    let rc = rect_of(field);
                    let focused = GetFocus() == edit;
                    theme::buffered(hdc, &rc, |m| {
                        theme::fill(m, &rc, BG);
                        theme::draw_field(m, &rc, dpi, theme::edit_border(focused, true));
                    });
                }
            }
        }
        let _ = EndPaint(hwnd, &ps);
    }
}

fn lo_word(v: usize) -> u32 {
    (v & 0xFFFF) as u32
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

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_OCR_RESULT => {
            on_result(hwnd, wp, lp);
            LRESULT(0)
        }
        WM_COMMAND => {
            let (id, code) = ((wp.0 & 0xFFFF) as i32, ((wp.0 >> 16) & 0xFFFF) as u32);
            match (id, code) {
                (ID_COPY, 0) => copy_and_close(hwnd),
                (ID_CLOSE, 0) => destroy(hwnd),
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
            // Ô chữ (kể cả khi chỉ đọc, gửi WM_CTLCOLORSTATIC): nền INPUT; dòng trạng thái: nền BG.
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
                // Mọi điều khiển đã bị hủy: phông không còn ai dùng.
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

/// Góc trên trái cho cửa sổ `w`×`h` trong vùng làm việc `work` (tọa độ màn hình), tránh khung
/// `avoid` (ảnh ghim, luôn ở trên cùng): thử bên phải, rồi trái, dưới, trên khung, cách khung `gap`;
/// không chỗ nào đủ (hay không có khung) thì giữa vùng làm việc. Luôn kẹp góc vào vùng làm việc.
fn place_avoiding(work: Rect, w: i32, h: i32, avoid: Option<Rect>, gap: i32) -> (i32, i32) {
    let clamp = |v: i32, lo: i32, hi: i32| v.min(hi).max(lo);
    let center = (
        (work.x + (work.w - w) / 2).max(work.x),
        (work.y + (work.h - h) / 2).max(work.y),
    );
    let Some(a) = avoid else {
        return center;
    };
    let (fits_w, fits_h) = (w <= work.w, h <= work.h);
    let y_beside = clamp(a.y, work.y, work.bottom() - h);
    let x_beside = clamp(a.x, work.x, work.right() - w);
    if fits_w && fits_h {
        let right = a.right() + gap;
        if right >= work.x && right + w <= work.right() {
            return (right, y_beside);
        }
        let left = a.x - gap - w;
        if left >= work.x && left + w <= work.right() {
            return (left, y_beside);
        }
        let below = a.bottom() + gap;
        if below >= work.y && below + h <= work.bottom() {
            return (x_beside, below);
        }
        let above = a.y - gap - h;
        if above >= work.y && above + h <= work.bottom() {
            return (x_beside, above);
        }
    }
    center
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
    let all: Vec<(HWND, HWND, HWND, HWND, HWND, Shown)> = WINS.with(|v| {
        v.borrow()
            .iter()
            .map(|w| (w.hwnd, w.edit, w.status, w.copy, w.close, w.shown.clone()))
            .collect()
    });
    for (hwnd, edit, status, copy, close, shown) in all {
        let (text, line) = relabel_parts(lang, &shown);
        let set = |h: HWND, s: &str| {
            // SAFETY: cửa sổ kết quả hoặc điều khiển con của nó; chuỗi sống suốt lời gọi.
            unsafe {
                let _ = SetWindowTextW(h, &HSTRING::from(s));
            }
        };
        set(hwnd, t(lang, Msg::OcrWindowTitle));
        set(copy, t(lang, Msg::OcrCopy));
        set(close, t(lang, Msg::OcrClose));
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
        assert_eq!((MIN_W, MIN_H), (420, 260));
        assert_eq!((DEFAULT_W, DEFAULT_H), (560, 380));
        assert_eq!(MAX_WINDOWS, 8);
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

    /// Ở cỡ tối thiểu và mặc định, mọi DPI 96–192 và hai ngôn ngữ: ô chữ, dòng trạng thái, hai nút
    /// nằm trong vùng client, không chồng nhau, nút đủ rộng cho chữ; ô chữ nằm trong khung của nó.
    #[test]
    fn layout_fits_without_overlap() {
        for lang in LANGS {
            for dpi in DPIS {
                let copy = text_px(t(lang, Msg::OcrCopy), dpi);
                let close = text_px(t(lang, Msg::OcrClose), dpi);
                for (w, h) in [(MIN_W, MIN_H), (DEFAULT_W, DEFAULT_H), (1200, 900)] {
                    let (cw, ch) = (scale(w, dpi), scale(h, dpi));
                    let client = Rect {
                        x: 0,
                        y: 0,
                        w: cw,
                        h: ch,
                    };
                    let l = layout(cw, ch, dpi, copy, close);
                    let ctx = format!("{lang:?} dpi {dpi} {w}x{h}: {l:?}");
                    let parts = [l.field, l.status, l.copy, l.close];
                    for (i, a) in parts.iter().enumerate() {
                        assert!(a.w > 0 && a.h > 0, "{ctx}");
                        assert!(within(*a, client), "{ctx}");
                        for b in &parts[i + 1..] {
                            assert!(!overlaps(*a, *b), "{ctx}");
                        }
                    }
                    assert!(within(l.edit, l.field) && l.edit != l.field, "{ctx}");
                    assert!(l.edit.h >= scale(100, dpi), "{ctx}");
                    assert!(l.copy.w >= copy + 2 * scale(12, dpi), "{ctx}");
                    assert!(l.close.w >= close + 2 * scale(12, dpi), "{ctx}");
                    // Chép bên trái Đóng, cùng hàng, sát mép phải; trạng thái ở trên hàng nút.
                    assert!(l.copy.right() <= l.close.x, "{ctx}");
                    assert_eq!(l.copy.y, l.close.y, "{ctx}");
                    assert_eq!(l.close.right(), cw - scale(12, dpi), "{ctx}");
                    assert!(l.status.bottom() <= l.copy.y, "{ctx}");
                    assert!(l.field.bottom() <= l.status.y, "{ctx}");
                    assert!(l.status.h >= scale(16, dpi), "{ctx}");
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
        let place = |avoid| place_avoiding(work, w, h, avoid, g);
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
        assert_eq!(place_avoiding(work, 2000, 1200, None, g), (-1920, 0));
        assert_eq!(
            place_avoiding(work, 2000, 1200, Some(r(-1800, 100, 100, 100)), g),
            (-1920, 0)
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

    /// `WM_OCR_RESULT` gửi tới cửa sổ không phải cửa sổ kết quả: `pre_translate` không đụng LPARAM.
    #[test]
    fn pre_translate_ignores_foreign_result_message() {
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

    fn results() -> Vec<Result<OcrResult, OcrError>> {
        let ok = |text: &str, lines| {
            Ok(OcrResult {
                text: text.into(),
                lang_tag: "en-US".into(),
                lang_name: "English".into(),
                lines,
            })
        };
        vec![
            ok(
                "Hello
world",
                2,
            ),
            ok("", 0),
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
                let v = view_of(lang, &res);
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
        assert_eq!(title_of(copy), t(Lang::En, Msg::OcrCopy));
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
        for lang in LANGS {
            let busy = busy_view(lang);
            assert_eq!(busy.text, t(lang, Msg::OcrBusy));
            assert!(!busy.editable && !busy.can_copy && busy.muted);
            assert_eq!(busy.status, "");

            let ok = Ok(OcrResult {
                text: "Hello\r\nworld".into(),
                lang_tag: "en-US".into(),
                lang_name: "English".into(),
                lines: 2,
            });
            let v = view_of(lang, &ok);
            assert_eq!(v.text, "Hello\r\nworld");
            assert!(v.editable && v.can_copy && !v.muted);
            assert_eq!(v.status, ocr_status(lang, "English, en-US", 2));

            let empty = Ok(OcrResult {
                text: String::new(),
                lang_tag: "en-US".into(),
                lang_name: "English".into(),
                lines: 0,
            });
            let v = view_of(lang, &empty);
            assert_eq!(v.text, t(lang, Msg::OcrEmpty));
            assert!(!v.editable && !v.can_copy);
            assert_eq!(v.status, ocr_status(lang, "English, en-US", 0));

            let v = view_of(lang, &Err(OcrError::NoLanguage));
            assert_eq!(v.text, t(lang, Msg::OcrNoLanguage));
            assert!(!v.editable && !v.can_copy);
            assert_eq!(v.status, "");

            let v = view_of(
                lang,
                &Err(OcrError::Failed("RecognizeAsync: 0x80004005".into())),
            );
            assert_eq!(v.text, t(lang, Msg::OcrFailed));
            assert!(!v.text.contains("0x8") && !v.status.contains("0x8"));
            assert!(!v.editable && !v.can_copy);
        }
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
}
