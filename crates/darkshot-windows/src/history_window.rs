//! Cửa sổ Lịch sử ảnh chụp: không chặn, tối đa một cửa sổ, đổi cỡ được, vẽ theo DPI của cửa sổ.
//! Thông điệp do vòng lặp của ứng dụng phân phối; `pre_translate` lo Esc và Tab (`IsDialogMessageW`).
//!
//! Bố cục thuần ở `history_layout`, vẽ GDI ở `history_paint`. Danh sách là cửa sổ con riêng
//! (`darkshot-history-list`, có `WS_TABSTOP` và thanh cuộn thật `WS_VSCROLL` theo chủ đề
//! `DarkMode_Explorer`); khung chi tiết vẽ trên cửa sổ chính, nút hành động là `BUTTON` con tự vẽ.
//! Nạp chỉ mục, ảnh nhỏ, bản xem trước chạy ở luồng nền và báo về bằng `PostMessageW` kèm mã thế hệ.

use crate::clipboard::WinClipboard;
use crate::history_layout::{
    actions_for, after_filter, card_text_width, chip_filter, chip_key_dir, chip_step,
    chip_tabstops, compose_over, confirm_card, empty_state_rects, empty_text_width, explorer_exe,
    explorer_select_args, filter_chip, layout, menu_items, preview_target, search_parts,
    select_after_remove, step_selection, url_openable, visible_indices, Avail, CardRects, ChipKey,
    Confirm, ConfirmBtn, ConfirmKey, ConfirmOutcome, Generation, Layout, Nav, SearchParts, Status,
    ThumbCache, ThumbSlot, BTN_PIN, MAX_THUMBS,
};
use crate::history_paint::{self as hp, Bmp, Fonts, PreviewView, RowView, ThumbView, Tip};
use crate::history_service::{self, WM_HISTORY_CHANGED};
use crate::lang;
use crate::paths;
use crate::pin;
use crate::theme::{self, ButtonKind};
use crate::winutil::{self, scale};
use darkshot_core::actions::Clipboard;
use darkshot_core::history::{Entry, Filter, Kind};
use darkshot_core::history_store::Store;
use darkshot_core::i18n::Msg;
use darkshot_core::image::Image;
use darkshot_core::theme::{BG, INPUT, SURFACE, TEXT, TEXT_DISABLED};
use darkshot_core::thumb::{decode_png_file, downscale};
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use windows::core::{w, BOOL, HSTRING, PCWSTR, PWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, ClientToScreen, DeleteObject, EndPaint, EnumDisplayMonitors, GetMonitorInfoW,
    InvalidateRect, MonitorFromPoint, ScreenToClient, HBRUSH, HDC, HFONT, HGDIOBJ, HMONITOR,
    MONITORINFO, MONITOR_DEFAULTTONEAREST, PAINTSTRUCT,
};
use windows::Win32::System::Com::{
    CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{
    EnableScrollBar, InitCommonControlsEx, SetScrollInfo, ESB_DISABLE_BOTH, ESB_ENABLE_BOTH,
    ICC_BAR_CLASSES, INITCOMMONCONTROLSEX, NMHDR, NM_CUSTOMDRAW, TOOLTIPS_CLASSW, TTF_IDISHWND,
    TTF_SUBCLASS, TTM_ADDTOOLW, TTM_NEWTOOLRECTW, TTM_SETMAXTIPWIDTH, TTM_UPDATETIPTEXTW,
    TTS_ALWAYSTIP, TTS_NOPREFIX, TTTOOLINFOW, WM_MOUSELEAVE,
};
use windows::Win32::UI::HiDpi::{
    AdjustWindowRectExForDpi, GetDpiForMonitor, GetDpiForWindow, MDT_EFFECTIVE_DPI,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    EnableWindow, GetFocus, GetKeyState, IsWindowEnabled, SetFocus, TrackMouseEvent, TME_LEAVE,
    TRACKMOUSEEVENT, VK_CONTROL, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_HOME, VK_LEFT, VK_NEXT,
    VK_PRIOR, VK_RETURN, VK_RIGHT, VK_SHIFT, VK_TAB, VK_UP,
};
use windows::Win32::UI::Shell::{
    DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass, ShellExecuteW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    GetClientRect, GetCursorPos, GetNextDlgTabItem, GetParent, GetScrollInfo, GetWindow,
    GetWindowLongPtrW, GetWindowTextLengthW, IsChild, IsDialogMessageW, IsWindow, IsWindowVisible,
    KillTimer, LoadCursorW, MoveWindow, PeekMessageW, PostMessageW, RegisterClassW, SendMessageW,
    SetCursor, SetForegroundWindow, SetTimer, SetWindowLongPtrW, SetWindowPos, SetWindowTextW,
    ShowWindow, TrackPopupMenu, BM_CLICK, BS_PUSHBUTTON, CS_DBLCLKS, CW_USEDEFAULT,
    DLGC_WANTARROWS, DLGC_WANTCHARS, EN_CHANGE, EN_KILLFOCUS, EN_SETFOCUS, ES_AUTOHSCROLL,
    GWL_STYLE, GW_OWNER, HMENU, HTCLIENT, HWND_TOP, IDC_ARROW, IDC_HAND, IDOK, MF_GRAYED,
    MF_SEPARATOR, MF_STRING, MINMAXINFO, MSG, PM_REMOVE, SB_BOTTOM, SB_LINEDOWN, SB_LINEUP,
    SB_PAGEDOWN, SB_PAGEUP, SB_THUMBPOSITION, SB_THUMBTRACK, SB_TOP, SB_VERT, SCROLLBAR_COMMAND,
    SCROLLINFO, SIF_DISABLENOSCROLL, SIF_PAGE, SIF_POS, SIF_RANGE, SIF_TRACKPOS, SWP_NOACTIVATE,
    SWP_NOZORDER, SW_HIDE, SW_SHOW, SW_SHOWNORMAL, TPM_RETURNCMD, TPM_RIGHTBUTTON, WA_INACTIVE,
    WINDOW_EX_STYLE, WINDOW_STYLE, WM_ACTIVATE, WM_APP, WM_CLOSE, WM_COMMAND, WM_CONTEXTMENU,
    WM_CTLCOLOREDIT, WM_CTLCOLORSTATIC, WM_DPICHANGED, WM_ERASEBKGND, WM_GETDLGCODE,
    WM_GETMINMAXINFO, WM_KEYDOWN, WM_KILLFOCUS, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_MOUSEMOVE,
    WM_MOUSEWHEEL, WM_NCDESTROY, WM_NOTIFY, WM_PAINT, WM_SETCURSOR, WM_SETFOCUS, WM_SETFONT,
    WM_SIZE, WM_TIMER, WM_UPDATEUISTATE, WM_VSCROLL, WNDCLASSW, WS_CAPTION, WS_CHILD,
    WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_EX_APPWINDOW, WS_EX_CONTROLPARENT, WS_EX_TOPMOST,
    WS_GROUP, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_POPUP, WS_SYSMENU, WS_TABSTOP, WS_THICKFRAME,
    WS_VSCROLL,
};

const CLASS: PCWSTR = w!("darkshot-history");
const LIST_CLASS: PCWSTR = w!("darkshot-history-list");
/// Cỡ vùng client mặc định và tối thiểu (DIP).
const DEFAULT_W: i32 = 960;
const DEFAULT_H: i32 = 640;
const MIN_W: i32 = 760;
const MIN_H: i32 = 520;
const STYLE: WINDOW_STYLE = WINDOW_STYLE(
    WS_CAPTION.0
        | WS_SYSMENU.0
        | WS_THICKFRAME.0
        | WS_MINIMIZEBOX.0
        | WS_MAXIMIZEBOX.0
        | WS_CLIPCHILDREN.0,
);
const EX_STYLE: WINDOW_EX_STYLE = WINDOW_EX_STYLE(WS_EX_APPWINDOW.0 | WS_EX_CONTROLPARENT.0);

/// Kết quả nạp chỉ mục (`LPARAM` = `Box<LoadMsg>`).
pub const WM_HISTORY_LOADED: u32 = WM_APP + 7;
/// Một ảnh nhỏ đã giải mã (`LPARAM` = `Box<ThumbMsg>`).
pub const WM_HISTORY_THUMB: u32 = WM_APP + 8;
/// Bản xem trước đã giải mã (`LPARAM` = `Box<PreviewMsg>`).
pub const WM_HISTORY_PREVIEW: u32 = WM_APP + 9;
/// Kết quả một hành động (`WPARAM` = `history_layout::Status`, không có `LPARAM`).
pub const WM_HISTORY_STATUS: u32 = WM_APP + 10;
/// Ảnh đã giải mã cho nút Ghim (`LPARAM` = `Box<Image>`).
pub const WM_HISTORY_PIN: u32 = WM_APP + 11;

const ID_LIST: i32 = 200;
/// Nút hành động theo thứ tự nút (và thứ tự Tab): Mở, Sao chép ảnh, Sao chép liên kết, Mở thư mục
/// chứa ảnh, Ghim, Xóa khỏi lịch sử.
const ACTION_IDS: [i32; 6] = [201, 202, 203, 204, 206, 205];
const ID_OPEN: i32 = 201;
const ID_REMOVE: i32 = 205;
const ID_PIN: i32 = 206;
/// Nút "Mở Tùy chọn" (trạng thái tắt và dải thông báo).
const ID_OPEN_OPTIONS: i32 = 210;
/// Nút "Xóa bộ lọc" (trạng thái không khớp), "Xóa lịch sử" (thanh trên).
const ID_CLEAR_FILTERS: i32 = 211;
const ID_CLEAR_ALL: i32 = 212;
/// Hai nút của thẻ xác nhận: Hủy, Xóa lịch sử.
const ID_CARD_CANCEL: i32 = 213;
const ID_CARD_YES: i32 = 214;
/// Ô tìm và nút `×` của nó.
const ID_SEARCH: i32 = 220;
const ID_SEARCH_CLEAR: i32 = 221;
/// Bốn viên lọc: Tất cả, Đã lưu, Đã tải lên, Đã chép.
const CHIP_IDS: [i32; 4] = [230, 231, 232, 233];
/// Hẹn giờ giải mã lại bản xem trước lớn hơn sau khi cửa sổ to ra.
const PREVIEW_TIMER: usize = 1;
const PREVIEW_REFRESH_MS: u32 = 200;
/// Hẹn giờ tắt dòng trạng thái.
const STATUS_TIMER: usize = 2;
const STATUS_MS: u32 = 2500;
/// Độ đục lớp phủ `BG` dưới thẻ xác nhận.
const DIM_ALPHA: u8 = 190;
const CARD_CLASS: PCWSTR = w!("darkshot-history-card");
/// Mã subclass của ô tìm (vẽ chữ gợi ý).
const SEARCH_SUBCLASS: usize = 57;
/// Mã công cụ tooltip của nút `×`.
const TIP_CLEAR: usize = 1;
/// Một "dòng" cuộn (nút mũi tên của thanh cuộn) và số dòng mỗi nấc bánh xe.
const LINE_DIP: i32 = 24;
const WHEEL_LINES: i32 = 3;
/// Đệm chữ hai bên nút hành động, bề rộng tối thiểu (DIP).
const BTN_PAD: i32 = 16;
const BTN_MIN: i32 = 72;

thread_local! {
    /// HWND của cửa sổ đang mở (0 = không có).
    static OPEN: Cell<isize> = const { Cell::new(0) };
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    /// Những gì `NM_CUSTOMDRAW` và ô tìm cần, giữ ngoài `STATE` để vẽ điều khiển không bao giờ đụng
    /// khóa của nó.
    static DRAW: Cell<DrawInfo> = const { Cell::new(DrawInfo::new()) };
}

/// Thông tin vẽ điều khiển con (xem `DRAW`).
#[derive(Clone, Copy)]
struct DrawInfo {
    dpi: u32,
    bold: Option<HFONT>,
    normal: Option<HFONT>,
    /// Nút "Mở Tùy chọn" đang nằm trên dải thông báo.
    in_banner: bool,
    /// Viên lọc đang chọn.
    chip: usize,
    /// Thẻ xác nhận đang mở: điều khiển trong thân bị phủ mờ.
    dim: bool,
}

impl DrawInfo {
    const fn new() -> DrawInfo {
        DrawInfo {
            dpi: 96,
            bold: None,
            normal: None,
            in_banner: false,
            chip: 0,
            dim: false,
        }
    }
}

fn draw_info() -> DrawInfo {
    DRAW.with(|d| d.get())
}

fn update_draw(f: impl FnOnce(&mut DrawInfo)) {
    DRAW.with(|d| {
        let mut v = d.get();
        f(&mut v);
        d.set(v);
    });
}

/// DPI 0 (không hỏi được) coi như 96.
fn norm_dpi(dpi: u32) -> u32 {
    if dpi == 0 {
        96
    } else {
        dpi
    }
}

/// Cỡ vùng client mặc định (pixel) ở `dpi`.
pub fn default_client_size(dpi: u32) -> (i32, i32) {
    let d = norm_dpi(dpi);
    (scale(DEFAULT_W, d), scale(DEFAULT_H, d))
}

/// Cỡ vùng client nhỏ nhất (pixel) ở `dpi`.
pub fn min_client_size(dpi: u32) -> (i32, i32) {
    let d = norm_dpi(dpi);
    (scale(MIN_W, d), scale(MIN_H, d))
}

/// Một màn hình: toàn bộ và vùng làm việc (trừ thanh tác vụ), pixel màn hình ảo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MonitorArea {
    pub bounds: RECT,
    pub work: RECT,
}

/// Bình phương khoảng cách từ điểm tới hình chữ nhật (0 khi nằm trong).
fn dist2(p: (i32, i32), r: &RECT) -> i64 {
    let dx = if p.0 < r.left {
        r.left - p.0
    } else if p.0 >= r.right {
        p.0 - r.right + 1
    } else {
        0
    };
    let dy = if p.1 < r.top {
        r.top - p.1
    } else if p.1 >= r.bottom {
        p.1 - r.bottom + 1
    } else {
        0
    };
    i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy)
}

/// Vị trí và cỡ cửa sổ `(x, y, w, h)`: giữa vùng làm việc của màn hình chứa con trỏ (không có thì
/// màn hình gần nhất), thu nhỏ cho vừa vùng làm việc. Danh sách rỗng: đặt ở (100, 100).
pub fn place_window(
    cursor: (i32, i32),
    monitors: &[MonitorArea],
    size: (i32, i32),
) -> (i32, i32, i32, i32) {
    let Some(m) = monitors.iter().min_by_key(|m| dist2(cursor, &m.bounds)) else {
        return (100, 100, size.0, size.1);
    };
    let a = m.work;
    let (aw, ah) = ((a.right - a.left).max(0), (a.bottom - a.top).max(0));
    let (w, h) = (size.0.min(aw), size.1.min(ah));
    (a.left + (aw - w) / 2, a.top + (ah - h) / 2, w, h)
}

/// Cỡ cửa sổ ngoài (pixel) cho vùng client `w` x `h` ở `dpi`.
fn window_size(w: i32, h: i32, dpi: u32) -> (i32, i32) {
    let mut rc = RECT {
        left: 0,
        top: 0,
        right: w,
        bottom: h,
    };
    // SAFETY: rc là biến cục bộ; hàm chỉ tính toán.
    unsafe {
        let _ = AdjustWindowRectExForDpi(&mut rc, STYLE, false, EX_STYLE, dpi);
    }
    (rc.right - rc.left, rc.bottom - rc.top)
}

unsafe extern "system" fn collect_monitor(
    m: HMONITOR,
    _dc: HDC,
    _rc: *mut RECT,
    data: LPARAM,
) -> BOOL {
    let mut mi = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: `data` trỏ tới Vec<MonitorArea> còn sống trong `monitors`; mi có cbSize đúng.
    unsafe {
        if GetMonitorInfoW(m, &mut mi).as_bool() {
            (*(data.0 as *mut Vec<MonitorArea>)).push(MonitorArea {
                bounds: mi.rcMonitor,
                work: mi.rcWork,
            });
        }
    }
    BOOL(1)
}

/// Mọi màn hình đang gắn.
fn monitors() -> Vec<MonitorArea> {
    let mut out: Vec<MonitorArea> = Vec::new();
    // SAFETY: callback chỉ ghi vào `out`, sống đến hết lời gọi đồng bộ này.
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(collect_monitor),
            LPARAM(&mut out as *mut Vec<MonitorArea> as isize),
        );
    }
    out
}

/// DPI của màn hình chứa (hoặc gần) điểm `pt`.
fn dpi_at(pt: POINT) -> u32 {
    let (mut x, mut y) = (0u32, 0u32);
    // SAFETY: các tham số ra là biến cục bộ.
    let ok = unsafe {
        let m = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
        GetDpiForMonitor(m, MDT_EFFECTIVE_DPI, &mut x, &mut y).is_ok()
    };
    if ok {
        norm_dpi(x)
    } else {
        96
    }
}

fn current() -> HWND {
    let h = OPEN.with(|o| o.get());
    let hwnd = HWND(h as *mut c_void);
    // SAFETY: IsWindow chỉ hỏi tính hợp lệ của handle.
    if h != 0 && unsafe { IsWindow(Some(hwnd)).as_bool() } {
        hwnd
    } else {
        HWND::default()
    }
}

/// Cửa sổ Lịch sử đang mở. Hợp đồng cho các task sau của đợt 7 (hiện chỉ kiểm thử dùng).
#[allow(dead_code)]
pub fn is_open() -> bool {
    !current().is_invalid()
}

/// HWND của cửa sổ đang mở (0 = không có). Hợp đồng cho các task sau (hiện chỉ kiểm thử dùng).
#[allow(dead_code)]
pub fn hwnd() -> isize {
    current().0 as isize
}

// ---------------------------------------------------------------------------
// Luồng nền: nạp chỉ mục, ảnh nhỏ, bản xem trước
// ---------------------------------------------------------------------------

/// Kết quả nạp chỉ mục; `exists[i]` = tệp ảnh của `entries[i]` còn trên đĩa.
struct LoadMsg {
    gen: u64,
    entries: Vec<Entry>,
    exists: Vec<bool>,
}

/// Ảnh nhỏ của mục `id` (điểm ảnh `0x00RRGGBB` đã trộn lên `INPUT`), `None` = lỗi.
struct ThumbMsg {
    gen: u64,
    id: String,
    px: Option<(u32, u32, Vec<u32>)>,
}

enum PreviewResult {
    Ready(u32, u32, Vec<u32>),
    Missing,
    Unreadable,
}

struct PreviewMsg {
    gen: u64,
    result: PreviewResult,
}

/// Đăng `Box` thành `LPARAM`; gửi không được (cửa sổ đã hủy) thì tự giải phóng.
fn post_box<T>(hwnd: isize, msg: u32, b: Box<T>) {
    let p = Box::into_raw(b);
    // SAFETY: PostMessageW an toàn từ luồng bất kỳ; khi thất bại không ai nhận con trỏ nên ta lấy lại.
    unsafe {
        let ok = PostMessageW(
            Some(HWND(hwnd as *mut c_void)),
            msg,
            WPARAM(0),
            LPARAM(p as isize),
        )
        .is_ok();
        if !ok {
            drop(Box::from_raw(p));
        }
    }
}

struct ThumbJob {
    gen: u64,
    entry: Entry,
    cell: (u32, u32),
}

struct PreviewJob {
    gen: u64,
    path: Option<PathBuf>,
    target: (u32, u32),
}

type PreviewSlot = Arc<(Mutex<Option<PreviewJob>>, Condvar)>;

/// Hai luồng nền sống cùng cửa sổ: ảnh nhỏ (hàng đợi) và xem trước (chỉ giữ yêu cầu mới nhất).
/// Bỏ `Workers` thì hai luồng dừng sau việc đang làm và không đăng gì nữa.
struct Workers {
    stop: Arc<AtomicBool>,
    thumbs: Option<mpsc::Sender<ThumbJob>>,
    preview: PreviewSlot,
}

fn decode_preview(path: Option<PathBuf>, target: (u32, u32)) -> PreviewResult {
    let Some(p) = path else {
        return PreviewResult::Missing;
    };
    if !p.exists() {
        return PreviewResult::Missing;
    }
    // Giải mã báo lỗi bằng `Err` (ảnh hỏng, quá cỡ), không panic. Không bọc `catch_unwind`: bản phát
    // hành dùng `panic = "abort"` nên không có gì để bắt.
    let r = decode_png_file(&p).ok().map(|img| {
        // Ảnh gốc bị bỏ ngay sau khi thu nhỏ; giao diện chỉ nhận bản đã thu.
        let small = downscale(&img, target.0, target.1);
        drop(img);
        (small.width, small.height, compose_over(&small, INPUT))
    });
    match r {
        Some((w, h, px)) => PreviewResult::Ready(w, h, px),
        None => PreviewResult::Unreadable,
    }
}

impl Workers {
    fn start(hwnd: isize, root: PathBuf) -> Workers {
        let stop = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel::<ThumbJob>();
        let st = stop.clone();
        let thumb_root = root;
        let thumbs = std::thread::Builder::new()
            .name("darkshot-history-thumb".into())
            .spawn(move || {
                let store = Store::new(thumb_root);
                for job in rx {
                    if st.load(Ordering::SeqCst) {
                        break;
                    }
                    // Lỗi đọc/giải mã trả `Err` (ô hiện ảnh vỡ); xem ghi chú ở `decode_preview`.
                    let px = store.thumbnail(&job.entry).ok().map(|img| {
                        let small = downscale(&img, job.cell.0, job.cell.1);
                        (small.width, small.height, compose_over(&small, INPUT))
                    });
                    if st.load(Ordering::SeqCst) {
                        break;
                    }
                    post_box(
                        hwnd,
                        WM_HISTORY_THUMB,
                        Box::new(ThumbMsg {
                            gen: job.gen,
                            id: job.entry.id,
                            px,
                        }),
                    );
                }
            })
            .is_ok()
            .then_some(tx);
        let preview: PreviewSlot = Arc::new((Mutex::new(None), Condvar::new()));
        let (slot, st) = (preview.clone(), stop.clone());
        let _ = std::thread::Builder::new()
            .name("darkshot-history-preview".into())
            .spawn(move || loop {
                let job = {
                    let (m, cv) = &*slot;
                    let mut g = m.lock().unwrap_or_else(|e| e.into_inner());
                    loop {
                        if st.load(Ordering::SeqCst) {
                            return;
                        }
                        if let Some(j) = g.take() {
                            break j;
                        }
                        g = cv.wait(g).unwrap_or_else(|e| e.into_inner());
                    }
                };
                let result = decode_preview(job.path, job.target);
                if st.load(Ordering::SeqCst) {
                    return;
                }
                post_box(
                    hwnd,
                    WM_HISTORY_PREVIEW,
                    Box::new(PreviewMsg {
                        gen: job.gen,
                        result,
                    }),
                );
            });
        Workers {
            stop,
            thumbs,
            preview,
        }
    }

    fn thumb(&self, job: ThumbJob) {
        if let Some(tx) = &self.thumbs {
            let _ = tx.send(job);
        }
    }

    fn preview(&self, job: PreviewJob) {
        let (m, cv) = &*self.preview;
        *m.lock().unwrap_or_else(|e| e.into_inner()) = Some(job);
        cv.notify_one();
    }
}

impl Drop for Workers {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.thumbs.take();
        let (m, cv) = &*self.preview;
        m.lock().unwrap_or_else(|e| e.into_inner()).take();
        cv.notify_all();
    }
}

/// Nạp chỉ mục trên luồng `darkshot-history-load`, kiểm tệp ảnh còn hay mất, báo về `hwnd`.
fn spawn_load(hwnd: isize, root: PathBuf, gen: u64) {
    let _ = std::thread::Builder::new()
        .name("darkshot-history-load".into())
        .spawn(move || {
            let entries = Store::new(root.clone()).load().entries;
            let exists = entries
                .iter()
                .map(|e| e.image_path(&root).is_some_and(|p| p.exists()))
                .collect();
            post_box(
                hwnd,
                WM_HISTORY_LOADED,
                Box::new(LoadMsg {
                    gen,
                    entries,
                    exists,
                }),
            );
        });
}

// ---------------------------------------------------------------------------
// Trạng thái
// ---------------------------------------------------------------------------

/// Bản xem trước của mục đang chọn; `Ready` giữ đúng một bitmap.
enum Preview {
    Idle,
    Loading,
    Ready(Bmp),
    Missing,
    Unreadable,
}

/// Phông của cửa sổ; xóa khi bị bỏ (sau khi mọi điều khiển dùng chúng đã hủy hoặc đổi phông).
struct FontSet(Fonts);

impl FontSet {
    fn new(dpi: u32) -> FontSet {
        FontSet(Fonts {
            normal: winutil::system_font(dpi),
            bold: winutil::system_font_styled(dpi, 100, true),
            title: winutil::system_font_styled(dpi, 130, true),
        })
    }
}

impl Drop for FontSet {
    fn drop(&mut self) {
        for f in [self.0.normal, self.0.bold, self.0.title]
            .into_iter()
            .flatten()
        {
            // SAFETY: phông do CreateFontIndirectW tạo, không còn điều khiển nào dùng.
            unsafe {
                let _ = DeleteObject(HGDIOBJ(f.0));
            }
        }
    }
}

/// Các điều khiển con (handle chép được, đọc ra khỏi `STATE` trước khi gửi thông điệp).
#[derive(Clone, Copy)]
struct Controls {
    list: HWND,
    buttons: [HWND; 6],
    options: HWND,
    search: HWND,
    search_clear: HWND,
    chips: [HWND; 4],
    clear_all: HWND,
    clear_filters: HWND,
    card: HWND,
    card_btns: [HWND; 2],
    tooltip: HWND,
}

impl Controls {
    /// Điều khiển phía sau thẻ xác nhận (bị vô hiệu khi thẻ mở).
    fn behind(&self) -> Vec<HWND> {
        let mut v = vec![
            self.list,
            self.options,
            self.search,
            self.search_clear,
            self.clear_all,
            self.clear_filters,
        ];
        v.extend(self.buttons);
        v.extend(self.chips);
        v
    }
}

struct State {
    hwnd: HWND,
    list: HWND,
    ctl: Controls,
    filter: Filter,
    query: String,
    confirm: Confirm,
    /// Vị trí thẻ xác nhận (tọa độ client cửa sổ chính) lúc đang mở.
    card: Option<CardRects>,
    /// Dòng trạng thái đang hiện.
    status: Option<Status>,
    /// Tooltip đang gắn cho đường dẫn và liên kết bị cắt.
    tips: Vec<Tip>,
    /// Chiều cao một dòng chữ của phông thường (pixel).
    text_h: i32,
    root: PathBuf,
    entries: Vec<Entry>,
    /// Song song với `entries`: tệp ảnh còn trên đĩa.
    exists: Vec<bool>,
    /// Chỉ số vào `entries` theo thứ tự hiển thị (Task 57 điền theo bộ lọc; ở đây là tất cả).
    visible: Vec<usize>,
    loaded: bool,
    selected: Option<String>,
    scroll: i32,
    thumbs: ThumbCache<Bmp>,
    thumb_gen: Generation,
    preview: Preview,
    preview_gen: Generation,
    preview_id: Option<String>,
    load_gen: Generation,
    hover: Option<usize>,
    tracking: bool,
    dpi: u32,
    enabled: bool,
    layout: Layout,
    fonts: FontSet,
    /// Điều khiển có tiêu điểm lúc cửa sổ mất kích hoạt.
    last_focus: HWND,
    workers: Workers,
}

impl State {
    fn sel_pos(&self) -> Option<usize> {
        let id = self.selected.as_deref()?;
        self.visible
            .iter()
            .position(|&i| self.entries.get(i).is_some_and(|e| e.id == id))
    }

    fn sel_entry(&self) -> Option<(&Entry, bool)> {
        let p = self.sel_pos()?;
        let i = *self.visible.get(p)?;
        Some((
            self.entries.get(i)?,
            self.exists.get(i).copied().unwrap_or(false),
        ))
    }

    fn has_entries(&self) -> bool {
        self.loaded && !self.entries.is_empty()
    }

    fn banner(&self) -> bool {
        self.has_entries() && !self.enabled
    }

    /// Có mục nhưng tìm/lọc không khớp mục nào.
    fn no_match(&self) -> bool {
        self.has_entries() && self.visible.is_empty()
    }

    /// Khung chi tiết đang hiện (có ít nhất một mục khớp).
    fn has_detail(&self) -> bool {
        self.has_entries() && !self.visible.is_empty()
    }

    /// Tính lại danh sách hiển thị theo bộ lọc và ô tìm.
    fn compute_visible(&mut self) {
        self.visible = visible_indices(&self.entries, self.filter, &self.query, lang::current());
    }

    fn visible_ids(&self) -> Vec<String> {
        self.visible
            .iter()
            .filter_map(|&i| self.entries.get(i).map(|e| e.id.clone()))
            .collect()
    }

    /// Dòng trạng thái: ở khung chi tiết, hoặc giữa đáy thân khi không có khung chi tiết.
    fn status_rect(&self) -> (RECT, bool) {
        if self.has_detail() {
            return (self.layout.status_line, false);
        }
        let s = |v: i32| scale(v, self.dpi);
        let b = self.layout.body;
        (
            RECT {
                left: b.left + s(16),
                top: b.bottom - s(16) - s(20),
                right: b.right - s(16),
                bottom: b.bottom - s(16),
            },
            true,
        )
    }

    /// Kích thước đích của bản xem trước (vùng xem trước trừ đệm).
    fn preview_inner(&self) -> (i32, i32) {
        let pad = scale(16, self.dpi);
        let a = self.layout.preview_area;
        (a.right - a.left - 2 * pad, a.bottom - a.top - 2 * pad)
    }

    /// Nhờ luồng nền giải mã bản xem trước của mục đang chọn. `refresh`: giữ bản đang hiện tới khi có
    /// bản mới (đổi cỡ cửa sổ).
    fn request_preview(&mut self, refresh: bool) {
        let gen = self.preview_gen.next();
        let Some((e, _)) = self.sel_entry() else {
            self.preview = Preview::Idle;
            self.preview_id = None;
            return;
        };
        let id = e.id.clone();
        let path = e.image_path(&self.root);
        if !refresh || self.preview_id.as_deref() != Some(id.as_str()) {
            self.preview = Preview::Loading;
        }
        self.preview_id = Some(id);
        let (w, h) = self.preview_inner();
        self.workers.preview(PreviewJob {
            gen,
            path,
            target: preview_target(w, h),
        });
    }

    /// Bản xem trước đang hiện nhỏ hơn mức có thể (cửa sổ vừa to ra).
    fn preview_can_grow(&self) -> bool {
        let (Preview::Ready(b), Some((e, _))) = (&self.preview, self.sel_entry()) else {
            return false;
        };
        let (w, h) = self.preview_inner();
        let (tw, th) = preview_target(w, h);
        let (fw, fh) = darkshot_core::thumb::fit_within(e.width, e.height, tw, th);
        fw as i32 > b.w + 1 || fh as i32 > b.h + 1
    }
}

fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    STATE.with(|st| {
        let mut g = st.try_borrow_mut().ok()?;
        g.as_mut().map(f)
    })
}

// ---------------------------------------------------------------------------
// Mở, đóng, bàn phím
// ---------------------------------------------------------------------------

/// Mở cửa sổ Lịch sử, hoặc đưa cửa sổ đang mở ra trước.
pub fn show(owner: HWND) {
    show_with_root(owner, paths::history_dir());
}

fn register_classes(hinst: windows::Win32::Foundation::HINSTANCE) {
    // SAFETY: con trỏ hệ thống dùng chung, không phải hủy; đăng ký lại lần hai báo lỗi vô hại.
    unsafe {
        let arrow = LoadCursorW(None, IDC_ARROW).unwrap_or_default();
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: hinst,
            lpszClassName: CLASS,
            hbrBackground: theme::brush(BG),
            hCursor: arrow,
            ..Default::default()
        };
        RegisterClassW(&wc);
        let lc = WNDCLASSW {
            style: CS_DBLCLKS,
            lpfnWndProc: Some(list_proc),
            hInstance: hinst,
            lpszClassName: LIST_CLASS,
            hCursor: arrow,
            ..Default::default()
        };
        RegisterClassW(&lc);
        let cc = WNDCLASSW {
            lpfnWndProc: Some(card_proc),
            hInstance: hinst,
            lpszClassName: CARD_CLASS,
            hCursor: arrow,
            hbrBackground: HBRUSH::default(),
            ..Default::default()
        };
        RegisterClassW(&cc);
    }
}

fn child(parent: HWND, class: PCWSTR, style: WINDOW_STYLE, id: i32) -> HWND {
    // SAFETY: tạo cửa sổ con của cửa sổ Lịch sử ở luồng chính; vị trí đặt sau ở `relayout`.
    unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class,
            &HSTRING::new(),
            // WS_CLIPSIBLINGS: không vẽ đè lên thẻ xác nhận (anh em nằm trên cùng khi mở).
            WS_CHILD | WS_CLIPSIBLINGS | style,
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

fn set_font(h: HWND, f: Option<HFONT>) {
    if let (Some(f), false) = (f, h.is_invalid()) {
        // SAFETY: phông sống tới khi cửa sổ Lịch sử hủy (hoặc tới khi được thay bằng phông mới).
        unsafe {
            SendMessageW(h, WM_SETFONT, Some(WPARAM(f.0 as usize)), Some(LPARAM(1)));
        }
    }
}

/// Như `show` nhưng với thư mục kho truyền vào (kiểm thử dùng kho tạm).
pub fn show_with_root(owner: HWND, root: PathBuf) {
    let open = current();
    if !open.is_invalid() {
        winutil::bring_to_front(open);
        return;
    }
    let mut pt = POINT::default();
    // SAFETY: pt là biến cục bộ; lỗi thì giữ (0, 0).
    unsafe {
        let _ = GetCursorPos(&mut pt);
    }
    // Tạo thẳng trên màn hình có con trỏ, theo DPI của nó: không có WM_DPICHANGED lúc mở.
    let dpi = dpi_at(pt);
    let (cw, ch) = default_client_size(dpi);
    let (ww, wh) = window_size(cw, ch, dpi);
    let (x, y, w, h) = place_window((pt.x, pt.y), &monitors(), (ww, wh));
    let title = HSTRING::from(lang::tr(Msg::HistoryTitle));
    // SAFETY: tạo cửa sổ ở luồng chính.
    let created = unsafe {
        let Ok(hinst) = GetModuleHandleW(None) else {
            return;
        };
        register_classes(hinst.into());
        CreateWindowExW(
            EX_STYLE,
            CLASS,
            &title,
            STYLE,
            x,
            y,
            w,
            h,
            Some(owner),
            None,
            Some(hinst.into()),
            None,
        )
    };
    let Ok(hwnd) = created else {
        return;
    };
    // SAFETY: hwnd vừa tạo ở luồng này.
    let dpi = norm_dpi(unsafe { GetDpiForWindow(hwnd) });
    let fonts = FontSet::new(dpi);
    let button_style = WINDOW_STYLE(WS_TABSTOP.0 | BS_PUSHBUTTON as u32);
    // Thứ tự tạo = thứ tự Tab: ô tìm, bộ lọc, Xóa lịch sử, danh sách, nút hành động, các nút khác.
    let search = child(
        hwnd,
        w!("EDIT"),
        WINDOW_STYLE(WS_TABSTOP.0 | WS_GROUP.0 | ES_AUTOHSCROLL as u32),
        ID_SEARCH,
    );
    let search_clear = child(
        hwnd,
        w!("BUTTON"),
        WINDOW_STYLE(BS_PUSHBUTTON as u32),
        ID_SEARCH_CLEAR,
    );
    let chips: [HWND; 4] = std::array::from_fn(|k| {
        let mut st = BS_PUSHBUTTON as u32;
        if k == 0 {
            st |= WS_GROUP.0 | WS_TABSTOP.0;
        }
        child(hwnd, w!("BUTTON"), WINDOW_STYLE(st), CHIP_IDS[k])
    });
    let clear_all = child(
        hwnd,
        w!("BUTTON"),
        WINDOW_STYLE(button_style.0 | WS_GROUP.0),
        ID_CLEAR_ALL,
    );
    let list = child(
        hwnd,
        LIST_CLASS,
        WINDOW_STYLE(WS_TABSTOP.0 | WS_VSCROLL.0),
        ID_LIST,
    );
    theme::set_dark_control_theme(list, theme::DarkClass::Explorer);
    let buttons = ACTION_IDS.map(|id| child(hwnd, w!("BUTTON"), button_style, id));
    let options = child(hwnd, w!("BUTTON"), button_style, ID_OPEN_OPTIONS);
    let clear_filters = child(hwnd, w!("BUTTON"), button_style, ID_CLEAR_FILTERS);
    let card = child(hwnd, CARD_CLASS, WINDOW_STYLE(WS_CLIPCHILDREN.0), 0);
    // SAFETY: thêm kiểu mở rộng cho cửa sổ thẻ vừa tạo (Tab đi vào hai nút của nó).
    unsafe {
        let ex = GetWindowLongPtrW(card, windows::Win32::UI::WindowsAndMessaging::GWL_EXSTYLE);
        SetWindowLongPtrW(
            card,
            windows::Win32::UI::WindowsAndMessaging::GWL_EXSTYLE,
            ex | WS_EX_CONTROLPARENT.0 as isize,
        );
    }
    let card_btns =
        [ID_CARD_CANCEL, ID_CARD_YES].map(|id| child(card, w!("BUTTON"), button_style, id));
    let labels = [
        Msg::HistoryOpen,
        Msg::HistoryCopyImage,
        Msg::HistoryCopyLink,
        Msg::HistoryShowInFolder,
        Msg::HistoryPin,
        Msg::HistoryRemove,
    ];
    for (b, m) in buttons.iter().zip(labels) {
        set_text(*b, lang::tr(m));
    }
    let chip_labels = [
        Msg::HistoryFilterAll,
        Msg::HistoryFilterSaved,
        Msg::HistoryFilterUploaded,
        Msg::HistoryFilterCopied,
    ];
    for (c, m) in chips.iter().zip(chip_labels) {
        set_text(*c, lang::tr(m));
    }
    set_text(options, lang::tr(Msg::HistoryOpenOptions));
    set_text(clear_all, lang::tr(Msg::HistoryClear));
    set_text(clear_filters, lang::tr(Msg::HistoryClearFilters));
    set_text(search_clear, lang::tr(Msg::HistorySearchClear));
    set_text(card_btns[0], lang::tr(Msg::HistoryCancel));
    set_text(card_btns[1], lang::tr(Msg::HistoryClearConfirmYes));
    let mut all = vec![
        search,
        search_clear,
        clear_all,
        options,
        clear_filters,
        card_btns[0],
        card_btns[1],
    ];
    all.extend(buttons);
    all.extend(chips);
    for c in all {
        set_font(c, fonts.0.normal);
    }
    setup_search(search);
    let tooltip = create_tooltip(hwnd, search_clear, dpi);
    update_draw(|d| {
        *d = DrawInfo {
            dpi,
            bold: fonts.0.bold,
            normal: fonts.0.normal,
            ..DrawInfo::new()
        }
    });
    let text_h = hp::with_screen_dc(|dc| hp::measure(dc, fonts.0.normal, "Ag").1);
    let ctl = Controls {
        list,
        buttons,
        options,
        search,
        search_clear,
        chips,
        clear_all,
        clear_filters,
        card,
        card_btns,
        tooltip,
    };
    let hwnd_i = hwnd.0 as isize;
    let mut load_gen = Generation::default();
    let gen = load_gen.next();
    let state = State {
        hwnd,
        list,
        ctl,
        filter: Filter::All,
        query: String::new(),
        confirm: Confirm::default(),
        card: None,
        status: None,
        tips: Vec::new(),
        text_h,
        root: root.clone(),
        entries: Vec::new(),
        exists: Vec::new(),
        visible: Vec::new(),
        loaded: false,
        selected: None,
        scroll: 0,
        thumbs: ThumbCache::new(MAX_THUMBS),
        thumb_gen: Generation::default(),
        preview: Preview::Idle,
        preview_gen: Generation::default(),
        preview_id: None,
        load_gen,
        hover: None,
        tracking: false,
        dpi,
        enabled: crate::history_enabled_now(),
        layout: layout(0, 0, dpi, false),
        fonts,
        last_focus: list,
        workers: Workers::start(hwnd_i, root.clone()),
    };
    STATE.with(|s| *s.borrow_mut() = Some(state));
    OPEN.with(|o| o.set(hwnd_i));
    history_service::set_window(hwnd_i);
    spawn_load(hwnd_i, root, gen);
    winutil::set_app_icons(hwnd);
    theme::apply_dark_title_bar(hwnd);
    relayout(hwnd);
    // SAFETY: hwnd và list vừa tạo ở luồng này.
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
        let _ = SetFocus(Some(list));
    }
}

/// Đóng cửa sổ nếu đang mở; không có thì không làm gì.
pub fn close_if_open() {
    let h = current();
    if !h.is_invalid() {
        // SAFETY: h là cửa sổ Lịch sử còn sống của luồng này; dọn dẹp ở WM_NCDESTROY.
        unsafe {
            let _ = DestroyWindow(h);
        }
    }
}

/// Cấu hình vừa đổi (Tùy chọn lưu xong): đọc lại cờ bật lịch sử, vẽ lại dải thông báo.
pub fn refresh_enabled() {
    let h = current();
    if h.is_invalid() {
        return;
    }
    let on = crate::history_enabled_now();
    if with_state(|s| std::mem::replace(&mut s.enabled, on) != on) == Some(true) {
        relayout(h);
    }
}

fn chip_key_of(vk: u16) -> Option<ChipKey> {
    Some(match vk {
        v if v == VK_LEFT.0 => ChipKey::Left,
        v if v == VK_RIGHT.0 => ChipKey::Right,
        v if v == VK_UP.0 => ChipKey::Up,
        v if v == VK_DOWN.0 => ChipKey::Down,
        _ => return None,
    })
}

/// `true`: thông điệp đã được xử lý, vòng lặp bỏ qua. Esc: đóng thẻ xác nhận, hoặc xóa chữ ô tìm, hoặc
/// đóng cửa sổ; Ctrl+F vào ô tìm; mũi tên trái/phải đổi bộ lọc khi tiêu điểm ở viên lọc; khi thẻ xác
/// nhận mở thì Tab chỉ xoay giữa hai nút của nó. Còn lại (Tab, Enter) qua `IsDialogMessageW`.
pub fn pre_translate(msg: &MSG) -> bool {
    let h = current();
    if h.is_invalid() || !winutil::belongs_to(h, msg) {
        return false;
    }
    if msg.message == WM_KEYDOWN {
        let vk = msg.wParam.0 as u16;
        // SAFETY: chỉ đọc trạng thái phím của luồng này.
        let (ctrl, shift) = unsafe {
            (
                GetKeyState(VK_CONTROL.0 as i32) < 0,
                GetKeyState(VK_SHIFT.0 as i32) < 0,
            )
        };
        if confirm_is_open() {
            if confirm_key(h, vk, shift) {
                return true;
            }
        } else if vk == VK_ESCAPE.0 {
            if query_is_empty() {
                close_if_open();
            } else {
                clear_search();
            }
            return true;
        } else if ctrl && !shift && vk == u16::from(b'F') {
            if let Some(search) = with_state(|s| s.ctl.search) {
                // SAFETY: đưa tiêu điểm vào ô tìm và chọn hết chữ (con của cửa sổ này).
                unsafe {
                    let _ = SetFocus(Some(search));
                    SendMessageW(
                        search,
                        windows::Win32::UI::Controls::EM_SETSEL,
                        Some(WPARAM(0)),
                        Some(LPARAM(-1)),
                    );
                }
            }
            return true;
        } else if let Some(key) = chip_key_of(vk) {
            let chips = with_state(|s| (s.ctl.chips, s.filter));
            // SAFETY: chỉ hỏi cửa sổ có tiêu điểm.
            let f = unsafe { GetFocus() };
            if let Some((chips, cur)) = chips.filter(|(c, _)| c.contains(&f)) {
                let next = chip_step(cur, chip_key_dir(key));
                set_filter(h, next);
                show_focus_cues(h);
                // SAFETY: đưa tiêu điểm tới viên vừa chọn (con của cửa sổ này).
                unsafe {
                    let _ = SetFocus(Some(chips[filter_chip(next)]));
                }
                return true;
            }
        }
    }
    // SAFETY: xử lý phím điều hướng của cửa sổ Lịch sử (cửa sổ của luồng này).
    unsafe { IsDialogMessageW(h, msg).as_bool() }
}

// ---------------------------------------------------------------------------
// Bố cục, cuộn, chọn
// ---------------------------------------------------------------------------

fn set_text(h: HWND, s: &str) {
    if theme::window_text(h) != s.encode_utf16().collect::<Vec<u16>>() {
        // SAFETY: đổi chữ của điều khiển con thuộc luồng này.
        unsafe {
            let _ = SetWindowTextW(h, &HSTRING::from(s));
        }
    }
}

/// Bề rộng nút cho nhãn `text` (pixel).
fn button_width(font: Option<HFONT>, text: &str, dpi: u32) -> i32 {
    let (w, _) = hp::with_screen_dc(|dc| hp::measure(dc, font, text));
    (w + 2 * scale(BTN_PAD, dpi)).max(scale(BTN_MIN, dpi))
}

/// Việc phải làm với các điều khiển con sau khi tính bố cục (làm ngoài khóa `STATE`).
struct ChildPlan {
    ctl: Controls,
    list: (RECT, bool),
    buttons: [(RECT, Avail); 6],
    open_label: Msg,
    options: Option<(RECT, bool)>,
    search: SearchParts,
    clear_search: bool,
    chips: [RECT; 4],
    chip: usize,
    clear_all: (RECT, bool),
    clear_filters: Option<RECT>,
    /// Thẻ xác nhận (tọa độ client cửa sổ chính) khi đang mở.
    card: Option<CardRects>,
    /// Lịch sử tắt và trống: tiêu điểm phải ở nút "Mở Tùy chọn".
    focus_options: bool,
}

/// Tính lại bố cục theo cỡ client hiện tại rồi đặt các cửa sổ con; vẽ lại cả cửa sổ.
fn relayout(hwnd: HWND) {
    let mut rc = RECT::default();
    // SAFETY: rc là biến cục bộ.
    unsafe {
        let _ = GetClientRect(hwnd, &mut rc);
    }
    let lang = lang::current();
    let tr = |m: Msg| darkshot_core::i18n::t(lang, m);
    let plan = with_state(|s| {
        let dpi = s.dpi;
        let mut l = layout(rc.right, rc.bottom, dpi, s.banner());
        let normal = s.fonts.0.normal;
        let bold = s.fonts.0.bold;
        let acts = s.sel_entry().map(|(e, ok)| actions_for(e, ok));
        let open_label = if acts.is_some_and(|a| a.open_is_link) {
            Msg::HistoryOpenLink
        } else {
            Msg::HistoryOpen
        };
        let labels = [
            open_label,
            Msg::HistoryCopyImage,
            Msg::HistoryCopyLink,
            Msg::HistoryShowInFolder,
            Msg::HistoryPin,
            Msg::HistoryRemove,
        ];
        let widths: [i32; 6] = std::array::from_fn(|k| {
            let f = if k == 0 { bold } else { normal };
            button_width(f, tr(labels[k]), dpi)
        });
        let avail = acts.map_or([Avail::Hidden; 6], |a| a.as_array());
        let shown = avail.map(|a| a != Avail::Hidden && s.has_detail());
        l.place_buttons(widths, shown);
        s.layout = l;
        let options = if s.loaded && !s.enabled {
            let bw = button_width(
                if s.has_entries() { normal } else { bold },
                tr(Msg::HistoryOpenOptions),
                dpi,
            );
            if s.has_entries() {
                let b = l.banner_btn;
                Some((
                    RECT {
                        left: (b.right - bw).min(b.left),
                        ..b
                    },
                    true,
                ))
            } else {
                let tw = empty_text_width(&l.body, dpi);
                let th = hp::with_screen_dc(|dc| {
                    hp::measure_wrapped(dc, normal, tr(Msg::HistoryDisabledBody), tw)
                });
                let e = empty_state_rects(&l.body, dpi, th, Some(bw));
                Some((e.button, false))
            }
        } else {
            None
        };
        let clear_filters = s.no_match().then(|| {
            let bw = button_width(normal, tr(Msg::HistoryClearFilters), dpi);
            let tw = empty_text_width(&l.body, dpi);
            let th = hp::with_screen_dc(|dc| {
                hp::measure_wrapped(dc, normal, tr(Msg::HistoryNoMatchBody), tw)
            });
            empty_state_rects(&l.body, dpi, th, Some(bw)).button
        });
        let card = s.confirm.is_open().then(|| {
            let title = tr(Msg::HistoryClearConfirmTitle);
            let body = darkshot_core::i18n::history_clear_confirm_body(lang, s.entries.len());
            let (th, bh) = hp::with_screen_dc(|dc| {
                (
                    hp::measure(dc, s.fonts.0.title, title).1,
                    hp::measure_wrapped_left(dc, normal, &body, card_text_width(dpi)),
                )
            });
            let bw = [
                button_width(normal, tr(Msg::HistoryCancel), dpi),
                button_width(bold, tr(Msg::HistoryClearConfirmYes), dpi),
            ];
            confirm_card(&l, th, bh, bw)
        });
        s.card = card;
        ChildPlan {
            ctl: s.ctl,
            list: (l.list, s.has_detail()),
            buttons: std::array::from_fn(|k| {
                let a = if shown[k] { avail[k] } else { Avail::Hidden };
                (l.buttons[k], a)
            }),
            open_label,
            options,
            search: search_parts(&l.search, dpi, s.text_h),
            clear_search: !s.query.is_empty(),
            chips: l.chips,
            chip: filter_chip(s.filter),
            clear_all: (l.clear_btn, s.has_entries()),
            clear_filters,
            card,
            focus_options: s.loaded && !s.enabled && s.entries.is_empty(),
        }
    });
    let Some(plan) = plan else {
        return;
    };
    let c = plan.ctl;
    let modal = plan.card.is_some();
    let mv = |h: HWND, r: &RECT, show: bool| {
        // SAFETY: đặt vị trí và hiện/ẩn cửa sổ con của luồng này (không giữ khóa STATE).
        unsafe {
            let _ = MoveWindow(h, r.left, r.top, r.right - r.left, r.bottom - r.top, false);
            let _ = ShowWindow(h, if show { SW_SHOW } else { SW_HIDE });
        }
    };
    let enable = |h: HWND, on: bool| {
        // SAFETY: bật/tắt điều khiển con của luồng này.
        unsafe {
            let _ = EnableWindow(h, on && !modal);
        }
    };
    update_draw(|d| {
        d.chip = plan.chip;
        d.dim = modal;
        d.in_banner = plan.options.is_some_and(|(_, b)| b);
    });
    // Thanh trên. Ô trống (nút `×` ẩn): ô EDIT kéo dài tới gần mép phải để chữ gợi ý đủ chỗ hơn.
    let mut edit = plan.search.edit;
    if !plan.clear_search {
        edit.right = plan.search.clear.right - scale(4, draw_info().dpi);
    }
    mv(c.search, &edit, true);
    enable(c.search, true);
    mv(c.search_clear, &plan.search.clear, plan.clear_search);
    enable(c.search_clear, true);
    for (k, ch) in c.chips.iter().enumerate() {
        mv(*ch, &plan.chips[k], true);
        enable(*ch, true);
    }
    set_chip_tabstops(&c.chips, plan.chip);
    mv(c.clear_all, &plan.clear_all.0, true);
    enable(c.clear_all, plan.clear_all.1);
    // Thân.
    mv(c.list, &plan.list.0, plan.list.1);
    enable(c.list, true);
    set_text(c.buttons[0], lang::tr(plan.open_label));
    for (k, b) in c.buttons.iter().enumerate() {
        let (r, a) = plan.buttons[k];
        enable(*b, a == Avail::Enabled);
        mv(*b, &r, a != Avail::Hidden);
    }
    match plan.options {
        Some((r, _)) => mv(c.options, &r, true),
        None => mv(c.options, &RECT::default(), false),
    }
    enable(c.options, true);
    match plan.clear_filters {
        Some(r) => mv(c.clear_filters, &r, true),
        None => mv(c.clear_filters, &RECT::default(), false),
    }
    enable(c.clear_filters, true);
    // Thẻ xác nhận: nằm trên cùng các anh em (chúng có WS_CLIPSIBLINGS nên không vẽ đè lên nó).
    match plan.card {
        Some(cr) => {
            let k = cr.card;
            let rel = |r: &RECT| RECT {
                left: r.left - k.left,
                top: r.top - k.top,
                right: r.right - k.left,
                bottom: r.bottom - k.top,
            };
            // SAFETY: đặt thẻ (cửa sổ con của luồng này) lên trên cùng và hiện nó.
            unsafe {
                let _ = SetWindowPos(
                    c.card,
                    Some(HWND_TOP),
                    k.left,
                    k.top,
                    k.right - k.left,
                    k.bottom - k.top,
                    windows::Win32::UI::WindowsAndMessaging::SWP_SHOWWINDOW,
                );
            }
            // Góc bo thật (vùng cửa sổ): phần ngoài góc để lộ lớp phủ bên dưới.
            let r2 = 2 * scale(crate::history_layout::CARD_RADIUS, draw_info().dpi);
            // SAFETY: vùng mới tạo được giao cho hệ thống sau SetWindowRgn (không xóa lại); lỗi thì xóa.
            unsafe {
                let rgn = windows::Win32::Graphics::Gdi::CreateRoundRectRgn(
                    0,
                    0,
                    k.right - k.left + 1,
                    k.bottom - k.top + 1,
                    r2,
                    r2,
                );
                if !rgn.is_invalid()
                    && windows::Win32::Graphics::Gdi::SetWindowRgn(c.card, Some(rgn), true) == 0
                {
                    let _ = DeleteObject(HGDIOBJ(rgn.0));
                }
            }
            mv(c.card_btns[0], &rel(&cr.cancel), true);
            mv(c.card_btns[1], &rel(&cr.yes), true);
        }
        None => mv(c.card, &RECT::default(), false),
    }
    // Bề rộng client thật của danh sách (trừ thanh cuộn thật).
    let mut lc = RECT::default();
    // SAFETY: lc là biến cục bộ.
    unsafe {
        let _ = GetClientRect(c.list, &mut lc);
    }
    with_state(|s| {
        if lc.right > 0 {
            s.layout.list_client_w = lc.right;
        }
        s.scroll = s.layout.scroll_clamp(s.scroll, s.visible.len());
    });
    update_scrollbar(c.list);
    fix_focus(hwnd, &c, plan.focus_options);
    // SAFETY: chỉ đánh dấu vùng cần vẽ lại (cả các con).
    unsafe {
        let _ = windows::Win32::Graphics::Gdi::RedrawWindow(
            Some(hwnd),
            None,
            None,
            windows::Win32::Graphics::Gdi::RDW_INVALIDATE
                | windows::Win32::Graphics::Gdi::RDW_ALLCHILDREN,
        );
    }
}

/// Chỉ viên lọc đang chọn có `WS_TABSTOP` (như nhóm nút radio).
fn set_chip_tabstops(chips: &[HWND; 4], sel: usize) {
    let stops = chip_tabstops(chip_filter(sel).unwrap_or(Filter::All));
    for (k, h) in chips.iter().enumerate() {
        // SAFETY: đọc/ghi kiểu cửa sổ của nút con thuộc luồng này.
        unsafe {
            let st = GetWindowLongPtrW(*h, GWL_STYLE);
            let want = if stops[k] {
                st | WS_TABSTOP.0 as isize
            } else {
                st & !(WS_TABSTOP.0 as isize)
            };
            if want != st {
                SetWindowLongPtrW(*h, GWL_STYLE, want);
            }
        }
    }
}

/// Điều khiển dùng được: hiện và không bị vô hiệu.
fn usable(h: HWND) -> bool {
    // SAFETY: chỉ hỏi trạng thái cửa sổ.
    !h.is_invalid() && unsafe { IsWindowVisible(h).as_bool() && IsWindowEnabled(h).as_bool() }
}

/// Tiêu điểm không được nằm trên điều khiển ẩn hay vô hiệu: chuyển sang điều khiển dùng được kế tiếp
/// (lịch sử tắt và trống: nút "Mở Tùy chọn"). Cửa sổ không có tiêu điểm thì sửa `last_focus`.
fn fix_focus(hwnd: HWND, c: &Controls, focus_options: bool) {
    // SAFETY: chỉ hỏi cửa sổ có tiêu điểm và quan hệ cha con.
    let f = unsafe { GetFocus() };
    // Vô hiệu điều khiển đang có tiêu điểm làm mất tiêu điểm (GetFocus rỗng) dù cửa sổ vẫn đang
    // kích hoạt: coi như tiêu điểm vẫn thuộc cửa sổ này.
    // SAFETY: như trên.
    let ours = unsafe {
        (!f.is_invalid() && IsChild(hwnd, f).as_bool())
            || (f.is_invalid()
                && windows::Win32::UI::Input::KeyboardAndMouse::GetActiveWindow() == hwnd)
    };
    let cur = if ours {
        f
    } else {
        with_state(|s| s.last_focus).unwrap_or_default()
    };
    if usable(cur) {
        return;
    }
    let target = if focus_options && usable(c.options) {
        c.options
    } else {
        // SAFETY: tìm điều khiển Tab kế tiếp dùng được (bỏ qua ẩn và vô hiệu) trong cửa sổ này.
        let next = unsafe { GetNextDlgTabItem(hwnd, Some(cur), false) }.unwrap_or_default();
        if usable(next) {
            next
        } else if usable(c.list) {
            c.list
        } else {
            c.search
        }
    };
    if ours {
        // SAFETY: đưa tiêu điểm tới điều khiển con dùng được của cửa sổ này.
        unsafe {
            let _ = SetFocus(Some(target));
        }
    } else {
        with_state(|s| s.last_focus = target);
    }
}

/// Trạng thái mũi tên của thanh cuộn danh sách sau `SetScrollInfo`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScrollArrows {
    /// Thẻ xác nhận mở: thanh cuộn mờ, không bấm được.
    Disabled,
    /// Nội dung dài hơn khung nhìn: bật lại (có thể đã bị vô hiệu khi thẻ mở).
    Enabled,
    /// Không cần cuộn: để `SIF_DISABLENOSCROLL` quyết định.
    Auto,
}

/// Quyết định thuần cho thanh cuộn danh sách.
fn scrollbar_arrows(card_open: bool, content: i32, view: i32) -> ScrollArrows {
    if card_open {
        ScrollArrows::Disabled
    } else if content > view {
        ScrollArrows::Enabled
    } else {
        ScrollArrows::Auto
    }
}

fn update_scrollbar(list: HWND) {
    let Some((content, view, pos, card_open)) = with_state(|s| {
        (
            s.layout.content_h(s.visible.len()),
            s.layout.list.bottom - s.layout.list.top,
            s.scroll,
            s.confirm.is_open(),
        )
    }) else {
        return;
    };
    let si = SCROLLINFO {
        cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
        fMask: SIF_RANGE | SIF_PAGE | SIF_POS | SIF_DISABLENOSCROLL,
        nMin: 0,
        nMax: (content - 1).max(0),
        nPage: view.max(0) as u32,
        nPos: pos,
        nTrackPos: 0,
    };
    // SAFETY: si là biến cục bộ đủ cbSize; list là cửa sổ con của luồng này.
    unsafe {
        SetScrollInfo(list, SB_VERT, &si, true);
    }
    // Sau `SetScrollInfo` (vì `SIF_DISABLENOSCROLL` có thể bật lại thanh cuộn). Thanh cuộn là vùng
    // ngoài client: `EnableWindow(false)` của danh sách không làm nó mờ khi thẻ xác nhận mở.
    let arrows = match scrollbar_arrows(card_open, content, view) {
        ScrollArrows::Disabled => ESB_DISABLE_BOTH,
        ScrollArrows::Enabled => ESB_ENABLE_BOTH,
        ScrollArrows::Auto => return,
    };
    // SAFETY: list là cửa sổ con của luồng này; lỗi (đã đúng trạng thái) bỏ qua.
    unsafe {
        let _ = EnableScrollBar(list, SB_VERT.0 as u32, arrows);
    }
}

/// Cuộn danh sách tới `target` pixel (đã kẹp).
fn scroll_to(list: HWND, target: i32) {
    let changed = with_state(|s| {
        let n = s.layout.scroll_clamp(target, s.visible.len());
        let c = n != s.scroll;
        s.scroll = n;
        s.hover = None;
        c
    });
    if changed == Some(true) {
        update_scrollbar(list);
        // SAFETY: chỉ đánh dấu vùng cần vẽ lại.
        unsafe {
            let _ = InvalidateRect(Some(list), None, false);
        }
    }
}

/// Chọn dòng hiển thị thứ `pos`: cuộn cho thấy, nạp bản xem trước, cập nhật nút.
fn select_pos(pos: usize) {
    let changed = with_state(|s| {
        let i = *s.visible.get(pos)?;
        let id = s.entries.get(i)?.id.clone();
        s.scroll = s.layout.ensure_visible(pos, s.scroll, s.visible.len());
        if s.selected.as_deref() == Some(id.as_str()) {
            return Some(false);
        }
        s.selected = Some(id);
        s.request_preview(false);
        Some(true)
    })
    .flatten();
    let Some((hwnd, list)) = with_state(|s| (s.hwnd, s.list)) else {
        return;
    };
    match changed {
        Some(true) => relayout(hwnd),
        Some(false) => {
            update_scrollbar(list);
            // SAFETY: chỉ đánh dấu vùng cần vẽ lại.
            unsafe {
                let _ = InvalidateRect(Some(list), None, false);
            }
        }
        None => {}
    }
}

fn nav_of(vk: u16) -> Option<Nav> {
    Some(match vk {
        v if v == VK_UP.0 => Nav::Up,
        v if v == VK_DOWN.0 => Nav::Down,
        v if v == VK_HOME.0 => Nav::Home,
        v if v == VK_END.0 => Nav::End,
        v if v == VK_PRIOR.0 => Nav::PageUp,
        v if v == VK_NEXT.0 => Nav::PageDown,
        _ => return None,
    })
}

fn on_loaded(hwnd: HWND, m: LoadMsg) {
    let enabled = crate::history_enabled_now();
    let done = with_state(|s| {
        if !s.load_gen.is_current(m.gen) {
            return false;
        }
        let old = s.visible_ids();
        let old_refs: Vec<&str> = old.iter().map(String::as_str).collect();
        let sel = s.selected.take();
        s.entries = m.entries;
        s.exists = m.exists;
        s.exists.resize(s.entries.len(), false);
        s.compute_visible();
        let new = s.visible_ids();
        let new_refs: Vec<&str> = new.iter().map(String::as_str).collect();
        // Giữ mục chọn theo id; mục vừa xóa thì chọn mục kế ở cùng vị trí (hoặc mục cuối).
        let pos = sel
            .as_deref()
            .and_then(|id| select_after_remove(&old_refs, id, &new_refs));
        s.selected = pos.and_then(|p| new.get(p).cloned());
        if let Some(p) = pos {
            s.scroll = s.layout.ensure_visible(p, s.scroll, s.visible.len());
        }
        s.thumbs.forget_broken();
        s.loaded = true;
        s.enabled = enabled;
        s.hover = None;
        s.scroll = s.layout.scroll_clamp(s.scroll, s.visible.len());
        if s.selected != s.preview_id {
            s.request_preview(false);
        }
        true
    });
    if done == Some(true) {
        relayout(hwnd);
    }
}

fn start_load(hwnd: HWND) {
    if let Some((root, gen)) = with_state(|s| (s.root.clone(), s.load_gen.next())) {
        spawn_load(hwnd.0 as isize, root, gen);
    }
}

fn on_thumb(m: ThumbMsg) {
    let row = with_state(|s| {
        if !s.thumb_gen.is_current(m.gen) {
            return None;
        }
        let bmp = m.px.and_then(|(w, h, px)| hp::bmp_from_pixels(w, h, &px));
        // Bitmap bị thay hoặc bị loại khỏi đệm bị xóa ngay tại đây (Drop của Bmp).
        drop(s.thumbs.fulfill(&m.id, bmp));
        let pos = s
            .visible
            .iter()
            .position(|&i| s.entries.get(i).is_some_and(|e| e.id == m.id))?;
        Some((s.list, s.layout.row_rect(pos, s.scroll)))
    })
    .flatten();
    if let Some((list, r)) = row {
        // SAFETY: chỉ đánh dấu dòng cần vẽ lại.
        unsafe {
            let _ = InvalidateRect(Some(list), Some(&r), false);
        }
    }
}

fn on_preview(m: PreviewMsg) {
    let target = with_state(|s| {
        if !s.preview_gen.is_current(m.gen) {
            return None;
        }
        s.preview = match m.result {
            PreviewResult::Ready(w, h, px) => match hp::bmp_from_pixels(w, h, &px) {
                Some(b) => Preview::Ready(b),
                None => Preview::Unreadable,
            },
            PreviewResult::Missing => Preview::Missing,
            PreviewResult::Unreadable => Preview::Unreadable,
        };
        Some((s.hwnd, s.layout.preview_area))
    })
    .flatten();
    if let Some((h, r)) = target {
        // SAFETY: chỉ đánh dấu vùng cần vẽ lại.
        unsafe {
            let _ = InvalidateRect(Some(h), Some(&r), false);
        }
    }
}

/// Giải phóng các `Box` còn nằm trong hàng đợi thông điệp của cửa sổ sắp hủy.
fn drain_results(hwnd: HWND) {
    let mut msg = MSG::default();
    // SAFETY: lấy ra đúng các thông điệp kết quả của cửa sổ này; mỗi LPARAM là Box do post_box cấp
    // phát với đúng kiểu theo mã thông điệp, chưa ai nhận.
    unsafe {
        while PeekMessageW(
            &mut msg,
            Some(hwnd),
            WM_HISTORY_LOADED,
            WM_HISTORY_PREVIEW,
            PM_REMOVE,
        )
        .as_bool()
        {
            let p = msg.lParam.0;
            if p == 0 {
                continue;
            }
            match msg.message {
                WM_HISTORY_LOADED => drop(Box::from_raw(p as *mut LoadMsg)),
                WM_HISTORY_THUMB => drop(Box::from_raw(p as *mut ThumbMsg)),
                WM_HISTORY_PREVIEW => drop(Box::from_raw(p as *mut PreviewMsg)),
                _ => {}
            }
        }
        // Ảnh ghim chưa nhận: giải phóng.
        while PeekMessageW(
            &mut msg,
            Some(hwnd),
            WM_HISTORY_PIN,
            WM_HISTORY_PIN,
            PM_REMOVE,
        )
        .as_bool()
        {
            if msg.lParam.0 != 0 {
                drop(Box::from_raw(msg.lParam.0 as *mut Image));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tìm, lọc
// ---------------------------------------------------------------------------

/// Đọc lại ô tìm, tính lại danh sách hiển thị: giữ mục chọn nếu còn, không thì chọn mục đầu và cuộn
/// về đầu.
fn apply_filter(hwnd: HWND) {
    let Some(search) = with_state(|s| s.ctl.search) else {
        return;
    };
    let query = String::from_utf16_lossy(&theme::window_text(search));
    let changed = with_state(|s| {
        s.query = query;
        let sel = s.selected.clone();
        s.compute_visible();
        let ids = s.visible_ids();
        let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
        let r = after_filter(sel.as_deref(), &refs);
        s.selected = r.selected.and_then(|p| ids.get(p).cloned());
        s.hover = None;
        if r.scroll_top {
            s.scroll = 0;
        }
        if let Some(p) = r.selected {
            s.scroll = s.layout.ensure_visible(p, s.scroll, s.visible.len());
        }
        if s.selected != sel || s.selected != s.preview_id {
            s.request_preview(false);
        }
    });
    if changed.is_some() {
        relayout(hwnd);
    }
}

/// Chọn bộ lọc `f` (viên thuốc, phím mũi tên, "Xóa bộ lọc").
fn set_filter(hwnd: HWND, f: Filter) {
    with_state(|s| s.filter = f);
    apply_filter(hwnd);
}

/// Xóa chữ ô tìm (`EN_CHANGE` tính lại danh sách).
fn clear_search() {
    if let Some(search) = with_state(|s| s.ctl.search) {
        // SAFETY: đổi chữ của ô tìm (con của cửa sổ này), không giữ khóa STATE.
        unsafe {
            let _ = SetWindowTextW(search, &HSTRING::new());
        }
    }
}

fn query_is_empty() -> bool {
    with_state(|s| s.query.is_empty()).unwrap_or(true)
}

// ---------------------------------------------------------------------------
// Hành động (thiết kế mục 3.6): việc nặng ở luồng nền, báo về bằng WM_HISTORY_STATUS
// ---------------------------------------------------------------------------

/// Báo kết quả `st` về cửa sổ (an toàn từ luồng bất kỳ; cửa sổ đã đóng thì bỏ qua).
fn post_status(hwnd: isize, st: Status) {
    // SAFETY: PostMessageW an toàn từ luồng bất kỳ; không có con trỏ đi kèm.
    unsafe {
        let _ = PostMessageW(
            Some(HWND(hwnd as *mut c_void)),
            WM_HISTORY_STATUS,
            WPARAM(st as usize),
            LPARAM(0),
        );
    }
}

/// Trao ảnh đã giải mã cho cửa sổ để ghim (`LPARAM` = `Box<Image>`); không gửi được thì báo lỗi.
fn post_pin_image(hwnd: isize, img: Image) {
    post_box(hwnd, WM_HISTORY_PIN, Box::new(img));
}

/// Ảnh đã giải mã của nút Ghim, trên luồng giao diện: mở ảnh ghim giữa màn hình chứa con trỏ.
/// Cửa sổ Lịch sử giữ nguyên. Lỗi ghim hiện thông báo như đường chụp (không kèm lý do kỹ thuật).
fn on_pin_image(img: Image) {
    if let Err(e) = pin::open(img, pin::PinPlace::CenterOnCursorMonitor) {
        let (kind, title, body) = crate::pin_failure_notice(lang::current(), &e);
        if let Some(tray) = crate::current_tray() {
            use darkshot_core::actions::Notifier;
            tray.notifier().notify(kind, title, &body);
        }
    }
}

/// Chạy `f` trên một luồng nền rồi báo kết quả; không tạo được luồng thì báo lỗi.
fn spawn_task(hwnd: HWND, f: impl FnOnce() -> Status + Send + 'static) {
    let h = hwnd.0 as isize;
    let ok = std::thread::Builder::new()
        .name("darkshot-history-action".into())
        .spawn(move || post_status(h, f()))
        .is_ok();
    if !ok {
        post_status(h, Status::Failed);
    }
}

/// `ShellExecuteW` trên luồng hiện tại (luồng nền) với COM khởi tạo cân bằng. `true` khi thành công.
fn shell_execute(file: &str, params: Option<&str>) -> bool {
    let file = HSTRING::from(file);
    let params = params.map(HSTRING::from);
    // SAFETY: các chuỗi sống đến hết lời gọi; ShellExecuteW không giữ con trỏ. COM được khởi tạo (nếu
    // được) rồi giải phóng cân bằng quanh lời gọi vì ShellExecuteW cần COM ở luồng gọi.
    unsafe {
        let inited =
            CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE).is_ok();
        let p = params
            .as_ref()
            .map_or(PCWSTR::null(), |p| PCWSTR(p.as_ptr()));
        let r = ShellExecuteW(None, w!("open"), &file, p, None, SW_SHOWNORMAL);
        if inited {
            CoUninitialize();
        }
        // Tài liệu ShellExecuteW: giá trị lớn hơn 32 là thành công.
        r.0 as isize > 32
    }
}

/// Thư mục Windows (`GetWindowsDirectoryW`), không hỏi được thì `None`.
fn windows_dir() -> Option<String> {
    let mut buf = [0u16; 260];
    // SAFETY: buf là bộ đệm cục bộ; hàm ghi tối đa buf.len() ký tự và trả số ký tự đã ghi.
    let n =
        unsafe { windows::Win32::System::SystemInformation::GetWindowsDirectoryW(Some(&mut buf)) }
            as usize;
    (n > 0 && n < buf.len()).then(|| String::from_utf16_lossy(&buf[..n]))
}

/// Hành động của nút/lệnh `id` (một phần tử của `ACTION_IDS`) cho mục đang chọn, nếu nó dùng được.
fn on_action(id: i32) {
    let Some((hwnd, e, ok, root)) = with_state(|s| {
        let (e, ok) = s.sel_entry()?;
        Some((s.hwnd, e.clone(), ok, s.root.clone()))
    })
    .flatten() else {
        return;
    };
    let Some(k) = ACTION_IDS.iter().position(|&x| x == id) else {
        return;
    };
    let a = actions_for(&e, ok);
    if a.as_array().get(k) != Some(&Avail::Enabled) {
        return;
    }
    match k {
        // Mở: tệp ảnh bằng ứng dụng mặc định, hoặc liên kết https trên trình duyệt.
        0 => {
            if e.kind == Kind::Uploaded {
                match e.url.filter(|u| url_openable(u)) {
                    Some(u) => spawn_task(hwnd, move || {
                        if shell_execute(&u, None) {
                            Status::None
                        } else {
                            Status::Failed
                        }
                    }),
                    None => on_status(hwnd, Status::Failed),
                }
            } else if let Some(p) = e.image_path(&root) {
                spawn_task(hwnd, move || {
                    if shell_execute(&p.to_string_lossy(), None) {
                        Status::None
                    } else {
                        Status::Failed
                    }
                });
            }
        }
        // Sao chép ảnh: giải mã PNG ở luồng nền rồi đặt lên clipboard.
        1 => {
            if let Some(p) = e.image_path(&root) {
                spawn_task(hwnd, move || {
                    match decode_png_file(&p)
                        .ok()
                        .map(|img| WinClipboard.set_image(&img))
                    {
                        Some(Ok(())) => Status::ImageCopied,
                        _ => Status::Failed,
                    }
                });
            }
        }
        // Sao chép liên kết: nhanh, làm ngay trên luồng giao diện.
        2 => {
            let st = match e.url.as_deref().map(|u| WinClipboard.set_text(u)) {
                Some(Ok(())) => Status::LinkCopied,
                _ => Status::Failed,
            };
            on_status(hwnd, st);
        }
        // Mở thư mục chứa ảnh, chọn sẵn tệp.
        3 => match e.path.as_deref().and_then(explorer_select_args) {
            Some(args) => {
                let exe = explorer_exe(windows_dir().as_deref());
                spawn_task(hwnd, move || {
                    if shell_execute(&exe, Some(&args)) {
                        Status::None
                    } else {
                        Status::Failed
                    }
                });
            }
            // Đường dẫn lạ (không tuyệt đối, có `"` hay ký tự điều khiển): không gọi explorer.
            None => on_status(hwnd, Status::Failed),
        },
        // Ghim: giải mã PNG ở luồng nền (như Sao chép ảnh), mở ảnh ghim trên luồng giao diện.
        BTN_PIN => {
            if let Some(p) = e.image_path(&root) {
                let h = hwnd.0 as isize;
                let spawned = std::thread::Builder::new()
                    .name("darkshot-history-pin".into())
                    .spawn(move || match decode_png_file(&p) {
                        Ok(img) => post_pin_image(h, img),
                        Err(_) => post_status(h, Status::Failed),
                    })
                    .is_ok();
                if !spawned {
                    on_status(hwnd, Status::Failed);
                }
            }
        }
        // Xóa khỏi lịch sử (không xác nhận; không đụng tệp của người dùng).
        _ => {
            let id = e.id.clone();
            spawn_task(hwnd, move || match Store::new(root).remove(&id) {
                Ok(_) => Status::Removed,
                Err(_) => Status::Failed,
            });
        }
    }
}

/// Kết quả một hành động: hiện dòng trạng thái 2,5 giây (thông báo mới thay cái cũ); xóa xong thì nạp lại.
fn on_status(hwnd: HWND, st: Status) {
    if matches!(st, Status::Removed | Status::Cleared) {
        start_load(hwnd);
    }
    if st.msg().is_none() {
        return;
    }
    let Some(r) = with_state(|s| {
        s.status = Some(st);
        s.status_rect().0
    }) else {
        return;
    };
    // SAFETY: hẹn giờ của chính cửa sổ này (đặt lại thì dời mốc); chỉ đánh dấu vùng cần vẽ lại.
    unsafe {
        SetTimer(Some(hwnd), STATUS_TIMER, STATUS_MS, None);
        let _ = InvalidateRect(Some(hwnd), Some(&r), false);
    }
}

// ---------------------------------------------------------------------------
// Menu chuột phải
// ---------------------------------------------------------------------------

/// Menu chuột phải cho mục đang chọn tại điểm màn hình `pt`.
fn show_context_menu(hwnd: HWND, pt: POINT) {
    let Some(acts) = with_state(|s| s.sel_entry().map(|(e, ok)| actions_for(e, ok))).flatten()
    else {
        return;
    };
    let items = menu_items(&acts);
    // SAFETY: menu tạo ở đây được hủy trên mọi đường đi; chuỗi sống đến hết AppendMenuW (menu tự chép).
    let cmd = unsafe {
        let Ok(menu) = CreatePopupMenu() else {
            return;
        };
        for it in &items {
            if it.separator_before {
                let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
            }
            let label = match it.shortcut {
                Some(k) => format!("{}\t{k}", lang::tr(it.label)),
                None => lang::tr(it.label).to_string(),
            };
            let flags = if it.enabled {
                MF_STRING
            } else {
                MF_STRING | MF_GRAYED
            };
            let _ = AppendMenuW(
                menu,
                flags,
                ACTION_IDS[it.action] as usize,
                &HSTRING::from(label),
            );
        }
        let cmd = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON,
            pt.x,
            pt.y,
            Some(0),
            hwnd,
            None,
        );
        let _ = DestroyMenu(menu);
        cmd.0
    };
    if ACTION_IDS.contains(&cmd) {
        on_action(cmd);
    }
}

/// `WM_CONTEXTMENU` của danh sách: chuột thì chọn dòng dưới con trỏ; bàn phím (`lp` = -1) thì mở ở
/// dòng đang chọn.
fn on_list_context_menu(list: HWND, lp: LPARAM) {
    let Some(hwnd) = with_state(|s| s.hwnd) else {
        return;
    };
    let mut pt;
    if lp.0 as i32 == -1 {
        let Some(r) = with_state(|s| s.sel_pos().map(|p| s.layout.row_rect(p, s.scroll))).flatten()
        else {
            return;
        };
        pt = POINT {
            x: r.left + scale(24, draw_info().dpi),
            y: r.bottom.min(scale(48, draw_info().dpi) + r.top),
        };
        // SAFETY: đổi tọa độ client của danh sách sang màn hình.
        unsafe {
            let _ = ClientToScreen(list, &mut pt);
        }
    } else {
        pt = point_of(lp);
        let mut cp = pt;
        // SAFETY: đổi tọa độ màn hình sang client của danh sách.
        unsafe {
            let _ = ScreenToClient(list, &mut cp);
        }
        match row_at(cp) {
            Some(p) => select_pos(p),
            None => return,
        }
    }
    show_context_menu(hwnd, pt);
}

// ---------------------------------------------------------------------------
// Thẻ xác nhận "Xóa toàn bộ lịch sử?"
// ---------------------------------------------------------------------------

fn confirm_is_open() -> bool {
    with_state(|s| s.confirm.is_open()).unwrap_or(false)
}

/// Mở thẻ xác nhận: mọi điều khiển phía sau bị vô hiệu, tiêu điểm ở "Hủy".
fn open_confirm(hwnd: HWND) {
    let ok = with_state(|s| {
        if s.has_entries() && !s.confirm.is_open() {
            s.confirm.open();
            true
        } else {
            false
        }
    });
    if ok != Some(true) {
        return;
    }
    relayout(hwnd);
    focus_card_button(ConfirmBtn::Cancel);
}

/// Đi bằng bàn phím (Tab, mũi tên) mà ta tự xử lý: hiện khung tiêu điểm như `IsDialogMessageW` làm.
fn show_focus_cues(hwnd: HWND) {
    use windows::Win32::UI::WindowsAndMessaging::{UISF_HIDEFOCUS, UIS_CLEAR, WM_CHANGEUISTATE};
    let wp = (UIS_CLEAR as usize) | ((UISF_HIDEFOCUS as usize) << 16);
    // SAFETY: gửi đồng bộ tới cửa sổ Lịch sử của luồng này.
    unsafe {
        SendMessageW(hwnd, WM_CHANGEUISTATE, Some(WPARAM(wp)), Some(LPARAM(0)));
    }
}

fn focus_card_button(b: ConfirmBtn) {
    let Some(btns) = with_state(|s| {
        s.confirm.set_focus(b);
        s.ctl.card_btns
    }) else {
        return;
    };
    let h = match b {
        ConfirmBtn::Cancel => btns[0],
        ConfirmBtn::Yes => btns[1],
    };
    // SAFETY: đưa tiêu điểm tới nút của thẻ (con của cửa sổ này).
    unsafe {
        let _ = SetFocus(Some(h));
    }
}

/// Đóng thẻ; `yes` = xóa toàn bộ ở luồng nền. Tiêu điểm về nút "Xóa lịch sử" (vô hiệu sau khi xóa
/// xong thì `fix_focus` chuyển tiếp).
fn close_confirm(hwnd: HWND, yes: bool) {
    let Some((root, clear_all)) = with_state(|s| {
        s.confirm.close();
        (s.root.clone(), s.ctl.clear_all)
    }) else {
        return;
    };
    relayout(hwnd);
    // SAFETY: đưa tiêu điểm về nút "Xóa lịch sử" (con của cửa sổ này).
    unsafe {
        let _ = SetFocus(Some(clear_all));
    }
    if yes {
        spawn_task(hwnd, move || match Store::new(root).clear() {
            Ok(_) => Status::Cleared,
            Err(_) => Status::Failed,
        });
    }
}

/// Phím khi thẻ đang mở: Esc = Hủy, Enter = nút đang có tiêu điểm, Tab chỉ xoay trong hai nút.
/// `true` = đã xử lý.
fn confirm_key(hwnd: HWND, vk: u16, shift: bool) -> bool {
    let key = match vk {
        v if v == VK_ESCAPE.0 => ConfirmKey::Esc,
        v if v == VK_RETURN.0 => ConfirmKey::Enter,
        v if v == VK_TAB.0 && shift => ConfirmKey::ShiftTab,
        v if v == VK_TAB.0 => ConfirmKey::Tab,
        _ => return false,
    };
    // SAFETY: chỉ hỏi cửa sổ có tiêu điểm.
    let f = unsafe { GetFocus() };
    let out = with_state(|s| {
        if f == s.ctl.card_btns[1] {
            s.confirm.set_focus(ConfirmBtn::Yes);
        } else if f == s.ctl.card_btns[0] {
            s.confirm.set_focus(ConfirmBtn::Cancel);
        }
        let out = s.confirm.key(key);
        (out, s.confirm.focus())
    });
    if matches!(key, ConfirmKey::Tab | ConfirmKey::ShiftTab) {
        show_focus_cues(hwnd);
    }
    match out {
        Some((ConfirmOutcome::Stay, b)) => focus_card_button(b),
        Some((ConfirmOutcome::Cancel, _)) => close_confirm(hwnd, false),
        Some((ConfirmOutcome::Confirm, _)) => close_confirm(hwnd, true),
        None => {}
    }
    true
}

fn paint_card(card: HWND) {
    let lang = lang::current();
    let mut ps = PAINTSTRUCT::default();
    // SAFETY: BeginPaint/EndPaint cặp đôi trong WM_PAINT của thẻ.
    let hdc = unsafe { BeginPaint(card, &mut ps) };
    with_state(|s| {
        let Some(cr) = s.card else {
            return;
        };
        let k = cr.card;
        let rel = |r: &RECT| RECT {
            left: r.left - k.left,
            top: r.top - k.top,
            right: r.right - k.left,
            bottom: r.bottom - k.top,
        };
        let body = darkshot_core::i18n::history_clear_confirm_body(lang, s.entries.len());
        let title = darkshot_core::i18n::t(lang, Msg::HistoryClearConfirmTitle);
        let all = rel(&k);
        theme::buffered(hdc, &all, |dc| {
            hp::draw_card(
                dc,
                &all,
                &rel(&cr.title),
                &rel(&cr.text),
                title,
                &body,
                &s.fonts.0,
                s.dpi,
            );
        });
    });
    // SAFETY: kết thúc lần vẽ đã mở ở trên.
    unsafe {
        let _ = EndPaint(card, &ps);
    }
}

unsafe extern "system" fn card_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            paint_card(hwnd);
            LRESULT(0)
        }
        // Nút của thẻ báo về cửa sổ chính.
        // SAFETY: chuyển nguyên tham số cho cửa sổ cha (cửa sổ Lịch sử của luồng này).
        WM_COMMAND => unsafe {
            match GetParent(hwnd) {
                Ok(p) => SendMessageW(p, msg, Some(wp), Some(lp)),
                Err(_) => LRESULT(0),
            }
        },
        WM_NOTIFY => {
            // SAFETY: lp của WM_NOTIFY trỏ tới NMHDR hợp lệ suốt lời gọi.
            let hdr = unsafe { (lp.0 as *const NMHDR).as_ref() };
            if let Some(h) = hdr.filter(|h| h.code == NM_CUSTOMDRAW) {
                let d = draw_info();
                // SAFETY: mã NM_CUSTOMDRAW nên lp trỏ tới NMCUSTOMDRAW.
                if let Some(cd) = unsafe { theme::nmcd_of(lp) } {
                    match h.idFrom as i32 {
                        ID_CARD_CANCEL => {
                            return theme::custom_draw_button(
                                cd,
                                ButtonKind::Normal,
                                SURFACE,
                                d.dpi,
                                None,
                            )
                        }
                        ID_CARD_YES => {
                            return theme::custom_draw_button(
                                cd,
                                ButtonKind::Danger,
                                SURFACE,
                                d.dpi,
                                d.bold,
                            )
                        }
                        _ => {}
                    }
                }
            }
            // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
            unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
        }
        // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
        _ => unsafe { DefWindowProcW(hwnd, msg, wp, lp) },
    }
}

// ---------------------------------------------------------------------------
// Ô tìm và tooltip
// ---------------------------------------------------------------------------

/// Ô tìm: không lề chữ (cửa sổ cha vẽ khung), giới hạn độ dài, subclass vẽ chữ gợi ý.
fn setup_search(search: HWND) {
    use windows::Win32::UI::Controls::EM_LIMITTEXT;
    use windows::Win32::UI::WindowsAndMessaging::{EC_LEFTMARGIN, EC_RIGHTMARGIN};
    // SAFETY: cấu hình ô EDIT con vừa tạo ở luồng này; subclass gỡ ở WM_NCDESTROY của chính nó.
    unsafe {
        SendMessageW(
            search,
            windows::Win32::UI::Controls::EM_SETMARGINS,
            Some(WPARAM((EC_LEFTMARGIN | EC_RIGHTMARGIN) as usize)),
            Some(LPARAM(0)),
        );
        SendMessageW(search, EM_LIMITTEXT, Some(WPARAM(256)), None);
        let _ = SetWindowSubclass(search, Some(search_proc), SEARCH_SUBCLASS, 0);
    }
    theme::set_dark_control_theme(search, theme::DarkClass::Explorer);
}

/// Ô tìm trống và không có tiêu điểm: tự vẽ chữ gợi ý `TEXT_MUTED` (chữ gợi ý hệ thống
/// `EM_SETCUEBANNER` dùng màu xám cố định không theo nền tối).
unsafe extern "system" fn search_proc(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    match msg {
        // SAFETY: chỉ đọc độ dài chữ và cửa sổ có tiêu điểm.
        WM_PAINT if unsafe { GetWindowTextLengthW(hwnd) == 0 && GetFocus() != hwnd } => {
            let mut ps = PAINTSTRUCT::default();
            // SAFETY: BeginPaint/EndPaint cặp đôi trong WM_PAINT của ô tìm.
            let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
            let mut rc = RECT::default();
            // SAFETY: rc là biến cục bộ; chỉ hỏi trạng thái của ô.
            let enabled = unsafe {
                let _ = GetClientRect(hwnd, &mut rc);
                IsWindowEnabled(hwnd).as_bool()
            };
            let font = theme::control_font(hwnd);
            theme::buffered(hdc, &rc, |dc| {
                hp::draw_placeholder(
                    dc,
                    &rc,
                    lang::tr(Msg::HistorySearchPlaceholder),
                    font,
                    enabled,
                );
            });
            // SAFETY: kết thúc lần vẽ đã mở ở trên.
            unsafe {
                let _ = EndPaint(hwnd, &ps);
            }
            LRESULT(0)
        }
        WM_SETFOCUS | WM_KILLFOCUS => {
            // SAFETY: chuyển tiếp trong thủ tục subclass rồi vẽ lại (chữ gợi ý hiện/ẩn).
            unsafe {
                let r = DefSubclassProc(hwnd, msg, wp, lp);
                let _ = InvalidateRect(Some(hwnd), None, true);
                r
            }
        }
        WM_NCDESTROY => {
            // SAFETY: gỡ subclass của chính ô này trước khi nó bị hủy.
            unsafe {
                let _ = RemoveWindowSubclass(hwnd, Some(search_proc), SEARCH_SUBCLASS);
                DefSubclassProc(hwnd, msg, wp, lp)
            }
        }
        // SAFETY: chuyển tiếp trong thủ tục subclass.
        _ => unsafe { DefSubclassProc(hwnd, msg, wp, lp) },
    }
}

fn tool_info(owner: HWND, flags: u32, id: usize, rect: RECT, text: &mut [u16]) -> TTTOOLINFOW {
    TTTOOLINFOW {
        cbSize: std::mem::size_of::<TTTOOLINFOW>() as u32,
        uFlags: windows::Win32::UI::Controls::TOOLTIP_FLAGS(flags),
        hwnd: owner,
        uId: id,
        rect,
        lpszText: PWSTR(text.as_mut_ptr()),
        ..Default::default()
    }
}

fn wide_z(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Tooltip tối của cửa sổ: nút `×` (nhãn đầy đủ), đường dẫn và liên kết bị cắt (gắn sau, theo lần vẽ).
/// Là cửa sổ được sở hữu nên bị hủy cùng cửa sổ Lịch sử.
fn create_tooltip(owner: HWND, clear_btn: HWND, dpi: u32) -> HWND {
    let icc = INITCOMMONCONTROLSEX {
        dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
        dwICC: ICC_BAR_CLASSES,
    };
    // SAFETY: đăng ký lớp tooltip (lặp lại vô hại) và tạo cửa sổ tooltip thuộc cửa sổ này; mọi chuỗi
    // sống đến hết lời gọi (tooltip tự chép chữ).
    unsafe {
        let _ = InitCommonControlsEx(&icc);
        let Ok(tt) = CreateWindowExW(
            WS_EX_TOPMOST,
            TOOLTIPS_CLASSW,
            &HSTRING::new(),
            WINDOW_STYLE(WS_POPUP.0 | TTS_ALWAYSTIP | TTS_NOPREFIX),
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            Some(owner),
            None,
            None,
            None,
        ) else {
            return HWND::default();
        };
        theme::set_dark_control_theme(tt, theme::DarkClass::Explorer);
        SendMessageW(
            tt,
            TTM_SETMAXTIPWIDTH,
            Some(WPARAM(0)),
            Some(LPARAM(scale(560, dpi) as isize)),
        );
        let mut t = wide_z(lang::tr(Msg::HistorySearchClear));
        let ti = tool_info(
            owner,
            TTF_IDISHWND.0 | TTF_SUBCLASS.0,
            clear_btn.0 as usize,
            RECT::default(),
            &mut t,
        );
        SendMessageW(
            tt,
            TTM_ADDTOOLW,
            None,
            Some(LPARAM(&ti as *const _ as isize)),
        );
        let _ = TIP_CLEAR;
        for id in [hp::TIP_PATH, hp::TIP_LINK] {
            let mut e = wide_z("");
            let ti = tool_info(owner, TTF_SUBCLASS.0, id, RECT::default(), &mut e);
            SendMessageW(
                tt,
                TTM_ADDTOOLW,
                None,
                Some(LPARAM(&ti as *const _ as isize)),
            );
        }
        tt
    }
}

/// Gắn tooltip cho đường dẫn và liên kết đang bị cắt (vùng rỗng, chữ rỗng = không hiện).
fn set_tips(tt: HWND, owner: HWND, tips: &[Tip]) {
    if tt.is_invalid() {
        return;
    }
    for id in [hp::TIP_PATH, hp::TIP_LINK] {
        let tip = tips.iter().find(|t| t.id == id);
        let mut text = wide_z(tip.map_or("", |t| t.text.as_str()));
        let ti = tool_info(
            owner,
            TTF_SUBCLASS.0,
            id,
            tip.map_or(RECT::default(), |t| t.rect),
            &mut text,
        );
        // SAFETY: ti và chữ sống đến hết lời gọi đồng bộ; tooltip tự chép.
        unsafe {
            SendMessageW(
                tt,
                TTM_NEWTOOLRECTW,
                None,
                Some(LPARAM(&ti as *const _ as isize)),
            );
            SendMessageW(
                tt,
                TTM_UPDATETIPTEXTW,
                None,
                Some(LPARAM(&ti as *const _ as isize)),
            );
        }
    }
}

/// Nút "Mở Tùy chọn": nhờ cửa sổ chính mở Tùy chọn như mục menu khay.
fn open_options(hwnd: HWND) {
    // SAFETY: chỉ hỏi cửa sổ chủ.
    if let Ok(owner) = unsafe { GetWindow(hwnd, GW_OWNER) } {
        crate::post_open_options(owner);
    }
}

// ---------------------------------------------------------------------------
// Vẽ
// ---------------------------------------------------------------------------

fn paint_main(hwnd: HWND) {
    let lang = lang::current();
    let today = paths::local_now();
    // Hỏi trước khi khóa STATE.
    // SAFETY: chỉ hỏi cửa sổ có tiêu điểm.
    let focus = unsafe { GetFocus() };
    let mut ps = PAINTSTRUCT::default();
    // SAFETY: BeginPaint/EndPaint cặp đôi trong WM_PAINT của chính cửa sổ này.
    let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
    let mut client = RECT::default();
    // SAFETY: client là biến cục bộ.
    unsafe {
        let _ = GetClientRect(hwnd, &mut client);
    }
    let tips = with_state(|s| {
        let mut tips = Vec::new();
        let search_focus = focus == s.ctl.search;
        let modal = s.confirm.is_open();
        theme::buffered(hdc, &ps.rcPaint, |dc| {
            theme::fill(dc, &client, BG);
            let l = &s.layout;
            let fonts = &s.fonts.0;
            let t = |m: Msg| darkshot_core::i18n::t(lang, m);
            let parts = search_parts(&l.search, l.dpi, s.text_h);
            hp::draw_search_field(dc, l, &parts, search_focus && !modal, !modal);
            if s.loaded {
                if s.entries.is_empty() {
                    hp::draw_separators(dc, l, false);
                    if s.enabled {
                        hp::draw_empty(
                            dc,
                            l,
                            fonts,
                            t(Msg::HistoryEmptyTitle),
                            t(Msg::HistoryEmptyBody),
                            None,
                        );
                    } else {
                        let bw = button_width(fonts.bold, t(Msg::HistoryOpenOptions), l.dpi);
                        hp::draw_empty(
                            dc,
                            l,
                            fonts,
                            t(Msg::HistoryDisabledTitle),
                            t(Msg::HistoryDisabledBody),
                            Some(bw),
                        );
                    }
                } else {
                    if s.banner() {
                        hp::draw_banner(dc, l, fonts, lang);
                    }
                    if s.no_match() {
                        hp::draw_separators(dc, l, false);
                        let bw = button_width(fonts.normal, t(Msg::HistoryClearFilters), l.dpi);
                        hp::draw_empty(
                            dc,
                            l,
                            fonts,
                            t(Msg::HistoryNoMatchTitle),
                            t(Msg::HistoryNoMatchBody),
                            Some(bw),
                        );
                    } else {
                        hp::draw_separators(dc, l, true);
                        let view = match &s.preview {
                            Preview::Idle => PreviewView::Idle,
                            Preview::Loading => PreviewView::Loading,
                            Preview::Ready(b) => PreviewView::Ready(b),
                            Preview::Missing => PreviewView::Missing,
                            Preview::Unreadable => PreviewView::Unreadable,
                        };
                        tips = hp::draw_detail(dc, l, s.sel_entry(), view, fonts, lang, &today);
                    }
                }
                if let Some(st) = s.status {
                    if let Some(m) = st.msg() {
                        let (r, center) = s.status_rect();
                        hp::draw_status(dc, &r, t(m), st.is_error(), fonts.normal, center);
                    }
                }
            } else {
                hp::draw_separators(dc, l, false);
            }
            if modal {
                hp::dim(dc, &l.body, DIM_ALPHA);
            }
        });
        let changed = tips != s.tips;
        if changed {
            s.tips = tips.clone();
        }
        changed.then_some((s.ctl.tooltip, tips))
    })
    .flatten();
    // SAFETY: kết thúc lần vẽ đã mở ở trên.
    unsafe {
        let _ = EndPaint(hwnd, &ps);
    }
    if let Some((tt, tips)) = tips {
        set_tips(tt, hwnd, &tips);
    }
}

fn paint_list(list: HWND) {
    let lang = lang::current();
    let now = paths::local_now();
    // Hỏi trước khi khóa STATE: các hàm này gửi thông điệp tới chính danh sách.
    let (show_focus, _) = theme::ui_cues(list);
    // SAFETY: chỉ hỏi cửa sổ có tiêu điểm.
    let focused = unsafe { GetFocus() } == list && show_focus;
    let mut ps = PAINTSTRUCT::default();
    // SAFETY: BeginPaint/EndPaint cặp đôi trong WM_PAINT của danh sách.
    let hdc = unsafe { BeginPaint(list, &mut ps) };
    let mut client = RECT::default();
    // SAFETY: client là biến cục bộ.
    unsafe {
        let _ = GetClientRect(list, &mut client);
    }
    with_state(|s| {
        let mut jobs = Vec::new();
        let sel = s.sel_pos();
        let rows = s
            .layout
            .rows_in(ps.rcPaint.top, ps.rcPaint.bottom, s.scroll, s.visible.len());
        let cell = (
            s.layout.thumb.0.max(1) as u32,
            s.layout.thumb.1.max(1) as u32,
        );
        let gen = s.thumb_gen.current();
        let dim = s.confirm.is_open();
        // Nhờ nạp ảnh nhỏ còn thiếu của các dòng sắp vẽ (không nạp lặp).
        for pos in rows.clone() {
            let i = s.visible[pos];
            let (e, ok) = (&s.entries[i], s.exists.get(i).copied().unwrap_or(false));
            if ok && s.thumbs.request(&e.id) {
                jobs.push(ThumbJob {
                    gen,
                    entry: e.clone(),
                    cell,
                });
            }
        }
        let State {
            entries,
            exists,
            visible,
            thumbs,
            layout: l,
            fonts,
            scroll,
            hover,
            dpi,
            ..
        } = s;
        theme::buffered(hdc, &ps.rcPaint, |dc| {
            theme::fill(dc, &client, BG);
            for pos in rows {
                let i = visible[pos];
                let e = &entries[i];
                let ok = exists.get(i).copied().unwrap_or(false);
                let thumb = match thumbs.get(&e.id) {
                    Some(ThumbSlot::Ready(b)) => ThumbView::Ready(b),
                    Some(ThumbSlot::Broken) => ThumbView::Broken,
                    _ => ThumbView::Pending,
                };
                let mut r = l.row_rect(pos, *scroll);
                r.right = client.right;
                let v = RowView {
                    entry: e,
                    exists: ok,
                    selected: sel == Some(pos),
                    hover: *hover == Some(pos),
                    focused,
                    thumb,
                };
                hp::draw_row(dc, &r, &v, &fonts.0, lang, &now, *dpi);
            }
            if focused && sel.is_none() && !visible.is_empty() {
                theme::focus_frame(dc, &client, *dpi);
            }
            if dim {
                hp::dim(dc, &client, DIM_ALPHA);
            }
        });
        for j in jobs {
            s.workers.thumb(j);
        }
    });
    // SAFETY: kết thúc lần vẽ đã mở ở trên.
    unsafe {
        let _ = EndPaint(list, &ps);
    }
}

// ---------------------------------------------------------------------------
// Thủ tục cửa sổ
// ---------------------------------------------------------------------------

fn lo_word(v: usize) -> u32 {
    (v & 0xFFFF) as u32
}

fn hi_word_i16(v: usize) -> i16 {
    ((v >> 16) & 0xFFFF) as u16 as i16
}

fn point_of(lp: LPARAM) -> POINT {
    POINT {
        x: (lp.0 & 0xFFFF) as u16 as i16 as i32,
        y: ((lp.0 >> 16) & 0xFFFF) as u16 as i16 as i32,
    }
}

/// Dòng hiển thị dưới điểm `pt` (tọa độ client danh sách).
fn row_at(pt: POINT) -> Option<usize> {
    with_state(|s| {
        let p = POINT {
            x: pt.x + s.layout.list.left,
            y: pt.y + s.layout.list.top,
        };
        match s.layout.hit_test(p, s.scroll, s.visible.len()) {
            crate::history_layout::Hit::Row(i) => Some(i),
            _ => None,
        }
    })
    .flatten()
}

fn invalidate_row(list: HWND, pos: Option<usize>) {
    let Some(pos) = pos else {
        return;
    };
    if let Some(mut r) = with_state(|s| s.layout.row_rect(pos, s.scroll)) {
        r.right = i32::MAX / 2;
        // SAFETY: chỉ đánh dấu dòng cần vẽ lại.
        unsafe {
            let _ = InvalidateRect(Some(list), Some(&r), false);
        }
    }
}

unsafe extern "system" fn list_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            paint_list(hwnd);
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        // Danh sách nhận phím mũi tên (và chữ) dù `IsDialogMessageW` đang lo Tab.
        WM_GETDLGCODE => LRESULT((DLGC_WANTARROWS | DLGC_WANTCHARS) as isize),
        WM_SETFOCUS | WM_KILLFOCUS | WM_UPDATEUISTATE => {
            // SAFETY: chỉ đánh dấu vẽ lại rồi để xử lý mặc định.
            unsafe {
                let _ = InvalidateRect(Some(hwnd), None, false);
                DefWindowProcW(hwnd, msg, wp, lp)
            }
        }
        WM_KEYDOWN => {
            let vk = wp.0 as u16;
            // SAFETY: chỉ đọc trạng thái phím của luồng này.
            let (ctrl, shift) = unsafe {
                (
                    GetKeyState(VK_CONTROL.0 as i32) < 0,
                    GetKeyState(VK_SHIFT.0 as i32) < 0,
                )
            };
            // Ctrl+C = sao chép ảnh, Ctrl+Shift+C = sao chép liên kết, Delete = xóa khỏi lịch sử.
            if ctrl && vk == u16::from(b'C') {
                on_action(if shift { 203 } else { 202 });
                return LRESULT(0);
            }
            if vk == VK_DELETE.0 && !ctrl && !shift {
                on_action(ID_REMOVE);
                return LRESULT(0);
            }
            let Some(nav) = nav_of(vk) else {
                // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
                return unsafe { DefWindowProcW(hwnd, msg, wp, lp) };
            };
            let next = with_state(|s| {
                step_selection(s.sel_pos(), s.visible.len(), nav, s.layout.visible_rows())
            })
            .flatten();
            if let Some(p) = next {
                select_pos(p);
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            // SAFETY: đưa tiêu điểm vào danh sách (không giữ khóa STATE).
            unsafe {
                let _ = SetFocus(Some(hwnd));
            }
            if let Some(p) = row_at(point_of(lp)) {
                select_pos(p);
            }
            LRESULT(0)
        }
        WM_LBUTTONDBLCLK => {
            // Nháy đúp dòng = Mở.
            if let Some(p) = row_at(point_of(lp)) {
                select_pos(p);
                on_action(ID_OPEN);
            }
            LRESULT(0)
        }
        WM_CONTEXTMENU => {
            on_list_context_menu(hwnd, lp);
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let row = row_at(point_of(lp));
            let change = with_state(|s| {
                let old = std::mem::replace(&mut s.hover, row);
                let track = !s.tracking;
                s.tracking = true;
                (old, track)
            });
            if let Some((old, track)) = change {
                if track {
                    let mut tme = TRACKMOUSEEVENT {
                        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    };
                    // SAFETY: tme là biến cục bộ đủ cbSize; theo dõi chính cửa sổ này.
                    unsafe {
                        let _ = TrackMouseEvent(&mut tme);
                    }
                }
                if old != row {
                    invalidate_row(hwnd, old);
                    invalidate_row(hwnd, row);
                }
            }
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            if let Some(old) = with_state(|s| {
                s.tracking = false;
                s.hover.take()
            }) {
                invalidate_row(hwnd, old);
            }
            LRESULT(0)
        }
        WM_SETCURSOR if lo_word(lp.0 as usize) == HTCLIENT => {
            let mut pt = POINT::default();
            // SAFETY: pt là biến cục bộ; đổi tọa độ sang client của chính danh sách.
            unsafe {
                let _ = GetCursorPos(&mut pt);
                let _ = ScreenToClient(hwnd, &mut pt);
            }
            if row_at(pt).is_some() {
                // SAFETY: con trỏ hệ thống dùng chung, không phải hủy.
                unsafe {
                    if let Ok(c) = LoadCursorW(None, IDC_HAND) {
                        SetCursor(Some(c));
                    }
                }
                return LRESULT(1);
            }
            // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
            unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
        }
        WM_MOUSEWHEEL => {
            let delta = i32::from(hi_word_i16(wp.0));
            if let Some((scroll, line)) = with_state(|s| (s.scroll, scale(LINE_DIP, s.dpi))) {
                scroll_to(hwnd, scroll - delta * WHEEL_LINES * line / 120);
            }
            LRESULT(0)
        }
        WM_VSCROLL => {
            let code = SCROLLBAR_COMMAND(lo_word(wp.0) as i32);
            let Some((scroll, line, page, max)) = with_state(|s| {
                let view = s.layout.list.bottom - s.layout.list.top;
                (
                    s.scroll,
                    scale(LINE_DIP, s.dpi),
                    (view - scale(LINE_DIP, s.dpi)).max(1),
                    s.layout.content_h(s.visible.len()),
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
                    // SAFETY: si là biến cục bộ đủ cbSize; hỏi thanh cuộn của chính cửa sổ này.
                    unsafe {
                        let _ = GetScrollInfo(hwnd, SB_VERT, &mut si);
                    }
                    si.nTrackPos
                }
                _ => scroll,
            };
            scroll_to(hwnd, target);
            LRESULT(0)
        }
        // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
        _ => unsafe { DefWindowProcW(hwnd, msg, wp, lp) },
    }
}

/// `WM_COMMAND` của cửa sổ chính (nút, ô tìm, Enter qua `IsDialogMessageW`).
fn on_command(hwnd: HWND, id: i32, code: u32) {
    if id == ID_SEARCH {
        match code {
            EN_CHANGE => apply_filter(hwnd),
            EN_SETFOCUS | EN_KILLFOCUS => {
                // Gạch dưới ACCENT của ô tìm theo tiêu điểm.
                if let Some(r) = with_state(|s| s.layout.search) {
                    // SAFETY: chỉ đánh dấu vùng cần vẽ lại.
                    unsafe {
                        let _ = InvalidateRect(Some(hwnd), Some(&r), false);
                    }
                }
            }
            _ => {}
        }
        return;
    }
    if code != 0 {
        return;
    }
    let modal = confirm_is_open();
    match id {
        ID_CARD_CANCEL => close_confirm(hwnd, false),
        ID_CARD_YES => close_confirm(hwnd, true),
        _ if modal => {}
        // Enter (`IsDialogMessageW` gửi IDOK): ở danh sách = Mở; ở một nút = bấm nút đó.
        x if x == IDOK.0 => {
            // SAFETY: chỉ hỏi cửa sổ có tiêu điểm.
            let f = unsafe { GetFocus() };
            let Some(c) = with_state(|s| s.ctl) else {
                return;
            };
            if f == c.list {
                on_action(ID_OPEN);
            } else if f != c.search && !f.is_invalid() && c.behind().contains(&f) {
                // SAFETY: bấm nút con đang có tiêu điểm của cửa sổ này.
                unsafe {
                    SendMessageW(f, BM_CLICK, None, None);
                }
            }
        }
        x if ACTION_IDS.contains(&x) => on_action(x),
        ID_OPEN_OPTIONS => open_options(hwnd),
        ID_CLEAR_ALL => open_confirm(hwnd),
        ID_SEARCH_CLEAR => {
            clear_search();
            if let Some(search) = with_state(|s| s.ctl.search) {
                // SAFETY: đưa tiêu điểm về ô tìm (con của cửa sổ này).
                unsafe {
                    let _ = SetFocus(Some(search));
                }
            }
        }
        ID_CLEAR_FILTERS => {
            with_state(|s| s.filter = Filter::All);
            clear_search();
            apply_filter(hwnd);
            if let Some(list) = with_state(|s| s.ctl.list) {
                // SAFETY: đưa tiêu điểm về danh sách (con của cửa sổ này).
                unsafe {
                    let _ = SetFocus(Some(list));
                }
            }
        }
        x if CHIP_IDS.contains(&x) => {
            if let Some(f) = chip_filter((x - CHIP_IDS[0]) as usize) {
                set_filter(hwnd, f);
            }
        }
        _ => {}
    }
}

/// `NM_CUSTOMDRAW` của các điều khiển con tự vẽ; `None` = để mặc định. Khi thẻ xác nhận mở, điều
/// khiển trong thân bị phủ mờ như phần thân.
fn custom_draw(id: i32, cd: &windows::Win32::UI::Controls::NMCUSTOMDRAW) -> Option<LRESULT> {
    let d = draw_info();
    let r = match id {
        201 => theme::custom_draw_button(cd, ButtonKind::Primary, BG, d.dpi, d.bold),
        202..=204 | ID_PIN | ID_CLEAR_FILTERS => {
            theme::custom_draw_button(cd, ButtonKind::Normal, BG, d.dpi, None)
        }
        ID_REMOVE => hp::custom_draw_danger(cd, BG, d.dpi, false),
        ID_CLEAR_ALL => hp::custom_draw_danger(cd, BG, d.dpi, true),
        ID_OPEN_OPTIONS if d.in_banner => {
            theme::custom_draw_button(cd, ButtonKind::Normal, SURFACE, d.dpi, None)
        }
        ID_OPEN_OPTIONS => theme::custom_draw_button(cd, ButtonKind::Primary, BG, d.dpi, d.bold),
        ID_SEARCH_CLEAR => hp::custom_draw_search_clear(cd, d.dpi),
        x if CHIP_IDS.contains(&x) => {
            hp::custom_draw_chip(cd, (x - CHIP_IDS[0]) as usize == d.chip, d.dpi)
        }
        _ => return None,
    };
    let in_body = ACTION_IDS.contains(&id)
        || id == ID_CLEAR_FILTERS
        || (id == ID_OPEN_OPTIONS && !d.in_banner);
    if d.dim && in_body && cd.dwDrawStage == windows::Win32::UI::Controls::CDDS_PREPAINT {
        hp::dim(cd.hdc, &cd.rc, DIM_ALPHA);
    }
    Some(r)
}

/// Đổi DPI: phông mới cho điều khiển, đệm ảnh nhỏ làm lại theo cỡ ô mới.
fn on_dpi_changed(hwnd: HWND, dpi: u32) {
    let fonts = FontSet::new(dpi);
    let normal = fonts.0.normal;
    let bold = fonts.0.bold;
    let text_h = hp::with_screen_dc(|dc| hp::measure(dc, normal, "Ag").1);
    let Some((old, controls)) = with_state(|s| {
        s.dpi = dpi;
        s.text_h = text_h;
        drop(s.thumbs.clear());
        s.thumb_gen.next();
        let old = std::mem::replace(&mut s.fonts, fonts);
        let mut c = s.ctl.behind();
        c.extend(s.ctl.card_btns);
        c.retain(|h| *h != s.ctl.list);
        (old, c)
    }) else {
        return;
    };
    update_draw(|d| {
        d.dpi = dpi;
        d.bold = bold;
        d.normal = normal;
    });
    for c in controls {
        set_font(c, normal);
    }
    // Các nút đã dùng phông mới: phông cũ xóa được.
    drop(old);
    let _ = hwnd;
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_CLOSE => {
            // SAFETY: hủy chính cửa sổ này; dọn dẹp ở WM_NCDESTROY.
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            paint_main(hwnd);
            LRESULT(0)
        }
        WM_SIZE => {
            relayout(hwnd);
            if with_state(|s| s.preview_can_grow()) == Some(true) {
                // SAFETY: hẹn giờ của chính cửa sổ này; đặt lại thì chỉ dời mốc.
                unsafe {
                    SetTimer(Some(hwnd), PREVIEW_TIMER, PREVIEW_REFRESH_MS, None);
                }
            }
            LRESULT(0)
        }
        WM_TIMER if wp.0 == STATUS_TIMER => {
            // SAFETY: hủy hẹn giờ một lần của chính cửa sổ này.
            unsafe {
                let _ = KillTimer(Some(hwnd), STATUS_TIMER);
            }
            if let Some(r) = with_state(|s| {
                s.status = None;
                s.status_rect().0
            }) {
                // SAFETY: chỉ đánh dấu vùng cần vẽ lại.
                unsafe {
                    let _ = InvalidateRect(Some(hwnd), Some(&r), false);
                }
            }
            LRESULT(0)
        }
        WM_TIMER if wp.0 == PREVIEW_TIMER => {
            // SAFETY: hủy hẹn giờ một lần của chính cửa sổ này.
            unsafe {
                let _ = KillTimer(Some(hwnd), PREVIEW_TIMER);
            }
            with_state(|s| {
                if s.preview_can_grow() {
                    s.request_preview(true);
                }
            });
            LRESULT(0)
        }
        WM_ACTIVATE => {
            let inactive = lo_word(wp.0) == WA_INACTIVE;
            if inactive {
                // SAFETY: chỉ hỏi cửa sổ có tiêu điểm và quan hệ cha con.
                let f = unsafe { GetFocus() };
                if !f.is_invalid() && unsafe { IsChild(hwnd, f).as_bool() } {
                    with_state(|s| s.last_focus = f);
                }
            }
            // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
            let r = unsafe { DefWindowProcW(hwnd, msg, wp, lp) };
            if !inactive {
                // Quay lại cửa sổ (ví dụ sau khi lưu Tùy chọn): đọc lại cờ bật lịch sử.
                refresh_enabled();
            }
            r
        }
        WM_SETFOCUS => {
            // Cửa sổ chính không nhận phím: trả tiêu điểm cho điều khiển dùng gần nhất.
            if let Some((f, c)) = with_state(|s| (s.last_focus, s.ctl)) {
                // Điều khiển cũ đã ẩn hoặc bị vô hiệu: chọn điều khiển dùng được.
                let f = [f, c.card_btns[0], c.list, c.options, c.search]
                    .into_iter()
                    .find(|h| usable(*h))
                    .unwrap_or(f);
                // SAFETY: f là con của cửa sổ này (hoặc đã hủy: SetFocus báo lỗi vô hại).
                unsafe {
                    if IsWindow(Some(f)).as_bool() {
                        let _ = SetFocus(Some(f));
                    }
                }
            }
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            // Bánh xe khi tiêu điểm ở nút: cuộn danh sách (trừ khi thẻ xác nhận đang mở).
            if let Some(list) = with_state(|s| s.list).filter(|_| !confirm_is_open()) {
                // SAFETY: gửi đồng bộ cho danh sách của cửa sổ này (không giữ khóa STATE).
                return unsafe { SendMessageW(list, msg, Some(wp), Some(lp)) };
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let (id, code) = ((wp.0 & 0xFFFF) as i32, ((wp.0 >> 16) & 0xFFFF) as u32);
            on_command(hwnd, id, code);
            LRESULT(0)
        }
        WM_CTLCOLOREDIT | WM_CTLCOLORSTATIC => {
            // Ô tìm: chữ TEXT (vô hiệu: TEXT_DISABLED) trên nền INPUT.
            let fg = if msg == WM_CTLCOLOREDIT {
                TEXT
            } else {
                TEXT_DISABLED
            };
            theme::ctl_color(HDC(wp.0 as *mut c_void), fg, INPUT)
        }
        WM_NOTIFY => {
            // SAFETY: lp của WM_NOTIFY trỏ tới NMHDR hợp lệ suốt lời gọi.
            let hdr = unsafe { (lp.0 as *const NMHDR).as_ref() };
            if let Some(h) = hdr.filter(|h| h.code == NM_CUSTOMDRAW) {
                // SAFETY: mã NM_CUSTOMDRAW nên lp trỏ tới NMCUSTOMDRAW.
                if let Some(cd) = unsafe { theme::nmcd_of(lp) } {
                    if let Some(r) = custom_draw(h.idFrom as i32, cd) {
                        return r;
                    }
                }
            }
            // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
            unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
        }
        WM_HISTORY_CHANGED => {
            // Cấu hình có thể vừa đổi (bật/tắt lịch sử): cập nhật dải thông báo rồi nạp lại.
            refresh_enabled();
            start_load(hwnd);
            LRESULT(0)
        }
        WM_HISTORY_STATUS => {
            on_status(hwnd, Status::from_wparam(wp.0));
            LRESULT(0)
        }
        WM_HISTORY_LOADED => {
            // SAFETY: con trỏ do post_box cấp phát cho đúng mã thông điệp này, chuyển quyền cho ta.
            let m = unsafe { Box::from_raw(lp.0 as *mut LoadMsg) };
            on_loaded(hwnd, *m);
            LRESULT(0)
        }
        WM_HISTORY_PIN => {
            // SAFETY: Box<Image> do post_pin_image cấp phát, chuyển quyền cho ta.
            let img = unsafe { Box::from_raw(lp.0 as *mut Image) };
            on_pin_image(*img);
            LRESULT(0)
        }
        WM_HISTORY_THUMB => {
            // SAFETY: như trên.
            let m = unsafe { Box::from_raw(lp.0 as *mut ThumbMsg) };
            on_thumb(*m);
            LRESULT(0)
        }
        WM_HISTORY_PREVIEW => {
            // SAFETY: như trên.
            let m = unsafe { Box::from_raw(lp.0 as *mut PreviewMsg) };
            on_preview(*m);
            LRESULT(0)
        }
        WM_GETMINMAXINFO => {
            // SAFETY: hwnd là cửa sổ này (có thể đang được tạo); chỉ hỏi DPI.
            let dpi = norm_dpi(unsafe { GetDpiForWindow(hwnd) });
            let (mw, mh) = min_client_size(dpi);
            let (ww, wh) = window_size(mw, mh, dpi);
            // SAFETY: với WM_GETMINMAXINFO, lp trỏ tới MINMAXINFO hợp lệ suốt lời gọi.
            if let Some(mmi) = unsafe { (lp.0 as *mut MINMAXINFO).as_mut() } {
                mmi.ptMinTrackSize.x = ww;
                mmi.ptMinTrackSize.y = wh;
            }
            LRESULT(0)
        }
        WM_DPICHANGED => {
            on_dpi_changed(hwnd, norm_dpi(lo_word(wp.0)));
            // Tài liệu WM_DPICHANGED: dùng đúng RECT đề nghị trong lp.
            // SAFETY: lp trỏ tới RECT hợp lệ suốt lời gọi.
            if let Some(r) = unsafe { (lp.0 as *const RECT).as_ref() } {
                // SAFETY: đổi vị trí và cỡ chính cửa sổ này (WM_SIZE sẽ tính lại bố cục).
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
            with_state(|s| s.request_preview(true));
            LRESULT(0)
        }
        WM_NCDESTROY => {
            // Quên handle, dù đóng bằng đường nào; các con đã hủy nên phông, bitmap, luồng nền được trả.
            if OPEN.with(|o| o.get()) == hwnd.0 as isize {
                OPEN.with(|o| o.set(0));
                history_service::set_window(0);
                // SAFETY: hủy hẹn giờ (nếu còn) của chính cửa sổ này.
                unsafe {
                    let _ = KillTimer(Some(hwnd), PREVIEW_TIMER);
                    let _ = KillTimer(Some(hwnd), STATUS_TIMER);
                }
                let state = STATE.with(|s| s.borrow_mut().take());
                drop(state);
                drain_results(hwnd);
                DRAW.with(|d| d.set(DrawInfo::new()));
            }
            // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
            unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
        }
        // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
        _ => unsafe { DefWindowProcW(hwnd, msg, wp, lp) },
    }
}

#[cfg(test)]
fn loaded_count() -> Option<usize> {
    with_state(|s| s.loaded.then_some(s.entries.len())).flatten()
}

#[cfg(test)]
fn list_hwnd() -> isize {
    with_state(|s| s.list.0 as isize).unwrap_or(0)
}

#[cfg(test)]
fn pending_thumbs() -> usize {
    with_state(|s| s.thumbs.loading()).unwrap_or(0)
}

#[cfg(test)]
fn preview_loading() -> bool {
    with_state(|s| matches!(s.preview, Preview::Loading)).unwrap_or(false)
}

#[cfg(test)]
fn selected_index() -> Option<usize> {
    with_state(|s| s.sel_pos()).flatten()
}

#[cfg(test)]
fn visible_count() -> usize {
    with_state(|s| s.visible.len()).unwrap_or(0)
}

#[cfg(test)]
fn search_hwnd() -> isize {
    with_state(|s| s.ctl.search.0 as isize).unwrap_or(0)
}

#[cfg(test)]
fn chip_hwnd(k: usize) -> isize {
    with_state(|s| s.ctl.chips[k].0 as isize).unwrap_or(0)
}

#[cfg(test)]
fn current_chip() -> usize {
    with_state(|s| filter_chip(s.filter)).unwrap_or(usize::MAX)
}

#[cfg(test)]
fn clear_hwnd() -> isize {
    with_state(|s| s.ctl.clear_all.0 as isize).unwrap_or(0)
}

/// Mã tooltip đang gắn cho chữ bị cắt trong khung chi tiết (sau lần vẽ gần nhất).
#[cfg(test)]
fn tip_ids() -> Vec<usize> {
    with_state(|s| s.tips.iter().map(|t| t.id).collect()).unwrap_or_default()
}

#[cfg(test)]
fn confirm_open() -> bool {
    confirm_is_open()
}

#[cfg(test)]
fn confirm_buttons() -> (isize, isize) {
    with_state(|s| (s.ctl.card_btns[0].0 as isize, s.ctl.card_btns[1].0 as isize)).unwrap_or((0, 0))
}

/// Chọn mục `id` nếu nó đang hiển thị.
#[cfg(test)]
fn select_id(id: &str) -> bool {
    let pos = with_state(|s| {
        s.visible
            .iter()
            .position(|&i| s.entries.get(i).is_some_and(|e| e.id == id))
    })
    .flatten();
    match pos {
        Some(p) => {
            select_pos(p);
            true
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use windows::core::w;
    use windows::Win32::Foundation::{LPARAM, LRESULT, RECT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Threading::{GetCurrentProcess, GetGuiResources, GR_GDIOBJECTS};
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, FindWindowExW, RegisterClassW,
        WINDOW_EX_STYLE, WNDCLASSW, WS_CAPTION,
    };

    fn rect(l: i32, t: i32, r: i32, b: i32) -> RECT {
        RECT {
            left: l,
            top: t,
            right: r,
            bottom: b,
        }
    }

    #[test]
    fn scrollbar_arrows_follow_card_and_content() {
        // Thẻ mở: luôn vô hiệu, dù nội dung dài hay ngắn.
        assert_eq!(scrollbar_arrows(true, 2000, 500), ScrollArrows::Disabled);
        assert_eq!(scrollbar_arrows(true, 100, 500), ScrollArrows::Disabled);
        // Thẻ đóng, nội dung dài hơn khung nhìn: bật lại.
        assert_eq!(scrollbar_arrows(false, 2000, 500), ScrollArrows::Enabled);
        // Thẻ đóng, vừa khung hoặc ngắn hơn: để hệ thống tự làm mờ.
        assert_eq!(scrollbar_arrows(false, 500, 500), ScrollArrows::Auto);
        assert_eq!(scrollbar_arrows(false, 0, 500), ScrollArrows::Auto);
    }

    #[test]
    fn default_and_min_sizes_scale_with_dpi() {
        assert_eq!(default_client_size(96), (960, 640));
        assert_eq!(default_client_size(120), (1200, 800));
        assert_eq!(default_client_size(144), (1440, 960));
        assert_eq!(default_client_size(192), (1920, 1280));
        assert_eq!(min_client_size(96), (760, 520));
        assert_eq!(min_client_size(144), (1140, 780));
        assert_eq!(min_client_size(192), (1520, 1040));
        // DPI 0 (không hỏi được) coi như 96.
        assert_eq!(default_client_size(0), (960, 640));
        assert_eq!(min_client_size(0), (760, 520));
    }

    #[test]
    fn placement_centers_on_monitor_under_cursor() {
        let m1 = MonitorArea {
            bounds: rect(0, 0, 1920, 1080),
            work: rect(0, 0, 1920, 1040),
        };
        let m2 = MonitorArea {
            bounds: rect(1920, -200, 4480, 1240),
            work: rect(1920, -200, 4480, 1200),
        };
        let mons = [m1, m2];
        // Con trỏ ở màn hình 2: giữa vùng làm việc của nó.
        assert_eq!(
            place_window((2500, 100), &mons, (1000, 700)),
            (1920 + 780, -200 + 350, 1000, 700)
        );
        // Con trỏ ở màn hình 1.
        assert_eq!(
            place_window((10, 10), &mons, (1000, 700)),
            (460, 170, 1000, 700)
        );
        // Con trỏ ngoài mọi màn hình: màn hình gần nhất (màn hình 1).
        assert_eq!(
            place_window((-500, 500), &mons, (1000, 700)),
            (460, 170, 1000, 700)
        );
        // Gần màn hình 2 hơn.
        assert_eq!(
            place_window((5000, 0), &mons, (1000, 700)),
            (1920 + 780, -200 + 350, 1000, 700)
        );
        // Lớn hơn vùng làm việc: thu về vừa vùng làm việc.
        assert_eq!(
            place_window((10, 10), &mons, (2500, 1200)),
            (0, 0, 1920, 1040)
        );
        // Không có danh sách màn hình: đặt ở (100, 100), giữ cỡ.
        assert_eq!(place_window((0, 0), &[], (800, 600)), (100, 100, 800, 600));
    }

    unsafe extern "system" fn proc_(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
        // SAFETY: chuyển nguyên tham số.
        unsafe { DefWindowProcW(h, m, w, l) }
    }

    fn gdi_count() -> u32 {
        // SAFETY: chỉ đọc bộ đếm của tiến trình hiện tại.
        unsafe { GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS) }
    }

    fn count_windows() -> usize {
        let mut n = 0;
        let mut after = None;
        // SAFETY: chỉ dò cửa sổ cấp cao theo tên lớp.
        while let Ok(h) = unsafe { FindWindowExW(None, after, w!("darkshot-history"), None) } {
            n += 1;
            after = Some(h);
        }
        n
    }

    /// Mở, mở lần hai (chỉ đưa ra trước), đóng; 20 vòng, số đối tượng GDI không tăng dần.
    /// Chạy tay: `cargo test -p darkshot-windows -- --ignored history_window`.
    #[test]
    #[ignore]
    fn history_window_opens_and_closes_without_leaks() {
        // SAFETY: cửa sổ cha ẩn riêng của kiểm thử, hủy ở cuối.
        let parent = unsafe {
            let hinst = GetModuleHandleW(None).unwrap();
            let class = w!("darkshot-history-test-parent");
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
        let mut counts = Vec::new();
        for _ in 0..20 {
            show(parent);
            assert!(is_open());
            let first = hwnd();
            assert_ne!(first, 0);
            assert_eq!(count_windows(), 1);
            show(parent);
            assert_eq!(hwnd(), first, "second show must reuse the window");
            assert_eq!(count_windows(), 1, "must stay a single window");
            close_if_open();
            assert!(!is_open());
            assert_eq!(hwnd(), 0);
            assert_eq!(count_windows(), 0, "window must be gone after close");
            counts.push(gdi_count());
        }
        close_if_open(); // không có cửa sổ: không làm gì
        assert!(
            counts[19] <= counts[1],
            "GDI objects grew across opens: {counts:?}"
        );
        // SAFETY: dọn cửa sổ cha.
        unsafe {
            let _ = DestroyWindow(parent);
        }
    }

    fn test_parent(class: PCWSTR) -> HWND {
        // SAFETY: cửa sổ cha ẩn riêng của kiểm thử, bên gọi hủy ở cuối.
        unsafe {
            let hinst = GetModuleHandleW(None).unwrap();
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
        }
    }

    /// Chạy vòng thông điệp của luồng này tới khi `done()` hoặc hết `ms` mili giây.
    fn pump_until(ms: u64, done: impl Fn() -> bool) -> bool {
        use windows::Win32::UI::WindowsAndMessaging::{
            DispatchMessageW, PeekMessageW, TranslateMessage, PM_REMOVE,
        };
        let end = std::time::Instant::now() + std::time::Duration::from_millis(ms);
        loop {
            let mut msg = MSG::default();
            // SAFETY: vòng thông điệp chuẩn của luồng kiểm thử.
            unsafe {
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    if !pre_translate(&msg) {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
            }
            if done() {
                return true;
            }
            if std::time::Instant::now() >= end {
                return false;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    /// Kho tổng hợp `n` mục (ảnh PNG sinh bằng mã) dưới `dir`: đủ bốn loại, mục thứ 4 có tệp đã mất.
    /// Kho giữ tối đa 50 mục mới nhất.
    fn synthetic_store(dir: &std::path::Path, n: usize) -> std::path::PathBuf {
        use darkshot_core::history::Kind;
        use darkshot_core::history_store::{NewEntry, Store};
        use darkshot_core::image::{encode_png, Image};
        use darkshot_core::naming::LocalTime;
        let root = dir.join("history");
        let pics = dir.join("Pictures");
        std::fs::create_dir_all(&pics).unwrap();
        let store = Store::new(root.clone());
        let sizes = [
            (1920, 1080),
            (1280, 720),
            (800, 1000),
            (2560, 1440),
            (640, 360),
            (300, 200),
        ];
        let mut lost = None;
        for i in 0..n {
            let (w, h) = sizes[i % sizes.len()];
            let mut bgra = Vec::with_capacity((w * h * 4) as usize);
            for y in 0..h {
                for x in 0..w {
                    let band = ((x * 6 / w) + i as u32) % 6;
                    let (r, g, b): (u8, u8, u8) = [
                        (0x2F, 0x6F, 0xE4),
                        (0x4A, 0xDE, 0x80),
                        (0xFF, 0x7B, 0x72),
                        (0xE7, 0xE9, 0xEC),
                        (0x33, 0x37, 0x3E),
                        (0xF5, 0xB0, 0x41),
                    ][band as usize];
                    let shade = (y * 60 / h) as u8;
                    bgra.extend_from_slice(&[
                        b.saturating_sub(shade),
                        g.saturating_sub(shade),
                        r.saturating_sub(shade),
                        255,
                    ]);
                }
            }
            let img = Image {
                width: w,
                height: h,
                bgra,
            };
            let kind = [
                Kind::Saved,
                Kind::Uploaded,
                Kind::Copied,
                Kind::UploadFallback,
            ][i % 4];
            let day = 18 - (n - 1 - i) as u8 / 3 % 15;
            let local = LocalTime {
                year: 2026,
                month: 11,
                day,
                hour: (8 + i % 12) as u8,
                minute: (i * 7 % 60) as u8,
                second: (i % 60) as u8,
            };
            let path = matches!(kind, Kind::Saved | Kind::UploadFallback).then(|| {
                let p = pics.join(format!("darkshot_2026-11-{day:02}_{i:03}.png"));
                std::fs::write(&p, encode_png(&img).unwrap()).unwrap();
                p
            });
            let new = NewEntry {
                kind,
                unix_millis: 1_790_000_000_000 + i as u64 * 60_000,
                local,
                path: path.clone(),
                url: (kind == Kind::Uploaded)
                    .then(|| format!("https://img.example.com/shots/darkshot_{i:03}.png")),
                expires: (kind == Kind::Uploaded).then(|| {
                    if i % 8 == 1 {
                        String::from("2026-11-10")
                    } else {
                        String::from("2026-12-18")
                    }
                }),
            };
            store.record(new, &img).unwrap();
            if i == n.saturating_sub(4) {
                lost = path;
            }
        }
        if let Some(p) = lost {
            let _ = std::fs::remove_file(p);
        }
        root
    }

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let d = std::env::temp_dir().join(format!("darkshot_hw_{tag}_{}_{t}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// Kho tạm 60 mục: mở, chờ nạp xong và ảnh nhỏ về, mô phỏng cuộn/chọn 100 lần, đóng; năm vòng,
    /// số đối tượng GDI không tăng dần.
    /// Chạy tay: `cargo test -p darkshot-windows -- --ignored --test-threads=1 history_window_with_entries`.
    #[test]
    #[ignore]
    fn history_window_with_entries_has_no_gdi_leak() {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            VK_DOWN, VK_END, VK_HOME, VK_NEXT, VK_UP,
        };
        use windows::Win32::UI::WindowsAndMessaging::{SendMessageW, WM_MOUSEWHEEL, WM_VSCROLL};
        let dir = temp_dir("leak");
        let root = synthetic_store(&dir, 60);
        let parent = test_parent(w!("darkshot-history-test-parent-2"));
        let mut counts = Vec::new();
        for _round in 0..5 {
            show_with_root(parent, root.clone());
            assert!(
                pump_until(10_000, || loaded_count() == Some(50)),
                "store did not load: {:?}",
                loaded_count()
            );
            let list = HWND(list_hwnd() as *mut c_void);
            assert!(!list.is_invalid());
            pump_until(3_000, || pending_thumbs() == 0);
            for i in 0..100u32 {
                let key = [VK_DOWN, VK_DOWN, VK_NEXT, VK_UP, VK_END, VK_HOME][i as usize % 6];
                // SAFETY: thông điệp gửi đồng bộ tới danh sách của cửa sổ Lịch sử trong luồng này.
                unsafe {
                    SendMessageW(
                        list,
                        WM_KEYDOWN,
                        Some(WPARAM(key.0 as usize)),
                        Some(LPARAM(0)),
                    );
                    let delta: i16 = if i % 2 == 0 { -120 } else { 120 };
                    SendMessageW(
                        list,
                        WM_MOUSEWHEEL,
                        Some(WPARAM((delta as u16 as usize) << 16)),
                        Some(LPARAM(0)),
                    );
                    SendMessageW(list, WM_VSCROLL, Some(WPARAM(3)), Some(LPARAM(0)));
                }
                pump_until(5, || false);
            }
            assert!(selected_index().is_some(), "keyboard must select a row");
            pump_until(3_000, || pending_thumbs() == 0 && !preview_loading());
            close_if_open();
            pump_until(50, || false);
            counts.push(gdi_count());
        }
        assert!(
            counts[4] <= counts[1] + 2,
            "GDI objects grew across opens: {counts:?}"
        );
        // SAFETY: dọn cửa sổ cha.
        unsafe {
            let _ = DestroyWindow(parent);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn send(h: HWND, msg: u32, wp: usize, lp: isize) -> isize {
        // SAFETY: thông điệp gửi đồng bộ tới cửa sổ của luồng kiểm thử này.
        unsafe { SendMessageW(h, msg, Some(WPARAM(wp)), Some(LPARAM(lp))).0 }
    }

    fn open_store(parent: HWND, root: &std::path::Path, n: usize) {
        show_with_root(parent, root.to_path_buf());
        assert!(
            pump_until(10_000, || loaded_count() == Some(n)),
            "store did not load: {:?}",
            loaded_count()
        );
    }

    /// Gõ chữ vào ô tìm bằng `WM_CHAR` thu hẹp danh sách ngay (không cần Enter); xóa chữ thì đủ lại.
    /// Chạy tay: `cargo test -p darkshot-windows -- --ignored --test-threads=1 history_search_typing`.
    #[test]
    #[ignore]
    fn history_search_typing_narrows_list() {
        use windows::Win32::UI::WindowsAndMessaging::{WM_CHAR, WM_SETTEXT};
        let dir = temp_dir("search");
        let root = synthetic_store(&dir, 12);
        let parent = test_parent(w!("darkshot-history-test-parent-3"));
        open_store(parent, &root, 12);
        assert_eq!(visible_count(), 12);
        let search = HWND(search_hwnd() as *mut c_void);
        assert!(!search.is_invalid());
        for c in "_003".encode_utf16() {
            send(search, WM_CHAR, c as usize, 1);
        }
        pump_until(200, || false);
        assert_eq!(visible_count(), 1, "typing must filter immediately");
        assert!(
            selected_index() == Some(0),
            "the only match must be selected"
        );
        let empty: [u16; 1] = [0];
        send(search, WM_SETTEXT, 0, empty.as_ptr() as isize);
        pump_until(200, || false);
        assert_eq!(visible_count(), 12);
        close_if_open();
        // SAFETY: dọn cửa sổ cha.
        unsafe {
            let _ = DestroyWindow(parent);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Bấm từng viên lọc: số mục hiển thị theo loại; bộ lọc đang chọn nằm trong vòng Tab.
    /// Chạy tay: `cargo test -p darkshot-windows -- --ignored --test-threads=1 history_filter_chips`.
    #[test]
    #[ignore]
    fn history_filter_chips_switch_filters() {
        use windows::Win32::UI::WindowsAndMessaging::BM_CLICK;
        let dir = temp_dir("chips");
        let root = synthetic_store(&dir, 12);
        let parent = test_parent(w!("darkshot-history-test-parent-4"));
        open_store(parent, &root, 12);
        // Mỗi loại 3 mục; Đã lưu gồm cả Lưu dự phòng.
        for (k, want) in [(1usize, 6usize), (2, 3), (3, 3), (0, 12), (2, 3), (0, 12)] {
            let chip = HWND(chip_hwnd(k) as *mut c_void);
            send(chip, BM_CLICK, 0, 0);
            pump_until(100, || false);
            assert_eq!(visible_count(), want, "chip {k}");
            assert_eq!(current_chip(), k);
        }
        close_if_open();
        // SAFETY: dọn cửa sổ cha.
        unsafe {
            let _ = DestroyWindow(parent);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Mở/đóng thẻ xác nhận 50 lần (bằng nút Hủy và bằng Esc): không xóa gì, không rò GDI.
    /// Chạy tay: `cargo test -p darkshot-windows -- --ignored --test-threads=1 history_confirm_card`.
    #[test]
    #[ignore]
    fn history_confirm_card_open_close_has_no_gdi_leak() {
        use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, BM_CLICK};
        let dir = temp_dir("confirm");
        let root = synthetic_store(&dir, 6);
        let parent = test_parent(w!("darkshot-history-test-parent-5"));
        open_store(parent, &root, 6);
        let clear = HWND(clear_hwnd() as *mut c_void);
        let mut counts = Vec::new();
        for i in 0..50 {
            send(clear, BM_CLICK, 0, 0);
            pump_until(50, || false);
            assert!(confirm_open(), "round {i}: card must open");
            if i % 2 == 0 {
                let (cancel, _) = confirm_buttons();
                send(HWND(cancel as *mut c_void), BM_CLICK, 0, 0);
            } else {
                // SAFETY: Esc đăng vào hàng đợi của luồng này, đi qua pre_translate như phím thật.
                unsafe {
                    let f = GetFocus();
                    let _ =
                        PostMessageW(Some(f), WM_KEYDOWN, WPARAM(VK_ESCAPE.0 as usize), LPARAM(0));
                }
            }
            pump_until(50, || false);
            assert!(!confirm_open(), "round {i}: card must close");
            assert!(is_open(), "Esc on the card must not close the window");
            counts.push(gdi_count());
        }
        assert_eq!(loaded_count(), Some(6), "cancel must not clear");
        assert!(counts[49] <= counts[5] + 2, "GDI objects grew: {counts:?}");
        close_if_open();
        // SAFETY: dọn cửa sổ cha.
        unsafe {
            let _ = DestroyWindow(parent);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Xóa một mục trên kho tạm: tệp `images/<id>.png` (bản sao của ứng dụng) biến mất, tệp `path` của
    /// người dùng vẫn còn trên đĩa; mục kế được chọn.
    /// Chạy tay: `cargo test -p darkshot-windows -- --ignored --test-threads=1 history_remove_entry`.
    #[test]
    #[ignore]
    fn history_remove_entry_deletes_only_owned_files() {
        use darkshot_core::history::Kind;
        let dir = temp_dir("remove");
        let root = synthetic_store(&dir, 8);
        let parent = test_parent(w!("darkshot-history-test-parent-6"));
        open_store(parent, &root, 8);
        let entries = Store::new(root.clone()).load().entries;
        // Một mục Đã chép (có bản sao) và một mục Đã lưu (tệp của người dùng).
        let copied = entries
            .iter()
            .find(|e| e.kind == Kind::Copied)
            .unwrap()
            .clone();
        let saved = entries
            .iter()
            .find(|e| e.kind == Kind::Saved && e.path.as_ref().is_some_and(|p| p.exists()))
            .unwrap()
            .clone();
        let managed = root.join("images").join(format!("{}.png", copied.id));
        assert!(managed.exists());
        let user = saved.path.clone().unwrap();
        // Đường dẫn dài (thư mục tạm) bị cắt trong khung chi tiết: có tooltip hiện đủ (thiết kế 3.5).
        assert!(select_id(&saved.id));
        pump_until(2_000, || tip_ids().contains(&hp::TIP_PATH));
        assert!(tip_ids().contains(&hp::TIP_PATH), "{:?}", tip_ids());
        for (e, n) in [(&copied, 7), (&saved, 6)] {
            assert!(select_id(&e.id), "entry must be visible");
            let pos = selected_index();
            on_action(ID_REMOVE);
            assert!(
                pump_until(5_000, || loaded_count() == Some(n)),
                "remove did not reload: {:?}",
                loaded_count()
            );
            // Chọn mục kế ở cùng vị trí (hoặc mục cuối).
            assert_eq!(selected_index(), pos.map(|p| p.min(n - 1)));
        }
        assert!(!managed.exists(), "managed copy must be deleted");
        assert!(user.exists(), "user file must stay");
        assert!(!root
            .join("thumbs")
            .join(format!("{}.png", saved.id))
            .exists());
        close_if_open();
        // SAFETY: dọn cửa sổ cha.
        unsafe {
            let _ = DestroyWindow(parent);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Dựng kho demo tổng hợp trong `%DARKSHOT_DEMO_DIR%\Local\darkshot\history` (bố cục của
    /// `DARKSHOT_PROFILE_DIR`). Không đặt biến thì bỏ qua.
    /// Chạy tay: `set DARKSHOT_DEMO_DIR=...` rồi `cargo test -p darkshot-windows -- --ignored make_demo_store`.
    #[test]
    #[ignore]
    fn make_demo_store() {
        let Some(base) = std::env::var_os("DARKSHOT_DEMO_DIR") else {
            return;
        };
        let base = std::path::PathBuf::from(base);
        let local = base.join("Local").join("darkshot");
        let n: usize = std::env::var("DARKSHOT_DEMO_COUNT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(16);
        let root = synthetic_store(&local, n);
        assert!(root.join("index.json").exists());
    }
}
