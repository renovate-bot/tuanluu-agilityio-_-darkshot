//! Phiên ghi GIF: khung vùng (bốn cửa sổ mảnh), thanh ghi nổi `● Đang ghi 00:05 / 00:15  [Dừng]
//! [Hủy]`, bộ ghi `darkshot_core::gif_record::Recorder` chạy trên luồng nền.
//!
//! Mọi cửa sổ của phiên luôn trên cùng, không lấy tiêu điểm, loại khỏi ảnh chụp
//! (`WDA_EXCLUDEFROMCAPTURE`). Windows cũ hơn 2004 (không loại được): không vẽ khung, thanh đặt
//! ngoài vùng ghi (không có chỗ thì `NoRoom`). Luồng giao diện không chụp, không mã hóa: hẹn giờ
//! 250 ms chỉ vẽ lại thời gian; luồng nền chỉ gọi `capture_rect` và `PostMessageW`.
//!
//! Quy tắc tái nhập (như `scroll_session`): trạng thái `SESSION` chỉ được mượn trong các khối ngắn
//! không gọi hàm Win32 có thể gửi thông điệp cho cửa sổ; kết thúc phiên lấy trạng thái ra khỏi
//! `SESSION` trước khi hủy cửa sổ. Không dùng chung trạng thái với `scroll_session`/`steps_bar`
//! (chỉ dùng lại các hàm đặt vị trí, khung và đo chữ của `scroll_session`).

use crate::capture;
use crate::lang;
use crate::paths;
use crate::scroll_session;
use crate::theme::{self, ButtonKind, ControlState};
use crate::winutil::{scale, system_font};
use darkshot_core::geometry::{Point, Rect};
use darkshot_core::gif_record::{self, GifError, Outcome, RecordConfig, Recorder};
use darkshot_core::i18n::{Lang, Msg};
use darkshot_core::naming;
use darkshot_core::theme::{DANGER, LINE, SURFACE, TEXT, TEXT_MUTED};
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::DwmFlush;
use windows::Win32::Graphics::Gdi::{
    BeginPaint, DeleteObject, EndPaint, InvalidateRect, DT_LEFT, DT_NOPREFIX, DT_SINGLELINE,
    DT_VCENTER, HFONT, HGDIOBJ, PAINTSTRUCT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::WM_MOUSELEAVE;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    ReleaseCapture, SetCapture, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowRect, KillTimer, LoadCursorW,
    PostMessageW, RegisterClassW, SetLayeredWindowAttributes, SetTimer, SetWindowDisplayAffinity,
    SetWindowPos, ShowWindow, HTTRANSPARENT, IDC_ARROW, LWA_ALPHA, MA_NOACTIVATE, SWP_NOACTIVATE,
    SWP_NOSIZE, SWP_NOZORDER, SW_HIDE, SW_SHOWNA, WDA_EXCLUDEFROMCAPTURE, WINDOW_EX_STYLE, WM_APP,
    WM_CAPTURECHANGED, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_ERASEBKGND, WM_LBUTTONDOWN,
    WM_LBUTTONUP, WM_MOUSEACTIVATE, WM_MOUSEMOVE, WM_NCHITTEST, WM_PAINT, WM_TIMER, WNDCLASSW,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

/// Luồng ghi báo đã có kết quả (gửi tới cửa sổ chủ của phiên). WM_APP+1..+17 đã dùng ở nơi khác.
pub const WM_RECORD_DONE: u32 = WM_APP + 18;

const HOST_CLASS: PCWSTR = w!("darkshot.record-host");
const FRAME_CLASS: PCWSTR = w!("darkshot.record-frame");
const BAR_CLASS: PCWSTR = w!("darkshot.record-bar");

// Kích thước ở 96 DPI (DIP).
/// Chiều cao tối thiểu của thanh.
const BAR_H: i32 = 40;
/// Lề trái/phải.
const PAD: i32 = 12;
/// Chấm đỏ: đường kính, khoảng cách tới chữ.
const DOT: i32 = 8;
const DOT_GAP: i32 = 8;
/// Giữa trạng thái và thời gian.
const TIME_GAP: i32 = 8;
/// Từ thời gian tới nút đầu.
const TEXT_GAP: i32 = 12;
/// Nút: lề chữ hai bên, chiều cao tối thiểu, khoảng cách giữa hai nút.
const BTN_PAD: i32 = 12;
const BTN_H: i32 = 28;
const BTN_GAP: i32 = 6;

/// Hẹn giờ vẽ lại thời gian của cửa sổ chủ (không chụp gì).
const TIMER_ID: usize = 1;
const TICK_MS: u32 = 250;

/// Vì sao không bắt đầu được lần ghi.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StartError {
    /// Đã có lần ghi đang chạy (kể cả đang lưu).
    Busy,
    /// Vùng rỗng.
    Empty,
    /// Không có chỗ cho thanh ngoài vùng ghi (thanh không loại khỏi ảnh chụp được).
    NoRoom,
    /// Không tạo được cửa sổ hoặc hẹn giờ.
    Failed,
    /// Bộ ghi từ chối (vùng không hợp lệ, quá lớn, không tạo được tệp tạm hay luồng).
    Recorder(GifError),
}

/// "MM:SS / MM:SS": thời gian đã ghi (làm tròn xuống giây, tối đa bằng `max_ms`) trên thời lượng
/// tối đa.
fn time_text(elapsed_ms: u64, max_ms: u64) -> String {
    let mmss = |ms: u64| {
        let s = ms / 1000;
        format!("{:02}:{:02}", s / 60, s % 60)
    };
    format!("{} / {}", mmss(elapsed_ms.min(max_ms)), mmss(max_ms))
}

/// Nút của thanh ghi.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BarButton {
    Stop,
    Cancel,
}

impl BarButton {
    fn label(self) -> Msg {
        match self {
            BarButton::Stop => Msg::RecordStop,
            BarButton::Cancel => Msg::RecordCancel,
        }
    }

    fn kind(self) -> ButtonKind {
        match self {
            BarButton::Stop => ButtonKind::Primary,
            BarButton::Cancel => ButtonKind::Normal,
        }
    }
}

/// Thứ tự nút từ trái sang phải: Dừng (chính), Hủy.
const BUTTONS: [BarButton; 2] = [BarButton::Stop, BarButton::Cancel];

/// Bề rộng chữ đã đo (pixel): trạng thái (chỗ cho chữ dài hơn trong "Đang ghi"/"Đang lưu…"),
/// thời gian (chỗ cho chuỗi rộng nhất), nhãn hai nút; chiều cao dòng.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Texts {
    status: i32,
    time: i32,
    buttons: [i32; 2],
    line: i32,
}

/// Các chuỗi của thanh ở ngôn ngữ `lang`: trạng thái ghi, trạng thái lưu, nhãn hai nút.
fn labels(lang: Lang) -> ([Vec<u16>; 2], [Vec<u16>; 2]) {
    let tr = |m: Msg| wide(darkshot_core::i18n::t(lang, m));
    (
        [tr(Msg::RecordStatus), tr(Msg::RecordSaving)],
        BUTTONS.map(|b| tr(b.label())),
    )
}

/// Mọi chuỗi thời gian có thể hiện (mỗi giây từ 0 tới thời lượng tối đa).
fn time_samples() -> Vec<Vec<u16>> {
    (0..=gif_record::MAX_DURATION_MS / 1000)
        .map(|s| wide(&time_text(s * 1000, gif_record::MAX_DURATION_MS)))
        .collect()
}

/// Bề rộng chữ của thanh ở ngôn ngữ `lang`, đo bằng `measure` (bề rộng, chiều cao mỗi chuỗi).
fn measure_texts(lang: Lang, measure: impl Fn(&[&[u16]]) -> Vec<(i32, i32)>) -> Texts {
    let (status, buttons) = labels(lang);
    let times = time_samples();
    let mut parts: Vec<&[u16]> = vec![&status[0], &status[1], &buttons[0], &buttons[1]];
    parts.extend(times.iter().map(|t| t.as_slice()));
    let m = measure(&parts);
    let w = |i: usize| m.get(i).map_or(0, |x| x.0);
    Texts {
        status: w(0).max(w(1)),
        time: (4..parts.len()).map(w).max().unwrap_or(0),
        buttons: [w(2), w(3)],
        line: m.iter().map(|x| x.1).max().unwrap_or(0),
    }
}

/// Bố cục thanh (tọa độ client, pixel).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Layout {
    size: (i32, i32),
    dot: Rect,
    status: Rect,
    time: Rect,
    buttons: [Rect; 2],
}

/// Chấm đỏ, trạng thái, thời gian, hai nút xếp ngang, căn giữa theo chiều dọc.
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
    let status = Rect {
        x,
        y: text_y,
        w: t.status,
        h: t.line,
    };
    x += t.status + k(TIME_GAP);
    let time = Rect {
        x,
        y: text_y,
        w: t.time,
        h: t.line,
    };
    x += t.time + k(TEXT_GAP);
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
        status,
        time,
        buttons,
    }
}

/// Nút dưới điểm `p` (tọa độ client); đang lưu thì không nút nào nhận bấm.
fn hit(l: &Layout, p: Point, saving: bool) -> Option<BarButton> {
    if saving {
        return None;
    }
    l.buttons
        .iter()
        .position(|b| b.contains(p))
        .map(|i| BUTTONS[i])
}

/// Chữ, phông và bố cục của thanh.
struct BarUi {
    dpi: u32,
    font: Option<HFONT>,
    /// "Đang ghi", "Đang lưu…".
    status: [Vec<u16>; 2],
    time: Vec<u16>,
    labels: [Vec<u16>; 2],
    layout: Layout,
    hover: Option<BarButton>,
    pressed: Option<BarButton>,
    tracking: bool,
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

/// Chữ, phông và bố cục ở `dpi`, ngôn ngữ hiện tại; thời gian `elapsed_ms`.
fn build_ui(dpi: u32, elapsed_ms: u64) -> BarUi {
    let font = system_font(dpi);
    let lang = lang::current();
    let texts = measure_texts(lang, |parts| scroll_session::measure(font, parts));
    let (status, labels) = labels(lang);
    BarUi {
        dpi,
        font,
        status,
        time: wide(&time_text(elapsed_ms, gif_record::MAX_DURATION_MS)),
        labels,
        layout: layout(dpi, &texts),
        hover: None,
        pressed: None,
        tracking: false,
    }
}

/// Một lần ghi đang chạy (một lần cho cả tiến trình, trên luồng giao diện). Drop dọn hẹn giờ, mọi
/// cửa sổ và phông; bộ ghi drop sau cùng (yêu cầu hủy nếu còn đang ghi, không chặn).
struct Session {
    /// Vùng ghi (tọa độ màn hình, pixel vật lý).
    region: Rect,
    recorder: Recorder,
    /// Cửa sổ ẩn giữ hẹn giờ, nhận `WM_RECORD_DONE` và WM_DISPLAYCHANGE.
    host: HWND,
    frames: Vec<HWND>,
    bar: HWND,
    /// Thanh đang được loại khỏi ảnh chụp (được phép nằm trong vùng).
    bar_excluded: bool,
    ui: BarUi,
    /// Đã Dừng (hoặc đã hết giờ): thanh hiện "Đang lưu…", nút mờ.
    saving: bool,
    /// Cửa sổ chủ cho hàm đánh thức của luồng nền (0 khi phiên đã dọn).
    wake_to: Arc<AtomicIsize>,
}

impl Drop for Session {
    fn drop(&mut self) {
        // Luồng nền không đăng thông điệp tới cửa sổ chủ sắp hủy nữa.
        self.wake_to.store(0, Ordering::Release);
        // SAFETY: các cửa sổ và hẹn giờ do phiên tạo trên luồng này; mỗi thứ được giải phóng đúng
        // một lần (Session chỉ drop một lần, sau khi đã ra khỏi `SESSION`).
        unsafe {
            let _ = KillTimer(Some(self.host), TIMER_ID);
            let _ = DestroyWindow(self.bar);
            for f in self.frames.drain(..) {
                let _ = DestroyWindow(f);
            }
            let _ = DestroyWindow(self.host);
        }
        // Phông của thanh được xóa khi `ui` drop (cửa sổ thanh đã hủy).
    }
}

fn noop_outcome(_: Outcome) {}

thread_local! {
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
    static CLASSES_DONE: Cell<bool> = const { Cell::new(false) };
    static ON_OUTCOME: Cell<fn(Outcome)> = const { Cell::new(noop_outcome) };
}

/// Hàm nhận kết quả của mỗi lần ghi đã bắt đầu (Dừng, Hủy, tự dừng, lỗi, màn hình đổi), sau khi
/// cửa sổ của phiên đã hủy. Không gọi khi thoát ứng dụng (`finish_on_exit`).
pub fn set_on_outcome(f: fn(Outcome)) {
    ON_OUTCOME.with(|c| c.set(f));
}

/// Có lần ghi đang chạy: từ `start` tới khi đã dọn xong (kể cả lúc đang lưu; trạng thái đang bận
/// cũng tính là có).
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

fn client_point(lp: LPARAM) -> Point {
    Point {
        x: (lp.0 & 0xFFFF) as i16 as i32,
        y: ((lp.0 >> 16) & 0xFFFF) as i16 as i32,
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
            &HSTRING::from(lang::tr(Msg::OverlayRecordGif)),
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
    // SAFETY: h là cửa sổ của luồng này, chưa thuộc phiên nào.
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
    let t = scroll_session::frame_thickness(dpi);
    for r in scroll_session::frame_rects(region, t) {
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

/// Chỗ đặt thanh cỡ `size` (DPI `dpi`) cho `region` (cách vùng như thanh chụp cuộn).
fn place(region: Rect, size: (i32, i32), dpi: u32, excluded: bool) -> Option<Point> {
    scroll_session::place(region, size, dpi, excluded)
}

/// Thanh cỡ `ui` đặt cho `region`, dựng lại theo DPI của màn hình chứa thanh nếu khác `ui.dpi`.
fn place_ui(region: Rect, ui: BarUi, excluded: bool, elapsed_ms: u64) -> Option<(Point, BarUi)> {
    let at = place(region, ui.layout.size, ui.dpi, excluded)?;
    let r = Rect {
        x: at.x,
        y: at.y,
        w: ui.layout.size.0,
        h: ui.layout.size.1,
    };
    let dpi = scroll_session::placement(r).dpi;
    if dpi == ui.dpi {
        return Some((at, ui));
    }
    let other = build_ui(dpi, elapsed_ms);
    match place(region, other.layout.size, dpi, excluded) {
        Some(p) => Some((p, other)),
        None => Some((at, ui)),
    }
}

/// Thanh ở chỗ hợp lệ, cùng chữ và bố cục theo DPI của màn hình chứa nó, và cờ đang loại khỏi ảnh
/// chụp; `None` khi không có chỗ nào ngoài vùng (không loại được) hoặc lỗi tạo.
fn create_bar(region: Rect, dpi: u32, supported: bool) -> Option<(HWND, BarUi, bool)> {
    let (at, ui) = place_ui(region, build_ui(dpi, 0), supported, 0)?;
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

/// Bắt đầu ghi `region` (tọa độ màn hình, pixel vật lý) thành GIF trong `save_dir`. Từ chối khi đã
/// có lần ghi (`Busy`), vùng rỗng (`Empty`), vùng không hợp lệ/quá lớn (`Recorder`, báo trước khi
/// tạo gì), không có chỗ cho thanh ngoài vùng (`NoRoom`), lỗi cửa sổ/hẹn giờ (`Failed`) hay lỗi của
/// bộ ghi (`Recorder`). Mọi đường lỗi dọn sạch cái đã tạo.
pub fn start(region: Rect, save_dir: PathBuf) -> Result<(), StartError> {
    if active() {
        return Err(StartError::Busy);
    }
    if region.w <= 0 || region.h <= 0 {
        return Err(StartError::Empty);
    }
    // Báo vùng không hợp lệ/quá lớn ngay, trước khi cấp phát khung thô nào.
    gif_record::frame_size(region.w as u32, region.h as u32).map_err(StartError::Recorder)?;
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
    let supported = scroll_session::exclude_supported(scroll_session::os_build());
    let dpi = scroll_session::placement(region).dpi;
    let Some((bar, ui, bar_excluded)) = create_bar(region, dpi, supported) else {
        destroy(host);
        return Err(StartError::NoRoom);
    };
    let frames = create_frames(region, dpi, supported);
    let destroy_all = |frames: Vec<HWND>| {
        destroy(bar);
        frames.into_iter().for_each(destroy);
        destroy(host);
    };
    // SAFETY: hẹn giờ gắn với cửa sổ chủ của luồng này; KillTimer trong Drop của Session (hoặc
    // cửa sổ bị hủy ngay dưới đây).
    if unsafe { SetTimer(Some(host), TIMER_ID, TICK_MS, None) } == 0 {
        drop(ui);
        destroy_all(frames);
        return Err(StartError::Failed);
    }
    // Lớp phủ vừa đóng: chờ DWM trình bày một khung để khung GIF đầu không còn lớp phủ.
    // SAFETY: chỉ chờ DWM; lỗi (DWM tắt) thì bỏ qua.
    unsafe {
        let _ = DwmFlush();
    }
    let wake_to = Arc::new(AtomicIsize::new(host.0 as isize));
    let cfg = RecordConfig::new(save_dir, naming::local_gif_name(&paths::local_now()));
    // Không panic: lỗi chụp thành `Err` (bản phát hành dùng `panic = "abort"`).
    let source = move || capture::capture_rect(region).map_err(|e| e.message());
    let wake = {
        let to = wake_to.clone();
        move || {
            let raw = to.load(Ordering::Acquire);
            if raw != 0 {
                // SAFETY: HWND không `Send` nên đi qua luồng dưới dạng số; PostMessageW được phép
                // gọi từ mọi luồng và chỉ báo lỗi (bỏ qua) khi cửa sổ không còn.
                unsafe {
                    let _ = PostMessageW(
                        Some(HWND(raw as *mut core::ffi::c_void)),
                        WM_RECORD_DONE,
                        WPARAM(0),
                        LPARAM(0),
                    );
                }
            }
        }
    };
    let recorder = match Recorder::start(cfg, source, wake) {
        Ok(r) => r,
        Err(e) => {
            drop(ui);
            destroy_all(frames);
            return Err(StartError::Recorder(e));
        }
    };
    let session = Session {
        region,
        recorder,
        host,
        frames,
        bar,
        bar_excluded,
        ui,
        saving: false,
        wake_to,
    };
    let shown: Vec<HWND> = session
        .frames
        .iter()
        .copied()
        .chain([session.bar])
        .collect();
    // Đang bị mượn (không xảy ra theo quy tắc tái nhập): bỏ phiên, không panic trong thủ tục cửa sổ.
    let mut slot = Some(session);
    SESSION.with(|s| {
        if let Ok(mut b) = s.try_borrow_mut() {
            *b = slot.take();
        }
    });
    if let Some(s) = slot {
        drop(s);
        return Err(StartError::Failed);
    }
    for h in shown {
        // SAFETY: hiện cửa sổ của phiên mà không kích hoạt (không giành tiêu điểm).
        unsafe {
            let _ = ShowWindow(h, SW_SHOWNA);
        }
    }
    Ok(())
}

/// Dừng và lưu phần đã ghi. Thanh đổi sang "Đang lưu…" (nút mờ) tới khi có kết quả. An toàn khi
/// gọi nhiều lần hoặc không có lần ghi.
pub fn stop() {
    let bar = with(|s| {
        if s.saving {
            return None;
        }
        s.recorder.request_stop();
        s.saving = true;
        s.ui.hover = None;
        s.ui.pressed = None;
        Some(s.bar)
    })
    .flatten();
    if let Some(b) = bar {
        invalidate(b);
    }
}

/// Hủy, không tạo GIF: ẩn thanh và khung ngay, dọn khi luồng ghi báo xong. Khi bộ ghi đã tự dừng
/// (đang lưu) thì Hủy không còn tác dụng. An toàn khi gọi nhiều lần hoặc không có lần ghi.
pub fn cancel() {
    let Some((cancelled, wins, bar)) = with(|s| {
        let ok = s.recorder.request_cancel();
        if !ok {
            s.saving = true;
        }
        let wins: Vec<HWND> = s.frames.iter().copied().chain([s.bar]).collect();
        (ok, wins, s.bar)
    }) else {
        return;
    };
    if cancelled {
        for h in wins {
            // SAFETY: ẩn cửa sổ của phiên trên luồng này (không giữ mượn `SESSION`).
            unsafe {
                let _ = ShowWindow(h, SW_HIDE);
            }
        }
    } else {
        invalidate(bar);
    }
}

/// Thoát ứng dụng: Dừng (lưu phần đã ghi) rồi chờ luồng ghi tới hạn chót `deadline`; dọn cửa sổ.
/// Quá hạn thì chỉ bỏ chờ: luồng nền tự chốt tệp nếu còn kịp, tệp `.part` sót được
/// `gif_record::sweep_stale` dọn ở lần khởi động sau. Không gọi hàm kết quả.
pub fn finish_on_exit(deadline: Instant) {
    let Some(mut s) = SESSION.with(|c| c.try_borrow_mut().ok().and_then(|mut x| x.take())) else {
        return;
    };
    s.recorder.request_stop();
    let _ = s
        .recorder
        .join(deadline.saturating_duration_since(Instant::now()));
    drop(s);
}

/// Ngôn ngữ đổi: dựng lại chữ và cỡ của thanh.
pub fn relabel() {
    relayout_bar(None);
}

/// Dựng lại chữ, phông và bố cục của thanh ở `dpi` (hoặc DPI của màn hình chứa vùng), đặt lại thanh
/// và khung theo vùng.
fn relayout_bar(dpi: Option<u32>) {
    let Some((region, bar, frames, excluded, elapsed)) = with(|s| {
        (
            s.region,
            s.bar,
            s.frames.clone(),
            s.bar_excluded,
            s.recorder.elapsed_ms(),
        )
    }) else {
        return;
    };
    let dpi = dpi.unwrap_or_else(|| scroll_session::placement(region).dpi);
    let ui = build_ui(dpi, elapsed);
    let size = ui.layout.size;
    let at = place(region, size, ui.dpi, excluded);
    let mut slot = Some(ui);
    let old = with(|s| slot.take().map(|ui| std::mem::replace(&mut s.ui, ui))).flatten();
    // Phông cũ được xóa ở đây (không DC nào giữ nó ngoài lúc vẽ).
    drop(old);
    drop(slot);
    let t = scroll_session::frame_thickness(dpi);
    let mut wr = RECT::default();
    // SAFETY: bar và khung là cửa sổ của phiên trên luồng này; đổi cỡ/dời không kích hoạt, không
    // giữ mượn `SESSION`.
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
        for (h, r) in frames.iter().zip(scroll_session::frame_rects(region, t)) {
            let _ = SetWindowPos(*h, None, r.x, r.y, r.w, r.h, SWP_NOZORDER | SWP_NOACTIVATE);
        }
    }
    invalidate(bar);
}

/// Có kết quả của luồng ghi: lấy kết quả và phiên ra cùng lúc, dọn cửa sổ (Drop), rồi gọi hàm kết
/// quả. Chưa có kết quả hoặc phiên đã dọn: không làm gì.
fn poll_done() {
    let ended = SESSION.with(|c| {
        let mut b = c.try_borrow_mut().ok()?;
        let outcome = b.as_mut()?.recorder.try_outcome()?;
        Some((outcome, b.take()?))
    });
    let Some((outcome, s)) = ended else {
        return;
    };
    drop(s);
    ON_OUTCOME.with(|c| c.get())(outcome);
}

/// Nhịp 250 ms: cập nhật thời gian (hết giờ thì hiện "Đang lưu…"); kết quả đã có mà thông điệp
/// đánh thức bị mất thì kết thúc luôn.
fn on_tick() {
    let bar = with(|s| {
        let elapsed = s.recorder.elapsed_ms();
        if elapsed >= gif_record::MAX_DURATION_MS && !s.saving {
            s.saving = true;
            s.ui.hover = None;
            s.ui.pressed = None;
        }
        s.ui.time = wide(&time_text(elapsed, gif_record::MAX_DURATION_MS));
        s.bar
    });
    if let Some(b) = bar {
        invalidate(b);
    }
    poll_done();
}

/// Màn hình đổi: đặt lại thanh; vùng không còn nằm trọn trên các màn hình thì Dừng (lưu phần đã
/// ghi).
fn on_display_change() {
    let Some(region) = with(|s| s.region) else {
        return;
    };
    let screens: Vec<Rect> = scroll_session::monitors()
        .into_iter()
        .map(|(m, _)| m)
        .collect();
    if !scroll_session::region_valid(region, &screens) {
        stop();
    }
    relayout_bar(None);
}

fn invalidate(h: HWND) {
    // SAFETY: h là cửa sổ của luồng này.
    unsafe {
        let _ = InvalidateRect(Some(h), None, false);
    }
}

/// Vẽ thanh ghi.
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
            let status = &ui.status[usize::from(s.saving)];
            theme::buffered(dc, &full, |m| {
                let bw = scale(1, ui.dpi).max(1);
                theme::round_rect(m, &full, 0, SURFACE, LINE, bw);
                theme::ellipse(m, &to_rect(l.dot), DANGER, DANGER, 0);
                theme::draw_text(m, status, &to_rect(l.status), TEXT, ui.font, flags);
                theme::draw_text(m, &ui.time, &to_rect(l.time), TEXT_MUTED, ui.font, flags);
                for (i, btn) in BUTTONS.iter().enumerate() {
                    let enabled = !s.saving;
                    let state = ControlState {
                        hot: enabled && ui.hover == Some(*btn),
                        pressed: enabled && ui.pressed == Some(*btn) && ui.hover == Some(*btn),
                        focused: false,
                        enabled,
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

fn run(button: BarButton) {
    match button {
        BarButton::Stop => stop(),
        BarButton::Cancel => cancel(),
    }
}

unsafe extern "system" fn host_proc(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_TIMER if wp.0 == TIMER_ID => {
            on_tick();
            LRESULT(0)
        }
        // Tới sau khi phiên đã dọn (hoặc chưa có kết quả): `poll_done` bỏ qua.
        WM_RECORD_DONE => {
            poll_done();
            LRESULT(0)
        }
        WM_DISPLAYCHANGE => {
            on_display_change();
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
            scroll_session::paint_frame(h);
            LRESULT(0)
        }
        // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
        _ => unsafe { DefWindowProcW(h, msg, wp, lp) },
    }
}

unsafe extern "system" fn bar_proc(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        // Bấm vào thanh không kích hoạt nó: ứng dụng đang được ghi giữ tiêu điểm.
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            paint_bar(h);
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let p = client_point(lp);
            let grab = with(|s| {
                let b = hit(&s.ui.layout, p, s.saving);
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
                let now = hit(&s.ui.layout, p, s.saving);
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
                match (pressed, hit(&s.ui.layout, p, s.saving)) {
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
            on_display_change();
            LRESULT(0)
        }
        // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
        _ => unsafe { DefWindowProcW(h, msg, wp, lp) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    #[test]
    fn time_text_counts_whole_seconds_up_to_max() {
        assert_eq!(time_text(0, 15_000), "00:00 / 00:15");
        assert_eq!(time_text(5_000, 15_000), "00:05 / 00:15");
        assert_eq!(time_text(5_999, 15_000), "00:05 / 00:15");
        assert_eq!(time_text(14_999, 15_000), "00:14 / 00:15");
        assert_eq!(time_text(15_000, 15_000), "00:15 / 00:15");
        // Không vượt thời lượng tối đa.
        assert_eq!(time_text(16_400, 15_000), "00:15 / 00:15");
        assert_eq!(time_text(61_000, 120_000), "01:01 / 02:00");
        assert_eq!(time_text(0, gif_record::MAX_DURATION_MS), "00:00 / 00:15");
        // Mỗi giây một mẫu đo, từ 0 tới tối đa.
        assert_eq!(time_samples().len(), 16);
    }

    /// Đo chữ giả: 7 px mỗi ký tự, dòng 16 px (nhân theo DPI).
    fn fake(dpi: u32) -> impl Fn(&[&[u16]]) -> Vec<(i32, i32)> {
        move |parts| {
            parts
                .iter()
                .map(|t| (scale(7 * t.len() as i32, dpi), scale(16, dpi)))
                .collect()
        }
    }

    #[test]
    fn bar_layout_fits_text_at_every_dpi_in_both_languages() {
        for lang in Lang::ALL {
            for dpi in [96u32, 120, 144, 192] {
                let k = |v: i32| scale(v, dpi);
                let t = measure_texts(lang, fake(dpi));
                // Chỗ cho chữ dài hơn trong hai trạng thái, chuỗi thời gian và nhãn nút.
                let (status, labels) = labels(lang);
                let wf = |s: &[u16]| scale(7 * s.len() as i32, dpi);
                assert_eq!(t.status, wf(&status[0]).max(wf(&status[1])));
                assert_eq!(t.time, wf(&wide("00:15 / 00:15")));
                assert_eq!(t.buttons, [wf(&labels[0]), wf(&labels[1])]);
                assert_eq!(t.line, k(16));
                let l = layout(dpi, &t);
                let win = r(0, 0, l.size.0, l.size.1);
                let inside = |x: Rect| {
                    x.x >= 0 && x.y >= 0 && x.right() <= win.right() && x.bottom() <= win.bottom()
                };
                assert!(l.size.1 >= k(BAR_H));
                let order = [l.dot, l.status, l.time, l.buttons[0], l.buttons[1]];
                for part in order {
                    assert!(inside(part), "{lang:?} dpi {dpi} {part:?}");
                    assert!(part.w > 0 && part.h > 0);
                }
                for pair in order.windows(2) {
                    assert!(pair[0].right() <= pair[1].x, "{lang:?} dpi {dpi} {pair:?}");
                }
                assert!(l.status.w >= t.status && l.time.w >= t.time);
                for (b, tw) in l.buttons.iter().zip(t.buttons) {
                    assert!(b.w >= tw + 2 * k(BTN_PAD));
                    assert!(b.h >= t.line);
                }
                // Thanh vừa màn hình nhỏ (1024 px) ở mọi DPI thử.
                assert!(l.size.0 <= 1024, "{lang:?} dpi {dpi} {:?}", l.size);
            }
        }
    }

    #[test]
    fn bar_buttons_hit_and_are_disabled_while_saving() {
        for dpi in [96u32, 120, 144, 192] {
            let l = layout(dpi, &measure_texts(Lang::Vi, fake(dpi)));
            let c = |x: Rect| Point {
                x: x.x + x.w / 2,
                y: x.y + x.h / 2,
            };
            assert_eq!(hit(&l, c(l.buttons[0]), false), Some(BarButton::Stop));
            assert_eq!(hit(&l, c(l.buttons[1]), false), Some(BarButton::Cancel));
            assert_eq!(hit(&l, c(l.status), false), None);
            assert_eq!(hit(&l, c(l.time), false), None);
            assert_eq!(hit(&l, c(l.dot), false), None);
            // Mép: góc trên trái trong nút, ngay sau mép phải thì không.
            let b = l.buttons[1];
            assert_eq!(
                hit(&l, Point { x: b.x, y: b.y }, false),
                Some(BarButton::Cancel)
            );
            assert_eq!(
                hit(
                    &l,
                    Point {
                        x: b.right(),
                        y: b.y
                    },
                    false
                ),
                None
            );
            // Đang lưu: không nút nào nhận bấm.
            assert_eq!(hit(&l, c(l.buttons[0]), true), None);
            assert_eq!(hit(&l, c(l.buttons[1]), true), None);
        }
        assert_eq!(BUTTONS, [BarButton::Stop, BarButton::Cancel]);
        assert_eq!(BarButton::Stop.label(), Msg::RecordStop);
        assert_eq!(BarButton::Cancel.label(), Msg::RecordCancel);
    }

    /// Không có phiên: Dừng, Hủy, thoát, đổi ngôn ngữ và thông điệp xong muộn đều vô hại.
    #[test]
    fn calls_without_session_are_harmless() {
        assert!(!active());
        stop();
        cancel();
        stop();
        cancel();
        relabel();
        poll_done();
        on_tick();
        on_display_change();
        finish_on_exit(Instant::now());
        assert!(!active());
    }

    #[test]
    fn start_rejects_empty_and_invalid_regions_before_creating_windows() {
        let dir = std::env::temp_dir();
        assert_eq!(start(r(0, 0, 0, 10), dir.clone()), Err(StartError::Empty));
        assert_eq!(start(r(0, 0, 10, -1), dir.clone()), Err(StartError::Empty));
        // Nhỏ hơn MIN_SIDE.
        assert_eq!(
            start(r(-50, -50, 1, 100), dir.clone()),
            Err(StartError::Recorder(GifError::InvalidRegion))
        );
        // Quá MAX_SOURCE_PIXELS: báo ngay, không cấp phát khung nào.
        assert_eq!(
            start(r(0, 0, 20_000, 20_000), dir),
            Err(StartError::Recorder(GifError::TooLarge))
        );
        assert!(!active());
    }

    #[test]
    fn message_id_follows_used_range() {
        assert_eq!(WM_RECORD_DONE, WM_APP + 18);
    }
}

/// Chạy tay (tạo cửa sổ thật của kiểm thử; không dời chuột, không gửi phím):
/// `cargo test -p darkshot-windows record_session::live -- --ignored --test-threads=1`.
#[cfg(test)]
mod live {
    use super::*;
    use std::time::Duration;
    use windows::Win32::Graphics::Gdi::{CreateSolidBrush, FillRect, UpdateWindow};
    use windows::Win32::Graphics::GdiPlus::{
        GdipDisposeImage, GdipGetImageHeight, GdipGetImageWidth, GdipImageGetFrameCount,
        GdipImageGetFrameDimensionsCount, GdipImageGetFrameDimensionsList,
        GdipImageSelectActiveFrame, GdipLoadImageFromFile, GpImage,
    };
    use windows::Win32::System::Threading::{
        GetCurrentProcess, GetGuiResources, GR_GDIOBJECTS, GR_USEROBJECTS,
    };
    use windows::Win32::UI::HiDpi::{
        SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetClientRect, GetWindowDisplayAffinity, IsWindow, PeekMessageW,
        TranslateMessage, MSG, PM_REMOVE, SW_SHOWNOACTIVATE,
    };

    /// Màu nền của cửa sổ thử (BGR của COLORREF: đỏ tươi).
    const FILL: u32 = 0x00FF_00FF;
    const TEST_W: i32 = 720;
    const TEST_H: i32 = 400;

    thread_local! {
        static OUTCOMES: RefCell<Vec<Outcome>> = const { RefCell::new(Vec::new()) };
        /// Vị trí ô đen chạy ngang ở góc trên của cửa sổ thử (`None`: không có ô, chỉ một màu).
        static ANIM: Cell<Option<i32>> = const { Cell::new(None) };
    }

    fn keep(o: Outcome) {
        OUTCOMES.with(|v| v.borrow_mut().push(o));
    }

    fn pump_for(ms: u64) {
        let end = Instant::now() + Duration::from_millis(ms);
        while Instant::now() < end {
            let mut msg = MSG::default();
            // SAFETY: vòng bơm thông điệp chuẩn của luồng kiểm thử.
            unsafe {
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Bơm tới khi không còn phiên (tối đa `ms`).
    fn pump_until_idle(ms: u64) {
        let end = Instant::now() + Duration::from_millis(ms);
        while active() && Instant::now() < end {
            pump_for(20);
        }
    }

    unsafe extern "system" fn test_proc(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        if msg == WM_PAINT {
            let mut ps = PAINTSTRUCT::default();
            // SAFETY: cặp BeginPaint/EndPaint; chổi tạo và xóa trong khối.
            unsafe {
                let dc = BeginPaint(h, &mut ps);
                let mut cr = RECT::default();
                let _ = GetClientRect(h, &mut cr);
                let br = CreateSolidBrush(COLORREF(FILL));
                FillRect(dc, &cr, br);
                if let Some(x) = ANIM.with(|a| a.get()) {
                    let sq = RECT {
                        left: x,
                        top: 10,
                        right: x + 40,
                        bottom: 50,
                    };
                    let black = CreateSolidBrush(COLORREF(0));
                    FillRect(dc, &sq, black);
                    let _ = DeleteObject(black.into());
                }
                let _ = DeleteObject(br.into());
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

    /// Cửa sổ thử một màu, luôn trên cùng, không lấy tiêu điểm; trả cửa sổ và vùng client.
    fn test_window() -> (HWND, Rect) {
        let ex = WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;
        // SAFETY: đăng ký lớp và tạo cửa sổ popup của luồng kiểm thử.
        unsafe {
            let hinst = GetModuleHandleW(None).unwrap();
            let class = w!("darkshot.record-test");
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
                w!("record test"),
                WS_POPUP,
                240,
                160,
                TEST_W,
                TEST_H,
                None,
                None,
                Some(hinst.into()),
                None,
            )
            .unwrap();
            let _ = ShowWindow(h, SW_SHOWNOACTIVATE);
            let _ = UpdateWindow(h);
            let mut wr = RECT::default();
            let _ = GetWindowRect(h, &mut wr);
            (h, r(wr.left, wr.top, TEST_W, TEST_H))
        }
    }

    fn r(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    fn dpi_aware() {
        // SAFETY: đặt chế độ DPI cho luồng kiểm thử (như manifest của ứng dụng).
        unsafe {
            SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
    }

    fn gui(kind: windows::Win32::System::Threading::GET_GUI_RESOURCES_FLAGS) -> u32 {
        // SAFETY: chỉ đọc bộ đếm của tiến trình hiện tại.
        unsafe { GetGuiResources(GetCurrentProcess(), kind) }
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "darkshot-record-{tag}-{}-{}",
            std::process::id(),
            Instant::now().elapsed().as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn files(d: &std::path::Path) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(d)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    }

    /// Thanh và khung loại khỏi ảnh chụp: thanh nằm trên phần dưới của cửa sổ thử, ảnh chụp vùng
    /// của thanh chỉ thấy màu cửa sổ thử. Hủy: không có tệp nào, cửa sổ đã hủy, không rò GDI/USER.
    #[test]
    #[ignore]
    fn bar_is_excluded_from_capture_and_cancel_leaves_nothing() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        dpi_aware();
        set_on_outcome(keep);
        let (tw, win) = test_window();
        pump_for(300);
        // Vùng ghi: nửa trên cửa sổ thử; thanh (dưới vùng) nằm trên nửa dưới.
        let region = r(win.x, win.y, win.w, win.h / 2);
        let dir = temp_dir("excl");
        let supported = scroll_session::exclude_supported(scroll_session::os_build());
        let (mut gdi0, mut user0) = (0, 0);
        for i in 0..4 {
            if i == 1 {
                gdi0 = gui(GR_GDIOBJECTS);
                user0 = gui(GR_USEROBJECTS);
            }
            OUTCOMES.with(|v| v.borrow_mut().clear());
            assert_eq!(start(region, dir.clone()), Ok(()));
            assert!(active());
            assert_eq!(start(region, dir.clone()), Err(StartError::Busy));
            pump_for(300);
            let (bar, frames) = with(|s| (s.bar, s.frames.clone())).unwrap();
            let mut br = RECT::default();
            // SAFETY: bar là cửa sổ của phiên; br là biến cục bộ.
            unsafe {
                let _ = GetWindowRect(bar, &mut br);
            }
            let bar_rect = r(br.left, br.top, br.right - br.left, br.bottom - br.top);
            eprintln!("region {region:?} bar {bar_rect:?} frames {}", frames.len());
            if supported {
                assert_eq!(frames.len(), 4);
                for h in frames.iter().chain([&bar]) {
                    let mut aff = 0u32;
                    // SAFETY: h là cửa sổ của tiến trình này; aff là biến cục bộ.
                    unsafe { GetWindowDisplayAffinity(*h, &mut aff) }.unwrap();
                    assert_eq!(aff, WDA_EXCLUDEFROMCAPTURE.0);
                }
                // Thanh nằm trọn trên cửa sổ thử: ảnh chụp vùng của thanh chỉ có màu cửa sổ thử.
                assert!(
                    bar_rect.x >= win.x
                        && bar_rect.y >= win.y
                        && bar_rect.right() <= win.right()
                        && bar_rect.bottom() <= win.bottom(),
                    "{bar_rect:?} {win:?}"
                );
                // SAFETY: chờ DWM trình bày khung có thanh.
                unsafe {
                    let _ = DwmFlush();
                }
                let img = crate::capture::capture_rect(bar_rect).unwrap();
                let want = [0xFF, 0x00, 0xFF];
                let bad = img
                    .bgra
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .filter(|p| p[..3] != want)
                    .count();
                assert_eq!(bad, 0, "thanh lọt vào ảnh chụp: {bad} điểm ảnh khác màu");
                // Vùng ghi cũng chỉ có màu cửa sổ thử (khung nằm ngoài vùng).
                let img = crate::capture::capture_rect(region).unwrap();
                assert!(img.bgra.as_chunks::<4>().0.iter().all(|p| p[..3] == want));
            } else {
                // Không loại được: không khung, thanh ngoài vùng.
                assert!(frames.is_empty());
                assert!(
                    bar_rect.bottom() <= region.y
                        || bar_rect.y >= region.bottom()
                        || bar_rect.right() <= region.x
                        || bar_rect.x >= region.right()
                );
            }
            cancel();
            cancel();
            stop();
            pump_until_idle(5000);
            assert!(!active());
            assert_eq!(
                OUTCOMES.with(|v| v.borrow_mut().drain(..).collect::<Vec<_>>()),
                vec![Outcome::Cancelled]
            );
            for h in frames.iter().chain([&bar]) {
                // SAFETY: chỉ hỏi cửa sổ đã hủy có còn không.
                assert!(!unsafe { IsWindow(Some(*h)) }.as_bool());
            }
            // WM_RECORD_DONE tới muộn sau khi đã dọn: bỏ qua.
            poll_done();
            assert!(files(&dir).is_empty(), "{:?}", files(&dir));
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
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Đọc GIF bằng GDI+: (rộng, cao, số khung); giải mã cả khung cuối.
    fn decode_gif(path: &std::path::Path) -> Option<(u32, u32, u32)> {
        let gp = crate::gdiplus::GdiPlus::start();
        assert!(gp.ok());
        let wpath = HSTRING::from(path.as_os_str());
        let mut img: *mut GpImage = std::ptr::null_mut();
        // SAFETY: đường dẫn là chuỗi rộng kết thúc 0, sống suốt lời gọi; ảnh được xóa bên dưới.
        unsafe {
            if GdipLoadImageFromFile(PCWSTR(wpath.as_ptr()), &mut img).0 != 0 || img.is_null() {
                return None;
            }
            let (mut w, mut h) = (0u32, 0u32);
            GdipGetImageWidth(img, &mut w);
            GdipGetImageHeight(img, &mut h);
            let mut dims = 0u32;
            GdipImageGetFrameDimensionsCount(img, &mut dims);
            let mut frames = 0u32;
            if dims > 0 {
                let mut ids = vec![windows::core::GUID::zeroed(); dims as usize];
                GdipImageGetFrameDimensionsList(img, ids.as_mut_ptr(), dims);
                GdipImageGetFrameCount(img, &ids[0], &mut frames);
                let ok = GdipImageSelectActiveFrame(img, &ids[0], frames.saturating_sub(1));
                if ok.0 != 0 {
                    frames = 0;
                }
            }
            GdipDisposeImage(img);
            Some((w, h, frames))
        }
    }

    /// Một lượt ghi thật ~1 s: Dừng → Saved, GIF giải mã được (đúng cỡ, nhiều khung), không còn
    /// tệp `.part`; rồi xóa.
    #[test]
    #[ignore]
    fn one_second_recording_saves_a_decodable_gif() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        dpi_aware();
        set_on_outcome(keep);
        OUTCOMES.with(|v| v.borrow_mut().clear());
        let (tw, win) = test_window();
        pump_for(300);
        let region = r(win.x, win.y, win.w, win.h / 2);
        let dir = temp_dir("rec");
        assert_eq!(start(region, dir.clone()), Ok(()));
        // Nội dung đổi mỗi 50 ms (ô đen chạy ngang) để GIF có nhiều khung khác nhau.
        let t0 = Instant::now();
        while t0.elapsed() < Duration::from_millis(1100) {
            ANIM.with(|a| a.set(Some((t0.elapsed().as_millis() as i32 / 50) * 20 % 600)));
            // SAFETY: vẽ lại cửa sổ thử của chính kiểm thử.
            unsafe {
                let _ = InvalidateRect(Some(tw), None, false);
                let _ = UpdateWindow(tw);
            }
            pump_for(50);
        }
        ANIM.with(|a| a.set(None));
        stop();
        stop();
        cancel(); // Hủy sau Dừng: không còn tác dụng.
        pump_until_idle(5000);
        assert!(!active());
        let got = OUTCOMES.with(|v| v.borrow_mut().drain(..).collect::<Vec<_>>());
        assert_eq!(got.len(), 1, "{got:?}");
        let Outcome::Saved {
            path,
            reason,
            frames,
        } = got[0].clone()
        else {
            panic!("{got:?}");
        };
        eprintln!("{} frames {frames} reason {reason:?}", path.display());
        assert_eq!(reason, gif_record::StopReason::Stopped);
        assert!(frames >= 1);
        assert_eq!(path.parent(), Some(dir.as_path()));
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        assert!(
            name.starts_with("darkshot_") && name.ends_with(".gif"),
            "{name}"
        );
        let (w, h, n) = decode_gif(&path).expect("GIF giải mã được");
        let (fw, fh) = gif_record::frame_size(region.w as u32, region.h as u32).unwrap();
        assert_eq!((w, h), (u32::from(fw), u32::from(fh)));
        // ~10 khung/giây trong ~1 s, nội dung đổi liên tục (máy chậm thì ít hơn).
        assert!(n >= 3, "{n}");
        assert_eq!(n, frames);
        // Chỉ còn đúng tệp GIF, không có tệp tạm.
        assert_eq!(files(&dir), vec![name]);
        // SAFETY: hủy cửa sổ thử của chính kiểm thử.
        unsafe {
            let _ = DestroyWindow(tw);
        }
        pump_for(50);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Thoát giữa lúc ghi: `finish_on_exit` lưu phần đã ghi trong hạn chót, dọn cửa sổ, không gọi
    /// hàm kết quả.
    #[test]
    #[ignore]
    fn exit_saves_partial_recording() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        dpi_aware();
        set_on_outcome(keep);
        OUTCOMES.with(|v| v.borrow_mut().clear());
        let (tw, win) = test_window();
        pump_for(300);
        let region = r(win.x, win.y, win.w, win.h / 2);
        let dir = temp_dir("exit");
        assert_eq!(start(region, dir.clone()), Ok(()));
        let bar = with(|s| s.bar).unwrap();
        pump_for(500);
        finish_on_exit(Instant::now() + Duration::from_secs(4));
        assert!(!active());
        // SAFETY: chỉ hỏi cửa sổ đã hủy có còn không.
        assert!(!unsafe { IsWindow(Some(bar)) }.as_bool());
        pump_for(100);
        assert!(OUTCOMES.with(|v| v.borrow().is_empty()));
        let names = files(&dir);
        assert_eq!(names.len(), 1, "{names:?}");
        assert!(names[0].ends_with(".gif"));
        assert!(decode_gif(&dir.join(&names[0])).is_some());
        // SAFETY: hủy cửa sổ thử của chính kiểm thử.
        unsafe {
            let _ = DestroyWindow(tw);
        }
        pump_for(50);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
