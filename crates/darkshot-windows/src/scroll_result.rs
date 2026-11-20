//! Cửa sổ "Ảnh chụp cuộn": kết quả của một phiên chụp cuộn. Tối, không chặn, đổi cỡ được; mỗi kết
//! quả một cửa sổ, tối đa `MAX_WINDOWS` (không bao giờ đóng một cửa sổ có ảnh chưa dùng).
//!
//! Xem trước: ảnh thu theo bề rộng khung xem (HALFTONE), cuộn dọc bằng con lăn, thanh cuộn,
//! PgUp/PgDn/Home/End. Ảnh cao hơn `PREBUILD_MIN_H` được thu nhỏ một lần trên luồng nền (trong lúc
//! đó hiện "Đang chuẩn bị…"); luồng nền chỉ báo xong bằng `WM_SCROLL_PREPARED` (không mang con trỏ),
//! kết quả nằm trong `Job` dùng chung nên cửa sổ đóng giữa chừng không rò gì.
//!
//! Lưu/Chép/Tải lên/Ghim giao một bản sao ảnh cho hàm hành động do `main` đăng ký (đúng đường
//! `run_on_image`, `pin::open`). Thông điệp do vòng lặp của ứng dụng phân phối; `pre_translate` lo
//! bàn phím.

use crate::history_paint::{bmp_from_pixels, measure, Bmp};
use crate::lang;
use crate::theme;
use crate::winutil::{self, scale};
use darkshot_core::actions::{Action, NoticeKind};
use darkshot_core::geometry::Rect;
use darkshot_core::i18n::{self, t, Lang, Msg};
use darkshot_core::image::Image;
use darkshot_core::theme::{BG, INPUT, SURFACE, TEXT_MUTED, WARNING};
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleDC, DeleteDC, DeleteObject, EndPaint, GetDC,
    GetMonitorInfoW, InvalidateRect, MonitorFromPoint, ReleaseDC, SelectObject, SetBrushOrgEx,
    SetStretchBltMode, StretchBlt, DT_CENTER, DT_END_ELLIPSIS, DT_LEFT, DT_NOPREFIX, DT_SINGLELINE,
    DT_VCENTER, DT_WORDBREAK, HALFTONE, HDC, HFONT, HGDIOBJ, HMONITOR, MONITORINFO,
    MONITOR_DEFAULTTONEAREST, PAINTSTRUCT, SRCCOPY, STRETCH_BLT_MODE,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{SetScrollInfo, NMHDR, NM_CUSTOMDRAW};
use windows::Win32::UI::HiDpi::{
    AdjustWindowRectExForDpi, GetDpiForMonitor, GetDpiForWindow, GetSystemMetricsForDpi,
    MDT_EFFECTIVE_DPI,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SetFocus, VK_END, VK_ESCAPE, VK_HOME, VK_NEXT, VK_PRIOR,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetCursorPos, GetScrollInfo,
    IsDialogMessageW, IsWindow, LoadCursorW, PostMessageW, RegisterClassW, SendMessageW,
    SetForegroundWindow, SetWindowPos, SetWindowTextW, ShowWindow, BS_PUSHBUTTON, HMENU, IDC_ARROW,
    MINMAXINFO, MSG, SB_BOTTOM, SB_CTL, SB_LINEDOWN, SB_LINEUP, SB_PAGEDOWN, SB_PAGEUP,
    SB_THUMBPOSITION, SB_THUMBTRACK, SB_TOP, SCROLLINFO, SIF_DISABLENOSCROLL, SIF_PAGE, SIF_POS,
    SIF_RANGE, SIF_TRACKPOS, SM_CXVSCROLL, SWP_NOACTIVATE, SWP_NOZORDER, SW_SHOW, WINDOW_EX_STYLE,
    WINDOW_STYLE, WM_APP, WM_CLOSE, WM_COMMAND, WM_DPICHANGED, WM_ERASEBKGND, WM_GETMINMAXINFO,
    WM_KEYDOWN, WM_MOUSEWHEEL, WM_NCDESTROY, WM_NOTIFY, WM_PAINT, WM_SETFOCUS, WM_SETFONT, WM_SIZE,
    WM_VSCROLL, WNDCLASSW, WS_CAPTION, WS_CHILD, WS_CLIPCHILDREN, WS_EX_APPWINDOW,
    WS_EX_CONTROLPARENT, WS_GROUP, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_SYSMENU, WS_TABSTOP,
    WS_THICKFRAME, WS_VISIBLE,
};

/// Ảnh thu nhỏ dựng xong trên luồng nền: WPARAM = mã lượt dựng; không mang dữ liệu (kết quả nằm
/// trong `Job` của cửa sổ). WM_APP + 1..+16 đã dùng ở nơi khác.
pub const WM_SCROLL_PREPARED: u32 = WM_APP + 17;
/// Mở thêm khi đã có 4 cửa sổ: đóng cái cũ nhất đã dùng; không có thì vẫn mở và báo.
const MAX_WINDOWS: usize = 4;
/// Vùng client tối thiểu (DIP).
const MIN_W: i32 = 480;
const MIN_H: i32 = 400;
/// Chiều cao mặc định: phần trăm chiều cao vùng làm việc; bề rộng tối đa: phần trăm bề rộng.
const DEFAULT_H_PCT: i32 = 70;
const MAX_W_PCT: i32 = 90;
/// Ảnh cao hơn mức này được thu nhỏ trước trên luồng nền.
const PREBUILD_MIN_H: u32 = 4096;

const CLASS: PCWSTR = w!("darkshot-scroll-result");
/// Lưu mang ID của IDOK (Enter), Đóng mang ID của IDCANCEL.
const ID_SAVE: i32 = 1;
const ID_CLOSE: i32 = 2;
const ID_COPY: i32 = 3;
const ID_UPLOAD: i32 = 4;
const ID_PIN: i32 = 5;
const ID_SCROLLBAR: i32 = 100;
/// Thứ tự nút từ trái sang phải.
const BUTTON_IDS: [i32; 5] = [ID_SAVE, ID_COPY, ID_UPLOAD, ID_PIN, ID_CLOSE];
const BUTTON_LABELS: [Msg; 5] = [
    Msg::ScrollSave,
    Msg::ScrollCopy,
    Msg::ScrollUpload,
    Msg::ScrollPin,
    Msg::ScrollClose,
];
/// Kiểu SCROLLBAR dọc (`SBS_VERT`).
const SBS_VERT: u32 = 0x0001;

const STYLE: WINDOW_STYLE = WINDOW_STYLE(
    WS_CAPTION.0
        | WS_SYSMENU.0
        | WS_THICKFRAME.0
        | WS_MINIMIZEBOX.0
        | WS_MAXIMIZEBOX.0
        | WS_CLIPCHILDREN.0,
);
const EX_STYLE: WINDOW_EX_STYLE = WINDOW_EX_STYLE(WS_EX_APPWINDOW.0 | WS_EX_CONTROLPARENT.0);

// Bố cục (DIP).
const MARGIN: i32 = 12;
const GAP: i32 = 8;
const ROW_GAP: i32 = 6;
const BTN_H: i32 = 28;
const BTN_MIN_W: i32 = 72;
const BTN_PAD: i32 = 12;
const INFO_H: i32 = 20;
const STRIP_PAD: i32 = 8;
const STRIP_BAR: i32 = 3;
const STRIP_MAX_LINES: i32 = 3;
/// Một dòng cuộn (DIP); một nấc con lăn = `WHEEL_LINES` dòng.
const LINE_DIP: i32 = 40;
const WHEEL_LINES: i64 = 3;

/// Việc khi mở thêm một cửa sổ, theo cờ "đã dùng" của các cửa sổ đang mở (cũ nhất trước).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OpenPolicy {
    Open,
    /// Đóng cửa sổ ở chỉ số này (cũ nhất trong các cửa sổ đã Lưu/Chép/Tải lên/Ghim) rồi mở.
    CloseOldestUsed(usize),
    /// Mọi cửa sổ đều có ảnh chưa dùng: mở thêm, không đóng gì, báo người dùng.
    OpenAndWarn,
}

fn open_policy(used: &[bool]) -> OpenPolicy {
    if used.len() < MAX_WINDOWS {
        return OpenPolicy::Open;
    }
    match used.iter().position(|u| *u) {
        Some(i) => OpenPolicy::CloseOldestUsed(i),
        None => OpenPolicy::OpenAndWarn,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Layout {
    /// Dải cảnh báo (cao 0 khi không có).
    strip: Rect,
    /// Khung xem ảnh (không gồm thanh cuộn).
    view: Rect,
    scrollbar: Rect,
    /// Dòng "W × H px".
    info: Rect,
    /// Lưu, Chép, Tải lên, Ghim, Đóng.
    buttons: [Rect; 5],
}

const ZERO: Rect = Rect {
    x: 0,
    y: 0,
    w: 0,
    h: 0,
};

/// Bề rộng nút cho chữ rộng `text_px` pixel ở `dpi`.
fn button_width(text_px: i32, dpi: u32) -> i32 {
    scale(BTN_MIN_W, dpi).max(text_px.max(0) + 2 * scale(BTN_PAD, dpi))
}

/// Bố cục trong vùng client `cw`×`ch` pixel: dải cảnh báo (nếu `warn_text` = bề rộng câu cảnh báo),
/// khung xem + thanh cuộn rộng `sb_w`, dòng thông tin, hàng nút sát góc dưới phải. `btn_text` là bề
/// rộng chữ của từng nút, `line_h` chiều cao một dòng chữ. Cỡ quá nhỏ thì các phần co về 0.
fn layout(
    cw: i32,
    ch: i32,
    dpi: u32,
    btn_text: [i32; 5],
    warn_text: Option<i32>,
    line_h: i32,
    sb_w: i32,
) -> Layout {
    let s = |v| scale(v, dpi);
    let m = s(MARGIN);
    let bh = s(BTN_H);
    let btn_y = ch - m - bh;
    let mut buttons = [ZERO; 5];
    let mut x = cw - m;
    for i in (0..buttons.len()).rev() {
        let w = button_width(btn_text[i], dpi);
        x -= w;
        buttons[i] = Rect {
            x,
            y: btn_y,
            w,
            h: bh,
        };
        x -= s(GAP);
    }
    let inner_w = (cw - 2 * m).max(0);
    let info = Rect {
        x: m,
        y: btn_y - s(ROW_GAP) - s(INFO_H),
        w: inner_w,
        h: s(INFO_H),
    };
    let strip_h = match warn_text {
        Some(tw) => {
            let pad = s(STRIP_PAD);
            let text_w = (inner_w - 3 * pad - s(STRIP_BAR)).max(1);
            let lines = ((tw.max(1) + text_w - 1) / text_w).clamp(1, STRIP_MAX_LINES);
            2 * pad + lines * line_h.max(1)
        }
        None => 0,
    };
    let strip = Rect {
        x: m,
        y: m,
        w: inner_w,
        h: strip_h,
    };
    let top = if strip_h > 0 {
        strip.bottom() + s(GAP)
    } else {
        m
    };
    let view_h = (info.y - s(ROW_GAP) - top).max(0);
    let sb_w = sb_w.clamp(0, inner_w);
    let view = Rect {
        x: m,
        y: top,
        w: inner_w - sb_w,
        h: view_h,
    };
    let scrollbar = Rect {
        x: view.right(),
        y: top,
        w: sb_w,
        h: view_h,
    };
    Layout {
        strip,
        view,
        scrollbar,
        info,
        buttons,
    }
}

/// Cỡ hiển thị của ảnh `img_w`×`img_h` trong khung xem rộng `view_w`: thu theo bề rộng, không phóng
/// to; chiều cao làm tròn lên (ít nhất 1 hàng).
fn fit_width(img_w: u32, img_h: u32, view_w: i32) -> (i32, i32) {
    if img_w == 0 || img_h == 0 || view_w <= 0 {
        return (0, 0);
    }
    let (iw, ih) = (i64::from(img_w), i64::from(img_h));
    let dw = iw.min(i64::from(view_w));
    let dh = ((ih * dw + iw - 1) / iw).max(1);
    (dw as i32, dh.min(i64::from(i32::MAX)) as i32)
}

fn max_scroll(disp_h: i32, view_h: i32) -> i32 {
    (disp_h - view_h).max(0)
}

/// Lệnh cuộn: thanh cuộn, phím, con lăn (`Wheel` mang delta của WM_MOUSEWHEEL, dương = lên).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScrollCmd {
    LineUp,
    LineDown,
    PageUp,
    PageDown,
    Top,
    Bottom,
    To(i32),
    Wheel(i32),
}

/// Vị trí cuộn mới (pixel hiển thị) sau lệnh `cmd`, kẹp trong [0, max].
fn scrolled(pos: i32, cmd: ScrollCmd, disp_h: i32, view_h: i32, line: i32) -> i32 {
    let max = i64::from(max_scroll(disp_h, view_h));
    let line = i64::from(line.max(1));
    let page = (i64::from(view_h) - line).max(line);
    let pos = i64::from(pos);
    let target = match cmd {
        ScrollCmd::LineUp => pos - line,
        ScrollCmd::LineDown => pos + line,
        ScrollCmd::PageUp => pos - page,
        ScrollCmd::PageDown => pos + page,
        ScrollCmd::Top => 0,
        ScrollCmd::Bottom => max,
        ScrollCmd::To(p) => i64::from(p),
        ScrollCmd::Wheel(d) => pos - i64::from(d) * WHEEL_LINES * line / 120,
    };
    target.clamp(0, max) as i32
}

/// Giữ vị trí tương đối khi chiều cao hiển thị đổi từ `old_h` sang `new_h`.
fn rescaled_scroll(pos: i32, old_h: i32, new_h: i32) -> i32 {
    if old_h <= 0 {
        return 0;
    }
    (i64::from(pos) * i64::from(new_h) / i64::from(old_h)) as i32
}

/// Dải hàng của bitmap nguồn cao `src_h` ứng với các hàng hiển thị [scroll, scroll + vis) của ảnh
/// hiển thị cao `disp_h`: (hàng đầu, số hàng), đầu làm tròn xuống, cuối làm tròn lên.
fn src_band(scroll: i32, vis: i32, disp_h: i32, src_h: i32) -> (i32, i32) {
    if disp_h <= 0 || vis <= 0 || src_h <= 0 {
        return (0, 0);
    }
    let (d, s) = (i64::from(disp_h), i64::from(src_h));
    let y0 = (i64::from(scroll.max(0)) * s / d).min(s - 1);
    let y1 = (((i64::from(scroll.max(0)) + i64::from(vis)) * s + d - 1) / d).min(s);
    (y0 as i32, (y1 - y0).max(1) as i32)
}

/// Cỡ vùng client mặc định (pixel) trong vùng làm việc `work`: cao 70% vùng làm việc, rộng theo tỉ lệ
/// ảnh (không quá bề rộng ảnh, không quá 90% vùng làm việc), tối thiểu `MIN_W`×`MIN_H` DIP, không
/// vượt vùng làm việc. `chrome_w`/`chrome_h`: phần client ngoài khung xem.
fn default_client(
    work: Rect,
    img_w: u32,
    img_h: u32,
    dpi: u32,
    chrome_w: i32,
    chrome_h: i32,
) -> (i32, i32) {
    let h = (work.h * DEFAULT_H_PCT / 100)
        .max(scale(MIN_H, dpi))
        .min(work.h.max(0));
    let ph = i64::from((h - chrome_h).max(1));
    let max_pw = i64::from((work.w * MAX_W_PCT / 100 - chrome_w).max(0));
    let pw = if img_h == 0 {
        0
    } else {
        (i64::from(img_w) * ph / i64::from(img_h))
            .min(i64::from(img_w))
            .min(max_pw)
    };
    let w = (pw as i32 + chrome_w)
        .max(scale(MIN_W, dpi))
        .min(work.w.max(0));
    (w, h)
}

fn warns_on_open(_open_now: usize) -> bool {
    false
}

fn rebuild_target(
    _built: Option<u32>,
    _img_w: u32,
    _disp_w: i32,
    _failed: Option<u32>,
) -> Option<u32> {
    None
}

fn needs_prebuild(img_h: u32) -> bool {
    img_h > PREBUILD_MIN_H
}

/// Bề rộng của ảnh thu nhỏ dựng trước: đúng bề rộng hiển thị, không quá ảnh gốc, ít nhất 1.
fn prebuild_width(img_w: u32, disp_w: i32) -> u32 {
    (disp_w.max(1) as u32).min(img_w)
}

/// Khung xem đã rộng hơn ảnh thu nhỏ đang có (và ảnh gốc còn rộng hơn): dựng lại cho nét.
fn needs_rebuild(built_w: u32, img_w: u32, disp_w: i32) -> bool {
    built_w < img_w && i64::from(disp_w) > i64::from(built_w)
}

/// Điểm ảnh `0x00RRGGBB` (hàng trên trước) đã thu nhỏ.
struct Prepared {
    w: u32,
    h: u32,
    px: Vec<u32>,
}

/// Thu `img` về bề rộng `tw` (không phóng to) bằng trung bình hộp; chiều cao theo tỉ lệ, làm tròn.
/// `cancel` bật thì dừng và trả `None`; ảnh hỏng hay thiếu bộ nhớ cũng `None`.
fn downscale(img: &Image, tw: u32, cancel: &AtomicBool) -> Option<Prepared> {
    if !img.is_valid() {
        return None;
    }
    let (w, h) = (img.width as usize, img.height as usize);
    let tw = (tw.max(1) as usize).min(w);
    let th = ((h as u64 * tw as u64 + w as u64 / 2) / w as u64).max(1) as usize;
    let mut px: Vec<u32> = Vec::new();
    px.try_reserve_exact(tw.checked_mul(th)?).ok()?;
    let stride = w * 4;
    let xs: Vec<(usize, usize)> = (0..tw)
        .map(|ox| {
            let x0 = ox * w / tw;
            (x0, ((ox + 1) * w / tw).max(x0 + 1).min(w))
        })
        .collect();
    let mut sums = vec![[0u64; 3]; tw];
    for oy in 0..th {
        if oy % 64 == 0 && cancel.load(Ordering::Relaxed) {
            return None;
        }
        let y0 = oy * h / th;
        let y1 = ((oy + 1) * h / th).max(y0 + 1).min(h);
        for s in sums.iter_mut() {
            *s = [0; 3];
        }
        for y in y0..y1 {
            let row = img.bgra.get(y * stride..(y + 1) * stride)?;
            for (s, &(x0, x1)) in sums.iter_mut().zip(&xs) {
                for p in row.get(x0 * 4..x1 * 4)?.as_chunks::<4>().0 {
                    s[0] += u64::from(p[0]);
                    s[1] += u64::from(p[1]);
                    s[2] += u64::from(p[2]);
                }
            }
        }
        let rows = (y1 - y0) as u64;
        for (s, &(x0, x1)) in sums.iter().zip(&xs) {
            let n = rows * (x1 - x0) as u64;
            let avg = |v: u64| ((v + n / 2) / n) as u32;
            px.push((avg(s[2]) << 16) | (avg(s[1]) << 8) | avg(s[0]));
        }
    }
    if cancel.load(Ordering::Relaxed) {
        return None;
    }
    Some(Prepared {
        w: tw as u32,
        h: th as u32,
        px,
    })
}

/// Bản sao ảnh; thiếu bộ nhớ thì `None` (không bỏ dở tiến trình).
fn try_clone(img: &Image) -> Option<Image> {
    let mut bgra = Vec::new();
    bgra.try_reserve_exact(img.bgra.len()).ok()?;
    bgra.extend_from_slice(&img.bgra);
    Some(Image {
        width: img.width,
        height: img.height,
        bgra,
    })
}

/// Một lượt dựng ảnh thu nhỏ trên luồng nền; cửa sổ và luồng cùng giữ.
struct Job {
    id: usize,
    cancel: AtomicBool,
    out: Mutex<Option<Prepared>>,
}

/// Một cửa sổ đang mở. Không bao giờ giữ mượn sổ `WINS` qua lời gọi có thể gửi thông điệp.
struct Win {
    hwnd: HWND,
    buttons: [HWND; 5],
    scrollbar: HWND,
    image: Arc<Image>,
    uncertain: u32,
    /// Đã Lưu/Chép/Tải lên/Ghim ít nhất một lần.
    used: bool,
    /// Ảnh để vẽ (gốc, hoặc thu nhỏ khi ảnh cao).
    bmp: Option<Bmp>,
    /// Lượt dựng đang chạy.
    job: Option<Arc<Job>>,
    /// Vị trí cuộn và cỡ hiển thị (pixel).
    scroll: i32,
    disp: (i32, i32),
    layout: Layout,
    dpi: u32,
    font: Option<HFONT>,
    bold: Option<HFONT>,
}

impl Drop for Win {
    fn drop(&mut self) {
        if let Some(j) = &self.job {
            j.cancel.store(true, Ordering::Relaxed);
        }
        // Gỡ khỏi sổ ở WM_NCDESTROY: mọi điều khiển đã hủy, phông không còn ai dùng.
        delete_font(self.font.take());
        delete_font(self.bold.take());
    }
}

/// Hàm `main` đăng ký: thực hiện Lưu/Chép/Tải lên/Ghim trên bản sao ảnh; `true` khi đã giao việc.
type Sink = fn(Action, Image);
type Notify = fn(NoticeKind, &str, &str);

thread_local! {
    /// Cửa sổ đang mở, cũ nhất trước.
    static WINS: RefCell<Vec<Win>> = const { RefCell::new(Vec::new()) };
    static NEXT_JOB: Cell<usize> = const { Cell::new(1) };
    static SINK: Cell<Option<Sink>> = const { Cell::new(None) };
    static NOTIFY: Cell<Option<Notify>> = const { Cell::new(None) };
}

fn with_win<R>(hwnd: HWND, f: impl FnOnce(&mut Win) -> R) -> Option<R> {
    WINS.with(|v| {
        let mut v = v.try_borrow_mut().ok()?;
        v.iter_mut().find(|w| w.hwnd == hwnd).map(f)
    })
}

/// Hàm hành động cho Lưu/Chép/Tải lên/Ghim (main: `run_on_image`, `pin::open`).
pub fn set_action_sink(f: Sink) {
    SINK.with(|s| s.set(Some(f)));
}

/// Hàm báo qua khay (main).
pub fn set_notifier(f: Notify) {
    NOTIFY.with(|n| n.set(Some(f)));
}

fn act(action: Action, image: Image) -> bool {
    match SINK.with(|s| s.get()) {
        Some(f) => {
            f(action, image);
            true
        }
        None => false,
    }
}

fn notify(kind: NoticeKind, title: &str, body: &str) {
    if let Some(f) = NOTIFY.with(|n| n.get()) {
        f(kind, title, body);
    }
}

/// Mở một cửa sổ kết quả cho ảnh ghép `image` có `uncertain` chỗ ghép không chắc chắn. Gọi trên
/// luồng giao diện. Không tạo được cửa sổ thì ảnh được lưu như Lưu thường (không bao giờ mất ảnh).
pub fn open(image: Image, uncertain: u32) {
    prune();
    let (hwnds, used): (Vec<HWND>, Vec<bool>) =
        WINS.with(|v| v.borrow().iter().map(|w| (w.hwnd, w.used)).unzip());
    let mut warn = false;
    match open_policy(&used) {
        OpenPolicy::Open => {}
        OpenPolicy::CloseOldestUsed(i) => {
            if let Some(&h) = hwnds.get(i) {
                destroy(h);
                prune();
            }
        }
        OpenPolicy::OpenAndWarn => warn = true,
    }
    if let Err(image) = create(image, uncertain) {
        act(Action::Save, image);
    }
    if warn {
        let lang = lang::current();
        notify(
            NoticeKind::Info,
            t(lang, Msg::ScrollManyOpenTitle),
            t(lang, Msg::ScrollManyOpenBody),
        );
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
    // SAFETY: cửa sổ kết quả của luồng này; dọn dẹp ở WM_NCDESTROY.
    unsafe {
        let _ = DestroyWindow(hwnd);
    }
}

/// Đóng mọi cửa sổ kết quả (thoát ứng dụng); ảnh chưa lưu bị bỏ.
pub fn close_all() {
    let all: Vec<HWND> = WINS.with(|v| v.borrow().iter().map(|w| w.hwnd).collect());
    for h in all {
        destroy(h);
    }
    prune();
}

/// Đổi ngôn ngữ khi cửa sổ đang mở: tiêu đề, chữ nút; dải cảnh báo và dòng thông tin vẽ lại.
pub fn relabel() {
    let lang = lang::current();
    let all: Vec<(HWND, [HWND; 5])> =
        WINS.with(|v| v.borrow().iter().map(|w| (w.hwnd, w.buttons)).collect());
    for (hwnd, buttons) in all {
        set_text(hwnd, t(lang, Msg::ScrollResultTitle));
        for (b, m) in buttons.iter().zip(BUTTON_LABELS) {
            set_text(*b, t(lang, m));
        }
        relayout(hwnd);
    }
}

fn set_text(h: HWND, s: &str) {
    // SAFETY: cửa sổ kết quả hoặc điều khiển con của nó; chuỗi sống suốt lời gọi.
    unsafe {
        let _ = SetWindowTextW(h, &HSTRING::from(s));
    }
}

/// `true`: thông điệp đã được xử lý. Chỉ đụng thông điệp của cửa sổ kết quả (và con của nó).
pub fn pre_translate(msg: &MSG) -> bool {
    let owner = WINS.with(|v| {
        v.try_borrow().ok().and_then(|v| {
            v.iter()
                .find(|w| winutil::belongs_to(w.hwnd, msg))
                .map(|w| w.hwnd)
        })
    });
    let Some(hwnd) = owner else {
        return false;
    };
    if msg.message == WM_KEYDOWN {
        let vk = msg.wParam.0 as u16;
        let cmd = match vk {
            v if v == VK_ESCAPE.0 => {
                destroy(hwnd);
                return true;
            }
            v if v == VK_PRIOR.0 => Some(ScrollCmd::PageUp),
            v if v == VK_NEXT.0 => Some(ScrollCmd::PageDown),
            v if v == VK_HOME.0 => Some(ScrollCmd::Top),
            v if v == VK_END.0 => Some(ScrollCmd::Bottom),
            _ => None,
        };
        if let Some(cmd) = cmd {
            scroll_by(hwnd, cmd);
            return true;
        }
    }
    // SAFETY: xử lý phím điều hướng còn lại (Tab, Enter, Space trên nút) của cửa sổ này.
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

/// Cỡ cửa sổ (pixel) cho vùng client `cw`×`ch` pixel ở `dpi`.
fn window_size(cw: i32, ch: i32, dpi: u32) -> (i32, i32) {
    let mut rc = RECT {
        left: 0,
        top: 0,
        right: cw,
        bottom: ch,
    };
    // SAFETY: rc là biến cục bộ.
    unsafe {
        let _ = AdjustWindowRectExForDpi(&mut rc, STYLE, false, EX_STYLE, dpi);
    }
    (rc.right - rc.left, rc.bottom - rc.top)
}

fn scrollbar_width(dpi: u32) -> i32 {
    // SAFETY: chỉ hỏi số đo hệ thống.
    match unsafe { GetSystemMetricsForDpi(SM_CXVSCROLL, dpi) } {
        v if v > 0 => v,
        _ => scale(17, dpi),
    }
}

fn delete_font(f: Option<HFONT>) {
    if let Some(f) = f {
        // SAFETY: phông do cửa sổ này tạo; không điều khiển nào còn dùng.
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

/// Bề rộng chữ các nút (phông đậm, bi quan cho mọi nút), bề rộng câu cảnh báo (nếu có) và chiều
/// cao một dòng, đo trên `dc`.
fn measure_texts(
    dc: HDC,
    lang: Lang,
    uncertain: u32,
    font: Option<HFONT>,
    bold: Option<HFONT>,
) -> ([i32; 5], Option<i32>, i32) {
    let btn = BUTTON_LABELS.map(|m| measure(dc, bold.or(font), t(lang, m)).0);
    let warn =
        (uncertain > 0).then(|| measure(dc, font, &i18n::scroll_uncertain(lang, uncertain)).0);
    let line_h = measure(dc, font, "Ag").1.max(1);
    (btn, warn, line_h)
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

/// Tạo, đăng ký và hiện cửa sổ. Lỗi thì trả ảnh lại (không còn gì sót).
fn create(image: Image, uncertain: u32) -> Result<(), Image> {
    if !image.is_valid() {
        return Err(image);
    }
    let lang = lang::current();
    // Màn hình có con trỏ; tạo thẳng theo DPI của nó (không có WM_DPICHANGED lúc mở).
    // SAFETY: tham số ra là biến cục bộ; MonitorFromPoint chỉ đọc.
    let mon = unsafe {
        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST)
    };
    let dpi = monitor_dpi(mon);
    let font = winutil::system_font(dpi);
    let bold = winutil::system_font_styled(dpi, 100, true);
    let (btn, warn, line_h) =
        crate::history_paint::with_screen_dc(|dc| measure_texts(dc, lang, uncertain, font, bold));
    // Phần client ngoài khung xem, đo ở bề rộng tối thiểu (dải cảnh báo nhiều dòng nhất).
    let (pw, ph) = (scale(MIN_W, dpi), scale(MIN_H, dpi) * 4);
    let probe = layout(pw, ph, dpi, btn, warn, line_h, scrollbar_width(dpi));
    let (chrome_w, chrome_h) = (pw - probe.view.w, ph - probe.view.h);
    let work = work_area(mon).unwrap_or(Rect {
        x: 0,
        y: 0,
        w: 1024,
        h: 768,
    });
    let (cw, ch) = default_client(work, image.width, image.height, dpi, chrome_w, chrome_h);
    let (ww, wh) = window_size(cw, ch, dpi);
    let x = (work.x + (work.w - ww) / 2).max(work.x);
    let y = (work.y + (work.h - wh) / 2).max(work.y);
    let title = HSTRING::from(t(lang, Msg::ScrollResultTitle));
    // SAFETY: đăng ký lớp và tạo cửa sổ ở luồng chính; đăng ký lại lần hai báo lỗi vô hại.
    let created = unsafe {
        GetModuleHandleW(None).ok().and_then(|hinst| {
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
            .ok()
        })
    };
    let Some(hwnd) = created else {
        delete_font(font);
        delete_font(bold);
        return Err(image);
    };
    let push = WINDOW_STYLE(WS_TABSTOP.0 | WS_GROUP.0 | BS_PUSHBUTTON as u32);
    let buttons: [HWND; 5] = std::array::from_fn(|i| {
        child(
            hwnd,
            w!("BUTTON"),
            t(lang, BUTTON_LABELS[i]),
            push,
            BUTTON_IDS[i],
        )
    });
    let scrollbar = child(
        hwnd,
        w!("SCROLLBAR"),
        "",
        WINDOW_STYLE(SBS_VERT),
        ID_SCROLLBAR,
    );
    if buttons.iter().any(|h| h.is_invalid()) || scrollbar.is_invalid() {
        // Chưa đăng ký vào sổ: hủy cửa sổ (kéo theo điều khiển con), rồi xóa phông.
        destroy(hwnd);
        delete_font(font);
        delete_font(bold);
        return Err(image);
    }
    for b in buttons {
        set_font(b, font);
    }
    theme::set_dark_control_theme(scrollbar, theme::DarkClass::Explorer);
    // SAFETY: hwnd vừa tạo ở luồng này.
    let dpi = match unsafe { GetDpiForWindow(hwnd) } {
        0 => dpi,
        d => d,
    };
    WINS.with(|v| {
        v.borrow_mut().push(Win {
            hwnd,
            buttons,
            scrollbar,
            image: Arc::new(image),
            uncertain,
            used: false,
            bmp: None,
            job: None,
            scroll: 0,
            disp: (0, 0),
            layout: Layout {
                strip: ZERO,
                view: ZERO,
                scrollbar: ZERO,
                info: ZERO,
                buttons: [ZERO; 5],
            },
            dpi,
            font,
            bold,
        })
    });
    relayout(hwnd);
    start_prepare(hwnd);
    theme::apply_dark_title_bar(hwnd);
    winutil::set_app_icons(hwnd);
    // SAFETY: hwnd là cửa sổ của luồng này.
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
        let _ = SetFocus(Some(buttons[0]));
    }
    Ok(())
}

/// Dựng ảnh để vẽ: ảnh thấp thì đổi định dạng ngay; ảnh cao thì thu nhỏ trên luồng nền.
fn start_prepare(hwnd: HWND) {
    let Some((img, disp_w)) = with_win(hwnd, |w| (w.image.clone(), w.disp.0)) else {
        return;
    };
    if needs_prebuild(img.height) {
        let tw = prebuild_width(img.width, disp_w);
        spawn_job(hwnd, img, tw);
    } else {
        let bmp = downscale(&img, img.width, &AtomicBool::new(false))
            .and_then(|p| bmp_from_pixels(p.w, p.h, &p.px));
        with_win(hwnd, |w| w.bmp = bmp);
    }
}

/// Bắt đầu một lượt thu nhỏ về bề rộng `tw` trên luồng nền (lượt cũ, nếu có, bị hủy). Không tạo
/// được luồng thì thu nhỏ ngay trên luồng này.
fn spawn_job(hwnd: HWND, img: Arc<Image>, tw: u32) {
    let id = NEXT_JOB.with(|n| {
        let v = n.get();
        n.set(v.wrapping_add(1).max(1));
        v
    });
    let job = Arc::new(Job {
        id,
        cancel: AtomicBool::new(false),
        out: Mutex::new(None),
    });
    let Some(()) = with_win(hwnd, |w| {
        if let Some(old) = w.job.replace(job.clone()) {
            old.cancel.store(true, Ordering::Relaxed);
        }
    }) else {
        return;
    };
    let (job2, img2) = (job.clone(), img.clone());
    let raw = hwnd.0 as isize;
    let spawned = std::thread::Builder::new()
        .name("darkshot-scroll-preview".into())
        .spawn(move || {
            let res = downscale(&img2, tw, &job2.cancel);
            drop(img2);
            if job2.cancel.load(Ordering::Relaxed) {
                return;
            }
            if let Ok(mut out) = job2.out.lock() {
                *out = res;
            }
            // SAFETY: chỉ đăng thông điệp không mang con trỏ; cửa sổ đã hủy thì lời gọi thất bại vô hại.
            unsafe {
                let _ = PostMessageW(
                    Some(HWND(raw as *mut c_void)),
                    WM_SCROLL_PREPARED,
                    WPARAM(id),
                    LPARAM(0),
                );
            }
        });
    if spawned.is_err() {
        let bmp = downscale(&img, tw, &AtomicBool::new(false))
            .and_then(|p| bmp_from_pixels(p.w, p.h, &p.px));
        with_win(hwnd, |w| {
            w.job = None;
            if bmp.is_some() {
                w.bmp = bmp;
            }
        });
        invalidate(hwnd);
    }
}

/// Lượt dựng `id` xong: lấy kết quả (đúng lượt đang chờ), tạo bitmap, vẽ lại.
fn on_prepared(hwnd: HWND, id: usize) {
    let Some(job) = with_win(hwnd, |w| w.job.clone()).flatten() else {
        return;
    };
    if job.id != id {
        return;
    }
    let res = job.out.lock().ok().and_then(|mut o| o.take());
    drop(job);
    let bmp = res.and_then(|p| bmp_from_pixels(p.w, p.h, &p.px));
    with_win(hwnd, |w| {
        w.job = None;
        if bmp.is_some() {
            w.bmp = bmp;
        }
    });
    maybe_rebuild(hwnd);
    invalidate(hwnd);
}

/// Khung xem rộng hơn ảnh thu nhỏ đang có: dựng lại (không lượt nào đang chạy).
fn maybe_rebuild(hwnd: HWND) {
    let Some(next) = with_win(hwnd, |w| {
        let built = w.bmp.as_ref().map(|b| b.w.max(0) as u32)?;
        (w.job.is_none()
            && needs_prebuild(w.image.height)
            && needs_rebuild(built, w.image.width, w.disp.0))
        .then(|| (w.image.clone(), prebuild_width(w.image.width, w.disp.0)))
    })
    .flatten() else {
        return;
    };
    spawn_job(hwnd, next.0, next.1);
}

fn invalidate(hwnd: HWND) {
    // SAFETY: vẽ lại cửa sổ này (nền do WM_PAINT tự vẽ).
    unsafe {
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
}

/// Tính lại bố cục theo vùng client hiện tại và DPI: đặt điều khiển, cỡ hiển thị, giữ vị trí cuộn
/// tương đối, cập nhật thanh cuộn, dựng lại ảnh thu nhỏ nếu khung xem rộng ra.
fn relayout(hwnd: HWND) {
    let Some((dpi, font, bold, uncertain, buttons, sb, iw, ih)) = with_win(hwnd, |w| {
        (
            w.dpi,
            w.font,
            w.bold,
            w.uncertain,
            w.buttons,
            w.scrollbar,
            w.image.width,
            w.image.height,
        )
    }) else {
        return;
    };
    let mut rc = RECT::default();
    // SAFETY: rc là biến cục bộ; hwnd là cửa sổ của luồng này.
    unsafe {
        let _ = GetClientRect(hwnd, &mut rc);
    }
    let lang = lang::current();
    // SAFETY: DC của cửa sổ này, trả ngay sau khi đo.
    let measured = unsafe {
        let dc = GetDC(Some(hwnd));
        if dc.is_invalid() {
            None
        } else {
            let m = measure_texts(dc, lang, uncertain, font, bold);
            ReleaseDC(Some(hwnd), dc);
            Some(m)
        }
    };
    let Some((btn, warn, line_h)) = measured else {
        return;
    };
    let l = layout(
        rc.right - rc.left,
        rc.bottom - rc.top,
        dpi,
        btn,
        warn,
        line_h,
        scrollbar_width(dpi),
    );
    for (h, r) in buttons
        .iter()
        .zip(l.buttons)
        .chain(std::iter::once((&sb, l.scrollbar)))
    {
        // SAFETY: điều khiển con của cửa sổ này.
        unsafe {
            let _ = SetWindowPos(*h, None, r.x, r.y, r.w, r.h, SWP_NOZORDER | SWP_NOACTIVATE);
        }
    }
    let (dw, dh) = fit_width(iw, ih, l.view.w);
    with_win(hwnd, |w| {
        let pos = rescaled_scroll(w.scroll, w.disp.1, dh);
        w.scroll = pos.clamp(0, max_scroll(dh, l.view.h));
        w.disp = (dw, dh);
        w.layout = l;
    });
    update_scrollbar(hwnd);
    maybe_rebuild(hwnd);
    invalidate(hwnd);
}

fn update_scrollbar(hwnd: HWND) {
    let Some((sb, si)) = with_win(hwnd, |w| {
        (
            w.scrollbar,
            SCROLLINFO {
                cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
                fMask: SIF_RANGE | SIF_PAGE | SIF_POS | SIF_DISABLENOSCROLL,
                nMin: 0,
                nMax: (w.disp.1 - 1).max(0),
                nPage: w.layout.view.h.max(0) as u32,
                nPos: w.scroll,
                nTrackPos: 0,
            },
        )
    }) else {
        return;
    };
    // SAFETY: si là biến cục bộ có cbSize đúng; thanh cuộn là điều khiển con của cửa sổ này.
    unsafe {
        SetScrollInfo(sb, SB_CTL, &si, true);
    }
}

/// Cuộn theo lệnh `cmd`; đổi vị trí thì cập nhật thanh cuộn và vẽ lại khung xem.
fn scroll_by(hwnd: HWND, cmd: ScrollCmd) {
    let Some((changed, view)) = with_win(hwnd, |w| {
        let line = scale(LINE_DIP, w.dpi);
        let next = scrolled(w.scroll, cmd, w.disp.1, w.layout.view.h, line);
        let changed = next != w.scroll;
        w.scroll = next;
        (changed, w.layout.view)
    }) else {
        return;
    };
    if changed {
        update_scrollbar(hwnd);
        let rc = rect_of(view);
        // SAFETY: vẽ lại khung xem của cửa sổ này.
        unsafe {
            let _ = InvalidateRect(Some(hwnd), Some(&rc), false);
        }
    }
}

/// WM_VSCROLL (từ thanh cuộn hoặc gửi thẳng tới cửa sổ).
fn on_vscroll(hwnd: HWND, wp: WPARAM) {
    let code = (wp.0 & 0xFFFF) as i32;
    let cmd = match code {
        c if c == SB_LINEUP.0 => ScrollCmd::LineUp,
        c if c == SB_LINEDOWN.0 => ScrollCmd::LineDown,
        c if c == SB_PAGEUP.0 => ScrollCmd::PageUp,
        c if c == SB_PAGEDOWN.0 => ScrollCmd::PageDown,
        c if c == SB_TOP.0 => ScrollCmd::Top,
        c if c == SB_BOTTOM.0 => ScrollCmd::Bottom,
        c if c == SB_THUMBTRACK.0 || c == SB_THUMBPOSITION.0 => {
            // Vị trí 32 bit của núm (HIWORD chỉ có 16 bit).
            let Some(sb) = with_win(hwnd, |w| w.scrollbar) else {
                return;
            };
            let mut si = SCROLLINFO {
                cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
                fMask: SIF_TRACKPOS,
                ..Default::default()
            };
            // SAFETY: si là biến cục bộ có cbSize đúng; thanh cuộn của cửa sổ này.
            let pos = if unsafe { GetScrollInfo(sb, SB_CTL, &mut si) }.is_ok() {
                si.nTrackPos
            } else {
                ((wp.0 >> 16) & 0xFFFF) as i32
            };
            ScrollCmd::To(pos)
        }
        _ => return,
    };
    scroll_by(hwnd, cmd);
}

fn rect_of(r: Rect) -> RECT {
    RECT {
        left: r.x,
        top: r.y,
        right: r.right(),
        bottom: r.bottom(),
    }
}

/// Chép dải hàng [sy, sy + sh) của `bmp` vào `dst` (HALFTONE khi khác cỡ).
fn stretch_band(dc: HDC, bmp: &Bmp, dst: &RECT, sy: i32, sh: i32) {
    let (dw, dh) = (dst.right - dst.left, dst.bottom - dst.top);
    if dw <= 0 || dh <= 0 || sh <= 0 {
        return;
    }
    // SAFETY: DC tạm tạo ở đây được trả bitmap cũ và xóa trên mọi đường đi; bitmap thuộc `bmp`.
    unsafe {
        let mem = CreateCompatibleDC(Some(dc));
        if mem.is_invalid() {
            return;
        }
        let old = SelectObject(mem, HGDIOBJ(bmp.handle.0));
        if dw == bmp.w && dh == sh {
            let _ = BitBlt(dc, dst.left, dst.top, dw, dh, Some(mem), 0, sy, SRCCOPY);
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
                sy,
                bmp.w,
                sh,
                SRCCOPY,
            );
            SetStretchBltMode(dc, STRETCH_BLT_MODE(prev));
        }
        SelectObject(mem, old);
        let _ = DeleteDC(mem);
    }
}

fn paint(hwnd: HWND) {
    let mut ps = PAINTSTRUCT::default();
    // SAFETY: BeginPaint/EndPaint cân bằng trong khối.
    let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
    if !hdc.is_invalid() {
        let mut rc = RECT::default();
        // SAFETY: rc là biến cục bộ.
        unsafe {
            let _ = GetClientRect(hwnd, &mut rc);
        }
        let lang = lang::current();
        theme::buffered(hdc, &rc, |dc| {
            theme::fill(dc, &rc, BG);
            // Vẽ trong một lần mượn sổ: các hàm GDI dưới đây không gửi thông điệp.
            with_win(hwnd, |w| draw(dc, w, lang));
        });
    }
    // SAFETY: cặp với BeginPaint ở trên.
    unsafe {
        let _ = EndPaint(hwnd, &ps);
    }
}

fn draw(dc: HDC, w: &Win, lang: Lang) {
    let l = &w.layout;
    let scale_dpi = |v| scale(v, w.dpi);
    if l.strip.h > 0 {
        let strip = rect_of(l.strip);
        theme::fill(dc, &strip, SURFACE);
        let bar = RECT {
            right: strip.left + scale_dpi(STRIP_BAR),
            ..strip
        };
        theme::fill(dc, &bar, WARNING);
        let pad = scale_dpi(STRIP_PAD);
        let text = RECT {
            left: bar.right + pad,
            top: strip.top + pad,
            right: strip.right - pad,
            bottom: strip.bottom - pad,
        };
        theme::draw_text(
            dc,
            &crate::history_paint::wide(&i18n::scroll_uncertain(lang, w.uncertain)),
            &text,
            WARNING,
            w.font,
            DT_LEFT | DT_WORDBREAK | DT_NOPREFIX,
        );
    }
    let view = rect_of(l.view);
    theme::fill(dc, &view, INPUT);
    let (dw, dh) = w.disp;
    match &w.bmp {
        Some(bmp) if dw > 0 && dh > 0 => {
            let vis = l.view.h.min(dh - w.scroll);
            let (sy, sh) = src_band(w.scroll, vis, dh, bmp.h);
            let left = l.view.x + (l.view.w - dw) / 2;
            let dst = RECT {
                left,
                top: l.view.y,
                right: left + dw,
                bottom: l.view.y + vis,
            };
            stretch_band(dc, bmp, &dst, sy, sh);
        }
        _ if w.job.is_some() => {
            theme::draw_text(
                dc,
                &crate::history_paint::wide(t(lang, Msg::ScrollPreparing)),
                &view,
                TEXT_MUTED,
                w.font,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );
        }
        _ => {}
    }
    theme::draw_text(
        dc,
        &crate::history_paint::wide(&i18n::scroll_size(lang, w.image.width, w.image.height)),
        &rect_of(l.info),
        TEXT_MUTED,
        w.font,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
    );
}

/// Nút Lưu/Chép/Tải lên/Ghim: giao bản sao ảnh cho hàm hành động; cửa sổ không đóng.
fn on_action(hwnd: HWND, action: Action) {
    let Some(img) = with_win(hwnd, |w| w.image.clone()) else {
        return;
    };
    let Some(copy) = try_clone(&img) else {
        return;
    };
    drop(img);
    if act(action, copy) {
        with_win(hwnd, |w| w.used = true);
    }
}

/// Đổi DPI: phông mới cho các nút, xóa phông cũ sau khi không ai dùng.
fn on_dpi_changed(hwnd: HWND, dpi: u32) {
    let font = winutil::system_font(dpi);
    let bold = winutil::system_font_styled(dpi, 100, true);
    let Some((old, buttons)) = with_win(hwnd, |w| {
        w.dpi = dpi;
        let old = [
            std::mem::replace(&mut w.font, font),
            std::mem::replace(&mut w.bold, bold),
        ];
        (old, w.buttons)
    }) else {
        delete_font(font);
        delete_font(bold);
        return;
    };
    for b in buttons {
        set_font(b, font);
    }
    for f in old {
        delete_font(f);
    }
}

fn lo_word(v: usize) -> u32 {
    (v & 0xFFFF) as u32
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_SCROLL_PREPARED => {
            on_prepared(hwnd, wp.0);
            LRESULT(0)
        }
        WM_COMMAND => {
            let (id, code) = ((wp.0 & 0xFFFF) as i32, (wp.0 >> 16) & 0xFFFF);
            if code == 0 {
                match id {
                    ID_SAVE => on_action(hwnd, Action::Save),
                    ID_COPY => on_action(hwnd, Action::Copy),
                    ID_UPLOAD => on_action(hwnd, Action::Upload),
                    ID_PIN => on_action(hwnd, Action::Pin),
                    ID_CLOSE => destroy(hwnd),
                    _ => {}
                }
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            destroy(hwnd);
            LRESULT(0)
        }
        WM_VSCROLL => {
            on_vscroll(hwnd, wp);
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            let delta = ((wp.0 >> 16) & 0xFFFF) as u16 as i16;
            scroll_by(hwnd, ScrollCmd::Wheel(i32::from(delta)));
            LRESULT(0)
        }
        WM_NOTIFY => {
            // SAFETY: lp của WM_NOTIFY trỏ tới NMHDR hợp lệ suốt lời gọi.
            let hdr = unsafe { (lp.0 as *const NMHDR).as_ref() };
            let kind = match hdr.filter(|h| h.code == NM_CUSTOMDRAW).map(|h| h.idFrom) {
                Some(id) if id == ID_SAVE as usize => Some(theme::ButtonKind::Primary),
                Some(id) if BUTTON_IDS.iter().any(|b| *b as usize == id) => {
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
        // WM_PAINT vẽ hết vùng client (qua bộ đệm): không xóa nền để khỏi nháy.
        WM_ERASEBKGND => LRESULT(1),
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
            let (mw, mh) = window_size(scale(MIN_W, dpi), scale(MIN_H, dpi), dpi);
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
        WM_SETFOCUS => {
            if let Some(save) = with_win(hwnd, |w| w.buttons[0]) {
                // SAFETY: nút của cửa sổ này.
                unsafe {
                    let _ = SetFocus(Some(save));
                }
            }
            LRESULT(0)
        }
        WM_NCDESTROY => {
            let removed = WINS.with(|v| {
                let mut v = v.try_borrow_mut().ok()?;
                let i = v.iter().position(|w| w.hwnd == hwnd)?;
                Some(v.remove(i))
            });
            // Drop: hủy lượt dựng, xóa phông, bitmap và ảnh.
            drop(removed);
            // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
            unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
        }
        // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
        _ => unsafe { DefWindowProcW(hwnd, msg, wp, lp) },
    }
}

#[cfg(test)]
fn test_windows() -> Vec<HWND> {
    WINS.with(|v| v.borrow().iter().map(|w| w.hwnd).collect())
}

/// (vị trí cuộn, vị trí cuộn tối đa).
#[cfg(test)]
fn scroll_of(h: HWND) -> (i32, i32) {
    with_win(h, |w| (w.scroll, max_scroll(w.disp.1, w.layout.view.h))).unwrap_or((-1, -1))
}

#[cfg(test)]
fn preparing(h: HWND) -> bool {
    with_win(h, |w| w.bmp.is_none() && w.job.is_some()).unwrap_or(false)
}

#[cfg(test)]
fn strip_of(h: HWND) -> Rect {
    with_win(h, |w| w.layout.strip).unwrap_or(ZERO)
}

#[cfg(test)]
fn used_of(h: HWND) -> bool {
    with_win(h, |w| w.used).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::winutil::scale;
    use darkshot_core::i18n::{t, Lang, Msg};
    use std::cell::RefCell;
    use std::time::{Duration, Instant};
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::System::Threading::{
        GetCurrentProcess, GetGuiResources, GR_GDIOBJECTS, GR_USEROBJECTS,
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::{VK_END, VK_ESCAPE, VK_HOME, VK_NEXT};
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, IsWindow, PeekMessageW, SendMessageW, TranslateMessage, PM_REMOVE,
        SB_BOTTOM, SB_LINEDOWN, SB_PAGEDOWN, SB_TOP, WM_COMMAND, WM_KEYDOWN, WM_MOUSEWHEEL,
        WM_VSCROLL,
    };

    const LANGS: [Lang; 2] = [Lang::Vi, Lang::En];
    const DPIS: [u32; 5] = [96, 120, 144, 168, 192];
    const LABELS: [Msg; 5] = [
        Msg::ScrollSave,
        Msg::ScrollCopy,
        Msg::ScrollUpload,
        Msg::ScrollPin,
        Msg::ScrollClose,
    ];

    /// Bề rộng chữ bi quan (mọi ký tự rộng 10 DIP) để kiểm thử không cắt chữ.
    fn text_px(s: &str, dpi: u32) -> i32 {
        s.chars().count() as i32 * scale(10, dpi)
    }

    fn r(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
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
    fn constants_match_design() {
        assert_eq!(MAX_WINDOWS, 4);
        assert_eq!((MIN_W, MIN_H), (480, 400));
        assert_eq!(PREBUILD_MIN_H, 4096);
        assert_eq!(WM_SCROLL_PREPARED, WM_APP + 17);
    }

    /// Mọi DPI 96–192, hai ngôn ngữ, có/không dải cảnh báo, cỡ tối thiểu/mặc định/lớn: các phần nằm
    /// trong vùng client, không chồng nhau, nút đủ rộng cho chữ, dải cảnh báo đủ dòng cho câu.
    #[test]
    fn layout_fits_without_overlap() {
        for lang in LANGS {
            for dpi in DPIS {
                let btn = LABELS.map(|m| text_px(t(lang, m), dpi));
                let warn = text_px(&darkshot_core::i18n::scroll_uncertain(lang, 12), dpi);
                let line_h = scale(18, dpi);
                let sb_w = scale(17, dpi);
                for (w, h) in [(MIN_W, MIN_H), (640, 728), (1600, 1000)] {
                    for warn_text in [None, Some(warn)] {
                        let (cw, ch) = (scale(w, dpi), scale(h, dpi));
                        let client = r(0, 0, cw, ch);
                        let l = layout(cw, ch, dpi, btn, warn_text, line_h, sb_w);
                        let ctx = format!("{lang:?} dpi {dpi} {w}x{h} warn {warn_text:?}: {l:?}");
                        let mut parts = vec![l.view, l.scrollbar, l.info];
                        parts.extend(l.buttons);
                        if warn_text.is_some() {
                            parts.push(l.strip);
                        } else {
                            assert_eq!(l.strip.h, 0, "{ctx}");
                        }
                        for (i, a) in parts.iter().enumerate() {
                            assert!(a.w > 0 && a.h > 0, "{ctx}");
                            assert!(within(*a, client), "{ctx}");
                            for b in &parts[i + 1..] {
                                assert!(!overlaps(*a, *b), "{ctx}");
                            }
                        }
                        assert!(l.view.h >= scale(100, dpi), "{ctx}");
                        assert_eq!(l.scrollbar.w, sb_w, "{ctx}");
                        assert_eq!(l.scrollbar.x, l.view.right(), "{ctx}");
                        assert_eq!((l.scrollbar.y, l.scrollbar.h), (l.view.y, l.view.h));
                        // Nút đủ rộng cho chữ, cùng hàng, theo thứ tự, nút cuối sát lề phải.
                        for (i, b) in l.buttons.iter().enumerate() {
                            assert!(b.w >= btn[i] + 2 * scale(12, dpi), "{ctx}");
                            assert_eq!(b.y, l.buttons[0].y, "{ctx}");
                            if i > 0 {
                                assert!(l.buttons[i - 1].right() <= b.x, "{ctx}");
                            }
                        }
                        assert_eq!(l.buttons[4].right(), cw - scale(12, dpi), "{ctx}");
                        assert!(l.info.bottom() <= l.buttons[0].y, "{ctx}");
                        assert!(l.view.bottom() <= l.info.y, "{ctx}");
                        assert!(l.info.h >= scale(16, dpi), "{ctx}");
                        if let Some(wt) = warn_text {
                            assert!(l.strip.bottom() <= l.view.y, "{ctx}");
                            // Đủ dòng: (cao − đệm) / dòng × bề rộng chữ ≥ câu.
                            let lines = (l.strip.h - 2 * scale(8, dpi)) / line_h;
                            let text_w = l.strip.w - 3 * scale(8, dpi) - scale(3, dpi);
                            assert!(lines >= 1 && lines * text_w >= wt, "{ctx}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn fit_width_scales_down_only() {
        assert_eq!(fit_width(900, 12000, 450), (450, 6000));
        assert_eq!(fit_width(900, 12000, 900), (900, 12000));
        // Không phóng to ảnh hẹp hơn khung xem.
        assert_eq!(fit_width(300, 1000, 900), (300, 1000));
        // Làm tròn lên, không bao giờ 0 hàng.
        assert_eq!(fit_width(1000, 3, 999), (999, 3));
        assert_eq!(fit_width(1000, 1, 10), (10, 1));
        assert_eq!(fit_width(900, 12000, 0), (0, 0));
        assert_eq!(fit_width(0, 10, 100), (0, 0));
        // Không tràn với ảnh lớn nhất cho phép.
        assert_eq!(fit_width(8000, 30_000, 4000), (4000, 15_000));
    }

    #[test]
    fn scroll_range_and_commands() {
        assert_eq!(max_scroll(6000, 500), 5500);
        assert_eq!(max_scroll(400, 500), 0);
        assert_eq!(max_scroll(0, 0), 0);
        let (dh, vh, line) = (6000, 500, 40);
        let s = |pos, cmd| scrolled(pos, cmd, dh, vh, line);
        assert_eq!(s(0, ScrollCmd::LineDown), 40);
        assert_eq!(s(100, ScrollCmd::LineUp), 60);
        assert_eq!(s(10, ScrollCmd::LineUp), 0);
        assert_eq!(s(0, ScrollCmd::PageDown), 460);
        assert_eq!(s(1000, ScrollCmd::PageUp), 540);
        assert_eq!(s(1000, ScrollCmd::Top), 0);
        assert_eq!(s(0, ScrollCmd::Bottom), 5500);
        assert_eq!(s(5400, ScrollCmd::PageDown), 5500);
        assert_eq!(s(0, ScrollCmd::To(9999)), 5500);
        assert_eq!(s(0, ScrollCmd::To(-5)), 0);
        assert_eq!(s(0, ScrollCmd::To(1234)), 1234);
        // Con lăn: một nấc xuống (−120) = 3 dòng; bàn di chuột gửi nấc nhỏ.
        assert_eq!(s(0, ScrollCmd::Wheel(-120)), 120);
        assert_eq!(s(500, ScrollCmd::Wheel(120)), 380);
        assert_eq!(s(0, ScrollCmd::Wheel(-40)), 40);
        assert_eq!(s(0, ScrollCmd::Wheel(120)), 0);
        // Ảnh vừa khung: không cuộn được.
        assert_eq!(scrolled(0, ScrollCmd::Bottom, 300, 500, line), 0);
        // Trang tối thiểu một dòng khi khung xem rất thấp.
        assert_eq!(scrolled(0, ScrollCmd::PageDown, 6000, 30, 40), 40);
    }

    #[test]
    fn scroll_kept_on_resize() {
        assert_eq!(rescaled_scroll(1000, 6000, 12000), 2000);
        assert_eq!(rescaled_scroll(1000, 6000, 3000), 500);
        assert_eq!(rescaled_scroll(0, 6000, 3000), 0);
        assert_eq!(rescaled_scroll(100, 0, 3000), 0);
        assert_eq!(rescaled_scroll(29_000, 30_000, 30_000), 29_000);
    }

    #[test]
    fn source_band_maps_visible_rows() {
        // Bitmap cùng cỡ hiển thị: đúng các hàng thấy.
        assert_eq!(src_band(100, 500, 6000, 6000), (100, 500));
        // Bitmap gốc cao gấp đôi.
        assert_eq!(src_band(100, 500, 6000, 12000), (200, 1000));
        // Làm tròn: đầu xuống, cuối lên.
        assert_eq!(src_band(1, 1, 3, 10), (3, 4));
        // Không vượt đáy bitmap.
        let (y0, h) = src_band(5500, 500, 6000, 11_999);
        assert!(y0 + h <= 11_999 && h > 0, "{y0} {h}");
        // Luôn ít nhất một hàng.
        assert_eq!(src_band(0, 1, 6000, 10).1, 1);
        assert_eq!(src_band(0, 0, 0, 10), (0, 0));
    }

    #[test]
    fn default_size_follows_work_area() {
        let work = r(-1920, 0, 1920, 1040);
        let (cw, chh) = (scale(41, 96), scale(120, 96));
        // Ảnh rất cao: cao 70% vùng làm việc, rộng tối thiểu.
        let (w, h) = default_client(work, 900, 12000, 96, cw, chh);
        assert_eq!(h, 1040 * 70 / 100);
        assert_eq!(w, 480);
        // Ảnh thấp, rộng: rộng theo tỉ lệ, không quá bề rộng ảnh.
        let (w, h) = default_client(work, 1600, 900, 96, cw, chh);
        assert_eq!(h, 728);
        assert_eq!(w, 1600 * (728 - chh) / 900 + cw);
        let (w, _) = default_client(work, 700, 600, 96, cw, chh);
        assert_eq!(w, 700 + cw);
        // Không quá 90% bề rộng vùng làm việc.
        let (w, _) = default_client(work, 7000, 800, 96, cw, chh);
        assert_eq!(w, 1920 * 90 / 100);
        // DPI cao: tối thiểu 480×400 DIP.
        let (w, h) = default_client(r(0, 0, 3840, 1000), 900, 12000, 192, cw, chh);
        assert_eq!((w, h), (960, 800));
        // Màn hình nhỏ hơn tối thiểu: không vượt vùng làm việc.
        let (w, h) = default_client(r(0, 0, 800, 600), 900, 12000, 192, cw, chh);
        assert_eq!((w, h), (800, 600));
    }

    #[test]
    fn prebuild_rules() {
        assert!(!needs_prebuild(4096));
        assert!(needs_prebuild(4097));
        assert!(needs_prebuild(30_000));
        assert_eq!(prebuild_width(900, 450), 450);
        assert_eq!(prebuild_width(900, 1200), 900);
        assert_eq!(prebuild_width(900, 0), 1);
        // Khung xem rộng hơn bitmap đã dựng (và ảnh gốc còn rộng hơn): dựng lại.
        assert!(needs_rebuild(450, 900, 600));
        assert!(!needs_rebuild(450, 900, 450));
        assert!(!needs_rebuild(450, 900, 300));
        assert!(!needs_rebuild(900, 900, 1200));
    }

    fn solid(w: u32, h: u32, bgra: [u8; 4]) -> Image {
        Image {
            width: w,
            height: h,
            bgra: bgra.repeat((w * h) as usize),
        }
    }

    #[test]
    fn downscale_averages_and_drops_alpha() {
        let no = AtomicBool::new(false);
        // Giữ màu đặc, bỏ alpha.
        let p = downscale(&solid(8, 40, [10, 20, 30, 255]), 4, &no).expect("ok");
        assert_eq!((p.w, p.h), (4, 20));
        assert!(p.px.iter().all(|&v| v == 0x001E140A), "{:x}", p.px[0]);
        // Cùng bề rộng: chỉ đổi định dạng.
        let p = downscale(&solid(3, 2, [1, 2, 3, 4]), 3, &no).expect("ok");
        assert_eq!((p.w, p.h, p.px.len()), (3, 2, 6));
        assert!(p.px.iter().all(|&v| v == 0x00030201));
        // Rộng hơn ảnh: không phóng to.
        let p = downscale(&solid(3, 2, [1, 2, 3, 4]), 10, &no).expect("ok");
        assert_eq!((p.w, p.h), (3, 2));
        // Cột đen/trắng xen kẽ thu một nửa: xám (trung bình).
        let mut img = solid(4, 2, [0, 0, 0, 255]);
        for y in 0..2usize {
            for x in [1usize, 3] {
                let i = (y * 4 + x) * 4;
                img.bgra[i..i + 3].copy_from_slice(&[255, 255, 255]);
            }
        }
        let p = downscale(&img, 2, &no).expect("ok");
        assert_eq!((p.w, p.h), (2, 1));
        for v in p.px {
            let b = v & 0xFF;
            assert!((127..=128).contains(&b), "{v:x}");
            assert_eq!(v >> 24, 0);
        }
        // Ảnh cao 900×12000 thu còn 450 rộng.
        let p = downscale(&solid(900, 12000, [5, 5, 5, 255]), 450, &no).expect("ok");
        assert_eq!((p.w, p.h, p.px.len()), (450, 6000, 450 * 6000));
        // Hủy: dừng, không trả gì.
        assert!(downscale(
            &solid(900, 12000, [5, 5, 5, 255]),
            450,
            &AtomicBool::new(true)
        )
        .is_none());
        // Ảnh hỏng: không panic.
        let bad = Image {
            width: 10,
            height: 10,
            bgra: vec![0; 7],
        };
        assert!(downscale(&bad, 5, &no).is_none());
    }

    /// Không bao giờ tự đóng cửa sổ kết quả; từ cửa sổ thứ năm thì báo.
    #[test]
    fn never_closes_windows_warns_from_fifth() {
        assert!(!warns_on_open(0));
        assert!(!warns_on_open(3));
        assert!(warns_on_open(4));
        assert!(warns_on_open(9));
    }

    /// Chiều cao hiển thị và chiều cao ảnh dựng trước dùng chung một công thức làm tròn: đúng tỉ lệ
    /// thì vẽ bằng BitBlt, không lệch 1 px.
    #[test]
    fn display_height_matches_prebuilt_height() {
        let no = AtomicBool::new(false);
        assert_eq!(fit_width(901, 12000, 450), (450, 5993));
        for (w, h, view) in [
            (901, 12000, 450),
            (900, 12000, 450),
            (1000, 4097, 333),
            (7, 5000, 3),
        ] {
            let (dw, dh) = fit_width(w, h, view);
            let p =
                downscale(&solid(w, h, [1, 2, 3, 255]), prebuild_width(w, dw), &no).expect("ok");
            assert_eq!((p.w as i32, p.h as i32), (dw, dh), "{w}x{h} view {view}");
        }
    }

    /// Lượt dựng thất bại ở một bề rộng: không thử lại đúng bề rộng đó, chỉ khi khung xem đổi.
    #[test]
    fn rebuild_skips_failed_width() {
        assert_eq!(rebuild_target(Some(450), 900, 600, None), Some(600));
        assert_eq!(rebuild_target(Some(450), 900, 600, Some(600)), None);
        assert_eq!(rebuild_target(Some(450), 900, 650, Some(600)), Some(650));
        assert_eq!(rebuild_target(Some(450), 900, 450, None), None);
        assert_eq!(rebuild_target(Some(900), 900, 1200, None), None);
        // Chưa có ảnh nào (lượt đầu thất bại): thử lại khi bề rộng đổi.
        assert_eq!(rebuild_target(None, 900, 450, Some(450)), None);
        assert_eq!(rebuild_target(None, 900, 500, Some(450)), Some(500));
        assert_eq!(rebuild_target(None, 900, 1200, Some(900)), None);
    }

    // ---- Chạy tay (mở cửa sổ thật; không lớp phủ, không dời chuột/phím thật):
    // cargo test -p darkshot-windows scroll_result::tests::manual -- --ignored --test-threads=1

    thread_local! {
        static ACTIONS: RefCell<Vec<(Action, u32, u32)>> = const { RefCell::new(Vec::new()) };
        static NOTICES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    fn record_action(a: Action, img: Image) {
        ACTIONS.with(|v| v.borrow_mut().push((a, img.width, img.height)));
    }

    fn record_notice(_: NoticeKind, title: &str, _: &str) {
        NOTICES.with(|v| v.borrow_mut().push(title.to_string()));
    }

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

    /// Ảnh tổng hợp: mỗi hàng một mức xám khác nhau theo chu kỳ.
    fn tall(w: u32, h: u32) -> Image {
        let mut bgra = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            let v = (y % 251) as u8;
            for _ in 0..w {
                bgra.extend_from_slice(&[v, v.wrapping_add(40), v.wrapping_add(80), 255]);
            }
        }
        Image {
            width: w,
            height: h,
            bgra,
        }
    }

    fn send(h: HWND, msg: u32, wp: usize) {
        // SAFETY: gửi tới cửa sổ kết quả do luồng này tạo.
        unsafe {
            SendMessageW(h, msg, Some(WPARAM(wp)), Some(LPARAM(0)));
        }
    }

    fn key(h: HWND, vk: u16) -> bool {
        let m = MSG {
            hwnd: h,
            message: WM_KEYDOWN,
            wParam: WPARAM(vk as usize),
            ..Default::default()
        };
        pre_translate(&m)
    }

    fn alive(h: HWND) -> bool {
        // SAFETY: chỉ hỏi tính hợp lệ của handle.
        unsafe { IsWindow(Some(h)) }.as_bool()
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

    /// Ảnh 900×12000: dựng ảnh thu nhỏ trên luồng nền, rồi cuộn bằng con lăn, thanh cuộn, phím.
    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_scrolls_tall_image() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        close_all();
        open(tall(900, 12000), 0);
        let wins = test_windows();
        assert_eq!(wins.len(), 1);
        let h = wins[0];
        assert!(preparing(h), "must prebuild a 12000 px tall image");
        assert!(pump(15_000, || !preparing(h)), "prebuild not done in 15 s");
        let (pos, max) = scroll_of(h);
        assert_eq!(pos, 0);
        assert!(max > 0, "{max}");
        assert_eq!(strip_of(h).h, 0);
        // Con lăn xuống một nấc.
        send(h, WM_MOUSEWHEEL, ((-120i16) as u16 as usize) << 16);
        let (p1, _) = scroll_of(h);
        assert!(p1 > 0, "{p1}");
        send(h, WM_VSCROLL, SB_LINEDOWN.0 as usize);
        assert!(scroll_of(h).0 > p1);
        send(h, WM_VSCROLL, SB_BOTTOM.0 as usize);
        assert_eq!(scroll_of(h).0, max);
        send(h, WM_VSCROLL, SB_TOP.0 as usize);
        assert_eq!(scroll_of(h).0, 0);
        send(h, WM_VSCROLL, SB_PAGEDOWN.0 as usize);
        assert!(scroll_of(h).0 > p1);
        assert!(key(h, VK_END.0));
        assert_eq!(scroll_of(h).0, max);
        assert!(key(h, VK_HOME.0));
        assert_eq!(scroll_of(h).0, 0);
        assert!(key(h, VK_NEXT.0));
        assert!(scroll_of(h).0 > 0);
        pump(100, || false);
        assert!(key(h, VK_ESCAPE.0));
        assert!(!alive(h));
        assert!(test_windows().is_empty());
    }

    /// Dải cảnh báo chỉ khi có chỗ ghép không chắc chắn; ảnh thấp không cần dựng nền.
    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_warning_strip() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        close_all();
        open(tall(600, 2000), 3);
        open(tall(600, 2000), 0);
        let wins = test_windows();
        assert_eq!(wins.len(), 2);
        assert!(!preparing(wins[0]) && !preparing(wins[1]));
        assert!(strip_of(wins[0]).h > 0);
        assert_eq!(strip_of(wins[1]).h, 0);
        pump(100, || false);
        close_all();
        assert!(test_windows().is_empty());
    }

    /// Không bao giờ tự đóng cửa sổ (kể cả cửa sổ đã Lưu); từ cửa sổ thứ năm vẫn mở và báo Info.
    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_window_policy() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        close_all();
        set_action_sink(record_action);
        set_notifier(record_notice);
        ACTIONS.with(|v| v.borrow_mut().clear());
        NOTICES.with(|v| v.borrow_mut().clear());
        for _ in 0..4 {
            open(tall(300, 500), 0);
        }
        let wins = test_windows();
        assert_eq!(wins.len(), 4);
        assert!(NOTICES.with(|v| v.borrow().is_empty()));
        // Lưu trên cửa sổ thứ hai: bản sao ảnh tới hàm hành động, cửa sổ không đóng.
        send(wins[1], WM_COMMAND, ID_SAVE as usize);
        assert_eq!(
            ACTIONS.with(|v| v.borrow().clone()),
            vec![(Action::Save, 300, 500)]
        );
        assert!(alive(wins[1]));
        open(tall(300, 500), 0);
        open(tall(300, 500), 0);
        assert_eq!(test_windows().len(), 6);
        assert!(wins.iter().all(|h| alive(*h)));
        let title = t(crate::lang::current(), Msg::ScrollManyOpenTitle).to_string();
        assert_eq!(
            NOTICES.with(|v| v.borrow().clone()),
            vec![title.clone(), title]
        );
        pump(100, || false);
        close_all();
        assert!(test_windows().is_empty());
    }

    /// Mở/đóng 10 lần (cả ảnh cao dựng nền, cả khi đóng lúc đang dựng): không rò GDI/USER.
    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_open_close_no_leak() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        close_all();
        open(tall(300, 500), 1);
        pump(100, || false);
        close_all();
        pump(200, || false);
        let before = gui_counts();
        for i in 0..10 {
            open(tall(900, if i % 2 == 0 { 12000 } else { 1500 }), i);
            if i % 3 != 0 {
                let h = test_windows()[0];
                pump(15_000, || !preparing(h));
            }
            pump(30, || false);
            close_all();
            assert!(test_windows().is_empty());
        }
        pump(1_000, || false);
        let after = gui_counts();
        assert!(
            after.0 <= before.0 && after.1 <= before.1,
            "GDI/USER grew: {before:?} -> {after:?}"
        );
    }

    /// Đổi ngôn ngữ khi đang mở: tiêu đề theo ngôn ngữ mới.
    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_relabel() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        close_all();
        let before = crate::lang::current();
        crate::lang::set(Lang::Vi);
        open(tall(300, 500), 1);
        let h = test_windows()[0];
        let title = || String::from_utf16_lossy(&crate::theme::window_text(h));
        assert_eq!(title(), t(Lang::Vi, Msg::ScrollResultTitle));
        crate::lang::set(Lang::En);
        relabel();
        assert_eq!(title(), t(Lang::En, Msg::ScrollResultTitle));
        pump(100, || false);
        close_all();
        crate::lang::set(before);
    }
}
