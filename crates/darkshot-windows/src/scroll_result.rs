//! Cửa sổ "Ảnh chụp cuộn" (kết quả của phiên chụp cuộn). Khung sườn cho kiểm thử; cài đặt ở commit sau.

use darkshot_core::actions::{Action, NoticeKind};
use darkshot_core::geometry::Rect;
use darkshot_core::image::Image;
use std::sync::atomic::AtomicBool;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{MSG, WM_APP};

/// Ảnh thu nhỏ dựng xong trên luồng nền (không mang dữ liệu; kết quả nằm trong sổ của cửa sổ).
pub const WM_SCROLL_PREPARED: u32 = WM_APP + 17;
const MAX_WINDOWS: usize = 4;
const MIN_W: i32 = 480;
const MIN_H: i32 = 400;
const PREBUILD_MIN_H: u32 = 4096;
const ID_SAVE: i32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OpenPolicy {
    Open,
    CloseOldestUsed(usize),
    OpenAndWarn,
}

fn open_policy(_used: &[bool]) -> OpenPolicy {
    OpenPolicy::Open
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Layout {
    strip: Rect,
    view: Rect,
    scrollbar: Rect,
    info: Rect,
    buttons: [Rect; 5],
}

const ZERO: Rect = Rect {
    x: 0,
    y: 0,
    w: 0,
    h: 0,
};

#[allow(clippy::too_many_arguments)]
fn layout(
    _cw: i32,
    _ch: i32,
    _dpi: u32,
    _btn_text: [i32; 5],
    _warn_text: Option<i32>,
    _line_h: i32,
    _sb_w: i32,
) -> Layout {
    Layout {
        strip: ZERO,
        view: ZERO,
        scrollbar: ZERO,
        info: ZERO,
        buttons: [ZERO; 5],
    }
}

fn fit_width(_img_w: u32, _img_h: u32, _view_w: i32) -> (i32, i32) {
    (0, 0)
}

fn max_scroll(_disp_h: i32, _view_h: i32) -> i32 {
    0
}

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

fn scrolled(_pos: i32, _cmd: ScrollCmd, _disp_h: i32, _view_h: i32, _line: i32) -> i32 {
    0
}

fn rescaled_scroll(_pos: i32, _old_h: i32, _new_h: i32) -> i32 {
    0
}

fn src_band(_scroll: i32, _vis: i32, _disp_h: i32, _src_h: i32) -> (i32, i32) {
    (0, 0)
}

fn default_client(
    _work: Rect,
    _img_w: u32,
    _img_h: u32,
    _dpi: u32,
    _chrome_w: i32,
    _chrome_h: i32,
) -> (i32, i32) {
    (0, 0)
}

fn needs_prebuild(_img_h: u32) -> bool {
    false
}

fn prebuild_width(_img_w: u32, _disp_w: i32) -> u32 {
    0
}

fn needs_rebuild(_built_w: u32, _img_w: u32, _disp_w: i32) -> bool {
    false
}

struct Prepared {
    w: u32,
    h: u32,
    px: Vec<u32>,
}

fn downscale(_img: &Image, _tw: u32, _cancel: &AtomicBool) -> Option<Prepared> {
    None
}

type Sink = fn(Action, Image) -> bool;
type Notify = fn(NoticeKind, &str, &str);

pub fn set_action_sink(_f: Sink) {}
pub fn set_notifier(_f: Notify) {}
pub fn open(_image: Image, _uncertain: u32) {}
pub fn close_all() {}
pub fn relabel() {}
pub fn pre_translate(_msg: &MSG) -> bool {
    false
}

#[cfg(test)]
fn test_windows() -> Vec<HWND> {
    Vec::new()
}
#[cfg(test)]
fn scroll_of(_h: HWND) -> (i32, i32) {
    (0, 0)
}
#[cfg(test)]
fn preparing(_h: HWND) -> bool {
    true
}
#[cfg(test)]
fn strip_of(_h: HWND) -> Rect {
    ZERO
}
#[cfg(test)]
fn used_of(_h: HWND) -> bool {
    false
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
        assert!(downscale(&solid(900, 12000, [5, 5, 5, 255]), 450, &AtomicBool::new(true)).is_none());
        // Ảnh hỏng: không panic.
        let bad = Image {
            width: 10,
            height: 10,
            bgra: vec![0; 7],
        };
        assert!(downscale(&bad, 5, &no).is_none());
    }

    #[test]
    fn at_most_four_windows_never_drops_unused() {
        assert_eq!(open_policy(&[]), OpenPolicy::Open);
        assert_eq!(open_policy(&[false, false, false]), OpenPolicy::Open);
        assert_eq!(open_policy(&[true, true, true]), OpenPolicy::Open);
        assert_eq!(
            open_policy(&[false, true, false, true]),
            OpenPolicy::CloseOldestUsed(1)
        );
        assert_eq!(
            open_policy(&[true, true, true, true]),
            OpenPolicy::CloseOldestUsed(0)
        );
        assert_eq!(open_policy(&[false; 4]), OpenPolicy::OpenAndWarn);
        assert_eq!(
            open_policy(&[false, false, false, false, true]),
            OpenPolicy::CloseOldestUsed(4)
        );
        assert_eq!(open_policy(&[false; 6]), OpenPolicy::OpenAndWarn);
    }

    // ---- Chạy tay (mở cửa sổ thật; không lớp phủ, không dời chuột/phím thật):
    // cargo test -p darkshot-windows scroll_result::tests::manual -- --ignored --test-threads=1

    thread_local! {
        static ACTIONS: RefCell<Vec<(Action, u32, u32)>> = const { RefCell::new(Vec::new()) };
        static NOTICES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    fn record_action(a: Action, img: Image) -> bool {
        ACTIONS.with(|v| v.borrow_mut().push((a, img.width, img.height)));
        true
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

    /// Tối đa 4: cái cũ nhất đã dùng bị đóng; cả 4 chưa dùng thì vẫn mở thêm và báo.
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
        // Lưu trên cửa sổ thứ hai: bản sao ảnh tới hàm hành động, cửa sổ không đóng.
        send(wins[1], WM_COMMAND, ID_SAVE as usize);
        assert_eq!(
            ACTIONS.with(|v| v.borrow().clone()),
            vec![(Action::Save, 300, 500)]
        );
        assert!(alive(wins[1]) && used_of(wins[1]));
        open(tall(300, 500), 0);
        let now = test_windows();
        assert_eq!(now.len(), 4);
        assert!(!alive(wins[1]));
        assert!(NOTICES.with(|v| v.borrow().is_empty()));
        // Cả 4 chưa dùng: mở cái thứ năm, không đóng gì, báo Info.
        open(tall(300, 500), 0);
        assert_eq!(test_windows().len(), 5);
        assert!(now.iter().all(|h| alive(*h)));
        let lang = crate::lang::current();
        assert_eq!(
            NOTICES.with(|v| v.borrow().clone()),
            vec![t(lang, Msg::ScrollManyOpenTitle).to_string()]
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
