//! Cửa sổ "Các bước" (Xem lại) của phiên chụp từng bước: tối, đổi cỡ được, một cửa sổ duy nhất.
//! Trái: danh sách bước (ảnh nhỏ, "Bước k", dòng đầu ghi chú). Phải: xem trước bước đang chọn và ô
//! Ghi chú nhiều dòng. Hàng nút: Lên, Xuống, Xóa bước | Xuất…, Kết thúc phiên, Đóng. Kết thúc khi
//! chưa xuất: thẻ xác nhận trong cửa sổ (phủ mờ, Hủy/Kết thúc, Esc = Hủy).
//!
//! Dữ liệu phiên thuộc `steps_store` (luồng giao diện); cửa sổ giữ bản chụp danh sách, làm mới khi
//! phiên đổi (`on_store_changed`). Ghi chú được lưu khi ô mất tiêu điểm, đổi bước, lên/xuống/xóa,
//! xuất, kết thúc và khi cửa sổ bị hủy (mọi đường đóng, kể cả thoát ứng dụng).
//!
//! Luồng nền: ảnh nhỏ (hàng đợi) và xem trước (chỉ yêu cầu mới nhất) giải mã PNG ngoài luồng giao
//! diện, đẩy điểm ảnh vào hàng kết quả chung rồi `PostMessageW(WM_STEPS_WORK)` (không có con trỏ
//! trong thông điệp nên cửa sổ đã hủy cũng không rò). Xuất chạy trên luồng `darkshot-steps-export`,
//! báo về cửa sổ chính bằng `WM_STEPS_EXPORTED` (kết quả trong `EXPORT_DONE`), nên đóng cửa sổ giữa
//! chừng vẫn có thông báo.
//!
//! Quy tắc tái nhập: trạng thái `STATE` chỉ được mượn trong khối ngắn, không gọi hàm của
//! `steps_store` (có thể gọi lại `on_store_changed`) hay hàm Win32 gửi thông điệp cho cửa sổ này.

use crate::history_layout::{
    compose_over, explorer_exe, explorer_select_args, fit_rect, preview_target, step_selection,
    Confirm, ConfirmBtn, ConfirmKey, ConfirmOutcome, Nav, CARD_RADIUS,
};
use crate::history_paint::{self as hp, Bmp, Fonts};
use crate::lang;
use crate::paths;
use crate::steps_export::{self, ExportJob};
use crate::steps_store;
use crate::theme::{self, ButtonKind};
use crate::winutil::{self, scale};
use darkshot_core::actions::NoticeKind;
use darkshot_core::geometry::Rect;
use darkshot_core::i18n::{self, step_label, t, Lang, Msg};
use darkshot_core::steps::{Step, MAX_CAPTION};
use darkshot_core::theme::{
    ACCENT, BG, BUTTON, INPUT, LINE, SURFACE, TEXT, TEXT_DISABLED, TEXT_MUTED,
};
use darkshot_core::thumb::{decode_png_file, downscale};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use windows::core::{w, HSTRING, PCWSTR, PWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateRoundRectRgn, DeleteObject, EndPaint, GetMonitorInfoW, InvalidateRect,
    MonitorFromPoint, RedrawWindow, SetWindowRgn, DRAW_TEXT_FORMAT, DT_CENTER, DT_END_ELLIPSIS,
    DT_LEFT, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, DT_WORDBREAK, HBRUSH, HDC, HFONT, HGDIOBJ,
    HMONITOR, MONITORINFO, MONITOR_DEFAULTTONEAREST, PAINTSTRUCT, RDW_ALLCHILDREN, RDW_INVALIDATE,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, IBindCtx,
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{SetScrollInfo, EM_LIMITTEXT, EM_SETSEL, NMHDR, NM_CUSTOMDRAW};
use windows::Win32::UI::HiDpi::{
    AdjustWindowRectExForDpi, GetDpiForMonitor, GetDpiForWindow, MDT_EFFECTIVE_DPI,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    EnableWindow, GetFocus, GetKeyState, IsWindowEnabled, SetFocus, VK_CONTROL, VK_DELETE, VK_DOWN,
    VK_END, VK_ESCAPE, VK_HOME, VK_NEXT, VK_PRIOR, VK_RETURN, VK_SHIFT, VK_SPACE, VK_TAB, VK_UP,
};
use windows::Win32::UI::Shell::{
    FileOpenDialog, IFileOpenDialog, IShellItem, SHCreateItemFromParsingName, ShellExecuteW,
    FOS_FORCEFILESYSTEM, FOS_PICKFOLDERS, SIGDN_FILESYSPATH,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetCursorPos, GetNextDlgTabItem,
    GetParent, GetScrollInfo, IsChild, IsDialogMessageW, IsWindow, IsWindowVisible, LoadCursorW,
    MoveWindow, PostMessageW, RegisterClassW, SendMessageW, SetWindowPos, SetWindowTextW,
    ShowWindow, BS_PUSHBUTTON, CS_DBLCLKS, DLGC_WANTARROWS, EN_KILLFOCUS, EN_SETFOCUS,
    ES_AUTOVSCROLL, ES_MULTILINE, ES_WANTRETURN, HMENU, HWND_TOP, IDC_ARROW, MINMAXINFO, MSG,
    SB_BOTTOM, SB_LINEDOWN, SB_LINEUP, SB_PAGEDOWN, SB_PAGEUP, SB_THUMBPOSITION, SB_THUMBTRACK,
    SB_TOP, SB_VERT, SCROLLBAR_COMMAND, SCROLLINFO, SIF_PAGE, SIF_POS, SIF_RANGE, SIF_TRACKPOS,
    SWP_NOACTIVATE, SWP_NOZORDER, SWP_SHOWWINDOW, SW_HIDE, SW_SHOW, SW_SHOWNORMAL, WA_INACTIVE,
    WINDOW_EX_STYLE, WINDOW_STYLE, WM_ACTIVATE, WM_APP, WM_CLOSE, WM_COMMAND, WM_CTLCOLOREDIT,
    WM_CTLCOLORSTATIC, WM_DESTROY, WM_DPICHANGED, WM_ERASEBKGND, WM_GETDLGCODE, WM_GETMINMAXINFO,
    WM_KEYDOWN, WM_KILLFOCUS, WM_LBUTTONDOWN, WM_MOUSEWHEEL, WM_NCDESTROY, WM_NOTIFY, WM_PAINT,
    WM_SETFOCUS, WM_SETFONT, WM_SIZE, WM_VSCROLL, WNDCLASSW, WS_CAPTION, WS_CHILD, WS_CLIPCHILDREN,
    WS_CLIPSIBLINGS, WS_EX_APPWINDOW, WS_EX_CONTROLPARENT, WS_GROUP, WS_MAXIMIZEBOX,
    WS_MINIMIZEBOX, WS_SYSMENU, WS_TABSTOP, WS_THICKFRAME, WS_VSCROLL,
};

/// Kết quả nạp ảnh nhỏ/xem trước của luồng nền đã sẵn (hàng đợi trong trạng thái cửa sổ).
pub const WM_STEPS_WORK: u32 = WM_APP + 15;
/// Gửi tới cửa sổ chính khi một lượt xuất xong (kết quả trong hàng đợi của tiến trình).
pub const WM_STEPS_EXPORTED: u32 = WM_APP + 16;

/// Vùng client mặc định và tối thiểu (DIP).
const DEFAULT_W: i32 = 860;
const DEFAULT_H: i32 = 600;
const MIN_W: i32 = 640;
const MIN_H: i32 = 440;

const CLASS: PCWSTR = w!("darkshot-steps");
const LIST_CLASS: PCWSTR = w!("darkshot-steps-list");
const CARD_CLASS: PCWSTR = w!("darkshot-steps-card");

const STYLE: WINDOW_STYLE = WINDOW_STYLE(
    WS_CAPTION.0
        | WS_SYSMENU.0
        | WS_THICKFRAME.0
        | WS_MINIMIZEBOX.0
        | WS_MAXIMIZEBOX.0
        | WS_CLIPCHILDREN.0,
);
const EX_STYLE: WINDOW_EX_STYLE = WINDOW_EX_STYLE(WS_EX_APPWINDOW.0 | WS_EX_CONTROLPARENT.0);

/// Mã điều khiển. Không dùng 1 và 2 (IDOK/IDCANCEL do `IsDialogMessageW` tự gửi).
const ID_BUTTONS: [i32; 6] = [301, 302, 303, 304, 305, 306];
const ID_LIST: i32 = 310;
const ID_EDIT: i32 = 311;
const ID_CARD: i32 = 312;
const ID_CARD_CANCEL: i32 = 313;
const ID_CARD_YES: i32 = 314;

// Bố cục (DIP).
const MARGIN: i32 = 16;
const GAP: i32 = 12;
const BTN_H: i32 = 32;
const BTN_GAP: i32 = 8;
const BTN_PAD: i32 = 16;
const BTN_MIN: i32 = 72;
/// Bề rộng danh sách: phần trăm bề rộng client, kẹp trong khoảng (DIP).
const LIST_PCT: i32 = 32;
const LIST_MIN: i32 = 220;
const LIST_MAX: i32 = 320;
const LABEL_GAP: i32 = 6;
const CAPTION_H: i32 = 96;
const FIELD_PAD: i32 = 6;
const ROW_H: i32 = 60;
const ROW_PAD: i32 = 8;
const THUMB_W: i32 = 72;
const THUMB_H: i32 = 44;
const PREVIEW_PAD: i32 = 8;
const PREVIEW_RADIUS: i32 = 6;
/// Thẻ xác nhận.
const CARD_W: i32 = 400;
const CARD_PAD: i32 = 24;
const CARD_TITLE_GAP: i32 = 8;
const CARD_BTN_TOP: i32 = 24;
/// Độ đục lớp phủ mờ khi thẻ xác nhận mở (như Lịch sử).
const DIM_ALPHA: u8 = 190;

/// Nút của cửa sổ, theo thứ tự trong `Layout::buttons`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Btn {
    Up,
    Down,
    Delete,
    Export,
    End,
    Close,
}

const BUTTONS: [Btn; 6] = [
    Btn::Up,
    Btn::Down,
    Btn::Delete,
    Btn::Export,
    Btn::End,
    Btn::Close,
];

impl Btn {
    fn label(self) -> Msg {
        match self {
            Btn::Up => Msg::StepsUp,
            Btn::Down => Msg::StepsDown,
            Btn::Delete => Msg::StepsDelete,
            Btn::Export => Msg::StepsExport,
            Btn::End => Msg::StepsEndSession,
            Btn::Close => Msg::StepsClose,
        }
    }

    fn index(self) -> usize {
        BUTTONS.iter().position(|b| *b == self).unwrap_or(0)
    }

    fn kind(self) -> ButtonKind {
        match self {
            Btn::Export => ButtonKind::Primary,
            _ => ButtonKind::Normal,
        }
    }
}

/// Cỡ chữ đã đo (pixel): chiều cao dòng, bề rộng nhãn "Ghi chú", bề rộng chữ từng nút.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Measure {
    text_h: i32,
    label_w: i32,
    btn_text: [i32; 6],
}

/// Bố cục (pixel, tọa độ client).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Layout {
    dpi: u32,
    /// Vùng thân (danh sách và cột phải; trạng thái trống và thẻ xác nhận nằm giữa vùng này).
    body: Rect,
    list: Rect,
    preview: Rect,
    label: Rect,
    /// Khung tự vẽ của ô ghi chú; `edit` là điều khiển EDIT bên trong.
    field: Rect,
    edit: Rect,
    buttons: [Rect; 6],
    row_h: i32,
    thumb: (i32, i32),
}

const ZERO: Rect = Rect {
    x: 0,
    y: 0,
    w: 0,
    h: 0,
};

/// Bề rộng nút cho chữ rộng `text_px` ở `dpi`: tối thiểu `BTN_MIN`, đủ chữ và lề hai bên.
fn button_width(text_px: i32, dpi: u32) -> i32 {
    scale(BTN_MIN, dpi).max(text_px.max(0) + 2 * scale(BTN_PAD, dpi))
}

/// Bố cục trong client `cw`×`ch` pixel. Hàng nút dưới cùng: Lên, Xuống, Xóa bước sát trái; Xuất…,
/// Kết thúc phiên, Đóng sát phải; không đủ một hàng thì nhóm trái lên hàng trên. Thân: danh sách
/// bên trái; bên phải xem trước, nhãn "Ghi chú" và khung ô ghi chú. Cỡ nhỏ hơn tối thiểu thì các phần
/// co về 0 chứ không âm.
fn layout(cw: i32, ch: i32, dpi: u32, m: &Measure) -> Layout {
    let s = |v: i32| scale(v, dpi);
    let (mg, gap, bgap) = (s(MARGIN), s(GAP), s(BTN_GAP));
    let btn_h = s(BTN_H).max(m.text_h + s(12));
    let w: [i32; 6] = std::array::from_fn(|k| button_width(m.btn_text[k], dpi));
    let left_w = w[0] + w[1] + w[2] + 2 * bgap;
    let right_w = w[3] + w[4] + w[5] + 2 * bgap;
    let one_row = mg + left_w + 2 * gap + right_w + mg <= cw;
    let bottom_y = ch - mg - btn_h;
    let left_y = if one_row {
        bottom_y
    } else {
        bottom_y - bgap - btn_h
    };
    let mut buttons = [ZERO; 6];
    let mut x = mg;
    for k in 0..3 {
        buttons[k] = Rect {
            x,
            y: left_y,
            w: w[k],
            h: btn_h,
        };
        x += w[k] + bgap;
    }
    let mut x = cw - mg;
    for k in (3..6).rev() {
        x -= w[k];
        buttons[k] = Rect {
            x,
            y: bottom_y,
            w: w[k],
            h: btn_h,
        };
        x -= bgap;
    }
    let body = Rect {
        x: mg,
        y: mg,
        w: (cw - 2 * mg).max(0),
        h: (left_y - gap - mg).max(0),
    };
    let list_w = (cw * LIST_PCT / 100)
        .clamp(s(LIST_MIN), s(LIST_MAX))
        .min((body.w - gap) / 2)
        .max(0);
    let list = Rect {
        x: body.x,
        y: body.y,
        w: list_w,
        h: body.h,
    };
    let x0 = list.right() + gap;
    let rw = (body.right() - x0).max(0);
    let field_h = s(CAPTION_H)
        .max(4 * m.text_h + 2 * s(FIELD_PAD))
        .min(body.h);
    let field = Rect {
        x: x0,
        y: body.bottom() - field_h,
        w: rw,
        h: field_h,
    };
    let label = Rect {
        x: x0,
        y: (field.y - s(LABEL_GAP) - m.text_h).max(body.y),
        w: rw,
        h: m.text_h.max(0),
    };
    let preview = Rect {
        x: x0,
        y: body.y,
        w: rw,
        h: (label.y - gap - body.y).max(0),
    };
    let pad = s(FIELD_PAD);
    let edit = Rect {
        x: field.x + pad,
        y: field.y + pad,
        w: (field.w - 2 * pad).max(0),
        h: (field.h - 2 * pad).max(0),
    };
    let row_h = s(ROW_H).max(2 * m.text_h + 2 * s(ROW_PAD) + s(4));
    let thumb = (s(THUMB_W), s(THUMB_H).min(row_h - 2 * s(ROW_PAD)));
    Layout {
        dpi,
        body,
        list,
        preview,
        label,
        field,
        edit,
        buttons,
        row_h,
        thumb,
    }
}

/// Thẻ xác nhận (pixel, tọa độ client cửa sổ).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Card {
    card: Rect,
    title: Rect,
    text: Rect,
    cancel: Rect,
    yes: Rect,
}

/// Bề rộng đoạn giải thích của thẻ trong thân rộng `body_w` (để đo chiều cao chữ xuống dòng).
fn card_text_width(body_w: i32, dpi: u32) -> i32 {
    (scale(CARD_W, dpi).min(body_w) - 2 * scale(CARD_PAD, dpi)).max(1)
}

/// Thẻ rộng 400 DIP (không quá thân) giữa `body`; `title_h`, `text_h` là chiều cao chữ đã đo;
/// `btn_w` là bề rộng nút Hủy và Kết thúc. Hai nút cùng hàng ở góc dưới phải, Hủy bên trái.
fn card_rects(body: Rect, dpi: u32, title_h: i32, text_h: i32, btn_w: [i32; 2]) -> Card {
    let s = |v: i32| scale(v, dpi);
    let pad = s(CARD_PAD);
    let w = s(CARD_W).min(body.w).max(0);
    let btn_h = s(BTN_H);
    let h = 2 * pad + title_h + s(CARD_TITLE_GAP) + text_h + s(CARD_BTN_TOP) + btn_h;
    let x = body.x + (body.w - w) / 2;
    let y = (body.y + (body.h - h) / 2).max(body.y);
    let inner = (w - 2 * pad).max(0);
    let title = Rect {
        x: x + pad,
        y: y + pad,
        w: inner,
        h: title_h,
    };
    let text = Rect {
        x: x + pad,
        y: title.bottom() + s(CARD_TITLE_GAP),
        w: inner,
        h: text_h,
    };
    let bt = text.bottom() + s(CARD_BTN_TOP);
    let yes_w = btn_w[1].min(inner);
    let yes = Rect {
        x: x + w - pad - yes_w,
        y: bt,
        w: yes_w,
        h: btn_h,
    };
    let cancel_w = btn_w[0].min((yes.x - s(BTN_GAP) - (x + pad)).max(0));
    let cancel = Rect {
        x: yes.x - s(BTN_GAP) - cancel_w,
        y: bt,
        w: cancel_w,
        h: btn_h,
    };
    Card {
        card: Rect { x, y, w, h },
        title,
        text,
        cancel,
        yes,
    }
}

/// Nút nào bật (thứ tự `BUTTONS`) khi chọn `sel` trong `count` bước. Đang xuất thì không xuất lần
/// nữa, không xóa ảnh và không kết thúc phiên (ảnh nguồn đang được chép).
fn button_enabled(sel: Option<usize>, count: usize, exporting: bool) -> [bool; 6] {
    let sel = sel.filter(|&i| i < count);
    [
        sel.is_some_and(|i| i > 0),
        sel.is_some_and(|i| i + 1 < count),
        sel.is_some() && !exporting,
        count > 0 && !exporting,
        !exporting,
        true,
    ]
}

/// Bước được chọn sau khi danh sách đổi: bước cũ (theo tên tệp) nếu còn; không thì cùng vị trí, kẹp
/// vào cuối danh sách (xóa bước: bước kế tiếp, hay bước trước khi xóa bước cuối).
fn reselect(old_file: Option<&str>, old_index: Option<usize>, new: &[Step]) -> Option<usize> {
    if new.is_empty() {
        return None;
    }
    if let Some(i) = old_file.and_then(|f| new.iter().position(|s| s.file == f)) {
        return Some(i);
    }
    Some(old_index.unwrap_or(0).min(new.len() - 1))
}

/// Dòng có chữ đầu tiên của ghi chú (đã bỏ khoảng trắng hai đầu).
fn first_line(caption: &str) -> &str {
    caption
        .split(['\r', '\n'])
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("")
}

/// Mọi kiểu xuống dòng thành `\r\n` (ô EDIT nhiều dòng chỉ hiểu `\r\n`).
fn to_crlf(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '\r' => {
                if it.peek() == Some(&'\n') {
                    it.next();
                }
                out.push_str("\r\n");
            }
            '\n' => out.push_str("\r\n"),
            c => out.push(c),
        }
    }
    out
}

/// Chữ của ô EDIT về dạng lưu (`\n`).
fn from_crlf(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}

/// Vị trí cuộn hợp lệ cho `count` dòng cao `row_h` trong khung cao `view_h`.
fn scroll_clamp(scroll: i32, count: usize, row_h: i32, view_h: i32) -> i32 {
    let content = (count as i32).saturating_mul(row_h);
    scroll.clamp(0, (content - view_h).max(0))
}

/// Vị trí cuộn để dòng `i` nằm trọn trong khung (cuộn ít nhất).
fn ensure_visible(i: usize, scroll: i32, row_h: i32, view_h: i32) -> i32 {
    let top = (i as i32).saturating_mul(row_h);
    if top < scroll {
        top
    } else if top + row_h > scroll + view_h {
        (top + row_h - view_h).max(0)
    } else {
        scroll
    }
}

/// Dòng dưới điểm `y` (tọa độ client danh sách).
fn row_at(y: i32, scroll: i32, row_h: i32, count: usize) -> Option<usize> {
    if y < 0 || row_h <= 0 {
        return None;
    }
    let i = ((y + scroll) / row_h) as usize;
    (i < count).then_some(i)
}

/// Phím di chuyển trong danh sách.
fn nav_of(vk: u16) -> Option<Nav> {
    match vk {
        v if v == VK_UP.0 => Some(Nav::Up),
        v if v == VK_DOWN.0 => Some(Nav::Down),
        v if v == VK_HOME.0 => Some(Nav::Home),
        v if v == VK_END.0 => Some(Nav::End),
        v if v == VK_PRIOR.0 => Some(Nav::PageUp),
        v if v == VK_NEXT.0 => Some(Nav::PageDown),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Luồng nền: ảnh nhỏ và xem trước
// ---------------------------------------------------------------------------

/// Điểm ảnh `0x00RRGGBB` đã thu nhỏ: (rộng, cao, điểm ảnh); `None` = không đọc được.
type Pixels = Option<(u32, u32, Vec<u32>)>;

enum WorkResult {
    Thumb { gen: u64, file: String, px: Pixels },
    Preview { gen: u64, px: Pixels },
}

struct ThumbJob {
    gen: u64,
    file: String,
    path: PathBuf,
    cell: (u32, u32),
}

struct PreviewJob {
    gen: u64,
    path: PathBuf,
    target: (u32, u32),
}

type PreviewSlot = Arc<(Mutex<Option<PreviewJob>>, Condvar)>;
type Results = Arc<Mutex<Vec<WorkResult>>>;

/// Giải mã PNG `path` và thu nhỏ vừa `max` (không phóng to), trộn lên nền `INPUT`.
/// Panic khi giải mã (bản debug) thành ảnh vỡ để luồng nạp sống tiếp; bản phát hành `panic =
/// "abort"` nên giải mã vốn báo lỗi bằng `Err`.
fn decode_fit(path: &Path, max: (u32, u32)) -> Pixels {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let img = decode_png_file(path).ok()?;
        let small = downscale(&img, max.0.max(1), max.1.max(1));
        drop(img);
        Some((small.width, small.height, compose_over(&small, INPUT)))
    }))
    .unwrap_or(None)
}

/// Hai luồng nền sống cùng cửa sổ. Bỏ `Workers` thì hai luồng dừng sau việc đang làm.
struct Workers {
    stop: Arc<AtomicBool>,
    thumbs: Option<mpsc::Sender<ThumbJob>>,
    preview: PreviewSlot,
    results: Results,
}

fn push_result(results: &Results, hwnd: isize, r: WorkResult) {
    results.lock().unwrap_or_else(|e| e.into_inner()).push(r);
    // SAFETY: PostMessageW an toàn từ luồng bất kỳ; thông điệp không mang con trỏ, cửa sổ đã hủy
    // thì lỗi và ta bỏ qua (kết quả bị bỏ cùng hàng).
    unsafe {
        let _ = PostMessageW(
            Some(HWND(hwnd as *mut c_void)),
            WM_STEPS_WORK,
            WPARAM(0),
            LPARAM(0),
        );
    }
}

impl Workers {
    fn start(hwnd: HWND) -> Workers {
        let h = hwnd.0 as isize;
        let stop = Arc::new(AtomicBool::new(false));
        let results: Results = Arc::new(Mutex::new(Vec::new()));
        let (tx, rx) = mpsc::channel::<ThumbJob>();
        let (st, res) = (stop.clone(), results.clone());
        let thumbs = std::thread::Builder::new()
            .name("darkshot-steps-thumb".into())
            .spawn(move || {
                for job in rx {
                    if st.load(Ordering::SeqCst) {
                        break;
                    }
                    let px = decode_fit(&job.path, job.cell);
                    if st.load(Ordering::SeqCst) {
                        break;
                    }
                    push_result(
                        &res,
                        h,
                        WorkResult::Thumb {
                            gen: job.gen,
                            file: job.file,
                            px,
                        },
                    );
                }
            })
            .is_ok()
            .then_some(tx);
        let preview: PreviewSlot = Arc::new((Mutex::new(None), Condvar::new()));
        let (slot, st, res) = (preview.clone(), stop.clone(), results.clone());
        let _ = std::thread::Builder::new()
            .name("darkshot-steps-preview".into())
            .spawn(move || loop {
                let job = {
                    let (m, cv) = &*slot;
                    let mut g = m.lock().unwrap_or_else(|e| e.into_inner());
                    loop {
                        if st.load(Ordering::SeqCst) {
                            return;
                        }
                        if let Some(j) = g.take() {
                            break j;
                        }
                        g = cv.wait(g).unwrap_or_else(|e| e.into_inner());
                    }
                };
                let px = decode_fit(&job.path, job.target);
                if st.load(Ordering::SeqCst) {
                    return;
                }
                push_result(&res, h, WorkResult::Preview { gen: job.gen, px });
            });
        Workers {
            stop,
            thumbs,
            preview,
            results,
        }
    }

    fn thumb(&self, job: ThumbJob) {
        if let Some(tx) = &self.thumbs {
            let _ = tx.send(job);
        }
    }

    fn preview(&self, job: PreviewJob) {
        let (m, cv) = &*self.preview;
        *m.lock().unwrap_or_else(|e| e.into_inner()) = Some(job);
        cv.notify_one();
    }

    fn take_results(&self) -> Vec<WorkResult> {
        std::mem::take(&mut *self.results.lock().unwrap_or_else(|e| e.into_inner()))
    }
}

impl Drop for Workers {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.thumbs.take();
        let (m, cv) = &*self.preview;
        m.lock().unwrap_or_else(|e| e.into_inner()).take();
        cv.notify_all();
    }
}

// ---------------------------------------------------------------------------
// Trạng thái cửa sổ
// ---------------------------------------------------------------------------

/// Phông của cửa sổ (thường, đậm, tiêu đề thẻ); xóa khi bị bỏ (sau khi điều khiển đã thôi dùng).
struct FontSet(Fonts);

impl FontSet {
    fn new(dpi: u32) -> FontSet {
        FontSet(Fonts {
            normal: winutil::system_font(dpi),
            bold: winutil::system_font_styled(dpi, 100, true),
            title: winutil::system_font_styled(dpi, 130, true),
        })
    }
}

impl Drop for FontSet {
    fn drop(&mut self) {
        for f in [self.0.normal, self.0.bold, self.0.title]
            .into_iter()
            .flatten()
        {
            // SAFETY: phông do cửa sổ tạo; không điều khiển nào còn dùng (đã đổi phông hay đã hủy).
            unsafe {
                let _ = DeleteObject(HGDIOBJ(f.0));
            }
        }
    }
}

enum Thumb {
    Pending,
    Ready(Bmp),
    Broken,
}

enum Preview {
    Idle,
    Loading,
    Ready(Bmp),
    Broken,
}

#[derive(Clone, Copy)]
struct Ctl {
    list: HWND,
    edit: HWND,
    buttons: [HWND; 6],
    card: HWND,
    card_btns: [HWND; 2],
}

struct State {
    hwnd: HWND,
    ctl: Ctl,
    dpi: u32,
    fonts: FontSet,
    measure: Measure,
    layout: Layout,
    /// Bản chụp các bước của phiên và đường dẫn ảnh tương ứng.
    steps: Vec<Step>,
    paths: Vec<PathBuf>,
    sel: Option<usize>,
    scroll: i32,
    /// Chiều cao client thật của danh sách.
    view_h: i32,
    /// Tệp của bước có ghi chú đang nằm trong ô EDIT.
    edit_file: Option<String>,
    thumbs: HashMap<String, Thumb>,
    thumb_gen: u64,
    preview: Preview,
    preview_file: Option<String>,
    preview_size: (i32, i32),
    preview_gen: u64,
    workers: Workers,
    confirm: Confirm,
    card: Option<Card>,
    /// Điều khiển có tiêu điểm khi cửa sổ mất kích hoạt.
    focus: HWND,
    /// Đang bị hủy: không làm mới theo phiên nữa.
    closing: bool,
}

type Notify = fn(NoticeKind, &str, &str);

fn no_dir() -> Option<PathBuf> {
    None
}

/// Một lượt xuất đã xong: các bước lúc bắt đầu xuất và kết quả.
struct ExportDone {
    steps: Vec<Step>,
    result: Result<PathBuf, String>,
}

/// Kết quả xuất chờ luồng giao diện (`on_exported`).
static EXPORT_DONE: Mutex<Vec<ExportDone>> = Mutex::new(Vec::new());
/// Cửa sổ chính nhận `WM_STEPS_EXPORTED` (0: chưa có).
static MAIN_WINDOW: AtomicIsize = AtomicIsize::new(0);

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    static HWND_CELL: Cell<Option<HWND>> = const { Cell::new(None) };
    static NOTIFY: Cell<Option<Notify>> = const { Cell::new(None) };
    static DEFAULT_DIR: Cell<fn() -> Option<PathBuf>> = const { Cell::new(no_dir) };
    /// Đang có một lượt xuất chạy nền.
    static EXPORTING: Cell<bool> = const { Cell::new(false) };
    /// Hộp chọn thư mục của nút Xuất… đang mở (vòng lặp modal).
    static PICKING: Cell<bool> = const { Cell::new(false) };
    /// Luồng xuất đang chạy (để lúc thoát chờ có hạn).
    static EXPORT_THREAD: RefCell<Option<std::thread::JoinHandle<()>>> = const { RefCell::new(None) };
}

/// Đang xuất hay đang chọn thư mục để xuất: không kết thúc phiên.
fn busy() -> bool {
    EXPORTING.with(|e| e.get()) || PICKING.with(|p| p.get())
}

fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    STATE.with(|s| s.try_borrow_mut().ok()?.as_mut().map(f))
}

/// Hàm báo thông báo qua khay (main).
pub fn set_notifier(f: Notify) {
    NOTIFY.with(|n| n.set(Some(f)));
}

fn notify(kind: NoticeKind, title: &str, body: &str) {
    if let Some(f) = NOTIFY.with(|n| n.get()) {
        f(kind, title, body);
    }
}

/// Thư mục mặc định của hộp chọn thư mục khi xuất (main: thư mục lưu ảnh của cấu hình).
pub fn set_default_dir(f: fn() -> Option<PathBuf>) {
    DEFAULT_DIR.with(|d| d.set(f));
}

/// Cửa sổ chính nhận `WM_STEPS_EXPORTED`.
pub fn set_main_window(h: HWND) {
    MAIN_WINDOW.store(h.0 as isize, Ordering::SeqCst);
}

/// Cửa sổ đang mở (handle còn sống).
fn current() -> Option<HWND> {
    // SAFETY: IsWindow chỉ hỏi tính hợp lệ của handle.
    HWND_CELL
        .with(|c| c.get())
        .filter(|h| unsafe { IsWindow(Some(*h)) }.as_bool())
}

fn to_rect(r: Rect) -> RECT {
    RECT {
        left: r.x,
        top: r.y,
        right: r.right(),
        bottom: r.bottom(),
    }
}

fn set_text(h: HWND, s: &str) {
    // SAFETY: cửa sổ hay điều khiển của luồng này; chuỗi sống suốt lời gọi.
    unsafe {
        let _ = SetWindowTextW(h, &HSTRING::from(s));
    }
}

fn set_font(h: HWND, f: Option<HFONT>) {
    if let (Some(f), false) = (f, h.is_invalid()) {
        // SAFETY: phông sống tới khi cửa sổ đổi phông hoặc bị hủy.
        unsafe {
            SendMessageW(h, WM_SETFONT, Some(WPARAM(f.0 as usize)), Some(LPARAM(1)));
        }
    }
}

fn invalidate(h: HWND) {
    // SAFETY: cửa sổ của luồng này.
    unsafe {
        let _ = InvalidateRect(Some(h), None, false);
    }
}

fn key_down(vk: u16) -> bool {
    // SAFETY: chỉ đọc trạng thái phím của luồng này.
    unsafe { GetKeyState(i32::from(vk)) < 0 }
}

fn measure_texts(f: &Fonts, lang: Lang) -> Measure {
    hp::with_screen_dc(|dc| {
        let mut m = Measure {
            text_h: hp::measure(dc, f.normal, "Ag").1,
            label_w: hp::measure(dc, f.normal, t(lang, Msg::StepsCaption)).0,
            btn_text: [0; 6],
        };
        for (k, b) in BUTTONS.iter().enumerate() {
            let font = if b.kind() == ButtonKind::Primary {
                f.bold
            } else {
                f.normal
            };
            m.btn_text[k] = hp::measure(dc, font, t(lang, b.label())).0;
        }
        m
    })
}

// ---------------------------------------------------------------------------
// Mở, đóng
// ---------------------------------------------------------------------------

/// Mở cửa sổ Xem lại (đang mở thì đưa ra trước). Không có phiên thì không làm gì.
pub fn open() {
    if let Some(h) = current() {
        winutil::bring_to_front(h);
        return;
    }
    if !steps_store::active() {
        return;
    }
    create();
}

/// Kết thúc phiên từ thanh phiên: còn bước chưa xuất thì mở cửa sổ với thẻ xác nhận; không thì
/// kết thúc ngay.
pub fn open_confirm_end() {
    if !steps_store::active() {
        return;
    }
    if busy() {
        // Ảnh đang được chép hay hộp chọn thư mục đang mở: không kết thúc giữa chừng; cửa sổ đang
        // mở thì đưa ra trước.
        if let Some(h) = current() {
            winutil::bring_to_front(h);
        }
        return;
    }
    // Thanh phiên không lấy tiêu điểm nên ô ghi chú chưa mất tiêu điểm: lưu trước khi xét
    // "đã xuất chưa" (cửa sổ chưa mở thì không có gì để lưu).
    commit_caption();
    if !needs_confirm() {
        end_session();
        return;
    }
    open();
    if current().is_some() {
        open_card();
    }
}

/// Đóng cửa sổ (nếu đang mở); ghi chú đang sửa được lưu ở `WM_DESTROY`.
pub fn close() {
    if let Some(h) = current() {
        // SAFETY: cửa sổ của luồng này; WM_DESTROY/WM_NCDESTROY dọn dẹp.
        unsafe {
            let _ = DestroyWindow(h);
        }
    }
}

fn register_classes(hinst: windows::Win32::Foundation::HINSTANCE) {
    // SAFETY: con trỏ hệ thống dùng chung, không phải hủy; đăng ký lại lần hai báo lỗi vô hại.
    unsafe {
        let arrow = LoadCursorW(None, IDC_ARROW).unwrap_or_default();
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: hinst,
            lpszClassName: CLASS,
            hbrBackground: theme::brush(BG),
            hCursor: arrow,
            ..Default::default()
        };
        RegisterClassW(&wc);
        let lc = WNDCLASSW {
            style: CS_DBLCLKS,
            lpfnWndProc: Some(list_proc),
            hInstance: hinst,
            lpszClassName: LIST_CLASS,
            hCursor: arrow,
            ..Default::default()
        };
        RegisterClassW(&lc);
        let cc = WNDCLASSW {
            lpfnWndProc: Some(card_proc),
            hInstance: hinst,
            lpszClassName: CARD_CLASS,
            hCursor: arrow,
            hbrBackground: HBRUSH::default(),
            ..Default::default()
        };
        RegisterClassW(&cc);
    }
}

fn child(parent: HWND, class: PCWSTR, text: &str, style: WINDOW_STYLE, id: i32) -> HWND {
    // SAFETY: tạo điều khiển con ở luồng giao diện; vị trí đặt sau ở `relayout`.
    unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class,
            &HSTRING::from(text),
            // WS_CLIPSIBLINGS: không vẽ đè lên thẻ xác nhận (anh em nằm trên cùng khi mở).
            WS_CHILD | WS_CLIPSIBLINGS | style,
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

/// Cỡ cửa sổ (pixel) cho vùng client `w`×`h` DIP ở `dpi`.
fn window_size(w: i32, h: i32, dpi: u32) -> (i32, i32) {
    let mut rc = RECT {
        left: 0,
        top: 0,
        right: scale(w, dpi),
        bottom: scale(h, dpi),
    };
    // SAFETY: rc là biến cục bộ.
    unsafe {
        let _ = AdjustWindowRectExForDpi(&mut rc, STYLE, false, EX_STYLE, dpi);
    }
    (rc.right - rc.left, rc.bottom - rc.top)
}

/// Tạo, đăng ký và hiện cửa sổ giữa màn hình có con trỏ. Lỗi thì không còn gì sót.
fn create() -> Option<HWND> {
    let lang = lang::current();
    // SAFETY: tham số ra là biến cục bộ; MonitorFromPoint chỉ đọc.
    let mon = unsafe {
        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST)
    };
    let dpi = monitor_dpi(mon);
    let (ww, wh) = window_size(DEFAULT_W, DEFAULT_H, dpi);
    let mut mi = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: mi là biến cục bộ có cbSize đúng.
    let (x, y) = if unsafe { GetMonitorInfoW(mon, &mut mi) }.as_bool() {
        let a = mi.rcWork;
        (
            (a.left + (a.right - a.left - ww) / 2).max(a.left),
            (a.top + (a.bottom - a.top - wh) / 2).max(a.top),
        )
    } else {
        (100, 100)
    };
    // SAFETY: đăng ký lớp và tạo cửa sổ ở luồng giao diện.
    let hwnd = unsafe {
        let hinst = GetModuleHandleW(None).ok()?;
        register_classes(hinst.into());
        CreateWindowExW(
            EX_STYLE,
            CLASS,
            &HSTRING::from(t(lang, Msg::StepsWindowTitle)),
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
        .ok()?
    };
    // SAFETY: hwnd vừa tạo ở luồng này.
    let dpi = match unsafe { GetDpiForWindow(hwnd) } {
        0 => dpi,
        d => d,
    };
    let fonts = FontSet::new(dpi);
    let measure = measure_texts(&fonts.0, lang);
    let list = child(
        hwnd,
        LIST_CLASS,
        "",
        WINDOW_STYLE(WS_TABSTOP.0 | WS_GROUP.0 | WS_VSCROLL.0),
        ID_LIST,
    );
    let edit_style = WINDOW_STYLE(
        WS_TABSTOP.0
            | WS_GROUP.0
            | WS_VSCROLL.0
            | ES_MULTILINE as u32
            | ES_AUTOVSCROLL as u32
            | ES_WANTRETURN as u32,
    );
    let edit = child(hwnd, w!("EDIT"), "", edit_style, ID_EDIT);
    let push = WINDOW_STYLE(WS_TABSTOP.0 | BS_PUSHBUTTON as u32);
    let buttons: [HWND; 6] = std::array::from_fn(|k| {
        child(
            hwnd,
            w!("BUTTON"),
            t(lang, BUTTONS[k].label()),
            if k == 0 || k == 3 {
                WINDOW_STYLE(push.0 | WS_GROUP.0)
            } else {
                push
            },
            ID_BUTTONS[k],
        )
    });
    let card = child(hwnd, CARD_CLASS, "", WINDOW_STYLE(WS_GROUP.0), ID_CARD);
    let card_btns = [
        child(
            card,
            w!("BUTTON"),
            t(lang, Msg::StepsCancel),
            WINDOW_STYLE(push.0 | WS_GROUP.0 | WS_SHOW_STYLE),
            ID_CARD_CANCEL,
        ),
        child(
            card,
            w!("BUTTON"),
            t(lang, Msg::StepsEndConfirmYes),
            WINDOW_STYLE(push.0 | WS_SHOW_STYLE),
            ID_CARD_YES,
        ),
    ];
    let ctl = Ctl {
        list,
        edit,
        buttons,
        card,
        card_btns,
    };
    let l = layout(0, 0, dpi, &measure);
    let workers = Workers::start(hwnd);
    STATE.with(|s| {
        *s.borrow_mut() = Some(State {
            hwnd,
            ctl,
            dpi,
            fonts,
            measure,
            layout: l,
            steps: Vec::new(),
            paths: Vec::new(),
            sel: None,
            scroll: 0,
            view_h: 0,
            edit_file: None,
            thumbs: HashMap::new(),
            thumb_gen: 0,
            preview: Preview::Idle,
            preview_file: None,
            preview_size: (0, 0),
            preview_gen: 0,
            workers,
            confirm: Confirm::default(),
            card: None,
            focus: HWND::default(),
            closing: false,
        })
    });
    HWND_CELL.with(|c| c.set(Some(hwnd)));
    let all = [list, edit, card]
        .into_iter()
        .chain(buttons)
        .chain(card_btns);
    if all.clone().any(|h| h.is_invalid()) {
        // WM_NCDESTROY bỏ trạng thái (phông, luồng nền).
        // SAFETY: cửa sổ vừa tạo của luồng này.
        unsafe {
            let _ = DestroyWindow(hwnd);
        }
        return None;
    }
    let normal = with_state(|s| s.fonts.0.normal).flatten();
    for h in all {
        set_font(h, normal);
    }
    theme::set_dark_control_theme(list, theme::DarkClass::Explorer);
    theme::set_dark_control_theme(edit, theme::DarkClass::Explorer);
    // SAFETY: ô ghi chú của cửa sổ này; giới hạn theo đơn vị UTF-16 (không vượt MAX_CAPTION ký tự).
    unsafe {
        SendMessageW(edit, EM_LIMITTEXT, Some(WPARAM(MAX_CAPTION)), None);
    }
    refresh();
    theme::apply_dark_title_bar(hwnd);
    winutil::set_app_icons(hwnd);
    // SAFETY: hwnd là cửa sổ của luồng này.
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(hwnd);
    }
    restore_focus(hwnd);
    Some(hwnd)
}

/// Kiểu WS_VISIBLE cho nút của thẻ (thẻ ẩn/hiện cả cụm).
const WS_SHOW_STYLE: u32 = windows::Win32::UI::WindowsAndMessaging::WS_VISIBLE.0;

// ---------------------------------------------------------------------------
// Đồng bộ với phiên
// ---------------------------------------------------------------------------

/// Phiên đổi (main đăng ký qua `steps_store::on_change`): phiên hết thì đóng cửa sổ, không thì
/// làm mới danh sách.
pub fn on_store_changed() {
    if current().is_none() {
        return;
    }
    if !steps_store::active() {
        close();
        return;
    }
    refresh();
}

/// Đọc lại các bước từ phiên; giữ bước đang chọn (theo tên tệp) và ghi chú đang sửa.
fn refresh() {
    let Some(h) = current() else {
        return;
    };
    if with_state(|s| s.closing).unwrap_or(true) {
        return;
    }
    let steps = steps_store::steps();
    let paths: Vec<PathBuf> = (0..steps.len())
        .map(|i| steps_store::image_path(i).unwrap_or_default())
        .collect();
    let reload = with_state(|s| {
        let old_file = s.sel.and_then(|i| s.steps.get(i)).map(|x| x.file.clone());
        let sel = reselect(old_file.as_deref(), s.sel, &steps);
        s.thumbs.retain(|f, _| steps.iter().any(|x| x.file == *f));
        if s.edit_file
            .as_ref()
            .is_some_and(|f| !steps.iter().any(|x| x.file == *f))
        {
            s.edit_file = None;
        }
        s.steps = steps;
        s.paths = paths;
        s.sel = sel;
        if let (Some(i), true) = (sel, s.view_h > 0) {
            s.scroll = ensure_visible(i, s.scroll, s.layout.row_h, s.view_h);
        }
        let new_file = sel.and_then(|i| s.steps.get(i)).map(|x| x.file.clone());
        new_file != s.edit_file
    })
    .unwrap_or(false);
    if reload {
        load_caption();
    }
    relayout(h);
}

/// Đưa ghi chú của bước đang chọn vào ô EDIT.
fn load_caption() {
    let Some((edit, text)) = with_state(|s| {
        let st = s.sel.and_then(|i| s.steps.get(i));
        s.edit_file = st.map(|x| x.file.clone());
        (
            s.ctl.edit,
            st.map(|x| to_crlf(&x.caption)).unwrap_or_default(),
        )
    }) else {
        return;
    };
    set_text(edit, &text);
}

/// Lưu chữ trong ô EDIT vào bước mà nó thuộc về (theo tên tệp), nếu khác ghi chú đang lưu.
fn commit_caption() {
    let Some((edit, Some(file))) = with_state(|s| (s.ctl.edit, s.edit_file.clone())) else {
        return;
    };
    // SAFETY: chỉ hỏi handle còn sống.
    if edit.is_invalid() || !unsafe { IsWindow(Some(edit)) }.as_bool() {
        return;
    }
    let text = from_crlf(&String::from_utf16_lossy(&theme::window_text(edit)));
    let steps = steps_store::steps();
    let Some(i) = steps.iter().position(|s| s.file == file) else {
        return;
    };
    let kept: String = text.chars().take(MAX_CAPTION).collect();
    if steps.get(i).is_some_and(|s| s.caption != kept) {
        steps_store::set_caption(i, &text);
    }
}

/// Chọn bước `i` (lưu ghi chú đang sửa trước), cuộn cho nó hiện ra.
fn set_selection(i: Option<usize>) {
    let Some(h) = current() else {
        return;
    };
    let same = with_state(|s| s.sel == i.filter(|&k| k < s.steps.len())).unwrap_or(true);
    if same {
        ensure_sel_visible();
        return;
    }
    commit_caption();
    with_state(|s| {
        s.sel = i.filter(|&k| k < s.steps.len());
    });
    load_caption();
    ensure_sel_visible();
    relayout(h);
}

fn ensure_sel_visible() {
    let Some(list) = with_state(|s| {
        if let (Some(i), true) = (s.sel, s.view_h > 0) {
            s.scroll = ensure_visible(i, s.scroll, s.layout.row_h, s.view_h);
            s.scroll = scroll_clamp(s.scroll, s.steps.len(), s.layout.row_h, s.view_h);
        }
        s.ctl.list
    }) else {
        return;
    };
    update_scrollbar(list);
    invalidate(list);
}

// ---------------------------------------------------------------------------
// Bố cục và điều khiển
// ---------------------------------------------------------------------------

struct Plan {
    ctl: Ctl,
    l: Layout,
    enabled: [bool; 6],
    modal: bool,
    card: Option<Card>,
    empty: bool,
}

fn relayout(h: HWND) {
    let mut rc = RECT::default();
    // SAFETY: rc là biến cục bộ.
    unsafe {
        let _ = GetClientRect(h, &mut rc);
    }
    let lang = lang::current();
    let exporting = EXPORTING.with(|e| e.get());
    let plan = with_state(|s| {
        let l = layout(rc.right, rc.bottom, s.dpi, &s.measure);
        s.layout = l;
        let count = s.steps.len();
        let modal = s.confirm.is_open();
        let card = modal.then(|| {
            let title = t(lang, Msg::StepsEndSession);
            let body = i18n::steps_end_confirm(lang, count);
            let f = &s.fonts.0;
            let (th, bh) = hp::with_screen_dc(|dc| {
                (
                    hp::measure(dc, f.title, title).1,
                    hp::measure_wrapped_left(dc, f.normal, &body, card_text_width(l.body.w, l.dpi)),
                )
            });
            let bw = hp::with_screen_dc(|dc| {
                [
                    button_width(
                        hp::measure(dc, f.normal, t(lang, Msg::StepsCancel)).0,
                        l.dpi,
                    ),
                    button_width(
                        hp::measure(dc, f.bold, t(lang, Msg::StepsEndConfirmYes)).0,
                        l.dpi,
                    ),
                ]
            });
            card_rects(l.body, l.dpi, th, bh, bw)
        });
        s.card = card;
        request_preview(s);
        Plan {
            ctl: s.ctl,
            l,
            enabled: button_enabled(s.sel, count, exporting),
            modal,
            card,
            empty: count == 0,
        }
    });
    let Some(p) = plan else {
        return;
    };
    let c = p.ctl;
    let mv = |w: HWND, r: Rect, show: bool| {
        // SAFETY: đặt vị trí và hiện/ẩn điều khiển con của luồng này (không giữ mượn STATE).
        unsafe {
            let _ = MoveWindow(w, r.x, r.y, r.w, r.h, false);
            let _ = ShowWindow(w, if show { SW_SHOW } else { SW_HIDE });
        }
    };
    let enable = |w: HWND, on: bool| {
        // SAFETY: bật/tắt điều khiển con của luồng này.
        unsafe {
            let _ = EnableWindow(w, on && !p.modal);
        }
    };
    mv(c.list, p.l.list, !p.empty);
    enable(c.list, true);
    mv(c.edit, p.l.edit, !p.empty);
    enable(c.edit, true);
    for (k, b) in c.buttons.iter().enumerate() {
        mv(*b, p.l.buttons[k], true);
        enable(*b, p.enabled[k]);
    }
    match p.card {
        Some(cr) => {
            let k = cr.card;
            let rel = |r: Rect| Rect {
                x: r.x - k.x,
                y: r.y - k.y,
                ..r
            };
            // SAFETY: đặt thẻ (con của cửa sổ này) lên trên cùng các anh em và hiện nó.
            unsafe {
                let _ = SetWindowPos(c.card, Some(HWND_TOP), k.x, k.y, k.w, k.h, SWP_SHOWWINDOW);
            }
            let r2 = 2 * scale(CARD_RADIUS, p.l.dpi);
            // SAFETY: vùng mới tạo được giao cho hệ thống sau SetWindowRgn (không xóa lại); lỗi thì xóa.
            unsafe {
                let rgn = CreateRoundRectRgn(0, 0, k.w + 1, k.h + 1, r2, r2);
                if !rgn.is_invalid() && SetWindowRgn(c.card, Some(rgn), true) == 0 {
                    let _ = DeleteObject(HGDIOBJ(rgn.0));
                }
            }
            for (b, r) in c.card_btns.iter().zip([cr.cancel, cr.yes]) {
                let r = rel(r);
                // SAFETY: nút con của thẻ.
                unsafe {
                    let _ = MoveWindow(*b, r.x, r.y, r.w, r.h, false);
                    let _ = EnableWindow(*b, true);
                }
            }
        }
        None => mv(c.card, ZERO, false),
    }
    let mut lc = RECT::default();
    // SAFETY: lc là biến cục bộ.
    unsafe {
        let _ = GetClientRect(c.list, &mut lc);
    }
    with_state(|s| {
        s.view_h = lc.bottom.max(0);
        s.scroll = scroll_clamp(s.scroll, s.steps.len(), s.layout.row_h, s.view_h);
    });
    update_scrollbar(c.list);
    fix_focus(h);
    // SAFETY: chỉ đánh dấu vùng cần vẽ lại (cả các con).
    unsafe {
        let _ = RedrawWindow(Some(h), None, None, RDW_INVALIDATE | RDW_ALLCHILDREN);
    }
}

/// Nhờ luồng nền giải mã bản xem trước khi bước chọn hay cỡ vùng xem trước đổi.
fn request_preview(s: &mut State) {
    let want = s.sel.and_then(|i| s.steps.get(i)).map(|x| x.file.clone());
    let pad = scale(PREVIEW_PAD, s.dpi);
    let size = (
        (s.layout.preview.w - 2 * pad).max(0),
        (s.layout.preview.h - 2 * pad).max(0),
    );
    if want == s.preview_file && size == s.preview_size {
        return;
    }
    if want != s.preview_file {
        s.preview = if want.is_some() {
            Preview::Loading
        } else {
            Preview::Idle
        };
    }
    s.preview_file = want;
    s.preview_size = size;
    let path = s.sel.and_then(|i| s.paths.get(i)).cloned();
    if let (Some(path), true) = (path, size.0 > 0 && size.1 > 0) {
        s.preview_gen += 1;
        s.workers.preview(PreviewJob {
            gen: s.preview_gen,
            path,
            target: preview_target(size.0, size.1),
        });
    }
}

fn update_scrollbar(list: HWND) {
    let Some((scroll, content, view)) = with_state(|s| {
        (
            s.scroll,
            (s.steps.len() as i32).saturating_mul(s.layout.row_h),
            s.view_h,
        )
    }) else {
        return;
    };
    let si = SCROLLINFO {
        cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
        fMask: SIF_RANGE | SIF_PAGE | SIF_POS,
        nMin: 0,
        nMax: (content - 1).max(0),
        nPage: view.max(0) as u32,
        nPos: scroll,
        nTrackPos: 0,
    };
    // SAFETY: thanh cuộn của danh sách (con của cửa sổ này); si là biến cục bộ.
    unsafe {
        SetScrollInfo(list, SB_VERT, &si, true);
    }
}

fn usable(h: HWND) -> bool {
    // SAFETY: chỉ hỏi trạng thái của điều khiển.
    unsafe { IsWindowVisible(h).as_bool() && IsWindowEnabled(h).as_bool() }
}

/// Tiêu điểm đang ở điều khiển bị ẩn hay vô hiệu (sau khi bố cục đổi): chuyển sang chỗ dùng được.
fn fix_focus(h: HWND) {
    // SAFETY: chỉ hỏi tiêu điểm và quan hệ cửa sổ của luồng này.
    let f = unsafe { GetFocus() };
    // SAFETY: như trên.
    let ours = !f.is_invalid() && (f == h || unsafe { IsChild(h, f) }.as_bool());
    if ours && f != h && usable(f) {
        return;
    }
    if !ours {
        return;
    }
    with_state(|s| s.focus = HWND::default());
    restore_focus(h);
}

/// Điều khiển nhận tiêu điểm: cái đã nhớ nếu còn dùng được; thẻ mở thì nút Hủy; không thì danh
/// sách, còn lại nút Đóng.
fn restore_focus(h: HWND) {
    let Some((saved, c, modal)) = with_state(|s| (s.focus, s.ctl, s.confirm.is_open())) else {
        return;
    };
    let target = if modal {
        c.card_btns[0]
    // SAFETY: chỉ hỏi quan hệ cửa sổ.
    } else if !saved.is_invalid() && unsafe { IsChild(h, saved) }.as_bool() && usable(saved) {
        saved
    } else if usable(c.list) {
        c.list
    } else {
        c.buttons[Btn::Close.index()]
    };
    // SAFETY: điều khiển con của cửa sổ này.
    unsafe {
        let _ = SetFocus(Some(target));
    }
}

/// Ngôn ngữ đổi: tiêu đề, chữ nút, nhãn, chữ danh sách và thẻ.
pub fn relabel() {
    let Some(h) = current() else {
        return;
    };
    let lang = lang::current();
    let Some(c) = with_state(|s| {
        s.measure = measure_texts(&s.fonts.0, lang);
        s.ctl
    }) else {
        return;
    };
    set_text(h, t(lang, Msg::StepsWindowTitle));
    for (k, b) in c.buttons.iter().enumerate() {
        set_text(*b, t(lang, BUTTONS[k].label()));
    }
    set_text(c.card_btns[0], t(lang, Msg::StepsCancel));
    set_text(c.card_btns[1], t(lang, Msg::StepsEndConfirmYes));
    relayout(h);
    invalidate(c.card);
}

fn on_dpi_changed(dpi: u32) {
    let fonts = FontSet::new(dpi);
    let lang = lang::current();
    let measure = measure_texts(&fonts.0, lang);
    let normal = fonts.0.normal;
    let Some((old, c)) = with_state(|s| {
        s.dpi = dpi;
        s.measure = measure;
        s.thumbs.clear();
        s.thumb_gen += 1;
        // Ép yêu cầu lại bản xem trước theo cỡ mới.
        s.preview_size = (0, 0);
        (std::mem::replace(&mut s.fonts, fonts), s.ctl)
    }) else {
        return;
    };
    for w in [c.list, c.edit]
        .into_iter()
        .chain(c.buttons)
        .chain(c.card_btns)
    {
        set_font(w, normal);
    }
    // Các điều khiển đã dùng phông mới: phông cũ xóa được.
    drop(old);
}

// ---------------------------------------------------------------------------
// Hành động
// ---------------------------------------------------------------------------

fn needs_confirm() -> bool {
    steps_store::dirty() && steps_store::count() > 0
}

fn on_button(h: HWND, b: Btn) {
    let Some((sel, count, modal)) = with_state(|s| (s.sel, s.steps.len(), s.confirm.is_open()))
    else {
        return;
    };
    if modal || !button_enabled(sel, count, EXPORTING.with(|e| e.get()))[b.index()] {
        return;
    }
    match (b, sel) {
        (Btn::Up, Some(i)) => {
            commit_caption();
            steps_store::move_up(i);
            refresh();
        }
        (Btn::Down, Some(i)) => {
            commit_caption();
            steps_store::move_down(i);
            refresh();
        }
        (Btn::Delete, Some(i)) => {
            commit_caption();
            steps_store::remove(i);
            refresh();
        }
        (Btn::Export, _) => on_export_click(h),
        (Btn::End, _) => request_end(),
        (Btn::Close, _) => close(),
        _ => {}
    }
}

/// Nút Kết thúc phiên: còn bước chưa xuất thì hỏi bằng thẻ; không thì kết thúc ngay.
fn request_end() {
    if busy() {
        return;
    }
    commit_caption();
    if needs_confirm() {
        open_card();
    } else {
        end_session();
    }
}

/// Kết thúc phiên. Thành công thì cửa sổ đóng (thanh phiên đóng theo `on_change` của main); lỗi
/// thì phiên và cửa sổ giữ nguyên (lỗi đã báo qua `steps_store`).
fn end_session() {
    if steps_store::end() {
        close();
    }
}

// ---------------------------------------------------------------------------
// Thẻ xác nhận
// ---------------------------------------------------------------------------

fn open_card() {
    let Some(h) = current() else {
        return;
    };
    let opened = with_state(|s| {
        if s.confirm.is_open() {
            false
        } else {
            s.confirm.open();
            true
        }
    });
    if opened != Some(true) {
        return;
    }
    relayout(h);
    focus_card_button(ConfirmBtn::Cancel);
}

fn focus_card_button(b: ConfirmBtn) {
    let Some(btns) = with_state(|s| {
        s.confirm.set_focus(b);
        s.ctl.card_btns
    }) else {
        return;
    };
    let target = match b {
        ConfirmBtn::Cancel => btns[0],
        ConfirmBtn::Yes => btns[1],
    };
    // SAFETY: nút của thẻ (con của cửa sổ này).
    unsafe {
        let _ = SetFocus(Some(target));
    }
}

/// Đóng thẻ; `yes` = kết thúc phiên.
fn close_card(yes: bool) {
    let Some(h) = current() else {
        return;
    };
    let Some(end_btn) = with_state(|s| {
        s.confirm.close();
        s.focus = s.ctl.buttons[Btn::End.index()];
        s.ctl.buttons[Btn::End.index()]
    }) else {
        return;
    };
    relayout(h);
    // SAFETY: đưa tiêu điểm về nút Kết thúc phiên (con của cửa sổ này).
    unsafe {
        let _ = SetFocus(Some(end_btn));
    }
    if yes {
        // Có thể hủy chính cửa sổ này: không đụng gì sau đó.
        end_session();
    }
}

/// Đi bằng bàn phím mà ta tự xử lý: hiện khung tiêu điểm như `IsDialogMessageW` làm.
fn show_focus_cues(h: HWND) {
    use windows::Win32::UI::WindowsAndMessaging::{UISF_HIDEFOCUS, UIS_CLEAR, WM_CHANGEUISTATE};
    let wp = (UIS_CLEAR as usize) | ((UISF_HIDEFOCUS as usize) << 16);
    // SAFETY: gửi đồng bộ tới cửa sổ của luồng này.
    unsafe {
        SendMessageW(h, WM_CHANGEUISTATE, Some(WPARAM(wp)), Some(LPARAM(0)));
    }
}

/// Phím khi thẻ mở: Esc = Hủy, Enter = nút đang có tiêu điểm, Tab xoay giữa hai nút.
/// `true` = đã xử lý.
fn confirm_key(h: HWND, vk: u16, shift: bool) -> bool {
    let key = match vk {
        v if v == VK_ESCAPE.0 => ConfirmKey::Esc,
        v if v == VK_RETURN.0 => ConfirmKey::Enter,
        v if v == VK_TAB.0 && shift => ConfirmKey::ShiftTab,
        v if v == VK_TAB.0 => ConfirmKey::Tab,
        _ => return false,
    };
    // SAFETY: chỉ hỏi cửa sổ có tiêu điểm.
    let f = unsafe { GetFocus() };
    let out = with_state(|s| {
        if f == s.ctl.card_btns[1] {
            s.confirm.set_focus(ConfirmBtn::Yes);
        } else if f == s.ctl.card_btns[0] {
            s.confirm.set_focus(ConfirmBtn::Cancel);
        }
        let out = s.confirm.key(key);
        (out, s.confirm.focus())
    });
    if matches!(key, ConfirmKey::Tab | ConfirmKey::ShiftTab) {
        show_focus_cues(h);
    }
    match out {
        Some((ConfirmOutcome::Stay, b)) => focus_card_button(b),
        Some((ConfirmOutcome::Cancel, _)) => close_card(false),
        Some((ConfirmOutcome::Confirm, _)) => close_card(true),
        None => {}
    }
    true
}

// ---------------------------------------------------------------------------
// Xuất
// ---------------------------------------------------------------------------

/// Nút Xuất…: lưu ghi chú, hỏi thư mục (hộp chọn của Windows, mặc định thư mục lưu ảnh), rồi
/// xuất trên luồng nền.
fn on_export_click(h: HWND) {
    if busy() || steps_store::count() == 0 {
        return;
    }
    commit_caption();
    let default = DEFAULT_DIR.with(|d| d.get())();
    PICKING.with(|p| p.set(true));
    let picked = pick_folder(h, default.as_deref());
    PICKING.with(|p| p.set(false));
    let Some(parent) = picked else {
        return;
    };
    // Hộp chọn là vòng lặp modal: phiên có thể đã kết thúc hay thẻ xác nhận đã mở trong lúc đó.
    if with_state(|s| s.confirm.is_open()).unwrap_or(false) {
        return;
    }
    start_export(parent);
}

/// Hộp chọn thư mục của Windows; `None` khi người dùng hủy hay lỗi.
fn pick_folder(owner: HWND, default: Option<&Path>) -> Option<PathBuf> {
    let title = t(lang::current(), Msg::StepsExport).trim_end_matches('…');
    // SAFETY: COM được khởi tạo và giải phóng cân bằng trong hàm này (các đối tượng COM được thả
    // trước CoUninitialize); chuỗi do COM cấp được giải phóng bằng CoTaskMemFree.
    unsafe {
        let inited = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();
        let r = (|| -> windows::core::Result<String> {
            let dlg: IFileOpenDialog =
                CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)?;
            let opts = dlg.GetOptions()?;
            dlg.SetOptions(opts | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM)?;
            dlg.SetTitle(&HSTRING::from(title))?;
            if let Some(d) = default.filter(|d| d.is_dir()) {
                let item: windows::core::Result<IShellItem> =
                    SHCreateItemFromParsingName(&HSTRING::from(d.as_os_str()), None::<&IBindCtx>);
                if let Ok(item) = item {
                    let _ = dlg.SetFolder(&item);
                }
            }
            dlg.Show(Some(owner))?;
            let item = dlg.GetResult()?;
            let name: PWSTR = item.GetDisplayName(SIGDN_FILESYSPATH)?;
            let s = name.to_string();
            CoTaskMemFree(Some(name.0 as *const c_void));
            Ok(s.unwrap_or_default())
        })();
        if inited {
            CoUninitialize();
        }
        r.ok().filter(|s| !s.is_empty()).map(PathBuf::from)
    }
}

/// Bắt đầu xuất các bước hiện có vào thư mục con mới của `parent` trên luồng nền. `false` khi
/// không có gì để xuất, đang xuất, hay không tạo được luồng (đã báo lỗi).
fn start_export(parent: PathBuf) -> bool {
    if EXPORTING.with(|e| e.get()) || !steps_store::active() {
        return false;
    }
    let steps = steps_store::steps();
    if steps.is_empty() {
        return false;
    }
    let sources: Vec<PathBuf> = (0..steps.len())
        .filter_map(steps_store::image_path)
        .collect();
    let lang = lang::current();
    let job = ExportJob {
        parent,
        when: paths::local_now(),
        lang,
        steps,
        sources,
    };
    let spawned = std::thread::Builder::new()
        .name("darkshot-steps-export".into())
        .spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                steps_export::export(&job)
            }))
            .unwrap_or_else(|_| Err(t(job.lang, Msg::StepsWorkerFailed).to_string()));
            if let Ok(dir) = &result {
                open_in_explorer(dir);
            }
            EXPORT_DONE
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(ExportDone {
                    steps: job.steps,
                    result,
                });
            let main = MAIN_WINDOW.load(Ordering::SeqCst);
            if main != 0 {
                // SAFETY: PostMessageW an toàn từ luồng bất kỳ; thông điệp không mang con trỏ.
                unsafe {
                    let _ = PostMessageW(
                        Some(HWND(main as *mut c_void)),
                        WM_STEPS_EXPORTED,
                        WPARAM(0),
                        LPARAM(0),
                    );
                }
            }
        });
    match spawned {
        Ok(handle) => {
            EXPORT_THREAD.with(|t| *t.borrow_mut() = Some(handle));
            EXPORTING.with(|e| e.set(true));
            if let Some(h) = current() {
                relayout(h);
            }
            true
        }
        Err(e) => {
            notify(
                NoticeKind::Error,
                t(lang, Msg::StepsExportFailedTitle),
                &i18n::reason_try_again(lang, &e.to_string()),
            );
            false
        }
    }
}

#[cfg(test)]
fn wait_export() {
    if let Some(t) = EXPORT_THREAD.with(|t| t.borrow_mut().take()) {
        let _ = t.join();
    }
}

/// Lúc thoát: chờ lượt xuất đang chạy xong tối đa `timeout` (xong thì thư mục đã đổi sang tên cuối;
/// chưa xong thì thư mục tạm còn sót được dọn ở lần xuất sau vào cùng thư mục cha).
pub fn wait_export_on_exit(timeout: std::time::Duration) {
    let Some(t) = EXPORT_THREAD.with(|t| t.borrow_mut().take()) else {
        return;
    };
    let until = std::time::Instant::now() + timeout;
    while !t.is_finished() && std::time::Instant::now() < until {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    if t.is_finished() {
        let _ = t.join();
    }
}

/// Lượt xuất xong (main gọi ở `WM_STEPS_EXPORTED`): đánh dấu đã xuất nếu phiên không đổi trong lúc
/// xuất, báo thông báo; lỗi thì báo lỗi, phiên giữ nguyên.
pub fn on_exported() {
    let done: Vec<ExportDone> =
        std::mem::take(&mut *EXPORT_DONE.lock().unwrap_or_else(|e| e.into_inner()));
    if done.is_empty() {
        return;
    }
    EXPORTING.with(|e| e.set(false));
    let lang = lang::current();
    for d in done {
        match d.result {
            Ok(dir) => {
                if steps_store::active() && steps_store::steps() == d.steps {
                    steps_store::mark_exported();
                }
                let name = dir
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                notify(
                    NoticeKind::Success,
                    &i18n::steps_exported(lang, d.steps.len()),
                    &name,
                );
            }
            Err(why) => notify(
                NoticeKind::Error,
                t(lang, Msg::StepsExportFailedTitle),
                &i18n::reason_try_again(lang, &why),
            ),
        }
    }
    if let Some(h) = current() {
        relayout(h);
    }
}

/// Thư mục Windows (`GetWindowsDirectoryW`), không hỏi được thì `None`.
#[cfg_attr(test, allow(dead_code))]
fn windows_dir() -> Option<String> {
    let mut buf = [0u16; 260];
    // SAFETY: buf là bộ đệm cục bộ; hàm ghi tối đa buf.len() ký tự và trả số ký tự đã ghi.
    let n =
        unsafe { windows::Win32::System::SystemInformation::GetWindowsDirectoryW(Some(&mut buf)) }
            as usize;
    (n > 0 && n < buf.len()).then(|| String::from_utf16_lossy(&buf[..n]))
}

/// Mở Explorer chọn sẵn thư mục `dir` (chạy trên luồng xuất). Kiểm thử không mở gì.
fn open_in_explorer(dir: &Path) {
    if cfg!(test) {
        return;
    }
    // Đường dẫn lạ (không tuyệt đối, có `"` hay ký tự điều khiển): không gọi explorer.
    let Some(args) = explorer_select_args(dir) else {
        return;
    };
    let exe = HSTRING::from(explorer_exe(windows_dir().as_deref()));
    let args = HSTRING::from(args);
    // SAFETY: các chuỗi sống đến hết lời gọi; COM khởi tạo cân bằng quanh ShellExecuteW (cần COM ở
    // luồng gọi).
    unsafe {
        let inited =
            CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE).is_ok();
        let _ = ShellExecuteW(
            None,
            w!("open"),
            &exe,
            PCWSTR(args.as_ptr()),
            None,
            SW_SHOWNORMAL,
        );
        if inited {
            CoUninitialize();
        }
    }
}

// ---------------------------------------------------------------------------
// Kết quả luồng nền
// ---------------------------------------------------------------------------

fn on_work() {
    let Some(results) = with_state(|s| s.workers.take_results()) else {
        return;
    };
    let (mut list_dirty, mut preview_dirty) = (false, false);
    for r in results {
        match r {
            WorkResult::Thumb { gen, file, px } => {
                let bmp = px.and_then(|(w, h, p)| hp::bmp_from_pixels(w, h, &p));
                with_state(|s| {
                    if gen == s.thumb_gen && s.thumbs.contains_key(&file) {
                        s.thumbs
                            .insert(file, bmp.map_or(Thumb::Broken, Thumb::Ready));
                        list_dirty = true;
                    }
                });
            }
            WorkResult::Preview { gen, px } => {
                let bmp = px.and_then(|(w, h, p)| hp::bmp_from_pixels(w, h, &p));
                with_state(|s| {
                    if gen == s.preview_gen {
                        s.preview = bmp.map_or(Preview::Broken, Preview::Ready);
                        preview_dirty = true;
                    }
                });
            }
        }
    }
    let Some((h, list, preview)) = with_state(|s| (s.hwnd, s.ctl.list, s.layout.preview)) else {
        return;
    };
    if list_dirty {
        invalidate(list);
    }
    if preview_dirty {
        let rc = to_rect(preview);
        // SAFETY: vẽ lại vùng xem trước của cửa sổ này.
        unsafe {
            let _ = InvalidateRect(Some(h), Some(&rc), false);
        }
    }
}

// ---------------------------------------------------------------------------
// Vẽ
// ---------------------------------------------------------------------------

const ONE_LINE: DRAW_TEXT_FORMAT =
    DRAW_TEXT_FORMAT(DT_SINGLELINE.0 | DT_VCENTER.0 | DT_END_ELLIPSIS.0 | DT_NOPREFIX.0);

fn text(
    dc: HDC,
    s: &str,
    r: &RECT,
    c: darkshot_core::theme::Rgb,
    font: Option<HFONT>,
    f: DRAW_TEXT_FORMAT,
) {
    theme::draw_text(dc, &hp::wide(s), r, c, font, f | DT_NOPREFIX);
}

fn paint_main(h: HWND) {
    let lang = lang::current();
    // Hỏi trước khi mượn STATE.
    // SAFETY: chỉ hỏi cửa sổ có tiêu điểm.
    let focus = unsafe { GetFocus() };
    let mut ps = PAINTSTRUCT::default();
    // SAFETY: BeginPaint/EndPaint cặp đôi trong WM_PAINT của cửa sổ này.
    let hdc = unsafe { BeginPaint(h, &mut ps) };
    let mut client = RECT::default();
    // SAFETY: client là biến cục bộ.
    unsafe {
        let _ = GetClientRect(h, &mut client);
    }
    with_state(|s| {
        let l = s.layout;
        let dpi = s.dpi;
        let modal = s.confirm.is_open();
        let f = &s.fonts.0;
        theme::buffered(hdc, &client, |dc| {
            theme::fill(dc, &client, BG);
            if s.steps.is_empty() {
                // Trạng thái trống: biểu tượng ảnh và lời nhắc giữa thân.
                let body = l.body;
                let msg = t(lang, Msg::StepsEmpty);
                let tw = body.w.min(scale(360, dpi)).max(1);
                let th = hp::measure_wrapped(dc, f.normal, msg, tw);
                let icon = scale(48, dpi);
                let total = icon + scale(12, dpi) + th;
                let top = body.y + ((body.h - total) / 2).max(0);
                let ix = body.x + (body.w - icon) / 2;
                let ir = RECT {
                    left: ix,
                    top,
                    right: ix + icon,
                    bottom: top + icon * 3 / 4,
                };
                hp::picture_icon(dc, &ir, TEXT_DISABLED, BG, dpi);
                let tx = body.x + (body.w - tw) / 2;
                let tr = RECT {
                    left: tx,
                    top: top + icon + scale(12, dpi),
                    right: tx + tw,
                    bottom: top + total,
                };
                text(dc, msg, &tr, TEXT_MUTED, f.normal, DT_WORDBREAK | DT_CENTER);
            } else {
                let pr = to_rect(l.preview);
                theme::round_rect(
                    dc,
                    &pr,
                    scale(PREVIEW_RADIUS, dpi),
                    INPUT,
                    LINE,
                    scale(1, dpi).max(1),
                );
                let pad = scale(PREVIEW_PAD, dpi);
                let inner = RECT {
                    left: pr.left + pad,
                    top: pr.top + pad,
                    right: pr.right - pad,
                    bottom: pr.bottom - pad,
                };
                match &s.preview {
                    Preview::Ready(b) => hp::blit(dc, b, &fit_rect(b.w as u32, b.h as u32, &inner)),
                    Preview::Broken => {
                        let d = scale(48, dpi);
                        let cx = (inner.left + inner.right) / 2;
                        let cy = (inner.top + inner.bottom) / 2;
                        let r = RECT {
                            left: cx - d / 2,
                            top: cy - d * 3 / 8,
                            right: cx + d / 2,
                            bottom: cy + d * 3 / 8,
                        };
                        hp::broken_icon(dc, &r, TEXT_DISABLED, INPUT, dpi);
                    }
                    Preview::Idle | Preview::Loading => {}
                }
                text(
                    dc,
                    t(lang, Msg::StepsCaption),
                    &to_rect(l.label),
                    TEXT_MUTED,
                    f.normal,
                    DT_SINGLELINE | DT_VCENTER | DT_LEFT,
                );
                theme::draw_field(
                    dc,
                    &to_rect(l.field),
                    dpi,
                    theme::edit_border(focus == s.ctl.edit && !modal, !modal),
                );
            }
            if modal {
                hp::dim(dc, &client, DIM_ALPHA);
            }
        });
    });
    // SAFETY: kết thúc lần vẽ đã mở ở trên.
    unsafe {
        let _ = EndPaint(h, &ps);
    }
}

fn paint_list(list: HWND) {
    let lang = lang::current();
    // Hỏi trước khi mượn STATE: các hàm này gửi thông điệp tới chính danh sách.
    let (show_focus, _) = theme::ui_cues(list);
    // SAFETY: chỉ hỏi cửa sổ có tiêu điểm.
    let focused = unsafe { GetFocus() } == list && show_focus;
    let mut ps = PAINTSTRUCT::default();
    // SAFETY: BeginPaint/EndPaint cặp đôi trong WM_PAINT của danh sách.
    let hdc = unsafe { BeginPaint(list, &mut ps) };
    let mut client = RECT::default();
    // SAFETY: client là biến cục bộ.
    unsafe {
        let _ = GetClientRect(list, &mut client);
    }
    with_state(|s| {
        let mut jobs = Vec::new();
        let (dpi, row_h, thumb) = (s.dpi, s.layout.row_h.max(1), s.layout.thumb);
        let k = |v: i32| scale(v, dpi);
        let first = (s.scroll / row_h).max(0) as usize;
        let last = ((s.scroll + client.bottom) / row_h + 1).max(0) as usize;
        let modal = s.confirm.is_open();
        let f = &s.fonts.0;
        theme::buffered(hdc, &client, |dc| {
            theme::fill(dc, &client, INPUT);
            for i in first..last.min(s.steps.len()) {
                let Some(step) = s.steps.get(i) else {
                    break;
                };
                let top = i as i32 * row_h - s.scroll;
                let r = RECT {
                    left: 0,
                    top,
                    right: client.right,
                    bottom: top + row_h,
                };
                let selected = s.sel == Some(i);
                if selected {
                    theme::fill(dc, &r, BUTTON);
                    let bar = RECT {
                        right: r.left + k(3).max(2),
                        ..r
                    };
                    theme::fill(dc, &bar, ACCENT);
                }
                let cell = RECT {
                    left: r.left + k(ROW_PAD) + k(2),
                    top: top + (row_h - thumb.1) / 2,
                    right: r.left + k(ROW_PAD) + k(2) + thumb.0,
                    bottom: top + (row_h - thumb.1) / 2 + thumb.1,
                };
                theme::fill(dc, &cell, INPUT);
                match s.thumbs.get(&step.file) {
                    Some(Thumb::Ready(b)) => {
                        hp::blit(dc, b, &fit_rect(b.w as u32, b.h as u32, &cell));
                    }
                    Some(Thumb::Broken) => {
                        hp::broken_icon(dc, &cell, TEXT_DISABLED, INPUT, dpi);
                    }
                    Some(Thumb::Pending) => {}
                    None => {
                        if let Some(path) = s.paths.get(i) {
                            jobs.push(ThumbJob {
                                gen: s.thumb_gen,
                                file: step.file.clone(),
                                path: path.clone(),
                                cell: (thumb.0.max(1) as u32, thumb.1.max(1) as u32),
                            });
                        }
                    }
                }
                let tx = cell.right + k(10);
                let line_h = s.measure.text_h;
                let ty = top + (row_h - 2 * line_h - k(2)) / 2;
                let l1 = RECT {
                    left: tx,
                    top: ty,
                    right: r.right - k(ROW_PAD),
                    bottom: ty + line_h,
                };
                let l2 = RECT {
                    top: l1.bottom + k(2),
                    bottom: l1.bottom + k(2) + line_h,
                    ..l1
                };
                text(
                    dc,
                    &step_label(lang, i + 1),
                    &l1,
                    TEXT,
                    f.normal,
                    ONE_LINE | DT_LEFT,
                );
                text(
                    dc,
                    first_line(&step.caption),
                    &l2,
                    TEXT_MUTED,
                    f.normal,
                    ONE_LINE | DT_LEFT,
                );
                if selected && focused && !modal {
                    let fr = RECT {
                        left: r.left + 2,
                        top: r.top + 2,
                        right: r.right - 2,
                        bottom: r.bottom - 2,
                    };
                    theme::focus_frame(dc, &fr, dpi);
                }
            }
            if modal {
                hp::dim(dc, &client, DIM_ALPHA);
            }
        });
        for j in jobs {
            s.thumbs.insert(j.file.clone(), Thumb::Pending);
            s.workers.thumb(j);
        }
    });
    // SAFETY: kết thúc lần vẽ đã mở ở trên.
    unsafe {
        let _ = EndPaint(list, &ps);
    }
}

fn paint_card(card: HWND) {
    let lang = lang::current();
    let mut ps = PAINTSTRUCT::default();
    // SAFETY: BeginPaint/EndPaint cặp đôi trong WM_PAINT của thẻ.
    let hdc = unsafe { BeginPaint(card, &mut ps) };
    with_state(|s| {
        let Some(cr) = s.card else {
            return;
        };
        let k = cr.card;
        let rel = |r: Rect| RECT {
            left: r.x - k.x,
            top: r.y - k.y,
            right: r.right() - k.x,
            bottom: r.bottom() - k.y,
        };
        let body = i18n::steps_end_confirm(lang, s.steps.len());
        let all = rel(k);
        theme::buffered(hdc, &all, |dc| {
            hp::draw_card(
                dc,
                &all,
                &rel(cr.title),
                &rel(cr.text),
                t(lang, Msg::StepsEndSession),
                &body,
                &s.fonts.0,
                s.dpi,
            );
        });
    });
    // SAFETY: kết thúc lần vẽ đã mở ở trên.
    unsafe {
        let _ = EndPaint(card, &ps);
    }
}

// ---------------------------------------------------------------------------
// Thủ tục cửa sổ
// ---------------------------------------------------------------------------

/// `true`: thông điệp đã được xử lý, vòng lặp bỏ qua. Chỉ đụng thông điệp của cửa sổ này (và con).
pub fn pre_translate(msg: &MSG) -> bool {
    let Some(h) = current() else {
        return false;
    };
    if !winutil::belongs_to(h, msg) {
        return false;
    }
    let Some((c, modal)) = with_state(|s| (s.ctl, s.confirm.is_open())) else {
        return false;
    };
    if msg.message == WM_KEYDOWN {
        let vk = msg.wParam.0 as u16;
        let ctrl = key_down(VK_CONTROL.0);
        let shift = key_down(VK_SHIFT.0);
        if modal {
            if confirm_key(h, vk, shift) {
                return true;
            }
            // Space bấm nút của thẻ; phím khác không đi tới điều khiển phía sau.
            return vk != VK_SPACE.0;
        }
        // SAFETY: chỉ hỏi điều khiển có tiêu điểm của luồng này.
        let focus = unsafe { GetFocus() };
        if vk == VK_ESCAPE.0 {
            close();
            return true;
        }
        if vk == VK_TAB.0 && !ctrl {
            // SAFETY: đi giữa các điều khiển có WS_TABSTOP của cửa sổ này.
            unsafe {
                if let Ok(next) = GetNextDlgTabItem(h, Some(focus), shift) {
                    let _ = SetFocus(Some(next));
                }
            }
            show_focus_cues(h);
            return true;
        }
        if focus == c.list && (nav_of(vk).is_some() || vk == VK_DELETE.0) {
            // Danh sách tự xử lý (list_proc).
            return false;
        }
        if focus == c.edit {
            if vk == u16::from(b'A') && ctrl {
                // SAFETY: chọn hết chữ trong ô ghi chú của cửa sổ này.
                unsafe {
                    SendMessageW(c.edit, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
                }
                return true;
            }
            // Enter xuống dòng, mũi tên đi trong chữ.
            return false;
        }
    }
    // SAFETY: phím điều hướng còn lại (Space trên nút, gợi ý bàn phím) của cửa sổ này.
    unsafe { IsDialogMessageW(h, msg).as_bool() }
}

fn lo_word(v: usize) -> u32 {
    (v & 0xFFFF) as u32
}

unsafe extern "system" fn wndproc(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_STEPS_WORK => {
            on_work();
            LRESULT(0)
        }
        WM_COMMAND => {
            let (id, code) = ((wp.0 & 0xFFFF) as i32, ((wp.0 >> 16) & 0xFFFF) as u32);
            if let Some(k) = ID_BUTTONS.iter().position(|x| *x == id) {
                if code == 0 {
                    on_button(h, BUTTONS[k]);
                }
            } else if id == ID_CARD_CANCEL && code == 0 {
                close_card(false);
            } else if id == ID_CARD_YES && code == 0 {
                close_card(true);
            } else if id == ID_EDIT && (code == EN_KILLFOCUS || code == EN_SETFOCUS) {
                if code == EN_KILLFOCUS {
                    commit_caption();
                }
                // Gạch dưới của khung đổi theo tiêu điểm.
                if let Some(f) = with_state(|s| s.layout.field) {
                    let rc = to_rect(f);
                    // SAFETY: vẽ lại vùng khung của cửa sổ này.
                    unsafe {
                        let _ = InvalidateRect(Some(h), Some(&rc), false);
                    }
                }
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            close();
            LRESULT(0)
        }
        WM_CTLCOLOREDIT | WM_CTLCOLORSTATIC => {
            let hdc = HDC(wp.0 as *mut c_void);
            let modal = with_state(|s| s.confirm.is_open()).unwrap_or(false);
            let fg = if modal { TEXT_DISABLED } else { TEXT };
            theme::ctl_color(hdc, fg, INPUT)
        }
        WM_NOTIFY => {
            // SAFETY: lp của WM_NOTIFY trỏ tới NMHDR hợp lệ suốt lời gọi.
            let hdr = unsafe { (lp.0 as *const NMHDR).as_ref() };
            let id = hdr
                .filter(|x| x.code == NM_CUSTOMDRAW)
                .map(|x| x.idFrom as i32);
            if let Some(k) = id.and_then(|id| ID_BUTTONS.iter().position(|x| *x == id)) {
                // SAFETY: mã NM_CUSTOMDRAW từ nút nên lp trỏ tới NMCUSTOMDRAW.
                if let Some(cd) = unsafe { theme::nmcd_of(lp) } {
                    let (dpi, bold, modal) =
                        with_state(|s| (s.dpi, s.fonts.0.bold, s.confirm.is_open()))
                            .unwrap_or((96, None, false));
                    let r = theme::custom_draw_button(cd, BUTTONS[k].kind(), BG, dpi, bold);
                    if modal && cd.dwDrawStage == windows::Win32::UI::Controls::CDDS_PREPAINT {
                        hp::dim(cd.hdc, &cd.rc, DIM_ALPHA);
                    }
                    return r;
                }
            }
            // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
            unsafe { DefWindowProcW(h, msg, wp, lp) }
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            paint_main(h);
            LRESULT(0)
        }
        WM_SIZE => {
            relayout(h);
            LRESULT(0)
        }
        WM_GETMINMAXINFO => {
            // SAFETY: hwnd là cửa sổ này (có thể đang được tạo); chỉ hỏi DPI.
            let dpi = match unsafe { GetDpiForWindow(h) } {
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
            on_dpi_changed(dpi);
            // Tài liệu WM_DPICHANGED: dùng đúng RECT đề nghị trong lp.
            // SAFETY: lp trỏ tới RECT hợp lệ suốt lời gọi.
            if let Some(r) = unsafe { (lp.0 as *const RECT).as_ref() } {
                // SAFETY: đổi vị trí và cỡ chính cửa sổ này (WM_SIZE tính lại bố cục).
                unsafe {
                    let _ = SetWindowPos(
                        h,
                        None,
                        r.left,
                        r.top,
                        r.right - r.left,
                        r.bottom - r.top,
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                }
            }
            relayout(h);
            LRESULT(0)
        }
        WM_ACTIVATE => {
            if lo_word(wp.0) == WA_INACTIVE {
                // SAFETY: chỉ hỏi tiêu điểm và quan hệ cửa sổ của luồng này.
                let f = unsafe { GetFocus() };
                if !f.is_invalid() && unsafe { IsChild(h, f) }.as_bool() {
                    with_state(|s| s.focus = f);
                }
            } else {
                restore_focus(h);
            }
            LRESULT(0)
        }
        WM_SETFOCUS => {
            restore_focus(h);
            LRESULT(0)
        }
        WM_DESTROY => {
            // Điều khiển con còn sống ở đây: lưu ghi chú đang sửa (mọi đường đóng).
            with_state(|s| s.closing = true);
            commit_caption();
            LRESULT(0)
        }
        WM_NCDESTROY => {
            HWND_CELL.with(|c| {
                if c.get() == Some(h) {
                    c.set(None);
                }
            });
            // Thả trạng thái (luồng nền, ảnh, phông) sau khi trả mượn; mọi điều khiển đã bị hủy.
            let old = STATE.with(|s| {
                let mut b = s.try_borrow_mut().ok()?;
                if b.as_ref().is_some_and(|x| x.hwnd == h) {
                    b.take()
                } else {
                    None
                }
            });
            drop(old);
            // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
            unsafe { DefWindowProcW(h, msg, wp, lp) }
        }
        // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
        _ => unsafe { DefWindowProcW(h, msg, wp, lp) },
    }
}

fn on_list_key(vk: u16) {
    let Some((h, sel, count, view, row_h)) = with_state(|s| {
        (
            s.hwnd,
            s.sel,
            s.steps.len(),
            s.view_h,
            s.layout.row_h.max(1),
        )
    }) else {
        return;
    };
    if vk == VK_DELETE.0 {
        on_button(h, Btn::Delete);
        return;
    }
    if let Some(nav) = nav_of(vk) {
        let page = (view / row_h).max(1) as usize;
        set_selection(step_selection(sel, count, nav, page));
    }
}

fn scroll_list(list: HWND, to: i32) {
    with_state(|s| {
        s.scroll = scroll_clamp(to, s.steps.len(), s.layout.row_h, s.view_h);
    });
    update_scrollbar(list);
    invalidate(list);
}

unsafe extern "system" fn list_proc(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            paint_list(h);
            LRESULT(0)
        }
        WM_GETDLGCODE => LRESULT(DLGC_WANTARROWS as isize),
        WM_KEYDOWN => {
            on_list_key(wp.0 as u16);
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            // SAFETY: đưa tiêu điểm vào danh sách (con của cửa sổ này).
            unsafe {
                let _ = SetFocus(Some(h));
            }
            let y = ((lp.0 >> 16) & 0xFFFF) as i16 as i32;
            let hit = with_state(|s| row_at(y, s.scroll, s.layout.row_h, s.steps.len())).flatten();
            if hit.is_some() {
                set_selection(hit);
            }
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            let delta = ((wp.0 >> 16) & 0xFFFF) as i16 as i32;
            if let Some((scroll, row_h)) = with_state(|s| (s.scroll, s.layout.row_h)) {
                scroll_list(h, scroll - delta * row_h / 120);
            }
            LRESULT(0)
        }
        WM_VSCROLL => {
            let Some((scroll, row_h, view)) = with_state(|s| (s.scroll, s.layout.row_h, s.view_h))
            else {
                return LRESULT(0);
            };
            let code = SCROLLBAR_COMMAND(lo_word(wp.0) as i32);
            let to = match code {
                SB_LINEUP => scroll - row_h,
                SB_LINEDOWN => scroll + row_h,
                SB_PAGEUP => scroll - view,
                SB_PAGEDOWN => scroll + view,
                SB_TOP => 0,
                SB_BOTTOM => i32::MAX / 2,
                SB_THUMBTRACK | SB_THUMBPOSITION => {
                    let mut si = SCROLLINFO {
                        cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
                        fMask: SIF_TRACKPOS,
                        ..Default::default()
                    };
                    // SAFETY: si là biến cục bộ; thanh cuộn của chính danh sách.
                    unsafe {
                        let _ = GetScrollInfo(h, SB_VERT, &mut si);
                    }
                    si.nTrackPos
                }
                _ => scroll,
            };
            scroll_list(h, to);
            LRESULT(0)
        }
        WM_SETFOCUS | WM_KILLFOCUS => {
            invalidate(h);
            LRESULT(0)
        }
        // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
        _ => unsafe { DefWindowProcW(h, msg, wp, lp) },
    }
}

unsafe extern "system" fn card_proc(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            paint_card(h);
            LRESULT(0)
        }
        // Nút của thẻ báo về cửa sổ chính.
        // SAFETY: chuyển nguyên tham số cho cửa sổ cha (cửa sổ Các bước của luồng này).
        WM_COMMAND => unsafe {
            match GetParent(h) {
                Ok(p) => SendMessageW(p, msg, Some(wp), Some(lp)),
                Err(_) => LRESULT(0),
            }
        },
        WM_NOTIFY => {
            // SAFETY: lp của WM_NOTIFY trỏ tới NMHDR hợp lệ suốt lời gọi.
            let hdr = unsafe { (lp.0 as *const NMHDR).as_ref() };
            if let Some(x) = hdr.filter(|x| x.code == NM_CUSTOMDRAW) {
                let kind = match x.idFrom as i32 {
                    ID_CARD_CANCEL => Some(ButtonKind::Normal),
                    ID_CARD_YES => Some(ButtonKind::Danger),
                    _ => None,
                };
                // SAFETY: mã NM_CUSTOMDRAW từ nút nên lp trỏ tới NMCUSTOMDRAW.
                if let (Some(kind), Some(cd)) = (kind, unsafe { theme::nmcd_of(lp) }) {
                    let (dpi, bold) = with_state(|s| (s.dpi, s.fonts.0.bold)).unwrap_or((96, None));
                    return theme::custom_draw_button(cd, kind, SURFACE, dpi, bold);
                }
            }
            // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
            unsafe { DefWindowProcW(h, msg, wp, lp) }
        }
        // SAFETY: chuyển nguyên tham số cho xử lý mặc định.
        _ => unsafe { DefWindowProcW(h, msg, wp, lp) },
    }
}

// ---------------------------------------------------------------------------
// Cho kiểm thử
// ---------------------------------------------------------------------------

#[cfg(test)]
fn hwnd() -> Option<HWND> {
    current()
}

/// Chữ từng dòng danh sách như khi vẽ: ("Bước k", dòng đầu ghi chú).
#[cfg(test)]
fn row_texts() -> Vec<(String, String)> {
    let lang = lang::current();
    with_state(|s| {
        s.steps
            .iter()
            .enumerate()
            .map(|(i, x)| (step_label(lang, i + 1), first_line(&x.caption).to_string()))
            .collect()
    })
    .unwrap_or_default()
}

#[cfg(test)]
fn edit_hwnd() -> HWND {
    with_state(|s| s.ctl.edit).unwrap_or_default()
}

#[cfg(test)]
fn button_hwnd(b: Btn) -> HWND {
    with_state(|s| s.ctl.buttons[b.index()]).unwrap_or_default()
}

#[cfg(test)]
fn card_button_hwnds() -> [HWND; 2] {
    with_state(|s| s.ctl.card_btns).unwrap_or_default()
}

#[cfg(test)]
fn confirm_open() -> bool {
    with_state(|s| s.confirm.is_open()).unwrap_or(false)
}

#[cfg(test)]
fn select(i: usize) {
    set_selection(Some(i));
}

#[cfg(test)]
fn selected() -> Option<usize> {
    with_state(|s| s.sel).flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::winutil::scale;
    use darkshot_core::i18n::{t, Lang, Msg};

    const DPIS: [u32; 4] = [96, 120, 144, 192];

    fn inside(outer: Rect, r: Rect) -> bool {
        r.x >= outer.x
            && r.y >= outer.y
            && r.right() <= outer.right()
            && r.bottom() <= outer.bottom()
    }

    fn overlap(a: Rect, b: Rect) -> bool {
        a.w > 0
            && a.h > 0
            && b.w > 0
            && b.h > 0
            && a.x < b.right()
            && b.x < a.right()
            && a.y < b.bottom()
            && b.y < a.bottom()
    }

    fn label_of(b: Btn) -> Msg {
        match b {
            Btn::Up => Msg::StepsUp,
            Btn::Down => Msg::StepsDown,
            Btn::Delete => Msg::StepsDelete,
            Btn::Export => Msg::StepsExport,
            Btn::End => Msg::StepsEndSession,
            Btn::Close => Msg::StepsClose,
        }
    }

    /// Đo chữ thật bằng phông hệ thống ở `dpi` (nút Xuất… đậm như khi vẽ).
    fn measured(lang: Lang, dpi: u32) -> Measure {
        use crate::history_paint::{measure, with_screen_dc};
        use windows::Win32::Graphics::Gdi::{DeleteObject, HGDIOBJ};
        let normal = crate::winutil::system_font(dpi);
        let bold = crate::winutil::system_font_styled(dpi, 100, true);
        let m = with_screen_dc(|dc| {
            let mut m = Measure {
                text_h: measure(dc, normal, "Ag").1,
                label_w: measure(dc, normal, t(lang, Msg::StepsCaption)).0,
                btn_text: [0; 6],
            };
            for (k, b) in BUTTONS.iter().enumerate() {
                let f = if *b == Btn::Export { bold } else { normal };
                m.btn_text[k] = measure(dc, f, t(lang, label_of(*b))).0;
            }
            m
        });
        for f in [normal, bold].into_iter().flatten() {
            // SAFETY: phông vừa tạo trong kiểm thử, không còn chọn vào DC nào.
            unsafe {
                let _ = DeleteObject(HGDIOBJ(f.0));
            }
        }
        m
    }

    /// Bố cục hợp lệ: mọi phần trong client, không chồng nhau, nút đủ chỗ cho chữ.
    fn check(l: &Layout, cw: i32, ch: i32, m: &Measure, what: &str) {
        let s = |v| scale(v, l.dpi);
        let client = Rect {
            x: 0,
            y: 0,
            w: cw,
            h: ch,
        };
        let parts = [
            ("list", l.list),
            ("preview", l.preview),
            ("label", l.label),
            ("field", l.field),
        ];
        for (name, r) in parts {
            assert!(inside(client, r), "{what}: {name} {r:?} ngoài client");
            assert!(r.w > 0 && r.h > 0, "{what}: {name} rỗng {r:?}");
        }
        assert!(inside(l.field, l.edit), "{what}: edit ngoài khung");
        assert!(
            l.edit.w > 0 && l.edit.h >= m.text_h * 3,
            "{what}: ô ghi chú quá thấp"
        );
        assert!(
            l.label.w >= m.label_w && l.label.h >= m.text_h,
            "{what}: nhãn bị cắt"
        );
        assert!(
            l.preview.h >= s(80),
            "{what}: xem trước quá thấp {:?}",
            l.preview
        );
        assert!(inside(client, l.body));
        for r in [l.list, l.preview, l.label, l.field] {
            assert!(inside(l.body, r), "{what}: {r:?} ngoài thân");
        }
        // Cột trái và cột phải không chồng; ba phần cột phải xếp dọc.
        for r in [l.preview, l.label, l.field] {
            assert!(!overlap(l.list, r), "{what}: danh sách chồng {r:?}");
        }
        assert!(l.preview.bottom() <= l.label.y, "{what}");
        assert!(l.label.bottom() <= l.field.y, "{what}");
        for (k, b) in l.buttons.iter().enumerate() {
            assert!(inside(client, *b), "{what}: nút {k} {b:?} ngoài client");
            assert!(
                b.w >= m.btn_text[k] + 2 * s(BTN_PAD),
                "{what}: nút {k} cắt chữ"
            );
            assert!(b.h >= m.text_h, "{what}: nút {k} thấp");
            assert!(!overlap(*b, l.body), "{what}: nút {k} chồng thân");
            for (j, o) in l.buttons.iter().enumerate() {
                if j != k {
                    assert!(!overlap(*b, *o), "{what}: nút {k} chồng nút {j}");
                }
            }
        }
        // Lên, Xuống, Xóa bước bên trái; Xuất…, Kết thúc phiên, Đóng bên phải, Đóng sát mép phải.
        assert!(l.buttons[0].x < l.buttons[1].x && l.buttons[1].x < l.buttons[2].x);
        assert!(l.buttons[3].x < l.buttons[4].x && l.buttons[4].x < l.buttons[5].x);
        assert_eq!(l.buttons[5].right(), cw - s(16), "{what}");
        assert!(l.row_h >= 2 * m.text_h, "{what}: dòng danh sách thấp");
        assert!(l.thumb.0 > 0 && l.thumb.1 > 0 && l.thumb.1 <= l.row_h);
    }

    #[test]
    fn layout_fits_real_text_in_both_languages_at_every_dpi() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        for dpi in DPIS {
            for lang in [Lang::Vi, Lang::En] {
                let m = measured(lang, dpi);
                assert!(m.text_h > 0 && m.btn_text.iter().all(|w| *w > 0));
                for (w, h) in [(MIN_W, MIN_H), (DEFAULT_W, DEFAULT_H), (1400, 900)] {
                    let (cw, ch) = (scale(w, dpi), scale(h, dpi));
                    let l = layout(cw, ch, dpi, &m);
                    check(&l, cw, ch, &m, &format!("{lang:?} dpi {dpi} {w}x{h}"));
                }
            }
        }
    }

    /// Chữ rất dài (phông lớn): hai nhóm nút xuống hai hàng, vẫn không chồng, không cắt.
    #[test]
    fn long_labels_wrap_buttons_onto_two_rows() {
        for dpi in DPIS {
            let k = |v: i32| scale(v, dpi);
            let m = Measure {
                text_h: k(20),
                label_w: k(60),
                btn_text: [k(60), k(80), k(90), k(80), k(130), k(60)],
            };
            let (cw, ch) = (k(MIN_W), k(MIN_H));
            let l = layout(cw, ch, dpi, &m);
            check(&l, cw, ch, &m, &format!("dpi {dpi}"));
            assert!(
                l.buttons[0].y < l.buttons[5].y,
                "dpi {dpi}: phải xuống hai hàng"
            );
            // Chữ vừa: một hàng.
            let small = Measure {
                text_h: k(16),
                label_w: k(40),
                btn_text: [k(20), k(40), k(50), k(40), k(80), k(30)],
            };
            let l = layout(cw, ch, dpi, &small);
            check(&l, cw, ch, &small, &format!("dpi {dpi} nhỏ"));
            assert_eq!(l.buttons[0].y, l.buttons[5].y, "dpi {dpi}: một hàng");
        }
    }

    /// Cỡ nhỏ hơn tối thiểu (không xảy ra nhờ WM_GETMINMAXINFO): không có cỡ âm.
    #[test]
    fn tiny_client_has_no_negative_sizes() {
        let m = Measure {
            text_h: 16,
            label_w: 40,
            btn_text: [20; 6],
        };
        let l = layout(100, 80, 96, &m);
        for r in [l.body, l.list, l.preview, l.label, l.field, l.edit] {
            assert!(r.w >= 0 && r.h >= 0, "{r:?}");
        }
    }

    #[test]
    fn card_is_centered_and_buttons_fit() {
        for dpi in DPIS {
            let k = |v: i32| scale(v, dpi);
            let body = Rect {
                x: k(16),
                y: k(16),
                w: k(608),
                h: k(360),
            };
            let c = card_rects(body, dpi, k(22), k(40), [k(80), k(90)]);
            assert!(inside(body, c.card), "dpi {dpi}: {c:?}");
            for r in [c.title, c.text, c.cancel, c.yes] {
                assert!(inside(c.card, r), "dpi {dpi}: {r:?}");
            }
            assert!(c.title.h >= k(22) && c.text.h >= k(40));
            assert!(c.cancel.w >= k(80) && c.yes.w >= k(90));
            assert!(c.cancel.right() <= c.yes.x, "Hủy bên trái Kết thúc");
            assert!(c.title.bottom() <= c.text.y && c.text.bottom() <= c.cancel.y);
            let mid = |r: Rect| (2 * r.x + r.w, 2 * r.y + r.h);
            let (a, b) = (mid(c.card), mid(body));
            assert!(
                (a.0 - b.0).abs() <= 2 && (a.1 - b.1).abs() <= 2,
                "dpi {dpi}"
            );
        }
    }

    #[test]
    fn button_states_follow_selection() {
        // [Lên, Xuống, Xóa bước, Xuất…, Kết thúc phiên, Đóng]
        assert_eq!(
            button_enabled(None, 0, false),
            [false, false, false, false, true, true]
        );
        assert_eq!(
            button_enabled(Some(0), 1, false),
            [false, false, true, true, true, true]
        );
        assert_eq!(
            button_enabled(Some(0), 3, false),
            [false, true, true, true, true, true]
        );
        assert_eq!(
            button_enabled(Some(1), 3, false),
            [true, true, true, true, true, true]
        );
        assert_eq!(
            button_enabled(Some(2), 3, false),
            [true, false, true, true, true, true]
        );
        // Chỉ số ngoài phạm vi: như chưa chọn.
        assert_eq!(
            button_enabled(Some(5), 3, false),
            [false, false, false, true, true, true]
        );
        // Đang xuất: không xuất lần nữa, không xóa ảnh, không kết thúc phiên.
        assert_eq!(
            button_enabled(Some(1), 3, true),
            [true, true, false, false, false, true]
        );
    }

    fn st(file: &str) -> Step {
        Step {
            file: file.to_string(),
            caption: String::new(),
        }
    }

    #[test]
    fn selection_kept_across_move_and_remove() {
        let three = [st("001.png"), st("002.png"), st("003.png")];
        // Lên/Xuống: giữ bước đang chọn theo tên tệp.
        let moved = [st("002.png"), st("001.png"), st("003.png")];
        assert_eq!(reselect(Some("002.png"), Some(1), &moved), Some(0));
        assert_eq!(reselect(Some("001.png"), Some(0), &moved), Some(1));
        // Xóa bước giữa: chọn bước kế tiếp (cùng chỉ số).
        let removed = [st("001.png"), st("003.png")];
        assert_eq!(reselect(Some("002.png"), Some(1), &removed), Some(1));
        // Xóa bước cuối: chọn bước trước.
        let last = [st("001.png"), st("002.png")];
        assert_eq!(reselect(Some("003.png"), Some(2), &last), Some(1));
        // Hết bước: không chọn.
        assert_eq!(reselect(Some("001.png"), Some(0), &[]), None);
        // Chưa chọn gì mà có bước: chọn bước đầu.
        assert_eq!(reselect(None, None, &three), Some(0));
        // Bước mới thêm không đổi lựa chọn.
        let four = [st("001.png"), st("002.png"), st("003.png"), st("004.png")];
        assert_eq!(reselect(Some("002.png"), Some(1), &four), Some(1));
    }

    #[test]
    fn first_line_of_caption() {
        assert_eq!(first_line(""), "");
        assert_eq!(first_line("Bấm Lưu"), "Bấm Lưu");
        assert_eq!(first_line("  một\r\nhai"), "một");
        assert_eq!(first_line("\n\n  ba  \nbốn"), "ba");
        assert_eq!(first_line("x\ry"), "x");
    }

    #[test]
    fn line_breaks_round_trip_between_edit_and_store() {
        assert_eq!(to_crlf("a\nb\r\nc\rd"), "a\r\nb\r\nc\r\nd");
        assert_eq!(from_crlf("a\r\nb\r\n"), "a\nb\n");
        assert_eq!(from_crlf("x\ry"), "x\ny");
        let s = "dòng một\nhai\n\nba";
        assert_eq!(from_crlf(&to_crlf(s)), s);
    }

    #[test]
    fn scrolling_math() {
        // 10 dòng cao 50, khung 120.
        assert_eq!(scroll_clamp(-5, 10, 50, 120), 0);
        assert_eq!(scroll_clamp(1000, 10, 50, 120), 380);
        assert_eq!(scroll_clamp(30, 2, 50, 120), 0);
        assert_eq!(ensure_visible(0, 100, 50, 120), 0);
        assert_eq!(ensure_visible(5, 0, 50, 120), 180);
        assert_eq!(ensure_visible(2, 50, 50, 120), 50);
        assert_eq!(row_at(10, 0, 50, 10), Some(0));
        assert_eq!(row_at(60, 50, 50, 10), Some(2));
        assert_eq!(row_at(-1, 0, 50, 10), None);
        assert_eq!(row_at(120, 0, 50, 2), None);
    }

    #[test]
    fn arrow_keys_map_to_navigation() {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            VK_DELETE, VK_DOWN, VK_END, VK_HOME, VK_NEXT, VK_PRIOR, VK_UP,
        };
        assert_eq!(nav_of(VK_UP.0), Some(Nav::Up));
        assert_eq!(nav_of(VK_DOWN.0), Some(Nav::Down));
        assert_eq!(nav_of(VK_HOME.0), Some(Nav::Home));
        assert_eq!(nav_of(VK_END.0), Some(Nav::End));
        assert_eq!(nav_of(VK_PRIOR.0), Some(Nav::PageUp));
        assert_eq!(nav_of(VK_NEXT.0), Some(Nav::PageDown));
        assert_eq!(nav_of(VK_DELETE.0), None);
    }

    // ---- Xuất qua phiên của luồng (không cửa sổ, không hộp chọn thư mục, không mở Explorer) ----

    use darkshot_core::actions::NoticeKind;
    use std::cell::RefCell;
    use std::path::{Path, PathBuf};

    thread_local! {
        static NOTICES: RefCell<Vec<(NoticeKind, String, String)>> = const { RefCell::new(Vec::new()) };
    }

    fn record(kind: NoticeKind, title: &str, body: &str) {
        NOTICES.with(|n| n.borrow_mut().push((kind, title.into(), body.into())));
    }

    fn take_notices() -> Vec<(NoticeKind, String, String)> {
        NOTICES.with(|n| std::mem::take(&mut *n.borrow_mut()))
    }

    fn tmp(name: &str) -> PathBuf {
        let p =
            std::env::temp_dir().join(format!("darkshot_steps_win_{}_{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn img(v: u8) -> darkshot_core::image::Image {
        darkshot_core::image::Image {
            width: 4,
            height: 3,
            bgra: vec![v; 4 * 3 * 4],
        }
    }

    /// Phiên tổng hợp 3 bước trong thư mục tạm `base\current` (phiên của luồng này).
    pub(super) fn session_of_three(base: &Path) {
        crate::steps_store::use_root_for_test(base.join("current"));
        assert!(crate::steps_store::start());
        for v in [10, 20, 30] {
            crate::steps_store::add(img(v)).unwrap();
        }
        crate::steps_store::flush(std::time::Duration::from_secs(20));
        assert_eq!(crate::steps_store::count(), 3);
    }

    fn export_dirs(parent: &Path) -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = std::fs::read_dir(parent)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| {
                p.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("darkshot-steps-"))
            })
            .collect();
        v.sort();
        v
    }

    #[test]
    fn export_marks_session_exported_and_notifies() {
        let base = tmp("export");
        session_of_three(&base);
        set_notifier(record);
        crate::steps_store::set_caption(1, "Bấm Lưu");
        assert!(crate::steps_store::dirty());
        let lang = crate::lang::current();
        let parent = base.join("out");
        std::fs::create_dir_all(&parent).unwrap();

        assert!(start_export(parent.clone()));
        // Kết thúc (thanh phiên) trong lúc đang xuất: không kết thúc, kể cả khi không cần hỏi.
        crate::steps_store::mark_exported();
        open_confirm_end();
        assert!(crate::steps_store::active(), "đang xuất: phiên còn");
        assert_eq!(crate::steps_store::count(), 3);
        wait_export();
        on_exported();
        assert!(
            !crate::steps_store::dirty(),
            "đã xuất: không cần hỏi khi kết thúc"
        );
        let dirs = export_dirs(&parent);
        assert_eq!(dirs.len(), 1);
        let mut files: Vec<String> = std::fs::read_dir(&dirs[0])
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        files.sort();
        assert_eq!(files, ["01.png", "02.png", "03.png", "steps.md"]);
        let md = std::fs::read_to_string(dirs[0].join("steps.md")).unwrap();
        assert!(md.contains("Bấm Lưu"));
        // Ảnh theo thứ tự hiển thị.
        for k in 0..3 {
            let src = std::fs::read(crate::steps_store::image_path(k).unwrap()).unwrap();
            let out = std::fs::read(dirs[0].join(format!("{:02}.png", k + 1))).unwrap();
            assert_eq!(src, out, "bước {}", k + 1);
        }
        let n = take_notices();
        assert_eq!(n.len(), 1, "{n:?}");
        assert_eq!(n[0].0, NoticeKind::Success);
        assert_eq!(n[0].1, darkshot_core::i18n::steps_exported(lang, 3));

        // Đổi ghi chú trong lúc xuất: phiên vẫn coi là chưa xuất.
        assert!(start_export(parent.clone()));
        crate::steps_store::set_caption(0, "đổi trong lúc xuất");
        wait_export();
        on_exported();
        assert!(crate::steps_store::dirty());
        assert_eq!(export_dirs(&parent).len(), 2, "lượt hai vào thư mục mới");
        assert_eq!(take_notices()[0].0, NoticeKind::Success);

        // Thư mục cha không còn: lỗi, phiên giữ nguyên, không để lại gì.
        crate::steps_store::mark_exported();
        crate::steps_store::set_caption(0, "lại đổi");
        let gone = base.join("gone");
        assert!(start_export(gone.clone()));
        wait_export();
        on_exported();
        assert!(crate::steps_store::dirty());
        assert!(!gone.exists());
        let n = take_notices();
        assert_eq!(n.len(), 1);
        assert_eq!(n[0].0, NoticeKind::Error);
        assert_eq!(n[0].1, t(lang, Msg::StepsExportFailedTitle));
        assert!(!n[0].2.is_empty());
        assert_eq!(crate::steps_store::count(), 3);

        assert!(crate::steps_store::end());
        let _ = std::fs::remove_dir_all(&base);
    }

    /// Không có phiên (hay 0 bước): không xuất.
    #[test]
    fn export_without_steps_does_nothing() {
        let base = tmp("noexport");
        crate::steps_store::use_root_for_test(base.join("current"));
        assert!(!start_export(base.clone()));
        assert!(crate::steps_store::start());
        assert!(!start_export(base.clone()));
        assert!(export_dirs(&base).is_empty());
        assert!(crate::steps_store::end());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn message_ids_are_unique() {
        assert_eq!(WM_STEPS_WORK, WM_APP + 15);
        assert_eq!(WM_STEPS_EXPORTED, WM_APP + 16);
        assert_ne!(WM_STEPS_WORK, crate::steps_store::WM_STEPS_SAVED);
        assert_ne!(WM_STEPS_WORK, crate::ocr_window::WM_OCR_RESULT);
    }
}

/// Chạy tay một lượt (mở cửa sổ thật, không lớp phủ, không dời chuột hay phím thật):
/// `cargo test -p darkshot-windows steps_window::live -- --ignored --test-threads=1`.
#[cfg(test)]
mod live {
    use super::*;
    use darkshot_core::i18n::{step_label, t, Msg};
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::System::Threading::{
        GetCurrentProcess, GetGuiResources, GR_GDIOBJECTS, GR_USEROBJECTS,
    };
    use windows::Win32::UI::HiDpi::{
        SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::{IsWindowEnabled, VK_ESCAPE};
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, IsWindow, PeekMessageW, SetWindowTextW, TranslateMessage, MSG, PM_REMOVE,
        WM_KEYDOWN,
    };

    fn pump() {
        let mut msg = MSG::default();
        // SAFETY: vòng bơm thông điệp chuẩn của luồng kiểm thử (qua `pre_translate` như ứng dụng).
        unsafe {
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if pre_translate(&msg) {
                    continue;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    fn gui(kind: windows::Win32::System::Threading::GET_GUI_RESOURCES_FLAGS) -> u32 {
        // SAFETY: chỉ đọc bộ đếm của tiến trình hiện tại.
        unsafe { GetGuiResources(GetCurrentProcess(), kind) }
    }

    fn text_of(h: windows::Win32::Foundation::HWND) -> String {
        String::from_utf16_lossy(&crate::theme::window_text(h))
    }

    fn enabled(b: Btn) -> bool {
        // SAFETY: chỉ hỏi trạng thái của nút thuộc cửa sổ của luồng này.
        unsafe { IsWindowEnabled(button_hwnd(b)) }.as_bool()
    }

    #[test]
    #[ignore]
    fn review_window_shows_steps_and_closes_without_leaks() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        // SAFETY: đặt chế độ DPI cho luồng kiểm thử (như manifest của ứng dụng).
        unsafe {
            SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
        let base = std::env::temp_dir().join(format!("darkshot_steps_live_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        super::tests::session_of_three(&base);
        crate::steps_store::on_change(on_store_changed);
        crate::steps_bar::set_on_confirm_end(open_confirm_end);
        assert!(crate::steps_store::set_caption(0, "Mở menu\nrồi chọn"));
        let lang = crate::lang::current();
        let (mut gdi0, mut user0) = (0, 0);
        for round in 0..11 {
            if round == 1 {
                gdi0 = gui(GR_GDIOBJECTS);
                user0 = gui(GR_USEROBJECTS);
            }
            open();
            pump();
            let h = hwnd().expect("cửa sổ Xem lại đã mở");
            // Mở lại: vẫn một cửa sổ.
            open();
            pump();
            assert_eq!(hwnd(), Some(h));
            assert_eq!(text_of(h), t(lang, Msg::StepsWindowTitle));
            let rows = row_texts();
            assert_eq!(
                rows,
                [
                    (step_label(lang, 1), "Mở menu".to_string()),
                    (step_label(lang, 2), String::new()),
                    (step_label(lang, 3), String::new()),
                ]
            );
            assert_eq!(selected(), Some(0));
            assert_eq!(text_of(edit_hwnd()), "Mở menu\r\nrồi chọn");
            assert!(!enabled(Btn::Up) && enabled(Btn::Down) && enabled(Btn::Delete));
            assert!(enabled(Btn::Export) && enabled(Btn::End) && enabled(Btn::Close));
            // Gõ ghi chú (bằng thông điệp) rồi đổi bước: ghi chú được lưu.
            select(2);
            pump();
            assert_eq!(text_of(edit_hwnd()), "");
            assert!(enabled(Btn::Up) && !enabled(Btn::Down));
            // SAFETY: đặt chữ cho ô ghi chú của cửa sổ này; chuỗi sống suốt lời gọi.
            unsafe {
                let _ = SetWindowTextW(edit_hwnd(), &windows::core::HSTRING::from("ba"));
            }
            select(0);
            pump();
            assert_eq!(crate::steps_store::steps()[2].caption, "ba");
            assert_eq!(row_texts()[2].1, "ba");
            // Đổi ngôn ngữ: chữ đổi theo.
            let other = match lang {
                darkshot_core::i18n::Lang::Vi => darkshot_core::i18n::Lang::En,
                darkshot_core::i18n::Lang::En => darkshot_core::i18n::Lang::Vi,
            };
            crate::lang::set(other);
            relabel();
            pump();
            assert_eq!(text_of(h), t(other, Msg::StepsWindowTitle));
            assert_eq!(
                text_of(button_hwnd(Btn::Export)),
                t(other, Msg::StepsExport)
            );
            assert_eq!(row_texts()[0].0, step_label(other, 1));
            crate::lang::set(lang);
            relabel();
            // Kết thúc khi chưa xuất: thẻ xác nhận; Esc (thông điệp qua `pre_translate`) = Hủy.
            assert!(crate::steps_store::dirty());
            open_confirm_end();
            pump();
            assert!(confirm_open());
            assert!(!enabled(Btn::Close), "nút phía sau bị vô hiệu khi thẻ mở");
            let esc = MSG {
                hwnd: card_button_hwnds()[0],
                message: WM_KEYDOWN,
                wParam: WPARAM(VK_ESCAPE.0 as usize),
                lParam: LPARAM(0),
                ..Default::default()
            };
            assert!(pre_translate(&esc));
            pump();
            assert!(!confirm_open());
            assert!(crate::steps_store::active(), "Hủy: phiên còn");
            assert!(enabled(Btn::Close));
            // Ghi chú đang sửa được lưu khi đóng.
            // SAFETY: như trên.
            unsafe {
                let _ = SetWindowTextW(edit_hwnd(), &windows::core::HSTRING::from("một"));
            }
            close();
            pump();
            assert!(hwnd().is_none());
            // SAFETY: chỉ hỏi cửa sổ đã hủy có còn không.
            unsafe { assert!(!IsWindow(Some(h)).as_bool()) };
            assert_eq!(crate::steps_store::steps()[0].caption, "một");
            assert!(crate::steps_store::set_caption(0, "Mở menu\nrồi chọn"));
            assert!(crate::steps_store::set_caption(2, ""));
        }
        let (gdi1, user1) = (gui(GR_GDIOBJECTS), gui(GR_USEROBJECTS));
        eprintln!("GDI {gdi0} -> {gdi1}, USER {user0} -> {user1}");
        assert!(gdi1 <= gdi0 + 2, "GDI {gdi0} -> {gdi1}");
        assert!(user1 <= user0 + 2, "USER {user0} -> {user1}");
        // Ô ghi chú có chữ chưa lưu (thanh phiên không lấy tiêu điểm nên không có EN_KILLFOCUS) và
        // phiên đang coi là đã xuất: Kết thúc từ thanh phiên lưu ghi chú trước, rồi mới hỏi.
        open();
        pump();
        crate::steps_store::mark_exported();
        // SAFETY: đặt chữ cho ô ghi chú của cửa sổ này; chuỗi sống suốt lời gọi.
        unsafe {
            let _ = SetWindowTextW(edit_hwnd(), &windows::core::HSTRING::from("chưa lưu"));
        }
        crate::steps_bar::finish();
        pump();
        assert_eq!(crate::steps_store::steps()[0].caption, "chưa lưu");
        assert!(crate::steps_store::dirty());
        assert!(confirm_open(), "ghi chú vừa lưu chưa xuất: phải hỏi");
        assert!(crate::steps_store::active());
        let esc = MSG {
            hwnd: card_button_hwnds()[0],
            message: WM_KEYDOWN,
            wParam: WPARAM(VK_ESCAPE.0 as usize),
            lParam: LPARAM(0),
            ..Default::default()
        };
        assert!(pre_translate(&esc));
        pump();
        assert!(!confirm_open());
        // Kết thúc phiên đã xuất (không cần hỏi): cửa sổ đang mở thì đóng theo.
        crate::steps_store::mark_exported();
        open_confirm_end();
        pump();
        assert!(!crate::steps_store::active());
        assert!(hwnd().is_none());
        let _ = std::fs::remove_dir_all(&base);
    }
}
