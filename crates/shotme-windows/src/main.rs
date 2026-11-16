#![windows_subsystem = "windows"]

#[allow(dead_code)]
mod capture;
mod hotkey;
#[allow(dead_code)]
mod overlay;
// Task 11 dùng các hàm trong paths; ở task này chỉ có kiểm thử gọi chúng.
#[allow(dead_code)]
mod paths;
mod tray;

use shotme_core::actions::{NoticeKind, Notifier};
use shotme_core::config::Hotkey;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use tray::{MenuCommand, Notice, Tray, WM_NOTICE, WM_TRAY};
use windows::core::w;
use windows::Win32::Foundation::{
    GetLastError, ERROR_ALREADY_EXISTS, HWND, LPARAM, LRESULT, WPARAM,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, KillTimer,
    PostQuitMessage, RegisterClassW, RegisterWindowMessageW, SetTimer, TranslateMessage,
    CW_USEDEFAULT, MSG, WM_DESTROY, WM_HOTKEY, WM_LBUTTONUP, WM_RBUTTONUP, WM_TIMER, WNDCLASSW,
    WS_EX_TOOLWINDOW, WS_OVERLAPPED,
};

/// Sự kiện nội bộ; Task 11 mở rộng.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppEvent {
    Capture,
    OpenSettings,
    Quit,
}

thread_local! {
    static TRAY: RefCell<Option<Rc<Tray>>> = const { RefCell::new(None) };
    static TASKBAR_CREATED: Cell<u32> = const { Cell::new(0) };
    static ADD_ATTEMPTS: Cell<u32> = const { Cell::new(0) };
}

const RETRY_TIMER_ID: usize = 1;
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

fn handle_event(hwnd: HWND, ev: AppEvent) {
    match ev {
        AppEvent::Capture | AppEvent::OpenSettings => {
            if let Some(t) = current_tray() {
                t.notifier().notify(NoticeKind::Info, "shotme", "Chưa có");
            }
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
            handle_event(hwnd, AppEvent::Capture);
            LRESULT(0)
        }
        WM_TRAY => {
            let ev = (lp.0 & 0xFFFF) as u32;
            if ev == WM_RBUTTONUP || ev == WM_LBUTTONUP {
                let cmd = current_tray().and_then(|t| t.show_menu());
                match cmd {
                    Some(MenuCommand::Capture) => handle_event(hwnd, AppEvent::Capture),
                    Some(MenuCommand::Settings) => handle_event(hwnd, AppEvent::OpenSettings),
                    Some(MenuCommand::Quit) => handle_event(hwnd, AppEvent::Quit),
                    None => {}
                }
            }
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

    let hk = Hotkey::default();
    if !hotkey::register(hwnd, &hk) {
        let name = hotkey::describe(&hk);
        notifier.notify(
            NoticeKind::Error,
            &format!("Phím {name} đang bị ứng dụng khác chiếm"),
            "Hãy đổi phím tắt trong Cài đặt.",
        );
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
