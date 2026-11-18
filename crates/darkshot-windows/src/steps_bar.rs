//! Thanh phiên chụp từng bước: cửa sổ nhỏ, tối, luôn trên cùng, không lên thanh tác vụ, không
//! giành tiêu điểm, không có trong ảnh chụp.
//!
//! Quy tắc tái nhập: trạng thái `BAR` chỉ được mượn trong các khối ngắn không gọi hàm Win32 có thể
//! gửi thông điệp cho cửa sổ (SetWindowPos, DestroyWindow, SetCapture...); việc của nút bấm chạy
//! sau khi đã trả mượn.

use crate::lang;
use crate::theme::{self, ButtonKind, ControlState};
use crate::winutil::{scale, system_font};
use darkshot_core::geometry::{Point, Rect};
use darkshot_core::i18n::{self, Msg};
use darkshot_core::theme::{DANGER_BUTTON, LINE, SURFACE, TEXT};
use std::cell::{Cell, RefCell};
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::DwmFlush;
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateCompatibleDC, DeleteDC, DeleteObject, EndPaint, GetMonitorInfoW,
    InvalidateRect, MonitorFromPoint, MonitorFromRect, DT_CALCRECT, DT_LEFT, DT_NOPREFIX,
    DT_SINGLELINE, DT_VCENTER, HFONT, HGDIOBJ, MONITORINFO, MONITOR_DEFAULTTONULL,
    MONITOR_DEFAULTTOPRIMARY, PAINTSTRUCT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};
use windows::Win32::UI::Controls::WM_MOUSELEAVE;
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    ReleaseCapture, SetCapture, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetCursorPos, GetWindowRect, IsWindowVisible,
    LoadCursorW, RegisterClassW, SetWindowDisplayAffinity, SetWindowPos, ShowWindow, IDC_ARROW,
    MA_NOACTIVATE, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, SW_HIDE, SW_SHOWNA,
    WDA_EXCLUDEFROMCAPTURE, WM_CAPTURECHANGED, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_ERASEBKGND,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEACTIVATE, WM_MOUSEMOVE, WM_NCDESTROY, WM_PAINT,
    WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

const CLASS: PCWSTR = w!("darkshot.steps-bar");

// Kích thước ở 96 DPI (DIP).
/// Chiều cao tối thiểu của thanh.
const BAR_H: i32 = 40;
/// Lề trái/phải.
const PAD: i32 = 12;
/// Chấm đỏ: đường kính, khoảng cách tới chữ.
const DOT: i32 = 8;
const DOT_GAP: i32 = 8;
/// Khoảng cách từ chữ tới nút đầu.
const TITLE_GAP: i32 = 16;
/// Nút: lề chữ hai bên, chiều cao tối thiểu, khoảng cách giữa hai nút.
const BTN_PAD: i32 = 12;
const BTN_H: i32 = 28;
const BTN_GAP: i32 = 6;
/// Cách mép trên vùng làm việc.
const TOP_MARGIN: i32 = 12;

/// Bản dựng Windows đầu tiên hỗ trợ `WDA_EXCLUDEFROMCAPTURE` (Windows 10 2004). Bản cũ hơn coi
/// giá trị này như `WDA_MONITOR` (ô đen trong ảnh chụp) nên không dùng được.
const EXCLUDE_MIN_BUILD: u32 = 19041;

/// Nút của thanh phiên.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarButton {
    Capture,
    Review,
    Finish,
}

impl BarButton {
    fn label(self) -> Msg {
        match self {
            BarButton::Capture => Msg::StepsCapture,
            BarButton::Review => Msg::StepsReview,
            BarButton::Finish => Msg::StepsFinish,
        }
    }

    fn kind(self) -> ButtonKind {
        match self {
            BarButton::Capture => ButtonKind::Primary,
            _ => ButtonKind::Normal,
        }
    }
}

/// Thứ tự nút từ trái sang phải.
const BUTTONS: [BarButton; 3] = [BarButton::Capture, BarButton::Review, BarButton::Finish];

/// Cỡ chữ đã đo (pixel): dòng tiêu đề, nhãn từng nút, chiều cao dòng.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Texts {
    title: i32,
    buttons: [i32; 3],
    height: i32,
}

/// Bố cục thanh (tọa độ client, pixel).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Layout {
    size: (i32, i32),
    dot: Rect,
    title: Rect,
    buttons: [Rect; 3],
}

/// Chấm đỏ, chữ, ba nút xếp ngang, căn giữa theo chiều dọc; đủ chỗ cho chữ đã đo.
fn layout(dpi: u32, t: &Texts) -> Layout {
    let k = |v: i32| scale(v, dpi);
    let h = k(BAR_H).max(t.height + 2 * k(8));
    let btn_h = k(BTN_H).max(t.height + k(8));
    let mut x = k(PAD);
    let dot = Rect {
        x,
        y: (h - k(DOT)) / 2,
        w: k(DOT),
        h: k(DOT),
    };
    x += k(DOT) + k(DOT_GAP);
    let title = Rect {
        x,
        y: (h - t.height) / 2,
        w: t.title,
        h: t.height,
    };
    x += t.title + k(TITLE_GAP);
    let mut buttons = [Rect {
        x: 0,
        y: 0,
        w: 0,
        h: 0,
    }; 3];
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
        buttons,
    }
}

/// Kết quả dò chuột trên thanh.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Hit {
    Button(BarButton),
    /// Phần còn lại của thanh (chấm, chữ, lề): kéo để dời thanh.
    Drag,
    None,
}

fn hit(l: &Layout, p: Point) -> Hit {
    let win = Rect {
        x: 0,
        y: 0,
        w: l.size.0,
        h: l.size.1,
    };
    if !win.contains(p) {
        return Hit::None;
    }
    match l.buttons.iter().position(|b| b.contains(p)) {
        Some(i) => Hit::Button(BUTTONS[i]),
        None => Hit::Drag,
    }
}

/// Góc trên trái mặc định: giữa mép trên vùng làm việc `work`, cách mép `TOP_MARGIN` DIP; thanh
/// rộng hơn vùng làm việc thì sát mép trái.
fn default_origin(work: Rect, size: (i32, i32), dpi: u32) -> Point {
    Point {
        x: work.x + ((work.w - size.0) / 2).max(0),
        y: work.y + scale(TOP_MARGIN, dpi),
    }
}

/// Tọa độ x mới khi bề rộng đổi từ `old_w` sang `new_w`, giữ tâm ngang.
fn recentered_x(x: i32, old_w: i32, new_w: i32) -> i32 {
    x + (old_w - new_w) / 2
}

/// `WDA_EXCLUDEFROMCAPTURE` dùng được ở bản dựng `build` (không biết thì coi là không).
fn exclude_supported(build: Option<u32>) -> bool {
    build.is_some_and(|b| b >= EXCLUDE_MIN_BUILD)
}

fn parse_build(s: &str) -> Option<u32> {
    s.trim().parse().ok()
}

/// Số bản dựng Windows (`CurrentBuildNumber` trong registry; không phụ thuộc manifest).
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
    parse_build(&String::from_utf16_lossy(&buf[..len]))
}

/// Trạng thái thanh phiên (một thanh cho cả tiến trình, trên luồng giao diện).
struct Bar {
    dpi: u32,
    font: Option<HFONT>,
    title: Vec<u16>,
    labels: [Vec<u16>; 3],
    layout: Layout,
    hover: Option<BarButton>,
    pressed: Option<BarButton>,
    /// Đang kéo: con trỏ trừ góc trên trái cửa sổ (tọa độ màn hình).
    drag: Option<Point>,
    tracking: bool,
}

impl Drop for Bar {
    fn drop(&mut self) {
        if let Some(f) = self.font.take() {
            // SAFETY: phông do thanh tạo, không còn được chọn vào DC nào.
            unsafe {
                let _ = DeleteObject(HGDIOBJ(f.0));
            }
        }
    }
}

fn noop() {}

thread_local! {
    static HWND_CELL: Cell<Option<HWND>> = const { Cell::new(None) };
    static BAR: RefCell<Option<Bar>> = const { RefCell::new(None) };
    static CLASS_DONE: Cell<bool> = const { Cell::new(false) };
    /// Thanh đang được loại khỏi ảnh chụp (`WDA_EXCLUDEFROMCAPTURE` đã đặt và được hỗ trợ).
    static EXCLUDED: Cell<bool> = const { Cell::new(false) };
    /// Thanh đã bị ẩn quanh lần chụp (khi không loại khỏi ảnh chụp được).
    static HIDDEN: Cell<bool> = const { Cell::new(false) };
    static ON_CAPTURE: Cell<fn()> = const { Cell::new(noop) };
    static ON_REVIEW: Cell<fn()> = const { Cell::new(noop) };
    static ON_CONFIRM_END: Cell<fn()> = const { Cell::new(noop) };
}

/// Hàm chạy khi bấm Chụp bước (main: yêu cầu chụp vùng qua cổng chụp).
pub fn set_on_capture(f: fn()) {
    ON_CAPTURE.with(|c| c.set(f));
}

/// Hàm mở cửa sổ Xem lại (main: `steps_window::open`); mặc định không làm gì.
pub fn set_on_review(f: fn()) {
    ON_REVIEW.with(|c| c.set(f));
}

/// Hàm mở Xem lại ở chế độ xác nhận kết thúc (main: `steps_window::open_confirm_end`); mặc định
/// không làm gì.
pub fn set_on_confirm_end(f: fn()) {
    ON_CONFIRM_END.with(|c| c.set(f));
}

/// Mở cửa sổ Xem lại (nút Xem lại của thanh phiên).
pub fn review() {
    ON_REVIEW.with(|c| c.get())();
}

/// Kết thúc phiên như nút Kết thúc: còn bước chưa xuất thì hỏi qua cửa sổ Xem lại, không thì
/// kết thúc ngay.
pub fn finish() {
    if crate::steps_store::dirty() && crate::steps_store::count() > 0 {
        ON_CONFIRM_END.with(|c| c.get())();
    } else {
        // Không xóa được `session.json`: phiên giữ nguyên. Xóa được json mà còn sót tệp: phiên đã
        // kết thúc; phần sót được đổi tên (giữ lại) ở lần bắt đầu phiên sau. Lỗi được báo qua
        // `steps_store::set_on_error`.
        crate::steps_store::end();
    }
}

pub fn hwnd() -> Option<HWND> {
    HWND_CELL.with(|h| h.get())
}

/// Khung thanh phiên đang hiện (tọa độ màn hình); `None` khi không có hay đang ẩn.
pub fn frame() -> Option<Rect> {
    let h = hwnd()?;
    let mut wr = RECT::default();
    // SAFETY: h là cửa sổ của luồng này; chỉ đọc trạng thái và khung.
    unsafe {
        if !IsWindowVisible(h).as_bool() || GetWindowRect(h, &mut wr).is_err() {
            return None;
        }
    }
    Some(Rect {
        x: wr.left,
        y: wr.top,
        w: wr.right - wr.left,
        h: wr.bottom - wr.top,
    })
}

/// Thanh đang được loại khỏi ảnh chụp.
pub fn excluded() -> bool {
    EXCLUDED.with(|e| e.get())
}

fn with_bar<R>(f: impl FnOnce(&mut Bar) -> R) -> Option<R> {
    BAR.with(|b| b.try_borrow_mut().ok()?.as_mut().map(f))
}

fn to_rect(r: Rect) -> RECT {
    RECT {
        left: r.x,
        top: r.y,
        right: r.right(),
        bottom: r.bottom(),
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

/// Dòng tiêu đề: "Chụp từng bước · N bước".
fn title_text() -> String {
    format!(
        "{} \u{00B7} {}",
        lang::tr(Msg::StepsBarTitle),
        i18n::steps_count(lang::current(), crate::steps_store::count())
    )
}

/// Đo chữ bằng phông `font` (một dòng).
fn measure(font: Option<HFONT>, title: &[u16], labels: &[Vec<u16>; 3]) -> Texts {
    // SAFETY: DC bộ nhớ tạo và xóa trong hàm; `draw_text` chọn lại phông cũ trước khi trả.
    unsafe {
        let dc = CreateCompatibleDC(None);
        if dc.is_invalid() {
            return Texts::default();
        }
        let flags = DT_CALCRECT | DT_SINGLELINE | DT_NOPREFIX;
        let size = |t: &[u16]| {
            let r = theme::draw_text(dc, t, &RECT::default(), TEXT, font, flags);
            (r.right - r.left, r.bottom - r.top)
        };
        let (tw, th) = size(title);
        let mut out = Texts {
            title: tw,
            buttons: [0; 3],
            height: th,
        };
        for (slot, l) in out.buttons.iter_mut().zip(labels) {
            let (w, h) = size(l);
            *slot = w;
            out.height = out.height.max(h);
        }
        let _ = DeleteDC(dc);
        out
    }
}

/// Chữ, phông và bố cục mới ở `dpi`; phông cũ (nếu có) bị xóa khi `Bar` cũ được thay.
fn build(dpi: u32) -> (Option<HFONT>, Vec<u16>, [Vec<u16>; 3], Layout) {
    let font = system_font(dpi);
    let title = wide(&title_text());
    let labels = BUTTONS.map(|b| wide(lang::tr(b.label())));
    let texts = measure(font, &title, &labels);
    let l = layout(dpi, &texts);
    (font, title, labels, l)
}

/// Vùng làm việc và DPI của màn hình chính.
fn primary_work() -> (Rect, u32) {
    // SAFETY: chỉ hỏi thông tin màn hình; các biến là cục bộ.
    unsafe {
        let m = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let work = if GetMonitorInfoW(m, &mut mi).as_bool() {
            let r = mi.rcWork;
            Rect {
                x: r.left,
                y: r.top,
                w: r.right - r.left,
                h: r.bottom - r.top,
            }
        } else {
            Rect {
                x: 0,
                y: 0,
                w: 1024,
                h: 768,
            }
        };
        let (mut x, mut y) = (96u32, 96u32);
        let dpi = if GetDpiForMonitor(m, MDT_EFFECTIVE_DPI, &mut x, &mut y).is_ok() {
            x
        } else {
            96
        };
        (work, dpi)
    }
}

fn register_class() {
    if CLASS_DONE.with(|c| c.get()) {
        return;
    }
    // SAFETY: đăng ký lớp cửa sổ của luồng này; đăng ký lại lần hai báo lỗi vô hại.
    unsafe {
        let Ok(hinst) = GetModuleHandleW(None) else {
            return;
        };
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: hinst.into(),
            lpszClassName: CLASS,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            ..Default::default()
        };
        RegisterClassW(&wc);
    }
    CLASS_DONE.with(|c| c.set(true));
}

/// Hiện thanh phiên (tạo nếu chưa có), không giành tiêu điểm; đã có thì cập nhật chữ.
pub fn show() {
    if let Some(h) = hwnd() {
        relayout(h, None);
        // SAFETY: h là cửa sổ còn sống của luồng này; SW_SHOWNA không kích hoạt.
        unsafe {
            if !HIDDEN.with(|x| x.get()) {
                let _ = ShowWindow(h, SW_SHOWNA);
            }
        }
        return;
    }
    register_class();
    let (work, dpi) = primary_work();
    let (font, title, labels, l) = build(dpi);
    let at = default_origin(work, l.size, dpi);
    // SAFETY: tạo cửa sổ popup của luồng này; lỗi thì xóa phông vừa tạo.
    let created = unsafe {
        let hinst = GetModuleHandleW(None).ok();
        CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            CLASS,
            &HSTRING::from(lang::tr(Msg::StepsBarTitle)),
            WS_POPUP,
            at.x,
            at.y,
            l.size.0,
            l.size.1,
            None,
            None,
            hinst.map(|h| h.into()),
            None,
        )
    };
    let Ok(h) = created else {
        if let Some(f) = font {
            // SAFETY: phông vừa tạo, chưa chọn vào DC nào.
            unsafe {
                let _ = DeleteObject(HGDIOBJ(f.0));
            }
        }
        return;
    };
    HWND_CELL.with(|c| c.set(Some(h)));
    BAR.with(|b| {
        *b.borrow_mut() = Some(Bar {
            dpi,
            font,
            title,
            labels,
            layout: l,
            hover: None,
            pressed: None,
            drag: None,
            tracking: false,
        })
    });
    // SAFETY: h là cửa sổ cấp cao nhất của tiến trình này.
    let set = unsafe { SetWindowDisplayAffinity(h, WDA_EXCLUDEFROMCAPTURE) }.is_ok();
    EXCLUDED.with(|e| e.set(set && exclude_supported(os_build())));
    // SAFETY: hiện mà không kích hoạt (không giành tiêu điểm).
    unsafe {
        let _ = ShowWindow(h, SW_SHOWNA);
    }
}

/// Số bước đổi: cập nhật chữ và cỡ.
pub fn refresh() {
    if let Some(h) = hwnd() {
        relayout(h, None);
    }
}

/// Ngôn ngữ đổi: cập nhật mọi chữ.
pub fn relabel() {
    refresh();
}

/// Đóng thanh phiên (nếu đang mở).
pub fn close() {
    HIDDEN.with(|x| x.set(false));
    if let Some(h) = hwnd() {
        // SAFETY: h là cửa sổ của luồng này; WM_NCDESTROY dọn trạng thái.
        unsafe {
            let _ = DestroyWindow(h);
        }
    }
}

/// Trước khi chụp: thanh không được loại khỏi ảnh chụp thì ẩn nó (trả `true` khi đã ẩn).
pub fn hide_for_capture() -> bool {
    let Some(h) = hwnd() else {
        return false;
    };
    // SAFETY: h là cửa sổ của luồng này.
    if excluded() || !unsafe { IsWindowVisible(h) }.as_bool() {
        return false;
    }
    HIDDEN.with(|x| x.set(true));
    // SAFETY: ẩn cửa sổ của luồng này rồi chờ DWM vẽ xong khung không có nó.
    unsafe {
        let _ = ShowWindow(h, SW_HIDE);
        let _ = DwmFlush();
    }
    true
}

/// Sau lần chụp: hiện lại thanh đã ẩn bởi `hide_for_capture`.
pub fn show_after_capture() {
    if !HIDDEN.with(|x| x.replace(false)) {
        return;
    }
    if let Some(h) = hwnd() {
        // SAFETY: hiện lại cửa sổ của luồng này, không kích hoạt.
        unsafe {
            let _ = ShowWindow(h, SW_SHOWNA);
        }
    }
}

/// Dựng lại chữ, phông và bố cục (ở `new_dpi` nếu có), đổi cỡ cửa sổ giữ tâm ngang, hoặc đặt
/// theo khung `suggested` (WM_DPICHANGED).
fn relayout(h: HWND, change: Option<(u32, RECT)>) {
    let Some(dpi) = with_bar(|b| change.map_or(b.dpi, |(d, _)| d)) else {
        return;
    };
    let (font, title, labels, l) = build(dpi);
    let old = with_bar(|b| {
        let old_size = b.layout.size;
        if let Some(f) = std::mem::replace(&mut b.font, font) {
            // SAFETY: phông cũ của thanh, không còn được chọn vào DC nào (chỉ chọn trong lúc vẽ).
            unsafe {
                let _ = DeleteObject(HGDIOBJ(f.0));
            }
        }
        b.dpi = dpi;
        b.title = title;
        b.labels = labels;
        b.layout = l;
        b.hover = None;
        old_size
    });
    let Some(old) = old else {
        return;
    };
    let mut wr = RECT::default();
    // SAFETY: h là cửa sổ của luồng này; đổi cỡ không kích hoạt; không giữ mượn `BAR`.
    unsafe {
        let _ = GetWindowRect(h, &mut wr);
        let (x, y) = match change {
            Some((_, s)) => (s.left, s.top),
            None => (recentered_x(wr.left, old.0, l.size.0), wr.top),
        };
        let _ = SetWindowPos(
            h,
            None,
            x,
            y,
            l.size.0,
            l.size.1,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
        let _ = InvalidateRect(Some(h), None, false);
    }
}

/// Màn hình đổi: thanh không còn nằm trên màn hình nào thì về chỗ mặc định.
fn keep_on_screen(h: HWND) {
    let mut wr = RECT::default();
    // SAFETY: h là cửa sổ của luồng này; chỉ hỏi màn hình rồi dời (không kích hoạt).
    unsafe {
        let _ = GetWindowRect(h, &mut wr);
        if !MonitorFromRect(&wr, MONITOR_DEFAULTTONULL).is_invalid() {
            return;
        }
        let (work, dpi) = primary_work();
        let at = default_origin(work, (wr.right - wr.left, wr.bottom - wr.top), dpi);
        let _ = SetWindowPos(
            h,
            None,
            at.x,
            at.y,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
    let dpi = primary_work().1;
    if with_bar(|b| b.dpi) != Some(dpi) {
        relayout(h, None);
    }
}

fn client_point(lp: LPARAM) -> Point {
    Point {
        x: (lp.0 & 0xFFFF) as i16 as i32,
        y: ((lp.0 >> 16) & 0xFFFF) as i16 as i32,
    }
}

fn cursor() -> Point {
    let mut pt = POINT::default();
    // SAFETY: chỉ đọc vị trí chuột.
    unsafe {
        let _ = GetCursorPos(&mut pt);
    }
    Point { x: pt.x, y: pt.y }
}

fn invalidate(h: HWND) {
    // SAFETY: h là cửa sổ của luồng này.
    unsafe {
        let _ = InvalidateRect(Some(h), None, false);
    }
}

fn paint(h: HWND) {
    let mut ps = PAINTSTRUCT::default();
    // SAFETY: cặp BeginPaint/EndPaint của cửa sổ này; mọi đối tượng GDI tạm do `theme` tạo và xóa.
    unsafe {
        let dc = BeginPaint(h, &mut ps);
        with_bar(|b| {
            let full = RECT {
                left: 0,
                top: 0,
                right: b.layout.size.0,
                bottom: b.layout.size.1,
            };
            theme::buffered(dc, &full, |m| {
                let bw = scale(1, b.dpi).max(1);
                theme::round_rect(m, &full, 0, SURFACE, LINE, bw);
                theme::ellipse(m, &to_rect(b.layout.dot), DANGER_BUTTON, DANGER_BUTTON, 0);
                theme::draw_text(
                    m,
                    &b.title,
                    &to_rect(b.layout.title),
                    TEXT,
                    b.font,
                    DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX,
                );
                for (i, btn) in BUTTONS.iter().enumerate() {
                    let state = ControlState {
                        hot: b.hover == Some(*btn),
                        pressed: b.pressed == Some(*btn) && b.hover == Some(*btn),
                        focused: false,
                        enabled: true,
                        checked: false,
                    };
                    theme::draw_button(
                        m,
                        &to_rect(b.layout.buttons[i]),
                        &b.labels[i],
                        btn.kind(),
                        state,
                        b.dpi,
                        b.font,
                        (false, false),
                    );
                }
            });
        });
        let _ = EndPaint(h, &ps);
    }
}

fn run(button: BarButton) {
    match button {
        BarButton::Capture => ON_CAPTURE.with(|c| c.get())(),
        BarButton::Review => review(),
        BarButton::Finish => finish(),
    }
}

unsafe extern "system" fn wndproc(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        // Bấm vào thanh không kích hoạt nó: cửa sổ đang dùng giữ tiêu điểm.
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            paint(h);
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let p = client_point(lp);
            let origin = {
                let mut wr = RECT::default();
                // SAFETY: h là cửa sổ của luồng này.
                unsafe {
                    let _ = GetWindowRect(h, &mut wr);
                }
                Point {
                    x: wr.left,
                    y: wr.top,
                }
            };
            let c = cursor();
            let grab = with_bar(|b| match hit(&b.layout, p) {
                Hit::Button(btn) => {
                    b.pressed = Some(btn);
                    b.hover = Some(btn);
                    true
                }
                Hit::Drag => {
                    b.drag = Some(Point {
                        x: c.x - origin.x,
                        y: c.y - origin.y,
                    });
                    true
                }
                Hit::None => false,
            });
            if grab == Some(true) {
                // SAFETY: giữ chuột cho cửa sổ của luồng này (không giữ mượn `BAR`).
                unsafe {
                    SetCapture(h);
                }
                invalidate(h);
            }
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let p = client_point(lp);
            let c = cursor();
            let (drag, changed, track) = with_bar(|b| {
                if let Some(off) = b.drag {
                    return (Some(off), false, false);
                }
                let now = match hit(&b.layout, p) {
                    Hit::Button(btn) => Some(btn),
                    _ => None,
                };
                let changed = now != b.hover;
                b.hover = now;
                let track = !b.tracking;
                b.tracking = true;
                (None, changed, track)
            })
            .unwrap_or((None, false, false));
            if let Some(off) = drag {
                // SAFETY: dời cửa sổ của luồng này, không kích hoạt, không đổi thứ tự.
                unsafe {
                    let _ = SetWindowPos(
                        h,
                        None,
                        c.x - off.x,
                        c.y - off.y,
                        0,
                        0,
                        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                }
                return LRESULT(0);
            }
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
            with_bar(|b| {
                b.tracking = false;
                b.hover = None;
            });
            invalidate(h);
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let p = client_point(lp);
            let clicked = with_bar(|b| {
                let pressed = b.pressed.take();
                b.drag = None;
                match (pressed, hit(&b.layout, p)) {
                    (Some(x), Hit::Button(y)) if x == y => Some(x),
                    _ => None,
                }
            })
            .flatten();
            // SAFETY: thả chuột của luồng này (không giữ mượn `BAR`).
            unsafe {
                let _ = ReleaseCapture();
            }
            invalidate(h);
            if let Some(btn) = clicked {
                // Có thể hủy chính thanh này (Kết thúc): không đụng `h` sau đó.
                run(btn);
            }
            LRESULT(0)
        }
        WM_CAPTURECHANGED => {
            with_bar(|b| {
                b.drag = None;
                b.pressed = None;
            });
            invalidate(h);
            LRESULT(0)
        }
        WM_DPICHANGED => {
            let dpi = (wp.0 & 0xFFFF) as u32;
            // SAFETY: lParam của WM_DPICHANGED trỏ tới RECT gợi ý do hệ thống cấp, sống suốt thông điệp.
            let suggested = unsafe { *(lp.0 as *const RECT) };
            relayout(h, Some((dpi, suggested)));
            LRESULT(0)
        }
        WM_DISPLAYCHANGE => {
            keep_on_screen(h);
            LRESULT(0)
        }
        WM_NCDESTROY => {
            HWND_CELL.with(|c| {
                if c.get() == Some(h) {
                    c.set(None);
                }
            });
            // Thả trạng thái (và phông) sau khi trả mượn.
            let old = BAR.with(|b| b.try_borrow_mut().ok().and_then(|mut x| x.take()));
            drop(old);
            EXCLUDED.with(|e| e.set(false));
            // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
            unsafe { DefWindowProcW(h, msg, wp, lp) }
        }
        // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
        _ => unsafe { DefWindowProcW(h, msg, wp, lp) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkshot_core::geometry::{Point, Rect};

    fn inside(outer: Rect, r: Rect) -> bool {
        r.x >= outer.x
            && r.y >= outer.y
            && r.right() <= outer.right()
            && r.bottom() <= outer.bottom()
    }

    #[test]
    fn layout_fits_text_at_every_dpi() {
        for dpi in [96u32, 120, 144, 192] {
            let k = |v: i32| v * dpi as i32 / 96;
            let texts = Texts {
                title: k(150),
                buttons: [k(70), k(50), k(55)],
                height: k(16),
            };
            let l = layout(dpi, &texts);
            let win = Rect {
                x: 0,
                y: 0,
                w: l.size.0,
                h: l.size.1,
            };
            assert!(l.size.1 >= k(BAR_H), "dpi {dpi}");
            assert!(inside(win, l.dot) && inside(win, l.title), "dpi {dpi}");
            assert!(l.title.w >= texts.title && l.title.h >= texts.height);
            assert!(l.dot.right() <= l.title.x);
            let mut prev = l.title.right();
            for (b, tw) in l.buttons.iter().zip(texts.buttons) {
                assert!(inside(win, *b), "dpi {dpi} {b:?}");
                // Chữ không bị cắt: đủ chỗ cho chữ và lề hai bên.
                assert!(b.w >= tw + 2 * k(BTN_PAD), "dpi {dpi}");
                assert!(b.h >= texts.height, "dpi {dpi}");
                assert!(b.x >= prev, "dpi {dpi}: chồng nhau");
                // Căn giữa theo chiều dọc.
                assert!((b.y - (win.h - b.bottom())).abs() <= 1, "dpi {dpi}");
                prev = b.right();
            }
            assert_eq!(l.size.0, prev + k(PAD));
        }
    }

    #[test]
    fn hit_testing_buttons_and_drag_area() {
        let texts = Texts {
            title: 150,
            buttons: [70, 50, 55],
            height: 16,
        };
        let l = layout(96, &texts);
        for (k, b) in l.buttons.iter().enumerate() {
            let c = Point {
                x: b.x + b.w / 2,
                y: b.y + b.h / 2,
            };
            assert_eq!(hit(&l, c), Hit::Button(BUTTONS[k]));
        }
        // Vùng chữ và chấm đỏ: kéo.
        let t = Point {
            x: l.title.x + 2,
            y: l.title.y + 2,
        };
        assert_eq!(hit(&l, t), Hit::Drag);
        assert_eq!(hit(&l, Point { x: 1, y: 1 }), Hit::Drag);
        assert_eq!(hit(&l, Point { x: -1, y: 1 }), Hit::None);
        assert_eq!(hit(&l, Point { x: l.size.0, y: 1 }), Hit::None);
        assert_eq!(
            BUTTONS,
            [BarButton::Capture, BarButton::Review, BarButton::Finish]
        );
    }

    #[test]
    fn default_position_is_top_center_of_work_area() {
        let work = Rect {
            x: 0,
            y: 0,
            w: 1920,
            h: 1040,
        };
        let p = default_origin(work, (400, 40), 96);
        assert_eq!(p, Point { x: 760, y: 12 });
        let p = default_origin(
            Rect {
                x: -1280,
                y: 100,
                w: 1280,
                h: 900,
            },
            (600, 60),
            192,
        );
        assert_eq!(p, Point { x: -940, y: 124 });
        // Thanh rộng hơn vùng làm việc: sát mép trái.
        let p = default_origin(
            Rect {
                x: 0,
                y: 0,
                w: 300,
                h: 500,
            },
            (400, 40),
            96,
        );
        assert_eq!(p.x, 0);
    }

    /// Giữ tâm ngang khi đổi bề rộng (đổi số bước, đổi ngôn ngữ).
    #[test]
    fn resize_keeps_horizontal_center() {
        assert_eq!(recentered_x(100, 400, 500), 50);
        assert_eq!(recentered_x(100, 400, 300), 150);
        assert_eq!(recentered_x(100, 400, 400), 100);
    }

    #[test]
    fn exclusion_needs_windows_10_2004() {
        assert!(!exclude_supported(None));
        assert!(!exclude_supported(Some(18363)));
        assert!(!exclude_supported(Some(19040)));
        assert!(exclude_supported(Some(19041)));
        assert!(exclude_supported(Some(26100)));
        assert_eq!(parse_build("26300"), Some(26300));
        assert_eq!(parse_build(" 19041 "), Some(19041));
        assert_eq!(parse_build("x"), None);
        assert!(os_build().is_some_and(|b| b > 7600));
    }
}

/// Chạy tay một lượt (mở cửa sổ thật, không dời chuột hay phím):
/// `cargo test -p darkshot-windows steps_bar::live -- --ignored --test-threads=1`.
#[cfg(test)]
mod live {
    use super::*;
    use windows::Win32::System::Threading::{GetCurrentProcess, GetGuiResources, GR_GDIOBJECTS};
    use windows::Win32::UI::HiDpi::{
        SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetForegroundWindow, GetWindowDisplayAffinity, IsWindow, IsWindowVisible,
        PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
    };

    fn pump() {
        let mut msg = MSG::default();
        // SAFETY: vòng bơm thông điệp chuẩn của luồng kiểm thử.
        unsafe {
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    fn gdi() -> u32 {
        // SAFETY: chỉ đọc bộ đếm của tiến trình hiện tại.
        unsafe { GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS) }
    }

    #[test]
    #[ignore]
    fn bar_opens_excluded_without_focus_and_closes_without_gdi_leak() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        // SAFETY: đặt chế độ DPI cho luồng kiểm thử (như manifest của ứng dụng).
        unsafe {
            SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
        let mut base = 0;
        for i in 0..21 {
            if i == 1 {
                base = gdi();
            }
            // SAFETY: chỉ đọc cửa sổ đang có tiêu điểm.
            let fg = unsafe { GetForegroundWindow() };
            show();
            pump();
            let h = hwnd().expect("thanh phiên đã mở");
            // SAFETY: h là cửa sổ của luồng này.
            unsafe {
                assert!(IsWindow(Some(h)).as_bool());
                assert!(IsWindowVisible(h).as_bool());
            }
            // Không giành tiêu điểm.
            // SAFETY: chỉ đọc cửa sổ đang có tiêu điểm.
            assert_eq!(unsafe { GetForegroundWindow() }, fg);
            let mut aff = 0u32;
            // SAFETY: h là cửa sổ của tiến trình này; aff là biến cục bộ.
            unsafe { GetWindowDisplayAffinity(h, &mut aff) }.unwrap();
            if exclude_supported(os_build()) {
                assert_eq!(aff, WDA_EXCLUDEFROMCAPTURE.0);
                assert!(excluded());
            }
            // Đổi chữ (số bước, ngôn ngữ) rồi đóng.
            refresh();
            relabel();
            pump();
            close();
            pump();
            assert!(hwnd().is_none());
            // SAFETY: chỉ hỏi cửa sổ đã hủy có còn không.
            unsafe { assert!(!IsWindow(Some(h)).as_bool()) };
        }
        let end = gdi();
        eprintln!("GDI {base} -> {end}");
        assert!(end <= base + 2, "GDI {base} -> {end}");
    }
}
