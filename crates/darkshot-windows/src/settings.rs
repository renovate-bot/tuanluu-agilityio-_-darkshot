//! Cửa sổ Tùy chọn bốn tab (Win32 thuần, Common Controls v6 qua manifest).
//!
//! Phần logic (`apply_form`, `capture_hotkey`, `key_route`, `tab_of`, `initial_tab`) là hàm thuần
//! có kiểm thử; phần cửa sổ mỏng.
//! `show` chạy vòng lặp thông điệp riêng (hộp thoại kiểu modal) và bảo đảm khi trả về:
//! OK thì phím tắt mới (nếu đổi) đã được đăng ký cho `parent`; Hủy thì phím tắt cũ đã đăng ký lại.
//! Việc ghi `Config` ra đĩa và bật/tắt khởi động cùng Windows do bên gọi (Task 11) làm.

use crate::hotkey;
use crate::lang;
use darkshot_core::config::{
    clamp_jpeg_quality, normalize_public_base_url, Config, Expiry, Hotkey, HotkeyRole, Hotkeys,
    R2Config, UploadFormat, JPEG_QUALITY_MAX, JPEG_QUALITY_MIN,
};
use darkshot_core::i18n::{self, expiry_label, t, Lang, Msg};
use darkshot_core::upload::R2Uploader;
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use windows::core::{w, HSTRING, PCWSTR, PWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    DeleteObject, FillRect, GetDC, GetSysColorBrush, GetTextExtentPoint32W, ReleaseDC,
    SelectObject, SetBkMode, SetTextColor, COLOR_BTNFACE, COLOR_WINDOW, HBRUSH, HDC, HFONT,
    HGDIOBJ, TRANSPARENT,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{
    InitCommonControlsEx, IsAppThemed, ICC_BAR_CLASSES, ICC_TAB_CLASSES, INITCOMMONCONTROLSEX,
    NMHDR, TBM_SETLINESIZE, TBM_SETPAGESIZE, TBM_SETPOS, TBM_SETRANGE, TBS_HORZ, TCIF_TEXT,
    TCITEMW, TCM_ADJUSTRECT, TCM_GETCURSEL, TCM_INSERTITEMW, TCM_SETCURSEL, TCN_SELCHANGE,
    TCN_SELCHANGING, TRACKBAR_CLASSW, WC_TABCONTROLW,
};
use windows::Win32::UI::HiDpi::{AdjustWindowRectExForDpi, GetDpiForWindow};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    EnableWindow, GetFocus, GetKeyState, SetFocus, VK_CONTROL, VK_ESCAPE, VK_LWIN, VK_MENU,
    VK_RETURN, VK_RWIN, VK_SHIFT,
};
use windows::Win32::UI::Shell::{
    FileOpenDialog, IFileOpenDialog, FOS_FORCEFILESYSTEM, FOS_PICKFOLDERS, SIGDN_FILESYSPATH,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect, GetMessageW,
    GetParent, GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW, IsChild, IsDialogMessageW,
    IsIconic, KillTimer, PostMessageW, PostQuitMessage, RegisterClassW, SendMessageW,
    SetForegroundWindow, SetTimer, SetWindowPos, SetWindowTextW, ShowWindow, TranslateMessage,
    BM_CLICK, BM_GETCHECK, BM_SETCHECK, BS_AUTOCHECKBOX, BS_AUTORADIOBUTTON, BS_DEFPUSHBUTTON,
    BS_PUSHBUTTON, CBS_DROPDOWNLIST, CB_ADDSTRING, CB_GETCURSEL, CB_GETDROPPEDSTATE, CB_SETCURSEL,
    CB_SHOWDROPDOWN, CW_USEDEFAULT, ES_AUTOHSCROLL, ES_PASSWORD, ES_READONLY, GWL_STYLE, HMENU,
    HWND_BOTTOM, MSG, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SW_HIDE, SW_RESTORE,
    SW_SHOW, WA_INACTIVE, WINDOW_EX_STYLE, WINDOW_STYLE, WM_ACTIVATE, WM_APP, WM_CLOSE, WM_COMMAND,
    WM_CTLCOLORSTATIC, WM_ERASEBKGND, WM_HSCROLL, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN,
    WM_MBUTTONDOWN, WM_NCLBUTTONDOWN, WM_NCRBUTTONDOWN, WM_NOTIFY, WM_PRINTCLIENT, WM_RBUTTONDOWN,
    WM_SETFONT, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_TIMER, WNDCLASSW, WS_BORDER, WS_CAPTION, WS_CHILD,
    WS_CLIPSIBLINGS, WS_DISABLED, WS_EX_APPWINDOW, WS_EX_CONTROLPARENT, WS_GROUP, WS_SYSMENU,
    WS_TABSTOP, WS_VISIBLE, WS_VSCROLL,
};

/// Giá trị đọc từ các ô của cửa sổ.
#[derive(Clone)]
pub struct Form {
    pub account_id: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub bucket: String,
    pub public_base_url: String,
    pub expiry: Expiry,
    pub hotkeys: Hotkeys,
    pub save_dir: String,
    pub autostart: bool,
    pub language: Lang,
    pub show_success_notices: bool,
    pub keep_selection: bool,
    pub include_cursor: bool,
    pub upload_format: UploadFormat,
    pub jpeg_quality: u8,
}

// Debug tự viết để khóa bí mật không bao giờ lọt vào log hay thông báo lỗi.
impl std::fmt::Debug for Form {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Form")
            .field("account_id", &self.account_id)
            .field("access_key_id", &self.access_key_id)
            .field("secret_access_key", &"<redacted>")
            .field("bucket", &self.bucket)
            .field("public_base_url", &self.public_base_url)
            .field("expiry", &self.expiry)
            .field("hotkeys", &self.hotkeys)
            .field("save_dir", &self.save_dir)
            .field("autostart", &self.autostart)
            .field("language", &self.language)
            .field("show_success_notices", &self.show_success_notices)
            .field("keep_selection", &self.keep_selection)
            .field("include_cursor", &self.include_cursor)
            .field("upload_format", &self.upload_format)
            .field("jpeg_quality", &self.jpeg_quality)
            .finish()
    }
}

pub struct SettingsResult {
    pub config: Config,
}

fn is_account_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-'
}

fn is_bucket_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-'
}

/// Tab của cửa sổ Tùy chọn, theo thứ tự trên dải tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    General,
    Hotkeys,
    Format,
    R2,
}

/// Ô có thể sai khi bấm OK.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    SaveDir,
    HotkeyRow(HotkeyRole),
    AccountId,
    Bucket,
    PublicUrl,
}

/// Lỗi của biểu mẫu: ô sai và câu báo lỗi (không bao giờ chứa khóa bí mật).
#[derive(Debug, PartialEq, Eq)]
pub struct FormError {
    pub field: Field,
    pub message: String,
}

/// Tab chứa ô `field`.
pub fn tab_of(field: Field) -> Tab {
    match field {
        Field::SaveDir => Tab::General,
        Field::HotkeyRow(_) => Tab::Hotkeys,
        Field::AccountId | Field::Bucket | Field::PublicUrl => Tab::R2,
    }
}

impl Tab {
    /// Thứ tự trên dải tab.
    const ALL: [Tab; 4] = [Tab::General, Tab::Hotkeys, Tab::Format, Tab::R2];

    fn index(self) -> usize {
        match self {
            Tab::General => 0,
            Tab::Hotkeys => 1,
            Tab::Format => 2,
            Tab::R2 => 3,
        }
    }

    fn label(self) -> Msg {
        match self {
            Tab::General => Msg::TabGeneral,
            Tab::Hotkeys => Msg::TabHotkeys,
            Tab::Format => Msg::TabFormat,
            Tab::R2 => Msg::TabR2,
        }
    }

    /// Tab cách `step` bước về bên phải, vòng tròn (`len - 1` bước = tab bên trái).
    fn cycle(self, step: usize) -> Tab {
        Tab::ALL[(self.index() + step) % Tab::ALL.len()]
    }
}

/// Ba ô chất lượng (nhãn, thanh trượt, số) chỉ sáng khi tải lên bằng JPEG.
pub fn quality_enabled(format: UploadFormat) -> bool {
    format == UploadFormat::Jpeg
}

/// Dải tab vừa một hàng: tổng bề rộng các tab không vượt `client_w` (cùng đơn vị).
fn tabs_fit(tab_widths: &[i32], client_w: i32) -> bool {
    let total: i64 = tab_widths.iter().map(|w| i64::from(*w)).sum();
    total <= i64::from(client_w)
}

/// Lý do mở cửa sổ Tùy chọn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenReason {
    /// Người dùng chọn "Tùy chọn..." ở menu.
    User,
    /// Ứng dụng tự mở vì R2 chưa cấu hình, mất khóa hoặc cấu hình lỗi.
    NeedsR2,
}

/// Tab mở đầu theo lý do mở.
pub fn initial_tab(reason: OpenReason) -> Tab {
    match reason {
        OpenReason::User => Tab::General,
        OpenReason::NeedsR2 => Tab::R2,
    }
}

/// Gộp biểu mẫu vào cấu hình hiện tại. Thuần: không đụng hệ điều hành.
/// Kiểm tra theo thứ tự tab: SaveDir, trùng tổ hợp giữa các dòng phím tắt đang bật, AccountId,
/// Bucket, PublicUrl; câu lỗi theo `lang`.
pub fn apply_form(current: &Config, form: &Form, lang: Lang) -> Result<Config, FormError> {
    let account_id = form.account_id.trim().to_string();
    let access_key_id = form.access_key_id.trim().to_string();
    let bucket = form.bucket.trim().to_string();
    let secret = form.secret_access_key.trim();
    let url = form.public_base_url.trim();
    let save_dir = form.save_dir.trim();
    let err = |field: Field, message: String| Err(FormError { field, message });

    if save_dir.is_empty() {
        return err(Field::SaveDir, t(lang, Msg::ErrPickSaveDir).into());
    }
    if let Some((first, second)) = form.hotkeys.first_conflict() {
        return err(
            Field::HotkeyRow(second),
            i18n::hotkeys_conflict(
                lang,
                t(lang, first.label()),
                t(lang, second.label()),
                &hotkey::describe(&form.hotkeys.get(second).key),
            ),
        );
    }
    if !account_id.chars().all(is_account_char) {
        return err(Field::AccountId, t(lang, Msg::ErrAccountChars).into());
    }
    if !bucket.chars().all(is_bucket_char) {
        return err(Field::Bucket, t(lang, Msg::ErrBucketChars).into());
    }
    let public_base_url = if url.is_empty() {
        String::new()
    } else {
        match normalize_public_base_url(url) {
            Ok(u) => u,
            Err(e) => return err(Field::PublicUrl, e.message(lang)),
        }
    };
    let secret_access_key = if secret.is_empty() {
        current.r2.secret_access_key.clone()
    } else {
        secret.to_string()
    };
    Ok(Config {
        r2: R2Config {
            account_id,
            access_key_id,
            secret_access_key,
            bucket,
            public_base_url,
        },
        expiry: form.expiry,
        hotkeys: form.hotkeys,
        show_success_notices: form.show_success_notices,
        keep_selection: form.keep_selection,
        include_cursor: form.include_cursor,
        save_dir: PathBuf::from(save_dir),
        autostart: form.autostart,
        language: form.language,
        upload_format: form.upload_format,
        jpeg_quality: clamp_jpeg_quality(i64::from(form.jpeg_quality)),
    })
}

/// Một phím (WM_KEYDOWN) trong cửa sổ Tùy chọn đi về đâu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyRoute {
    Capture,
    NextTab,
    PrevTab,
    Cancel,
    Accept,
    Dialog,
}

/// Định tuyến phím; `combo_dropped` đúng khi một hộp chọn đang thả.
pub fn key_route(
    capturing: bool,
    vk: u32,
    ctrl: bool,
    shift: bool,
    combo_dropped: bool,
) -> KeyRoute {
    const TAB: u32 = 0x09;
    if capturing {
        return KeyRoute::Capture;
    }
    match vk {
        TAB if ctrl && shift => KeyRoute::PrevTab,
        TAB if ctrl => KeyRoute::NextTab,
        _ if combo_dropped => KeyRoute::Dialog,
        v if v == VK_ESCAPE.0 as u32 => KeyRoute::Cancel,
        v if v == VK_RETURN.0 as u32 => KeyRoute::Accept,
        _ => KeyRoute::Dialog,
    }
}

/// Kết quả xử lý một lần nhả phím khi đang chọn phím tắt mới.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capture {
    /// Phím bổ trợ đứng một mình: chờ tiếp.
    Ignore,
    /// Phím thường cần kèm Ctrl, Alt hoặc Win (Shift một mình không đủ).
    NeedModifier,
    /// Không nuốt phím: để hệ thống xử lý như thường (Alt+F4 đóng cửa sổ).
    PassThrough,
    Got(Hotkey),
}

/// Sự kiện phím khi đang chọn phím tắt, kèm trạng thái phím bổ trợ lúc xảy ra.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEv {
    Down {
        vk: u32,
        ctrl: bool,
        alt: bool,
        shift: bool,
        win: bool,
    },
    Up {
        vk: u32,
        ctrl: bool,
        alt: bool,
        shift: bool,
        win: bool,
    },
}

/// Máy trạng thái của chế độ bắt phím: chốt tổ hợp ở KEYDOWN (đúng phím bổ trợ lúc bấm);
/// KEYUP không có KEYDOWN tương ứng bị bỏ, trừ PrintScreen (không sinh KEYDOWN).
#[derive(Debug, Default)]
pub struct CaptureState {
    downs: Vec<u32>,
}

impl CaptureState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn feed(&mut self, ev: KeyEv) -> Capture {
        match ev {
            KeyEv::Down {
                vk,
                ctrl,
                alt,
                shift,
                win,
            } => {
                let out = capture_hotkey(vk, ctrl, alt, shift, win);
                if matches!(out, Capture::Got(_) | Capture::NeedModifier)
                    && !self.downs.contains(&vk)
                {
                    self.downs.push(vk);
                }
                out
            }
            KeyEv::Up {
                vk,
                ctrl,
                alt,
                shift,
                win,
            } => {
                if let Some(i) = self.downs.iter().position(|k| *k == vk) {
                    self.downs.swap_remove(i);
                    Capture::Ignore
                } else if vk == 0x2C {
                    // PrintScreen không sinh KEYDOWN: nhận ở KEYUP.
                    capture_hotkey(vk, ctrl, alt, shift, win)
                } else {
                    Capture::Ignore
                }
            }
        }
    }
}

/// Phím cho phép đứng một mình làm phím tắt: PrintScreen, Pause, ScrollLock, F1..F24.
fn allowed_bare(vk: u32) -> bool {
    matches!(vk, 0x2C | 0x13 | 0x91 | 0x70..=0x87)
}

fn is_modifier_vk(vk: u32) -> bool {
    matches!(vk, 0x10 | 0x11 | 0x12 | 0x5B | 0x5C | 0xA0..=0xA5)
}

pub fn capture_hotkey(vk: u32, ctrl: bool, alt: bool, shift: bool, win: bool) -> Capture {
    if is_modifier_vk(vk) {
        return Capture::Ignore;
    }
    // Alt+F4 luôn là lệnh đóng cửa sổ của hệ thống.
    if vk == 0x73 && alt {
        return Capture::PassThrough;
    }
    // Shift một mình không đủ: sẽ chặn việc gõ chữ hoa.
    if ctrl || alt || win || allowed_bare(vk) {
        Capture::Got(Hotkey {
            ctrl,
            alt,
            shift,
            win,
            vk,
        })
    } else {
        Capture::NeedModifier
    }
}

// ---------------------------------------------------------------------------
// Cửa sổ
// ---------------------------------------------------------------------------

const ID_OK: i32 = 1;
const ID_CANCEL: i32 = 2;
const ID_ACCOUNT: i32 = 101;
const ID_KEYID: i32 = 102;
const ID_SECRET: i32 = 103;
const ID_BUCKET: i32 = 104;
const ID_URL: i32 = 105;
const ID_TEST: i32 = 106;
const ID_EXPIRY: i32 = 107;
const ID_DIR: i32 = 108;
const ID_BROWSE: i32 = 109;
const ID_AUTOSTART: i32 = 111;
const ID_LANGUAGE: i32 = 112;
const ID_TABS: i32 = 113;
const ID_HELP: i32 = 114;
const ID_NOTICES: i32 = 115;
const ID_KEEP_SELECTION: i32 = 116;
const ID_INCLUDE_CURSOR: i32 = 117;
/// Tab Định dạng: hai nút chọn PNG/JPEG, thanh trượt chất lượng và ô số cạnh nó.
const ID_FMT_PNG: i32 = 118;
const ID_FMT_JPEG: i32 = 119;
const ID_QUALITY: i32 = 140;
const ID_QUALITY_VALUE: i32 = 141;
/// `TBM_GETPOS` (= `WM_USER`, commctrl.h); crate `windows` không có hằng này.
const TBM_GETPOS: u32 = 0x0400;
/// Nút "Đổi…" của dòng phím tắt thứ i có ID `ID_HK_CHANGE0 + i`; ô tích của dòng là `ID_HK_CHECK0 + i`.
const ID_HK_CHANGE0: i32 = 130;
const ID_HK_CHECK0: i32 = 133;
/// Khung của tab thứ i có ID `ID_FRAME0 + i` (120 đến 123).
const ID_FRAME0: i32 = 120;
const FRAME_CLASS: PCWSTR = w!("shotme-settings-frame");

const fn ws(v: i32) -> WINDOW_STYLE {
    WINDOW_STYLE(v as u32)
}

const TEST_TIMER_ID: usize = 1;
/// Thời gian chờ tối đa của `test_connection` (60 s) cộng dư.
const TEST_FAILSAFE_MS: u32 = 70_000;
const WM_APP_TEST_DONE: u32 = WM_APP + 1;
const VK_CONTROL_U: i32 = VK_CONTROL.0 as i32;

#[derive(Clone, Copy)]
struct Ctl {
    tabs: HWND,
    /// Khung của từng tab, theo `Tab::index`.
    frames: [HWND; 4],
    language: HWND,
    fmt_png: HWND,
    fmt_jpeg: HWND,
    quality_label: HWND,
    quality: HWND,
    quality_value: HWND,
    account: HWND,
    keyid: HWND,
    secret: HWND,
    bucket: HWND,
    url: HWND,
    test_btn: HWND,
    test_res: HWND,
    expiry: HWND,
    dir: HWND,
    /// Ô tích, ô chỉ đọc và nút "Đổi…" của từng dòng phím tắt, theo `HotkeyRole::ALL`.
    hk_chk: [HWND; 3],
    hk_edit: [HWND; 3],
    hk_btn: [HWND; 3],
    autostart: HWND,
    notices: HWND,
    keep_selection: HWND,
    include_cursor: HWND,
    err: HWND,
}

/// Vị trí của dòng `r` trong `HotkeyRole::ALL` (chỉ số của các mảng `hk_*` trong `Ctl`).
fn row(r: HotkeyRole) -> usize {
    HotkeyRole::ALL.iter().position(|x| *x == r).unwrap_or(0)
}

struct St {
    current: Config,
    parent: HWND,
    /// Ba dòng phím tắt đang hiện trên biểu mẫu (bật/tắt theo ô tích).
    hotkeys: Hotkeys,
    /// Dòng đang ở chế độ bắt phím; mỗi lúc tối đa một dòng.
    capturing: Option<HotkeyRole>,
    cap: CaptureState,
    /// Các phím tắt cũ đã bị hủy đăng ký (do "Đổi…" hoặc lần OK lỗi) và cần đăng ký lại nếu Hủy.
    hk_unregistered: bool,
    done: bool,
    result: Option<Config>,
    font: Option<HFONT>,
    /// Tab đang hiện.
    tab: Tab,
}

thread_local! {
    /// Một lần Kiểm tra kết nối đang chạy ở luồng nền (nút đang bị làm mờ).
    static TEST_RUNNING: Cell<bool> = const { Cell::new(false) };
    /// HWND của cửa sổ đang mở (0 = không có).
    static OPEN: Cell<isize> = const { Cell::new(0) };
    static CTL: Cell<Option<Ctl>> = const { Cell::new(None) };
    static ST: RefCell<Option<St>> = const { RefCell::new(None) };
    /// Các dòng không đăng ký lại được ở lượt Hủy vừa rồi; bên gọi lấy bằng `take_cancel_failures`.
    static CANCEL_FAILED: RefCell<Vec<HotkeyRole>> = const { RefCell::new(Vec::new()) };
}

/// Thế hệ cửa sổ: kết quả của luồng nền thuộc cửa sổ đã đóng sẽ bị bỏ.
static GENERATION: AtomicU64 = AtomicU64::new(0);
static TEST_RESULT: Mutex<Option<(u64, String)>> = Mutex::new(None);

fn with_st<R>(f: impl FnOnce(&mut St) -> R) -> Option<R> {
    ST.with(|s| s.borrow_mut().as_mut().map(f))
}

fn ctl() -> Option<Ctl> {
    CTL.with(|c| c.get())
}

fn get_text(h: HWND) -> String {
    // SAFETY: h là cửa sổ con của luồng này; bộ đệm đủ chỗ cho chuỗi cộng ký tự kết thúc.
    unsafe {
        let n = GetWindowTextLengthW(h);
        if n <= 0 {
            return String::new();
        }
        let mut buf = vec![0u16; n as usize + 1];
        let got = GetWindowTextW(h, &mut buf).max(0) as usize;
        let s = String::from_utf16_lossy(&buf[..got.min(buf.len())]);
        buf.fill(0);
        s
    }
}

fn set_text(h: HWND, s: &str) {
    // SAFETY: HSTRING sống suốt lời gọi.
    unsafe {
        let _ = SetWindowTextW(h, &HSTRING::from(s));
    }
}

fn read_form() -> Option<Form> {
    let c = ctl()?;
    // SAFETY: các HWND là điều khiển của cửa sổ đang mở ở luồng này.
    let (sel, lang_sel) = unsafe {
        (
            SendMessageW(c.expiry, CB_GETCURSEL, None, None).0,
            SendMessageW(c.language, CB_GETCURSEL, None, None).0,
        )
    };
    let expiry = usize::try_from(sel)
        .ok()
        .and_then(|i| Expiry::ALL.get(i).copied())
        .unwrap_or_default();
    let (mut hotkeys, old_lang) = with_st(|s| (s.hotkeys, s.current.language))?;
    let upload_format = if is_checked(c.fmt_jpeg) {
        UploadFormat::Jpeg
    } else {
        UploadFormat::Png
    };
    // Ô tích là nguồn đúng của trạng thái bật/tắt.
    for r in HotkeyRole::ALL {
        let mut slot = hotkeys.get(r);
        slot.enabled = is_checked(c.hk_chk[row(r)]);
        hotkeys.set(r, slot);
    }
    let language = usize::try_from(lang_sel)
        .ok()
        .and_then(|i| Lang::ALL.get(i).copied())
        .unwrap_or(old_lang);
    Some(Form {
        account_id: get_text(c.account),
        access_key_id: get_text(c.keyid),
        secret_access_key: get_text(c.secret),
        bucket: get_text(c.bucket),
        public_base_url: get_text(c.url),
        expiry,
        hotkeys,
        save_dir: get_text(c.dir),
        autostart: is_checked(c.autostart),
        language,
        show_success_notices: is_checked(c.notices),
        keep_selection: is_checked(c.keep_selection),
        include_cursor: is_checked(c.include_cursor),
        upload_format,
        // Đọc cả khi đang chọn PNG: giá trị được giữ cho lần chọn lại JPEG.
        jpeg_quality: trackbar_pos(c.quality),
    })
}

/// Vị trí thanh trượt chất lượng, kẹp vào khoảng JPEG cho phép.
fn trackbar_pos(h: HWND) -> u8 {
    // SAFETY: h là thanh trượt của cửa sổ đang mở ở luồng này; chỉ đọc vị trí.
    let pos = unsafe { SendMessageW(h, TBM_GETPOS, None, None).0 };
    clamp_jpeg_quality(pos as i64)
}

/// Ô số cạnh thanh trượt hiện vị trí hiện tại.
fn refresh_quality_text(c: &Ctl) {
    set_text(c.quality_value, &trackbar_pos(c.quality).to_string());
}

/// Nhãn, thanh trượt và ô số chất lượng sáng khi chọn JPEG, mờ khi chọn PNG; giá trị giữ nguyên.
fn apply_quality_enabled(c: &Ctl) {
    let format = if is_checked(c.fmt_jpeg) {
        UploadFormat::Jpeg
    } else {
        UploadFormat::Png
    };
    let on = quality_enabled(format);
    // SAFETY: ba điều khiển thuộc cửa sổ đang mở ở luồng này.
    unsafe {
        for h in [c.quality_label, c.quality, c.quality_value] {
            let _ = EnableWindow(h, on);
        }
    }
}

fn is_checked(h: HWND) -> bool {
    // SAFETY: h là ô tích của cửa sổ đang mở ở luồng này; chỉ đọc trạng thái.
    unsafe { SendMessageW(h, BM_GETCHECK, None, None).0 == 1 }
}

fn set_checked(h: HWND, on: bool) {
    // SAFETY: h là ô tích của cửa sổ đang mở ở luồng này.
    unsafe {
        SendMessageW(h, BM_SETCHECK, Some(WPARAM(usize::from(on))), None);
    }
}

/// Ô đầu tiên (theo thứ tự Tab) của mỗi tab.
fn first_ctl(c: &Ctl, t: Tab) -> HWND {
    match t {
        Tab::General => c.autostart,
        Tab::Hotkeys => c.hk_chk[0],
        // Nút chọn đang được chọn: đặt tiêu điểm vào nút kia sẽ tự chọn nó (BS_AUTORADIOBUTTON).
        Tab::Format => {
            if is_checked(c.fmt_jpeg) {
                c.fmt_jpeg
            } else {
                c.fmt_png
            }
        }
        Tab::R2 => c.account,
    }
}

fn field_ctl(c: &Ctl, f: Field) -> HWND {
    match f {
        Field::SaveDir => c.dir,
        Field::HotkeyRow(r) => c.hk_btn[row(r)],
        Field::AccountId => c.account,
        Field::Bucket => c.bucket,
        Field::PublicUrl => c.url,
    }
}

/// Hiện khung của tab `t`, ẩn và tắt các khung còn lại. Tiêu điểm đang nằm trong khung sắp ẩn
/// thì chuyển sang ô đầu của khung mới trước khi ẩn, nên ô của khung ẩn không bao giờ giữ tiêu điểm.
/// Không gửi thông điệp tới dải tab (bên gọi tự đặt lựa chọn của dải tab).
fn show_frame(c: &Ctl, t: Tab) {
    with_st(|s| s.tab = t);
    let shown = c.frames[t.index()];
    // SAFETY: các HWND là cửa sổ con của cửa sổ Tùy chọn đang mở ở luồng này.
    unsafe {
        let _ = EnableWindow(shown, true);
        let _ = ShowWindow(shown, SW_SHOW);
        let focus = GetFocus();
        let in_hidden = !focus.is_invalid()
            && c.frames
                .iter()
                .any(|f| *f != shown && (focus == *f || IsChild(*f, focus).as_bool()));
        if in_hidden {
            let _ = SetFocus(Some(first_ctl(c, t)));
        }
        for f in c.frames.iter().filter(|f| **f != shown) {
            let _ = ShowWindow(*f, SW_HIDE);
            let _ = EnableWindow(*f, false);
        }
    }
}

/// Chuyển tab bằng mã (Ctrl+Tab, lỗi khi OK): thoát chế độ bắt phím, đóng hộp chọn đang thả,
/// đặt lựa chọn của dải tab (TCM_SETCURSEL không sinh TCN_SELCHANGE) rồi hiện khung.
fn select_tab(t: Tab) {
    let Some(c) = ctl() else { return };
    end_capture();
    // SAFETY: gửi thông điệp tới các điều khiển của cửa sổ đang mở ở luồng này.
    unsafe {
        for combo in [c.expiry, c.language] {
            SendMessageW(combo, CB_SHOWDROPDOWN, Some(WPARAM(0)), None);
        }
        SendMessageW(c.tabs, TCM_SETCURSEL, Some(WPARAM(t.index())), None);
    }
    show_frame(&c, t);
}

/// Ctrl+Tab (`step` = 1) hoặc Ctrl+Shift+Tab (`step` = số tab - 1).
fn cycle_tab(step: usize) {
    if let Some(cur) = with_st(|s| s.tab) {
        select_tab(cur.cycle(step));
    }
}

/// Nền của vùng hiển thị dải tab: trắng khi có giao diện (visual styles), xám cổ điển khi không.
fn body_brush() -> HBRUSH {
    // SAFETY: hàm hệ thống không tham số; chổi màu hệ thống không phải giải phóng.
    unsafe {
        if IsAppThemed().as_bool() {
            GetSysColorBrush(COLOR_WINDOW)
        } else {
            GetSysColorBrush(COLOR_BTNFACE)
        }
    }
}

/// Thủ tục cửa sổ của khung: ô con báo WM_COMMAND, WM_CTLCOLORSTATIC và WM_HSCROLL (thanh trượt) cho khung (cha trực tiếp),
/// nên khung chuyển nguyên chúng lên cửa sổ chính. Nền khung tô bằng `body_brush`.
unsafe extern "system" fn frame_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_COMMAND | WM_CTLCOLORSTATIC | WM_HSCROLL => {
            // SAFETY: cha của khung là cửa sổ chính cùng luồng; SendMessageW gọi thẳng thủ tục của nó.
            unsafe {
                match GetParent(hwnd) {
                    Ok(p) => SendMessageW(p, msg, Some(wp), Some(lp)),
                    Err(_) => DefWindowProcW(hwnd, msg, wp, lp),
                }
            }
        }
        WM_ERASEBKGND | WM_PRINTCLIENT => {
            let hdc = HDC(wp.0 as *mut c_void);
            let mut rc = RECT::default();
            // SAFETY: hdc do hệ thống (hoặc nút vẽ nền cha) cấp cho lần vẽ này; rc là biến cục bộ.
            unsafe {
                let _ = GetClientRect(hwnd, &mut rc);
                FillRect(hdc, &rc, body_brush());
            }
            LRESULT(1)
        }
        // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
        _ => unsafe { DefWindowProcW(hwnd, msg, wp, lp) },
    }
}

fn show_error(msg: &str) {
    if let Some(c) = ctl() {
        set_text(c.err, msg);
    }
}

/// Ô chỉ đọc của dòng `r` hiện tổ hợp đang chọn trên biểu mẫu.
fn refresh_hotkey_text(r: HotkeyRole) {
    if let (Some(c), Some(hk)) = (ctl(), with_st(|s| s.hotkeys.get(r).key)) {
        set_text(c.hk_edit[row(r)], &hotkey::describe(&hk));
    }
}

/// Ô phím và nút "Đổi…" của dòng `r` sáng khi dòng bật, mờ khi tắt; tổ hợp giữ nguyên.
fn apply_row_enabled(c: &Ctl, r: HotkeyRole, on: bool) {
    // SAFETY: hai điều khiển thuộc cửa sổ đang mở ở luồng này.
    unsafe {
        let _ = EnableWindow(c.hk_edit[row(r)], on);
        let _ = EnableWindow(c.hk_btn[row(r)], on);
    }
}

/// Người dùng bấm ô tích của dòng `r`.
fn on_toggle_row(r: HotkeyRole) {
    let Some(c) = ctl() else { return };
    let on = is_checked(c.hk_chk[row(r)]);
    // Bỏ tích dòng đang bắt phím: thoát chế độ bắt trước (giữ phím cũ của dòng).
    if !on && with_st(|s| s.capturing == Some(r)).unwrap_or(false) {
        end_capture();
    }
    with_st(|s| {
        let mut slot = s.hotkeys.get(r);
        slot.enabled = on;
        s.hotkeys.set(r, slot);
    });
    apply_row_enabled(&c, r, on);
}

fn finish(result: Option<Config>) {
    with_st(|s| {
        s.result = result;
        s.done = true;
    });
}

fn cancel() {
    with_st(|s| {
        if s.hk_unregistered {
            // Đăng ký lại các dòng đang bật của cấu hình cũ (dòng tắt không đăng ký). Dòng bị chiếm
            // giữa chừng được giữ lại để bên gọi báo bằng thông báo gộp.
            let failed = hotkey::register_enabled(s.parent, &s.current.hotkeys);
            CANCEL_FAILED.with(|f| *f.borrow_mut() = failed);
            s.hk_unregistered = false;
        }
    });
    finish(None);
}

/// Chuyển sang tab chứa ô sai, đặt tiêu điểm vào ô đó và hiện câu lỗi; giá trị ở mọi tab giữ nguyên.
fn show_form_error(e: &FormError) {
    select_tab(tab_of(e.field));
    if let Some(c) = ctl() {
        // SAFETY: ô thuộc khung vừa hiện của cửa sổ đang mở ở luồng này.
        unsafe {
            let _ = SetFocus(Some(field_ctl(&c, e.field)));
        }
    }
    show_error(&e.message);
}

fn save() {
    let Some(form) = read_form() else { return };
    let Some(current) = with_st(|s| s.current.clone()) else {
        return;
    };
    let cfg = match apply_form(&current, &form, lang::current()) {
        Ok(c) => c,
        Err(e) => {
            show_form_error(&e);
            return;
        }
    };
    let Some((parent, need_rereg)) = with_st(|s| {
        (
            s.parent,
            s.hk_unregistered || cfg.hotkeys != s.current.hotkeys,
        )
    }) else {
        return;
    };
    if need_rereg {
        hotkey::unregister_all(parent);
        // Đăng ký lần lượt các dòng đang bật; dòng tắt không bao giờ được đăng ký.
        for r in HotkeyRole::ALL {
            let slot = cfg.hotkeys.get(r);
            if slot.enabled && !hotkey::register(parent, r, &slot.key) {
                // Không nửa vời: hủy mọi dòng đã đăng ký, chờ OK lần sau hoặc Hủy.
                hotkey::unregister_all(parent);
                with_st(|s| s.hk_unregistered = true);
                select_tab(Tab::Hotkeys);
                if let Some(c) = ctl() {
                    // SAFETY: nút "Đổi…" thuộc khung vừa hiện của cửa sổ đang mở ở luồng này.
                    unsafe {
                        let _ = SetFocus(Some(c.hk_btn[row(r)]));
                    }
                }
                let lang = lang::current();
                show_error(&i18n::hotkey_row_taken(
                    lang,
                    t(lang, r.label()),
                    &hotkey::describe(&slot.key),
                ));
                return;
            }
        }
        with_st(|s| s.hk_unregistered = false);
    }
    finish(Some(cfg));
}

fn on_command(hwnd: HWND, id: i32) {
    match id {
        ID_OK => save(),
        ID_CANCEL => cancel(),
        ID_TEST => on_test(hwnd),
        ID_BROWSE => on_browse(hwnd),
        ID_HELP => on_help(),
        ID_FMT_PNG | ID_FMT_JPEG => {
            if let Some(c) = ctl() {
                apply_quality_enabled(&c);
            }
        }
        _ => {
            for (i, r) in HotkeyRole::ALL.into_iter().enumerate() {
                if id == ID_HK_CHANGE0 + i as i32 {
                    on_change_hotkey(r);
                } else if id == ID_HK_CHECK0 + i as i32 {
                    on_toggle_row(r);
                }
            }
        }
    }
}

/// Mở Trợ giúp với các dòng phím tắt đang hiện trong biểu mẫu. Chủ của cửa sổ Trợ giúp là cửa sổ ẩn của
/// ứng dụng, không phải Tùy chọn, nên đóng Tùy chọn không kéo Trợ giúp theo.
fn on_help() {
    // Đang bắt phím mà bấm nút này: thoát chế độ bắt trước, như bấm ra ngoài.
    end_capture();
    // `St.hotkeys` theo sát ô tích (`on_toggle_row`) và phím vừa bắt; không đọc cả biểu mẫu
    // để khóa bí mật không bị chép ra chỉ vì mở Trợ giúp.
    if let Some((parent, keys)) = with_st(|s| (s.parent, s.hotkeys)) {
        crate::help::show(parent, &hotkey::help_keys(&keys));
    }
}

/// Bấm "Đổi…" của dòng `r`: dòng khác đang bắt thì thoát (giữ phím cũ của nó), rồi dòng `r` vào
/// chế độ bắt. Lần đầu trong phiên mở cửa sổ: hủy đăng ký cả ba dòng (Hủy sẽ đăng ký lại).
fn on_change_hotkey(r: HotkeyRole) {
    let Some(c) = ctl() else { return };
    if with_st(|s| s.capturing == Some(r)).unwrap_or(true) {
        return;
    }
    end_capture();
    let started = with_st(|s| {
        hotkey::unregister_all(s.parent);
        s.hk_unregistered = true;
        s.capturing = Some(r);
        s.cap = CaptureState::new();
    })
    .is_some();
    if started {
        set_text(c.hk_edit[row(r)], lang::tr(Msg::HotkeyCapturePrompt));
        show_error("");
    }
}

/// Thoát chế độ bắt phím, giữ phím tắt đang chọn trong biểu mẫu.
fn end_capture() {
    if let Some(r) = with_st(|s| s.capturing.take()).flatten() {
        refresh_hotkey_text(r);
    }
}

fn key_down(k: i32) -> bool {
    // SAFETY: chỉ đọc trạng thái phím.
    unsafe { GetKeyState(k) < 0 }
}

/// Xử lý thông điệp khi đang chọn phím tắt. Trả về `true` nếu đã nuốt thông điệp.
fn capture_msg(msg: &MSG, hk_btn: HWND) -> bool {
    match msg.message {
        WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_NCLBUTTONDOWN | WM_NCRBUTTONDOWN => {
            // Bấm ra ngoài nút "Đổi…": thoát chế độ bắt rồi xử lý cú bấm bình thường.
            if msg.hwnd != hk_btn {
                end_capture();
            }
            false
        }
        WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP => {
            let is_down = msg.message == WM_KEYDOWN || msg.message == WM_SYSKEYDOWN;
            let vk = msg.wParam.0 as u32;
            if is_down {
                if vk == VK_ESCAPE.0 as u32 {
                    end_capture();
                    return true;
                }
                // Tự lặp phím (kể cả phím giữ từ trước khi vào chế độ bắt): bỏ qua.
                if (msg.lParam.0 >> 30) & 1 == 1 {
                    return true;
                }
            }
            let (ctrl, alt, shift, win) = (
                key_down(VK_CONTROL_U),
                key_down(VK_MENU.0 as i32),
                key_down(VK_SHIFT.0 as i32),
                key_down(VK_LWIN.0 as i32) || key_down(VK_RWIN.0 as i32),
            );
            let ev = if is_down {
                KeyEv::Down {
                    vk,
                    ctrl,
                    alt,
                    shift,
                    win,
                }
            } else {
                KeyEv::Up {
                    vk,
                    ctrl,
                    alt,
                    shift,
                    win,
                }
            };
            let Some(out) = with_st(|s| s.cap.feed(ev)) else {
                return false;
            };
            match out {
                Capture::Ignore => true,
                Capture::PassThrough => false,
                Capture::NeedModifier => {
                    if let (Some(c), Some(Some(r))) = (ctl(), with_st(|s| s.capturing)) {
                        set_text(c.hk_edit[row(r)], lang::tr(Msg::HotkeyNeedModifier));
                    }
                    true
                }
                Capture::Got(h) => {
                    if let Some(r) = with_st(|s| s.capturing.take()).flatten() {
                        with_st(|s| {
                            let mut slot = s.hotkeys.get(r);
                            slot.key = h;
                            s.hotkeys.set(r, slot);
                        });
                        refresh_hotkey_text(r);
                    }
                    true
                }
            }
        }
        _ => false,
    }
}

fn on_browse(hwnd: HWND) {
    let Some(c) = ctl() else { return };
    // SAFETY: COM được khởi tạo và giải phóng cân bằng trong hàm này; chuỗi do COM cấp được giải phóng bằng CoTaskMemFree.
    let picked: Option<String> = unsafe {
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let inited = hr.is_ok();
        let r = (|| -> windows::core::Result<String> {
            let dlg: IFileOpenDialog =
                CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)?;
            let opts = dlg.GetOptions()?;
            dlg.SetOptions(opts | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM)?;
            dlg.SetTitle(&HSTRING::from(lang::tr(Msg::PickFolderTitle)))?;
            dlg.Show(Some(hwnd))?;
            let item = dlg.GetResult()?;
            let name: PWSTR = item.GetDisplayName(SIGDN_FILESYSPATH)?;
            let s = name.to_string();
            CoTaskMemFree(Some(name.0 as *const c_void));
            Ok(s.unwrap_or_default())
        })();
        if inited {
            CoUninitialize();
        }
        r.ok().filter(|s| !s.is_empty())
    };
    if let Some(p) = picked {
        set_text(c.dir, &p);
    }
}

fn on_test(hwnd: HWND) {
    let Some(c) = ctl() else { return };
    let (Some(form), Some(current)) = (read_form(), with_st(|s| s.current.clone())) else {
        return;
    };
    let cfg = match apply_form(&current, &form, lang::current()) {
        Ok(c) => c,
        Err(e) => {
            // Ô sai có thể ở tab khác (ví dụ Thư mục lưu trống): đưa người dùng tới đúng ô.
            show_form_error(&e);
            return;
        }
    };
    show_error("");
    if !cfg.r2.is_complete() {
        set_text(c.test_res, lang::tr(Msg::TestFillAll));
        return;
    }
    if TEST_RUNNING.with(|r| r.get()) {
        return;
    }
    // Thế hệ mới: kết quả của lần chạy trước (nếu có) sẽ bị bỏ.
    let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    let target = hwnd.0 as isize;
    let r2 = cfg.r2;
    let spawned = std::thread::Builder::new()
        .name("shotme-test-connection".into())
        .spawn(move || {
            let msg = match R2Uploader::new(&r2).test_connection() {
                Ok(()) => lang::tr(Msg::TestSuccess).to_string(),
                Err(e) => e.message(lang::current()),
            };
            if let Ok(mut slot) = TEST_RESULT.lock() {
                // Cửa sổ đã đóng (thế hệ đã đổi): bỏ kết quả, không đè lên kết quả của cửa sổ mới.
                if generation == GENERATION.load(Ordering::SeqCst) {
                    *slot = Some((generation, msg));
                }
            }
            // SAFETY: nếu cửa sổ đã đóng thì PostMessageW trả lỗi và ta bỏ qua; kết quả nằm trong mutex, không có con trỏ treo.
            unsafe {
                let _ = PostMessageW(
                    Some(HWND(target as *mut c_void)),
                    WM_APP_TEST_DONE,
                    WPARAM(0),
                    LPARAM(0),
                );
            }
        });
    if spawned.is_err() {
        set_text(c.test_res, lang::tr(Msg::TestThreadFailed));
        return;
    }
    TEST_RUNNING.with(|r| r.set(true));
    // SAFETY: chuyển focus sang ô còn bật trước khi làm mờ nút; bộ hẹn giờ dự phòng gắn với cửa sổ này.
    unsafe {
        if GetFocus() == c.test_btn {
            let _ = SetFocus(Some(c.account));
        }
        let _ = EnableWindow(c.test_btn, false);
        SetTimer(Some(hwnd), TEST_TIMER_ID, TEST_FAILSAFE_MS, None);
    }
    set_text(c.test_res, lang::tr(Msg::TestRunning));
}

/// Bật lại nút "Kiểm tra kết nối" và hủy bộ hẹn giờ dự phòng.
fn test_finished(hwnd: HWND, c: &Ctl) {
    TEST_RUNNING.with(|r| r.set(false));
    // SAFETY: bật lại nút và hủy bộ hẹn giờ của chính cửa sổ này.
    unsafe {
        let _ = KillTimer(Some(hwnd), TEST_TIMER_ID);
        let _ = EnableWindow(c.test_btn, true);
    }
}

fn on_test_timeout(hwnd: HWND) {
    let Some(c) = ctl() else { return };
    if !TEST_RUNNING.with(|r| r.get()) {
        return;
    }
    // Bỏ kết quả muộn của luồng nền.
    GENERATION.fetch_add(1, Ordering::SeqCst);
    set_text(c.test_res, lang::tr(Msg::TestTimeout));
    test_finished(hwnd, &c);
}

fn on_test_done(hwnd: HWND) {
    let Some(c) = ctl() else { return };
    let taken = TEST_RESULT.lock().ok().and_then(|mut s| s.take());
    let Some((generation, msg)) = taken else {
        return;
    };
    if generation != GENERATION.load(Ordering::SeqCst) {
        return;
    }
    set_text(c.test_res, &msg);
    test_finished(hwnd, &c);
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let code = (wp.0 >> 16) & 0xFFFF;
            if code == 0 {
                on_command(hwnd, (wp.0 & 0xFFFF) as i32);
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            cancel();
            LRESULT(0)
        }
        WM_HSCROLL => {
            // Thanh trượt chất lượng (do khung chuyển lên): số đổi theo ngay khi kéo và khi bấm phím.
            let from = HWND(lp.0 as *mut c_void);
            if let Some(c) = ctl() {
                if from == c.quality {
                    refresh_quality_text(&c);
                }
            }
            LRESULT(0)
        }
        WM_APP_TEST_DONE => {
            on_test_done(hwnd);
            LRESULT(0)
        }
        WM_TIMER if wp.0 == TEST_TIMER_ID => {
            on_test_timeout(hwnd);
            LRESULT(0)
        }
        WM_ACTIVATE => {
            // Mất kích hoạt: thoát chế độ bắt phím (rồi để hệ thống xử lý tiếp, giữ focus đúng).
            if (wp.0 & 0xFFFF) as u32 == WA_INACTIVE {
                end_capture();
            }
            // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
            unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
        }
        WM_NOTIFY => {
            // Thông báo của dải tab (con trực tiếp của cửa sổ này).
            let hdr = lp.0 as *const NMHDR;
            if let (Some(c), false) = (ctl(), hdr.is_null()) {
                // SAFETY: với WM_NOTIFY, lParam trỏ tới NMHDR hợp lệ suốt lời gọi.
                let (from, code) = unsafe { ((*hdr).hwndFrom, (*hdr).code) };
                if from == c.tabs && code == TCN_SELCHANGING {
                    // Đổi tab (chuột hoặc mũi tên) lúc đang bắt phím: thoát chế độ bắt trước.
                    end_capture();
                    return LRESULT(0);
                }
                if from == c.tabs && code == TCN_SELCHANGE {
                    // SAFETY: hỏi tab đang chọn của dải tab thuộc cửa sổ này.
                    let sel = unsafe { SendMessageW(c.tabs, TCM_GETCURSEL, None, None).0 };
                    if let Some(t) = usize::try_from(sel).ok().and_then(|i| Tab::ALL.get(i)) {
                        show_frame(&c, *t);
                    }
                    return LRESULT(0);
                }
            }
            // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
            unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
        }
        WM_CTLCOLORSTATIC => {
            let from = HWND(lp.0 as *mut c_void);
            let hdc = HDC(wp.0 as *mut c_void);
            if let Some(c) = ctl() {
                if from == c.err {
                    // SAFETY: hdc do hệ thống cấp cho lần vẽ này; nền lấy từ màu hệ thống.
                    unsafe {
                        SetTextColor(hdc, windows::Win32::Foundation::COLORREF(0x0000_00B4));
                        SetBkMode(hdc, TRANSPARENT);
                        let b: HBRUSH = GetSysColorBrush(COLOR_BTNFACE);
                        return LRESULT(b.0 as isize);
                    }
                }
                // Nhãn và ô tích trong khung (do khung chuyển lên): nền trùng nền dải tab.
                // Ô chỉ đọc của phím tắt giữ nền mặc định để không trông như ô gõ được.
                // SAFETY: IsChild chỉ đọc quan hệ cửa sổ.
                let in_frame = !c.hk_edit.contains(&from)
                    && c.frames
                        .iter()
                        .any(|f| unsafe { IsChild(*f, from) }.as_bool());
                if in_frame {
                    // SAFETY: hdc do hệ thống cấp cho lần vẽ này; chổi màu hệ thống không phải giải phóng.
                    unsafe {
                        SetBkMode(hdc, TRANSPARENT);
                    }
                    return LRESULT(body_brush().0 as isize);
                }
            }
            // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
            unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
        }
        // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
        _ => unsafe { DefWindowProcW(hwnd, msg, wp, lp) },
    }
}

struct Builder {
    parent: HWND,
    dpi: i32,
    font: Option<HFONT>,
    /// Bố cục theo bề rộng (DIP), tính một lần bằng `Layout::measure` trước khi dựng ô.
    layout: Layout,
}

/// Bề rộng (DIP) suy ra từ nhãn dài nhất của tab Phím tắt: cửa sổ nới ra vừa đủ để ô phím của
/// mỗi dòng rộng ít nhất `HK_EDIT_MIN_W`; mọi tab và hàng nút giãn theo `client_w`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Layout {
    /// Bề rộng vùng client của cửa sổ.
    client_w: i32,
    /// Bề rộng ô nhập ở các tab (cột bắt đầu từ `EX`).
    ew: i32,
    /// Bề rộng ô tích mang nhãn của dòng phím tắt.
    hk_chk_w: i32,
}

/// Bề rộng tối thiểu của vùng client (bố cục đợt 1) và của ô phím trên tab Phím tắt.
const CLIENT_MIN_W: i32 = 480;
const HK_EDIT_MIN_W: i32 = 194;
/// Lề phải từ mép ô nhập tới mép vùng client (dải tab, viền khung, lề trong).
const RIGHT_MARGIN: i32 = 50;
/// Ô vuông của ô tích và khoảng cách tới chữ; bề rộng nút "Đổi…"; khe giữa các ô trong một dòng.
const HK_BOX: i32 = 24;
const HK_BTN_W: i32 = 70;
const HK_GAP: i32 = 6;
/// Phần cộng thêm vào bề rộng chữ của mỗi tab (lề trong của mục tab, rộng rãi hơn mức mặc định
/// 2 × 6 của Common Controls) và phần dải tab không dùng cho mục (tab đang chọn nhô ra hai bên).
const TAB_ITEM_PAD: i32 = 24;
const TAB_STRIP_SLACK: i32 = 8;

impl Layout {
    /// `label_w`: bề rộng chữ (DIP) của nhãn dài nhất trong mọi ngôn ngữ, `None` khi không đo được.
    fn from_label_width(label_w: Option<i32>) -> Layout {
        let hk_chk_w = (label_w.unwrap_or(176) + HK_BOX).clamp(142, 240);
        // Mép phải của nút "Đổi…" khi ô phím rộng đúng `HK_EDIT_MIN_W`.
        let need_right = LX + hk_chk_w + HK_GAP + HK_EDIT_MIN_W + HK_GAP + HK_BTN_W;
        let ew = (CLIENT_MIN_W - RIGHT_MARGIN - EX).max(need_right - EX);
        Layout {
            client_w: EX + ew + RIGHT_MARGIN,
            ew,
            hk_chk_w,
        }
    }

    /// Bề rộng dành cho các mục của dải tab.
    /// Dải tab rộng `client_w - 24` (lề 12 mỗi bên); trừ thêm `TAB_STRIP_SLACK` cho phần nhô của tab
    /// đang chọn và lề trong của dải.
    fn tab_strip_w(self) -> i32 {
        self.client_w - 24 - TAB_STRIP_SLACK
    }

    /// Nới `client_w` (và ô nhập theo cùng lượng) khi `tab_widths` không vừa một hàng.
    fn widen_for_tabs(self, tab_widths: &[i32]) -> Layout {
        if tabs_fit(tab_widths, self.tab_strip_w()) {
            return self;
        }
        let total: i32 = tab_widths.iter().sum();
        let extra = total - self.tab_strip_w();
        Layout {
            client_w: self.client_w + extra,
            ew: self.ew + extra,
            hk_chk_w: self.hk_chk_w,
        }
    }
}

impl Builder {
    fn s(&self, v: i32) -> i32 {
        (v * self.dpi + 48) / 96
    }

    #[allow(clippy::too_many_arguments)]
    fn make(
        &mut self,
        class: PCWSTR,
        text: &str,
        style: WINDOW_STYLE,
        (x, y, w, h): (i32, i32, i32, i32),
        id: i32,
    ) -> HWND {
        // SAFETY: tạo cửa sổ con của cửa sổ Tùy chọn ở luồng chính; lỗi thì trả cửa sổ rỗng (các hàm sau chịu được).
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                class,
                &HSTRING::from(text),
                WS_CHILD | WS_VISIBLE | style,
                self.s(x),
                self.s(y),
                self.s(w),
                self.s(h),
                Some(self.parent),
                Some(HMENU(id as usize as *mut c_void)),
                None,
                None,
            )
            .unwrap_or_default()
        };
        if let (Some(f), false) = (self.font, hwnd.is_invalid()) {
            // SAFETY: đặt phông cho điều khiển vừa tạo; phông sống đến khi đóng cửa sổ.
            unsafe {
                SendMessageW(
                    hwnd,
                    WM_SETFONT,
                    Some(WPARAM(f.0 as usize)),
                    Some(LPARAM(1)),
                );
            }
        }
        hwnd
    }

    /// Khung của một tab: con của cửa sổ chính, ẩn và tắt lúc tạo (`show_frame` bật khung đang chọn).
    /// `WS_EX_CONTROLPARENT` để Tab/Shift+Tab của hộp thoại đi vào các ô bên trong.
    fn frame(&mut self, id: i32) -> HWND {
        // SAFETY: tạo cửa sổ con của cửa sổ Tùy chọn ở luồng chính; lớp đã đăng ký trong `show`.
        unsafe {
            let hinst = GetModuleHandleW(None).ok();
            CreateWindowExW(
                WS_EX_CONTROLPARENT,
                FRAME_CLASS,
                PCWSTR::null(),
                WS_CHILD | WS_DISABLED,
                0,
                0,
                0,
                0,
                Some(self.parent),
                Some(HMENU(id as usize as *mut c_void)),
                hinst.map(|h| h.into()),
                None,
            )
            .unwrap_or_default()
        }
    }

    fn label(&mut self, text: &str, x: i32, y: i32, w: i32, h: i32) -> HWND {
        self.make(w!("STATIC"), text, WINDOW_STYLE(0), (x, y, w, h), -1)
    }

    fn edit(&mut self, style: WINDOW_STYLE, rect: (i32, i32, i32, i32), id: i32) -> HWND {
        self.make(
            w!("EDIT"),
            "",
            WS_GROUP | WS_TABSTOP | ws(ES_AUTOHSCROLL) | style | WS_BORDER,
            rect,
            id,
        )
    }

    fn button(
        &mut self,
        text: &str,
        style: WINDOW_STYLE,
        rect: (i32, i32, i32, i32),
        id: i32,
    ) -> HWND {
        self.make(w!("BUTTON"), text, WS_GROUP | WS_TABSTOP | style, rect, id)
    }
}

/// Lề trái nhãn, lề trái ô nhập và bề rộng ô nhập trong một khung (DIP, tính từ góc khung).
const LX: i32 = 12;
const EX: i32 = 160;

/// Dòng nhãn + ô nhập một dòng trong khung; trả về ô nhập.
fn row_edit(b: &mut Builder, label: &str, y: i32, w: i32, style: WINDOW_STYLE, id: i32) -> HWND {
    b.label(label, LX, y + 3, EX - LX - 4, 20);
    b.edit(style, (EX, y, w, 24), id)
}

fn combo(b: &mut Builder, y: i32, id: i32) -> HWND {
    b.make(
        w!("COMBOBOX"),
        "",
        WS_GROUP | WS_TABSTOP | WS_VSCROLL | WINDOW_STYLE(CBS_DROPDOWNLIST as u32),
        (EX, y, 160, 200),
        id,
    )
}

struct GeneralCtl {
    autostart: HWND,
    notices: HWND,
    keep_selection: HWND,
    include_cursor: HWND,
    dir: HWND,
    language: HWND,
    height: i32,
}

/// Ô của tab Thông thường.
fn build_general(b: &mut Builder) -> GeneralCtl {
    let mut y = 12;
    let autostart = b.button(
        lang::tr(Msg::ChkAutostart),
        ws(BS_AUTOCHECKBOX),
        (LX, y, 300, 22),
        ID_AUTOSTART,
    );
    // Ba ô tích của đợt 2, liền nhau ngay dưới "Khởi động cùng Windows".
    let mut check = |b: &mut Builder, msg: Msg, id: i32| {
        y += 26;
        let w = b.layout.client_w - 60;
        b.button(lang::tr(msg), ws(BS_AUTOCHECKBOX), (LX, y, w, 22), id)
    };
    let notices = check(b, Msg::ChkShowSuccessNotices, ID_NOTICES);
    let keep_selection = check(b, Msg::ChkKeepSelection, ID_KEEP_SELECTION);
    let include_cursor = check(b, Msg::ChkIncludeCursor, ID_INCLUDE_CURSOR);
    y += 32;
    let dir = row_edit(
        b,
        lang::tr(Msg::LabelSaveDir),
        y,
        b.layout.ew - 76,
        WINDOW_STYLE(0),
        ID_DIR,
    );
    b.button(
        lang::tr(Msg::BtnChoose),
        ws(BS_PUSHBUTTON),
        (EX + b.layout.ew - 70, y - 1, 70, 26),
        ID_BROWSE,
    );
    y += 34;
    b.label(lang::tr(Msg::LabelLanguage), LX, y + 3, EX - LX - 4, 20);
    let language = combo(b, y, ID_LANGUAGE);
    y += 32;
    GeneralCtl {
        autostart,
        notices,
        keep_selection,
        include_cursor,
        dir,
        language,
        height: y + 4,
    }
}

struct HotkeysCtl {
    chk: [HWND; 3],
    edit: [HWND; 3],
    btn: [HWND; 3],
    height: i32,
}

/// Bề rộng (DIP) của chuỗi dài nhất trong `texts` khi vẽ bằng phông của cửa sổ; `None` khi không đo được.
fn max_text_width_dip(b: &Builder, texts: &[&str]) -> Option<i32> {
    let font = b.font?;
    // SAFETY: DC màn hình lấy và trả trong khối này; phông cũ được chọn lại trước khi trả DC.
    unsafe {
        let hdc = GetDC(None);
        if hdc.is_invalid() {
            return None;
        }
        let old = SelectObject(hdc, HGDIOBJ(font.0));
        let mut widest = 0;
        for s in texts {
            let w: Vec<u16> = s.encode_utf16().collect();
            let mut size = SIZE::default();
            if GetTextExtentPoint32W(hdc, &w, &mut size).as_bool() {
                widest = widest.max(size.cx);
            }
        }
        SelectObject(hdc, old);
        ReleaseDC(None, hdc);
        Some((widest * 96 + b.dpi - 1) / b.dpi.max(1))
    }
}

/// Ô của tab Phím tắt: mỗi dòng theo `HotkeyRole::ALL` có ô tích (nhãn của dòng), ô chỉ đọc, nút "Đổi…".
/// Cột nhãn đủ rộng cho nhãn dài nhất của mọi ngôn ngữ; ô phím lấy phần còn lại.
fn build_hotkeys(b: &mut Builder) -> HotkeysCtl {
    let right = EX + b.layout.ew;
    let chk_w = b.layout.hk_chk_w;
    let edit_x = LX + chk_w + HK_GAP;
    let edit_w = right - HK_BTN_W - HK_GAP - edit_x;
    let mut y = 12;
    let (mut chk, mut edit, mut btn) = (
        [HWND::default(); 3],
        [HWND::default(); 3],
        [HWND::default(); 3],
    );
    for (i, r) in HotkeyRole::ALL.into_iter().enumerate() {
        chk[i] = b.button(
            lang::tr(r.label()),
            ws(BS_AUTOCHECKBOX),
            (LX, y + 1, chk_w, 22),
            ID_HK_CHECK0 + i as i32,
        );
        edit[i] = b.edit(
            WINDOW_STYLE(ES_READONLY as u32),
            (edit_x, y, edit_w, 24),
            -1,
        );
        btn[i] = b.button(
            lang::tr(Msg::BtnChange),
            ws(BS_PUSHBUTTON),
            (right - HK_BTN_W, y - 1, HK_BTN_W, 26),
            ID_HK_CHANGE0 + i as i32,
        );
        y += 34;
    }
    HotkeysCtl {
        chk,
        edit,
        btn,
        height: y + 4,
    }
}

struct FormatCtl {
    png: HWND,
    jpeg: HWND,
    quality_label: HWND,
    quality: HWND,
    quality_value: HWND,
    height: i32,
}

/// Ô của tab Định dạng. Thứ tự Tab: nhóm nút chọn (một điểm dừng, mũi tên đổi giữa hai nút), rồi
/// thanh trượt. Nhãn chất lượng đứng ngay trước thanh trượt để trình đọc màn hình lấy làm tên.
fn build_format(b: &mut Builder) -> FormatCtl {
    let w = b.layout.client_w - 60;
    let mut y = 12;
    b.label(lang::tr(Msg::LabelUploadFormat), LX, y, w, 20);
    y += 24;
    let png = b.button(
        lang::tr(UploadFormat::Png.label()),
        ws(BS_AUTORADIOBUTTON),
        (LX + 12, y, w - 12, 22),
        ID_FMT_PNG,
    );
    y += 26;
    // Nút thứ hai cùng nhóm: không WS_GROUP, không WS_TABSTOP.
    let jpeg = b.make(
        w!("BUTTON"),
        lang::tr(UploadFormat::Jpeg.label()),
        ws(BS_AUTORADIOBUTTON),
        (LX + 12, y, w - 12, 22),
        ID_FMT_JPEG,
    );
    y += 36;
    // WS_GROUP khép nhóm nút chọn ở trên.
    let quality_label = b.make(
        w!("STATIC"),
        lang::tr(Msg::LabelJpegQuality),
        WS_GROUP,
        (LX, y + 5, EX - LX - 4, 20),
        -1,
    );
    let value_w = 40;
    let quality = b.make(
        TRACKBAR_CLASSW,
        "",
        WS_GROUP | WS_TABSTOP | ws(TBS_HORZ as i32),
        (EX, y, b.layout.ew - value_w - HK_GAP, 30),
        ID_QUALITY,
    );
    let quality_value = b.make(
        w!("STATIC"),
        "",
        WINDOW_STYLE(0),
        (EX + b.layout.ew - value_w, y + 5, value_w, 20),
        ID_QUALITY_VALUE,
    );
    y += 40;
    b.label(lang::tr(Msg::NoteSavedAlwaysPng), LX, y, w, 20);
    y += 28;
    FormatCtl {
        png,
        jpeg,
        quality_label,
        quality,
        quality_value,
        height: y + 4,
    }
}

struct R2Ctl {
    account: HWND,
    keyid: HWND,
    secret: HWND,
    bucket: HWND,
    url: HWND,
    expiry: HWND,
    test_btn: HWND,
    test_res: HWND,
    height: i32,
}

/// Ô của tab Lưu trữ R2. "Account ID", "Access Key ID", "Secret Access Key" là tên riêng, không dịch.
fn build_r2(b: &mut Builder, current: &Config) -> R2Ctl {
    let ew = b.layout.ew;
    let mut y = 12;
    let account = row_edit(b, "Account ID", y, ew, WINDOW_STYLE(0), ID_ACCOUNT);
    y += 28;
    let keyid = row_edit(b, "Access Key ID", y, ew, WINDOW_STYLE(0), ID_KEYID);
    y += 28;
    let secret = row_edit(
        b,
        "Secret Access Key",
        y,
        ew,
        WINDOW_STYLE(ES_PASSWORD as u32),
        ID_SECRET,
    );
    y += 28;
    let hint = if current.r2.secret_access_key.is_empty() {
        lang::tr(Msg::SecretNone)
    } else {
        lang::tr(Msg::SecretSaved)
    };
    b.label(hint, EX, y - 2, ew, 18);
    y += 20;
    let bucket = row_edit(
        b,
        lang::tr(Msg::LabelBucket),
        y,
        ew,
        WINDOW_STYLE(0),
        ID_BUCKET,
    );
    y += 28;
    let url = row_edit(
        b,
        lang::tr(Msg::LabelPublicUrl),
        y,
        ew,
        WINDOW_STYLE(0),
        ID_URL,
    );
    y += 28;
    b.label(lang::tr(Msg::LabelExpiry), LX, y + 3, EX - LX - 4, 20);
    let expiry = combo(b, y, ID_EXPIRY);
    y += 32;
    let test_btn = b.button(
        lang::tr(Msg::BtnTest),
        ws(BS_PUSHBUTTON),
        (LX, y, 140, 26),
        ID_TEST,
    );
    let test_res = b.label("", EX, y + 4, ew, 36);
    y += 44;
    R2Ctl {
        account,
        keyid,
        secret,
        bucket,
        url,
        expiry,
        test_btn,
        test_res,
        height: y + 4,
    }
}

/// Dựng các điều khiển; trả về chiều cao vùng client (đơn vị DIP) và bộ HWND.
///
/// Thứ tự Z (cũng là thứ tự Tab của `IsDialogMessageW`): khung các tab, dòng lỗi, Trợ giúp, OK, Hủy, rồi
/// dải tab ở đáy. Vòng Tab vì vậy là: dải tab → ô của khung đang hiện → Trợ giúp → OK → Hủy → dải tab. Dải tab có
/// `WS_CLIPSIBLINGS` và nằm dưới các khung nên không vẽ đè lên chúng.
fn build_controls(b: &mut Builder, current: &Config, notice: Option<&str>) -> (i32, Option<Ctl>) {
    let main = b.parent;
    let labels: Vec<&str> = HotkeyRole::ALL
        .iter()
        .flat_map(|r| Lang::ALL.map(|l| t(l, r.label())))
        .collect();
    let mut layout = Layout::from_label_width(max_text_width_dip(b, &labels));
    // Dải tab bốn mục phải vừa một hàng ở mọi ngôn ngữ: đo nhãn tab của từng ngôn ngữ và nới nếu cần.
    for l in Lang::ALL {
        let widths: Option<Vec<i32>> = Tab::ALL
            .iter()
            .map(|tab| max_text_width_dip(b, &[t(l, tab.label())]).map(|w| w + TAB_ITEM_PAD))
            .collect();
        if let Some(tw) = widths {
            layout = layout.widen_for_tabs(&tw);
        }
    }
    b.layout = layout;
    let inner_w = b.layout.client_w - 24;
    let mut y = 12;
    if let Some(n) = notice {
        b.label(n, 12, y, inner_w, 34);
        y += 42;
    }
    let tab_top = y;

    // Dải tab: phải có phông và các mục trước khi hỏi TCM_ADJUSTRECT.
    let tabs = b.make(
        WC_TABCONTROLW,
        "",
        WS_GROUP | WS_TABSTOP | WS_CLIPSIBLINGS,
        (12, tab_top, inner_w, 300),
        ID_TABS,
    );
    for (i, t) in Tab::ALL.iter().enumerate() {
        let mut text: Vec<u16> = lang::tr(t.label())
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let item = TCITEMW {
            mask: TCIF_TEXT,
            pszText: PWSTR(text.as_mut_ptr()),
            ..Default::default()
        };
        // SAFETY: `item` và bộ đệm chữ sống suốt lời gọi; dải tab tự chép chữ.
        unsafe {
            SendMessageW(
                tabs,
                TCM_INSERTITEMW,
                Some(WPARAM(i)),
                Some(LPARAM(&item as *const TCITEMW as isize)),
            );
        }
    }

    // Bốn khung (ẩn, tắt) và ô của chúng; kích thước khung đặt sau khi biết tab cao nhất.
    let frames = [
        b.frame(ID_FRAME0),
        b.frame(ID_FRAME0 + 1),
        b.frame(ID_FRAME0 + 2),
        b.frame(ID_FRAME0 + 3),
    ];
    b.parent = frames[Tab::General.index()];
    let g = build_general(b);
    b.parent = frames[Tab::Hotkeys.index()];
    let hk = build_hotkeys(b);
    b.parent = frames[Tab::Format.index()];
    let fmt = build_format(b);
    b.parent = frames[Tab::R2.index()];
    let r2 = build_r2(b, current);
    b.parent = main;
    // Cửa sổ cao theo tab cao nhất.
    let frame_h = g.height.max(hk.height).max(fmt.height).max(r2.height);

    // Vùng hiển thị của dải tab cao đúng bằng tab cao nhất; khung phủ kín vùng đó.
    let (left, top, width) = (b.s(12), b.s(tab_top), b.s(inner_w));
    let probe_h = b.s(300);
    let mut probe = RECT {
        left,
        top,
        right: left + width,
        bottom: top + probe_h,
    };
    // SAFETY: probe là biến cục bộ; dải tab chỉ đổi giá trị của nó.
    unsafe {
        SendMessageW(
            tabs,
            TCM_ADJUSTRECT,
            Some(WPARAM(0)),
            Some(LPARAM(&mut probe as *mut RECT as isize)),
        );
    }
    let bottom_inset = (top + probe_h - probe.bottom).max(0);
    let disp = RECT {
        left: probe.left,
        top: probe.top.max(top),
        right: probe.right.max(probe.left),
        bottom: probe.top.max(top) + b.s(frame_h),
    };
    let tab_bottom = disp.bottom + bottom_inset;
    // SAFETY: đổi vị trí và kích thước các cửa sổ con vừa tạo.
    unsafe {
        for f in frames {
            let _ = SetWindowPos(
                f,
                None,
                disp.left,
                disp.top,
                disp.right - disp.left,
                disp.bottom - disp.top,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
        let _ = SetWindowPos(
            tabs,
            None,
            left,
            top,
            width,
            tab_bottom - top,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }

    // Dòng lỗi và hàng nút, dưới dải tab. Nút Trợ giúp ở x = 12; OK và Hủy ở bên phải.
    y = (tab_bottom * 96 + b.dpi - 1) / b.dpi.max(1) + 8;
    let err = b.label("", 12, y, inner_w, 36);
    y += 40;
    // Nút Trợ giúp tạo ngay trước OK để đứng liền trước OK trong thứ tự Tab; không phải nút mặc định.
    b.button(
        lang::tr(Msg::BtnHelp),
        ws(BS_PUSHBUTTON),
        (12, y, 80, 28),
        ID_HELP,
    );
    b.button(
        i18n::OK_LABEL,
        ws(BS_DEFPUSHBUTTON),
        (b.layout.client_w - 180, y, 80, 28),
        ID_OK,
    );
    // Hủy không có WS_GROUP: OK và Hủy cùng một nhóm mũi tên.
    b.make(
        w!("BUTTON"),
        lang::tr(Msg::BtnCancel),
        WS_TABSTOP | ws(BS_PUSHBUTTON),
        (b.layout.client_w - 92, y, 80, 28),
        ID_CANCEL,
    );
    y += 40;
    // Đặt lại dải tab xuống đáy thứ tự Z sau khi mọi cửa sổ con đã có.
    // SAFETY: đổi thứ tự Z của dải tab vừa tạo.
    unsafe {
        let _ = SetWindowPos(
            tabs,
            Some(HWND_BOTTOM),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }

    let all = [
        tabs,
        frames[0],
        frames[1],
        frames[2],
        frames[3],
        fmt.png,
        fmt.jpeg,
        fmt.quality_label,
        fmt.quality,
        fmt.quality_value,
        g.language,
        r2.account,
        r2.keyid,
        r2.secret,
        r2.bucket,
        r2.url,
        r2.test_btn,
        r2.test_res,
        r2.expiry,
        g.dir,
        g.autostart,
        g.notices,
        g.keep_selection,
        g.include_cursor,
        err,
    ];
    if all
        .iter()
        .chain(hk.chk.iter())
        .chain(hk.edit.iter())
        .chain(hk.btn.iter())
        .any(|h| h.is_invalid())
    {
        return (y, None);
    }
    let ctl = Ctl {
        tabs,
        frames,
        language: g.language,
        fmt_png: fmt.png,
        fmt_jpeg: fmt.jpeg,
        quality_label: fmt.quality_label,
        quality: fmt.quality,
        quality_value: fmt.quality_value,
        account: r2.account,
        keyid: r2.keyid,
        secret: r2.secret,
        bucket: r2.bucket,
        url: r2.url,
        test_btn: r2.test_btn,
        test_res: r2.test_res,
        expiry: r2.expiry,
        dir: g.dir,
        hk_chk: hk.chk,
        hk_edit: hk.edit,
        hk_btn: hk.btn,
        autostart: g.autostart,
        notices: g.notices,
        keep_selection: g.keep_selection,
        include_cursor: g.include_cursor,
        err,
    };
    (y, Some(ctl))
}

/// Đưa cửa sổ Tùy chọn đang mở (nếu có) ra trước; không làm gì khác.
pub fn focus_existing() {
    let open = OPEN.with(|o| o.get());
    if open == 0 {
        return;
    }
    let h = HWND(open as *mut c_void);
    // SAFETY: `h` là cửa sổ Tùy chọn của luồng này, OPEN chỉ khác 0 khi cửa sổ còn sống.
    unsafe {
        if IsIconic(h).as_bool() {
            let _ = ShowWindow(h, SW_RESTORE);
        }
        let _ = SetForegroundWindow(h);
    }
}

/// Các dòng phím tắt không đăng ký lại được khi lượt `show` vừa rồi kết thúc bằng Hủy (rỗng nếu lượt
/// đó không phải đăng ký lại, hoặc đăng ký lại đủ). Lấy ra một lần.
pub fn take_cancel_failures() -> Vec<HotkeyRole> {
    CANCEL_FAILED.with(|f| std::mem::take(&mut *f.borrow_mut()))
}

/// Mở cửa sổ Tùy chọn và chờ đến khi người dùng bấm OK hoặc Hủy. `None` khi Hủy (hoặc đã có cửa sổ mở).
pub fn show(
    parent: HWND,
    current: &Config,
    notice: Option<&str>,
    reason: OpenReason,
) -> Option<SettingsResult> {
    let open = OPEN.with(|o| o.get());
    if open != 0 {
        let h = HWND(open as *mut c_void);
        // SAFETY: đưa cửa sổ Tùy chọn đang có ra trước.
        unsafe {
            if IsIconic(h).as_bool() {
                let _ = ShowWindow(h, SW_RESTORE);
            }
            let _ = SetForegroundWindow(h);
        }
        return None;
    }
    CANCEL_FAILED.with(|f| f.borrow_mut().clear());

    let title = HSTRING::from(lang::tr(Msg::OptionsTitle));
    // SAFETY: đăng ký lớp và tạo cửa sổ ở luồng chính; mọi handle được giải phóng ở cuối hàm.
    let hwnd = unsafe {
        let hinst = GetModuleHandleW(None).ok()?;
        let class = w!("shotme-settings");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: hinst.into(),
            lpszClassName: class,
            hbrBackground: GetSysColorBrush(COLOR_BTNFACE),
            ..Default::default()
        };
        // Đăng ký lại lần hai báo lỗi vô hại.
        RegisterClassW(&wc);
        // Lớp của khung tab: nền tự tô trong `frame_proc`.
        let frame_wc = WNDCLASSW {
            lpfnWndProc: Some(frame_proc),
            hInstance: hinst.into(),
            lpszClassName: FRAME_CLASS,
            ..Default::default()
        };
        RegisterClassW(&frame_wc);
        // Đăng ký lớp dải tab và thanh trượt của Common Controls v6.
        let icc = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_TAB_CLASSES | ICC_BAR_CLASSES,
        };
        let _ = InitCommonControlsEx(&icc);
        let hwnd = CreateWindowExW(
            WS_EX_CONTROLPARENT | WS_EX_APPWINDOW,
            class,
            &title,
            WS_CAPTION | WS_SYSMENU,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            100,
            100,
            Some(parent),
            None,
            Some(hinst.into()),
            None,
        )
        .ok()?;
        crate::winutil::set_app_icons(hwnd);
        hwnd
    };
    // Cửa sổ mới chưa có lần kiểm tra nào: bỏ kết quả và cờ của cửa sổ trước (nếu còn sót).
    GENERATION.fetch_add(1, Ordering::SeqCst);
    TEST_RUNNING.with(|r| r.set(false));
    if let Ok(mut s) = TEST_RESULT.lock() {
        *s = None;
    }
    // SAFETY: hwnd vừa tạo.
    let dpi = unsafe { GetDpiForWindow(hwnd) }.max(96);
    let font = crate::winutil::system_font(dpi);
    let mut b = Builder {
        parent: hwnd,
        dpi: dpi as i32,
        font,
        layout: Layout::from_label_width(None),
    };
    let (client_h, controls) = build_controls(&mut b, current, notice);
    let Some(c) = controls else {
        // SAFETY: dọn cửa sổ vừa tạo (kéo theo mọi cửa sổ con) rồi phông.
        unsafe {
            let _ = DestroyWindow(hwnd);
            if let Some(f) = font {
                let _ = DeleteObject(HGDIOBJ(f.0));
            }
        }
        return None;
    };
    CTL.with(|x| x.set(Some(c)));
    OPEN.with(|o| o.set(hwnd.0 as isize));
    ST.with(|s| {
        *s.borrow_mut() = Some(St {
            current: current.clone(),
            parent,
            hotkeys: current.hotkeys,
            capturing: None,
            cap: CaptureState::new(),
            hk_unregistered: false,
            done: false,
            result: None,
            font,
            tab: Tab::General,
        })
    });
    let first_tab = initial_tab(reason);
    // SAFETY: chọn tab mở đầu trên dải tab của cửa sổ này (không sinh TCN_SELCHANGE).
    unsafe {
        SendMessageW(c.tabs, TCM_SETCURSEL, Some(WPARAM(first_tab.index())), None);
    }
    show_frame(&c, first_tab);

    // Điền giá trị ban đầu. Ô Secret luôn để trống.
    set_text(c.account, &current.r2.account_id);
    set_text(c.keyid, &current.r2.access_key_id);
    set_text(c.bucket, &current.r2.bucket);
    set_text(c.url, &current.r2.public_base_url);
    set_text(c.dir, &current.save_dir.to_string_lossy());
    for r in HotkeyRole::ALL {
        let on = current.hotkeys.get(r).enabled;
        set_checked(c.hk_chk[row(r)], on);
        apply_row_enabled(&c, r, on);
        refresh_hotkey_text(r);
    }
    set_checked(c.autostart, current.autostart);
    set_checked(c.notices, current.show_success_notices);
    set_checked(c.keep_selection, current.keep_selection);
    set_checked(c.include_cursor, current.include_cursor);
    set_checked(c.fmt_png, current.upload_format == UploadFormat::Png);
    set_checked(c.fmt_jpeg, current.upload_format == UploadFormat::Jpeg);
    let range = (usize::from(JPEG_QUALITY_MAX) << 16) | usize::from(JPEG_QUALITY_MIN);
    let pos = clamp_jpeg_quality(i64::from(current.jpeg_quality));
    // SAFETY: gửi thông điệp tới thanh trượt của cửa sổ này; tham số là số nguyên.
    unsafe {
        SendMessageW(
            c.quality,
            TBM_SETRANGE,
            Some(WPARAM(0)),
            Some(LPARAM(range as isize)),
        );
        SendMessageW(c.quality, TBM_SETLINESIZE, None, Some(LPARAM(1)));
        SendMessageW(c.quality, TBM_SETPAGESIZE, None, Some(LPARAM(10)));
        SendMessageW(
            c.quality,
            TBM_SETPOS,
            Some(WPARAM(1)),
            Some(LPARAM(isize::from(pos))),
        );
    }
    refresh_quality_text(&c);
    apply_quality_enabled(&c);
    let labels = Expiry::ALL.map(|e| expiry_label(lang::current(), e));
    // SAFETY: gửi thông điệp tới các điều khiển của cửa sổ này; HSTRING sống suốt lời gọi.
    unsafe {
        for l in labels {
            let hs = HSTRING::from(l);
            SendMessageW(
                c.expiry,
                CB_ADDSTRING,
                None,
                Some(LPARAM(hs.as_ptr() as isize)),
            );
        }
        let idx = Expiry::ALL
            .iter()
            .position(|e| *e == current.expiry)
            .unwrap_or(2);
        SendMessageW(c.expiry, CB_SETCURSEL, Some(WPARAM(idx)), None);
        // Mỗi ngôn ngữ luôn hiện bằng chính ngôn ngữ đó.
        for l in Lang::ALL {
            let hs = HSTRING::from(l.native_name());
            SendMessageW(
                c.language,
                CB_ADDSTRING,
                None,
                Some(LPARAM(hs.as_ptr() as isize)),
            );
        }
        let lang_idx = Lang::ALL
            .iter()
            .position(|l| *l == current.language)
            .unwrap_or(0);
        SendMessageW(c.language, CB_SETCURSEL, Some(WPARAM(lang_idx)), None);
    }

    // Đổi kích thước theo DPI, căn giữa màn hình đang có con trỏ.
    // SAFETY: các lời gọi hệ thống chỉ dùng giá trị cục bộ.
    unsafe {
        let mut rc = RECT {
            left: 0,
            top: 0,
            right: b.s(b.layout.client_w),
            bottom: b.s(client_h),
        };
        let _ = AdjustWindowRectExForDpi(
            &mut rc,
            WS_CAPTION | WS_SYSMENU,
            false,
            WS_EX_CONTROLPARENT | WS_EX_APPWINDOW,
            dpi,
        );
        crate::winutil::center_on_cursor_monitor(hwnd, rc.right - rc.left, rc.bottom - rc.top);
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
        let _ = SetFocus(Some(first_ctl(&c, first_tab)));
    }

    run_loop(hwnd, c);

    // Dọn dẹp: cửa sổ, phông, trạng thái.
    let result = with_st(|s| s.result.take()).flatten();
    // SAFETY: hủy cửa sổ và phông do hàm này tạo.
    unsafe {
        let _ = DestroyWindow(hwnd);
    }
    if let Some(Some(f)) = ST.with(|s| s.borrow_mut().take().map(|st| st.font)) {
        // SAFETY: phông do CreateFontIndirectW tạo, cửa sổ dùng nó đã bị hủy.
        unsafe {
            let _ = DeleteObject(HGDIOBJ(f.0));
        }
    }
    CTL.with(|x| x.set(None));
    OPEN.with(|o| o.set(0));
    // Thế hệ mới: kết quả muộn của luồng kiểm tra đang chạy sẽ bị bỏ. Hẹn giờ dự phòng đã chết
    // theo cửa sổ, nên cờ phải hạ ở đây, không thì nút Kiểm tra kết nối của lần mở sau bị khóa.
    GENERATION.fetch_add(1, Ordering::SeqCst);
    TEST_RUNNING.with(|r| r.set(false));
    result.map(|config| SettingsResult { config })
}

fn run_loop(hwnd: HWND, c: Ctl) {
    let mut msg = MSG::default();
    loop {
        if with_st(|s| s.done).unwrap_or(true) {
            break;
        }
        // SAFETY: vòng lặp thông điệp chuẩn của luồng này.
        let r = unsafe { GetMessageW(&mut msg, None, 0, 0) };
        if r.0 == 0 {
            // WM_QUIT: trả lại cho vòng lặp ngoài rồi thoát.
            // SAFETY: gửi lại yêu cầu thoát.
            unsafe { PostQuitMessage(msg.wParam.0 as i32) };
            cancel();
            break;
        }
        if r.0 < 0 {
            cancel();
            break;
        }
        // Thông tin và Trợ giúp dùng được cả khi Tùy chọn đang mở: chúng xử lý phím của chính chúng trước.
        if crate::about::pre_translate(&msg) || crate::help::pre_translate(&msg) {
            continue;
        }
        // SAFETY: IsChild chỉ đọc quan hệ cửa sổ.
        let mine = msg.hwnd == hwnd || unsafe { IsChild(hwnd, msg.hwnd).as_bool() };
        if mine {
            let capturing_row = with_st(|s| s.capturing).flatten();
            let capturing = capturing_row.is_some();
            if let Some(r) = capturing_row {
                if capture_msg(&msg, c.hk_btn[row(r)]) {
                    continue;
                }
            }
            if msg.message == WM_KEYDOWN {
                let vk = msg.wParam.0 as u32;
                // SAFETY: chỉ đọc trạng thái hộp thả.
                let dropped = [c.expiry, c.language]
                    .iter()
                    .any(|h| unsafe { SendMessageW(*h, CB_GETDROPPEDSTATE, None, None).0 != 0 });
                let route = key_route(
                    capturing,
                    vk,
                    key_down(VK_CONTROL_U),
                    key_down(VK_SHIFT.0 as i32),
                    dropped,
                );
                match route {
                    KeyRoute::NextTab => {
                        cycle_tab(1);
                        continue;
                    }
                    KeyRoute::PrevTab => {
                        cycle_tab(Tab::ALL.len() - 1);
                        continue;
                    }
                    KeyRoute::Cancel => {
                        cancel();
                        continue;
                    }
                    // Đang bắt phím thì `capture_msg` ở trên đã nhận phím; còn lại cho hộp thoại.
                    KeyRoute::Capture | KeyRoute::Dialog => {}
                    KeyRoute::Accept => {
                        // SAFETY: chỉ đọc điều khiển đang giữ focus.
                        let focus = unsafe { GetFocus() };
                        let is_push = unsafe {
                            let st = GetWindowLongPtrW(focus, GWL_STYLE) as u32;
                            let kind = st & 0xF;
                            focus != HWND::default()
                                && (kind == BS_PUSHBUTTON as u32 || kind == BS_DEFPUSHBUTTON as u32)
                                && focus != c.autostart
                                && get_class_is_button(focus)
                        };
                        if is_push {
                            // SAFETY: bấm nút đang giữ focus.
                            unsafe {
                                SendMessageW(focus, BM_CLICK, None, None);
                            }
                        } else {
                            save();
                        }
                        continue;
                    }
                }
            }
            // SAFETY: xử lý phím điều hướng (Tab, mũi tên) của hộp thoại.
            if unsafe { IsDialogMessageW(hwnd, &msg).as_bool() } {
                continue;
            }
        }
        // SAFETY: chuyển thông điệp chuẩn.
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

fn get_class_is_button(h: HWND) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::GetClassNameW;
    let mut buf = [0u16; 16];
    // SAFETY: bộ đệm cục bộ đủ chỗ.
    let n = unsafe { GetClassNameW(h, &mut buf) }.max(0) as usize;
    String::from_utf16_lossy(&buf[..n.min(buf.len())]).eq_ignore_ascii_case("button")
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkshot_core::config::HotkeySlot;
    use std::path::Path;

    fn base() -> Config {
        let mut c = Config::defaults(Path::new("C:/pics"), Lang::Vi);
        c.r2 = R2Config {
            account_id: "acc".into(),
            access_key_id: "kid".into(),
            secret_access_key: "OLD".into(),
            bucket: "b".into(),
            public_base_url: "https://img.example.com".into(),
        };
        c
    }

    fn form_of(c: &Config) -> Form {
        Form {
            account_id: c.r2.account_id.clone(),
            access_key_id: c.r2.access_key_id.clone(),
            secret_access_key: String::new(),
            bucket: c.r2.bucket.clone(),
            public_base_url: c.r2.public_base_url.clone(),
            expiry: c.expiry,
            hotkeys: c.hotkeys,
            save_dir: c.save_dir.to_string_lossy().into_owned(),
            autostart: c.autostart,
            language: c.language,
            show_success_notices: c.show_success_notices,
            keep_selection: c.keep_selection,
            include_cursor: c.include_cursor,
            upload_format: c.upload_format,
            jpeg_quality: c.jpeg_quality,
        }
    }

    #[test]
    fn empty_secret_keeps_existing() {
        let c = base();
        let mut f = form_of(&c);
        assert_eq!(
            apply_form(&c, &f, Lang::Vi).unwrap().r2.secret_access_key,
            "OLD"
        );
        f.secret_access_key = "NEW".into();
        assert_eq!(
            apply_form(&c, &f, Lang::Vi).unwrap().r2.secret_access_key,
            "NEW"
        );
    }

    #[test]
    fn url_is_normalized_and_bad_url_rejected() {
        let c = base();
        let mut f = form_of(&c);
        f.public_base_url = "https://img.example.com/".into();
        assert_eq!(
            apply_form(&c, &f, Lang::Vi).unwrap().r2.public_base_url,
            "https://img.example.com"
        );
        f.public_base_url = "http://x".into();
        assert!(apply_form(&c, &f, Lang::Vi).is_err());
    }

    #[test]
    fn empty_url_allowed_when_r2_not_filled() {
        let c = Config::defaults(Path::new("C:/pics"), Lang::Vi);
        let mut f = form_of(&c);
        f.save_dir = "C:/pics".into();
        let out = apply_form(&c, &f, Lang::Vi).unwrap();
        assert!(!out.r2.is_complete());
        assert_eq!(out.r2.public_base_url, "");
    }

    #[test]
    fn fields_are_trimmed() {
        let c = base();
        let mut f = form_of(&c);
        f.bucket = " bkt ".into();
        f.account_id = " acc2 ".into();
        f.access_key_id = "\tk2 ".into();
        f.save_dir = "  C:/x  ".into();
        f.secret_access_key = " S ".into();
        let out = apply_form(&c, &f, Lang::Vi).unwrap();
        assert_eq!(out.r2.bucket, "bkt");
        assert_eq!(out.r2.account_id, "acc2");
        assert_eq!(out.r2.access_key_id, "k2");
        assert_eq!(out.r2.secret_access_key, "S");
        assert_eq!(out.save_dir, PathBuf::from("C:/x"));
    }

    #[test]
    fn empty_save_dir_rejected() {
        let c = base();
        let mut f = form_of(&c);
        f.save_dir = "   ".into();
        assert!(apply_form(&c, &f, Lang::Vi).is_err());
    }

    #[test]
    fn other_fields_are_carried() {
        let c = base();
        let mut f = form_of(&c);
        f.expiry = Expiry::Keep;
        f.autostart = true;
        f.hotkeys.region.key = Hotkey {
            ctrl: true,
            alt: false,
            shift: false,
            win: false,
            vk: 0x53,
        };
        let out = apply_form(&c, &f, Lang::Vi).unwrap();
        assert_eq!(out.expiry, Expiry::Keep);
        assert!(out.autostart);
        assert_eq!(out.hotkeys.region.key, f.hotkeys.region.key);
    }

    #[test]
    fn bad_account_or_bucket_chars_rejected() {
        let c = base();
        let mut f = form_of(&c);
        f.account_id = "a/b".into();
        assert!(apply_form(&c, &f, Lang::Vi).is_err());
        let mut f = form_of(&c);
        f.bucket = "my bucket".into();
        assert!(apply_form(&c, &f, Lang::Vi).is_err());
    }

    #[test]
    fn errors_and_debug_never_contain_secret() {
        let c = base();
        let mut f = form_of(&c);
        f.secret_access_key = "SUPER-SECRET".into();
        f.public_base_url = "http://x".into();
        for lang in Lang::ALL {
            let e = apply_form(&c, &f, lang).unwrap_err();
            assert!(!e.message.contains("SUPER-SECRET"));
            assert!(!format!("{e:?}").contains("SUPER-SECRET"));
        }
        assert!(!format!("{f:?}").contains("SUPER-SECRET"));
    }

    #[test]
    fn field_to_tab() {
        assert_eq!(tab_of(Field::SaveDir), Tab::General);
        for f in [Field::AccountId, Field::Bucket, Field::PublicUrl] {
            assert_eq!(tab_of(f), Tab::R2);
        }
    }

    #[test]
    fn layout_keeps_hotkey_edit_wide_enough() {
        for label_w in [None, Some(0), Some(100), Some(168), Some(250), Some(400)] {
            let l = Layout::from_label_width(label_w);
            let edit_x = LX + l.hk_chk_w + HK_GAP;
            let edit_w = EX + l.ew - HK_BTN_W - HK_GAP - edit_x;
            assert!(edit_w >= HK_EDIT_MIN_W, "{label_w:?}: {edit_w}");
            assert!(l.client_w >= CLIENT_MIN_W, "{label_w:?}");
            assert_eq!(l.client_w, EX + l.ew + RIGHT_MARGIN);
        }
        // Nhãn ngắn: giữ bố cục cũ 480 DIP, ô nhập 270 DIP.
        let l = Layout::from_label_width(Some(50));
        assert_eq!((l.client_w, l.ew), (CLIENT_MIN_W, 270));
    }

    #[test]
    fn hotkey_rows_belong_to_hotkeys_tab() {
        for r in HotkeyRole::ALL {
            assert_eq!(tab_of(Field::HotkeyRow(r)), Tab::Hotkeys);
        }
    }

    #[test]
    fn hotkey_conflict_is_a_form_error() {
        let c = base();
        let mut f = form_of(&c);
        f.hotkeys.save_full = HotkeySlot {
            enabled: true,
            key: f.hotkeys.region.key,
        };
        let e = apply_form(&c, &f, Lang::Vi).unwrap_err();
        assert_eq!(e.field, Field::HotkeyRow(HotkeyRole::SaveFull));
        assert!(e.message.contains("PrintScreen"));
        assert!(e.message.contains(t(Lang::Vi, HotkeyRole::Region.label())));
        assert!(e
            .message
            .contains(t(Lang::Vi, HotkeyRole::SaveFull.label())));
        f.hotkeys.save_full.enabled = false; // dòng tắt không tính
        assert!(apply_form(&c, &f, Lang::Vi).is_ok());
    }

    #[test]
    fn error_order_is_savedir_hotkeys_r2() {
        let c = base();
        let mut f = form_of(&c);
        f.hotkeys.save_full = HotkeySlot {
            enabled: true,
            key: f.hotkeys.region.key,
        };
        f.bucket = "x y".into();
        assert_eq!(
            apply_form(&c, &f, Lang::Vi).unwrap_err().field,
            Field::HotkeyRow(HotkeyRole::SaveFull)
        );
        f.save_dir = "".into();
        assert_eq!(
            apply_form(&c, &f, Lang::Vi).unwrap_err().field,
            Field::SaveDir
        );
    }

    #[test]
    fn new_options_are_carried_from_form() {
        let c = base();
        let mut f = form_of(&c);
        f.show_success_notices = false;
        f.keep_selection = true;
        f.include_cursor = true;
        f.hotkeys.upload_full.enabled = true;
        f.hotkeys.region.enabled = false;
        let n = apply_form(&c, &f, Lang::Vi).unwrap();
        assert!(!n.show_success_notices && n.keep_selection && n.include_cursor);
        assert_eq!(n.hotkeys, f.hotkeys);
    }

    #[test]
    fn tab_order_has_four_tabs() {
        let want = [Tab::General, Tab::Hotkeys, Tab::Format, Tab::R2];
        assert_eq!(Tab::ALL.as_slice(), want.as_slice());
        for (i, t) in want.iter().enumerate() {
            assert_eq!(t.index(), i, "{t:?}");
        }
        assert_eq!(Tab::R2.index(), 3);
        // Đi tới: qua đủ bốn tab rồi về đầu.
        let mut t = Tab::General;
        let mut seen = Vec::new();
        for _ in 0..4 {
            seen.push(t);
            t = t.cycle(1);
        }
        assert_eq!(seen, want.to_vec());
        assert_eq!(t, Tab::General);
        // Đi lùi: General, R2, Format, Hotkeys rồi về General.
        let back = Tab::ALL.len() - 1;
        let mut t = Tab::General;
        let mut seen = Vec::new();
        for _ in 0..4 {
            seen.push(t);
            t = t.cycle(back);
        }
        assert_eq!(seen, vec![Tab::General, Tab::R2, Tab::Format, Tab::Hotkeys]);
        assert_eq!(t, Tab::General);
    }

    #[test]
    fn form_reads_and_writes_format() {
        let current = base();
        let mut src = base();
        src.upload_format = UploadFormat::Jpeg;
        src.jpeg_quality = 75;
        let f = form_of(&src);
        assert_eq!(f.upload_format, UploadFormat::Jpeg);
        assert_eq!(f.jpeg_quality, 75);
        let out = apply_form(&current, &f, Lang::Vi).unwrap();
        assert_eq!(out.upload_format, UploadFormat::Jpeg);
        assert_eq!(out.jpeg_quality, 75);
    }

    #[test]
    fn quality_survives_png_selection() {
        let mut current = base();
        current.upload_format = UploadFormat::Jpeg;
        current.jpeg_quality = 90;
        let mut f = form_of(&current);
        f.upload_format = UploadFormat::Png;
        f.jpeg_quality = 70;
        let out = apply_form(&current, &f, Lang::Vi).unwrap();
        assert_eq!(out.upload_format, UploadFormat::Png);
        assert_eq!(out.jpeg_quality, 70);
    }

    #[test]
    fn quality_enabled_only_for_jpeg() {
        assert!(quality_enabled(UploadFormat::Jpeg));
        assert!(!quality_enabled(UploadFormat::Png));
    }

    #[test]
    fn tabs_fit_compares_total_width() {
        assert!(tabs_fit(&[100, 100, 100, 100], 401));
        assert!(tabs_fit(&[100, 100, 100, 100], 400));
        assert!(!tabs_fit(&[100, 100, 100, 101], 400));
        assert!(!tabs_fit(&[300, 200], 499));
        assert!(tabs_fit(&[], 0));
    }

    #[test]
    fn layout_widens_for_tab_strip() {
        let base = Layout::from_label_width(Some(50));
        // Dải tab ngắn: giữ nguyên bố cục.
        assert_eq!(base.widen_for_tabs(&[60, 60, 60, 60]), base);
        // Dải tab dài: client_w và ô nhập nới cùng một lượng, dải tab vừa khít.
        let wide = [200, 200, 200, 200];
        let l = base.widen_for_tabs(&wide);
        assert!(l.client_w > base.client_w);
        assert_eq!(l.client_w - base.client_w, l.ew - base.ew);
        assert_eq!(l.hk_chk_w, base.hk_chk_w);
        assert_eq!(l.client_w, EX + l.ew + RIGHT_MARGIN);
        assert!(tabs_fit(&wide, l.tab_strip_w()));
        let mut one_less = l;
        one_less.client_w -= 1;
        assert!(!tabs_fit(&wide, one_less.tab_strip_w()));
    }

    #[test]
    fn initial_tab_by_reason() {
        assert_eq!(initial_tab(OpenReason::User), Tab::General);
        assert_eq!(initial_tab(OpenReason::NeedsR2), Tab::R2);
    }

    #[test]
    fn form_errors_name_their_field() {
        let c = base();
        let mut f = form_of(&c);
        f.save_dir = "  ".into();
        assert_eq!(
            apply_form(&c, &f, Lang::Vi).unwrap_err().field,
            Field::SaveDir
        );
        let mut f = form_of(&c);
        f.account_id = "a/b".into();
        assert_eq!(
            apply_form(&c, &f, Lang::Vi).unwrap_err().field,
            Field::AccountId
        );
        let mut f = form_of(&c);
        f.bucket = "my bucket".into();
        assert_eq!(
            apply_form(&c, &f, Lang::Vi).unwrap_err().field,
            Field::Bucket
        );
        let mut f = form_of(&c);
        f.public_base_url = "http://x".into();
        assert_eq!(
            apply_form(&c, &f, Lang::Vi).unwrap_err().field,
            Field::PublicUrl
        );
    }

    #[test]
    fn first_error_wins_in_tab_order() {
        // Sai cả SaveDir lẫn Bucket: báo SaveDir.
        let c = base();
        let mut f = form_of(&c);
        f.save_dir = "".into();
        f.bucket = "x y".into();
        assert_eq!(
            apply_form(&c, &f, Lang::Vi).unwrap_err().field,
            Field::SaveDir
        );
    }

    #[test]
    fn form_error_message_follows_language() {
        let c = base();
        let mut f = form_of(&c);
        f.bucket = "x y".into();
        assert_ne!(
            apply_form(&c, &f, Lang::Vi).unwrap_err().message,
            apply_form(&c, &f, Lang::En).unwrap_err().message
        );
    }

    #[test]
    fn language_is_carried_from_form() {
        let c = base();
        let mut f = form_of(&c);
        f.language = Lang::En;
        assert_eq!(apply_form(&c, &f, Lang::Vi).unwrap().language, Lang::En);
    }

    #[test]
    fn key_routing() {
        const TAB: u32 = 0x09;
        const ESC: u32 = 0x1B;
        const ENTER: u32 = 0x0D;
        // Đang bắt phím: mọi thứ là của chế độ bắt, kể cả Ctrl+Tab, Esc, Enter.
        for (vk, ctrl) in [(TAB, true), (ESC, false), (ENTER, false), (0x41, true)] {
            assert_eq!(key_route(true, vk, ctrl, false, false), KeyRoute::Capture);
        }
        assert_eq!(key_route(false, TAB, true, false, false), KeyRoute::NextTab);
        assert_eq!(key_route(false, TAB, true, true, false), KeyRoute::PrevTab);
        assert_eq!(key_route(false, TAB, false, false, false), KeyRoute::Dialog);
        assert_eq!(key_route(false, ESC, false, false, false), KeyRoute::Cancel);
        assert_eq!(
            key_route(false, ENTER, false, false, false),
            KeyRoute::Accept
        );
        assert_eq!(key_route(false, ESC, false, false, true), KeyRoute::Dialog);
        assert_eq!(
            key_route(false, ENTER, false, false, true),
            KeyRoute::Dialog
        );
        assert_eq!(key_route(false, TAB, true, false, true), KeyRoute::NextTab);
    }

    #[test]
    fn capture_hotkey_rules() {
        // PrintScreen đứng một mình được phép; phím chữ phải kèm phím bổ trợ.
        assert_eq!(
            capture_hotkey(0x2C, false, false, false, false),
            Capture::Got(Hotkey::default())
        );
        assert_eq!(
            capture_hotkey(0x53, false, false, false, false),
            Capture::NeedModifier
        );
        assert_eq!(
            capture_hotkey(0x53, true, false, true, false),
            Capture::Got(Hotkey {
                ctrl: true,
                alt: false,
                shift: true,
                win: false,
                vk: 0x53
            })
        );
        // Chỉ phím bổ trợ: bỏ qua.
        assert_eq!(
            capture_hotkey(0x11, true, false, false, false),
            Capture::Ignore
        );
        assert_eq!(
            capture_hotkey(0x5B, false, false, false, true),
            Capture::Ignore
        );
        // Shift + chữ không đủ.
        assert_eq!(
            capture_hotkey(0x41, false, false, true, false),
            Capture::NeedModifier
        );
        // F-key đứng một mình được phép.
        assert!(matches!(
            capture_hotkey(0x71, false, false, false, false),
            Capture::Got(_)
        ));
    }

    fn dn(vk: u32, ctrl: bool, alt: bool, shift: bool, win: bool) -> KeyEv {
        KeyEv::Down {
            vk,
            ctrl,
            alt,
            shift,
            win,
        }
    }
    fn up(vk: u32, ctrl: bool, alt: bool, shift: bool, win: bool) -> KeyEv {
        KeyEv::Up {
            vk,
            ctrl,
            alt,
            shift,
            win,
        }
    }
    fn hk(ctrl: bool, alt: bool, shift: bool, win: bool, vk: u32) -> Capture {
        Capture::Got(Hotkey {
            ctrl,
            alt,
            shift,
            win,
            vk,
        })
    }

    #[test]
    fn ctrl_released_before_main_key_still_gives_full_combo() {
        let mut st = CaptureState::new();
        // Nhấn Ctrl, Shift (bổ trợ: bỏ qua), rồi S với cả hai đang giữ.
        assert_eq!(
            st.feed(dn(0x11, true, false, false, false)),
            Capture::Ignore
        );
        assert_eq!(st.feed(dn(0x10, true, false, true, false)), Capture::Ignore);
        assert_eq!(
            st.feed(dn(0x53, true, false, true, false)),
            hk(true, false, true, false, 0x53)
        );
        // Ctrl nhả trước S: KEYUP không đổi kết quả.
        assert_eq!(
            st.feed(up(0x11, false, false, true, false)),
            Capture::Ignore
        );
        assert_eq!(
            st.feed(up(0x53, false, false, true, false)),
            Capture::Ignore
        );
    }

    #[test]
    fn orphan_keyup_is_ignored() {
        let mut st = CaptureState::new();
        assert_eq!(
            st.feed(up(0x0D, false, false, false, false)),
            Capture::Ignore
        );
        assert_eq!(
            st.feed(up(0x20, false, false, false, false)),
            Capture::Ignore
        );
    }

    #[test]
    fn printscreen_keyup_only_is_accepted() {
        let mut st = CaptureState::new();
        assert_eq!(
            st.feed(up(0x2C, false, false, false, false)),
            Capture::Got(Hotkey::default())
        );
    }

    #[test]
    fn shift_plus_letter_rejected_but_other_modifiers_accepted() {
        let mut st = CaptureState::new();
        assert_eq!(
            st.feed(dn(0x41, false, false, true, false)),
            Capture::NeedModifier
        );
        assert_eq!(
            st.feed(dn(0x41, true, false, false, false)),
            hk(true, false, false, false, 0x41)
        );
        assert_eq!(
            st.feed(dn(0x41, false, true, false, false)),
            hk(false, true, false, false, 0x41)
        );
        assert_eq!(
            st.feed(dn(0x41, false, false, false, true)),
            hk(false, false, false, true, 0x41)
        );
    }

    #[test]
    fn function_keys_alone_or_with_shift_accepted() {
        let mut st = CaptureState::new();
        assert_eq!(
            st.feed(dn(0x74, false, false, true, false)),
            hk(false, false, true, false, 0x74)
        );
        assert_eq!(
            st.feed(dn(0x74, false, false, false, false)),
            hk(false, false, false, false, 0x74)
        );
    }

    #[test]
    fn alt_f4_is_never_a_combo() {
        let mut st = CaptureState::new();
        assert_eq!(
            st.feed(dn(0x73, false, true, false, false)),
            Capture::PassThrough
        );
        assert_eq!(
            capture_hotkey(0x73, true, true, false, false),
            Capture::PassThrough
        );
    }

    /// Mở cửa sổ thật, đóng bằng WM_CLOSE rồi bằng nút Hủy. Không ghi đĩa, không gọi mạng.
    /// Chạy tay: `cargo test -p darkshot-windows -- --ignored settings_window`.
    #[test]
    #[ignore]
    fn settings_window_opens_and_cancels() {
        use windows::Win32::UI::WindowsAndMessaging::{
            DefWindowProcW as Def, FindWindowW, IsWindow, WM_COMMAND,
        };
        unsafe extern "system" fn proc_(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
            // SAFETY: chuyển nguyên tham số.
            unsafe { Def(h, m, w, l) }
        }
        // Cửa sổ cha ẩn riêng của kiểm thử (phím tắt toàn cục gắn với nó, không đụng ứng dụng thật).
        let parent = unsafe {
            let hinst = GetModuleHandleW(None).unwrap();
            let class = w!("shotme-settings-test-parent");
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
        let cfg = Config::defaults(std::path::Path::new("C:/shotme-test-not-created"), Lang::Vi);
        for (use_command, reason) in [
            (false, OpenReason::User),
            (true, OpenReason::NeedsR2),
            (true, OpenReason::User),
            (false, OpenReason::NeedsR2),
        ] {
            let closer = std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(500));
                // SAFETY: tìm cửa sổ theo tên lớp, hỏi tab đang chọn rồi đăng thông điệp; HWND chỉ dùng như số.
                let sel = unsafe {
                    let h = FindWindowW(w!("shotme-settings"), None)
                        .expect("settings window not found");
                    let tab = windows::Win32::UI::WindowsAndMessaging::FindWindowExW(
                        Some(h),
                        None,
                        w!("SysTabControl32"),
                        None,
                    )
                    .expect("tab control not found");
                    let sel =
                        SendMessageW(tab, windows::Win32::UI::Controls::TCM_GETCURSEL, None, None)
                            .0;
                    // Mép phải của tab cuối và bề rộng dải tab: tab cuối phải nằm trọn trong dải.
                    let mut last = RECT::default();
                    SendMessageW(
                        tab,
                        windows::Win32::UI::Controls::TCM_GETITEMRECT,
                        Some(WPARAM(Tab::ALL.len() - 1)),
                        Some(LPARAM(&mut last as *mut RECT as isize)),
                    );
                    let mut client = RECT::default();
                    let _ = GetClientRect(tab, &mut client);
                    let rows = (last.right, client.right);
                    if use_command {
                        PostMessageW(Some(h), WM_COMMAND, WPARAM(ID_CANCEL as usize), LPARAM(0))
                            .unwrap();
                    } else {
                        PostMessageW(Some(h), WM_CLOSE, WPARAM(0), LPARAM(0)).unwrap();
                    }
                    (sel, rows)
                };
                let (sel, rows) = sel;
                let want = match initial_tab(reason) {
                    Tab::General => 0,
                    Tab::Hotkeys => 1,
                    Tab::Format => 2,
                    Tab::R2 => 3,
                };
                assert_eq!(sel, want, "{reason:?}");
                // Bốn tab nằm trọn trên một hàng (không cần nút cuộn của dải tab).
                let (last_right, strip_right) = rows;
                assert!(
                    last_right > 0 && last_right <= strip_right,
                    "{reason:?}: {last_right} > {strip_right}"
                );
            });
            // Giả lập lần Kiểm tra kết nối trước bị bỏ dở (Hủy ngay sau khi bấm): cờ kẹt ở true.
            TEST_RUNNING.with(|r| r.set(true));
            assert!(show(parent, &cfg, Some("Thông báo thử"), reason).is_none());
            assert!(
                !TEST_RUNNING.with(|r| r.get()),
                "TEST_RUNNING must be cleared when the window closes"
            );
            closer.join().unwrap();
            // SAFETY: tìm lại cửa sổ sau khi show trả về.
            assert!(unsafe { FindWindowW(w!("shotme-settings"), None) }.is_err());
        }
        // SAFETY: dọn cửa sổ cha.
        unsafe {
            assert!(IsWindow(Some(parent)).as_bool());
            let _ = DestroyWindow(parent);
        }
    }
}
