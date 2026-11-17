//! Cửa sổ Lịch sử ảnh chụp: không chặn, tối đa một cửa sổ, đổi cỡ được, vẽ theo DPI của cửa sổ.
//! Thông điệp do vòng lặp của ứng dụng phân phối; `pre_translate` lo Esc và Tab (`IsDialogMessageW`).
//!
//! Bố cục thuần ở `history_layout`, vẽ GDI ở `history_paint`. Danh sách là cửa sổ con riêng
//! (`darkshot-history-list`, có `WS_TABSTOP` và thanh cuộn thật `WS_VSCROLL` theo chủ đề
//! `DarkMode_Explorer`); khung chi tiết vẽ trên cửa sổ chính, nút hành động là `BUTTON` con tự vẽ.
//! Nạp chỉ mục, ảnh nhỏ, bản xem trước chạy ở luồng nền và báo về bằng `PostMessageW` kèm mã thế hệ.

use crate::history_layout::{
    actions_for, compose_over, layout, preview_target, reselect, step_selection, Avail, Generation,
    Layout, Nav, ThumbCache, ThumbSlot, MAX_THUMBS,
};
use crate::history_paint::{self as hp, Bmp, Fonts, PreviewView, RowView, ThumbView};
use crate::history_service::{self, WM_HISTORY_CHANGED};
use crate::lang;
use crate::paths;
use crate::theme::{self, ButtonKind};
use crate::winutil::{self, scale};
use darkshot_core::history::Entry;
use darkshot_core::history_store::Store;
use darkshot_core::i18n::Msg;
use darkshot_core::theme::{BG, INPUT, SURFACE};
use darkshot_core::thumb::{decode_png_file, downscale};
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use windows::core::{w, BOOL, HSTRING, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, DeleteObject, EndPaint, EnumDisplayMonitors, GetMonitorInfoW, InvalidateRect,
    MonitorFromPoint, ScreenToClient, HDC, HFONT, HGDIOBJ, HMONITOR, MONITORINFO,
    MONITOR_DEFAULTTONEAREST, PAINTSTRUCT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{SetScrollInfo, NMHDR, NM_CUSTOMDRAW, WM_MOUSELEAVE};
use windows::Win32::UI::HiDpi::{
    AdjustWindowRectExForDpi, GetDpiForMonitor, GetDpiForWindow, MDT_EFFECTIVE_DPI,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    EnableWindow, GetFocus, SetFocus, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT, VK_DOWN, VK_END,
    VK_ESCAPE, VK_HOME, VK_NEXT, VK_PRIOR, VK_UP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetCursorPos, GetScrollInfo,
    GetWindow, IsChild, IsDialogMessageW, IsWindow, KillTimer, LoadCursorW, MoveWindow,
    PeekMessageW, PostMessageW, RegisterClassW, SendMessageW, SetCursor, SetForegroundWindow,
    SetTimer, SetWindowPos, SetWindowTextW, ShowWindow, BS_PUSHBUTTON, DLGC_WANTARROWS,
    DLGC_WANTCHARS, GW_OWNER, HMENU, HTCLIENT, IDC_ARROW, IDC_HAND, MINMAXINFO, MSG, PM_REMOVE,
    SB_BOTTOM, SB_LINEDOWN, SB_LINEUP, SB_PAGEDOWN, SB_PAGEUP, SB_THUMBPOSITION, SB_THUMBTRACK,
    SB_TOP, SB_VERT, SCROLLBAR_COMMAND, SCROLLINFO, SIF_DISABLENOSCROLL, SIF_PAGE, SIF_POS,
    SIF_RANGE, SIF_TRACKPOS, SWP_NOACTIVATE, SWP_NOZORDER, SW_HIDE, SW_SHOW, WA_INACTIVE,
    WINDOW_EX_STYLE, WINDOW_STYLE, WM_ACTIVATE, WM_APP, WM_CLOSE, WM_COMMAND, WM_DPICHANGED,
    WM_ERASEBKGND, WM_GETDLGCODE, WM_GETMINMAXINFO, WM_KEYDOWN, WM_KILLFOCUS, WM_LBUTTONDOWN,
    WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCDESTROY, WM_NOTIFY, WM_PAINT, WM_SETCURSOR, WM_SETFOCUS,
    WM_SETFONT, WM_SIZE, WM_TIMER, WM_UPDATEUISTATE, WM_VSCROLL, WNDCLASSW, WS_CAPTION, WS_CHILD,
    WS_CLIPCHILDREN, WS_EX_APPWINDOW, WS_EX_CONTROLPARENT, WS_MAXIMIZEBOX, WS_MINIMIZEBOX,
    WS_SYSMENU, WS_TABSTOP, WS_THICKFRAME, WS_VSCROLL,
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

const ID_LIST: i32 = 200;
/// Nút hành động: Mở, Sao chép ảnh, Sao chép liên kết, Mở thư mục chứa ảnh, Xóa khỏi lịch sử.
const ACTION_IDS: [i32; 5] = [201, 202, 203, 204, 205];
const ID_REMOVE: i32 = 205;
/// Nút "Mở Tùy chọn" (trạng thái tắt và dải thông báo).
const ID_OPEN_OPTIONS: i32 = 210;
/// Hẹn giờ giải mã lại bản xem trước lớn hơn sau khi cửa sổ to ra.
const PREVIEW_TIMER: usize = 1;
const PREVIEW_REFRESH_MS: u32 = 200;
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
    /// Những gì `NM_CUSTOMDRAW` cần, giữ ngoài `STATE` để vẽ nút không bao giờ đụng khóa của nó:
    /// DPI, phông đậm, nút "Mở Tùy chọn" đang nằm trên dải thông báo.
    static DRAW: Cell<(u32, Option<HFONT>, bool)> = const { Cell::new((96, None, false)) };
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
    let r = std::panic::catch_unwind(|| {
        decode_png_file(&p).ok().map(|img| {
            // Ảnh gốc bị bỏ ngay sau khi thu nhỏ; giao diện chỉ nhận bản đã thu.
            let small = downscale(&img, target.0, target.1);
            drop(img);
            (small.width, small.height, compose_over(&small, INPUT))
        })
    });
    match r {
        Ok(Some((w, h, px))) => PreviewResult::Ready(w, h, px),
        _ => PreviewResult::Unreadable,
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
                    let px = std::panic::catch_unwind(|| {
                        store.thumbnail(&job.entry).ok().map(|img| {
                            let small = downscale(&img, job.cell.0, job.cell.1);
                            (small.width, small.height, compose_over(&small, INPUT))
                        })
                    })
                    .ok()
                    .flatten();
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

struct State {
    hwnd: HWND,
    list: HWND,
    buttons: [HWND; 5],
    options: HWND,
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
            lpfnWndProc: Some(list_proc),
            hInstance: hinst,
            lpszClassName: LIST_CLASS,
            hCursor: arrow,
            ..Default::default()
        };
        RegisterClassW(&lc);
    }
}

fn child(parent: HWND, class: PCWSTR, style: WINDOW_STYLE, id: i32) -> HWND {
    // SAFETY: tạo cửa sổ con của cửa sổ Lịch sử ở luồng chính; vị trí đặt sau ở `relayout`.
    unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class,
            &HSTRING::new(),
            WS_CHILD | style,
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
    let list = child(
        hwnd,
        LIST_CLASS,
        WINDOW_STYLE(WS_TABSTOP.0 | WS_VSCROLL.0),
        ID_LIST,
    );
    theme::set_dark_control_theme(list, theme::DarkClass::Explorer);
    let button_style = WINDOW_STYLE(WS_TABSTOP.0 | BS_PUSHBUTTON as u32);
    let buttons = ACTION_IDS.map(|id| child(hwnd, w!("BUTTON"), button_style, id));
    let options = child(hwnd, w!("BUTTON"), button_style, ID_OPEN_OPTIONS);
    let labels = [
        Msg::HistoryOpen,
        Msg::HistoryCopyImage,
        Msg::HistoryCopyLink,
        Msg::HistoryShowInFolder,
        Msg::HistoryRemove,
    ];
    for (b, m) in buttons.iter().zip(labels) {
        set_text(*b, lang::tr(m));
        set_font(*b, fonts.0.normal);
    }
    set_text(options, lang::tr(Msg::HistoryOpenOptions));
    set_font(options, fonts.0.normal);
    DRAW.with(|d| d.set((dpi, fonts.0.bold, false)));
    let hwnd_i = hwnd.0 as isize;
    let mut load_gen = Generation::default();
    let gen = load_gen.next();
    let state = State {
        hwnd,
        list,
        buttons,
        options,
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

/// `true`: thông điệp đã được xử lý, vòng lặp bỏ qua. Esc trong cửa sổ (hoặc con của nó) đóng cửa sổ;
/// Tab và phím mũi tên giữa các điều khiển qua `IsDialogMessageW`.
pub fn pre_translate(msg: &MSG) -> bool {
    let h = current();
    if h.is_invalid() || !winutil::belongs_to(h, msg) {
        return false;
    }
    if msg.message == WM_KEYDOWN && msg.wParam.0 as u16 == VK_ESCAPE.0 {
        close_if_open();
        return true;
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
    list: (RECT, bool),
    buttons: [(RECT, Avail); 5],
    open_label: Msg,
    options: Option<(RECT, bool)>,
}

/// Tính lại bố cục theo cỡ client hiện tại rồi đặt các cửa sổ con; vẽ lại cả cửa sổ.
fn relayout(hwnd: HWND) {
    let mut rc = RECT::default();
    // SAFETY: rc là biến cục bộ.
    unsafe {
        let _ = GetClientRect(hwnd, &mut rc);
    }
    let lang = lang::current();
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
            Msg::HistoryRemove,
        ];
        let widths: [i32; 5] = std::array::from_fn(|k| {
            let f = if k == 0 { bold } else { normal };
            button_width(f, darkshot_core::i18n::t(lang, labels[k]), dpi)
        });
        let avail = acts.map_or([Avail::Hidden; 5], |a| a.as_array());
        let shown = avail.map(|a| a != Avail::Hidden && s.has_entries());
        l.place_buttons(widths, shown);
        s.layout = l;
        let options = if s.loaded && !s.enabled {
            let bw = button_width(
                if s.has_entries() { normal } else { bold },
                darkshot_core::i18n::t(lang, Msg::HistoryOpenOptions),
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
                let body = darkshot_core::i18n::t(lang, Msg::HistoryDisabledBody);
                let tw = crate::history_layout::empty_text_width(&l.body, dpi);
                let th = hp::with_screen_dc(|dc| hp::measure_wrapped(dc, normal, body, tw));
                let e = crate::history_layout::empty_state_rects(&l.body, dpi, th, Some(bw));
                Some((e.button, false))
            }
        } else {
            None
        };
        ChildPlan {
            list: (l.list, s.has_entries()),
            buttons: std::array::from_fn(|k| {
                let a = if shown[k] { avail[k] } else { Avail::Hidden };
                (l.buttons[k], a)
            }),
            open_label,
            options,
        }
    });
    let Some(plan) = plan else {
        return;
    };
    let Some((list, buttons, options)) = with_state(|s| (s.list, s.buttons, s.options)) else {
        return;
    };
    let mv = |h: HWND, r: &RECT, show: bool| {
        // SAFETY: đặt vị trí và hiện/ẩn cửa sổ con của luồng này (không giữ khóa STATE).
        unsafe {
            let _ = MoveWindow(h, r.left, r.top, r.right - r.left, r.bottom - r.top, false);
            let _ = ShowWindow(h, if show { SW_SHOW } else { SW_HIDE });
        }
    };
    mv(list, &plan.list.0, plan.list.1);
    set_text(buttons[0], lang::tr(plan.open_label));
    for (k, b) in buttons.iter().enumerate() {
        let (r, a) = plan.buttons[k];
        // SAFETY: bật/tắt nút con của luồng này.
        unsafe {
            let _ = EnableWindow(*b, a == Avail::Enabled);
        }
        mv(*b, &r, a != Avail::Hidden);
    }
    match plan.options {
        Some((r, in_banner)) => {
            DRAW.with(|d| {
                let (dpi, bold, _) = d.get();
                d.set((dpi, bold, in_banner));
            });
            mv(options, &r, true);
        }
        None => mv(options, &RECT::default(), false),
    }
    // Bề rộng client thật của danh sách (trừ thanh cuộn thật).
    let mut lc = RECT::default();
    // SAFETY: lc là biến cục bộ.
    unsafe {
        let _ = GetClientRect(list, &mut lc);
    }
    with_state(|s| {
        if lc.right > 0 {
            s.layout.list_client_w = lc.right;
        }
        s.scroll = s.layout.scroll_clamp(s.scroll, s.visible.len());
    });
    update_scrollbar(list);
    // SAFETY: chỉ đánh dấu vùng cần vẽ lại.
    unsafe {
        let _ = InvalidateRect(Some(hwnd), None, false);
        let _ = InvalidateRect(Some(list), None, false);
        for b in buttons.iter().chain([&options]) {
            let _ = InvalidateRect(Some(*b), None, false);
        }
    }
}

fn update_scrollbar(list: HWND) {
    let Some((content, view, pos)) = with_state(|s| {
        (
            s.layout.content_h(s.visible.len()),
            s.layout.list.bottom - s.layout.list.top,
            s.scroll,
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
        let old: Vec<String> = s
            .visible
            .iter()
            .filter_map(|&i| s.entries.get(i).map(|e| e.id.clone()))
            .collect();
        let old_refs: Vec<&str> = old.iter().map(String::as_str).collect();
        let sel = s.selected.take();
        s.entries = m.entries;
        s.exists = m.exists;
        s.exists.resize(s.entries.len(), false);
        s.visible = (0..s.entries.len()).collect();
        let new_refs: Vec<&str> = s.entries.iter().map(|e| e.id.as_str()).collect();
        let pos = reselect(&old_refs, sel.as_deref(), &new_refs);
        s.selected = pos.map(|p| s.entries[p].id.clone());
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
    }
}

/// Hành động của nút 201–205. Task 57 thay bằng hành động thật.
fn on_action(_id: i32) {}

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
    let mut ps = PAINTSTRUCT::default();
    // SAFETY: BeginPaint/EndPaint cặp đôi trong WM_PAINT của chính cửa sổ này.
    let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
    let mut client = RECT::default();
    // SAFETY: client là biến cục bộ.
    unsafe {
        let _ = GetClientRect(hwnd, &mut client);
    }
    with_state(|s| {
        theme::buffered(hdc, &ps.rcPaint, |dc| {
            theme::fill(dc, &client, BG);
            let l = &s.layout;
            let fonts = &s.fonts.0;
            hp::draw_header(dc, l, fonts, lang, s.has_entries());
            if !s.loaded {
                hp::draw_separators(dc, l, false);
                return;
            }
            if s.entries.is_empty() {
                hp::draw_separators(dc, l, false);
                let t = |m: Msg| darkshot_core::i18n::t(lang, m);
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
                return;
            }
            if s.banner() {
                hp::draw_banner(dc, l, fonts, lang);
            }
            hp::draw_separators(dc, l, true);
            let view = match &s.preview {
                Preview::Idle => PreviewView::Idle,
                Preview::Loading => PreviewView::Loading,
                Preview::Ready(b) => PreviewView::Ready(b),
                Preview::Missing => PreviewView::Missing,
                Preview::Unreadable => PreviewView::Unreadable,
            };
            hp::draw_detail(dc, l, s.sel_entry(), view, fonts, lang, &today);
        });
    });
    // SAFETY: kết thúc lần vẽ đã mở ở trên.
    unsafe {
        let _ = EndPaint(hwnd, &ps);
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
            let Some(nav) = nav_of(wp.0 as u16) else {
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

/// Đổi DPI: phông mới cho điều khiển, đệm ảnh nhỏ làm lại theo cỡ ô mới.
fn on_dpi_changed(hwnd: HWND, dpi: u32) {
    let fonts = FontSet::new(dpi);
    let normal = fonts.0.normal;
    let bold = fonts.0.bold;
    let Some((old, controls)) = with_state(|s| {
        s.dpi = dpi;
        drop(s.thumbs.clear());
        s.thumb_gen.next();
        let old = std::mem::replace(&mut s.fonts, fonts);
        let mut c = s.buttons.to_vec();
        c.push(s.options);
        (old, c)
    }) else {
        return;
    };
    DRAW.with(|d| {
        let (_, _, banner) = d.get();
        d.set((dpi, bold, banner));
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
            if lo_word(wp.0) == WA_INACTIVE {
                // SAFETY: chỉ hỏi cửa sổ có tiêu điểm và quan hệ cha con.
                let f = unsafe { GetFocus() };
                if !f.is_invalid() && unsafe { IsChild(hwnd, f).as_bool() } {
                    with_state(|s| s.last_focus = f);
                }
            }
            // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
            unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
        }
        WM_SETFOCUS => {
            // Cửa sổ chính không nhận phím: trả tiêu điểm cho điều khiển dùng gần nhất.
            if let Some(f) = with_state(|s| s.last_focus) {
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
            // Bánh xe khi tiêu điểm ở nút: cuộn danh sách.
            if let Some(list) = with_state(|s| s.list) {
                // SAFETY: gửi đồng bộ cho danh sách của cửa sổ này (không giữ khóa STATE).
                return unsafe { SendMessageW(list, msg, Some(wp), Some(lp)) };
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let (id, code) = ((wp.0 & 0xFFFF) as i32, (wp.0 >> 16) & 0xFFFF);
            if code == 0 {
                if ACTION_IDS.contains(&id) {
                    on_action(id);
                } else if id == ID_OPEN_OPTIONS {
                    open_options(hwnd);
                }
            }
            LRESULT(0)
        }
        WM_NOTIFY => {
            // SAFETY: lp của WM_NOTIFY trỏ tới NMHDR hợp lệ suốt lời gọi.
            let hdr = unsafe { (lp.0 as *const NMHDR).as_ref() };
            if let Some(h) = hdr.filter(|h| h.code == NM_CUSTOMDRAW) {
                let id = h.idFrom as i32;
                let (dpi, bold, in_banner) = DRAW.with(|d| d.get());
                // SAFETY: mã NM_CUSTOMDRAW nên lp trỏ tới NMCUSTOMDRAW.
                if let Some(cd) = unsafe { theme::nmcd_of(lp) } {
                    match id {
                        201 => {
                            return theme::custom_draw_button(
                                cd,
                                ButtonKind::Primary,
                                BG,
                                dpi,
                                bold,
                            )
                        }
                        202..=204 => {
                            return theme::custom_draw_button(cd, ButtonKind::Normal, BG, dpi, None)
                        }
                        ID_REMOVE => return hp::custom_draw_danger(cd, BG, dpi),
                        ID_OPEN_OPTIONS if in_banner => {
                            return theme::custom_draw_button(
                                cd,
                                ButtonKind::Normal,
                                SURFACE,
                                dpi,
                                None,
                            )
                        }
                        ID_OPEN_OPTIONS => {
                            return theme::custom_draw_button(
                                cd,
                                ButtonKind::Primary,
                                BG,
                                dpi,
                                bold,
                            )
                        }
                        _ => {}
                    }
                }
            }
            // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
            unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
        }
        WM_HISTORY_CHANGED => {
            start_load(hwnd);
            LRESULT(0)
        }
        WM_HISTORY_LOADED => {
            // SAFETY: con trỏ do post_box cấp phát cho đúng mã thông điệp này, chuyển quyền cho ta.
            let m = unsafe { Box::from_raw(lp.0 as *mut LoadMsg) };
            on_loaded(hwnd, *m);
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
                }
                let state = STATE.with(|s| s.borrow_mut().take());
                drop(state);
                drain_results(hwnd);
                DRAW.with(|d| d.set((96, None, false)));
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
