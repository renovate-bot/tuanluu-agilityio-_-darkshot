//! Phiên chụp cuộn: khung vùng (bốn cửa sổ mảnh), thanh điều khiển nổi, Esc toàn cục tạm, và một
//! luồng nền `darkshot-scroll` tự chụp vùng (`capture_rect`) rồi ghép bằng
//! `darkshot_core::scroll_stitch::Stitcher`, báo kết quả về bằng `WM_SCROLL_WORK`. Luồng giao diện
//! chỉ giữ máy trạng thái, thanh và nút; không bao giờ chụp hay ghép (Task 145, quyết định 163).
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
use darkshot_core::scroll_stitch::{Push, Reason, Stitcher, MAX_HEIGHT};
use darkshot_core::theme::{ACCENT, BG, DANGER_BUTTON, LINE, SURFACE, TEXT, TEXT_MUTED, WARNING};
use std::cell::{Cell, RefCell};
use std::sync::atomic::{AtomicIsize, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};
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
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetWindowRect, IsWindow,
    KillTimer, LoadCursorW, PostMessageW, RegisterClassW, SetLayeredWindowAttributes, SetTimer,
    SetWindowDisplayAffinity, SetWindowPos, ShowWindow, HTTRANSPARENT, IDC_ARROW, LWA_ALPHA,
    MA_NOACTIVATE, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, SW_HIDE, SW_SHOWNA,
    WDA_EXCLUDEFROMCAPTURE, WINDOW_EX_STYLE, WM_APP, WM_CAPTURECHANGED, WM_DISPLAYCHANGE,
    WM_DPICHANGED, WM_ERASEBKGND, WM_HOTKEY, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEACTIVATE,
    WM_MOUSEMOVE, WM_NCHITTEST, WM_PAINT, WM_TIMER, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
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

/// Luồng nền báo có kết quả mới (lượt chụp hoặc ảnh ghép) cho cửa sổ chủ của phiên. Số kế tiếp
/// chưa dùng trong cả repo (WM_APP + 1 … + 24 đã dùng).
pub const WM_SCROLL_WORK: u32 = WM_APP + 25;

/// Hẹn giờ của cửa sổ chủ: hết hạn trạng thái vàng và lưới an toàn khi mất `WM_SCROLL_WORK`.
const TIMER_ID: usize = 1;
const UI_TICK_MS: u32 = 250;
/// Nghỉ tối thiểu giữa hai lượt chụp của luồng nền (máy chậm hơn nhịp: không chụp liền mạch).
const MIN_REST_MS: u32 = 10;
/// Nhịp chụp thường và nhịp giãn khi máy chậm (luồng nền).
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
pub(crate) fn exclude_supported(build: Option<u32>) -> bool {
    build.is_some_and(|b| b >= EXCLUDE_MIN_BUILD)
}

/// Số bản dựng Windows (`CurrentBuildNumber` trong registry; không phụ thuộc manifest). Như
/// `steps_bar` (hàm của nó riêng tư, gói task không cho sửa file đó).
pub(crate) fn os_build() -> Option<u32> {
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
pub(crate) fn frame_thickness(dpi: u32) -> i32 {
    scale(FRAME_DIP, dpi).max(1)
}

/// Bốn ô khung bao ngoài `region`, dày `t`: trên, dưới (gồm góc), trái, phải.
pub(crate) fn frame_rects(region: Rect, t: i32) -> [Rect; 4] {
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
pub(crate) fn region_valid(region: Rect, monitors: &[Rect]) -> bool {
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
    /// Lý do của khung không ghép được gần nhất (chữ của trạng thái vàng).
    warn_reason: Option<Reason>,
}

impl Machine {
    fn new() -> Machine {
        Machine {
            phase: Phase::Running,
            interval: INTERVAL_MS,
            slow: 0,
            fast: 0,
            warn_until: None,
            warn_reason: None,
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
            Input::Push(Push::Uncertain(r)) => {
                self.warn_until = Some(now_ms + WARN_MS);
                self.warn_reason = Some(r);
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

    /// Chữ của trạng thái vàng lúc `now_ms` (theo lý do mới nhất), `None` khi không cảnh báo.
    fn warning_msg(&self, now_ms: u64) -> Option<Msg> {
        self.warning(now_ms)
            .then_some(self.warn_reason)
            .flatten()
            .map(warn_msg)
    }

    /// Kết quả một lượt chụp của luồng nền (`WM_SCROLL_WORK`) lúc `now_ms`: trả (phiên vừa kết
    /// thúc, nhịp mới khi nhịp đổi). Phiên đã kết thúc (đang chờ ảnh ghép) thì bỏ qua.
    fn tick(&mut self, t: Tick, now_ms: u64) -> (bool, Option<u32>) {
        if self.phase != Phase::Running {
            return (false, None);
        }
        let ended = t.push.is_some_and(|p| self.input(Input::Push(p), now_ms));
        (ended, self.tick_took(t.took_ms))
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

/// Chữ trạng thái vàng cho lý do một khung không ghép được.
fn warn_msg(r: Reason) -> Msg {
    match r {
        Reason::Backward => Msg::ScrollDownNotUp,
        Reason::Changed => Msg::ScrollContentChanging,
        Reason::NoOverlap => Msg::ScrollBackUp,
        Reason::Ambiguous | Reason::SizeMismatch => Msg::ScrollSlowDown,
    }
}

/// Mọi chữ trạng thái vàng (thanh dành chỗ cho chữ dài nhất).
const WARN_MSGS: [Msg; 4] = [
    Msg::ScrollSlowDown,
    Msg::ScrollBackUp,
    Msg::ScrollDownNotUp,
    Msg::ScrollContentChanging,
];

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
/// cho chữ trạng thái dài nhất, `WARN_MSGS`), nhãn hai nút; chiều cao dòng.
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
    /// Chữ trạng thái vàng đang được vẽ (`None`: không cảnh báo).
    warn: Option<Msg>,
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

/// Lệnh từ luồng giao diện tới luồng nền.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cmd {
    /// Nhịp chụp mới (ms), áp dụng từ lượt sau.
    Interval(u32),
    /// Kết thúc có giao ảnh: `grab` (chỉ Xong) chụp một khung khi chưa có khung nào, rồi `finish`
    /// và gửi ảnh ghép. Luồng nền dừng sau đó.
    Finish { grab: bool },
}

/// Một lượt chụp của luồng nền: kết quả ghép (`None`: chụp lỗi), chiều cao ảnh ghép sau lượt, thời
/// gian chụp + ghép (ms).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Tick {
    push: Option<Push>,
    height: u32,
    took_ms: u32,
}

/// Kết quả luồng nền gửi về luồng giao diện.
#[derive(Debug)]
enum Event {
    Tick(Tick),
    /// Ảnh ghép, số chỗ không chắc chắn, cờ "chưa có khung mà lần chụp cuối lỗi" và cờ đã chạm
    /// giới hạn (kể cả khi lượt `Push::Limit` chưa kịp được luồng giao diện xử lý trước Xong).
    Finished {
        image: Image,
        uncertain: u32,
        capture_failed: bool,
        limited: bool,
    },
}

/// Hộp kết quả chung (luồng nền ghi, luồng giao diện lấy hết). Chỉ chứa lượt chụp nhỏ và đúng một
/// ảnh ghép cuối: không có hàng đợi khung.
type Events = Arc<Mutex<Vec<Event>>>;

/// Số luồng nền còn chạy (kiểm thử chờ luồng dừng hẳn trước khi đếm tài nguyên).
static WORKERS_LIVE: AtomicUsize = AtomicUsize::new(0);

/// Giảm `WORKERS_LIVE` khi luồng nền ra khỏi vòng (mọi đường).
struct LiveGuard;

impl Drop for LiveGuard {
    fn drop(&mut self) {
        WORKERS_LIVE.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Đầu luồng giao diện của luồng nền. Drop (Hủy, Esc, thoát, đăng xuất) đóng kênh lệnh: luồng nền
/// dừng ở lần chờ kế tiếp hoặc ngay sau lần chụp đang dở, không giao gì. Không chờ luồng nền.
struct Worker {
    tx: Sender<Cmd>,
    events: Events,
    handle: std::thread::JoinHandle<()>,
}

impl Worker {
    /// Chạy luồng `darkshot-scroll`: lượt đầu sau `interval` ms, mỗi lượt gọi `source(region)` rồi
    /// `push`, ghi kết quả và gọi `wake` (đăng `WM_SCROLL_WORK`; `false` chỉ khi cửa sổ chủ không
    /// còn: luồng dừng. Đăng lỗi mà cửa sổ còn thì vẫn chạy, hẹn giờ giao diện lấy kết quả).
    /// `None` khi không tạo được luồng.
    fn spawn<S, W>(region: Rect, interval: u32, source: S, wake: W) -> Option<Worker>
    where
        S: FnMut(Rect) -> Result<Image, ()> + Send + 'static,
        W: Fn() -> bool + Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        let events: Events = Arc::new(Mutex::new(Vec::new()));
        let out = events.clone();
        WORKERS_LIVE.fetch_add(1, Ordering::AcqRel);
        let spawned = std::thread::Builder::new()
            .name("darkshot-scroll".into())
            .spawn(move || {
                let _live = LiveGuard;
                worker_loop(region, interval, rx, &out, source, wake);
            });
        let Ok(handle) = spawned else {
            WORKERS_LIVE.fetch_sub(1, Ordering::AcqRel);
            return None;
        };
        Some(Worker { tx, events, handle })
    }

    /// Gửi lệnh; `false` khi luồng nền đã dừng.
    fn send(&self, c: Cmd) -> bool {
        self.tx.send(c).is_ok()
    }

    /// Luồng nền đã ra khỏi vòng. Kiểm TRƯỚC `take_events`: mọi kết quả nó ghi đã nằm trong hộp.
    fn dead(&self) -> bool {
        self.handle.is_finished()
    }

    /// Lấy hết kết quả đang chờ, theo thứ tự.
    fn take_events(&self) -> Vec<Event> {
        std::mem::take(&mut *self.events.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

/// Ghi một kết quả rồi đánh thức luồng giao diện; `false` khi cửa sổ chủ không còn.
fn post(events: &Events, e: Event, wake: &impl Fn() -> bool) -> bool {
    events
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(e);
    wake()
}

/// Vòng của luồng nền: mỗi lúc chỉ một lượt chụp (chụp, ghép, rồi mới hẹn lượt sau), không giữ
/// khung nào ngoài khung đang ghép. Dừng khi kênh lệnh đóng, cửa sổ chủ không còn, hoặc sau `Finish`.
fn worker_loop<S, W>(
    region: Rect,
    mut interval: u32,
    rx: Receiver<Cmd>,
    events: &Events,
    mut source: S,
    wake: W,
) where
    S: FnMut(Rect) -> Result<Image, ()>,
    W: Fn() -> bool,
{
    let mut st = Stitcher::new(region.w.max(0) as u32, region.h.max(0) as u32);
    let mut next = Instant::now() + Duration::from_millis(u64::from(interval));
    // Đạt giới hạn: không chụp nữa, chỉ chờ Finish hoặc Hủy.
    let mut limited = false;
    loop {
        let cmd = if limited {
            rx.recv().map_err(|_| RecvTimeoutError::Disconnected)
        } else {
            rx.recv_timeout(next.saturating_duration_since(Instant::now()))
        };
        match cmd {
            Ok(Cmd::Interval(ms)) => {
                interval = ms;
                continue;
            }
            Ok(Cmd::Finish { grab }) => {
                finish_and_post(st, grab, limited, region, &rx, events, &mut source, &wake);
                return;
            }
            Err(RecvTimeoutError::Disconnected) => return,
            Err(RecvTimeoutError::Timeout) => {}
        }
        let t0 = Instant::now();
        let frame = source(region);
        // Lệnh tới trong lúc chụp: Hủy thì bỏ khung vừa chụp, không ghép, không giao.
        let mut finish = None;
        loop {
            match rx.try_recv() {
                Ok(Cmd::Interval(ms)) => interval = ms,
                Ok(Cmd::Finish { grab }) => {
                    finish = Some(grab);
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }
        let push = frame.ok().map(|f| st.push(&f));
        let took_ms = u32::try_from(t0.elapsed().as_millis()).unwrap_or(u32::MAX);
        let tick = Tick {
            push,
            height: st.height(),
            took_ms,
        };
        if !post(events, Event::Tick(tick), &wake) {
            return;
        }
        limited |= push == Some(Push::Limit);
        if let Some(grab) = finish {
            finish_and_post(st, grab, limited, region, &rx, events, &mut source, &wake);
            return;
        }
        let rest = Duration::from_millis(u64::from(interval))
            .saturating_sub(t0.elapsed())
            .max(Duration::from_millis(u64::from(MIN_REST_MS)));
        next = Instant::now() + rest;
    }
}

/// Lượt cuối: chụp một khung khi `grab` và chưa có khung nào, rồi gửi ảnh ghép kèm cờ `limited`
/// (đã chạm giới hạn). Đã Hủy (kênh đóng) thì không ghép, không giao.
#[allow(clippy::too_many_arguments)]
fn finish_and_post<S, W>(
    mut st: Stitcher,
    grab: bool,
    mut limited: bool,
    region: Rect,
    rx: &Receiver<Cmd>,
    events: &Events,
    source: &mut S,
    wake: &W,
) where
    S: FnMut(Rect) -> Result<Image, ()>,
    W: Fn() -> bool,
{
    let mut capture_failed = false;
    if grab && st.height() == 0 {
        match source(region) {
            Ok(img) => limited |= st.push(&img) == Push::Limit,
            Err(()) => capture_failed = true,
        }
    }
    if matches!(rx.try_recv(), Err(TryRecvError::Disconnected)) {
        return;
    }
    let (image, uncertain) = st.finish();
    let e = Event::Finished {
        image,
        uncertain,
        capture_failed,
        limited,
    };
    post(events, e, wake);
}

/// Hàm chụp của phiên thật.
fn capture_source(r: Rect) -> Result<Image, ()> {
    capture::capture_rect(r).map_err(|_| ())
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
/// hẹn giờ, phím Esc tạm, mọi cửa sổ và phông, và thả luồng nền (nó tự dừng, không giao gì), nên
/// mọi đường kết thúc đều dọn sạch mà không chờ.
struct Session {
    /// Vùng chụp (tọa độ màn hình, pixel vật lý).
    region: Rect,
    worker: Worker,
    /// Cửa sổ chủ cho hàm đánh thức của luồng nền (0 khi phiên đã dọn).
    wake_to: Arc<AtomicIsize>,
    /// Chiều cao ảnh ghép theo lượt chụp mới nhất.
    height: u32,
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
        // Luồng nền không đăng thông điệp tới cửa sổ chủ sắp hủy nữa (và tự dừng).
        self.wake_to.store(0, Ordering::Release);
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
        // Phông của thanh được xóa sau thân hàm này, khi `ui` drop (cửa sổ thanh đã hủy); `worker`
        // drop đóng kênh lệnh.
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
    static ON_CAPTURE_FAILED: Cell<fn()> = const { Cell::new(noop) };
}

/// Hàm báo không chụp được vùng (Xong khi chưa có khung nào và lần chụp cuối lỗi; main: thông báo
/// "Không chụp được màn hình").
pub fn set_on_capture_failed(f: fn()) {
    ON_CAPTURE_FAILED.with(|c| c.set(f));
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
pub(crate) fn monitors() -> Vec<(Rect, Rect)> {
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
pub(crate) struct Place {
    work: Rect,
    mon: Rect,
    pub(crate) dpi: u32,
    others: Vec<Rect>,
}

/// Thông tin màn hình gần `region` nhất (xem `Place`).
pub(crate) fn placement(region: Rect) -> Place {
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
pub(crate) fn place(region: Rect, size: (i32, i32), dpi: u32, excluded: bool) -> Option<Point> {
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
pub(crate) fn measure(font: Option<HFONT>, parts: &[&[u16]]) -> Vec<(i32, i32)> {
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
    let warns = WARN_MSGS.map(|m| wide(lang::tr(m)));
    let labels = BUTTONS.map(|b| wide(lang::tr(b.label())));
    let mut parts: Vec<&[u16]> = vec![&title, &widest, &labels[0], &labels[1]];
    parts.extend(warns.iter().map(|w| w.as_slice()));
    let m = measure(font, &parts);
    let texts = Texts {
        title: m[0].0,
        height: m[1].0,
        status: m[4..].iter().map(|x| x.0).max().unwrap_or(0),
        buttons: [m[2].0, m[3].0],
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
        warn: None,
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
    start_with(region, capture_source)
}

/// Thân của `start` với hàm chụp `source` (chạy trên luồng nền).
fn start_with<S>(region: Rect, source: S) -> Result<(), StartError>
where
    S: FnMut(Rect) -> Result<Image, ()> + Send + 'static,
{
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
    let wake_to = Arc::new(AtomicIsize::new(host.0 as isize));
    let wake = {
        let to = wake_to.clone();
        move || {
            let raw = to.load(Ordering::Acquire);
            if raw == 0 {
                return false;
            }
            let h = HWND(raw as *mut core::ffi::c_void);
            #[cfg(test)]
            match TEST_WAKE.load(Ordering::Acquire) {
                WAKE_DROP_POSTS => return true,
                WAKE_HOST_GONE => return false,
                _ => {}
            }
            // SAFETY: HWND không `Send` nên đi qua luồng dưới dạng số; PostMessageW và IsWindow
            // được phép gọi từ mọi luồng. Đăng lỗi (hàng đợi đầy) không dừng luồng nền: hẹn giờ
            // giao diện vẫn lấy kết quả từ hộp chung; chỉ dừng khi cửa sổ chủ không còn.
            unsafe {
                let _ = PostMessageW(Some(h), WM_SCROLL_WORK, WPARAM(0), LPARAM(0));
                IsWindow(Some(h)).as_bool()
            }
        }
    };
    // Lượt chụp đầu sau một nhịp, như hẹn giờ cũ: cửa sổ của phiên đã loại khỏi ảnh chụp (hoặc nằm
    // ngoài vùng) trước khi hiện.
    let Some(worker) = Worker::spawn(region, INTERVAL_MS, source, wake) else {
        drop(ui);
        destroy(bar);
        frames.into_iter().for_each(destroy);
        if esc {
            // SAFETY: gỡ phím tắt vừa đăng ký cho cửa sổ chủ của luồng này.
            unsafe {
                let _ = UnregisterHotKey(Some(host), ESC_HOTKEY_ID);
            }
        }
        destroy(host);
        return Err(StartError::Failed);
    };
    let session = Session {
        region,
        worker,
        wake_to,
        height: 0,
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
    if unsafe { SetTimer(Some(host), TIMER_ID, UI_TICK_MS, None) } == 0 {
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
        (
            s.region,
            s.ui.dpi,
            s.bar,
            s.bar_excluded,
            s.ui.warn,
            s.height,
        )
    }) else {
        return;
    };
    let Some(bar) = bar else {
        return;
    };
    let mut ui = build_ui(dpi.unwrap_or(cur));
    ui.height = height_text(px);
    ui.warn = warn;
    if let Some(m) = warn {
        ui.status = wide(lang::tr(m));
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

/// Xong: kết thúc phiên và giao ảnh ghép. Luồng nền chụp một khung nếu chưa có khung nào (bấm rất
/// sớm), rồi gửi ảnh ghép; cửa sổ của phiên ẩn ngay trong lúc chờ.
pub fn done() {
    end_with(Input::Done);
}

/// Kiểm thử giả lập đánh thức: đăng thông điệp lỗi (cửa sổ còn), hoặc báo cửa sổ chủ không còn.
#[cfg(test)]
static TEST_WAKE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
#[cfg(test)]
const WAKE_DROP_POSTS: u8 = 1;
#[cfg(test)]
const WAKE_HOST_GONE: u8 = 2;

/// Luồng nền đã chết mà phiên chưa xong (không còn ai giao ảnh): kết thúc phiên, không giao ảnh,
/// báo không chụp được. Không bao giờ để phiên kẹt (`active()` đúng mãi chặn mọi lần chụp).
fn end_dead_worker() {
    if active() {
        finish_ended(None);
        ON_CAPTURE_FAILED.with(|c| c.get())();
    }
}

/// Hủy (nút Hủy, Esc, thoát ứng dụng, đăng xuất): bỏ hết, không giao ảnh, không chờ luồng nền.
/// Đang chờ ảnh ghép (sau Xong, giới hạn, vùng mất) thì cũng bỏ.
pub fn cancel() {
    end_with(Input::Cancel);
    if with(|s| s.machine.phase() != Phase::Running) == Some(true) {
        finish_ended(None);
    }
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
        on_ended(i == Input::Done);
    }
}

/// Phiên vừa sang giai đoạn kết thúc. Hủy: dọn ngay. Có giao ảnh: báo luồng nền chụp lần cuối
/// (`grab`: chỉ Xong) và ghép xong, ẩn cửa sổ, gỡ Esc tạm; `finish_ended` chạy khi ảnh tới.
fn on_ended(grab: bool) {
    let Some((delivers, sent, wins, host, esc)) = with(|s| {
        let delivers = s.machine.phase().delivers();
        let sent = delivers && s.worker.send(Cmd::Finish { grab });
        let esc = std::mem::take(&mut s.esc);
        let wins: Vec<HWND> = s.frames.iter().copied().chain(s.bar).collect();
        (delivers, sent, wins, s.host, esc)
    }) else {
        return;
    };
    if esc {
        // SAFETY: gỡ phím Esc tạm của cửa sổ chủ trên luồng này (Drop không gỡ lần nữa).
        unsafe {
            let _ = UnregisterHotKey(Some(host), ESC_HOTKEY_ID);
        }
    }
    if !delivers {
        finish_ended(None);
        return;
    }
    // Luồng nền đã dừng (kênh lệnh đóng): ảnh ghép sẽ không bao giờ tới. Kết quả nó đã ghi (nếu
    // có) được lấy trước.
    if !sent {
        on_work();
        end_dead_worker();
        return;
    }
    for h in wins {
        // SAFETY: ẩn cửa sổ của phiên trên luồng này (không giữ mượn `SESSION`).
        unsafe {
            let _ = ShowWindow(h, SW_HIDE);
        }
    }
}

/// Ảnh ghép luồng nền gửi về: ảnh, số chỗ không chắc chắn, chưa có khung mà lần chụp cuối lỗi, đã
/// chạm giới hạn.
type Finished = (Image, u32, bool, bool);

/// Lấy trạng thái ra, dọn cửa sổ (Drop), rồi `ON_END` → `ON_RESULT` → `ON_LIMIT`, và
/// `ON_CAPTURE_FAILED` khi Xong mà không chụp được khung nào. `None`: Hủy, không giao gì.
fn finish_ended(result: Option<Finished>) {
    let Some(s) = SESSION.with(|c| c.try_borrow_mut().ok().and_then(|mut x| x.take())) else {
        return;
    };
    let phase = s.machine.phase();
    drop(s);
    ON_END.with(|c| c.get())();
    let Some(f) = result else {
        return;
    };
    let (show, limit, failed) = deliveries(phase, &f);
    let (img, uncertain) = (f.0, f.1);
    let height = img.height;
    if show {
        ON_RESULT.with(|c| c.get())(img, uncertain);
    }
    if limit {
        ON_LIMIT.with(|c| c.get())(height);
    }
    if failed {
        ON_CAPTURE_FAILED.with(|c| c.get())();
    }
}

/// Giao gì cho ảnh ghép `f` khi phiên kết thúc ở `phase`: (hàm kết quả, báo giới hạn, báo không
/// chụp được), theo thứ tự đó. Giới hạn theo giai đoạn HOẶC cờ của luồng nền: lượt `Push::Limit`
/// có thể chưa được xử lý khi người dùng bấm Xong.
fn deliveries(phase: Phase, f: &Finished) -> (bool, bool, bool) {
    if !phase.delivers() {
        return (false, false, false);
    }
    let (img, _, failed, limited) = f;
    let show = img.width > 0 && img.height > 0;
    (show, phase == Phase::Limit || *limited, *failed)
}

/// `WM_SCROLL_WORK` hoặc nhịp hẹn giờ giao diện: nhận kết quả luồng nền (máy trạng thái, chiều
/// cao, nhịp), cập nhật trạng thái vàng, giao ảnh ghép khi đã có. Không chụp, không ghép.
fn on_work() {
    let Some((finished, ended, repaint, dead)) = with(|s| {
        let now = now_ms(s);
        // Trước `take_events`: luồng đã dừng thì mọi kết quả của nó đã nằm trong hộp.
        let dead = s.worker.dead();
        let events = s.worker.take_events();
        let out = apply_events(&mut s.machine, events, now);
        s.height = out.height.unwrap_or(s.height);
        let mut repaint = false;
        if out.appended {
            s.ui.height = height_text(s.height);
            repaint = true;
        }
        if let Some(ms) = out.interval {
            s.worker.send(Cmd::Interval(ms));
        }
        let warn = s.machine.warning_msg(now);
        if warn != s.ui.warn {
            s.ui.warn = warn;
            s.ui.status = warn.map_or_else(Vec::new, |m| wide(lang::tr(m)));
            repaint = true;
        }
        (
            out.finished,
            out.ended,
            repaint.then_some(s.bar).flatten(),
            dead,
        )
    }) else {
        return;
    };
    if let Some(f) = finished {
        finish_ended(Some(f));
        return;
    }
    if dead {
        end_dead_worker();
        return;
    }
    if ended {
        on_ended(false);
        return;
    }
    if let Some(b) = repaint {
        invalidate(b);
    }
}

/// Kết quả của một loạt thông điệp từ luồng nền sau khi máy trạng thái đã nhận.
#[derive(Debug, Default)]
struct Applied {
    /// Phiên vừa kết thúc (giới hạn).
    ended: bool,
    /// Nhịp mới cho luồng nền.
    interval: Option<u32>,
    /// Chiều cao ảnh ghép theo lượt mới nhất.
    height: Option<u32>,
    /// Có lượt nối được (vẽ lại chiều cao).
    appended: bool,
    /// Ảnh ghép đã về.
    finished: Option<Finished>,
}

/// Thời gian các lượt luồng nền báo về (chỉ kiểm thử đo).
#[cfg(test)]
static TOOK_MS: Mutex<Vec<u32>> = Mutex::new(Vec::new());

/// Máy trạng thái nhận các kết quả `events` (theo thứ tự) lúc `now_ms`.
fn apply_events(m: &mut Machine, events: Vec<Event>, now_ms: u64) -> Applied {
    let mut out = Applied::default();
    for e in events {
        match e {
            Event::Tick(t) => {
                #[cfg(test)]
                TOOK_MS
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .push(t.took_ms);
                let (ended, interval) = m.tick(t, now_ms);
                out.ended |= ended;
                out.interval = interval.or(out.interval);
                out.height = Some(t.height);
                out.appended |= matches!(t.push, Some(Push::Appended(_)));
            }
            Event::Finished {
                image,
                uncertain,
                capture_failed,
                limited,
            } => out.finished = Some((image, uncertain, capture_failed, limited)),
        }
    }
    out
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
                if ui.warn.is_some() {
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
pub(crate) fn paint_frame(h: HWND) {
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
        // Tới sau khi phiên đã dọn: `on_work` bỏ qua.
        WM_TIMER if wp.0 == TIMER_ID => {
            on_work();
            LRESULT(0)
        }
        WM_SCROLL_WORK => {
            on_work();
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

    /// Trang thử cho luồng nền: mỗi điểm ảnh nhiễu riêng (mọi hàng khác nhau), BGRA.
    fn noise_page(w: u32, h: u32, seed: u32) -> Image {
        let mut s = seed.wrapping_mul(2_654_435_761) | 1;
        let mut bgra = Vec::with_capacity((w * h * 4) as usize);
        for _ in 0..w * h {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            bgra.extend_from_slice(&[s as u8, (s >> 8) as u8, (s >> 16) as u8, 255]);
        }
        Image {
            width: w,
            height: h,
            bgra,
        }
    }

    /// Hàng [`y`, `y + h`) của `pg`.
    fn rows_of(pg: &Image, y: u32, h: u32) -> Image {
        let s = (pg.width * 4) as usize;
        Image {
            width: pg.width,
            height: h,
            bgra: pg.bgra[y as usize * s..(y + h) as usize * s].to_vec(),
        }
    }

    const PW: u32 = 64;
    const PH: u32 = 60;

    fn page_region() -> Rect {
        r(0, 0, PW as i32, PH as i32)
    }

    /// Lấy kết quả của luồng nền tới khi `done` đúng hoặc hết `ms`.
    fn collect(w: &Worker, ms: u64, mut done: impl FnMut(&[Event]) -> bool) -> Vec<Event> {
        let end = Instant::now() + Duration::from_millis(ms);
        let mut all = Vec::new();
        while Instant::now() < end {
            all.extend(w.take_events());
            if done(&all) {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        all
    }

    fn ticks(ev: &[Event]) -> Vec<Tick> {
        ev.iter()
            .filter_map(|e| match e {
                Event::Tick(t) => Some(*t),
                Event::Finished { .. } => None,
            })
            .collect()
    }

    fn finished(ev: &[Event]) -> Option<(&Image, u32, bool)> {
        ev.iter().find_map(|e| match e {
            Event::Finished {
                image,
                uncertain,
                capture_failed,
                ..
            } => Some((image, *uncertain, *capture_failed)),
            Event::Tick(_) => None,
        })
    }

    /// Cờ "đã chạm giới hạn" của ảnh ghép.
    fn finished_limited(ev: &[Event]) -> Option<bool> {
        ev.iter().find_map(|e| match e {
            Event::Finished { limited, .. } => Some(*limited),
            Event::Tick(_) => None,
        })
    }

    /// Chờ tới khi `alive` chỉ còn một chủ (bao đóng của luồng nền đã drop: luồng đã ra khỏi vòng).
    fn wait_released(alive: &Arc<()>, ms: u64) -> bool {
        let end = Instant::now() + Duration::from_millis(ms);
        while Instant::now() < end {
            if Arc::strong_count(alive) == 1 {
                return true;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        false
    }

    /// Luồng nền tự chụp và ghép, báo từng lượt; Xong giao đúng ảnh ghép của mọi lượt đã báo.
    #[test]
    fn worker_ticks_then_finish_delivers_stitched_image() {
        let pg = noise_page(PW, 400, 1);
        let calls = Arc::new(AtomicUsize::new(0));
        let c = calls.clone();
        let source = move |reg: Rect| {
            assert_eq!(reg, page_region());
            let k = c.fetch_add(1, Ordering::SeqCst) as u32;
            Ok(rows_of(&pg, (k * 10).min(400 - PH), PH))
        };
        let woke = Arc::new(AtomicUsize::new(0));
        let wk = woke.clone();
        let w = Worker::spawn(page_region(), 5, source, move || {
            wk.fetch_add(1, Ordering::SeqCst);
            true
        })
        .unwrap();
        let mut ev = collect(&w, 5000, |e| ticks(e).len() >= 12);
        w.send(Cmd::Finish { grab: true });
        ev.extend(collect(&w, 5000, |e| finished(e).is_some()));
        let ts = ticks(&ev);
        assert!(ts.len() >= 12, "{}", ts.len());
        assert_eq!(ts[0].push, Some(Push::Appended(PH)));
        for t in &ts[1..] {
            assert_eq!(t.push, Some(Push::Appended(10)), "{t:?}");
        }
        // Ảnh ghép là lần báo cuối, là phần đầu trang; không có gì sau nó.
        assert!(matches!(ev.last(), Some(Event::Finished { .. })));
        let (img, uncertain, failed) = finished(&ev).unwrap();
        assert_eq!(finished_limited(&ev), Some(false));
        let h = ts.last().unwrap().height;
        assert_eq!(h, PH + 10 * (ts.len() as u32 - 1));
        assert_eq!(*img, rows_of(&noise_page(PW, 400, 1), 0, h));
        assert_eq!((uncertain, failed), (0, false));
        // Không chụp thêm khi đã có khung; mỗi lượt và ảnh ghép đánh thức một lần.
        assert_eq!(calls.load(Ordering::SeqCst), ts.len());
        assert_eq!(woke.load(Ordering::SeqCst), ts.len() + 1);
    }

    /// Hủy (thả `Worker`) khi luồng nền đang chụp dở: trả ngay, không chờ; luồng nền bỏ khung đó,
    /// không ghép, không giao, không đánh thức, rồi tự dừng.
    #[test]
    fn cancel_while_worker_is_busy_returns_at_once_and_delivers_nothing() {
        let (enter_tx, enter_rx) = mpsc::channel::<()>();
        let (go_tx, go_rx) = mpsc::channel::<()>();
        let alive = Arc::new(());
        let keep = alive.clone();
        let pg = noise_page(PW, PH, 2);
        let source = move |_: Rect| {
            let _keep = &keep;
            let _ = enter_tx.send(());
            let _ = go_rx.recv_timeout(Duration::from_secs(5));
            Ok(pg.clone())
        };
        let woke = Arc::new(AtomicUsize::new(0));
        let wk = woke.clone();
        let w = Worker::spawn(page_region(), 1, source, move || {
            wk.fetch_add(1, Ordering::SeqCst);
            true
        })
        .unwrap();
        enter_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let events = w.events.clone();
        let t0 = Instant::now();
        drop(w);
        assert!(
            t0.elapsed() < Duration::from_millis(50),
            "{:?}",
            t0.elapsed()
        );
        go_tx.send(()).unwrap();
        assert!(wait_released(&alive, 2000), "luồng nền không dừng");
        assert!(events.lock().unwrap().is_empty());
        assert_eq!(woke.load(Ordering::SeqCst), 0);
    }

    /// Xong khi luồng nền đang chụp dở: lượt đó được ghép rồi giao ảnh, không chụp thêm.
    #[test]
    fn finish_while_worker_is_busy_delivers_that_frame() {
        let (enter_tx, enter_rx) = mpsc::channel::<()>();
        let (go_tx, go_rx) = mpsc::channel::<()>();
        let pg = noise_page(PW, PH, 3);
        let expect = pg.clone();
        let calls = Arc::new(AtomicUsize::new(0));
        let c = calls.clone();
        let source = move |_: Rect| {
            c.fetch_add(1, Ordering::SeqCst);
            let _ = enter_tx.send(());
            let _ = go_rx.recv_timeout(Duration::from_secs(5));
            Ok(pg.clone())
        };
        let w = Worker::spawn(page_region(), 1, source, || true).unwrap();
        enter_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        w.send(Cmd::Finish { grab: true });
        go_tx.send(()).unwrap();
        let ev = collect(&w, 3000, |e| finished(e).is_some());
        assert_eq!(ticks(&ev).len(), 1);
        let (img, uncertain, failed) = finished(&ev).unwrap();
        assert_eq!(*img, expect);
        assert_eq!((uncertain, failed), (0, false));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    /// Xong trước lượt chụp đầu: chụp đúng một khung; chụp lỗi thì báo lỗi chụp, ảnh rỗng. Kết thúc
    /// không do Xong (giới hạn, vùng mất) thì không chụp.
    #[test]
    fn finish_without_frames_grabs_once_or_reports_failure() {
        let pg = noise_page(PW, PH, 4);
        let expect = pg.clone();
        let calls = Arc::new(AtomicUsize::new(0));
        let c = calls.clone();
        let ok = move |_: Rect| {
            c.fetch_add(1, Ordering::SeqCst);
            Ok(pg.clone())
        };
        let w = Worker::spawn(page_region(), 60_000, ok, || true).unwrap();
        w.send(Cmd::Finish { grab: true });
        let ev = collect(&w, 3000, |e| finished(e).is_some());
        assert!(ticks(&ev).is_empty());
        let (img, _, failed) = finished(&ev).unwrap();
        assert_eq!(*img, expect);
        assert!(!failed);
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let fail = |_: Rect| Err(());
        let w = Worker::spawn(page_region(), 60_000, fail, || true).unwrap();
        w.send(Cmd::Finish { grab: true });
        let ev = collect(&w, 3000, |e| finished(e).is_some());
        let (img, _, failed) = finished(&ev).unwrap();
        assert_eq!((img.width, img.height), (0, 0));
        assert!(failed);

        let calls = Arc::new(AtomicUsize::new(0));
        let c = calls.clone();
        let counted = move |_: Rect| {
            c.fetch_add(1, Ordering::SeqCst);
            Err(())
        };
        let w = Worker::spawn(page_region(), 60_000, counted, || true).unwrap();
        w.send(Cmd::Finish { grab: false });
        let ev = collect(&w, 3000, |e| finished(e).is_some());
        let (img, _, failed) = finished(&ev).unwrap();
        assert_eq!((img.width, img.height, failed), (0, 0, false));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    /// Cửa sổ chủ không còn (đánh thức thất bại): luồng nền tự dừng sau lượt đó.
    #[test]
    fn worker_stops_when_host_window_is_gone() {
        let alive = Arc::new(());
        let keep = alive.clone();
        let pg = noise_page(PW, PH, 5);
        let calls = Arc::new(AtomicUsize::new(0));
        let c = calls.clone();
        let source = move |_: Rect| {
            let _keep = &keep;
            c.fetch_add(1, Ordering::SeqCst);
            Ok(pg.clone())
        };
        let w = Worker::spawn(page_region(), 1, source, || false).unwrap();
        assert!(wait_released(&alive, 2000), "luồng nền không dừng");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(ticks(&w.take_events()).len(), 1);
    }

    /// Đạt giới hạn: luồng nền không chụp nữa, chờ lệnh; Xong giao ảnh hiện có.
    #[test]
    fn worker_idles_after_limit_until_finish() {
        let tall = MAX_HEIGHT + 1;
        let region = r(0, 0, 1, tall as i32);
        let calls = Arc::new(AtomicUsize::new(0));
        let c = calls.clone();
        let source = move |_: Rect| {
            c.fetch_add(1, Ordering::SeqCst);
            Ok(noise_page(1, tall, 6))
        };
        let w = Worker::spawn(region, 1, source, || true).unwrap();
        let mut ev = collect(&w, 3000, |e| !ticks(e).is_empty());
        std::thread::sleep(Duration::from_millis(60));
        ev.extend(w.take_events());
        assert_eq!(ticks(&ev).len(), 1);
        assert_eq!(ticks(&ev)[0].push, Some(Push::Limit));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        w.send(Cmd::Finish { grab: false });
        let ev = collect(&w, 3000, |e| finished(e).is_some());
        let (img, _, failed) = finished(&ev).unwrap();
        assert_eq!((img.height, failed), (0, false));
        // Ảnh ghép mang cờ giới hạn (lượt `Limit` có thể chưa được luồng giao diện xử lý).
        assert_eq!(finished_limited(&ev), Some(true));
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        // Xong trước lượt đầu, khung chụp lần cuối vượt giới hạn: cũng mang cờ.
        let w = Worker::spawn(
            region,
            60_000,
            move |_: Rect| Ok(noise_page(1, tall, 8)),
            || true,
        )
        .unwrap();
        w.send(Cmd::Finish { grab: true });
        let ev = collect(&w, 3000, |e| finished(e).is_some());
        assert_eq!(finished_limited(&ev), Some(true));
    }

    /// Thứ tự giao khi phiên kết thúc: giới hạn báo cả khi Xong tới trước lượt `Limit` (cờ của
    /// luồng nền); Hủy không giao gì; ảnh rỗng không mở kết quả.
    #[test]
    fn deliveries_report_limit_even_when_done_came_first() {
        let img = Image {
            width: 2,
            height: 1,
            bgra: vec![1; 8],
        };
        let empty = Image {
            width: 0,
            height: 0,
            bgra: Vec::new(),
        };
        let f = |i: &Image, failed, limited| (i.clone(), 0, failed, limited);
        assert_eq!(
            deliveries(Phase::Done, &f(&img, false, true)),
            (true, true, false)
        );
        assert_eq!(
            deliveries(Phase::Done, &f(&img, false, false)),
            (true, false, false)
        );
        assert_eq!(
            deliveries(Phase::Limit, &f(&img, false, false)),
            (true, true, false)
        );
        assert_eq!(
            deliveries(Phase::Lost, &f(&img, false, true)),
            (true, true, false)
        );
        assert_eq!(
            deliveries(Phase::Done, &f(&empty, true, false)),
            (false, false, true)
        );
        assert_eq!(
            deliveries(Phase::Cancelled, &f(&img, true, true)),
            (false, false, false)
        );
    }

    /// Không chụp dồn: máy chậm hơn nhịp thì lượt sau chỉ bắt đầu sau khi lượt trước xong, nghỉ ít
    /// nhất `MIN_REST_MS`; đổi nhịp áp dụng từ lượt sau.
    #[test]
    fn worker_never_overlaps_captures() {
        let pg = noise_page(PW, PH, 7);
        let spans: Arc<Mutex<Vec<(Instant, Instant)>>> = Arc::new(Mutex::new(Vec::new()));
        let sp = spans.clone();
        let source = move |_: Rect| {
            let t0 = Instant::now();
            std::thread::sleep(Duration::from_millis(15));
            sp.lock().unwrap().push((t0, Instant::now()));
            Ok(pg.clone())
        };
        let w = Worker::spawn(page_region(), 1, source, || true).unwrap();
        let _ = collect(&w, 5000, |e| ticks(e).len() >= 6);
        w.send(Cmd::Interval(60));
        let _ = collect(&w, 5000, |e| ticks(e).len() >= 3);
        drop(w);
        let s = spans.lock().unwrap().clone();
        assert!(s.len() >= 9, "{}", s.len());
        for p in s.windows(2) {
            let gap = p[1].0.saturating_duration_since(p[0].1);
            assert!(
                gap >= Duration::from_millis(u64::from(MIN_REST_MS) - 1),
                "{gap:?}"
            );
        }
        // Sau khi đổi nhịp 60 ms: hai lượt cuối cách nhau ít nhất ~60 ms (tính từ đầu lượt).
        let n = s.len();
        let start_gap = s[n - 1].0.saturating_duration_since(s[n - 2].0);
        assert!(start_gap >= Duration::from_millis(55), "{start_gap:?}");
    }

    /// Máy trạng thái nhận kết quả từ thông điệp của luồng nền: chiều cao, lời nhắc, giãn nhịp,
    /// giới hạn kết thúc phiên đúng một lần; sau khi kết thúc chỉ còn chờ ảnh ghép.
    #[test]
    fn machine_takes_tick_results_from_messages() {
        let tick = |push, height, took_ms| {
            Event::Tick(Tick {
                push,
                height,
                took_ms,
            })
        };
        let mut m = Machine::new();
        let out = apply_events(
            &mut m,
            vec![
                tick(Some(Push::Appended(60)), 60, 20),
                tick(Some(Push::Uncertain(Reason::Changed)), 60, 90),
                tick(None, 60, 95),
            ],
            100,
        );
        assert!(!out.ended && out.appended && out.finished.is_none());
        assert_eq!(out.height, Some(60));
        assert_eq!(out.interval, None);
        assert_eq!(m.warning_msg(100), Some(Msg::ScrollContentChanging));
        // Lượt chậm thứ ba liên tiếp: giãn nhịp (theo thời gian luồng nền báo).
        let out = apply_events(&mut m, vec![tick(Some(Push::NoChange), 60, 120)], 200);
        assert_eq!(out.interval, Some(SLOW_INTERVAL_MS));
        assert!(!out.appended);
        // Giới hạn: kết thúc đúng một lần; lượt sau bị bỏ; ảnh ghép về sau đó.
        let out = apply_events(
            &mut m,
            vec![
                tick(Some(Push::Appended(40)), 100, 30),
                tick(Some(Push::Limit), 100, 30),
                tick(Some(Push::Uncertain(Reason::Backward)), 100, 30),
            ],
            300,
        );
        assert!(out.ended && out.appended);
        assert_eq!(m.phase(), Phase::Limit);
        assert_eq!(m.warning_msg(300), Some(Msg::ScrollContentChanging));
        let img = Image {
            width: 2,
            height: 1,
            bgra: vec![1; 8],
        };
        let out = apply_events(
            &mut m,
            vec![
                tick(Some(Push::Limit), 100, 30),
                Event::Finished {
                    image: img.clone(),
                    uncertain: 3,
                    capture_failed: false,
                    limited: true,
                },
            ],
            400,
        );
        assert!(!out.ended);
        assert_eq!(out.finished, Some((img, 3, false, true)));
        assert_eq!(m.phase(), Phase::Limit);
    }

    /// Chữ trạng thái vàng theo lý do mới nhất trong `WARN_MS`.
    #[test]
    fn machine_warning_follows_latest_reason() {
        let mut m = Machine::new();
        assert_eq!(m.warning_msg(0), None);
        m.input(Input::Push(Push::Uncertain(Reason::NoOverlap)), 100);
        assert_eq!(m.warning_msg(100), Some(Msg::ScrollBackUp));
        m.input(Input::Push(Push::Uncertain(Reason::Backward)), 200);
        assert_eq!(m.warning_msg(200), Some(Msg::ScrollDownNotUp));
        m.input(Input::Push(Push::Uncertain(Reason::Changed)), 250);
        assert_eq!(m.warning_msg(250), Some(Msg::ScrollContentChanging));
        for r in [Reason::Ambiguous, Reason::SizeMismatch] {
            m.input(Input::Push(Push::Uncertain(r)), 300);
            assert_eq!(m.warning_msg(300), Some(Msg::ScrollSlowDown), "{r:?}");
        }
        m.input(Input::Push(Push::Uncertain(Reason::NoOverlap)), 300);
        assert_eq!(m.warning_msg(300), Some(Msg::ScrollBackUp));
        m.input(Input::Push(Push::Uncertain(Reason::Changed)), 300);
        // Nối được hay không đổi: chữ giữ đến hết hạn, rồi tắt.
        m.input(Input::Push(Push::Appended(10)), 400);
        m.input(Input::Push(Push::NoChange), 500);
        assert_eq!(
            m.warning_msg(300 + WARN_MS - 1),
            Some(Msg::ScrollContentChanging)
        );
        assert_eq!(m.warning_msg(300 + WARN_MS), None);
        m.input(Input::Push(Push::Uncertain(Reason::Backward)), 5000);
        assert_eq!(m.warning_msg(5000), Some(Msg::ScrollDownNotUp));
    }

    /// Thanh dành đủ chỗ cho chữ trạng thái vàng dài nhất (đo bằng phông thật) ở DPI 100–200%,
    /// cả hai ngôn ngữ, và vẫn hẹp hơn màn hình 1280 DIP.
    #[test]
    fn bar_reserves_room_for_every_warning() {
        use darkshot_core::i18n::{t, Lang};
        for dpi in [96u32, 120, 144, 192] {
            let ui = build_ui(dpi);
            let status_w = ui.layout.status.w;
            let other = ui.layout.size.0 - status_w;
            for l in Lang::ALL {
                let warns: Vec<Vec<u16>> = WARN_MSGS.iter().map(|&m| wide(t(l, m))).collect();
                let parts: Vec<&[u16]> = warns.iter().map(|w| w.as_slice()).collect();
                let widest = measure(ui.font, &parts).iter().map(|x| x.0).max().unwrap();
                assert!(widest > 0);
                if l == lang::current() {
                    assert!(widest <= status_w, "dpi {dpi} {l:?}: {widest} > {status_w}");
                }
                let bar = other + widest;
                assert!(bar <= scale(1280, dpi), "dpi {dpi} {l:?}: bar {bar} px");
            }
        }
        // Mọi lý do đều có chữ nằm trong `WARN_MSGS`.
        for r in [
            Reason::Ambiguous,
            Reason::NoOverlap,
            Reason::Changed,
            Reason::Backward,
            Reason::SizeMismatch,
        ] {
            assert!(WARN_MSGS.contains(&warn_msg(r)), "{r:?}");
        }
    }

    #[test]
    fn message_id_follows_used_range() {
        assert_eq!(WM_SCROLL_WORK, WM_APP + 25);
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

    /// Bơm thông điệp tới khi phiên đã dọn hoặc hết `ms`.
    fn pump_until_idle(ms: u64) {
        let end = std::time::Instant::now() + std::time::Duration::from_millis(ms);
        while active() && std::time::Instant::now() < end {
            pump_for(5);
        }
    }

    /// Chờ mọi luồng nền dừng hẳn (trước khi đếm tài nguyên).
    fn wait_workers(ms: u64) {
        let end = std::time::Instant::now() + std::time::Duration::from_millis(ms);
        while WORKERS_LIVE.load(Ordering::Acquire) > 0 && std::time::Instant::now() < end {
            pump_for(5);
        }
        assert_eq!(
            WORKERS_LIVE.load(Ordering::Acquire),
            0,
            "luồng nền còn chạy"
        );
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
        // Luồng nền chụp lỗi; Xong trước lượt chụp đầu: chưa có khung.
        assert_eq!(start_with(region, |_| Err(())), Ok(()));
        done();
        // Cửa sổ ẩn ngay, phiên chờ ảnh ghép từ luồng nền.
        assert!(active());
        pump_until_idle(2000);
        assert!(!active());
        assert_eq!(CAPTURE_FAILS.with(|c| c.get()), 1);
        assert!(RESULT.with(|r| r.borrow_mut().take()).is_none());
        // Chụp được: không báo lỗi.
        assert_eq!(start(region), Ok(()));
        done();
        pump_until_idle(2000);
        assert!(!active());
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
        pump_until_idle(2000);
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
                pump_until_idle(2000);
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
        wait_workers(3000);
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

    /// Hủy (như Esc, thoát ứng dụng, đăng xuất) khi luồng nền đang chụp dở một lượt rất chậm: trả
    /// ngay (không chờ luồng nền, trong hạn chót thoát 4 s), không giao ảnh, ON_END đúng một lần;
    /// luồng nền tự dừng sau lượt đó.
    #[test]
    #[ignore]
    fn cancel_while_capture_busy_returns_at_once() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        dpi_aware();
        PAGE.with(|p| *p.borrow_mut() = make_page());
        let (tw, region) = test_window();
        pump_for(100);
        set_on_result(keep_result);
        set_on_end(count_end);
        ENDS.with(|c| c.set(0));
        let entered = Arc::new(AtomicUsize::new(0));
        let e = entered.clone();
        let slow = move |r: Rect| {
            e.fetch_add(1, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(1500));
            capture_source(r)
        };
        for finish_first in [false, true] {
            entered.store(0, Ordering::SeqCst);
            assert_eq!(start_with(region, slow.clone()), Ok(()));
            let end = Instant::now() + Duration::from_secs(2);
            while entered.load(Ordering::SeqCst) == 0 && Instant::now() < end {
                pump_for(5);
            }
            assert_eq!(entered.load(Ordering::SeqCst), 1, "luồng nền chưa chụp");
            if finish_first {
                // Đã Xong (đang chờ ảnh ghép) rồi mới thoát: vẫn bỏ ngay.
                done();
                assert!(active());
            }
            let t0 = Instant::now();
            cancel();
            let took = t0.elapsed();
            eprintln!("cancel while busy (after Done: {finish_first}): {took:?}");
            assert!(took < Duration::from_millis(100), "{took:?}");
            assert!(!active());
            pump_for(50);
            assert!(RESULT.with(|r| r.borrow_mut().take()).is_none());
        }
        assert_eq!(ENDS.with(|c| c.get()), 2);
        wait_workers(4000);
        // Lượt chụp dở xong sau khi đã hủy: không giao gì.
        pump_for(50);
        assert!(RESULT.with(|r| r.borrow_mut().take()).is_none());
        // SAFETY: hủy cửa sổ thử của chính kiểm thử.
        unsafe {
            let _ = DestroyWindow(tw);
        }
        pump_for(50);
    }

    /// Đánh thức thất bại mà cửa sổ chủ vẫn còn (hàng đợi đầy): luồng nền không dừng, hẹn giờ giao
    /// diện vẫn lấy kết quả; Xong kết thúc phiên và giao ảnh. Luồng nền chết (cửa sổ chủ báo không
    /// còn) khi phiên chưa xong: phiên tự kết thúc, báo không chụp được, không kẹt.
    #[test]
    #[ignore]
    fn lost_wake_messages_never_leave_the_session_stuck() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        dpi_aware();
        PAGE.with(|p| *p.borrow_mut() = make_page());
        OFFSET.with(|o| o.set(0));
        let (tw, region) = test_window();
        pump_for(200);
        set_on_result(keep_result);
        set_on_capture_failed(count_capture_failed);
        CAPTURE_FAILS.with(|c| c.set(0));

        // Mọi lần đăng đều "lỗi" (không tới), cửa sổ chủ còn.
        TEST_WAKE.store(WAKE_DROP_POSTS, Ordering::Release);
        assert_eq!(start(region), Ok(()));
        pump_for(400);
        assert_eq!(
            WORKERS_LIVE.load(Ordering::Acquire),
            1,
            "luồng nền phải còn chạy"
        );
        done();
        pump_until_idle(3000);
        assert!(!active(), "phiên kẹt sau Xong");
        assert!(RESULT.with(|r| r.borrow_mut().take()).is_some());
        assert_eq!(CAPTURE_FAILS.with(|c| c.get()), 0);
        wait_workers(3000);

        // Luồng nền dừng sau lượt đầu (báo cửa sổ chủ không còn) rồi mới bấm Xong.
        TEST_WAKE.store(WAKE_HOST_GONE, Ordering::Release);
        assert_eq!(start(region), Ok(()));
        let end = Instant::now() + Duration::from_secs(2);
        while WORKERS_LIVE.load(Ordering::Acquire) > 0 && Instant::now() < end {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(WORKERS_LIVE.load(Ordering::Acquire), 0);
        done();
        pump_until_idle(1000);
        assert!(!active(), "phiên kẹt khi luồng nền đã chết");
        assert_eq!(CAPTURE_FAILS.with(|c| c.get()), 1);
        assert!(RESULT.with(|r| r.borrow_mut().take()).is_none());

        // Luồng nền chết khi phiên còn chạy (không bấm gì): hẹn giờ giao diện kết thúc phiên.
        assert_eq!(start(region), Ok(()));
        pump_until_idle(2000);
        assert!(!active(), "phiên kẹt khi luồng nền chết giữa chừng");
        assert_eq!(CAPTURE_FAILS.with(|c| c.get()), 2);
        TEST_WAKE.store(0, Ordering::Release);
        wait_workers(3000);
        // SAFETY: hủy cửa sổ thử của chính kiểm thử.
        unsafe {
            let _ = DestroyWindow(tw);
        }
        pump_for(50);
    }

    /// Đo: vùng lớn (2874×1634, kẹp vào màn hình chính) chụp thật trên luồng nền; luồng giao diện
    /// chỉ xử lý thông điệp. In thời gian chụp của luồng nền, thời gian lượt (chụp + ghép) và thời
    /// gian xử lý thông điệp lâu nhất trên luồng giao diện. Ảnh chỉ nằm trong bộ nhớ, không lưu.
    /// `cargo test -p darkshot-windows --release scroll_session::live::measure -- --ignored --nocapture --test-threads=1`
    #[test]
    #[ignore]
    fn measure_ui_thread_on_large_region() {
        use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        dpi_aware();
        // SAFETY: chỉ đọc kích thước màn hình chính.
        let (sw, sh) = unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };
        let region = r_of(0, 0, sw.min(2874), (sh - 120).clamp(200, 1634));
        set_on_result(keep_result);
        let grabs: Arc<Mutex<Vec<u32>>> = Arc::new(Mutex::new(Vec::new()));
        let g = grabs.clone();
        let source = move |r: Rect| {
            let t0 = Instant::now();
            let out = capture_source(r);
            g.lock()
                .unwrap()
                .push(t0.elapsed().as_millis().min(u128::from(u32::MAX)) as u32);
            out
        };
        TOOK_MS.lock().unwrap().clear();
        assert_eq!(start_with(region, source), Ok(()));
        let (mut worst, mut total, mut n) = (Duration::ZERO, Duration::ZERO, 0u32);
        let end = Instant::now() + Duration::from_secs(4);
        while Instant::now() < end {
            let mut msg = MSG::default();
            // SAFETY: vòng bơm thông điệp chuẩn của luồng kiểm thử.
            unsafe {
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    let t0 = Instant::now();
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                    let d = t0.elapsed();
                    worst = worst.max(d);
                    total += d;
                    n += 1;
                }
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        let t0 = Instant::now();
        done();
        let done_took = t0.elapsed();
        pump_until_idle(3000);
        let res = RESULT.with(|r| r.borrow_mut().take());
        let grabs = grabs.lock().unwrap().clone();
        let took = TOOK_MS.lock().unwrap().clone();
        let stat = |v: &[u32]| {
            let mut v = v.to_vec();
            v.sort_unstable();
            let med = v.get(v.len() / 2).copied().unwrap_or(0);
            (
                v.first().copied().unwrap_or(0),
                med,
                v.last().copied().unwrap_or(0),
            )
        };
        eprintln!(
            "region {}x{}: {} ticks; capture_rect ms (min, median, max) {:?}; tick (capture + push) ms {:?}",
            region.w,
            region.h,
            took.len(),
            stat(&grabs),
            stat(&took)
        );
        eprintln!(
            "UI thread: {n} messages, longest {worst:?}, mean {:?}; done() returned in {done_took:?}",
            total / n.max(1)
        );
        assert!(res.is_some());
        assert!(!took.is_empty());
        assert!(worst < Duration::from_millis(50), "{worst:?}");
        wait_workers(3000);
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
