//! Phiên chụp cuộn: khung vùng (bốn cửa sổ mảnh), thanh điều khiển nổi, hẹn giờ chụp vùng và ghép
//! bằng `darkshot_core::scroll_stitch::Stitcher`, Esc toàn cục tạm.
//!
//! Mọi cửa sổ của phiên luôn trên cùng, không lấy tiêu điểm, loại khỏi ảnh chụp
//! (`WDA_EXCLUDEFROMCAPTURE`). Windows cũ hơn 2004 (không loại được): không vẽ khung, thanh đặt
//! ngoài vùng chụp. Không bao giờ gửi đầu vào tới ứng dụng khác: người dùng tự cuộn.
//!
//! Quy tắc tái nhập: trạng thái `SESSION` chỉ được mượn trong các khối ngắn không gọi hàm Win32 có
//! thể gửi thông điệp cho cửa sổ (SetWindowPos, ShowWindow, DestroyWindow, SetCapture...); kết thúc
//! phiên lấy trạng thái ra khỏi `SESSION` trước khi hủy cửa sổ.

use crate::capture;
use crate::lang;
use crate::theme::{self, ButtonKind, ControlState};
use crate::winutil::{scale, system_font};
use darkshot_core::geometry::{Point, Rect};
use darkshot_core::i18n::{self, Msg};
use darkshot_core::image::Image;
use darkshot_core::scroll_stitch::{Push, Stitcher, MAX_HEIGHT};
use darkshot_core::theme::{ACCENT, BG, DANGER_BUTTON, LINE, SURFACE, TEXT, TEXT_MUTED, WARNING};
use std::cell::{Cell, RefCell};
use std::time::Instant;
use windows::core::{w, BOOL, HSTRING, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateCompatibleDC, DeleteDC, DeleteObject, EndPaint, EnumDisplayMonitors,
    GetMonitorInfoW, InvalidateRect, MonitorFromRect, DT_CALCRECT, DT_LEFT, DT_NOPREFIX,
    DT_SINGLELINE, DT_VCENTER, HDC, HFONT, HGDIOBJ, HMONITOR, MONITORINFO,
    MONITOR_DEFAULTTONEAREST, PAINTSTRUCT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};
use windows::Win32::UI::Controls::WM_MOUSELEAVE;
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, ReleaseCapture, SetCapture, TrackMouseEvent, UnregisterHotKey, MOD_NOREPEAT,
    TME_LEAVE, TRACKMOUSEEVENT, VK_ESCAPE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetWindowRect, KillTimer,
    LoadCursorW, RegisterClassW, SetLayeredWindowAttributes, SetTimer, SetWindowDisplayAffinity,
    SetWindowPos, ShowWindow, HTTRANSPARENT, IDC_ARROW, LWA_ALPHA, MA_NOACTIVATE, SWP_NOACTIVATE,
    SWP_NOSIZE, SWP_NOZORDER, SW_SHOWNA, WDA_EXCLUDEFROMCAPTURE, WINDOW_EX_STYLE,
    WM_CAPTURECHANGED, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_ERASEBKGND, WM_HOTKEY, WM_LBUTTONDOWN,
    WM_LBUTTONUP, WM_MOUSEACTIVATE, WM_MOUSEMOVE, WM_NCHITTEST, WM_PAINT, WM_TIMER, WNDCLASSW,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

const HOST_CLASS: PCWSTR = w!("darkshot.scroll-host");
const FRAME_CLASS: PCWSTR = w!("darkshot.scroll-frame");
const BAR_CLASS: PCWSTR = w!("darkshot.scroll-bar");

// Kích thước ở 96 DPI (DIP).
/// Bề dày khung vùng (bao ngoài vùng).
const FRAME_DIP: i32 = 2;
/// Khoảng cách giữa vùng và thanh điều khiển.
const BAR_GAP: i32 = 8;
/// Chiều cao tối thiểu của thanh.
const BAR_H: i32 = 40;
/// Lề trái/phải.
const PAD: i32 = 12;
/// Chấm đỏ: đường kính, khoảng cách tới chữ.
const DOT: i32 = 8;
const DOT_GAP: i32 = 8;
/// Khoảng cách giữa các phần chữ, và từ chữ tới nút đầu.
const TEXT_GAP: i32 = 12;
/// Nút: lề chữ hai bên, chiều cao tối thiểu, khoảng cách giữa hai nút.
const BTN_PAD: i32 = 12;
const BTN_H: i32 = 28;
const BTN_GAP: i32 = 6;

/// Hẹn giờ chụp của cửa sổ chủ.
const TIMER_ID: usize = 1;
/// Nhịp chụp thường và nhịp giãn khi máy chậm.
const INTERVAL_MS: u32 = 100;
const SLOW_INTERVAL_MS: u32 = 200;
/// Một lượt lâu hơn mức này là chậm; đủ `SLOW_STREAK` lượt chậm liên tiếp thì giãn nhịp.
const SLOW_TICK_MS: u32 = 80;
const SLOW_STREAK: u32 = 3;
/// Đang giãn: đủ `FAST_STREAK` lượt liên tiếp không quá `FAST_TICK_MS` thì về nhịp thường.
const FAST_TICK_MS: u32 = 40;
const FAST_STREAK: u32 = 20;
/// Trạng thái vàng sau một khung không ghép được.
const WARN_MS: u64 = 1500;

/// ID phím Esc tạm (của cửa sổ chủ phiên; khác ID của lớp phủ và phím chụp).
const ESC_HOTKEY_ID: i32 = 0x5C01;

/// Bản dựng Windows đầu tiên hỗ trợ `WDA_EXCLUDEFROMCAPTURE` (Windows 10 2004). Bản cũ hơn coi
/// giá trị này như `WDA_MONITOR` (ô đen trong ảnh chụp) nên không dùng được.
const EXCLUDE_MIN_BUILD: u32 = 19041;

/// `WDA_EXCLUDEFROMCAPTURE` dùng được ở bản dựng `build` (không biết thì coi là không).
fn exclude_supported(build: Option<u32>) -> bool {
    build.is_some_and(|b| b >= EXCLUDE_MIN_BUILD)
}

/// Số bản dựng Windows (`CurrentBuildNumber` trong registry; không phụ thuộc manifest). Như
/// `steps_bar` (hàm của nó riêng tư, gói task không cho sửa file đó).
fn os_build() -> Option<u32> {
    let mut buf = [0u16; 64];
    let mut bytes = (buf.len() * 2) as u32;
    // SAFETY: bộ đệm dài `bytes` byte, sống suốt lời gọi; RegGetValueW tự mở và đóng khóa con.
    let r = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            &HSTRING::from(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion"),
            &HSTRING::from("CurrentBuildNumber"),
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr().cast()),
            Some(&mut bytes),
        )
    };
    if r.is_err() {
        return None;
    }
    let len = buf.iter().position(|&u| u == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len]).trim().parse().ok()
}

/// Bề dày khung (pixel) ở `dpi`.
fn frame_thickness(dpi: u32) -> i32 {
    scale(FRAME_DIP, dpi).max(1)
}

/// Bốn ô khung bao ngoài `region`, dày `t`: trên, dưới (gồm góc), trái, phải.
fn frame_rects(region: Rect, t: i32) -> [Rect; 4] {
    let Rect { x, y, w, h } = region;
    [
        Rect {
            x: x - t,
            y: y - t,
            w: w + 2 * t,
            h: t,
        },
        Rect {
            x: x - t,
            y: y + h,
            w: w + 2 * t,
            h: t,
        },
        Rect {
            x: x - t,
            y,
            w: t,
            h,
        },
        Rect {
            x: x + w,
            y,
            w: t,
            h,
        },
    ]
}

/// `v` kẹp vào [`lo`, `hi`]; `hi` < `lo` thì lấy `lo`.
fn clamp_lo(v: i32, lo: i32, hi: i32) -> i32 {
    v.min(hi).max(lo)
}

fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.right() && b.x < a.right() && a.y < b.bottom() && b.y < a.bottom()
}

/// Góc trên trái của thanh cỡ `size` cho vùng `region` (tọa độ màn hình): ngay dưới vùng (cách
/// `gap`), không đủ chỗ thì ngay trên, trong vùng làm việc `work` của màn hình chứa vùng; ngang
/// căn giữa theo vùng, kẹp vào vùng làm việc. Còn lại:
/// - `excluded` (thanh không có trong ảnh chụp): trong vùng làm việc, sát đáy;
/// - không loại được: thanh KHÔNG BAO GIỜ nằm trong vùng chụp: bên phải, bên trái vùng, rồi đáy
///   một vùng làm việc khác (`others`) không chạm vùng; không còn chỗ thì `None` (không có thanh).
fn bar_origin(
    region: Rect,
    size: (i32, i32),
    gap: i32,
    work: Rect,
    others: &[Rect],
    excluded: bool,
) -> Option<Point> {
    let (w, h) = size;
    let fits_v = |y: i32| y >= work.y && y + h <= work.bottom();
    let x = clamp_lo(region.x + (region.w - w) / 2, work.x, work.right() - w);
    let below = region.bottom() + gap;
    if fits_v(below) {
        return Some(Point { x, y: below });
    }
    let above = region.y - gap - h;
    if fits_v(above) {
        return Some(Point { x, y: above });
    }
    if excluded {
        let y = clamp_lo(work.bottom() - gap - h, work.y, work.bottom() - h);
        return Some(Point { x, y });
    }
    if h <= work.h {
        let y = clamp_lo(region.bottom() - h, work.y, work.bottom() - h);
        let right = region.right() + gap;
        if right >= work.x && right + w <= work.right() {
            return Some(Point { x: right, y });
        }
        let left = region.x - gap - w;
        if left >= work.x && left + w <= work.right() {
            return Some(Point { x: left, y });
        }
    }
    others.iter().find_map(|o| {
        if w > o.w || h > o.h {
            return None;
        }
        let p = Point {
            x: o.x + (o.w - w) / 2,
            y: clamp_lo(o.bottom() - gap - h, o.y, o.bottom() - h),
        };
        let bar = Rect {
            x: p.x,
            y: p.y,
            w,
            h,
        };
        (!overlaps(bar, region)).then_some(p)
    })
}

/// Như `bar_origin` trong vùng làm việc `work`; không có chỗ thì thử cả màn hình `mon` (đè lên
/// thanh tác vụ), vẫn không bao giờ trong vùng khi không loại được khỏi ảnh chụp.
fn bar_spot(
    region: Rect,
    size: (i32, i32),
    gap: i32,
    work: Rect,
    mon: Rect,
    others: &[Rect],
    excluded: bool,
) -> Option<Point> {
    bar_origin(region, size, gap, work, others, excluded)
        .or_else(|| bar_origin(region, size, gap, mon, &[], excluded))
}

/// Vùng còn nằm trọn trên các màn hình `monitors` (tọa độ màn hình, không chồng nhau).
fn region_valid(region: Rect, monitors: &[Rect]) -> bool {
    if region.w <= 0 || region.h <= 0 {
        return false;
    }
    let covered: i64 = monitors
        .iter()
        .map(|m| {
            let w = (region.right().min(m.right()) - region.x.max(m.x)).max(0);
            let h = (region.bottom().min(m.bottom()) - region.y.max(m.y)).max(0);
            i64::from(w) * i64::from(h)
        })
        .sum();
    covered >= i64::from(region.w) * i64::from(region.h)
}

/// Giai đoạn của phiên.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Running,
    /// Xong (nút, hoặc không còn khung nào để chờ).
    Done,
    /// Hủy hoặc Esc: bỏ hết.
    Cancelled,
    /// Đạt giới hạn chiều cao hoặc bộ nhớ: dừng như Xong, có thông báo.
    Limit,
    /// Vùng không còn hợp lệ (màn hình đổi): dừng như Xong.
    Lost,
}

impl Phase {
    /// Kết thúc có giao ảnh ghép cho hàm kết quả.
    fn delivers(self) -> bool {
        matches!(self, Phase::Done | Phase::Limit | Phase::Lost)
    }
}

/// Đầu vào của máy trạng thái phiên.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Input {
    Push(Push),
    Done,
    Cancel,
    RegionLost,
}

/// Máy trạng thái thuần của phiên: giai đoạn, nhịp hẹn giờ, trạng thái vàng.
#[derive(Clone, Debug)]
struct Machine {
    phase: Phase,
    interval: u32,
    slow: u32,
    fast: u32,
    /// Thời điểm (ms từ đầu phiên) trạng thái vàng hết hạn.
    warn_until: Option<u64>,
}

impl Machine {
    fn new() -> Machine {
        Machine {
            phase: Phase::Running,
            interval: INTERVAL_MS,
            slow: 0,
            fast: 0,
            warn_until: None,
        }
    }

    fn phase(&self) -> Phase {
        self.phase
    }

    #[cfg(test)]
    fn interval_ms(&self) -> u32 {
        self.interval
    }

    /// Nhận một đầu vào lúc `now_ms`; trả `true` đúng một lần, khi phiên vừa kết thúc.
    fn input(&mut self, i: Input, now_ms: u64) -> bool {
        if self.phase != Phase::Running {
            return false;
        }
        let next = match i {
            Input::Push(Push::Limit) => Phase::Limit,
            Input::Push(Push::Uncertain(_)) => {
                self.warn_until = Some(now_ms + WARN_MS);
                Phase::Running
            }
            Input::Push(_) => Phase::Running,
            Input::Done => Phase::Done,
            Input::Cancel => Phase::Cancelled,
            Input::RegionLost => Phase::Lost,
        };
        self.phase = next;
        next != Phase::Running
    }

    /// Trạng thái vàng đang hiện lúc `now_ms`.
    fn warning(&self, now_ms: u64) -> bool {
        self.warn_until.is_some_and(|t| now_ms < t)
    }

    /// Một lượt chụp vừa mất `ms`; trả nhịp mới khi nhịp đổi.
    fn tick_took(&mut self, ms: u32) -> Option<u32> {
        if ms > SLOW_TICK_MS {
            self.slow += 1;
        } else {
            self.slow = 0;
        }
        if ms <= FAST_TICK_MS {
            self.fast += 1;
        } else {
            self.fast = 0;
        }
        if self.interval == INTERVAL_MS && self.slow >= SLOW_STREAK {
            self.interval = SLOW_INTERVAL_MS;
            self.fast = 0;
            return Some(self.interval);
        }
        if self.interval == SLOW_INTERVAL_MS && self.fast >= FAST_STREAK {
            self.interval = INTERVAL_MS;
            self.slow = 0;
            return Some(self.interval);
        }
        None
    }
}

/// Nút của thanh điều khiển.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BarButton {
    Done,
    Cancel,
}

impl BarButton {
    fn label(self) -> Msg {
        match self {
            BarButton::Done => Msg::ScrollDone,
            BarButton::Cancel => Msg::ScrollCancel,
        }
    }

    fn kind(self) -> ButtonKind {
        match self {
            BarButton::Done => ButtonKind::Primary,
            BarButton::Cancel => ButtonKind::Normal,
        }
    }
}

/// Thứ tự nút từ trái sang phải: Xong (chính), Hủy.
const BUTTONS: [BarButton; 2] = [BarButton::Done, BarButton::Cancel];

/// Bề rộng chữ đã đo (pixel): tiêu đề, chiều cao (dành chỗ cho số lớn nhất), trạng thái (dành chỗ
/// cho "Cuộn chậm hơn"), nhãn hai nút; chiều cao dòng.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Texts {
    title: i32,
    height: i32,
    status: i32,
    buttons: [i32; 2],
    line: i32,
}

/// Bố cục thanh (tọa độ client, pixel).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Layout {
    size: (i32, i32),
    dot: Rect,
    title: Rect,
    height: Rect,
    status: Rect,
    buttons: [Rect; 2],
}

/// Chấm đỏ, tiêu đề, chiều cao, trạng thái, hai nút xếp ngang, căn giữa theo chiều dọc.
fn layout(dpi: u32, t: &Texts) -> Layout {
    let k = |v: i32| scale(v, dpi);
    let h = k(BAR_H).max(t.line + 2 * k(8));
    let btn_h = k(BTN_H).max(t.line + k(8));
    let text_y = (h - t.line) / 2;
    let mut x = k(PAD);
    let dot = Rect {
        x,
        y: (h - k(DOT)) / 2,
        w: k(DOT),
        h: k(DOT),
    };
    x += k(DOT) + k(DOT_GAP);
    let mut text = |w: i32| {
        let r = Rect {
            x,
            y: text_y,
            w,
            h: t.line,
        };
        x += w + k(TEXT_GAP);
        r
    };
    let title = text(t.title);
    let height = text(t.height);
    let status = text(t.status);
    let mut buttons = [Rect {
        x: 0,
        y: 0,
        w: 0,
        h: 0,
    }; 2];
    for (i, (b, tw)) in buttons.iter_mut().zip(t.buttons).enumerate() {
        if i > 0 {
            x += k(BTN_GAP);
        }
        let w = tw + 2 * k(BTN_PAD);
        *b = Rect {
            x,
            y: (h - btn_h) / 2,
            w,
            h: btn_h,
        };
        x += w;
    }
    Layout {
        size: (x + k(PAD), h),
        dot,
        title,
        height,
        status,
        buttons,
    }
}

/// Nút dưới điểm `p` (tọa độ client).
fn hit(l: &Layout, p: Point) -> Option<BarButton> {
    l.buttons
        .iter()
        .position(|b| b.contains(p))
        .map(|i| BUTTONS[i])
}

/// Chữ, phông và bố cục của thanh.
struct BarUi {
    dpi: u32,
    font: Option<HFONT>,
    title: Vec<u16>,
    height: Vec<u16>,
    status: Vec<u16>,
    labels: [Vec<u16>; 2],
    layout: Layout,
    hover: Option<BarButton>,
    pressed: Option<BarButton>,
    tracking: bool,
    /// Trạng thái vàng đang được vẽ.
    warn: bool,
}

impl Drop for BarUi {
    fn drop(&mut self) {
        if let Some(f) = self.font.take() {
            // SAFETY: phông do thanh tạo; chỉ được chọn vào DC trong lúc vẽ, nên lúc này không DC
            // nào giữ nó (BarUi bị thay hay Session đã hủy cửa sổ thanh).
            unsafe {
                let _ = DeleteObject(HGDIOBJ(f.0));
            }
        }
    }
}

/// Vì sao không bắt đầu được phiên.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartError {
    /// Đã có phiên đang chạy.
    Busy,
    /// Vùng rỗng.
    Empty,
    /// Không có chỗ cho thanh điều khiển ngoài vùng chụp (thanh không loại khỏi ảnh chụp được).
    NoRoom,
    /// Không tạo được cửa sổ hoặc hẹn giờ.
    Failed,
}

/// Một phiên chụp cuộn đang chạy (một phiên cho cả tiến trình, trên luồng giao diện). Drop dọn
/// hẹn giờ, phím Esc tạm, mọi cửa sổ và phông, nên mọi đường kết thúc đều dọn sạch.
struct Session {
    /// Vùng chụp (tọa độ màn hình, pixel vật lý).
    region: Rect,
    stitcher: Option<Stitcher>,
    machine: Machine,
    started: Instant,
    /// Cửa sổ ẩn giữ hẹn giờ và phím Esc tạm, nhận WM_DISPLAYCHANGE.
    host: HWND,
    frames: Vec<HWND>,
    bar: Option<HWND>,
    /// Thanh đang được loại khỏi ảnh chụp (được phép nằm trong vùng).
    bar_excluded: bool,
    ui: BarUi,
    esc: bool,
}

impl Drop for Session {
    fn drop(&mut self) {
        // SAFETY: các cửa sổ, hẹn giờ và phím tắt do phiên tạo trên luồng này; mỗi thứ được giải
        // phóng đúng một lần (Session chỉ drop một lần, sau khi đã ra khỏi `SESSION`).
        unsafe {
            let _ = KillTimer(Some(self.host), TIMER_ID);
            if self.esc {
                let _ = UnregisterHotKey(Some(self.host), ESC_HOTKEY_ID);
            }
            if let Some(b) = self.bar.take() {
                let _ = DestroyWindow(b);
            }
            for f in self.frames.drain(..) {
                let _ = DestroyWindow(f);
            }
            let _ = DestroyWindow(self.host);
        }
        // Phông của thanh được xóa sau thân hàm này, khi `ui` drop (cửa sổ thanh đã hủy).
    }
}

fn noop_result(_: Image, _: u32) {}
fn noop_limit(_: u32) {}
fn noop() {}

thread_local! {
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
    static CLASSES_DONE: Cell<bool> = const { Cell::new(false) };
    static ON_RESULT: Cell<fn(Image, u32)> = const { Cell::new(noop_result) };
    static ON_LIMIT: Cell<fn(u32)> = const { Cell::new(noop_limit) };
    static ON_END: Cell<fn()> = const { Cell::new(noop) };
}

/// Hàm chạy mỗi khi một phiên đã bắt đầu kết thúc (mọi đường: Xong, Hủy, Esc, giới hạn, vùng mất,
/// thoát), sau khi cửa sổ của phiên đã hủy và trước hàm kết quả (main: hiện lại thanh phiên Chụp
/// từng bước đã ẩn).
pub fn set_on_end(f: fn()) {
    ON_END.with(|c| c.set(f));
}

/// Hàm nhận ảnh ghép và số chỗ ghép không chắc chắn khi phiên kết thúc bằng Xong, giới hạn hay
/// vùng mất (main: mở cửa sổ kết quả `scroll_result::open`).
pub fn set_on_result(f: fn(Image, u32)) {
    ON_RESULT.with(|c| c.set(f));
}

/// Hàm báo đạt giới hạn (nhận chiều cao ảnh ghép), gọi sau hàm kết quả.
pub fn set_on_limit(f: fn(u32)) {
    ON_LIMIT.with(|c| c.set(f));
}

/// Có phiên chụp cuộn đang chạy (trạng thái đang bận cũng tính là có).
pub fn active() -> bool {
    SESSION.with(|s| s.try_borrow().map(|x| x.is_some()).unwrap_or(true))
}

fn with<R>(f: impl FnOnce(&mut Session) -> R) -> Option<R> {
    SESSION.with(|s| s.try_borrow_mut().ok()?.as_mut().map(f))
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

fn to_rect(r: Rect) -> RECT {
    RECT {
        left: r.x,
        top: r.y,
        right: r.right(),
        bottom: r.bottom(),
    }
}

fn from_rect(r: RECT) -> Rect {
    Rect {
        x: r.left,
        y: r.top,
        w: r.right - r.left,
        h: r.bottom - r.top,
    }
}

/// Màn hình (cả màn hình, vùng làm việc) của mọi màn hình.
fn monitors() -> Vec<(Rect, Rect)> {
    unsafe extern "system" fn collect(m: HMONITOR, _: HDC, _: *mut RECT, lp: LPARAM) -> BOOL {
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        // SAFETY: lp là con trỏ tới Vec do `monitors` truyền, sống suốt EnumDisplayMonitors; m do
        // hệ thống cấp cho lời gọi lại này.
        unsafe {
            if GetMonitorInfoW(m, &mut mi).as_bool() {
                let v = &mut *(lp.0 as *mut Vec<(Rect, Rect)>);
                v.push((from_rect(mi.rcMonitor), from_rect(mi.rcWork)));
            }
        }
        BOOL(1)
    }
    let mut v: Vec<(Rect, Rect)> = Vec::new();
    // SAFETY: lời gọi lại chỉ ghi vào `v`, sống đến hết lời gọi đồng bộ này.
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(collect),
            LPARAM(&mut v as *mut Vec<(Rect, Rect)> as isize),
        );
    }
    v
}

/// Màn hình gần một hình chữ nhật nhất: vùng làm việc, cả màn hình, DPI; vùng làm việc các màn
/// hình khác.
struct Place {
    work: Rect,
    mon: Rect,
    dpi: u32,
    others: Vec<Rect>,
}

/// Thông tin màn hình gần `region` nhất (xem `Place`).
fn placement(region: Rect) -> Place {
    // SAFETY: chỉ hỏi thông tin màn hình; các biến là cục bộ.
    let (work, mon, dpi) = unsafe {
        let m = MonitorFromRect(&to_rect(region), MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let (work, mon) = if GetMonitorInfoW(m, &mut mi).as_bool() {
            (from_rect(mi.rcWork), from_rect(mi.rcMonitor))
        } else {
            (region, region)
        };
        let (mut x, mut y) = (96u32, 96u32);
        let dpi = if GetDpiForMonitor(m, MDT_EFFECTIVE_DPI, &mut x, &mut y).is_ok() {
            x
        } else {
            96
        };
        (work, mon, dpi)
    };
    let others = monitors()
        .into_iter()
        .filter(|(m, _)| *m != mon)
        .map(|(_, w)| w)
        .collect();
    Place {
        work,
        mon,
        dpi,
        others,
    }
}

/// Góc trên trái của thanh cỡ `size` (DPI `dpi`) cho vùng `region` (xem `bar_spot`).
fn place(region: Rect, size: (i32, i32), dpi: u32, excluded: bool) -> Option<Point> {
    let p = placement(region);
    let gap = scale(BAR_GAP, dpi);
    bar_spot(region, size, gap, p.work, p.mon, &p.others, excluded)
}

/// Thanh cỡ `ui` đặt cho `region`, dựng lại theo DPI của màn hình chứa thanh nếu khác `ui.dpi`.
fn place_ui(region: Rect, ui: BarUi, excluded: bool) -> Option<(Point, BarUi)> {
    let at = place(region, ui.layout.size, ui.dpi, excluded)?;
    let r = Rect {
        x: at.x,
        y: at.y,
        w: ui.layout.size.0,
        h: ui.layout.size.1,
    };
    let dpi = placement(r).dpi;
    if dpi == ui.dpi {
        return Some((at, ui));
    }
    let other = build_ui(dpi);
    match place(region, other.layout.size, dpi, excluded) {
        Some(p) => Some((p, other)),
        None => Some((at, ui)),
    }
}

/// Dòng tiêu đề: "Chụp cuộn · Cuộn xuống từ từ".
fn title_text() -> String {
    format!(
        "{} \u{00B7} {}",
        lang::tr(Msg::ScrollBarTitle),
        lang::tr(Msg::ScrollHint)
    )
}

fn height_text(px: u32) -> Vec<u16> {
    wide(&i18n::scroll_height(lang::current(), px))
}

/// Đo chữ một dòng bằng phông `font`.
fn measure(font: Option<HFONT>, parts: &[&[u16]]) -> Vec<(i32, i32)> {
    // SAFETY: DC bộ nhớ tạo và xóa trong hàm; `draw_text` chọn lại phông cũ trước khi trả.
    unsafe {
        let dc = CreateCompatibleDC(None);
        if dc.is_invalid() {
            return vec![(0, 0); parts.len()];
        }
        let flags = DT_CALCRECT | DT_SINGLELINE | DT_NOPREFIX;
        let out = parts
            .iter()
            .map(|t| {
                let r = theme::draw_text(dc, t, &RECT::default(), TEXT, font, flags);
                (r.right - r.left, r.bottom - r.top)
            })
            .collect();
        let _ = DeleteDC(dc);
        out
    }
}

/// Chữ, phông và bố cục ở `dpi`.
fn build_ui(dpi: u32) -> BarUi {
    let font = system_font(dpi);
    let title = wide(&title_text());
    let widest = height_text(MAX_HEIGHT);
    let slow = wide(lang::tr(Msg::ScrollSlowDown));
    let labels = BUTTONS.map(|b| wide(lang::tr(b.label())));
    let m = measure(font, &[&title, &widest, &slow, &labels[0], &labels[1]]);
    let texts = Texts {
        title: m[0].0,
        height: m[1].0,
        status: m[2].0,
        buttons: [m[3].0, m[4].0],
        line: m.iter().map(|x| x.1).max().unwrap_or(0),
    };
    BarUi {
        dpi,
        font,
        title,
        height: height_text(0),
        status: Vec::new(),
        labels,
        layout: layout(dpi, &texts),
        hover: None,
        pressed: None,
        tracking: false,
        warn: false,
    }
}

fn register_classes() {
    if CLASSES_DONE.with(|c| c.get()) {
        return;
    }
    // SAFETY: đăng ký lớp cửa sổ của luồng này; đăng ký lại lần hai báo lỗi vô hại.
    unsafe {
        let Ok(hinst) = GetModuleHandleW(None) else {
            return;
        };
        let cursor = LoadCursorW(None, IDC_ARROW).unwrap_or_default();
        for (class, proc_) in [
            (
                HOST_CLASS,
                host_proc as unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT,
            ),
            (FRAME_CLASS, frame_proc),
            (BAR_CLASS, bar_proc),
        ] {
            let wc = WNDCLASSW {
                lpfnWndProc: Some(proc_),
                hInstance: hinst.into(),
                lpszClassName: class,
                hCursor: cursor,
                ..Default::default()
            };
            RegisterClassW(&wc);
        }
    }
    CLASSES_DONE.with(|c| c.set(true));
}

/// Tạo một cửa sổ popup (không hiện) của luồng này.
fn create(ex: WINDOW_EX_STYLE, class: PCWSTR, r: Rect) -> Option<HWND> {
    // SAFETY: tạo cửa sổ popup cấp cao nhất của luồng này; lỗi trả None.
    unsafe {
        let hinst = GetModuleHandleW(None).ok();
        CreateWindowExW(
            ex,
            class,
            &HSTRING::from(lang::tr(Msg::ScrollBarTitle)),
            WS_POPUP,
            r.x,
            r.y,
            r.w,
            r.h,
            None,
            None,
            hinst.map(|h| h.into()),
            None,
        )
        .ok()
    }
}

/// Loại cửa sổ khỏi ảnh chụp; `false` khi hệ thống từ chối.
fn exclude(h: HWND) -> bool {
    // SAFETY: h là cửa sổ cấp cao nhất của tiến trình này.
    unsafe { SetWindowDisplayAffinity(h, WDA_EXCLUDEFROMCAPTURE) }.is_ok()
}

fn destroy(h: HWND) {
    // SAFETY: h là cửa sổ của luồng này, chưa thuộc phiên nào (hoặc phiên đã bỏ nó).
    unsafe {
        let _ = DestroyWindow(h);
    }
}

/// Bốn cửa sổ khung quanh `region`; không loại được khỏi ảnh chụp thì không có khung.
fn create_frames(region: Rect, dpi: u32, supported: bool) -> Vec<HWND> {
    if !supported {
        return Vec::new();
    }
    let ex =
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_LAYERED | WS_EX_TRANSPARENT;
    let mut out = Vec::new();
    for r in frame_rects(region, frame_thickness(dpi)) {
        let Some(h) = create(ex, FRAME_CLASS, r) else {
            out.into_iter().for_each(destroy);
            return Vec::new();
        };
        out.push(h);
        // SAFETY: h là cửa sổ lớp vừa tạo; độ đục 255 để nó hiện (không dùng UpdateLayeredWindow).
        let ok = unsafe { SetLayeredWindowAttributes(h, COLORREF(0), 255, LWA_ALPHA) }.is_ok();
        if !ok || !exclude(h) {
            out.into_iter().for_each(destroy);
            return Vec::new();
        }
    }
    out
}

/// Thanh điều khiển ở chỗ hợp lệ, cùng chữ và bố cục theo DPI của màn hình chứa nó, và cờ đang
/// loại khỏi ảnh chụp; `None` khi không có chỗ nào ngoài vùng (không loại được) hoặc lỗi tạo.
fn create_bar(region: Rect, dpi: u32, supported: bool) -> Option<(HWND, BarUi, bool)> {
    let (at, ui) = place_ui(region, build_ui(dpi), supported)?;
    let size = ui.layout.size;
    let ex = WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;
    let r = Rect {
        x: at.x,
        y: at.y,
        w: size.0,
        h: size.1,
    };
    let h = create(ex, BAR_CLASS, r)?;
    // Windows cũ hơn 2004 coi giá trị này như WDA_MONITOR: không đặt.
    if supported && exclude(h) {
        return Some((h, ui, true));
    }
    // Không loại được khỏi ảnh chụp: thanh phải nằm ngoài vùng.
    match place(region, size, ui.dpi, false) {
        Some(p) => {
            // SAFETY: dời cửa sổ vừa tạo (chưa hiện, chưa thuộc phiên), không kích hoạt.
            unsafe {
                let _ = SetWindowPos(
                    h,
                    None,
                    p.x,
                    p.y,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
            Some((h, ui, false))
        }
        None => {
            destroy(h);
            None
        }
    }
}

/// Bắt đầu phiên chụp cuộn trên `region` (tọa độ màn hình, pixel vật lý). Từ chối khi đã có
/// phiên, vùng rỗng, không có chỗ cho thanh ngoài vùng chụp (`NoRoom`: không có thanh thì không
/// bấm Xong được), hoặc không tạo được cửa sổ chủ/hẹn giờ.
pub fn start(region: Rect) -> Result<(), StartError> {
    if active() {
        return Err(StartError::Busy);
    }
    if region.w <= 0 || region.h <= 0 {
        return Err(StartError::Empty);
    }
    register_classes();
    let empty = Rect {
        x: 0,
        y: 0,
        w: 0,
        h: 0,
    };
    let Some(host) = create(WS_EX_TOOLWINDOW, HOST_CLASS, empty) else {
        return Err(StartError::Failed);
    };
    let supported = exclude_supported(os_build());
    let dpi = placement(region).dpi;
    let Some((bar, ui, bar_excluded)) = create_bar(region, dpi, supported) else {
        destroy(host);
        return Err(StartError::NoRoom);
    };
    let frames = create_frames(region, dpi, supported);
    // SAFETY: phím tắt gắn với cửa sổ chủ của luồng này; gỡ trong Drop của Session.
    let esc = unsafe {
        RegisterHotKey(
            Some(host),
            ESC_HOTKEY_ID,
            MOD_NOREPEAT,
            u32::from(VK_ESCAPE.0),
        )
    }
    .is_ok();
    let session = Session {
        region,
        stitcher: Some(Stitcher::new(region.w as u32, region.h as u32)),
        machine: Machine::new(),
        started: Instant::now(),
        host,
        frames,
        bar: Some(bar),
        bar_excluded,
        ui,
        esc,
    };
    // SAFETY: hẹn giờ gắn với cửa sổ chủ của luồng này; KillTimer trong Drop của Session.
    if unsafe { SetTimer(Some(host), TIMER_ID, INTERVAL_MS, None) } == 0 {
        drop(session);
        return Err(StartError::Failed);
    }
    let shown: Vec<HWND> = session.frames.iter().copied().chain(session.bar).collect();
    SESSION.with(|s| *s.borrow_mut() = Some(session));
    for h in shown {
        // SAFETY: hiện cửa sổ của phiên mà không kích hoạt (không giành tiêu điểm).
        unsafe {
            let _ = ShowWindow(h, SW_SHOWNA);
        }
    }
    Ok(())
}

/// Ngôn ngữ đổi: dựng lại chữ và cỡ của thanh.
pub fn relabel() {
    relayout_bar(None);
}

/// Dựng lại chữ, phông và bố cục của thanh ở `dpi` (hoặc DPI hiện tại), giữ chiều cao và trạng
/// thái đang hiện, rồi đặt lại thanh theo vùng.
fn relayout_bar(dpi: Option<u32>) {
    let Some((region, cur, bar, excluded, warn, px)) = with(|s| {
        let px = s.stitcher.as_ref().map_or(0, |st| st.height());
        (s.region, s.ui.dpi, s.bar, s.bar_excluded, s.ui.warn, px)
    }) else {
        return;
    };
    let Some(bar) = bar else {
        return;
    };
    let mut ui = build_ui(dpi.unwrap_or(cur));
    ui.height = height_text(px);
    ui.warn = warn;
    if warn {
        ui.status = wide(lang::tr(Msg::ScrollSlowDown));
    }
    let size = ui.layout.size;
    let at = place(region, size, ui.dpi, excluded);
    let mut slot = Some(ui);
    let old = with(|s| slot.take().map(|ui| std::mem::replace(&mut s.ui, ui))).flatten();
    // Phông cũ được xóa ở đây (không DC nào giữ nó ngoài lúc vẽ).
    drop(old);
    drop(slot);
    let mut wr = RECT::default();
    // SAFETY: bar là cửa sổ của phiên trên luồng này; đổi cỡ/dời không kích hoạt, không giữ mượn
    // `SESSION`.
    unsafe {
        let _ = GetWindowRect(bar, &mut wr);
        let p = at.unwrap_or(Point {
            x: wr.left,
            y: wr.top,
        });
        let _ = SetWindowPos(
            bar,
            None,
            p.x,
            p.y,
            size.0,
            size.1,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
    invalidate(bar);
}

/// Xong: kết thúc phiên và giao ảnh ghép. Chưa có khung nào (bấm rất sớm) thì chụp một khung.
pub fn done() {
    let empty = with(|s| {
        s.stitcher
            .as_ref()
            .is_some_and(|st| st.height() == 0)
            .then_some(s.region)
    })
    .flatten();
    if let Some(r) = empty {
        if let Ok(img) = capture::capture_rect(r) {
            with(|s| s.stitcher.as_mut().map(|st| st.push(&img)));
        }
    }
    end_with(Input::Done);
}

/// Hủy (nút Hủy, Esc, thoát ứng dụng): bỏ hết, không giao ảnh.
pub fn cancel() {
    end_with(Input::Cancel);
}

fn now_ms(s: &Session) -> u64 {
    s.started.elapsed().as_millis() as u64
}

fn end_with(i: Input) {
    if with(|s| {
        let now = now_ms(s);
        s.machine.input(i, now)
    }) == Some(true)
    {
        finish_ended();
    }
}

/// Phiên đã sang giai đoạn kết thúc: lấy trạng thái ra, dọn cửa sổ (Drop), rồi giao kết quả.
fn finish_ended() {
    let Some(mut s) = SESSION.with(|c| c.try_borrow_mut().ok().and_then(|mut x| x.take())) else {
        return;
    };
    let phase = s.machine.phase();
    let stitcher = s.stitcher.take();
    drop(s);
    ON_END.with(|c| c.get())();
    let Some(st) = stitcher.filter(|_| phase.delivers()) else {
        return;
    };
    let (img, uncertain) = st.finish();
    let height = img.height;
    if img.width > 0 && img.height > 0 {
        ON_RESULT.with(|c| c.get())(img, uncertain);
    }
    if phase == Phase::Limit {
        ON_LIMIT.with(|c| c.get())(height);
    }
}

/// Một nhịp hẹn giờ: chụp vùng, ghép, cập nhật thanh, giãn nhịp nếu máy chậm.
fn on_tick() {
    let Some(region) = with(|s| s.region) else {
        return;
    };
    let t0 = Instant::now();
    // Không giữ mượn `SESSION` trong lúc chụp (BitBlt không gửi thông điệp, nhưng giữ cho chắc).
    let frame = capture::capture_rect(region);
    let out = with(|s| {
        let push = match (&frame, s.stitcher.as_mut()) {
            (Ok(img), Some(st)) => Some(st.push(img)),
            _ => None,
        };
        let now = now_ms(s);
        let ended = push.is_some_and(|p| s.machine.input(Input::Push(p), now));
        let elapsed = u32::try_from(t0.elapsed().as_millis()).unwrap_or(u32::MAX);
        let interval = s.machine.tick_took(elapsed);
        let mut repaint = false;
        if let (Some(Push::Appended(_)), Some(st)) = (push, s.stitcher.as_ref()) {
            s.ui.height = height_text(st.height());
            repaint = true;
        }
        let warn = s.machine.warning(now);
        if warn != s.ui.warn {
            s.ui.warn = warn;
            s.ui.status = if warn {
                wide(lang::tr(Msg::ScrollSlowDown))
            } else {
                Vec::new()
            };
            repaint = true;
        }
        (ended, interval, repaint.then_some(s.bar).flatten(), s.host)
    });
    drop(frame);
    let Some((ended, interval, repaint, host)) = out else {
        return;
    };
    if ended {
        finish_ended();
        return;
    }
    if let Some(ms) = interval {
        // SAFETY: thay hẹn giờ cùng ID của cửa sổ chủ (SetTimer thay hẹn giờ đang có).
        unsafe {
            SetTimer(Some(host), TIMER_ID, ms, None);
        }
    }
    if let Some(b) = repaint {
        invalidate(b);
    }
}

/// Màn hình đổi: vùng không còn nằm trọn trên các màn hình thì dừng như Xong.
fn check_region() {
    let Some(region) = with(|s| s.region) else {
        return;
    };
    let screens: Vec<Rect> = monitors().into_iter().map(|(m, _)| m).collect();
    if !region_valid(region, &screens) {
        end_with(Input::RegionLost);
    }
}

fn invalidate(h: HWND) {
    // SAFETY: h là cửa sổ của luồng này.
    unsafe {
        let _ = InvalidateRect(Some(h), None, false);
    }
}

fn client_point(lp: LPARAM) -> Point {
    Point {
        x: (lp.0 & 0xFFFF) as i16 as i32,
        y: ((lp.0 >> 16) & 0xFFFF) as i16 as i32,
    }
}

/// Vẽ thanh điều khiển.
fn paint_bar(h: HWND) {
    let mut ps = PAINTSTRUCT::default();
    // SAFETY: cặp BeginPaint/EndPaint của cửa sổ này; mọi đối tượng GDI tạm do `theme` tạo và xóa.
    unsafe {
        let dc = BeginPaint(h, &mut ps);
        with(|s| {
            let ui = &s.ui;
            let l = &ui.layout;
            let full = RECT {
                left: 0,
                top: 0,
                right: l.size.0,
                bottom: l.size.1,
            };
            let flags = DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX;
            theme::buffered(dc, &full, |m| {
                let bw = scale(1, ui.dpi).max(1);
                theme::round_rect(m, &full, 0, SURFACE, LINE, bw);
                theme::ellipse(m, &to_rect(l.dot), DANGER_BUTTON, DANGER_BUTTON, 0);
                theme::draw_text(m, &ui.title, &to_rect(l.title), TEXT, ui.font, flags);
                theme::draw_text(
                    m,
                    &ui.height,
                    &to_rect(l.height),
                    TEXT_MUTED,
                    ui.font,
                    flags,
                );
                if ui.warn {
                    theme::draw_text(m, &ui.status, &to_rect(l.status), WARNING, ui.font, flags);
                }
                for (i, btn) in BUTTONS.iter().enumerate() {
                    let state = ControlState {
                        hot: ui.hover == Some(*btn),
                        pressed: ui.pressed == Some(*btn) && ui.hover == Some(*btn),
                        focused: false,
                        enabled: true,
                        checked: false,
                    };
                    theme::draw_button(
                        m,
                        &to_rect(l.buttons[i]),
                        &ui.labels[i],
                        btn.kind(),
                        state,
                        ui.dpi,
                        ui.font,
                        (false, false),
                    );
                }
            });
        });
        let _ = EndPaint(h, &ps);
    }
}

/// Vẽ một cạnh khung: nét đứt màu nhấn xen nền tối (đọc rõ trên nội dung sáng lẫn tối).
fn paint_frame(h: HWND) {
    let mut ps = PAINTSTRUCT::default();
    let mut cr = RECT::default();
    // SAFETY: cặp BeginPaint/EndPaint của cửa sổ này; `theme::fill` tạo và xóa chổi của nó.
    unsafe {
        let dc = BeginPaint(h, &mut ps);
        let _ = GetClientRect(h, &mut cr);
        let (w, ht) = (cr.right - cr.left, cr.bottom - cr.top);
        let horizontal = w >= ht;
        let len = if horizontal { w } else { ht };
        // Nét dài gấp ba bề dày (6 DIP khi khung 2 DIP).
        let dash = (3 * w.min(ht)).max(3);
        let mut at = 0;
        let mut on = true;
        while at < len {
            let end = (at + dash).min(len);
            let seg = if horizontal {
                RECT {
                    left: at,
                    top: 0,
                    right: end,
                    bottom: ht,
                }
            } else {
                RECT {
                    left: 0,
                    top: at,
                    right: w,
                    bottom: end,
                }
            };
            theme::fill(dc, &seg, if on { ACCENT } else { BG });
            on = !on;
            at = end;
        }
        let _ = EndPaint(h, &ps);
    }
}

fn run(button: BarButton) {
    match button {
        BarButton::Done => done(),
        BarButton::Cancel => cancel(),
    }
}

unsafe extern "system" fn host_proc(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_TIMER if wp.0 == TIMER_ID => {
            on_tick();
            LRESULT(0)
        }
        WM_HOTKEY if wp.0 as i32 == ESC_HOTKEY_ID => {
            cancel();
            LRESULT(0)
        }
        WM_DISPLAYCHANGE => {
            check_region();
            LRESULT(0)
        }
        // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
        _ => unsafe { DefWindowProcW(h, msg, wp, lp) },
    }
}

unsafe extern "system" fn frame_proc(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        // Xuyên chuột (cùng WS_EX_TRANSPARENT).
        WM_NCHITTEST => LRESULT(HTTRANSPARENT as isize),
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            paint_frame(h);
            LRESULT(0)
        }
        WM_DISPLAYCHANGE => {
            check_region();
            LRESULT(0)
        }
        // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
        _ => unsafe { DefWindowProcW(h, msg, wp, lp) },
    }
}

unsafe extern "system" fn bar_proc(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        // Bấm vào thanh không kích hoạt nó: ứng dụng đang cuộn giữ tiêu điểm.
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            paint_bar(h);
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let p = client_point(lp);
            let grab = with(|s| {
                let b = hit(&s.ui.layout, p);
                s.ui.pressed = b;
                s.ui.hover = b;
                b.is_some()
            });
            if grab == Some(true) {
                // SAFETY: giữ chuột cho cửa sổ của luồng này (không giữ mượn `SESSION`).
                unsafe {
                    SetCapture(h);
                }
                invalidate(h);
            }
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let p = client_point(lp);
            let (changed, track) = with(|s| {
                let now = hit(&s.ui.layout, p);
                let changed = now != s.ui.hover;
                s.ui.hover = now;
                let track = !s.ui.tracking;
                s.ui.tracking = true;
                (changed, track)
            })
            .unwrap_or((false, false));
            if track {
                let mut tme = TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: h,
                    dwHoverTime: 0,
                };
                // SAFETY: tme là biến cục bộ, cbSize đúng.
                unsafe {
                    let _ = TrackMouseEvent(&mut tme);
                }
            }
            if changed {
                invalidate(h);
            }
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            with(|s| {
                s.ui.tracking = false;
                s.ui.hover = None;
            });
            invalidate(h);
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let p = client_point(lp);
            let clicked = with(|s| {
                let pressed = s.ui.pressed.take();
                match (pressed, hit(&s.ui.layout, p)) {
                    (Some(x), Some(y)) if x == y => Some(x),
                    _ => None,
                }
            })
            .flatten();
            // SAFETY: thả chuột của luồng này (không giữ mượn `SESSION`).
            unsafe {
                let _ = ReleaseCapture();
            }
            invalidate(h);
            if let Some(btn) = clicked {
                // Kết thúc phiên hủy chính thanh này: không đụng `h` sau đó.
                run(btn);
            }
            LRESULT(0)
        }
        WM_CAPTURECHANGED => {
            with(|s| s.ui.pressed = None);
            invalidate(h);
            LRESULT(0)
        }
        WM_DPICHANGED => {
            // Thanh sang màn hình có DPI khác (hoặc đổi tỉ lệ): dựng lại chữ và cỡ.
            let dpi = u32::from((wp.0 & 0xFFFF) as u16);
            if with(|s| s.ui.dpi) != Some(dpi) {
                relayout_bar(Some(dpi));
            }
            LRESULT(0)
        }
        WM_DISPLAYCHANGE => {
            check_region();
            LRESULT(0)
        }
        // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
        _ => unsafe { DefWindowProcW(h, msg, wp, lp) },
    }
}

/// Các cửa sổ hiện của phiên (khung, thanh) cho kiểm thử chạy tay.
#[cfg(test)]
fn session_windows() -> Vec<HWND> {
    with(|s| s.frames.iter().copied().chain(s.bar).collect()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    fn overlaps(a: Rect, b: Rect) -> bool {
        a.x < b.right() && b.x < a.right() && a.y < b.bottom() && b.y < a.bottom()
    }

    #[test]
    fn frame_is_two_dip_outside_region() {
        assert_eq!(frame_thickness(96), 2);
        assert_eq!(frame_thickness(144), 3);
        assert_eq!(frame_thickness(192), 4);
        let f = frame_rects(r(100, 100, 400, 300), 2);
        assert_eq!(
            f,
            [
                r(98, 98, 404, 2),
                r(98, 400, 404, 2),
                r(98, 100, 2, 300),
                r(500, 100, 2, 300),
            ]
        );
        // Màn hình bên trái (tọa độ âm), DPI 200%.
        let region = r(-1000, -150, 300, 200);
        let f = frame_rects(region, 4);
        assert_eq!(f[0], r(-1004, -154, 308, 4));
        assert_eq!(f[3], r(-700, -150, 4, 200));
        // Không ô khung nào chồng lên vùng chụp.
        for part in f {
            assert!(!overlaps(part, region), "{part:?}");
        }
    }

    #[test]
    fn bar_goes_below_then_above_then_inside() {
        let work = r(0, 0, 1920, 1040);
        let size = (300, 40);
        // Dưới vùng, căn giữa theo vùng.
        assert_eq!(
            bar_origin(r(100, 100, 400, 300), size, 8, work, &[], true),
            Some(Point { x: 150, y: 408 })
        );
        // Không đủ chỗ dưới: trên.
        assert_eq!(
            bar_origin(r(100, 700, 400, 330), size, 8, work, &[], true),
            Some(Point { x: 150, y: 652 })
        );
        // Sát mép phải: kẹp vào vùng làm việc.
        assert_eq!(
            bar_origin(r(1800, 100, 120, 300), size, 8, work, &[], true),
            Some(Point { x: 1620, y: 408 })
        );
        // Vùng chiếm cả vùng làm việc, thanh loại khỏi ảnh chụp được: trong vùng làm việc, đáy.
        assert_eq!(
            bar_origin(work, size, 8, work, &[], true),
            Some(Point { x: 810, y: 992 })
        );
    }

    #[test]
    fn bar_on_negative_monitor() {
        let work = r(-1280, -200, 1280, 1000);
        assert_eq!(
            bar_origin(r(-1000, 0, 400, 300), (300, 40), 8, work, &[], false),
            Some(Point { x: -950, y: 308 })
        );
        // Không đủ chỗ dưới (đáy vùng làm việc ở 800): trên.
        assert_eq!(
            bar_origin(r(-1000, 400, 400, 380), (300, 40), 8, work, &[], false),
            Some(Point { x: -950, y: 352 })
        );
    }

    /// Thanh không loại khỏi ảnh chụp được: không bao giờ nằm trong vùng chụp.
    #[test]
    fn bar_never_inside_region_when_not_excluded() {
        let work = r(0, 0, 1920, 1040);
        let size = (300, 40);
        // Không có chỗ trên/dưới: bên phải, rồi bên trái.
        let region = r(0, 0, 1500, 1040);
        let at = bar_origin(region, size, 8, work, &[], false).unwrap();
        assert_eq!(at, Point { x: 1508, y: 1000 });
        let region = r(400, 0, 1520, 1040);
        let at = bar_origin(region, size, 8, work, &[], false).unwrap();
        assert_eq!(at, Point { x: 92, y: 1000 });
        // Vùng phủ kín màn hình: sang màn hình khác (đáy, giữa).
        let other = r(1920, 0, 1280, 1024);
        let at = bar_origin(work, size, 8, work, &[other], false).unwrap();
        assert_eq!(at, Point { x: 2410, y: 976 });
        // Không còn chỗ nào ngoài vùng: không đặt thanh.
        assert_eq!(bar_origin(work, size, 8, work, &[], false), None);
        // Màn hình khác cũng nằm trọn trong vùng: không đặt.
        let both = r(0, 0, 3200, 1040);
        assert_eq!(bar_origin(both, size, 8, work, &[other], false), None);
        // Nhiều vị trí ngẫu nhiên: luôn ngoài vùng và trong một vùng làm việc.
        for (x, y, w, h) in [
            (0, 0, 1920, 1000),
            (10, 10, 1900, 1020),
            (500, 300, 900, 700),
            (0, 30, 1920, 980),
        ] {
            let region = r(x, y, w, h);
            if let Some(p) = bar_origin(region, size, 8, work, &[other], false) {
                let bar = r(p.x, p.y, size.0, size.1);
                assert!(!overlaps(bar, region), "{region:?} -> {bar:?}");
                let inside = |wa: Rect| {
                    bar.x >= wa.x
                        && bar.y >= wa.y
                        && bar.right() <= wa.right()
                        && bar.bottom() <= wa.bottom()
                };
                assert!(inside(work) || inside(other), "{bar:?}");
            }
        }
    }

    /// Không đặt được trong vùng làm việc: thử cả màn hình (đè lên thanh tác vụ), vẫn ngoài vùng.
    #[test]
    fn bar_spot_falls_back_to_monitor_rect() {
        let mon = r(0, 0, 1920, 1080);
        let work = r(0, 0, 1920, 1032);
        let size = (300, 40);
        // Vùng chiếm cả vùng làm việc, không loại được: dưới vùng, trên thanh tác vụ.
        assert_eq!(
            bar_spot(work, size, 8, work, mon, &[], false),
            Some(Point { x: 810, y: 1040 })
        );
        // Vùng làm việc còn chỗ: như `bar_origin`.
        let region = r(100, 100, 400, 300);
        assert_eq!(
            bar_spot(region, size, 8, work, mon, &[], false),
            bar_origin(region, size, 8, work, &[], false)
        );
        // Vùng phủ kín màn hình: không có chỗ.
        assert_eq!(bar_spot(mon, size, 8, work, mon, &[], false), None);
        // Loại được khỏi ảnh chụp: luôn có chỗ (trong vùng làm việc).
        assert!(bar_spot(mon, size, 8, work, mon, &[], true).is_some());
    }

    #[test]
    fn region_valid_only_when_fully_on_screens() {
        let a = r(0, 0, 1920, 1080);
        let b = r(1920, 0, 1280, 1024);
        let left = r(-1280, 0, 1280, 1024);
        assert!(region_valid(r(100, 100, 400, 300), &[a, b]));
        // Vắt qua hai màn hình liền nhau.
        assert!(region_valid(r(1800, 100, 300, 300), &[a, b]));
        assert!(region_valid(r(-200, 100, 400, 300), &[left, a]));
        // Màn hình phải đã rút: phần vùng nằm ngoài.
        assert!(!region_valid(r(1800, 100, 300, 300), &[a]));
        // Phần dưới màn hình phải (cao 1024) nằm ngoài.
        assert!(!region_valid(r(1900, 1000, 100, 60), &[a, b]));
        assert!(!region_valid(r(0, 0, 0, 10), &[a]));
        assert!(!region_valid(r(0, 0, 10, 10), &[]));
    }

    #[test]
    fn machine_ends_once() {
        use darkshot_core::scroll_stitch::{Push, Reason};
        let mut m = Machine::new();
        assert_eq!(m.phase(), Phase::Running);
        assert!(!m.input(Input::Push(Push::Appended(300)), 0));
        assert!(!m.input(Input::Push(Push::NoChange), 100));
        assert!(m.input(Input::Done, 200));
        assert_eq!(m.phase(), Phase::Done);
        assert!(m.phase().delivers());
        // Đã kết thúc: mọi đầu vào sau bị bỏ.
        assert!(!m.input(Input::Cancel, 300));
        assert!(!m.input(Input::Push(Push::Limit), 300));
        assert_eq!(m.phase(), Phase::Done);

        let mut m = Machine::new();
        assert!(m.input(Input::Cancel, 0));
        assert_eq!(m.phase(), Phase::Cancelled);
        assert!(!m.phase().delivers());
        assert!(!m.input(Input::Done, 0));

        let mut m = Machine::new();
        assert!(m.input(Input::Push(Push::Limit), 0));
        assert_eq!(m.phase(), Phase::Limit);
        assert!(m.phase().delivers());

        let mut m = Machine::new();
        assert!(m.input(Input::RegionLost, 0));
        assert_eq!(m.phase(), Phase::Lost);
        assert!(m.phase().delivers());

        // Khung không ghép được: trạng thái vàng 1,5 s, vẫn chạy.
        let mut m = Machine::new();
        assert!(!m.warning(0));
        assert!(!m.input(Input::Push(Push::Uncertain(Reason::NoOverlap)), 1000));
        assert_eq!(m.phase(), Phase::Running);
        assert!(m.warning(1000));
        assert!(m.warning(2499));
        assert!(!m.warning(2500));
        // Lần nữa: gia hạn.
        m.input(Input::Push(Push::Uncertain(Reason::Backward)), 2000);
        assert!(m.warning(3499));
        // Nối được không tắt trạng thái vàng sớm.
        m.input(Input::Push(Push::Appended(10)), 2100);
        assert!(m.warning(3000));
    }

    #[test]
    fn machine_slows_timer_after_slow_ticks() {
        let mut m = Machine::new();
        assert_eq!(m.interval_ms(), INTERVAL_MS);
        assert_eq!(INTERVAL_MS, 100);
        assert_eq!(SLOW_INTERVAL_MS, 200);
        // Lượt chậm lẻ tẻ: không giãn.
        assert_eq!(m.tick_took(90), None);
        assert_eq!(m.tick_took(20), None);
        assert_eq!(m.tick_took(90), None);
        assert_eq!(m.tick_took(80), None, "80 ms chưa phải chậm");
        assert_eq!(m.interval_ms(), 100);
        // Đủ lượt chậm liên tiếp: giãn lên 200 ms đúng một lần.
        for _ in 1..SLOW_STREAK {
            assert_eq!(m.tick_took(81), None);
        }
        assert_eq!(m.tick_took(81), Some(SLOW_INTERVAL_MS));
        assert_eq!(m.interval_ms(), 200);
        assert_eq!(m.tick_took(120), None);
        // Nhanh trở lại đủ lâu: về 100 ms.
        for _ in 1..FAST_STREAK {
            assert_eq!(m.tick_took(10), None);
        }
        assert_eq!(m.tick_took(10), Some(INTERVAL_MS));
        assert_eq!(m.interval_ms(), 100);
        // Một lượt chậm cắt chuỗi nhanh.
        for _ in 0..SLOW_STREAK {
            m.tick_took(100);
        }
        assert_eq!(m.interval_ms(), 200);
        for _ in 1..FAST_STREAK {
            m.tick_took(10);
        }
        assert_eq!(m.tick_took(60), None);
        assert_eq!(m.tick_took(10), None);
        assert_eq!(m.interval_ms(), 200);
    }

    #[test]
    fn bar_layout_fits_text_at_every_dpi() {
        for dpi in [96u32, 120, 144, 192] {
            let k = |v: i32| v * dpi as i32 / 96;
            let t = Texts {
                title: k(200),
                height: k(60),
                status: k(90),
                buttons: [k(40), k(45)],
                line: k(16),
            };
            let l = layout(dpi, &t);
            let win = r(0, 0, l.size.0, l.size.1);
            let inside = |x: Rect| {
                x.x >= 0 && x.y >= 0 && x.right() <= win.right() && x.bottom() <= win.bottom()
            };
            assert!(l.size.1 >= k(BAR_H));
            for part in [
                l.dot,
                l.title,
                l.height,
                l.status,
                l.buttons[0],
                l.buttons[1],
            ] {
                assert!(inside(part), "dpi {dpi} {part:?}");
            }
            assert!(l.title.w >= t.title && l.height.w >= t.height && l.status.w >= t.status);
            // Trái sang phải, không chồng nhau.
            let order = [
                l.dot,
                l.title,
                l.height,
                l.status,
                l.buttons[0],
                l.buttons[1],
            ];
            for pair in order.windows(2) {
                assert!(pair[0].right() <= pair[1].x, "dpi {dpi} {pair:?}");
            }
            for (b, tw) in l.buttons.iter().zip(t.buttons) {
                assert!(b.w >= tw + 2 * k(BTN_PAD));
                assert!(b.h >= t.line);
            }
            // Dò ô.
            let c = |x: Rect| Point {
                x: x.x + x.w / 2,
                y: x.y + x.h / 2,
            };
            assert_eq!(hit(&l, c(l.buttons[0])), Some(BarButton::Done));
            assert_eq!(hit(&l, c(l.buttons[1])), Some(BarButton::Cancel));
            assert_eq!(hit(&l, c(l.title)), None);
        }
        assert_eq!(BUTTONS, [BarButton::Done, BarButton::Cancel]);
    }

    /// Thanh và khung chỉ được tạo khi loại khỏi ảnh chụp được (Windows 10 2004 trở lên).
    #[test]
    fn exclusion_needs_windows_10_2004() {
        assert!(!exclude_supported(None));
        assert!(!exclude_supported(Some(19040)));
        assert!(exclude_supported(Some(19041)));
        assert!(os_build().is_some_and(|b| b > 7600));
    }
}

/// Chạy tay một lượt (mở cửa sổ thật của kiểm thử, không dời chuột hay phím, không cuộn ứng dụng
/// khác): `cargo test -p darkshot-windows scroll_session::live -- --ignored --test-threads=1`.
#[cfg(test)]
mod live {
    use super::*;
    use std::cell::RefCell;
    use windows::Win32::Graphics::Dwm::DwmFlush;
    use windows::Win32::Graphics::Gdi::{
        SetDIBitsToDevice, UpdateWindow, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    };
    use windows::Win32::System::Threading::{
        GetCurrentProcess, GetGuiResources, GR_GDIOBJECTS, GR_USEROBJECTS,
    };
    use windows::Win32::UI::HiDpi::{
        SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetForegroundWindow, GetWindowDisplayAffinity, GetWindowRect, IsWindow,
        PeekMessageW, TranslateMessage, MSG, PM_REMOVE, SW_SHOWNOACTIVATE,
    };

    /// Nội dung của cửa sổ thử: cao `PAGE_H`, vẽ từ hàng `OFFSET`.
    const TEST_W: i32 = 360;
    const TEST_H: i32 = 240;
    const PAGE_H: i32 = 2000;

    thread_local! {
        static OFFSET: Cell<i32> = const { Cell::new(0) };
        static PAGE: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
        static RESULT: RefCell<Option<(Image, u32)>> = const { RefCell::new(None) };
        static ENDS: Cell<u32> = const { Cell::new(0) };
    }

    fn count_end() {
        ENDS.with(|c| c.set(c.get() + 1));
    }

    fn keep_result(img: Image, uncertain: u32) {
        RESULT.with(|r| *r.borrow_mut() = Some((img, uncertain)));
    }

    /// Trang thử: mỗi hàng một mẫu riêng (băm của chỉ số hàng), có chữ giả ở nhiều cột.
    fn make_page() -> Vec<u32> {
        let mut v = vec![0u32; (TEST_W * PAGE_H) as usize];
        for y in 0..PAGE_H {
            let mut s = (y as u32).wrapping_mul(2_654_435_761) ^ 0x9E37_79B9;
            for x in 0..TEST_W {
                s ^= s << 13;
                s ^= s >> 17;
                s ^= s << 5;
                let on = (x / 6 + y / 3) % 7 != 0 && s.is_multiple_of(3);
                v[(y * TEST_W + x) as usize] = if on { s & 0x00FF_FFFF } else { 0x00F0_F0F0 };
            }
        }
        v
    }

    fn pump_for(ms: u64) {
        let end = std::time::Instant::now() + std::time::Duration::from_millis(ms);
        while std::time::Instant::now() < end {
            let mut msg = MSG::default();
            // SAFETY: vòng bơm thông điệp chuẩn của luồng kiểm thử.
            unsafe {
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    unsafe extern "system" fn test_proc(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        if msg == WM_PAINT {
            let mut ps = PAINTSTRUCT::default();
            // SAFETY: cặp BeginPaint/EndPaint; trang thử sống trong thread_local của luồng này.
            unsafe {
                let dc = BeginPaint(h, &mut ps);
                let off = OFFSET.with(|o| o.get());
                PAGE.with(|p| {
                    let p = p.borrow();
                    let rows = &p[(off * TEST_W) as usize..((off + TEST_H) * TEST_W) as usize];
                    let bmi = BITMAPINFO {
                        bmiHeader: BITMAPINFOHEADER {
                            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                            biWidth: TEST_W,
                            biHeight: -TEST_H,
                            biPlanes: 1,
                            biBitCount: 32,
                            biCompression: BI_RGB.0,
                            ..Default::default()
                        },
                        ..Default::default()
                    };
                    SetDIBitsToDevice(
                        dc,
                        0,
                        0,
                        TEST_W as u32,
                        TEST_H as u32,
                        0,
                        0,
                        0,
                        TEST_H as u32,
                        rows.as_ptr().cast(),
                        &bmi,
                        DIB_RGB_COLORS,
                    );
                });
                let _ = EndPaint(h, &ps);
            }
            return LRESULT(0);
        }
        if msg == WM_ERASEBKGND {
            return LRESULT(1);
        }
        // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
        unsafe { DefWindowProcW(h, msg, wp, lp) }
    }

    /// Cửa sổ thử luôn trên cùng, không lấy tiêu điểm; trả cửa sổ và vùng client (tọa độ màn hình).
    fn test_window() -> (HWND, Rect) {
        test_window_with(false)
    }

    /// Như `test_window`; `layered`: cửa sổ lớp (`WS_EX_LAYERED`, độ đục 255).
    fn test_window_with(layered: bool) -> (HWND, Rect) {
        let ex = WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;
        let ex = if layered { ex | WS_EX_LAYERED } else { ex };
        // SAFETY: đăng ký lớp và tạo cửa sổ popup của luồng kiểm thử.
        unsafe {
            let hinst = GetModuleHandleW(None).unwrap();
            let class = w!("darkshot.scroll-test");
            let wc = WNDCLASSW {
                lpfnWndProc: Some(test_proc),
                hInstance: hinst.into(),
                lpszClassName: class,
                ..Default::default()
            };
            RegisterClassW(&wc);
            let h = CreateWindowExW(
                ex,
                class,
                w!("scroll test"),
                WS_POPUP,
                200,
                150,
                TEST_W,
                TEST_H,
                None,
                None,
                Some(hinst.into()),
                None,
            )
            .unwrap();
            if layered {
                SetLayeredWindowAttributes(h, COLORREF(0), 255, LWA_ALPHA).unwrap();
            }
            let _ = ShowWindow(h, SW_SHOWNOACTIVATE);
            let _ = UpdateWindow(h);
            let mut cr = RECT::default();
            let _ = GetClientRect(h, &mut cr);
            let mut wr = RECT::default();
            let _ = GetWindowRect(h, &mut wr);
            (h, r_of(wr.left, wr.top, cr.right, cr.bottom))
        }
    }

    fn r_of(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    fn scroll_to(h: HWND, off: i32) {
        OFFSET.with(|o| o.set(off));
        // SAFETY: vẽ lại cửa sổ thử của chính kiểm thử rồi chờ DWM trình bày.
        unsafe {
            let _ = InvalidateRect(Some(h), None, false);
            let _ = UpdateWindow(h);
            let _ = DwmFlush();
        }
    }

    fn gui(kind: windows::Win32::System::Threading::GET_GUI_RESOURCES_FLAGS) -> u32 {
        // SAFETY: chỉ đọc bộ đếm của tiến trình hiện tại.
        unsafe { GetGuiResources(GetCurrentProcess(), kind) }
    }

    fn dpi_aware() {
        // SAFETY: đặt chế độ DPI cho luồng kiểm thử (như manifest của ứng dụng).
        unsafe {
            SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
    }

    thread_local! {
        static CAPTURE_FAILS: Cell<u32> = const { Cell::new(0) };
    }

    fn count_capture_failed() {
        CAPTURE_FAILS.with(|c| c.set(c.get() + 1));
    }

    /// Xong khi chưa có khung nào mà chụp vùng lỗi: báo lỗi chụp, không giao ảnh, phiên vẫn dọn.
    #[test]
    #[ignore]
    fn done_without_frames_reports_capture_failure() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        dpi_aware();
        PAGE.with(|p| *p.borrow_mut() = make_page());
        let (tw, region) = test_window();
        pump_for(100);
        set_on_result(keep_result);
        set_on_capture_failed(count_capture_failed);
        CAPTURE_FAILS.with(|c| c.set(0));
        assert_eq!(start(region), Ok(()));
        // Trước nhịp hẹn giờ đầu: chưa có khung.
        done_with(|_| Err(windows::core::Error::from_hresult(windows::Win32::Foundation::E_FAIL)));
        assert!(!active());
        assert_eq!(CAPTURE_FAILS.with(|c| c.get()), 1);
        assert!(RESULT.with(|r| r.borrow_mut().take()).is_none());
        // Chụp được: không báo lỗi.
        assert_eq!(start(region), Ok(()));
        done();
        assert_eq!(CAPTURE_FAILS.with(|c| c.get()), 1);
        assert!(RESULT.with(|r| r.borrow_mut().take()).is_some());
        // SAFETY: hủy cửa sổ thử của chính kiểm thử.
        unsafe {
            let _ = DestroyWindow(tw);
        }
        pump_for(50);
    }

    #[test]
    #[ignore]
    fn session_stitches_test_window_scroll() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        dpi_aware();
        PAGE.with(|p| *p.borrow_mut() = make_page());
        OFFSET.with(|o| o.set(0));
        let (tw, region) = test_window();
        pump_for(300);
        set_on_result(keep_result);
        // SAFETY: chỉ đọc cửa sổ đang có tiêu điểm.
        let fg = unsafe { GetForegroundWindow() };
        assert_eq!(start(region), Ok(()));
        assert!(active());
        // Bắt đầu lại khi đang chạy: từ chối.
        assert_eq!(start(region), Err(StartError::Busy));
        // Đổi ngôn ngữ giữa phiên: thanh dựng lại, phiên vẫn chạy.
        relabel();
        assert!(active());
        // SAFETY: chỉ đọc cửa sổ đang có tiêu điểm.
        assert_eq!(unsafe { GetForegroundWindow() }, fg, "không lấy tiêu điểm");
        pump_for(350);
        let step = 60;
        let steps = 8;
        for i in 1..=steps {
            scroll_to(tw, i * step);
            pump_for(300);
        }
        done();
        assert!(!active());
        let (img, uncertain) = RESULT.with(|r| r.borrow_mut().take()).expect("có kết quả");
        assert_eq!(uncertain, 0);
        assert_eq!(img.width, region.w as u32);
        assert_eq!(img.height, (region.h + steps * step) as u32);
        // Ảnh ghép đúng nội dung trang (vài hàng mẫu).
        PAGE.with(|p| {
            let p = p.borrow();
            for y in [0, 100, 239, 300, 500, 719] {
                for x in [0, 77, 359] {
                    let i = ((y * region.w + x) * 4) as usize;
                    let got =
                        u32::from_le_bytes([img.bgra[i], img.bgra[i + 1], img.bgra[i + 2], 0]);
                    assert_eq!(got, p[(y * TEST_W + x) as usize], "({x}, {y})");
                }
            }
        });
        // SAFETY: hủy cửa sổ thử của chính kiểm thử.
        unsafe {
            let _ = DestroyWindow(tw);
        }
        pump_for(50);
    }

    #[test]
    #[ignore]
    fn windows_are_excluded_and_session_does_not_leak() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        dpi_aware();
        PAGE.with(|p| *p.borrow_mut() = make_page());
        let (tw, region) = test_window();
        pump_for(200);
        set_on_result(keep_result);
        set_on_end(count_end);
        ENDS.with(|c| c.set(0));
        let supported = exclude_supported(os_build());
        let (mut gdi0, mut user0) = (0, 0);
        for i in 0..11 {
            if i == 1 {
                gdi0 = gui(GR_GDIOBJECTS);
                user0 = gui(GR_USEROBJECTS);
            }
            assert_eq!(start(region), Ok(()));
            pump_for(150);
            // Đổi ngôn ngữ và dựng lại theo DPI khác giữa phiên: không rò.
            relabel();
            relayout_bar(Some(if i % 2 == 0 { 144 } else { 96 }));
            pump_for(30);
            let ws = session_windows();
            if supported {
                // Bốn khung và thanh, tất cả loại khỏi ảnh chụp.
                assert_eq!(ws.len(), 5);
                for h in &ws {
                    let mut aff = 0u32;
                    // SAFETY: h là cửa sổ của tiến trình này; aff là biến cục bộ.
                    unsafe { GetWindowDisplayAffinity(*h, &mut aff) }.unwrap();
                    assert_eq!(aff, WDA_EXCLUDEFROMCAPTURE.0);
                }
            }
            // Xen kẽ Hủy (bỏ) và Xong (giao kết quả).
            if i % 2 == 0 {
                cancel();
                assert!(RESULT.with(|r| r.borrow_mut().take()).is_none());
            } else {
                done();
                assert!(RESULT.with(|r| r.borrow_mut().take()).is_some());
            }
            pump_for(30);
            assert!(!active());
            // Hàm kết thúc chạy đúng một lần mỗi phiên (Hủy lẫn Xong).
            assert_eq!(ENDS.with(|c| c.get()), i + 1);
            for h in ws {
                // SAFETY: chỉ hỏi cửa sổ đã hủy có còn không.
                assert!(!unsafe { IsWindow(Some(h)) }.as_bool());
            }
        }
        let (gdi1, user1) = (gui(GR_GDIOBJECTS), gui(GR_USEROBJECTS));
        eprintln!("GDI {gdi0} -> {gdi1}, USER {user0} -> {user1}");
        assert!(gdi1 <= gdi0 + 2, "GDI {gdi0} -> {gdi1}");
        assert!(user1 <= user0 + 2, "USER {user0} -> {user1}");
        // SAFETY: hủy cửa sổ thử của chính kiểm thử.
        unsafe {
            let _ = DestroyWindow(tw);
        }
        pump_for(50);
    }

    /// `capture_rect` chỉ dùng SRCCOPY (không CAPTUREBLT): khi DWM bật, nội dung của cửa sổ lớp
    /// (`WS_EX_LAYERED`) vẫn có trong ảnh chụp.
    #[test]
    #[ignore]
    fn capture_rect_sees_layered_window() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        dpi_aware();
        PAGE.with(|p| *p.borrow_mut() = make_page());
        OFFSET.with(|o| o.set(0));
        let (tw, region) = test_window_with(true);
        pump_for(300);
        // SAFETY: chờ DWM trình bày khung có cửa sổ thử.
        unsafe {
            let _ = DwmFlush();
        }
        let img = crate::capture::capture_rect(region).unwrap();
        assert_eq!((img.width, img.height), (TEST_W as u32, TEST_H as u32));
        PAGE.with(|p| {
            let p = p.borrow();
            for y in [0, 57, 120, 239] {
                for x in [0, 33, 200, 359] {
                    let i = ((y * TEST_W + x) * 4) as usize;
                    let got =
                        u32::from_le_bytes([img.bgra[i], img.bgra[i + 1], img.bgra[i + 2], 0]);
                    assert_eq!(got, p[(y * TEST_W + x) as usize], "({x}, {y})");
                }
            }
        });
        // SAFETY: hủy cửa sổ thử của chính kiểm thử.
        unsafe {
            let _ = DestroyWindow(tw);
        }
        pump_for(50);
    }
}
