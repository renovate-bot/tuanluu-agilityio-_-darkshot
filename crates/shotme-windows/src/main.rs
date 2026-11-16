#![windows_subsystem = "windows"]

// Task 10 và 11 mới nối autostart, clipboard, secret vào luồng chạy; ở task này chỉ có kiểm thử gọi chúng.
#[allow(dead_code)]
mod app;
mod autostart;
mod capture;
mod clipboard;
mod hotkey;
mod overlay;
mod paths;
mod secret;
mod settings;
mod tray;

use app::{AppState, UploadJob};
use secret::DpapiStore;
use shotme_core::actions::{run_action, Action, Clipboard, Ctx, NoticeKind, Notifier};
use shotme_core::config::Hotkey;
use shotme_core::upload::{R2Uploader, Uploader};
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::rc::Rc;
use std::sync::Arc;
use tray::{MenuCommand, Notice, Tray, WM_NOTICE, WM_TRAY};
use windows::core::w;
use windows::Win32::Foundation::{
    GetLastError, ERROR_ALREADY_EXISTS, HWND, LPARAM, LRESULT, WPARAM,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, EndMenu, GetMessageW,
    KillTimer, PostMessageW, PostQuitMessage, RegisterClassW, RegisterWindowMessageW, SetTimer,
    TranslateMessage, CW_USEDEFAULT, MSG, WM_APP, WM_DESTROY, WM_HOTKEY, WM_LBUTTONUP,
    WM_RBUTTONUP, WM_TIMER, WNDCLASSW, WS_EX_TOOLWINDOW, WS_OVERLAPPED,
};

/// Sự kiện nội bộ; Task 11 mở rộng.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppEvent {
    Capture,
    OpenSettings,
    Quit,
}

/// Cổng chụp: bảo đảm không bao giờ có hai lớp phủ và menu khay không chen vào lúc chờ chụp.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureGate {
    Idle,
    /// Đã chọn "Chụp màn hình" ở menu, đang chờ menu tắt hẳn.
    PendingMenu,
    /// Đang chụp hoặc lớp phủ đang mở.
    Capturing,
    /// Cửa sổ Cài đặt đang mở.
    SettingsOpen,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateEvent {
    MenuCapture,
    Hotkey,
    TimerFired,
    OverlayClosed,
    TrayClick,
    OpenSettings,
    SettingsClosed,
}

/// Việc bên gọi phải làm sau một sự kiện.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateAction {
    /// Không có gì đặc biệt (với `TrayClick`: mở menu như thường).
    None,
    StartTimer,
    /// Hủy bộ hẹn giờ của menu (nếu có) rồi chụp ngay.
    CaptureNow,
    /// Mở cửa sổ Cài đặt.
    OpenSettings,
    Ignore,
}

/// Hàm chuyển trạng thái thuần của cổng chụp.
pub fn gate_step(state: CaptureGate, ev: GateEvent) -> (CaptureGate, GateAction) {
    use CaptureGate::*;
    use GateEvent::*;
    match (state, ev) {
        (Idle, MenuCapture) => (PendingMenu, GateAction::StartTimer),
        (Idle | PendingMenu, Hotkey) | (PendingMenu, TimerFired) => {
            (Capturing, GateAction::CaptureNow)
        }
        (Idle, TrayClick) => (Idle, GateAction::None),
        (Idle, OpenSettings) => (SettingsOpen, GateAction::OpenSettings),
        (SettingsOpen, SettingsClosed) => (Idle, GateAction::None),
        (s, SettingsClosed) => (s, GateAction::None),
        (Capturing, OverlayClosed) => (Idle, GateAction::None),
        (s, OverlayClosed) => (s, GateAction::None),
        (s, _) => (s, GateAction::Ignore),
    }
}

thread_local! {
    static TRAY: RefCell<Option<Rc<Tray>>> = const { RefCell::new(None) };
    static TASKBAR_CREATED: Cell<u32> = const { Cell::new(0) };
    static ADD_ATTEMPTS: Cell<u32> = const { Cell::new(0) };
    static APP: RefCell<Option<AppState>> = const { RefCell::new(None) };
    static GATE: Cell<CaptureGate> = const { Cell::new(CaptureGate::Idle) };
    /// Menu khay đang hiện.
    static MENU_OPEN: Cell<bool> = const { Cell::new(false) };
}

/// Luồng nền (hoặc lúc khởi động) nhờ luồng chính mở Cài đặt.
const WM_OPEN_SETTINGS: u32 = WM_APP + 3;
const RETRY_TIMER_ID: usize = 1;
const MENU_CAPTURE_TIMER_ID: usize = 2;
const MENU_CAPTURE_DELAY_MS: u32 = 250;
const RETRY_INTERVAL_MS: u32 = 2000;

fn on_retry_timer(hwnd: HWND) {
    let attempts = ADD_ATTEMPTS.with(|c| c.get());
    let added = match current_tray() {
        Some(t) => {
            if tray::should_retry(t.is_added(), attempts) {
                ADD_ATTEMPTS.with(|c| c.set(attempts + 1));
                t.try_add()
            } else {
                t.is_added()
            }
        }
        None => true,
    };
    let attempts = ADD_ATTEMPTS.with(|c| c.get());
    if !tray::should_retry(added, attempts) {
        // SAFETY: hủy bộ hẹn giờ của chính cửa sổ này.
        unsafe {
            let _ = KillTimer(Some(hwnd), RETRY_TIMER_ID);
        }
    }
}

fn current_tray() -> Option<Rc<Tray>> {
    TRAY.with(|t| t.borrow().clone())
}

fn post_open_settings(hwnd: isize) {
    // SAFETY: PostMessageW an toàn từ luồng bất kỳ; cửa sổ đã hủy thì lỗi và ta bỏ qua.
    unsafe {
        let _ = PostMessageW(
            Some(HWND(hwnd as *mut c_void)),
            WM_OPEN_SETTINGS,
            WPARAM(0),
            LPARAM(0),
        );
    }
}

fn with_app<R>(f: impl FnOnce(&mut AppState) -> R) -> Option<R> {
    APP.with(|a| a.borrow_mut().as_mut().map(f))
}

/// Chụp, mở lớp phủ rồi thực hiện hành động đã chọn.
/// Chỉ gọi qua `gate` (trạng thái `Capturing`), nên không bao giờ có hai lớp phủ.
fn capture_flow(hwnd: HWND) {
    let notifier: Option<Arc<dyn Notifier>> =
        current_tray().map(|t| Arc::new(t.notifier()) as Arc<dyn Notifier>);
    let notify = |kind: NoticeKind, title: &str, body: &str| {
        if let Some(n) = &notifier {
            n.notify(kind, title, body);
        }
    };
    let shot = match capture::capture_virtual_screen() {
        Ok(s) => s,
        Err(_) => {
            notify(
                NoticeKind::Error,
                "Không chụp được màn hình",
                "Windows không cho chụp lúc này (ví dụ màn hình khóa hoặc hộp thoại quyền quản trị). Hãy thử lại.",
            );
            return;
        }
    };
    with_app(|a| a.overlay_open = true);
    let picked = overlay::pick_region(&shot);
    with_app(|a| a.overlay_open = false);
    let (action, rect) = match picked {
        Ok(Some(p)) => p,
        Ok(None) => return,
        Err(e) => {
            notify(
                NoticeKind::Error,
                "Không mở được lớp phủ chọn vùng",
                &format!("Lý do: {e}. Hãy thử lại."),
            );
            return;
        }
    };
    let image = shot.image.crop(rect);
    // Giải phóng ảnh chụp ngay khi đã cắt xong.
    drop(shot);
    let (Some(notifier), Some(config)) = (notifier, with_app(|a| a.config.clone())) else {
        return;
    };
    let clipboard: Arc<dyn Clipboard> = Arc::new(clipboard::WinClipboard);
    match action {
        Action::Cancel => {}
        Action::Save | Action::Copy => {
            let ctx = Ctx {
                save_dir: &config.save_dir,
                expiry: config.expiry,
                now: paths::local_now(),
                uploader: None,
                clipboard: clipboard.as_ref(),
                notifier: notifier.as_ref(),
            };
            run_action(&ctx, action, &image);
        }
        Action::Upload => {
            let uploader: Option<Arc<dyn Uploader>> = if config.r2.is_complete() {
                Some(Arc::new(R2Uploader::new(&config.r2)))
            } else {
                None
            };
            let image = Arc::new(image);
            let hwnd_raw = hwnd.0 as isize;
            let job = UploadJob {
                save_dir: config.save_dir.clone(),
                expiry: config.expiry,
                uploader,
                clipboard: clipboard.clone(),
                notifier: notifier.clone(),
                image: image.clone(),
                clock: paths::local_now,
                on_needs_settings: Box::new(move || post_open_settings(hwnd_raw)),
            };
            if let Err(e) = app::spawn_upload(job) {
                // Không tạo được luồng: lưu ảnh ngay thay vì mất nó.
                notifier.notify(
                    NoticeKind::Error,
                    "Không bắt đầu được việc tải lên",
                    &format!("{e}. Ảnh sẽ được lưu vào máy."),
                );
                let ctx = Ctx {
                    save_dir: &config.save_dir,
                    expiry: config.expiry,
                    now: paths::local_now(),
                    uploader: None,
                    clipboard: clipboard.as_ref(),
                    notifier: notifier.as_ref(),
                };
                run_action(&ctx, Action::Save, &image);
            }
        }
    }
}

/// Mở Cài đặt, lưu kết quả. Chỉ gọi qua `gate` (trạng thái `SettingsOpen`).
fn open_settings(hwnd: HWND) {
    let Some((config, notice)) = with_app(|a| (a.config.clone(), a.config_notice.take())) else {
        return;
    };
    let Some(result) = settings::show(hwnd, &config, notice.as_deref()) else {
        return;
    };
    let notify = |kind: NoticeKind, title: &str, body: &str| {
        if let Some(t) = current_tray() {
            t.notifier().notify(kind, title, body);
        }
    };
    // `show` đã đăng ký phím mới cho hwnd; ở đây chỉ ghi đĩa.
    let new = result.config;
    match shotme_core::config::save(&paths::config_path(), &new, &DpapiStore) {
        Ok(()) => {
            if let Err(e) = autostart::set_enabled(new.autostart) {
                notify(
                    NoticeKind::Error,
                    "Không đổi được khởi động cùng Windows",
                    &format!("Lý do: {e}"),
                );
            }
            with_app(|a| a.config = new);
        }
        Err(e) => {
            // Không ghi được: giữ cấu hình cũ và phím cũ.
            hotkey::unregister(hwnd);
            let _ = hotkey::register(hwnd, &config.hotkey);
            let why = match e {
                shotme_core::config::ConfigError::Io(m)
                | shotme_core::config::ConfigError::Corrupt(m)
                | shotme_core::config::ConfigError::Secret(m) => m,
            };
            notify(
                NoticeKind::Error,
                "Không lưu được cài đặt",
                &format!("Lý do: {why}. Cài đặt cũ vẫn được dùng."),
            );
        }
    }
}

/// Đưa một sự kiện qua cổng chụp và làm việc mà cổng yêu cầu.
fn gate(hwnd: HWND, ev: GateEvent) -> GateAction {
    let (next, act) = gate_step(GATE.with(|g| g.get()), ev);
    GATE.with(|g| g.set(next));
    match act {
        GateAction::StartTimer => {
            // Chờ menu khay tắt hẳn (hiệu ứng mờ dần) để nó không lọt vào ảnh.
            // SAFETY: bộ hẹn giờ gắn với cửa sổ của luồng này; WM_TIMER xử lý ở wndproc.
            let timer = unsafe {
                SetTimer(
                    Some(hwnd),
                    MENU_CAPTURE_TIMER_ID,
                    MENU_CAPTURE_DELAY_MS,
                    None,
                )
            };
            if timer == 0 {
                // Không đặt được hẹn giờ: chụp ngay thay vì để cổng kẹt ở PendingMenu.
                return gate(hwnd, GateEvent::TimerFired);
            }
        }
        GateAction::CaptureNow => {
            // SAFETY: hủy bộ hẹn giờ của menu (nếu có) của chính cửa sổ này.
            unsafe {
                let _ = KillTimer(Some(hwnd), MENU_CAPTURE_TIMER_ID);
            }
            capture_flow(hwnd);
            let (next, _) = gate_step(GATE.with(|g| g.get()), GateEvent::OverlayClosed);
            GATE.with(|g| g.set(next));
        }
        GateAction::OpenSettings => {
            open_settings(hwnd);
            let (next, _) = gate_step(GATE.with(|g| g.get()), GateEvent::SettingsClosed);
            GATE.with(|g| g.set(next));
        }
        GateAction::None | GateAction::Ignore => {}
    }
    act
}

fn handle_event(hwnd: HWND, ev: AppEvent) {
    match ev {
        AppEvent::Capture => {
            gate(hwnd, GateEvent::Hotkey);
        }
        AppEvent::OpenSettings => {
            gate(hwnd, GateEvent::OpenSettings);
        }
        AppEvent::Quit => {
            // SAFETY: hwnd là cửa sổ của luồng này.
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
        }
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let tb = TASKBAR_CREATED.with(|c| c.get());
    if tb != 0 && msg == tb {
        if let Some(t) = current_tray() {
            t.readd();
        }
        return LRESULT(0);
    }
    match msg {
        WM_HOTKEY if wp.0 as i32 == hotkey::HOTKEY_ID => {
            if MENU_OPEN.with(|m| m.get()) {
                // Menu khay đang mở: đóng nó rồi chụp sau khi menu tắt hẳn, như chọn từ menu.
                // SAFETY: đóng menu đang hiện của luồng này.
                unsafe {
                    let _ = EndMenu();
                }
                gate(hwnd, GateEvent::MenuCapture);
            } else {
                handle_event(hwnd, AppEvent::Capture);
            }
            LRESULT(0)
        }
        WM_TRAY => {
            let ev = (lp.0 & 0xFFFF) as u32;
            if (ev == WM_RBUTTONUP || ev == WM_LBUTTONUP)
                && gate(hwnd, GateEvent::TrayClick) == GateAction::None
            {
                MENU_OPEN.with(|m| m.set(true));
                let cmd = current_tray().and_then(|t| t.show_menu());
                MENU_OPEN.with(|m| m.set(false));
                match cmd {
                    Some(MenuCommand::Capture) => {
                        gate(hwnd, GateEvent::MenuCapture);
                    }
                    Some(MenuCommand::Settings) => handle_event(hwnd, AppEvent::OpenSettings),
                    Some(MenuCommand::Quit) => handle_event(hwnd, AppEvent::Quit),
                    None => {}
                }
            }
            LRESULT(0)
        }
        WM_OPEN_SETTINGS => {
            handle_event(hwnd, AppEvent::OpenSettings);
            LRESULT(0)
        }
        WM_NOTICE => {
            // SAFETY: con trỏ do TrayNotifier::notify cấp phát và chuyển quyền sở hữu cho ta.
            let notice = unsafe { Box::from_raw(lp.0 as *mut Notice) };
            if let Some(t) = current_tray() {
                t.show_notice(&notice);
            }
            LRESULT(0)
        }
        WM_TIMER if wp.0 == MENU_CAPTURE_TIMER_ID => {
            // SAFETY: hủy bộ hẹn giờ một lần của chính cửa sổ này.
            unsafe {
                let _ = KillTimer(Some(hwnd), MENU_CAPTURE_TIMER_ID);
            }
            gate(hwnd, GateEvent::TimerFired);
            LRESULT(0)
        }
        WM_TIMER if wp.0 == RETRY_TIMER_ID => {
            on_retry_timer(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            hotkey::unregister(hwnd);
            TRAY.with(|t| t.borrow_mut().take());
            // SAFETY: kết thúc vòng lặp thông điệp của luồng này.
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
        _ => unsafe { DefWindowProcW(hwnd, msg, wp, lp) },
    }
}

fn create_window() -> windows::core::Result<HWND> {
    // SAFETY: đăng ký lớp và tạo cửa sổ ẩn (không bao giờ ShowWindow) ở luồng chính.
    unsafe {
        let hinst = GetModuleHandleW(None)?;
        let class = w!("shotme-hidden-window");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: hinst.into(),
            lpszClassName: class,
            ..Default::default()
        };
        // Lớp đã tồn tại không xảy ra vì chỉ có một bản chạy; lỗi thật sẽ lộ ra ở CreateWindowExW.
        RegisterClassW(&wc);
        CreateWindowExW(
            WS_EX_TOOLWINDOW,
            class,
            w!("shotme"),
            WS_OVERLAPPED,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            0,
            0,
            None,
            None,
            Some(hinst.into()),
            None,
        )
    }
}

fn main() {
    // SAFETY: mutex được giữ đến khi tiến trình thoát (biến `_instance`).
    let _instance = unsafe {
        let h = CreateMutexW(None, false, w!("Local\\shotme-single-instance"));
        if h.is_ok() && GetLastError() == ERROR_ALREADY_EXISTS {
            return;
        }
        h
    };

    let Ok(hwnd) = create_window() else { return };
    // SAFETY: chuỗi hằng kết thúc bằng 0.
    TASKBAR_CREATED.with(|c| c.set(unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) }));

    let Ok(tray) = Tray::new(hwnd) else {
        // SAFETY: hwnd vừa tạo ở luồng này.
        unsafe {
            let _ = DestroyWindow(hwnd);
        }
        return;
    };
    let notifier = tray.notifier();
    if !tray.is_added() {
        // SAFETY: bộ hẹn giờ gắn với cửa sổ của luồng này; WM_TIMER xử lý ở wndproc.
        unsafe {
            SetTimer(Some(hwnd), RETRY_TIMER_ID, RETRY_INTERVAL_MS, None);
        }
    }
    TRAY.with(|t| *t.borrow_mut() = Some(Rc::new(tray)));

    let config_path = paths::config_path();
    let state = app::startup(&config_path, &DpapiStore, &paths::default_save_dir());
    let open_settings_now = app::needs_settings_on_start(&state, config_path.exists());
    let hk: Hotkey = state.config.hotkey;
    APP.with(|a| *a.borrow_mut() = Some(state));
    // Ghi lại đường dẫn exe hiện tại phòng khi file đã được chuyển chỗ; lỗi thì bỏ qua.
    if autostart::is_enabled() {
        let _ = autostart::set_enabled(true);
    }
    if !hotkey::register(hwnd, &hk) {
        let name = hotkey::describe(&hk);
        notifier.notify(
            NoticeKind::Error,
            &format!("Phím {name} đang bị ứng dụng khác chiếm"),
            "Hãy đổi phím tắt trong Cài đặt.",
        );
    }

    if open_settings_now {
        post_open_settings(hwnd.0 as isize);
    }

    let mut msg = MSG::default();
    // SAFETY: vòng lặp thông điệp chuẩn; GetMessageW trả về 0 (WM_QUIT) hoặc -1 (lỗi) thì dừng.
    unsafe {
        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use CaptureGate::*;
    use GateEvent::*;

    #[test]
    fn capture_gate_transitions() {
        assert_eq!(
            gate_step(Idle, MenuCapture),
            (PendingMenu, GateAction::StartTimer)
        );
        assert_eq!(
            gate_step(PendingMenu, TrayClick),
            (PendingMenu, GateAction::Ignore)
        );
        assert_eq!(
            gate_step(PendingMenu, Hotkey),
            (Capturing, GateAction::CaptureNow)
        );
        assert_eq!(
            gate_step(PendingMenu, TimerFired),
            (Capturing, GateAction::CaptureNow)
        );
        assert_eq!(
            gate_step(Capturing, Hotkey),
            (Capturing, GateAction::Ignore)
        );
        assert_eq!(
            gate_step(Capturing, TimerFired),
            (Capturing, GateAction::Ignore)
        );
        assert_eq!(
            gate_step(Capturing, OverlayClosed),
            (Idle, GateAction::None)
        );
        // Thêm: phím tắt khi rảnh chụp ngay; bấm khay khi rảnh mở menu như thường;
        // menu và khay khi đang chụp bị bỏ qua; hẹn giờ cũ khi rảnh bị bỏ qua.
        assert_eq!(gate_step(Idle, Hotkey), (Capturing, GateAction::CaptureNow));
        assert_eq!(gate_step(Idle, TrayClick), (Idle, GateAction::None));
        assert_eq!(
            gate_step(Capturing, TrayClick),
            (Capturing, GateAction::Ignore)
        );
        assert_eq!(
            gate_step(Capturing, MenuCapture),
            (Capturing, GateAction::Ignore)
        );
        assert_eq!(gate_step(Idle, TimerFired), (Idle, GateAction::Ignore));
        assert_eq!(
            gate_step(PendingMenu, MenuCapture),
            (PendingMenu, GateAction::Ignore)
        );
    }

    #[test]
    fn settings_gate_transitions() {
        assert_eq!(
            gate_step(Idle, OpenSettings),
            (SettingsOpen, GateAction::OpenSettings)
        );
        // Đang mở Cài đặt: bỏ qua mọi yêu cầu chụp và mở thêm.
        for ev in [MenuCapture, Hotkey, TimerFired, TrayClick, OpenSettings] {
            assert_eq!(
                gate_step(SettingsOpen, ev),
                (SettingsOpen, GateAction::Ignore),
                "{ev:?}"
            );
        }
        assert_eq!(
            gate_step(SettingsOpen, OverlayClosed),
            (SettingsOpen, GateAction::None)
        );
        assert_eq!(
            gate_step(SettingsOpen, SettingsClosed),
            (Idle, GateAction::None)
        );
        // Đang chụp hoặc chờ menu: không mở Cài đặt.
        assert_eq!(
            gate_step(Capturing, OpenSettings),
            (Capturing, GateAction::Ignore)
        );
        assert_eq!(
            gate_step(PendingMenu, OpenSettings),
            (PendingMenu, GateAction::Ignore)
        );
        // Đóng Cài đặt khi không ở trạng thái đó: không đổi.
        assert_eq!(gate_step(Idle, SettingsClosed), (Idle, GateAction::None));
        assert_eq!(
            gate_step(Capturing, SettingsClosed),
            (Capturing, GateAction::None)
        );
    }
}
