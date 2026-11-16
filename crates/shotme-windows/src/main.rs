#![windows_subsystem = "windows"]

mod about;
mod app;
mod autostart;
mod capture;
mod clipboard;
mod help;
mod hotkey;
mod lang;
mod overlay;
mod paths;
mod secret;
mod settings;
mod tray;
mod wer;
mod winutil;

use app::{AppState, UploadJob};
use secret::DpapiStore;
use settings::OpenReason;
use shotme_core::actions::{
    notice_visible, run_action, Action, Clipboard, Ctx, NoticeKind, Notifier,
};
use shotme_core::config::{Config, HotkeyRole, Hotkeys};
use shotme_core::geometry::Rect;
use shotme_core::i18n::{self, Lang, Msg};
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
    TranslateMessage, CW_USEDEFAULT, MSG, WM_APP, WM_DESTROY, WM_ENDSESSION, WM_HOTKEY,
    WM_LBUTTONUP, WM_QUERYENDSESSION, WM_RBUTTONUP, WM_TIMER, WNDCLASSW, WS_EX_TOOLWINDOW,
    WS_OVERLAPPED,
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
    /// Như `PendingMenu`, kèm yêu cầu mở Cài đặt đang chờ.
    PendingMenuSettings,
    /// Như `Capturing`, kèm yêu cầu mở Cài đặt đang chờ.
    CapturingSettings,
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
    /// Đưa cửa sổ Cài đặt đang mở ra trước.
    FocusSettings,
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
        (PendingMenuSettings, Hotkey | TimerFired) => (CapturingSettings, GateAction::CaptureNow),
        (Idle, TrayClick) => (Idle, GateAction::None),
        (Idle, OpenSettings) => (SettingsOpen, GateAction::OpenSettings),
        // Yêu cầu mở Cài đặt đến lúc đang chụp hoặc chờ menu: nhớ lại, mở sau khi lớp phủ đóng.
        (PendingMenu | PendingMenuSettings, OpenSettings) => {
            (PendingMenuSettings, GateAction::Ignore)
        }
        (Capturing | CapturingSettings, OpenSettings) => (CapturingSettings, GateAction::Ignore),
        // Cài đặt đang mở: mọi yêu cầu từ người dùng (kể cả chụp) đưa cửa sổ ra trước để nó không bị khuất.
        (SettingsOpen, TrayClick | OpenSettings | Hotkey | MenuCapture) => {
            (SettingsOpen, GateAction::FocusSettings)
        }
        (SettingsOpen, SettingsClosed) => (Idle, GateAction::None),
        (s, SettingsClosed) => (s, GateAction::None),
        (Capturing, OverlayClosed) => (Idle, GateAction::None),
        (CapturingSettings, OverlayClosed) => (SettingsOpen, GateAction::OpenSettings),
        (s, OverlayClosed) => (s, GateAction::None),
        (s, _) => (s, GateAction::Ignore),
    }
}

/// Loại của một lượt chụp: chụp vùng (lớp phủ) hoặc một trong hai kiểu toàn màn hình.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureKind {
    Region,
    SaveFull,
    UploadFull,
}

/// Loại chụp của một dòng phím tắt.
pub fn kind_of(role: HotkeyRole) -> CaptureKind {
    todo!()
}

/// Loại chụp sau một bước của cổng: chỉ yêu cầu được nhận (`StartTimer`, `CaptureNow`) mới đổi loại.
pub fn next_kind(current: CaptureKind, requested: CaptureKind, act: GateAction) -> CaptureKind {
    todo!()
}

/// Khung nhớ (tọa độ màn hình) sau một lượt chụp vùng kết thúc bằng `action` với khung `picked`:
/// tải lên, lưu, chép thì nhớ khung mới; hủy thì giữ khung cũ.
pub fn remember_after(prev: Option<Rect>, action: Action, picked: Rect) -> Option<Rect> {
    todo!()
}

/// Gộp lý do của hai yêu cầu mở Tùy chọn đang chờ: `NeedsR2` thắng.
pub fn merge_reason(a: OpenReason, b: OpenReason) -> OpenReason {
    if a == OpenReason::NeedsR2 || b == OpenReason::NeedsR2 {
        OpenReason::NeedsR2
    } else {
        OpenReason::User
    }
}

/// Ngôn ngữ sau khi đóng Tùy chọn: chỉ đổi khi đã ghi cấu hình thành công.
pub fn lang_after_save(old: Lang, new: Lang, saved: bool) -> Lang {
    if saved {
        new
    } else {
        old
    }
}

/// Chữ trong cửa sổ Thông tin và Trợ giúp đã cũ sau khi lưu cấu hình mới (ngôn ngữ hoặc phím tắt đổi).
pub fn info_windows_stale(old: &Config, new: &Config) -> bool {
    old.language != new.language || old.hotkeys != new.hotkeys
}

/// Sự kiện chuột `ev` (từ `WM_TRAY`) có mở menu khay không. Menu đang mở thì không mở lồng.
pub fn tray_click_opens_menu(ev: u32, menu_open: bool) -> bool {
    !menu_open && (ev == WM_RBUTTONUP || ev == WM_LBUTTONUP)
}

thread_local! {
    static TRAY: RefCell<Option<Rc<Tray>>> = const { RefCell::new(None) };
    static TASKBAR_CREATED: Cell<u32> = const { Cell::new(0) };
    static ADD_ATTEMPTS: Cell<u32> = const { Cell::new(0) };
    static APP: RefCell<Option<AppState>> = const { RefCell::new(None) };
    static GATE: Cell<CaptureGate> = const { Cell::new(CaptureGate::Idle) };
    /// Lý do của yêu cầu mở Tùy chọn đang chờ (gộp bằng `merge_reason`); `run_settings` lấy ra.
    static OPEN_REASON: Cell<OpenReason> = const { Cell::new(OpenReason::User) };
    /// Menu khay đang hiện.
    static MENU_OPEN: Cell<bool> = const { Cell::new(false) };
    /// Sổ các lượt tải lên đang chạy; luồng tải lên giữ bản sao `Arc`.
    static PENDING: Arc<app::PendingUploads> = Arc::new(app::PendingUploads::new());
}

/// Lưu vào máy ảnh của mọi lượt tải lên chưa xong. Chạy trên luồng chính, chỉ ghi file, không mạng.
fn save_pending_uploads() {
    let items = PENDING.with(|p| p.drain());
    if !items.is_empty() {
        app::save_pending(items, paths::local_now());
    }
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

/// Ghi lý do của một yêu cầu mở Tùy chọn, gộp với yêu cầu đang chờ.
fn note_open_reason(reason: OpenReason) {
    OPEN_REASON.with(|r| r.set(merge_reason(r.get(), reason)));
}

fn with_app<R>(f: impl FnOnce(&mut AppState) -> R) -> Option<R> {
    APP.with(|a| a.borrow_mut().as_mut().map(f))
}

/// Đăng ký các dòng đang bật của `keys` (dòng tắt không bao giờ được đăng ký); dòng nào không đăng
/// ký được thì báo **một** thông báo lỗi liệt kê mọi tổ hợp bị chiếm.
fn register_hotkeys_or_notify(hwnd: HWND, keys: &Hotkeys, notifier: &dyn Notifier) {
    let failed = hotkey::register_enabled(hwnd, keys);
    notify_hotkeys_taken(keys, &failed, notifier);
}

/// Tiêu đề và nội dung của thông báo gộp "Phím tắt chưa hoạt động"; `None` khi không có dòng lỗi.
pub fn hotkeys_taken_notice(
    lang: Lang,
    keys: &Hotkeys,
    failed: &[HotkeyRole],
) -> Option<(&'static str, String)> {
    if failed.is_empty() {
        return None;
    }
    Some((
        i18n::t(lang, Msg::HotkeysNotWorkingTitle),
        i18n::hotkeys_taken_body(lang, &hotkey::taken_combos(keys, failed)),
    ))
}

/// Báo **một** thông báo lỗi cho các dòng `failed` (nếu có).
fn notify_hotkeys_taken(keys: &Hotkeys, failed: &[HotkeyRole], notifier: &dyn Notifier) {
    if let Some((title, body)) = hotkeys_taken_notice(lang::current(), keys, failed) {
        notifier.notify(NoticeKind::Error, title, &body);
    }
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
                lang::tr(Msg::CaptureFailedTitle),
                lang::tr(Msg::CaptureFailedBody),
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
                lang::tr(Msg::OverlayFailedTitle),
                &i18n::reason_try_again(lang::current(), &e),
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
                lang: lang::current(),
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
            let registry = PENDING.with(Arc::clone);
            let pending_id = registry.add(image.clone(), config.save_dir.clone());
            let job = UploadJob {
                save_dir: config.save_dir.clone(),
                expiry: config.expiry,
                uploader,
                clipboard: clipboard.clone(),
                notifier: notifier.clone(),
                image: image.clone(),
                clock: paths::local_now,
                on_needs_settings: Box::new(move || post_open_settings(hwnd_raw)),
                pending: Some((registry.clone(), pending_id)),
                lang: lang::current(),
            };
            if let Err(e) = app::spawn_upload(job) {
                // Không có luồng nào giữ lượt này: lưu ngay ở dưới, rút khỏi sổ.
                registry.remove(pending_id);
                // Không tạo được luồng: lưu ảnh ngay thay vì mất nó.
                notifier.notify(
                    NoticeKind::Error,
                    lang::tr(Msg::UploadStartFailedTitle),
                    &i18n::upload_start_failed_body(lang::current(), &e),
                );
                let ctx = Ctx {
                    save_dir: &config.save_dir,
                    expiry: config.expiry,
                    now: paths::local_now(),
                    uploader: None,
                    clipboard: clipboard.as_ref(),
                    notifier: notifier.as_ref(),
                    lang: lang::current(),
                };
                run_action(&ctx, Action::Save, &image);
            }
        }
    }
}

/// Mở Cài đặt, lưu kết quả. Chỉ gọi qua `gate` (trạng thái `SettingsOpen`).
fn open_settings(hwnd: HWND, reason: OpenReason) {
    let Some((config, notice)) = with_app(|a| (a.config.clone(), a.config_notice.take())) else {
        return;
    };
    let Some(result) = settings::show(
        hwnd,
        &config,
        notice.map(|n| n.message(lang::current())).as_deref(),
        reason,
    ) else {
        // Hủy: `show` đã đăng ký lại các dòng của cấu hình cũ; dòng nào lỗi thì báo gộp một lần.
        let failed = settings::take_cancel_failures();
        if let Some(t) = current_tray() {
            notify_hotkeys_taken(&config.hotkeys, &failed, &t.notifier());
        }
        return;
    };
    let notify = |kind: NoticeKind, title: &str, body: &str| {
        if let Some(t) = current_tray() {
            t.notifier().notify(kind, title, body);
        }
    };
    // `show` đã đăng ký phím mới cho hwnd; ở đây chỉ ghi đĩa.
    let new = result.config;
    let saved = shotme_core::config::save(&paths::config_path(), &new, &DpapiStore);
    // Ghi thất bại thì ngôn ngữ giữ nguyên, cùng với cấu hình cũ và phím cũ.
    lang::set(lang_after_save(
        config.language,
        new.language,
        saved.is_ok(),
    ));
    match saved {
        Ok(()) => {
            if let Err(e) = autostart::set_enabled(new.autostart) {
                notify(
                    NoticeKind::Error,
                    lang::tr(Msg::AutostartFailedTitle),
                    &i18n::reason(lang::current(), &e),
                );
            }
            // Chữ trong hai cửa sổ này (ngôn ngữ, phím tắt) không còn đúng thì đóng chúng.
            if info_windows_stale(&config, &new) {
                about::close();
                help::close();
            }
            with_app(|a| a.config = new);
        }
        Err(e) => {
            // Không ghi được: giữ cấu hình cũ và phím cũ (chỉ các dòng đang bật của cấu hình cũ).
            hotkey::unregister_all(hwnd);
            let why = match e {
                shotme_core::config::ConfigError::Io(m)
                | shotme_core::config::ConfigError::Corrupt(m)
                | shotme_core::config::ConfigError::Secret(m) => m,
            };
            notify(
                NoticeKind::Error,
                lang::tr(Msg::OptionsSaveFailedTitle),
                &i18n::options_save_failed_body(lang::current(), &why),
            );
            if let Some(t) = current_tray() {
                register_hotkeys_or_notify(hwnd, &config.hotkeys, &t.notifier());
            } else {
                let _ = hotkey::register_enabled(hwnd, &config.hotkeys);
            }
        }
    }
}

/// Mở Cài đặt (cổng đã ở `SettingsOpen`), rồi đóng cổng lại.
fn run_settings(hwnd: HWND) {
    // Lấy lý do đang chờ ra, đặt lại `User` cho lần mở sau.
    let reason = OPEN_REASON.with(|r| r.replace(OpenReason::User));
    open_settings(hwnd, reason);
    let (next, _) = gate_step(GATE.with(|g| g.get()), GateEvent::SettingsClosed);
    GATE.with(|g| g.set(next));
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
            let (next, after) = gate_step(GATE.with(|g| g.get()), GateEvent::OverlayClosed);
            GATE.with(|g| g.set(next));
            if after == GateAction::OpenSettings {
                run_settings(hwnd);
            }
        }
        GateAction::OpenSettings => {
            run_settings(hwnd);
        }
        GateAction::FocusSettings => {
            // Yêu cầu mở đã được đáp bằng cửa sổ đang mở: không để lý do của nó dính sang lần mở sau.
            if ev == GateEvent::OpenSettings {
                OPEN_REASON.with(|r| r.set(OpenReason::User));
            }
            settings::focus_existing();
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
            // Thoát lúc đang tải lên: lưu ảnh vào máy trước, không chờ mạng.
            save_pending_uploads();
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
            if !t.readd() {
                // Explorer chưa nhận biểu tượng: thử lại theo hẹn giờ như lúc khởi động.
                ADD_ATTEMPTS.with(|c| c.set(0));
                // SAFETY: bộ hẹn giờ gắn với cửa sổ của luồng này; WM_TIMER xử lý ở wndproc.
                unsafe {
                    SetTimer(Some(hwnd), RETRY_TIMER_ID, RETRY_INTERVAL_MS, None);
                }
            }
        }
        return LRESULT(0);
    }
    match msg {
        WM_HOTKEY if hotkey::role_of(wp.0 as i32) == Some(HotkeyRole::Region) => {
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
        WM_HOTKEY if hotkey::role_of(wp.0 as i32).is_some() => {
            // Task 22: Lưu ảnh và Tải lên nhanh (toàn màn hình) nối vào luồng chụp ở đây.
            LRESULT(0)
        }
        WM_TRAY => {
            let ev = (lp.0 & 0xFFFF) as u32;
            // Menu đang mở thì bỏ qua hẳn: không mở menu lồng và không hạ cờ MENU_OPEN sớm.
            if tray_click_opens_menu(ev, MENU_OPEN.with(|m| m.get()))
                && gate(hwnd, GateEvent::TrayClick) == GateAction::None
            {
                MENU_OPEN.with(|m| m.set(true));
                let cmd = current_tray().and_then(|t| t.show_menu());
                MENU_OPEN.with(|m| m.set(false));
                match cmd {
                    Some(MenuCommand::Capture) => {
                        gate(hwnd, GateEvent::MenuCapture);
                    }
                    Some(MenuCommand::Options) => {
                        note_open_reason(OpenReason::User);
                        handle_event(hwnd, AppEvent::OpenSettings);
                    }
                    // Thông tin và Trợ giúp không đi qua cổng chụp: không chặn gì và không bị chặn.
                    Some(MenuCommand::About) => about::show(hwnd),
                    Some(MenuCommand::Help) => {
                        if let Some(keys) = with_app(|a| hotkey::help_keys(&a.config.hotkeys)) {
                            help::show(hwnd, &keys);
                        }
                    }
                    Some(MenuCommand::Quit) => handle_event(hwnd, AppEvent::Quit),
                    None => {}
                }
            }
            LRESULT(0)
        }
        WM_OPEN_SETTINGS => {
            // Ứng dụng tự mở (khởi động thiếu cấu hình, tải lên báo thiếu cấu hình): mở ở tab R2.
            note_open_reason(OpenReason::NeedsR2);
            if MENU_OPEN.with(|m| m.get()) {
                // Menu khay đang mở: đóng nó rồi mở Cài đặt sau khi menu thoát, không mở lồng.
                // SAFETY: đóng menu đang hiện của luồng này.
                unsafe {
                    let _ = EndMenu();
                }
                post_open_settings(hwnd.0 as isize);
            } else {
                handle_event(hwnd, AppEvent::OpenSettings);
            }
            LRESULT(0)
        }
        WM_NOTICE => {
            // SAFETY: con trỏ do TrayNotifier::notify cấp phát và chuyển quyền sở hữu cho ta.
            let notice = unsafe { Box::from_raw(lp.0 as *mut Notice) };
            // Chỗ lọc duy nhất: thông báo thành công theo cài đặt; lỗi và thông tin luôn hiện.
            let show = with_app(|a| a.config.show_success_notices).unwrap_or(true);
            if notice_visible(notice.kind, show) {
                if let Some(t) = current_tray() {
                    t.show_notice(&notice);
                }
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
        // Tài liệu WM_QUERYENDSESSION: trả TRUE ngay, để việc dọn dẹp tới WM_ENDSESSION.
        WM_QUERYENDSESSION => LRESULT(1),
        WM_ENDSESSION => {
            if wp.0 != 0 {
                // Phiên sắp kết thúc (tắt máy, đăng xuất): chỉ ghi file ảnh của các lượt còn dở.
                save_pending_uploads();
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            // Cửa sổ Thông tin và Trợ giúp thuộc cửa sổ ẩn này: hủy cả hai, trả phông của chúng.
            about::close();
            help::close();
            hotkey::unregister_all(hwnd);
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
    // Không để Windows gửi bản kết xuất bộ nhớ (có thể chứa khóa, ảnh) đi khi sự cố.
    wer::exclude_from_error_reporting();

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
    let state = app::startup(
        &config_path,
        &DpapiStore,
        &paths::default_save_dir(),
        lang::os_default(),
    );
    lang::set(state.config.language);
    let open_settings_now = app::needs_settings_on_start(&state, config_path.exists());
    let keys = state.config.hotkeys;
    APP.with(|a| *a.borrow_mut() = Some(state));
    // Ghi lại đường dẫn exe hiện tại phòng khi file đã được chuyển chỗ; lỗi thì bỏ qua.
    if autostart::is_enabled() {
        let _ = autostart::set_enabled(true);
    }
    register_hotkeys_or_notify(hwnd, &keys, &notifier);

    if open_settings_now {
        post_open_settings(hwnd.0 as isize);
    }

    let mut msg = MSG::default();
    // SAFETY: vòng lặp thông điệp chuẩn; GetMessageW trả về 0 (WM_QUIT) hoặc -1 (lỗi) thì dừng.
    unsafe {
        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
            if about::pre_translate(&msg) || help::pre_translate(&msg) {
                continue;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    // Lưới an toàn: vòng lặp kết thúc theo đường nào thì ảnh của lượt còn dở cũng được lưu.
    save_pending_uploads();
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
        // Đang mở Cài đặt: bấm khay hoặc mở lại thì đưa cửa sổ ra trước, vẫn SettingsOpen.
        for ev in [TrayClick, OpenSettings] {
            assert_eq!(
                gate_step(SettingsOpen, ev),
                (SettingsOpen, GateAction::FocusSettings),
                "{ev:?}"
            );
        }
        // Yêu cầu chụp không chụp nhưng đưa cửa sổ Cài đặt ra trước để nó không bị khuất.
        for ev in [MenuCapture, Hotkey] {
            assert_eq!(
                gate_step(SettingsOpen, ev),
                (SettingsOpen, GateAction::FocusSettings),
                "{ev:?}"
            );
        }
        // Hẹn giờ cũ của menu thì chỉ bị bỏ qua.
        assert_eq!(
            gate_step(SettingsOpen, TimerFired),
            (SettingsOpen, GateAction::Ignore)
        );
        assert_eq!(
            gate_step(SettingsOpen, OverlayClosed),
            (SettingsOpen, GateAction::None)
        );
        assert_eq!(
            gate_step(SettingsOpen, SettingsClosed),
            (Idle, GateAction::None)
        );
        // Đóng Cài đặt khi không ở trạng thái đó: không đổi.
        assert_eq!(gate_step(Idle, SettingsClosed), (Idle, GateAction::None));
        assert_eq!(
            gate_step(Capturing, SettingsClosed),
            (Capturing, GateAction::None)
        );
    }

    #[test]
    fn open_reason_merge_prefers_r2() {
        use settings::OpenReason::*;
        assert_eq!(merge_reason(User, User), User);
        assert_eq!(merge_reason(User, NeedsR2), NeedsR2);
        assert_eq!(merge_reason(NeedsR2, User), NeedsR2);
    }

    #[test]
    fn language_changes_only_when_saved() {
        assert_eq!(lang_after_save(Lang::Vi, Lang::En, true), Lang::En);
        assert_eq!(lang_after_save(Lang::Vi, Lang::En, false), Lang::Vi);
    }

    #[test]
    fn info_windows_close_when_language_or_hotkey_changes() {
        use std::path::Path;
        let a = Config::defaults(Path::new("C:/p"), Lang::Vi);
        assert!(!info_windows_stale(&a, &a.clone()));
        let mut b = a.clone();
        b.language = Lang::En;
        assert!(info_windows_stale(&a, &b));
        let mut b = a.clone();
        b.hotkeys.region.key.ctrl = true;
        assert!(info_windows_stale(&a, &b));
        let mut b = a.clone();
        b.autostart = true;
        assert!(!info_windows_stale(&a, &b));
    }

    #[test]
    fn info_windows_close_when_any_hotkey_row_changes() {
        use std::path::Path;
        let a = Config::defaults(Path::new("C:/p"), Lang::Vi);
        // Bật/tắt một dòng: stale.
        for r in shotme_core::config::HotkeyRole::ALL {
            let mut b = a.clone();
            let mut slot = b.hotkeys.get(r);
            slot.enabled = !slot.enabled;
            b.hotkeys.set(r, slot);
            assert!(info_windows_stale(&a, &b), "{r:?}");
        }
        // Đổi tổ hợp của dòng 2 hay 3: stale.
        let mut b = a.clone();
        b.hotkeys.save_full.key.alt = true;
        assert!(info_windows_stale(&a, &b));
        let mut b = a.clone();
        b.hotkeys.upload_full.key.vk = 0x71;
        assert!(info_windows_stale(&a, &b));
        // Đổi show_success_notices: không stale.
        let mut b = a.clone();
        b.show_success_notices = !b.show_success_notices;
        assert!(!info_windows_stale(&a, &b));
    }

    #[test]
    fn hotkeys_taken_notice_only_when_rows_failed() {
        use shotme_core::config::HotkeyRole::*;
        let keys = Hotkeys::default();
        assert_eq!(hotkeys_taken_notice(Lang::Vi, &keys, &[]), None);
        let (title, body) = hotkeys_taken_notice(Lang::Vi, &keys, &[UploadFull, Region]).unwrap();
        assert_eq!(title, i18n::t(Lang::Vi, Msg::HotkeysNotWorkingTitle));
        assert_eq!(
            body,
            i18n::hotkeys_taken_body(
                Lang::Vi,
                &["PrintScreen".to_string(), "Ctrl+PrintScreen".to_string()]
            )
        );
    }

    #[test]
    fn tray_click_while_menu_open_is_ignored() {
        assert!(tray_click_opens_menu(WM_RBUTTONUP, false));
        assert!(tray_click_opens_menu(WM_LBUTTONUP, false));
        assert!(!tray_click_opens_menu(WM_RBUTTONUP, true));
        assert!(!tray_click_opens_menu(WM_LBUTTONUP, true));
        assert!(!tray_click_opens_menu(WM_TIMER, false));
    }

    #[test]
    fn settings_request_during_capture_is_remembered() {
        let (s, a) = gate_step(Capturing, OpenSettings);
        assert_eq!(a, GateAction::Ignore);
        // Hai yêu cầu chờ chỉ mở một lần.
        let (s, a) = gate_step(s, OpenSettings);
        assert_eq!(a, GateAction::Ignore);
        // Phím chụp trong lúc đó vẫn bị bỏ qua.
        assert_eq!(gate_step(s, Hotkey).1, GateAction::Ignore);
        assert_eq!(
            gate_step(s, OverlayClosed),
            (SettingsOpen, GateAction::OpenSettings)
        );
        // Không có yêu cầu chờ: về Idle như cũ.
        assert_eq!(
            gate_step(Capturing, OverlayClosed),
            (Idle, GateAction::None)
        );
    }

    #[test]
    fn kinds_of_roles() {
        assert_eq!(kind_of(HotkeyRole::Region), CaptureKind::Region);
        assert_eq!(kind_of(HotkeyRole::SaveFull), CaptureKind::SaveFull);
        assert_eq!(kind_of(HotkeyRole::UploadFull), CaptureKind::UploadFull);
    }

    #[test]
    fn capture_kind_follows_the_accepted_request() {
        use CaptureKind::*;
        let step = |state, ev, cur, req| {
            let (s, a) = gate_step(state, ev);
            (s, a, next_kind(cur, req, a))
        };
        // rảnh, bấm phím lưu toàn màn hình: chụp ngay loại đó
        assert_eq!(
            step(Idle, Hotkey, Region, SaveFull),
            (Capturing, GateAction::CaptureNow, SaveFull)
        );
        // menu đang mở: chờ, loại được nhớ; hết giờ không đổi loại
        let (s, a, k) = step(Idle, MenuCapture, Region, UploadFull);
        assert_eq!((s, a, k), (PendingMenu, GateAction::StartTimer, UploadFull));
        assert_eq!(
            gate_step(s, TimerFired),
            (Capturing, GateAction::CaptureNow)
        );
        // đang chờ menu (Region) mà bấm phím loại khác: chụp ngay một lần, loại của phím vừa bấm
        assert_eq!(
            step(PendingMenu, Hotkey, Region, SaveFull),
            (Capturing, GateAction::CaptureNow, SaveFull)
        );
        // lớp phủ đang mở: bỏ qua, loại không đổi
        assert_eq!(
            step(Capturing, Hotkey, Region, SaveFull),
            (Capturing, GateAction::Ignore, Region)
        );
        assert_eq!(step(Capturing, MenuCapture, Region, UploadFull).2, Region);
        // Tùy chọn đang mở: đưa ra trước, loại không đổi
        assert_eq!(
            step(SettingsOpen, Hotkey, Region, UploadFull),
            (SettingsOpen, GateAction::FocusSettings, Region)
        );
    }

    #[test]
    fn selection_is_remembered_only_when_used() {
        let old = Some(Rect {
            x: 1,
            y: 2,
            w: 3,
            h: 4,
        });
        let new = Rect {
            x: 10,
            y: 20,
            w: 30,
            h: 40,
        };
        for a in [Action::Upload, Action::Save, Action::Copy] {
            assert_eq!(remember_after(old, a, new), Some(new));
            assert_eq!(remember_after(None, a, new), Some(new));
        }
        assert_eq!(remember_after(old, Action::Cancel, new), old);
        assert_eq!(remember_after(None, Action::Cancel, new), None);
    }

    #[test]
    fn settings_request_during_pending_menu_is_remembered() {
        let (s, a) = gate_step(PendingMenu, OpenSettings);
        assert_eq!(a, GateAction::Ignore);
        let (s, a) = gate_step(s, OpenSettings);
        assert_eq!(a, GateAction::Ignore);
        // Hẹn giờ nổ: vẫn chụp trước, yêu cầu còn chờ.
        let (s, a) = gate_step(s, TimerFired);
        assert_eq!(a, GateAction::CaptureNow);
        assert_eq!(
            gate_step(s, OverlayClosed),
            (SettingsOpen, GateAction::OpenSettings)
        );
        // Phím chụp thay cho hẹn giờ cũng vậy.
        let (s, _) = gate_step(PendingMenu, OpenSettings);
        let (s, a) = gate_step(s, Hotkey);
        assert_eq!(a, GateAction::CaptureNow);
        assert_eq!(
            gate_step(s, OverlayClosed),
            (SettingsOpen, GateAction::OpenSettings)
        );
    }
}
