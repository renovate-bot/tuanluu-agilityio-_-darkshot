//! Cửa sổ Duyệt của Privacy Shield: tối, topmost, đổi cỡ được, tự vẽ hoàn toàn (một HWND, không
//! điều khiển con). Trái: ảnh co vừa khung (vùng chọn = ô đen đặc, bỏ chọn = viền đứt nét, mục đang
//! trỏ = viền màu nhấn). Phải: tóm tắt, Chọn tất cả / Bỏ chọn hết, danh sách mục. Dưới: hàng nút và
//! thanh xác nhận (không MessageBox). Logic ở `darkshot_core::privacy::review::ReviewModel`.
//!
//! Kết quả: mỗi lựa chọn của người dùng được gửi bằng `PostMessageW(notify, msg, token, Box<
//! ReviewOutcome>)`; bên nhận lấy lại Box bằng `take_outcome`. Kết quả cuối (Che, Tiếp tục, Hủy) gửi
//! đúng MỘT lần rồi cửa sổ tự đóng; đóng bằng ✕/Alt+F4/Esc mà chưa gửi gì thì gửi `Cancel` (ở
//! `WM_DESTROY`). `Retry` không kết thúc: cửa sổ về "Đang quét…" và chờ `update`. `close(token)` và
//! `close_all()` đóng mà không gửi. Không bao giờ hiện chữ OCR hay giá trị trùng khớp: chỉ nhãn loại
//! và độ tin cậy. Không làm việc tệp hay mạng; không ghi log.

use crate::history_paint::{self, Bmp};
use crate::lang;
use crate::ocr;
use crate::ocr_view;
use crate::theme::{self, ButtonKind, ControlState};
use crate::winutil::{self, scale};
use darkshot_core::geometry::Rect;
use darkshot_core::i18n::{self, t, Lang, Msg};
use darkshot_core::image::Image;
use darkshot_core::privacy::model::Confidence;
use darkshot_core::privacy::{
    Confirm, Finding, RedactionSource, ReviewModel, ReviewOutcome, ReviewPhase, ScanError,
};
use darkshot_core::theme::{
    Rgb, ACCENT, BG, INPUT, LINE, SURFACE, TAB_HOVER, TEXT, TEXT_MUTED, TRACK, WARNING, WHITE,
};
use std::cell::RefCell;
use std::sync::Arc;
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreatePen, DeleteObject, EndPaint, GetMonitorInfoW, GetStockObject,
    IntersectClipRect, InvalidateRect, MonitorFromPoint, Rectangle, RestoreDC, SaveDC,
    ScreenToClient, SelectObject, SetBkColor, SetBkMode, DRAW_TEXT_FORMAT, DT_END_ELLIPSIS,
    DT_LEFT, DT_NOPREFIX, DT_RIGHT, DT_SINGLELINE, DT_VCENTER, DT_WORDBREAK, HDC, HFONT, HGDIOBJ,
    HMONITOR, MONITORINFO, MONITOR_DEFAULTTONEAREST, NULL_BRUSH, OPAQUE, PAINTSTRUCT, PS_DOT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::WM_MOUSELEAVE;
use windows::Win32::UI::HiDpi::{
    AdjustWindowRectExForDpi, GetDpiForMonitor, GetDpiForWindow, MDT_EFFECTIVE_DPI,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, ReleaseCapture, SetCapture, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT,
    VIRTUAL_KEY, VK_CONTROL, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_HOME, VK_RETURN, VK_SHIFT,
    VK_SPACE, VK_TAB, VK_UP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetCursorPos, IsWindow,
    LoadCursorW, PeekMessageW, PostMessageW, RegisterClassW, SetCursor, SetForegroundWindow,
    SetWindowDisplayAffinity, SetWindowPos, ShowWindow, HTCLIENT, IDC_ARROW, IDC_CROSS, IDC_HAND,
    MINMAXINFO, MSG, PM_REMOVE, SWP_NOACTIVATE, SWP_NOZORDER, SW_SHOW, WDA_EXCLUDEFROMCAPTURE,
    WINDOW_EX_STYLE, WINDOW_STYLE, WM_APP, WM_CAPTURECHANGED, WM_CLOSE, WM_DESTROY, WM_DPICHANGED,
    WM_ERASEBKGND, WM_GETMINMAXINFO, WM_KEYDOWN, WM_KILLFOCUS, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCDESTROY, WM_PAINT, WM_SETCURSOR, WM_SETFOCUS, WM_SIZE,
    WNDCLASSW, WS_CAPTION, WS_EX_APPWINDOW, WS_EX_TOPMOST, WS_MAXIMIZEBOX, WS_MINIMIZEBOX,
    WS_SYSMENU, WS_THICKFRAME,
};

/// Kết quả gửi cho cổng: WPARAM = token, LPARAM = `Box<ReviewOutcome>` (lấy bằng `take_outcome`).
pub const WM_PRIVACY_REVIEW: u32 = WM_APP + 20;

/// Vùng client mặc định và tối thiểu (DIP).
const DEFAULT_W: i32 = 960;
const DEFAULT_H: i32 = 640;
const MIN_W: i32 = 640;
const MIN_H: i32 = 420;

const CLASS: PCWSTR = w!("darkshot-privacy-review");

const STYLE: WINDOW_STYLE = WINDOW_STYLE(
    WS_CAPTION.0 | WS_SYSMENU.0 | WS_THICKFRAME.0 | WS_MINIMIZEBOX.0 | WS_MAXIMIZEBOX.0,
);
/// Topmost: cửa sổ chặn một hành động người dùng vừa yêu cầu.
const EX_STYLE: WINDOW_EX_STYLE = WINDOW_EX_STYLE(WS_EX_APPWINDOW.0 | WS_EX_TOPMOST.0);

// Bố cục (DIP).
const MARGIN: i32 = 12;
const GAP: i32 = 8;
const BTN_H: i32 = 28;
const BTN_MIN_W: i32 = 72;
const BTN_PAD: i32 = 14;
const LINE_H: i32 = 20;
const ROW_H: i32 = 28;
const NOTE_H: i32 = 40;
const DETAIL_H: i32 = 64;
const CONFIRM_H: i32 = 40;
const RIGHT_MIN: i32 = 250;
const RIGHT_MAX: i32 = 380;
const RIGHT_SHARE: i32 = 36;
const PREVIEW_MIN: i32 = 200;
const VIEW_PAD: i32 = 8;
const REMOVE_H: i32 = 22;
const REMOVE_PAD: i32 = 8;
const SCROLL_W: i32 = 4;
/// Một nấc bánh xe cuộn danh sách bao nhiêu DIP.
const WHEEL_DIP: i32 = 3 * ROW_H;
/// Bản giữ để vẽ xem trước: co vừa hộp này (≤ 16 MP), bản hiển thị co tiếp từ nó khi đổi cỡ.
const BASE_MAX: u32 = 4096;
/// Màu ô che trên ảnh xem trước (giống ô che thật: đen đặc).
const BLACK: Rgb = Rgb(0, 0, 0);

/// Yêu cầu mở cửa sổ Duyệt (từ cổng xuất).
pub struct ReviewRequest {
    pub token: usize,
    pub image: Arc<Image>,
    /// Tên hành động đang chờ (ví dụ `Msg::PrivacyActionSave`).
    pub action: Msg,
    /// `Some((k, n))`: duyệt bước k trên n của lượt xuất các bước (tiêu đề "Bước k/n").
    pub step: Option<(u32, u32)>,
    pub phase: ReviewPhase,
    pub findings: Vec<Finding>,
}

/// Điều khiển tự vẽ (cũng là thứ tự Tab).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ctl {
    SelectAll,
    DeselectAll,
    List,
    Cancel,
    Retry,
    Add,
    Primary,
    Back,
    Yes,
}

/// Phần thân theo trạng thái.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Body {
    Scanning,
    Findings,
    Error,
}

fn body_of(p: ReviewPhase) -> Body {
    match p {
        ReviewPhase::Scanning { .. } => Body::Scanning,
        ReviewPhase::Findings => Body::Findings,
        ReviewPhase::Error(_) => Body::Error,
    }
}

/// Nhóm nút trái và phải của hàng dưới. Trên thanh xác nhận, "Quay lại" nằm đúng chỗ nút chính
/// (phải) và nút đi tiếp nằm bên trái: bấm đúp vào nút chính không bao giờ trúng nút đi tiếp.
fn footer(body: Body, confirm: Confirm) -> (&'static [Ctl], &'static [Ctl]) {
    if confirm != Confirm::None {
        return (&[Ctl::Yes], &[Ctl::Back]);
    }
    match body {
        Body::Scanning => (&[Ctl::Cancel], &[]),
        Body::Findings => (&[Ctl::Cancel], &[Ctl::Add, Ctl::Primary]),
        Body::Error => (&[Ctl::Cancel, Ctl::Retry], &[Ctl::Add, Ctl::Primary]),
    }
}

/// Nút đi tiếp trên thanh xác nhận chỉ nhận bấm khi thanh đã hiện ít nhất `dbl_ms` (thời gian bấm
/// đúp của hệ thống) tính từ `shown_ms` (đồng hồ `GetTickCount`, tính cả lúc quay vòng).
fn yes_allowed(shown_ms: u32, now_ms: u32, dbl_ms: u32) -> bool {
    now_ms.wrapping_sub(shown_ms) >= dbl_ms
}

fn tick_ms() -> u32 {
    // SAFETY: hàm hệ thống không tham số.
    unsafe { windows::Win32::System::SystemInformation::GetTickCount() }
}

fn double_click_ms() -> u32 {
    // SAFETY: hàm hệ thống không tham số.
    unsafe { windows::Win32::UI::Input::KeyboardAndMouse::GetDoubleClickTime() }
}

/// Điều khiển nhận tiêu điểm theo thứ tự Tab.
fn focusables(body: Body, confirm: Confirm, has_rows: bool) -> Vec<Ctl> {
    if confirm != Confirm::None {
        return vec![Ctl::Back, Ctl::Yes];
    }
    let mut v = Vec::new();
    if body == Body::Findings {
        v.extend([Ctl::SelectAll, Ctl::DeselectAll]);
    }
    if has_rows && body != Body::Scanning {
        v.push(Ctl::List);
    }
    let (l, r) = footer(body, confirm);
    v.extend_from_slice(l);
    v.extend_from_slice(r);
    v
}

/// Tiêu điểm mặc định: nút chính (Enter = nút chính), "Quay lại" trên thanh xác nhận, "Hủy" khi quét.
fn default_focus(body: Body, confirm: Confirm) -> Ctl {
    match (confirm, body) {
        (Confirm::None, Body::Scanning) => Ctl::Cancel,
        (Confirm::None, _) => Ctl::Primary,
        _ => Ctl::Back,
    }
}

fn label(ctl: Ctl, body: Body, confirm: Confirm, lang: Lang) -> &'static str {
    let m = match ctl {
        Ctl::SelectAll => Msg::PrivacySelectAll,
        Ctl::DeselectAll => Msg::PrivacyDeselectAll,
        Ctl::List => return "",
        Ctl::Cancel => Msg::PrivacyCancel,
        Ctl::Retry => Msg::PrivacyRetry,
        Ctl::Add => Msg::PrivacyAddArea,
        Ctl::Primary if body == Body::Error => Msg::PrivacyContinueUnscanned,
        Ctl::Primary => Msg::PrivacyApply,
        Ctl::Back => Msg::PrivacyBack,
        Ctl::Yes if confirm == Confirm::Unscanned => Msg::PrivacyContinueAnyway,
        Ctl::Yes => Msg::PrivacyContinueNone,
    };
    t(lang, m)
}

fn kind_of(ctl: Ctl, body: Body) -> ButtonKind {
    match ctl {
        Ctl::Primary if body == Body::Findings => ButtonKind::Primary,
        Ctl::Retry => ButtonKind::Primary,
        // Hai nút đi tiếp mà không che / không quét: màu nguy hiểm.
        Ctl::Yes => ButtonKind::Danger,
        _ => ButtonKind::Normal,
    }
}

fn confirm_text(c: Confirm, lang: Lang) -> &'static str {
    match c {
        Confirm::None => "",
        Confirm::EmptySelection => t(lang, Msg::PrivacyConfirmEmpty),
        Confirm::Unscanned => t(lang, Msg::PrivacyConfirmUnscanned),
    }
}

// ---- Bố cục (thuần, kiểm thử được)

#[derive(Clone, Debug, PartialEq, Eq)]
struct Layout {
    /// Khung ảnh (viền) và chỗ ảnh co vừa trong đó.
    preview: Rect,
    image: Rect,
    right: Rect,
    /// Dòng tóm tắt / tiêu đề lỗi / "Đang quét…".
    summary: Rect,
    /// Lý do lỗi (nhiều dòng) hoặc "dải k/n".
    detail: Rect,
    list: Rect,
    note: Rect,
    /// Thanh xác nhận (rỗng khi không có).
    confirm: Rect,
    /// Đường kẻ trên hàng nút.
    footer_y: i32,
    buttons: Vec<(Ctl, Rect)>,
}

const ZERO: Rect = Rect {
    x: 0,
    y: 0,
    w: 0,
    h: 0,
};

impl Layout {
    fn empty() -> Layout {
        Layout {
            preview: ZERO,
            image: ZERO,
            right: ZERO,
            summary: ZERO,
            detail: ZERO,
            list: ZERO,
            note: ZERO,
            confirm: ZERO,
            footer_y: 0,
            buttons: Vec::new(),
        }
    }
}

/// Bề rộng nút cho chữ rộng `text_px`.
fn button_width(text_px: i32, dpi: u32) -> i32 {
    (text_px + 2 * scale(BTN_PAD, dpi)).max(scale(BTN_MIN_W, dpi))
}

/// Ảnh `iw`×`ih` co vừa `area` (không phóng quá 100%), căn giữa.
fn fit(iw: u32, ih: u32, area: Rect) -> Rect {
    if iw == 0 || ih == 0 || area.w <= 0 || area.h <= 0 {
        return Rect {
            x: area.x,
            y: area.y,
            w: 0,
            h: 0,
        };
    }
    let (iw, ih) = (i64::from(iw), i64::from(ih));
    let (aw, ah) = (i64::from(area.w), i64::from(area.h));
    let (dw, dh) = if iw <= aw && ih <= ah {
        (iw, ih)
    } else if iw * ah >= ih * aw {
        (aw, (ih * aw / iw).max(1))
    } else {
        ((iw * ah / ih).max(1), ah)
    };
    let (dw, dh) = (dw as i32, dh as i32);
    Rect {
        x: area.x + (area.w - dw) / 2,
        y: area.y + (area.h - dh) / 2,
        w: dw,
        h: dh,
    }
}

/// Bố cục cho vùng client `cw`×`ch` px. `widths(ctl)` = bề rộng chữ (px) của nút.
fn layout(
    cw: i32,
    ch: i32,
    dpi: u32,
    body: Body,
    confirm: Confirm,
    image: (u32, u32),
    widths: &dyn Fn(Ctl) -> i32,
) -> Layout {
    let s = |v: i32| scale(v, dpi);
    let (m, gap) = (s(MARGIN), s(GAP));
    let btn_h = s(BTN_H);
    let footer_y = (ch - btn_h - 2 * m).max(0);
    // Hàng nút: nhóm trái từ mép trái, nhóm phải từ mép phải; thiếu chỗ thì co mọi nút theo tỉ lệ.
    let (left, right) = footer(body, confirm);
    let mut want: Vec<(Ctl, i32)> = left
        .iter()
        .chain(right)
        .map(|&c| {
            // "Quay lại" phủ hết chỗ của nút chính cũ: lần bấm thứ hai của cú bấm đúp trúng nó.
            let tw = if c == Ctl::Back {
                widths(c).max(widths(Ctl::Primary))
            } else {
                widths(c)
            };
            (c, button_width(tw, dpi))
        })
        .collect();
    let n = want.len() as i32;
    let split = if !left.is_empty() && !right.is_empty() {
        2 * gap
    } else {
        0
    };
    let avail = (cw - 2 * m - gap * (n - 1).max(0) - split).max(0);
    let total: i32 = want.iter().map(|w| w.1).sum();
    if total > avail && total > 0 {
        for w in &mut want {
            w.1 = (i64::from(w.1) * i64::from(avail) / i64::from(total)) as i32;
        }
    }
    let by = footer_y + m;
    let mut buttons = Vec::new();
    let mut x = m;
    for &(c, w) in &want[..left.len()] {
        buttons.push((
            c,
            Rect {
                x,
                y: by,
                w,
                h: btn_h,
            },
        ));
        x += w + gap;
    }
    let mut x = cw - m;
    for &(c, w) in want[left.len()..].iter().rev() {
        x -= w;
        buttons.push((
            c,
            Rect {
                x,
                y: by,
                w,
                h: btn_h,
            },
        ));
        x -= gap;
    }
    // Thanh xác nhận ngay trên hàng nút, rộng hết cửa sổ.
    let confirm_r = if confirm != Confirm::None {
        Rect {
            x: m,
            y: (footer_y - s(CONFIRM_H) - gap).max(0),
            w: (cw - 2 * m).max(0),
            h: s(CONFIRM_H),
        }
    } else {
        ZERO
    };
    let body_bottom = if confirm != Confirm::None {
        confirm_r.y - gap
    } else {
        footer_y - m
    };
    let body_h = (body_bottom - m).max(0);
    // Khung phải: tỉ lệ bề rộng, kẹp [RIGHT_MIN, RIGHT_MAX], chừa chỗ tối thiểu cho ảnh.
    let inner = (cw - 2 * m - gap).max(0);
    let rw = (cw * RIGHT_SHARE / 100)
        .clamp(s(RIGHT_MIN), s(RIGHT_MAX))
        .min((inner - s(PREVIEW_MIN)).max(0));
    let pw = (inner - rw).max(0);
    let preview = Rect {
        x: m,
        y: m,
        w: pw,
        h: body_h,
    };
    let pad = s(VIEW_PAD);
    let image_r = fit(
        image.0,
        image.1,
        Rect {
            x: preview.x + pad,
            y: preview.y + pad,
            w: (preview.w - 2 * pad).max(0),
            h: (preview.h - 2 * pad).max(0),
        },
    );
    let right_r = Rect {
        x: preview.right() + gap,
        y: m,
        w: rw,
        h: body_h,
    };
    let line = s(LINE_H);
    let summary = Rect { h: line, ..right_r };
    let mut y = summary.bottom() + gap;
    let detail = match body {
        Body::Scanning => Rect {
            y,
            h: line,
            ..right_r
        },
        Body::Error => Rect {
            y,
            h: s(DETAIL_H),
            ..right_r
        },
        Body::Findings => Rect { y, h: 0, ..right_r },
    };
    if detail.h > 0 {
        y = detail.bottom() + gap;
    }
    if body == Body::Findings {
        let bw = ((rw - gap) / 2).max(0);
        buttons.push((
            Ctl::SelectAll,
            Rect {
                x: right_r.x,
                y,
                w: bw,
                h: btn_h,
            },
        ));
        buttons.push((
            Ctl::DeselectAll,
            Rect {
                x: right_r.x + bw + gap,
                y,
                w: bw,
                h: btn_h,
            },
        ));
        y += btn_h + gap;
    }
    let note = if body == Body::Scanning {
        ZERO
    } else {
        Rect {
            x: right_r.x,
            y: (right_r.bottom() - s(NOTE_H)).max(y),
            w: rw,
            h: s(NOTE_H).min((right_r.bottom() - y).max(0)),
        }
    };
    let list = if body == Body::Scanning {
        ZERO
    } else {
        Rect {
            x: right_r.x,
            y,
            w: rw,
            h: (note.y - gap - y).max(0),
        }
    };
    Layout {
        preview,
        image: image_r,
        right: right_r,
        summary,
        detail,
        list,
        note,
        confirm: confirm_r,
        footer_y,
        buttons,
    }
}

/// Khung ảnh `r` (px ảnh gốc) trên ảnh hiển thị `d` (ảnh gốc `img`): luôn ≥ 1 px mỗi cạnh.
fn to_display(r: Rect, img: (u32, u32), d: Rect) -> RECT {
    let (iw, ih) = (i64::from(img.0.max(1)), i64::from(img.1.max(1)));
    let (dw, dh) = (i64::from(d.w), i64::from(d.h));
    let fx = |v: i32| (i64::from(v) * dw).div_euclid(iw) as i32;
    let fy = |v: i32| (i64::from(v) * dh).div_euclid(ih) as i32;
    let cx = |v: i32| (i64::from(v) * dw + iw - 1).div_euclid(iw) as i32;
    let cy = |v: i32| (i64::from(v) * dh + ih - 1).div_euclid(ih) as i32;
    let left = d.x + fx(r.x);
    let top = d.y + fy(r.y);
    RECT {
        left,
        top,
        right: (d.x + cx(r.right())).max(left + 1),
        bottom: (d.y + cy(r.bottom())).max(top + 1),
    }
}

/// Điểm client → px ảnh gốc (không kẹp; dùng cho kéo vẽ).
fn to_image(x: i32, y: i32, img: (u32, u32), d: Rect) -> (i32, i32) {
    if d.w <= 0 || d.h <= 0 {
        return (0, 0);
    }
    let px = (i64::from(x - d.x) * i64::from(img.0)).div_euclid(i64::from(d.w));
    let py = (i64::from(y - d.y) * i64::from(img.1)).div_euclid(i64::from(d.h));
    (px as i32, py as i32)
}

fn inside(r: Rect, x: i32, y: i32) -> bool {
    r.w > 0 && r.h > 0 && x >= r.x && y >= r.y && x < r.right() && y < r.bottom()
}

/// Cỡ bản giữ của ảnh `w`×`h`: vừa hộp `BASE_MAX`, không phóng.
fn base_size(w: u32, h: u32) -> (u32, u32) {
    let r = fit(
        w,
        h,
        Rect {
            x: 0,
            y: 0,
            w: BASE_MAX as i32,
            h: BASE_MAX as i32,
        },
    );
    (r.w.max(0) as u32, r.h.max(0) as u32)
}

// ---- Cửa sổ

/// Chuột đang giữ.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Press {
    None,
    Button(Ctl),
    Remove(usize),
    Drag,
}

struct Win {
    hwnd: HWND,
    token: usize,
    notify: HWND,
    msg: u32,
    /// Đã gửi kết quả cuối (hoặc bị đóng bởi cổng): không gửi gì thêm.
    sent: bool,
    model: ReviewModel,
    action: Msg,
    /// Ảnh chờ duyệt: giữ tới khi đóng (thả cùng cửa sổ).
    _image: Arc<Image>,
    /// Bản giữ (≤ `BASE_MAX`) và bản đã co cho khung hiện tại (`None` = vẽ thẳng bản giữ).
    base: Option<Bmp>,
    disp: Option<Bmp>,
    dpi: u32,
    font: Option<HFONT>,
    bold: Option<HFONT>,
    layout: Layout,
    focus: Ctl,
    hot: Option<Ctl>,
    hot_remove: Option<usize>,
    press: Press,
    /// Lúc thanh xác nhận hiện (`GetTickCount`), để chặn bấm đúp đi xuyên.
    confirm_at: u32,
    list_scroll: i32,
    tracking: bool,
    /// Bề rộng cột độ tin cậy và nút Xóa (px, đo lúc bố cục).
    conf_w: i32,
    remove_w: i32,
}

thread_local! {
    static WINS: RefCell<Vec<Win>> = const { RefCell::new(Vec::new()) };
}

fn with_win<R>(hwnd: HWND, f: impl FnOnce(&mut Win) -> R) -> Option<R> {
    WINS.with(|v| {
        let mut v = v.try_borrow_mut().ok()?;
        v.iter_mut().find(|w| w.hwnd == hwnd).map(f)
    })
}

fn hwnd_of(token: usize) -> Option<HWND> {
    WINS.with(|v| {
        let v = v.try_borrow().ok()?;
        v.iter().find(|w| w.token == token).map(|w| w.hwnd)
    })
}

/// Cửa sổ Duyệt của `token` còn mở (cổng dùng để dọn lượt khi cửa sổ mất mà không có kết quả).
pub fn is_open(token: usize) -> bool {
    // SAFETY: IsWindow chỉ hỏi tính hợp lệ của handle.
    hwnd_of(token).is_some_and(|h| unsafe { IsWindow(Some(h)) }.as_bool())
}

/// Gửi `out` cho cổng. Gửi lỗi thì lấy lại Box (không rò).
fn post(notify: HWND, msg: u32, token: usize, out: ReviewOutcome) {
    let p = Box::into_raw(Box::new(out));
    // SAFETY: Box vừa tạo; bên nhận lấy lại đúng một lần bằng `take_outcome`; gửi lỗi thì lấy lại ở đây.
    unsafe {
        if PostMessageW(Some(notify), msg, WPARAM(token), LPARAM(p as isize)).is_err() {
            drop(Box::from_raw(p));
        }
    }
}

/// Lấy lại `Box` kết quả của một thông điệp `msg` do cửa sổ Duyệt gửi.
///
/// # Safety
/// `lp` là LPARAM của đúng một thông điệp do cửa sổ này gửi (đã lấy khỏi hàng đợi), gọi một lần.
pub unsafe fn take_outcome(lp: LPARAM) -> Box<ReviewOutcome> {
    // SAFETY: theo hợp đồng của hàm, lp là con trỏ Box<ReviewOutcome> do `post` tạo.
    unsafe { Box::from_raw(lp.0 as *mut ReviewOutcome) }
}

/// Vét và giải phóng mọi kết quả còn trong hàng đợi của `notify` (bên nhận gọi khi bị hủy).
pub fn drain_outcomes(notify: HWND, msg: u32) {
    let mut m = MSG::default();
    // SAFETY: chỉ lấy thông điệp `msg` của `notify` khỏi hàng đợi của luồng này; mỗi Box lấy một lần.
    while unsafe { PeekMessageW(&mut m, Some(notify), msg, msg, PM_REMOVE) }.as_bool() {
        // SAFETY: thông điệp vừa lấy khỏi hàng đợi, do `post` gửi.
        drop(unsafe { take_outcome(m.lParam) });
    }
}

/// Gửi kết quả cuối (một lần) rồi đóng cửa sổ.
fn finish(hwnd: HWND, out: ReviewOutcome) {
    let target = with_win(hwnd, |w| {
        if w.sent {
            return None;
        }
        w.sent = true;
        Some((w.notify, w.msg, w.token))
    })
    .flatten();
    if let Some((n, m, tok)) = target {
        post(n, m, tok, out);
    }
    destroy(hwnd);
}

fn destroy(hwnd: HWND) {
    // SAFETY: cửa sổ của luồng này; dọn dẹp ở WM_DESTROY/WM_NCDESTROY.
    unsafe {
        let _ = DestroyWindow(hwnd);
    }
}

fn invalidate(hwnd: HWND) {
    // SAFETY: chỉ đánh dấu vẽ lại cửa sổ của luồng này.
    unsafe {
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
}

/// Cửa sổ Duyệt hiện ảnh CHƯA che: loại nó khỏi ảnh chụp màn hình (chụp mới, chụp cuộn, ghi GIF
/// không quét, chia sẻ màn hình) như thanh phiên (quyết định 122), khi Windows hỗ trợ
/// `WDA_EXCLUDEFROMCAPTURE` (bản dựng ≥ 19041; bản cũ coi cờ này như ô đen). Không đặt khi chạy
/// kiểm thử: cờ này làm `PrintWindow` ra ảnh trắng, mà QA dùng `PrintWindow` (quyết định 121).
fn exclude_from_capture(testing: bool, build: Option<u32>) -> bool {
    !testing && crate::scroll_session::exclude_supported(build)
}

/// Mở cửa sổ Duyệt. Kết quả người dùng gửi tới `notify` bằng thông điệp `msg` (WPARAM = token,
/// LPARAM = `Box<ReviewOutcome>`).
pub fn open(req: ReviewRequest, notify: HWND, msg: u32) -> Result<(), String> {
    prune();
    if hwnd_of(req.token).is_some() {
        return Err("review window already open for token".into());
    }
    let lang = lang::current();
    let (w, h) = (req.image.width, req.image.height);
    let base = base_bitmap(&req.image);
    let hwnd = create(lang, req.step).ok_or_else(|| "CreateWindowExW failed".to_string())?;
    if exclude_from_capture(cfg!(test), crate::scroll_session::os_build()) {
        // SAFETY: hwnd là cửa sổ cấp cao nhất vừa tạo của tiến trình này. Hệ thống từ chối thì bỏ
        // qua (cửa sổ vẫn dùng được).
        let _ = unsafe { SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE) };
    }
    // SAFETY: hwnd vừa tạo ở luồng này.
    let dpi = match unsafe { GetDpiForWindow(hwnd) } {
        0 => 96,
        d => d,
    };
    let model = ReviewModel::new(req.phase, req.findings, w, h);
    let body = body_of(model.phase());
    WINS.with(|v| {
        v.borrow_mut().push(Win {
            hwnd,
            token: req.token,
            notify,
            msg,
            sent: false,
            model,
            action: req.action,
            _image: req.image,
            base,
            disp: None,
            dpi,
            font: winutil::system_font(dpi),
            bold: winutil::system_font_styled(dpi, 100, true),
            layout: Layout::empty(),
            focus: default_focus(body, Confirm::None),
            hot: None,
            hot_remove: None,
            press: Press::None,
            confirm_at: 0,
            list_scroll: 0,
            tracking: false,
            conf_w: 0,
            remove_w: 0,
        })
    });
    relayout(hwnd);
    theme::apply_dark_title_bar(hwnd);
    winutil::set_app_icons(hwnd);
    // SAFETY: hwnd là cửa sổ của luồng này.
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
    }
    Ok(())
}

/// Bản giữ để vẽ xem trước (DIB, vừa hộp `BASE_MAX`).
fn base_bitmap(image: &Image) -> Option<Bmp> {
    if !image.is_valid() {
        return None;
    }
    let (w, h) = (image.width, image.height);
    let (bw, bh) = base_size(w, h);
    if (bw, bh) == (w, h) {
        ocr_view::bmp_from_bgra(w, h, &image.bgra)
    } else {
        ocr_view::bmp_from_bgra(bw, bh, &ocr::scale_bgra(&image.bgra, w, h, bw, bh))
    }
}

/// Kết quả quét về cho cửa sổ `token`. `false` khi không có cửa sổ đó.
pub fn update(token: usize, r: Result<Vec<Finding>, ScanError>) -> bool {
    let Some(hwnd) = hwnd_of(token) else {
        return false;
    };
    let changed = with_win(hwnd, |w| {
        let before = w.model.phase();
        w.model.set_result(r);
        if w.model.phase() == before {
            // Kết quả muộn (không còn Scanning): bỏ, không đụng trạng thái người dùng.
            return false;
        }
        w.focus = default_focus(body_of(w.model.phase()), w.model.confirm());
        w.list_scroll = 0;
        w.model.cancel_drag();
        true
    });
    if changed == Some(true) {
        release_capture(hwnd);
        relayout(hwnd);
    }
    true
}

/// Tiến độ quét: dải `tile` (từ 1) trên `tiles`.
pub fn progress(token: usize, tile: u32, tiles: u32) {
    if let Some(hwnd) = hwnd_of(token) {
        with_win(hwnd, |w| w.model.set_progress(tile, tiles));
        invalidate(hwnd);
    }
}

/// Đóng cửa sổ `token` mà không gửi kết quả (cổng đã quyết định).
pub fn close(token: usize) {
    if let Some(hwnd) = hwnd_of(token) {
        with_win(hwnd, |w| w.sent = true);
        destroy(hwnd);
    }
    prune();
}

/// Đóng mọi cửa sổ Duyệt (thoát ứng dụng), không gửi kết quả.
pub fn close_all() {
    let all: Vec<HWND> = WINS.with(|v| {
        v.borrow_mut()
            .iter_mut()
            .map(|w| {
                w.sent = true;
                w.hwnd
            })
            .collect()
    });
    for h in all {
        destroy(h);
    }
    prune();
}

/// Bỏ khỏi sổ các handle đã chết (WM_NCDESTROY đã gỡ trong trường hợp thường).
fn prune() {
    let dead: Vec<Win> = WINS.with(|v| {
        let Ok(mut v) = v.try_borrow_mut() else {
            return Vec::new();
        };
        // SAFETY: IsWindow chỉ hỏi tính hợp lệ của handle.
        let (dead, alive): (Vec<Win>, Vec<Win>) = std::mem::take(&mut *v)
            .into_iter()
            .partition(|w| !unsafe { IsWindow(Some(w.hwnd)) }.as_bool());
        *v = alive;
        dead
    });
    for w in dead {
        release(w);
    }
}

/// Giải phóng phông (bitmap và ảnh thả theo `Drop`).
fn release(w: Win) {
    for f in [w.font, w.bold].into_iter().flatten() {
        // SAFETY: phông do cửa sổ này tạo; không còn chọn vào DC nào.
        unsafe {
            let _ = DeleteObject(HGDIOBJ(f.0));
        }
    }
}

fn key_down(vk: VIRTUAL_KEY) -> bool {
    // SAFETY: chỉ đọc trạng thái phím của luồng này.
    unsafe { GetKeyState(i32::from(vk.0)) < 0 }
}

/// `true`: phím đã xử lý (vòng lặp bỏ qua). Chỉ đụng `WM_KEYDOWN` gửi tới cửa sổ Duyệt.
pub fn pre_translate(msg: &MSG) -> bool {
    if msg.message != WM_KEYDOWN {
        return false;
    }
    if !WINS.with(|v| {
        v.try_borrow()
            .map(|v| v.iter().any(|w| w.hwnd == msg.hwnd))
            .unwrap_or(false)
    }) {
        return false;
    }
    on_key(
        msg.hwnd,
        msg.wParam.0 as u16,
        key_down(VK_CONTROL),
        key_down(VK_SHIFT),
    )
}

/// Thao tác của một phím. `true` khi đã dùng phím.
fn on_key(hwnd: HWND, vk: u16, ctrl: bool, shift: bool) -> bool {
    let Some((body, confirm, adding, focus, rows)) = with_win(hwnd, |w| {
        (
            body_of(w.model.phase()),
            w.model.confirm(),
            w.model.adding(),
            w.focus,
            w.model.row_count(),
        )
    }) else {
        return false;
    };
    match vk {
        v if v == VK_ESCAPE.0 => {
            if adding {
                with_win(hwnd, |w| w.model.set_adding(false));
                release_capture(hwnd);
            } else if confirm != Confirm::None {
                activate(hwnd, Ctl::Back);
                return true;
            } else {
                finish(hwnd, ReviewOutcome::Cancel);
                return true;
            }
        }
        v if v == VK_RETURN.0 => {
            let target = if focus == Ctl::List {
                if confirm != Confirm::None {
                    Ctl::Back
                } else {
                    Ctl::Primary
                }
            } else {
                focus
            };
            activate(hwnd, target);
            return true;
        }
        v if v == VK_SPACE.0 => {
            if focus == Ctl::List {
                with_win(hwnd, |w| {
                    let c = w.model.cursor.unwrap_or(0);
                    if let Some(r) = w.model.rows().get(c) {
                        let i = r.start;
                        w.model.cursor = Some(c);
                        w.model.toggle(i);
                    }
                });
            } else {
                activate(hwnd, focus);
                return true;
            }
        }
        v if v == VK_TAB.0 && !ctrl => {
            let order = focusables(body, confirm, rows > 0);
            if !order.is_empty() {
                let i = order.iter().position(|&c| c == focus);
                let n = order.len();
                let next = match (i, shift) {
                    (Some(i), false) => order[(i + 1) % n],
                    (Some(i), true) => order[(i + n - 1) % n],
                    (None, false) => order[0],
                    (None, true) => order[n - 1],
                };
                with_win(hwnd, |w| {
                    w.focus = next;
                    if next == Ctl::List && w.model.cursor.is_none() {
                        w.model.cursor = Some(0);
                    }
                });
            }
        }
        v if (v == VK_UP.0 || v == VK_DOWN.0 || v == VK_HOME.0 || v == VK_END.0)
            && rows > 0
            && confirm == Confirm::None
            && body != Body::Scanning =>
        {
            with_win(hwnd, |w| {
                let last = rows - 1;
                let c = w.model.cursor;
                let next = match v {
                    x if x == VK_UP.0 => c.map_or(0, |c| c.saturating_sub(1)),
                    x if x == VK_DOWN.0 => c.map_or(0, |c| (c + 1).min(last)),
                    x if x == VK_HOME.0 => 0,
                    _ => last,
                };
                w.model.cursor = Some(next);
                w.focus = Ctl::List;
                scroll_to_row(w, next);
            });
        }
        v if v == u16::from(b'A') && ctrl && body != Body::Scanning => {
            with_win(hwnd, |w| {
                w.model.select_all();
                fix_focus(w);
            });
            relayout(hwnd);
        }
        v if v == VK_DELETE.0 && focus == Ctl::List => {
            with_win(hwnd, |w| {
                if let Some(r) = w.model.cursor.and_then(|c| w.model.rows().get(c).cloned()) {
                    w.model.remove(r.start);
                }
            });
            relayout(hwnd);
        }
        _ => return false,
    }
    invalidate(hwnd);
    true
}

/// Cuộn danh sách cho mục `row` hiện đủ.
fn scroll_to_row(w: &mut Win, row: usize) {
    let rh = scale(ROW_H, w.dpi);
    let top = row as i32 * rh;
    let view = w.layout.list.h;
    if top < w.list_scroll {
        w.list_scroll = top;
    } else if top + rh > w.list_scroll + view {
        w.list_scroll = top + rh - view;
    }
    clamp_scroll(w);
}

fn clamp_scroll(w: &mut Win) {
    let content = w.model.row_count() as i32 * scale(ROW_H, w.dpi);
    w.list_scroll = w.list_scroll.clamp(0, (content - w.layout.list.h).max(0));
}

/// Tiêu điểm hợp lệ cho trạng thái hiện tại (mặc định khi điều khiển cũ không còn).
fn fix_focus(w: &mut Win) {
    let body = body_of(w.model.phase());
    let confirm = w.model.confirm();
    if !focusables(body, confirm, w.model.row_count() > 0).contains(&w.focus) {
        w.focus = default_focus(body, confirm);
    }
}

/// Bấm điều khiển `ctl` (chuột, Enter hay Space).
fn activate(hwnd: HWND, ctl: Ctl) {
    let Some(out) = with_win(hwnd, |w| {
        let shown_before = w.model.confirm() != Confirm::None;
        if ctl == Ctl::Yes && !yes_allowed(w.confirm_at, tick_ms(), double_click_ms()) {
            // Lần bấm thứ hai của một cú bấm đúp vào nút chính: bỏ qua.
            return None;
        }
        let out = match ctl {
            Ctl::Cancel => Some(ReviewOutcome::Cancel),
            Ctl::Retry => w.model.retry(),
            Ctl::Add => {
                let on = !w.model.adding();
                w.model.set_adding(on);
                None
            }
            Ctl::Primary => w.model.primary(),
            Ctl::Back => {
                w.model.confirm_back();
                None
            }
            Ctl::Yes => w.model.confirm_yes(),
            Ctl::SelectAll => {
                w.model.select_all();
                None
            }
            Ctl::DeselectAll => {
                w.model.deselect_all();
                None
            }
            Ctl::List => None,
        };
        let body = body_of(w.model.phase());
        let confirm = w.model.confirm();
        let rows = w.model.row_count() > 0;
        if !focusables(body, confirm, rows).contains(&w.focus) || matches!(ctl, Ctl::Back) {
            w.focus = default_focus(body, confirm);
        }
        if confirm != Confirm::None {
            w.focus = Ctl::Back;
            if !shown_before {
                w.confirm_at = tick_ms();
            }
        }
        out
    }) else {
        return;
    };
    match out {
        Some(ReviewOutcome::Retry) => {
            let target = with_win(hwnd, |w| (w.notify, w.msg, w.token));
            if let Some((n, m, tok)) = target {
                post(n, m, tok, ReviewOutcome::Retry);
            }
            relayout(hwnd);
        }
        Some(o) => finish(hwnd, o),
        None => relayout(hwnd),
    }
}

fn release_capture(hwnd: HWND) {
    let had = with_win(hwnd, |w| {
        std::mem::replace(&mut w.press, Press::None) != Press::None
    });
    if had == Some(true) {
        // SAFETY: trả chuột mà cửa sổ này đã giữ; WM_CAPTURECHANGED thấy `press` đã rỗng.
        unsafe {
            let _ = ReleaseCapture();
        }
    }
}

// ---- Bố cục theo cửa sổ thật

/// Tính lại bố cục, bản hiển thị của ảnh và vẽ lại.
fn relayout(hwnd: HWND) {
    let Some((dpi, font, bold, body, confirm, img)) = with_win(hwnd, |w| {
        (
            w.dpi,
            w.font,
            w.bold,
            body_of(w.model.phase()),
            w.model.confirm(),
            w.model.image_size(),
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
    let (lay, conf_w, remove_w) = history_paint::with_screen_dc(|dc| {
        let widths = |c: Ctl| {
            let f = if kind_of(c, body) == ButtonKind::Normal {
                font
            } else {
                bold.or(font)
            };
            history_paint::measure(dc, f, label(c, body, confirm, lang)).0
        };
        let lay = layout(rc.right, rc.bottom, dpi, body, confirm, img, &widths);
        let conf = [Confidence::High, Confidence::Medium]
            .iter()
            .map(|c| history_paint::measure(dc, font, c.label(lang)).0)
            .max()
            .unwrap_or(0);
        let remove = history_paint::measure(dc, font, t(lang, Msg::PrivacyRemove)).0
            + 2 * scale(REMOVE_PAD, dpi);
        (lay, conf + scale(GAP, dpi), remove)
    });
    with_win(hwnd, |w| {
        let (dw, dh) = (lay.image.w, lay.image.h);
        let base_px = w.base.as_ref().map(|b| (b.w, b.h));
        let ok = match (&w.disp, base_px) {
            (Some(d), _) => (d.w, d.h) == (dw, dh),
            (None, Some(b)) => b == (dw, dh),
            (None, None) => true,
        };
        if !ok {
            // Bỏ bản cũ trước khi tạo bản mới: chỉ một bản đã co sống cùng bản giữ.
            w.disp = None;
            if base_px != Some((dw, dh)) {
                w.disp = w.base.as_ref().and_then(|b| ocr_view::scaled(b, dw, dh));
            }
        }
        w.layout = lay;
        w.conf_w = conf_w;
        w.remove_w = remove_w;
        clamp_scroll(w);
    });
    invalidate(hwnd);
}

/// Khung mục danh sách `row` (client px), chưa cắt theo khung danh sách.
fn row_rect(w: &Win, row: usize) -> Rect {
    let rh = scale(ROW_H, w.dpi);
    let l = w.layout.list;
    Rect {
        x: l.x,
        y: l.y + row as i32 * rh - w.list_scroll,
        w: l.w - scale(SCROLL_W + 2, w.dpi),
        h: rh,
    }
}

fn remove_rect(w: &Win, row: usize) -> Rect {
    let r = row_rect(w, row);
    let h = scale(REMOVE_H, w.dpi).min(r.h);
    Rect {
        x: r.right() - w.remove_w,
        y: r.y + (r.h - h) / 2,
        w: w.remove_w,
        h,
    }
}

fn is_manual_row(w: &Win, row: usize) -> bool {
    w.model
        .rows()
        .get(row)
        .is_some_and(|r| w.model.items()[r.start].source == RedactionSource::Manual)
}

/// Thứ dưới chuột tại `(x, y)` client.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Spot {
    Nothing,
    Button(Ctl),
    Row(usize),
    Remove(usize),
    /// Trên ảnh: px ảnh gốc và khung trúng (nếu có).
    Image((i32, i32), Option<usize>),
}

fn spot(w: &Win, x: i32, y: i32) -> Spot {
    if let Some((c, _)) = w.layout.buttons.iter().find(|(_, r)| inside(*r, x, y)) {
        return Spot::Button(*c);
    }
    let editable = body_of(w.model.phase()) != Body::Scanning && w.model.confirm() == Confirm::None;
    if inside(w.layout.list, x, y) && editable {
        let rh = scale(ROW_H, w.dpi).max(1);
        let row = ((y - w.layout.list.y + w.list_scroll) / rh) as usize;
        if row < w.model.row_count() {
            if is_manual_row(w, row) && inside(remove_rect(w, row), x, y) {
                return Spot::Remove(row);
            }
            return Spot::Row(row);
        }
        return Spot::Nothing;
    }
    if inside(w.layout.image, x, y) {
        let p = to_image(x, y, w.model.image_size(), w.layout.image);
        let hit = if editable {
            w.model.hit(p.0, p.1)
        } else {
            None
        };
        return Spot::Image(p, hit);
    }
    Spot::Nothing
}

fn point_of(lp: LPARAM) -> (i32, i32) {
    let v = lp.0 as u32;
    (
        i32::from(v as u16 as i16),
        i32::from((v >> 16) as u16 as i16),
    )
}

fn on_mouse_move(hwnd: HWND, x: i32, y: i32) {
    let track = with_win(hwnd, |w| {
        if w.press == Press::Drag {
            let p = to_image(x, y, w.model.image_size(), w.layout.image);
            w.model.drag_move(p.0, p.1);
        }
        let s = spot(w, x, y);
        w.hot = match s {
            Spot::Button(c) => Some(c),
            _ => None,
        };
        w.hot_remove = match s {
            Spot::Remove(r) => Some(r),
            _ => None,
        };
        w.model.hover = match s {
            Spot::Row(r) | Spot::Remove(r) => Some(r),
            Spot::Image(_, Some(i)) if !w.model.adding() => w.model.row_of(i),
            _ => None,
        };
        !std::mem::replace(&mut w.tracking, true)
    });
    if track == Some(true) {
        let mut tme = TRACKMOUSEEVENT {
            cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
            dwFlags: TME_LEAVE,
            hwndTrack: hwnd,
            dwHoverTime: 0,
        };
        // SAFETY: tme là biến cục bộ có cbSize đúng; theo dõi chuột rời cửa sổ này.
        unsafe {
            let _ = TrackMouseEvent(&mut tme);
        }
    }
    invalidate(hwnd);
}

fn on_button_down(hwnd: HWND, x: i32, y: i32) {
    let capture = with_win(hwnd, |w| {
        let s = spot(w, x, y);
        match s {
            Spot::Button(c) => {
                w.press = Press::Button(c);
                w.focus = c;
            }
            Spot::Remove(r) => w.press = Press::Remove(r),
            Spot::Row(r) => {
                w.model.cursor = Some(r);
                w.focus = Ctl::List;
                if let Some(range) = w.model.rows().get(r).cloned() {
                    w.model.toggle(range.start);
                }
            }
            Spot::Image(p, hit) => {
                if w.model.adding() {
                    if w.model.drag_begin(p.0, p.1) {
                        w.press = Press::Drag;
                    }
                } else if let Some(i) = hit {
                    w.model.toggle(i);
                    w.model.cursor = w.model.row_of(i);
                }
            }
            Spot::Nothing => {}
        }
        w.press != Press::None
    });
    if capture == Some(true) {
        // SAFETY: giữ chuột cho cửa sổ này tới WM_LBUTTONUP.
        unsafe {
            SetCapture(hwnd);
        }
    }
    invalidate(hwnd);
}

fn on_button_up(hwnd: HWND, x: i32, y: i32) {
    let Some((press, s)) = with_win(hwnd, |w| {
        let p = std::mem::replace(&mut w.press, Press::None);
        if p == Press::Drag {
            let q = to_image(x, y, w.model.image_size(), w.layout.image);
            w.model.drag_move(q.0, q.1);
        }
        (p, spot(w, x, y))
    }) else {
        return;
    };
    if press != Press::None {
        // SAFETY: trả chuột cửa sổ này đang giữ; `press` đã rỗng nên WM_CAPTURECHANGED không làm gì.
        unsafe {
            let _ = ReleaseCapture();
        }
    }
    match (press, s) {
        (Press::Button(Ctl::Back), Spot::Button(Ctl::Back))
            if with_win(hwnd, |w| {
                !yes_allowed(w.confirm_at, tick_ms(), double_click_ms())
            }) == Some(true) =>
        {
            // Lần bấm thứ hai của cú bấm đúp vào nút chính: giữ thanh xác nhận.
            invalidate(hwnd);
        }
        (Press::Button(c), Spot::Button(d)) if c == d => activate(hwnd, c),
        (Press::Remove(r), Spot::Remove(q)) if r == q => {
            with_win(hwnd, |w| {
                if let Some(range) = w.model.rows().get(r).cloned() {
                    w.model.remove(range.start);
                }
            });
            relayout(hwnd);
        }
        (Press::Drag, _) => {
            with_win(hwnd, |w| {
                if w.model.drag_end() {
                    let n = w.model.row_count();
                    w.model.cursor = n.checked_sub(1);
                    if let Some(c) = w.model.cursor {
                        scroll_to_row(w, c);
                    }
                }
            });
            relayout(hwnd);
        }
        _ => invalidate(hwnd),
    }
}

fn on_wheel(hwnd: HWND, wp: WPARAM, lp: LPARAM) {
    let delta = i32::from((wp.0 >> 16) as u16 as i16);
    let (sx, sy) = point_of(lp);
    let mut pt = POINT { x: sx, y: sy };
    // SAFETY: pt là biến cục bộ; đổi tọa độ màn hình sang client của cửa sổ này.
    unsafe {
        let _ = ScreenToClient(hwnd, &mut pt);
    }
    with_win(hwnd, |w| {
        if inside(w.layout.list, pt.x, pt.y) || inside(w.layout.right, pt.x, pt.y) {
            w.list_scroll -= delta * scale(WHEEL_DIP, w.dpi) / 120;
            clamp_scroll(w);
        }
    });
    invalidate(hwnd);
}

fn on_set_cursor(hwnd: HWND) -> bool {
    let mut pt = POINT::default();
    // SAFETY: pt là biến cục bộ; tọa độ con trỏ đổi sang client của cửa sổ này.
    unsafe {
        let _ = GetCursorPos(&mut pt);
        let _ = ScreenToClient(hwnd, &mut pt);
    }
    let Some(kind) = with_win(hwnd, |w| match spot(w, pt.x, pt.y) {
        Spot::Image(..) if w.model.adding() => Some(IDC_CROSS),
        Spot::Image(_, Some(_)) | Spot::Button(_) | Spot::Remove(_) | Spot::Row(_) => {
            Some(IDC_HAND)
        }
        _ => None,
    })
    .flatten() else {
        return false;
    };
    // SAFETY: con trỏ hệ thống dùng chung, không phải hủy.
    unsafe {
        if let Ok(c) = LoadCursorW(None, kind) {
            SetCursor(Some(c));
        }
    }
    true
}

fn on_dpi_changed(hwnd: HWND, dpi: u32, suggested: Option<RECT>) {
    let font = winutil::system_font(dpi);
    let bold = winutil::system_font_styled(dpi, 100, true);
    let old = with_win(hwnd, |w| {
        w.dpi = dpi;
        [
            std::mem::replace(&mut w.font, font),
            std::mem::replace(&mut w.bold, bold),
        ]
    });
    let stale = old.unwrap_or([font, bold]);
    for f in stale.into_iter().flatten() {
        // SAFETY: phông cũ (hoặc phông mới khi cửa sổ không còn trong sổ) không còn ai dùng.
        unsafe {
            let _ = DeleteObject(HGDIOBJ(f.0));
        }
    }
    if let Some(r) = suggested {
        // SAFETY: đổi vị trí và cỡ chính cửa sổ này theo RECT đề nghị (WM_SIZE tính lại bố cục).
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
}

// ---- Vẽ

fn rc(r: Rect) -> RECT {
    RECT {
        left: r.x,
        top: r.y,
        right: r.right(),
        bottom: r.bottom(),
    }
}

fn text(dc: HDC, s: &str, r: Rect, c: Rgb, font: Option<HFONT>, flags: DRAW_TEXT_FORMAT) {
    theme::draw_text(
        dc,
        &history_paint::wide(s),
        &rc(r),
        c,
        font,
        flags | DT_NOPREFIX,
    );
}

const ONE_LINE: DRAW_TEXT_FORMAT =
    DRAW_TEXT_FORMAT(DT_LEFT.0 | DT_VCENTER.0 | DT_SINGLELINE.0 | DT_END_ELLIPSIS.0);

/// Viền đứt nét trắng/đen (thấy được trên nền sáng lẫn tối) quanh `r`.
fn dashed(dc: HDC, r: &RECT) {
    if r.right - r.left < 2 || r.bottom - r.top < 2 {
        return;
    }
    // SAFETY: bút tạo ở đây được bỏ chọn rồi xóa; chổi rỗng là đối tượng kho, không xóa; trạng
    // thái DC được trả bằng RestoreDC.
    unsafe {
        let pen = CreatePen(PS_DOT, 1, theme::colorref(WHITE));
        if pen.is_invalid() {
            return;
        }
        let saved = SaveDC(dc);
        SelectObject(dc, HGDIOBJ(pen.0));
        SelectObject(dc, GetStockObject(NULL_BRUSH));
        SetBkMode(dc, OPAQUE);
        SetBkColor(dc, theme::colorref(BLACK));
        let _ = Rectangle(dc, r.left, r.top, r.right, r.bottom);
        let _ = RestoreDC(dc, saved);
        let _ = DeleteObject(HGDIOBJ(pen.0));
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

fn paint(hwnd: HWND) {
    let mut ps = PAINTSTRUCT::default();
    // SAFETY: cặp BeginPaint/EndPaint của chính cửa sổ này.
    let dc = unsafe { BeginPaint(hwnd, &mut ps) };
    let mut client = RECT::default();
    // SAFETY: client là biến cục bộ.
    unsafe {
        let _ = GetClientRect(hwnd, &mut client);
    }
    let focused = {
        // SAFETY: chỉ hỏi cửa sổ có tiêu điểm của luồng này.
        unsafe { windows::Win32::UI::Input::KeyboardAndMouse::GetFocus() == hwnd }
    };
    let lang = lang::current();
    theme::buffered(dc, &client, |m| {
        WINS.with(|v| {
            if let Ok(v) = v.try_borrow() {
                if let Some(w) = v.iter().find(|w| w.hwnd == hwnd) {
                    draw(m, &client, w, lang, focused);
                }
            }
        });
    });
    // SAFETY: kết thúc lần vẽ đã bắt đầu ở trên.
    unsafe {
        let _ = EndPaint(hwnd, &ps);
    }
}

fn draw(dc: HDC, client: &RECT, w: &Win, lang: Lang, focused: bool) {
    let s = |v: i32| scale(v, w.dpi);
    let l = &w.layout;
    let body = body_of(w.model.phase());
    let confirm = w.model.confirm();
    theme::fill(dc, client, BG);
    // Khung ảnh.
    let pane = rc(l.preview);
    theme::fill(dc, &pane, INPUT);
    let border = if w.model.adding() { ACCENT } else { LINE };
    ocr_view::frame(dc, &pane, border, s(1).max(1));
    let img = rc(l.image);
    match (&w.disp, &w.base) {
        (Some(d), _) | (None, Some(d)) => history_paint::blit(dc, d, &img),
        (None, None) => theme::fill(dc, &img, SURFACE),
    }
    draw_regions(dc, w);
    // Khung phải.
    match w.model.phase() {
        ReviewPhase::Scanning { tile } => {
            text(
                dc,
                t(lang, Msg::PrivacyScanning),
                l.summary,
                TEXT,
                w.bold,
                ONE_LINE,
            );
            if let Some((k, n)) = tile {
                text(
                    dc,
                    &i18n::privacy_scan_tile(lang, k, n),
                    l.detail,
                    TEXT_MUTED,
                    w.font,
                    ONE_LINE,
                );
            }
        }
        ReviewPhase::Findings => {
            // Số phát hiện (không tính vùng tự thêm).
            let found = w
                .model
                .rows()
                .iter()
                .filter(|r| w.model.items()[r.start].source != RedactionSource::Manual)
                .count();
            let summary = i18n::privacy_found(lang, found, t(lang, w.action));
            text(dc, &summary, l.summary, TEXT, w.bold, ONE_LINE);
        }
        ReviewPhase::Error(e) => {
            text(
                dc,
                t(lang, Msg::PrivacyErrHeading),
                l.summary,
                WARNING,
                w.bold,
                ONE_LINE,
            );
            let detail = format!("{}\n{}", e.message(lang), t(lang, Msg::PrivacyErrUnchecked));
            text(dc, &detail, l.detail, TEXT, w.font, DT_LEFT | DT_WORDBREAK);
        }
    }
    if body != Body::Scanning {
        draw_list(dc, w, lang, focused);
        let note = if w.model.adding() {
            t(lang, Msg::PrivacyDragHint)
        } else {
            t(lang, Msg::PrivacyNote)
        };
        let c = if w.model.adding() { ACCENT } else { TEXT_MUTED };
        text(dc, note, l.note, c, w.font, DT_LEFT | DT_WORDBREAK);
    }
    // Thanh xác nhận.
    if confirm != Confirm::None {
        let r = rc(l.confirm);
        theme::fill(dc, &r, SURFACE);
        let bar = RECT {
            right: r.left + s(3),
            ..r
        };
        theme::fill(dc, &bar, WARNING);
        let tr = Rect {
            x: l.confirm.x + s(3 + GAP + 4),
            w: l.confirm.w - s(3 + GAP + 4) - s(GAP),
            ..l.confirm
        };
        text(dc, confirm_text(confirm, lang), tr, TEXT, w.bold, ONE_LINE);
    }
    // Đường kẻ và hàng nút.
    let line = RECT {
        left: 0,
        top: l.footer_y,
        right: client.right,
        bottom: l.footer_y + s(1).max(1),
    };
    theme::fill(dc, &line, LINE);
    for &(c, r) in &l.buttons {
        let kind = kind_of(c, body);
        let state = ControlState {
            hot: w.hot == Some(c),
            pressed: w.press == Press::Button(c) && w.hot == Some(c)
                || (c == Ctl::Add && w.model.adding()),
            focused: focused && w.focus == c,
            enabled: true,
            checked: false,
        };
        let f = if kind == ButtonKind::Normal {
            w.font
        } else {
            w.bold.or(w.font)
        };
        let label = history_paint::wide(label(c, body, confirm, lang));
        theme::draw_button(dc, &rc(r), &label, kind, state, w.dpi, f, (true, false));
        if c == Ctl::Add && w.model.adding() {
            ocr_view::frame(dc, &rc(r), ACCENT, s(1).max(1));
        }
    }
}

/// Vùng che trên ảnh: chọn = ô đen đặc, bỏ chọn = viền đứt nét; mục trỏ/chọn = viền màu nhấn.
fn draw_regions(dc: HDC, w: &Win) {
    let img = w.model.image_size();
    let d = w.layout.image;
    if d.w <= 0 || d.h <= 0 {
        return;
    }
    // SAFETY: cắt nét vẽ trong ảnh hiển thị; trạng thái DC trả bằng RestoreDC.
    let saved = unsafe {
        let saved = SaveDC(dc);
        IntersectClipRect(dc, d.x, d.y, d.right(), d.bottom());
        saved
    };
    for it in w.model.items() {
        let r = to_display(it.rect, img, d);
        if it.selected {
            theme::fill(dc, &r, BLACK);
        } else {
            dashed(dc, &r);
        }
    }
    let rows = w.model.rows();
    let mut marks: Vec<usize> = Vec::new();
    marks.extend(w.model.hover);
    if w.focus == Ctl::List {
        marks.extend(w.model.cursor);
    }
    for row in marks {
        if let Some(range) = rows.get(row) {
            for it in &w.model.items()[range.clone()] {
                let r = inflate(&to_display(it.rect, img, d), scale(2, w.dpi));
                ocr_view::frame(dc, &r, ACCENT, scale(2, w.dpi).max(1));
            }
        }
    }
    if let Some(drag) = w.model.drag() {
        let r = to_display(drag.rect(), img, d);
        ocr_view::frame(dc, &r, ACCENT, 1);
        dashed(dc, &inflate(&r, -1));
    }
    // SAFETY: trả trạng thái DC đã lưu ở trên.
    unsafe {
        let _ = RestoreDC(dc, saved);
    }
}

fn row_name(w: &Win, row: usize, lang: Lang) -> (&'static str, Option<&'static str>) {
    let rows = w.model.rows();
    let Some(r) = rows.get(row) else {
        return ("", None);
    };
    match w.model.items()[r.start].source {
        RedactionSource::Manual => (t(lang, Msg::PrivacyManual), None),
        RedactionSource::Detected { finding, category } => {
            let conf = w.model.confidence(finding).map(|c| c.label(lang));
            (category.label(lang), conf)
        }
    }
}

fn draw_list(dc: HDC, w: &Win, lang: Lang, focused: bool) {
    let l = w.layout.list;
    if l.w <= 0 || l.h <= 0 {
        return;
    }
    let s = |v: i32| scale(v, w.dpi);
    // SAFETY: cắt nét vẽ trong khung danh sách; trạng thái DC trả bằng RestoreDC.
    let saved = unsafe {
        let saved = SaveDC(dc);
        IntersectClipRect(dc, l.x, l.y, l.right(), l.bottom());
        saved
    };
    let rows = w.model.rows();
    for (i, range) in rows.iter().enumerate() {
        let r = row_rect(w, i);
        if r.bottom() <= l.y || r.y >= l.bottom() {
            continue;
        }
        if w.model.hover == Some(i) {
            theme::fill(dc, &rc(r), TAB_HOVER);
        }
        let item = w.model.items()[range.start];
        let (name, conf) = row_name(w, i, lang);
        let manual = item.source == RedactionSource::Manual;
        let side = if manual { w.remove_w } else { w.conf_w };
        let check = Rect {
            x: r.x + s(4),
            w: (r.w - side - s(8)).max(0),
            ..r
        };
        let state = ControlState {
            hot: w.model.hover == Some(i),
            pressed: false,
            focused: false,
            enabled: true,
            checked: item.selected,
        };
        theme::draw_check(
            dc,
            &rc(check),
            &history_paint::wide(name),
            false,
            state,
            w.dpi,
            w.font,
            (false, false),
        );
        if manual {
            let rr = remove_rect(w, i);
            let st = ControlState {
                hot: w.hot_remove == Some(i),
                pressed: w.press == Press::Remove(i),
                focused: false,
                enabled: true,
                checked: false,
            };
            let label = history_paint::wide(t(lang, Msg::PrivacyRemove));
            theme::draw_button(
                dc,
                &rc(rr),
                &label,
                ButtonKind::Normal,
                st,
                w.dpi,
                w.font,
                (false, false),
            );
        } else if let Some(c) = conf {
            let cr = Rect {
                x: r.right() - w.conf_w,
                w: w.conf_w - s(4),
                ..r
            };
            text(
                dc,
                c,
                cr,
                TEXT_MUTED,
                w.font,
                DRAW_TEXT_FORMAT(DT_RIGHT.0 | DT_VCENTER.0 | DT_SINGLELINE.0),
            );
        }
        if focused && w.focus == Ctl::List && w.model.cursor == Some(i) {
            theme::focus_frame(dc, &rc(r), w.dpi);
        }
    }
    // Thanh cuộn mảnh khi danh sách dài hơn khung.
    let content = rows.len() as i32 * s(ROW_H);
    if content > l.h && content > 0 {
        let th = (i64::from(l.h) * i64::from(l.h) / i64::from(content)) as i32;
        let th = th.max(s(16)).min(l.h);
        let ty = l.y
            + (i64::from(w.list_scroll) * i64::from(l.h - th) / i64::from((content - l.h).max(1)))
                as i32;
        let bar = RECT {
            left: l.right() - s(SCROLL_W),
            top: ty,
            right: l.right(),
            bottom: ty + th,
        };
        theme::fill(dc, &bar, TRACK);
    }
    // SAFETY: trả trạng thái DC đã lưu ở trên.
    unsafe {
        let _ = RestoreDC(dc, saved);
    }
}

// ---- Tạo cửa sổ

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
    let mut r = RECT {
        left: 0,
        top: 0,
        right: scale(w, dpi),
        bottom: scale(h, dpi),
    };
    // SAFETY: r là biến cục bộ.
    unsafe {
        let _ = AdjustWindowRectExForDpi(&mut r, STYLE, false, EX_STYLE, dpi);
    }
    (r.right - r.left, r.bottom - r.top)
}

/// Cỡ mặc định kẹp vào vùng làm việc (không nhỏ hơn tối thiểu trừ khi vùng làm việc nhỏ hơn).
fn fit_window_size(want: (i32, i32), min: (i32, i32), work: (i32, i32)) -> (i32, i32) {
    let axis = |want: i32, min: i32, work: i32| {
        if work <= min {
            work.max(0)
        } else {
            want.clamp(min, work)
        }
    };
    (axis(want.0, min.0, work.0), axis(want.1, min.1, work.1))
}

/// Tạo cửa sổ (chưa hiện) giữa vùng làm việc của màn hình có con trỏ.
fn create(lang: Lang, step: Option<(u32, u32)>) -> Option<HWND> {
    // SAFETY: tham số ra là biến cục bộ; MonitorFromPoint chỉ đọc.
    let mon = unsafe {
        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST)
    };
    // Tạo thẳng theo DPI của màn hình đó: không có WM_DPICHANGED lúc mở.
    let dpi = monitor_dpi(mon);
    let want = window_size(DEFAULT_W, DEFAULT_H, dpi);
    let work = work_area(mon);
    let (ww, wh) = match work {
        Some(a) => fit_window_size(want, window_size(MIN_W, MIN_H, dpi), (a.w, a.h)),
        None => want,
    };
    let (x, y) = match work {
        Some(a) => (a.x + (a.w - ww) / 2, a.y + (a.h - wh) / 2),
        None => (100, 100),
    };
    let title = HSTRING::from(i18n::privacy_review_title(lang, step));
    // SAFETY: đăng ký lớp và tạo cửa sổ ở luồng chính; đăng ký lại lần hai báo lỗi vô hại.
    unsafe {
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
        .ok()
    }
}

fn lo_word(v: usize) -> u32 {
    (v & 0xFFFF) as u32
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            paint(hwnd);
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
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
            // Tài liệu WM_DPICHANGED: dùng đúng RECT đề nghị trong lp.
            // SAFETY: lp trỏ tới RECT hợp lệ suốt lời gọi.
            let suggested = unsafe { (lp.0 as *const RECT).as_ref() }.copied();
            on_dpi_changed(hwnd, dpi, suggested);
            LRESULT(0)
        }
        WM_KEYDOWN => {
            // Vòng lặp không qua `pre_translate` (ví dụ hộp thoại khác đang bơm): xử lý ở đây.
            let vk = wp.0 as u16;
            if on_key(hwnd, vk, key_down(VK_CONTROL), key_down(VK_SHIFT)) {
                LRESULT(0)
            } else {
                // SAFETY: xử lý mặc định cho cửa sổ này.
                unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
            }
        }
        WM_MOUSEMOVE => {
            let (x, y) = point_of(lp);
            on_mouse_move(hwnd, x, y);
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            with_win(hwnd, |w| {
                w.tracking = false;
                w.hot = None;
                w.hot_remove = None;
                w.model.hover = None;
            });
            invalidate(hwnd);
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let (x, y) = point_of(lp);
            on_button_down(hwnd, x, y);
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let (x, y) = point_of(lp);
            on_button_up(hwnd, x, y);
            LRESULT(0)
        }
        WM_CAPTURECHANGED => {
            // Mất chuột giữa chừng (không phải do ReleaseCapture của chính cửa sổ): bỏ thao tác dở.
            with_win(hwnd, |w| {
                if w.press == Press::Drag {
                    w.model.cancel_drag();
                }
                w.press = Press::None;
            });
            invalidate(hwnd);
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            on_wheel(hwnd, wp, lp);
            LRESULT(0)
        }
        WM_SETCURSOR if lo_word(lp.0 as usize) == HTCLIENT => {
            if on_set_cursor(hwnd) {
                LRESULT(1)
            } else {
                // SAFETY: xử lý mặc định cho cửa sổ này.
                unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
            }
        }
        WM_SETFOCUS | WM_KILLFOCUS => {
            invalidate(hwnd);
            LRESULT(0)
        }
        WM_CLOSE => {
            // ✕ / Alt+F4: Hủy (gửi ở WM_DESTROY nếu chưa gửi gì).
            destroy(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            let target = with_win(hwnd, |w| {
                if w.sent {
                    return None;
                }
                w.sent = true;
                Some((w.notify, w.msg, w.token))
            })
            .flatten();
            if let Some((n, m, tok)) = target {
                post(n, m, tok, ReviewOutcome::Cancel);
            }
            LRESULT(0)
        }
        WM_NCDESTROY => {
            let gone = WINS.with(|v| {
                let mut v = v.try_borrow_mut().ok()?;
                let i = v.iter().position(|w| w.hwnd == hwnd)?;
                Some(v.remove(i))
            });
            if let Some(w) = gone {
                release(w);
            }
            // SAFETY: xử lý mặc định cho cửa sổ này.
            unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
        }
        // SAFETY: xử lý mặc định cho cửa sổ này.
        _ => unsafe { DefWindowProcW(hwnd, msg, wp, lp) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkshot_core::privacy::model::{Category, Mapping, RuleId};
    use darkshot_core::privacy::RedactionPlan;
    use std::time::{Duration, Instant};
    use windows::Win32::System::Threading::{
        GetCurrentProcess, GetGuiResources, GR_GDIOBJECTS, GR_USEROBJECTS,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, SendMessageW, TranslateMessage, HWND_MESSAGE,
    };

    #[test]
    fn excluded_from_capture_only_outside_tests_on_2004_or_later() {
        assert!(exclude_from_capture(false, Some(19041)));
        assert!(exclude_from_capture(false, Some(26300)));
        assert!(!exclude_from_capture(false, Some(19040)));
        assert!(!exclude_from_capture(false, None));
        assert!(!exclude_from_capture(true, Some(26300)));
    }

    const LANGS: [Lang; 2] = [Lang::Vi, Lang::En];
    const DPIS: [u32; 4] = [96, 120, 144, 192];
    const ALL_CTL: [Ctl; 9] = [
        Ctl::SelectAll,
        Ctl::DeselectAll,
        Ctl::List,
        Ctl::Cancel,
        Ctl::Retry,
        Ctl::Add,
        Ctl::Primary,
        Ctl::Back,
        Ctl::Yes,
    ];
    const STATES: [(Body, Confirm); 5] = [
        (Body::Scanning, Confirm::None),
        (Body::Findings, Confirm::None),
        (Body::Findings, Confirm::EmptySelection),
        (Body::Error, Confirm::None),
        (Body::Error, Confirm::Unscanned),
    ];

    fn r(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    fn overlaps(a: Rect, b: Rect) -> bool {
        a.w > 0
            && a.h > 0
            && b.w > 0
            && b.h > 0
            && a.x < b.right()
            && b.x < a.right()
            && a.y < b.bottom()
            && b.y < a.bottom()
    }

    fn within(inner: Rect, outer: Rect) -> bool {
        inner.x >= outer.x
            && inner.y >= outer.y
            && inner.right() <= outer.right()
            && inner.bottom() <= outer.bottom()
    }

    /// Bề rộng chữ bi quan: 8 DIP mỗi ký tự (phông 9 pt của Windows trung bình ~6–7 DIP).
    fn text_px(s: &str, dpi: u32) -> i32 {
        s.chars().count() as i32 * scale(8, dpi)
    }

    #[test]
    fn sizes_match_design() {
        assert_eq!((DEFAULT_W, DEFAULT_H), (960, 640));
        assert_eq!((MIN_W, MIN_H), (640, 420));
        assert_eq!(WM_PRIVACY_REVIEW, WM_APP + 20);
        assert_ne!(EX_STYLE.0 & WS_EX_TOPMOST.0, 0);
    }

    #[test]
    fn layout_has_no_overlaps_at_every_dpi_and_language() {
        for dpi in DPIS {
            for lang in LANGS {
                for (cw_dip, ch_dip) in [(MIN_W, MIN_H), (DEFAULT_W, DEFAULT_H), (1600, 1000)] {
                    let (cw, ch) = (scale(cw_dip, dpi), scale(ch_dip, dpi));
                    for (body, confirm) in STATES {
                        let widths = |c: Ctl| text_px(label(c, body, confirm, lang), dpi);
                        let l = layout(cw, ch, dpi, body, confirm, (1920, 1080), &widths);
                        let ctx = format!("dpi {dpi} {lang:?} {cw}x{ch} {body:?} {confirm:?}");
                        let client = r(0, 0, cw, ch);
                        let (left, right) = footer(body, confirm);
                        let want =
                            left.len() + right.len() + if body == Body::Findings { 2 } else { 0 };
                        assert_eq!(l.buttons.len(), want, "{ctx}");
                        for (i, &(c, b)) in l.buttons.iter().enumerate() {
                            assert!(within(b, client), "{ctx} {c:?} {b:?}");
                            assert!(b.w > 0 && b.h > 0, "{ctx} {c:?}");
                            // Chữ vừa nút (không phải co) với bề rộng bi quan.
                            assert!(
                                b.w >= widths(c) + 2 * scale(4, dpi),
                                "{ctx} {c:?} {} < {}",
                                b.w,
                                widths(c)
                            );
                            for &(d, o) in &l.buttons[i + 1..] {
                                assert!(!overlaps(b, o), "{ctx} {c:?} vs {d:?}");
                            }
                            for (name, part) in [
                                ("preview", l.preview),
                                ("list", l.list),
                                ("note", l.note),
                                ("summary", l.summary),
                                ("detail", l.detail),
                                ("confirm", l.confirm),
                            ] {
                                assert!(!overlaps(b, part), "{ctx} {c:?} vs {name}");
                            }
                        }
                        assert!(!overlaps(l.preview, l.right), "{ctx}");
                        assert!(
                            within(l.preview, client) && within(l.right, client),
                            "{ctx}"
                        );
                        assert!(within(l.image, l.preview), "{ctx}");
                        for part in [l.summary, l.detail, l.list, l.note] {
                            assert!(part.h == 0 || within(part, l.right), "{ctx} {part:?}");
                        }
                        assert!(!overlaps(l.list, l.note), "{ctx}");
                        assert!(!overlaps(l.detail, l.list), "{ctx}");
                        assert!(l.footer_y > l.preview.bottom(), "{ctx}");
                        if confirm != Confirm::None {
                            assert!(within(l.confirm, client), "{ctx}");
                            assert!(l.confirm.bottom() <= l.footer_y, "{ctx}");
                            assert!(l.preview.bottom() <= l.confirm.y, "{ctx}");
                        } else {
                            assert_eq!(l.confirm, ZERO, "{ctx}");
                        }
                        if body != Body::Scanning {
                            // Ít nhất hai mục danh sách thấy được ở cỡ tối thiểu.
                            assert!(l.list.h >= 2 * scale(ROW_H, dpi), "{ctx} list {:?}", l.list);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn confirm_bar_keeps_continue_away_from_primary() {
        for dpi in DPIS {
            for lang in LANGS {
                for (cw_dip, ch_dip) in [(MIN_W, MIN_H), (DEFAULT_W, DEFAULT_H)] {
                    let (cw, ch) = (scale(cw_dip, dpi), scale(ch_dip, dpi));
                    for (body, confirm) in [
                        (Body::Findings, Confirm::EmptySelection),
                        (Body::Error, Confirm::Unscanned),
                    ] {
                        let ctx = format!("dpi {dpi} {lang:?} {cw}x{ch} {body:?}");
                        let before = layout(cw, ch, dpi, body, Confirm::None, (100, 100), &|c| {
                            text_px(label(c, body, Confirm::None, lang), dpi)
                        });
                        let after = layout(cw, ch, dpi, body, confirm, (100, 100), &|c| {
                            text_px(label(c, body, confirm, lang), dpi)
                        });
                        let find = |l: &Layout, c: Ctl| {
                            l.buttons.iter().find(|b| b.0 == c).map(|b| b.1).unwrap()
                        };
                        let primary = find(&before, Ctl::Primary);
                        let yes = find(&after, Ctl::Yes);
                        let back = find(&after, Ctl::Back);
                        assert!(!overlaps(yes, primary), "{ctx} {yes:?} {primary:?}");
                        assert_eq!(back.right(), primary.right(), "{ctx}");
                        let (cx, cy) = (primary.x + primary.w / 2, primary.y + primary.h / 2);
                        assert!(inside(back, cx, cy), "{ctx}");
                    }
                }
            }
        }
    }

    #[test]
    fn continue_ignored_within_double_click_time() {
        assert!(!yes_allowed(1000, 1000, 500));
        assert!(!yes_allowed(1000, 1499, 500));
        assert!(yes_allowed(1000, 1500, 500));
        assert!(yes_allowed(1000, 9000, 500));
        // GetTickCount quay vòng sau ~49,7 ngày.
        assert!(!yes_allowed(u32::MAX - 100, 200, 500));
        assert!(yes_allowed(u32::MAX - 100, 400, 500));
    }

    #[test]
    fn layout_shrinks_buttons_instead_of_overlapping_when_tiny() {
        let widths = |_: Ctl| 400;
        let l = layout(300, 200, 96, Body::Error, Confirm::None, (10, 10), &widths);
        for (i, &(_, a)) in l.buttons.iter().enumerate() {
            assert!(within(a, r(0, 0, 300, 200)), "{a:?}");
            for &(_, b) in &l.buttons[i + 1..] {
                assert!(!overlaps(a, b));
            }
        }
    }

    #[test]
    fn fit_never_upscales_and_keeps_aspect() {
        let area = r(10, 20, 400, 300);
        assert_eq!(fit(100, 50, area), r(160, 145, 100, 50));
        let f = fit(1600, 900, area);
        assert_eq!((f.w, f.h), (400, 225));
        assert!(within(f, area));
        let f = fit(900, 1600, area);
        assert_eq!((f.w, f.h), (168, 300));
        assert!(within(f, area));
        let f = fit(10, 100_000, area);
        assert_eq!((f.w, f.h), (1, 300));
        assert_eq!(fit(0, 10, area).w, 0);
        assert_eq!(fit(10, 10, r(0, 0, 0, 0)).w, 0);
        assert_eq!(base_size(800, 600), (800, 600));
        assert_eq!(base_size(8000, 20000), (1638, 4096));
        assert_eq!(base_size(20000, 100), (4096, 20));
    }

    #[test]
    fn display_mapping_round_trips_and_is_never_empty() {
        let d = r(100, 50, 200, 100);
        let img = (400, 200);
        let m = to_display(r(0, 0, 400, 200), img, d);
        assert_eq!((m.left, m.top, m.right, m.bottom), (100, 50, 300, 150));
        let m = to_display(r(10, 10, 1, 1), img, d);
        assert_eq!((m.left, m.top, m.right, m.bottom), (105, 55, 106, 56));
        assert_eq!(to_image(100, 50, img, d), (0, 0));
        assert_eq!(to_image(299, 149, img, d), (398, 198));
        assert_eq!(to_image(90, 40, img, d), (-20, -20));
        assert_eq!(to_image(10, 10, img, r(0, 0, 0, 0)), (0, 0));
    }

    #[test]
    fn focus_order_and_defaults() {
        assert_eq!(
            focusables(Body::Findings, Confirm::None, true),
            vec![
                Ctl::SelectAll,
                Ctl::DeselectAll,
                Ctl::List,
                Ctl::Cancel,
                Ctl::Add,
                Ctl::Primary
            ]
        );
        assert_eq!(
            focusables(Body::Error, Confirm::None, false),
            vec![Ctl::Cancel, Ctl::Retry, Ctl::Add, Ctl::Primary]
        );
        assert_eq!(
            focusables(Body::Scanning, Confirm::None, true),
            vec![Ctl::Cancel]
        );
        assert_eq!(
            focusables(Body::Findings, Confirm::EmptySelection, true),
            vec![Ctl::Back, Ctl::Yes]
        );
        assert_eq!(default_focus(Body::Findings, Confirm::None), Ctl::Primary);
        assert_eq!(default_focus(Body::Error, Confirm::None), Ctl::Primary);
        assert_eq!(default_focus(Body::Scanning, Confirm::None), Ctl::Cancel);
        assert_eq!(default_focus(Body::Error, Confirm::Unscanned), Ctl::Back);
        // Nút đi tiếp mà không che/quét không bao giờ là nút chính (màu nhấn).
        assert_eq!(kind_of(Ctl::Yes, Body::Error), ButtonKind::Danger);
        assert_eq!(kind_of(Ctl::Primary, Body::Error), ButtonKind::Normal);
        assert_eq!(kind_of(Ctl::Primary, Body::Findings), ButtonKind::Primary);
    }

    #[test]
    fn labels_exist_in_both_languages() {
        for (body, confirm) in STATES {
            for c in ALL_CTL {
                if c == Ctl::List {
                    continue;
                }
                let vi = label(c, body, confirm, Lang::Vi);
                let en = label(c, body, confirm, Lang::En);
                assert!(!vi.trim().is_empty() && !en.trim().is_empty(), "{c:?}");
                assert_ne!(vi, en, "{c:?}");
            }
            if confirm != Confirm::None {
                assert_ne!(
                    confirm_text(confirm, Lang::Vi),
                    confirm_text(confirm, Lang::En)
                );
            }
        }
        assert_eq!(
            label(Ctl::Primary, Body::Findings, Confirm::None, Lang::Vi),
            "Che và tiếp tục"
        );
        assert_eq!(
            label(Ctl::Primary, Body::Error, Confirm::None, Lang::Vi),
            "Tiếp tục không quét"
        );
        assert_eq!(
            label(Ctl::Yes, Body::Findings, Confirm::EmptySelection, Lang::Vi),
            "Tiếp tục không che"
        );
        assert_eq!(
            label(Ctl::Yes, Body::Error, Confirm::Unscanned, Lang::Vi),
            "Vẫn tiếp tục"
        );
        assert_eq!(
            label(Ctl::Back, Body::Error, Confirm::Unscanned, Lang::En),
            "Go back"
        );
    }

    // ---- Chạy tay (mở cửa sổ thật, không giả lập chuột/phím: thông điệp gửi thẳng tới HWND):
    // cargo test -p darkshot-windows review_window::tests::manual -- --ignored --test-threads=1

    thread_local! {
        static OUTCOMES: RefCell<Vec<(usize, ReviewOutcome)>> = const { RefCell::new(Vec::new()) };
    }

    /// Cửa sổ chỉ nhận thông điệp, đóng vai cổng xuất.
    struct Gate(HWND);

    impl Gate {
        fn new() -> Gate {
            // SAFETY: cửa sổ chỉ nhận thông điệp của luồng kiểm thử; hủy ở Drop.
            let h = unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    w!("STATIC"),
                    None,
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
                .expect("message window")
            };
            OUTCOMES.with(|o| o.borrow_mut().clear());
            Gate(h)
        }
    }

    impl Drop for Gate {
        fn drop(&mut self) {
            drain_outcomes(self.0, WM_PRIVACY_REVIEW);
            // SAFETY: cửa sổ do `Gate::new` tạo.
            unsafe {
                let _ = DestroyWindow(self.0);
            }
        }
    }

    fn outcomes() -> Vec<(usize, ReviewOutcome)> {
        OUTCOMES.with(|o| o.borrow().clone())
    }

    fn pump(ms: u64, done: impl Fn() -> bool) -> bool {
        let deadline = Instant::now() + Duration::from_millis(ms);
        let mut m = MSG::default();
        loop {
            // SAFETY: vòng lặp thông điệp của luồng kiểm thử; mỗi kết quả lấy lại đúng một lần.
            unsafe {
                while PeekMessageW(&mut m, None, 0, 0, PM_REMOVE).as_bool() {
                    if m.message == WM_PRIVACY_REVIEW {
                        let out = take_outcome(m.lParam);
                        OUTCOMES.with(|o| o.borrow_mut().push((m.wParam.0, *out)));
                        continue;
                    }
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
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Ảnh tổng hợp (không chữ): nền sáng, vài khối màu.
    fn sample_image(w: u32, h: u32) -> Arc<Image> {
        let mut bgra = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                let px: [u8; 4] = match ((x / 80) + (y / 60)) % 3 {
                    0 => [240, 240, 240, 255],
                    1 => [190, 210, 230, 255],
                    _ => [235, 220, 200, 255],
                };
                bgra.extend_from_slice(&px);
            }
        }
        Arc::new(Image {
            width: w,
            height: h,
            bgra,
        })
    }

    fn finding(id: u32, category: Category, confidence: Confidence, rects: Vec<Rect>) -> Finding {
        Finding {
            id,
            category,
            confidence,
            rects,
            rule: RuleId::EmailPlain,
            mapping: Mapping::Word,
        }
    }

    fn findings() -> Vec<Finding> {
        vec![
            finding(
                0,
                Category::Email,
                Confidence::High,
                vec![r(80, 60, 360, 40)],
            ),
            finding(
                1,
                Category::ApiKey,
                Confidence::High,
                vec![r(80, 160, 520, 40)],
            ),
            finding(
                2,
                Category::Phone,
                Confidence::Medium,
                vec![r(80, 260, 280, 40)],
            ),
            finding(
                3,
                Category::PrivateKey,
                Confidence::High,
                vec![r(80, 360, 640, 36), r(80, 400, 640, 36)],
            ),
        ]
    }

    fn open_test(token: usize, gate: &Gate, phase: ReviewPhase, f: Vec<Finding>) -> HWND {
        let req = ReviewRequest {
            token,
            image: sample_image(960, 540),
            action: Msg::PrivacyActionSave,
            step: None,
            phase,
            findings: f,
        };
        open(req, gate.0, WM_PRIVACY_REVIEW).expect("open");
        pump(50, || false);
        hwnd_of(token).expect("registered")
    }

    fn alive(h: HWND) -> bool {
        // SAFETY: chỉ hỏi tính hợp lệ của handle.
        unsafe { IsWindow(Some(h)) }.as_bool()
    }

    fn lparam_xy(x: i32, y: i32) -> LPARAM {
        LPARAM((((y as u16 as u32) << 16) | (x as u16 as u32)) as isize)
    }

    fn post_mouse(h: HWND, msg: u32, x: i32, y: i32, wp: usize) {
        // SAFETY: gửi vào hàng đợi cửa sổ do luồng này tạo.
        unsafe {
            PostMessageW(Some(h), msg, WPARAM(wp), lparam_xy(x, y)).unwrap();
        }
    }

    fn click(h: HWND, x: i32, y: i32) {
        post_mouse(h, WM_LBUTTONDOWN, x, y, 1);
        post_mouse(h, WM_LBUTTONUP, x, y, 0);
        pump(40, || false);
    }

    fn key(h: HWND, vk: u16) {
        // SAFETY: gửi vào hàng đợi cửa sổ do luồng này tạo; đi qua `pre_translate`.
        unsafe {
            PostMessageW(Some(h), WM_KEYDOWN, WPARAM(usize::from(vk)), LPARAM(1)).unwrap();
        }
        pump(40, || false);
    }

    fn button(h: HWND, c: Ctl) -> Option<(i32, i32)> {
        with_win(h, |w| {
            w.layout
                .buttons
                .iter()
                .find(|b| b.0 == c)
                .map(|(_, b)| (b.x + b.w / 2, b.y + b.h / 2))
        })
        .flatten()
    }

    fn press(h: HWND, c: Ctl) {
        let (x, y) = button(h, c).unwrap_or_else(|| panic!("no button {c:?}"));
        click(h, x, y);
    }

    /// Tâm (client) của khung `item` trên ảnh xem trước.
    fn item_point(h: HWND, item: usize) -> (i32, i32) {
        with_win(h, |w| {
            let d = to_display(
                w.model.items()[item].rect,
                w.model.image_size(),
                w.layout.image,
            );
            ((d.left + d.right) / 2, (d.top + d.bottom) / 2)
        })
        .expect("win")
    }

    fn image_point(h: HWND, x: i32, y: i32) -> (i32, i32) {
        with_win(h, |w| {
            let d = to_display(r(x, y, 1, 1), w.model.image_size(), w.layout.image);
            (d.left, d.top)
        })
        .expect("win")
    }

    fn model_of(h: HWND) -> ReviewModel {
        with_win(h, |w| w.model.clone()).expect("win")
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

    /// Chờ qua thời gian bấm đúp (nút đi tiếp trên thanh xác nhận mới nhận bấm).
    fn wait_double_click() {
        pump(u64::from(double_click_ms()) + 60, || false);
    }

    /// Bấm đúp (hai cặp DOWN/UP cùng tọa độ, không có CS_DBLCLKS nên không có WM_LBUTTONDBLCLK).
    fn double_press(h: HWND, c: Ctl) {
        let (x, y) = button(h, c).unwrap_or_else(|| panic!("no button {c:?}"));
        post_mouse(h, WM_LBUTTONDOWN, x, y, 1);
        post_mouse(h, WM_LBUTTONUP, x, y, 0);
        post_mouse(h, WM_LBUTTONDOWN, x, y, 1);
        post_mouse(h, WM_LBUTTONUP, x, y, 0);
        pump(60, || false);
    }

    fn close_msg(h: HWND) {
        // SAFETY: gửi vào hàng đợi cửa sổ do luồng này tạo (như bấm ✕).
        unsafe {
            PostMessageW(Some(h), WM_CLOSE, WPARAM(0), LPARAM(0)).unwrap();
        }
        pump(200, || !alive(h));
    }

    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_open_close_20_times_no_gdi_user_leak() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        crate::webshot::per_monitor();
        close_all();
        let gate = Gate::new();
        // Khởi động: chổi dùng chung (`theme::brush`) và lớp cửa sổ được tạo trước khi đếm.
        for t in 0..2 {
            let h = open_test(900 + t, &gate, ReviewPhase::Findings, findings());
            let (x, y) = item_point(h, 0);
            post_mouse(h, WM_MOUSEMOVE, x, y, 0);
            pump(100, || false);
            close_msg(h);
        }
        pump(200, || false);
        let before = gui_counts();
        for t in 0..20usize {
            let phase = match t % 3 {
                0 => ReviewPhase::Findings,
                1 => ReviewPhase::Scanning { tile: Some((1, 2)) },
                _ => ReviewPhase::Error(ScanError::Failed),
            };
            let h = open_test(1000 + t, &gate, phase, findings());
            // Đổi cỡ để tạo lại bản hiển thị, rê chuột để vẽ khung trỏ.
            // SAFETY: đổi cỡ cửa sổ của luồng này.
            unsafe {
                let _ = SetWindowPos(
                    h,
                    None,
                    0,
                    0,
                    800 + t as i32 * 7,
                    560,
                    windows::Win32::UI::WindowsAndMessaging::SWP_NOMOVE
                        | SWP_NOZORDER
                        | SWP_NOACTIVATE,
                );
            }
            if t % 3 == 0 {
                let (x, y) = item_point(h, 0);
                post_mouse(h, WM_MOUSEMOVE, x, y, 0);
            }
            pump(40, || false);
            if t % 2 == 0 {
                close_msg(h);
            } else {
                close(1000 + t);
            }
            assert!(!alive(h));
        }
        pump(300, || false);
        assert!(WINS.with(|v| v.borrow().is_empty()));
        let after = gui_counts();
        assert!(
            after.0 <= before.0 && after.1 <= before.1,
            "GDI/USER grew: {before:?} -> {after:?}"
        );
        // Mười lần đóng bằng ✕: mười Cancel; mười lần cổng đóng: không gì.
        let got = outcomes();
        let tail: Vec<_> = got.iter().filter(|(t, _)| *t >= 1000).collect();
        assert_eq!(tail.len(), 10, "{got:?}");
        assert!(tail
            .iter()
            .all(|(t, o)| *o == ReviewOutcome::Cancel && t % 2 == 0));
    }

    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_outcome_sent_once_on_close() {
        close_all();
        let gate = Gate::new();
        let h = open_test(7, &gate, ReviewPhase::Findings, findings());
        close_msg(h);
        assert!(!alive(h));
        // `close` của cổng và `close_all` sau đó không gửi thêm.
        close(7);
        close_all();
        pump(100, || false);
        assert_eq!(outcomes(), vec![(7, ReviewOutcome::Cancel)]);
        assert!(!update(7, Ok(Vec::new())));

        // Cổng đóng: không gửi gì.
        let h = open_test(8, &gate, ReviewPhase::Findings, findings());
        close(8);
        pump(100, || false);
        assert!(!alive(h));
        assert_eq!(outcomes().len(), 1);
        // Thoát ứng dụng: không gửi gì.
        let h = open_test(9, &gate, ReviewPhase::Scanning { tile: None }, Vec::new());
        close_all();
        pump(100, || false);
        assert!(!alive(h));
        assert_eq!(outcomes().len(), 1);
        // Mở trùng token: lỗi, không có cửa sổ thứ hai.
        let h = open_test(10, &gate, ReviewPhase::Findings, findings());
        let dup = ReviewRequest {
            token: 10,
            image: sample_image(10, 10),
            action: Msg::PrivacyActionCopy,
            step: None,
            phase: ReviewPhase::Findings,
            findings: Vec::new(),
        };
        assert!(open(dup, gate.0, WM_PRIVACY_REVIEW).is_err());
        assert_eq!(WINS.with(|v| v.borrow().len()), 1);
        // Esc = Hủy, một lần.
        key(h, VK_ESCAPE.0);
        pump(100, || !alive(h));
        assert!(!alive(h));
        assert_eq!(
            outcomes(),
            vec![(7, ReviewOutcome::Cancel), (10, ReviewOutcome::Cancel)]
        );
    }

    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_enter_applies_all_selected() {
        close_all();
        let gate = Gate::new();
        let h = open_test(11, &gate, ReviewPhase::Findings, findings());
        key(h, VK_RETURN.0);
        pump(100, || !alive(h));
        assert!(!alive(h));
        let rects: Vec<Rect> = findings().into_iter().flat_map(|f| f.rects).collect();
        assert_eq!(
            outcomes(),
            vec![(11, ReviewOutcome::Apply(RedactionPlan { rects }))]
        );
    }

    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_clicks_toggle_and_add_area() {
        close_all();
        let gate = Gate::new();
        let h = open_test(12, &gate, ReviewPhase::Findings, findings());
        // Bấm vùng email trên ảnh: bỏ chọn; bấm lại: chọn.
        let (x, y) = item_point(h, 0);
        click(h, x, y);
        assert!(!model_of(h).items()[0].selected);
        click(h, x, y);
        assert!(model_of(h).items()[0].selected);
        // Bấm mục PEM trong danh sách: cả hai khung đổi.
        let (lx, ly) = with_win(h, |w| {
            let rr = row_rect(w, 3);
            (rr.x + rr.w / 2, rr.y + rr.h / 2)
        })
        .unwrap();
        click(h, lx, ly);
        let m = model_of(h);
        assert!(!m.items()[3].selected && !m.items()[4].selected);
        // Bỏ chọn hết / Chọn tất cả.
        press(h, Ctl::DeselectAll);
        assert!(!model_of(h).any_selected());
        press(h, Ctl::SelectAll);
        assert!(model_of(h).items().iter().all(|i| i.selected));
        // Thêm vùng che: kéo từ (700, 60) tới (900, 140) px ảnh.
        press(h, Ctl::Add);
        assert!(model_of(h).adding());
        let (ax, ay) = image_point(h, 700, 60);
        let (bx, by) = image_point(h, 900, 140);
        post_mouse(h, WM_LBUTTONDOWN, ax, ay, 1);
        post_mouse(h, WM_MOUSEMOVE, (ax + bx) / 2, (ay + by) / 2, 1);
        pump(30, || false);
        assert!(model_of(h).drag().is_some());
        post_mouse(h, WM_MOUSEMOVE, bx, by, 1);
        post_mouse(h, WM_LBUTTONUP, bx, by, 0);
        pump(40, || false);
        let m = model_of(h);
        assert_eq!(m.items().len(), 6);
        let last = m.items()[5];
        assert_eq!(last.source, RedactionSource::Manual);
        assert!(last.selected);
        let lr = last.rect;
        assert!((lr.x - 700).abs() <= 3 && (lr.y - 60).abs() <= 3, "{lr:?}");
        assert!(
            (lr.right() - 900).abs() <= 3 && (lr.bottom() - 140).abs() <= 3,
            "{lr:?}"
        );
        // Kéo quá nhỏ: bỏ.
        let (cx, cy) = image_point(h, 700, 300);
        post_mouse(h, WM_LBUTTONDOWN, cx, cy, 1);
        post_mouse(h, WM_LBUTTONUP, cx + 1, cy + 1, 0);
        pump(40, || false);
        assert_eq!(model_of(h).items().len(), 6);
        // Esc trong chế độ kéo: thoát chế độ, cửa sổ ở lại.
        key(h, VK_ESCAPE.0);
        assert!(!model_of(h).adding());
        assert!(alive(h));
        assert!(outcomes().is_empty());
        // Nút Xóa của vùng tự thêm (mục danh sách thứ 5).
        let (rx, ry) = with_win(h, |w| {
            let rr = remove_rect(w, 4);
            (rr.x + rr.w / 2, rr.y + rr.h / 2)
        })
        .unwrap();
        click(h, rx, ry);
        assert_eq!(model_of(h).items().len(), 5);
        // Bàn phím trong danh sách: Home ↓ ↓ ↓ ↓ ↑ → mục 2 (cuối là 3), Space bỏ chọn.
        key(h, VK_HOME.0);
        assert_eq!(model_of(h).cursor, Some(0));
        for _ in 0..4 {
            key(h, VK_DOWN.0);
        }
        assert_eq!(model_of(h).cursor, Some(3));
        key(h, VK_UP.0);
        assert_eq!(model_of(h).cursor, Some(2));
        key(h, VK_SPACE.0);
        assert!(!model_of(h).items()[2].selected);
        assert!(model_of(h).items()[1].selected);
        // Ctrl không giả lập được bằng thông điệp: gọi thẳng thao tác của Ctrl+A.
        assert!(on_key(h, u16::from(b'A'), true, false));
        assert!(model_of(h).items().iter().all(|i| i.selected));
        // Tab: đi hết vòng sáu điều khiển.
        let start = with_win(h, |w| w.focus).unwrap();
        for _ in 0..6 {
            key(h, VK_TAB.0);
        }
        assert_eq!(with_win(h, |w| w.focus).unwrap(), start);
        key(h, VK_ESCAPE.0);
        pump(100, || !alive(h));
        assert_eq!(outcomes(), vec![(12, ReviewOutcome::Cancel)]);
    }

    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_update_retry_and_confirmations() {
        close_all();
        let gate = Gate::new();
        let h = open_test(20, &gate, ReviewPhase::Scanning { tile: None }, Vec::new());
        // Đang quét: chỉ có Hủy.
        assert_eq!(with_win(h, |w| w.layout.buttons.len()), Some(1));
        progress(20, 2, 5);
        assert_eq!(
            model_of(h).phase(),
            ReviewPhase::Scanning { tile: Some((2, 5)) }
        );
        assert!(!update(99, Ok(Vec::new())), "unknown token");
        assert!(update(20, Err(ScanError::NoLanguage)));
        assert_eq!(
            model_of(h).phase(),
            ReviewPhase::Error(ScanError::NoLanguage)
        );
        assert!(button(h, Ctl::Retry).is_some());
        // Tiếp tục không quét: lần một chỉ hiện thanh xác nhận.
        press(h, Ctl::Primary);
        assert_eq!(model_of(h).confirm(), Confirm::Unscanned);
        assert!(outcomes().is_empty());
        // Enter trên thanh xác nhận = Quay lại (tiêu điểm mặc định).
        key(h, VK_RETURN.0);
        assert_eq!(model_of(h).confirm(), Confirm::None);
        assert!(outcomes().is_empty());
        // Thử lại: gửi Retry, ở lại, về Đang quét.
        press(h, Ctl::Retry);
        assert_eq!(outcomes(), vec![(20, ReviewOutcome::Retry)]);
        assert!(alive(h));
        assert_eq!(model_of(h).phase(), ReviewPhase::Scanning { tile: None });
        assert!(update(20, Err(ScanError::Timeout)));
        press(h, Ctl::Primary);
        wait_double_click();
        press(h, Ctl::Yes);
        pump(100, || !alive(h));
        assert!(!alive(h));
        assert_eq!(
            outcomes()[1],
            (
                20,
                ReviewOutcome::ContinueUnscanned(RedactionPlan::default())
            )
        );

        // Bỏ chọn hết rồi Che và tiếp tục: xác nhận, rồi Tiếp tục không che.
        let h = open_test(21, &gate, ReviewPhase::Scanning { tile: None }, Vec::new());
        assert!(update(21, Ok(findings())));
        assert_eq!(model_of(h).phase(), ReviewPhase::Findings);
        assert_eq!(model_of(h).items().len(), 5);
        press(h, Ctl::DeselectAll);
        // Tiêu điểm ở Bỏ chọn hết: Enter bấm lại nút đó; Tab tới nút chính rồi Enter.
        with_win(h, |w| w.focus = Ctl::Primary);
        key(h, VK_RETURN.0);
        assert_eq!(model_of(h).confirm(), Confirm::EmptySelection);
        assert!(alive(h));
        // Esc trên thanh xác nhận = Quay lại.
        key(h, VK_ESCAPE.0);
        assert_eq!(model_of(h).confirm(), Confirm::None);
        assert!(alive(h));
        press(h, Ctl::Primary);
        wait_double_click();
        press(h, Ctl::Yes);
        pump(100, || !alive(h));
        assert_eq!(outcomes()[2], (21, ReviewOutcome::ContinueNone));
        assert_eq!(outcomes().len(), 3);
    }

    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_double_click_on_primary_never_passes_confirmation() {
        close_all();
        let gate = Gate::new();
        // Có phát hiện, bỏ chọn hết.
        let h = open_test(50, &gate, ReviewPhase::Findings, findings());
        press(h, Ctl::DeselectAll);
        double_press(h, Ctl::Primary);
        assert_eq!(model_of(h).confirm(), Confirm::EmptySelection);
        assert!(alive(h));
        // Bấm nút đi tiếp ngay (trong thời gian bấm đúp): bỏ qua.
        press(h, Ctl::Yes);
        assert_eq!(model_of(h).confirm(), Confirm::EmptySelection);
        assert!(outcomes().is_empty());
        // Ctrl+A khi thanh đang hiện: bỏ thanh, tiêu điểm về nút chính, bố cục về hàng nút thường.
        assert!(on_key(h, u16::from(b'A'), true, false));
        assert_eq!(model_of(h).confirm(), Confirm::None);
        assert_eq!(with_win(h, |w| w.focus), Some(Ctl::Primary));
        assert!(button(h, Ctl::Yes).is_none() && button(h, Ctl::Primary).is_some());
        close(50);
        // Lỗi quét.
        let h = open_test(51, &gate, ReviewPhase::Error(ScanError::Failed), Vec::new());
        double_press(h, Ctl::Primary);
        assert_eq!(model_of(h).confirm(), Confirm::Unscanned);
        assert!(alive(h));
        assert!(outcomes().is_empty());
        // Kết quả muộn khi không còn Scanning: bỏ, thanh xác nhận và tiêu điểm giữ nguyên.
        assert!(update(51, Ok(findings())));
        assert_eq!(model_of(h).confirm(), Confirm::Unscanned);
        assert_eq!(with_win(h, |w| w.focus), Some(Ctl::Back));
        wait_double_click();
        press(h, Ctl::Yes);
        pump(100, || !alive(h));
        assert_eq!(
            outcomes(),
            vec![(
                51,
                ReviewOutcome::ContinueUnscanned(RedactionPlan::default())
            )]
        );
    }

    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_layout_after_dpi_change_with_real_fonts() {
        crate::webshot::per_monitor();
        close_all();
        let gate = Gate::new();
        let h = open_test(30, &gate, ReviewPhase::Findings, findings());
        for dpi in DPIS {
            for lang in LANGS {
                crate::lang::set(lang);
                let (ww, wh) = window_size(MIN_W, MIN_H, dpi);
                let mut now = RECT::default();
                // SAFETY: now là biến cục bộ; hỏi khung của cửa sổ này.
                unsafe {
                    let _ = windows::Win32::UI::WindowsAndMessaging::GetWindowRect(h, &mut now);
                }
                let rect = RECT {
                    left: now.left,
                    top: now.top,
                    right: now.left + ww,
                    bottom: now.top + wh,
                };
                // SAFETY: gửi đồng bộ tới cửa sổ của luồng này; RECT sống suốt lời gọi.
                unsafe {
                    SendMessageW(
                        h,
                        WM_DPICHANGED,
                        Some(WPARAM(((dpi as usize) << 16) | dpi as usize)),
                        Some(LPARAM(&rect as *const RECT as isize)),
                    );
                }
                // Đọc ngay (không bơm): hệ thống có thể gửi lại DPI thật của màn hình sau đó.
                let (buttons, font, bold, list, note) = with_win(h, |w| {
                    (
                        w.layout.buttons.clone(),
                        w.font,
                        w.bold,
                        w.layout.list,
                        w.layout.note,
                    )
                })
                .unwrap();
                let ctx = format!("dpi {dpi} {lang:?}");
                assert_eq!(with_win(h, |w| w.dpi), Some(dpi), "{ctx}");
                assert!(!overlaps(list, note), "{ctx}");
                for (i, &(c, b)) in buttons.iter().enumerate() {
                    let f = if kind_of(c, Body::Findings) == ButtonKind::Normal {
                        font
                    } else {
                        bold
                    };
                    let tw = history_paint::with_screen_dc(|dc| {
                        let s = label(c, Body::Findings, Confirm::None, lang);
                        history_paint::measure(dc, f, s).0
                    });
                    assert!(b.w >= tw + scale(8, dpi), "{ctx} {c:?} {} < {tw}", b.w);
                    for &(d, o) in &buttons[i + 1..] {
                        assert!(!overlaps(b, o), "{ctx} {c:?} vs {d:?}");
                    }
                }
            }
        }
        crate::lang::set(Lang::Vi);
        close(30);
    }

    /// Ảnh PrintWindow của các trạng thái (hai ngôn ngữ). `DARKSHOT_REVIEW_SHOTS` = thư mục lưu PNG
    /// (ngoài repo); thiếu biến thì chỉ kiểm ảnh không đen trơn.
    #[test]
    #[ignore = "mở cửa sổ thật; chạy tay"]
    fn manual_print_window_states() {
        crate::webshot::per_monitor();
        close_all();
        let gate = Gate::new();
        let dir = std::env::var_os("DARKSHOT_REVIEW_SHOTS").map(std::path::PathBuf::from);
        if let Some(d) = &dir {
            std::fs::create_dir_all(d).expect("dir");
        }
        crate::webshot::each_lang(|_, code| {
            let shot = |h: HWND, name: &str| {
                invalidate(h);
                pump(150, || false);
                let img = crate::webshot::print_window(h);
                assert!(crate::webshot::not_blank(&img));
                if let Some(d) = &dir {
                    crate::webshot::save(d, &format!("review-{name}-{code}"), &img);
                }
            };
            // Có phát hiện: một mục bỏ chọn, một vùng tự thêm, trỏ mục PEM.
            let h = open_test(40, &gate, ReviewPhase::Findings, findings());
            with_win(h, |w| {
                w.model.toggle(2);
                w.model.set_adding(true);
                w.model.drag_begin(700, 60);
                w.model.drag_move(900, 140);
                w.model.drag_end();
                w.model.set_adding(false);
                w.model.hover = Some(3);
            });
            relayout(h);
            shot(h, "findings");
            close(40);
            // Bước k/n + thanh xác nhận rỗng.
            let req = ReviewRequest {
                token: 41,
                image: sample_image(960, 540),
                action: Msg::PrivacyActionAddStep,
                step: Some((3, 12)),
                phase: ReviewPhase::Findings,
                findings: findings(),
            };
            open(req, gate.0, WM_PRIVACY_REVIEW).expect("open");
            pump(50, || false);
            let h = hwnd_of(41).unwrap();
            press(h, Ctl::DeselectAll);
            press(h, Ctl::Primary);
            shot(h, "confirm-empty");
            close(41);
            // Thêm vùng che đang kéo.
            let h = open_test(43, &gate, ReviewPhase::Findings, findings());
            press(h, Ctl::Add);
            with_win(h, |w| {
                w.model.drag_begin(620, 40);
                w.model.drag_move(920, 130);
            });
            shot(h, "adding");
            close(43);
            // Đang quét, dải 2/5.
            let h = open_test(42, &gate, ReviewPhase::Scanning { tile: None }, Vec::new());
            progress(42, 2, 5);
            shot(h, "scanning");
            // Lỗi, rồi thanh xác nhận chưa quét.
            update(42, Err(ScanError::NoLanguage));
            shot(h, "error");
            press(h, Ctl::Primary);
            shot(h, "confirm-unscanned");
            close(42);
        });
        // Chỉ `close` của cổng: không gửi gì.
        assert!(outcomes().is_empty(), "{:?}", outcomes());
    }
}
