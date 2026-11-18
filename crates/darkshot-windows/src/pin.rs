//! Ảnh ghim: cửa sổ nổi luôn trên cùng hiện một ảnh chụp, kéo để dời, cuộn để phóng/thu.
//! Mọi ảnh ghim sống trên luồng giao diện chính; sổ `PINS` chỉ giữ HWND.

// Task 69 nối luồng chụp, menu khay và dọn khi thoát; đến lúc đó phần lớn API chưa có người gọi.
#![cfg_attr(not(test), allow(dead_code))]

use darkshot_core::actions::Action;
use darkshot_core::geometry::{Point, Rect};
use darkshot_core::i18n::{t, Lang, Msg};
use darkshot_core::image::Image;
use darkshot_core::pin as core_pin;

/// Nơi đặt ảnh ghim mới.
pub enum PinPlace {
    /// Ảnh hiện 100% với góc trên trái của ẢNH tại điểm màn hình này (cửa sổ = ảnh nới một viền ra ngoài).
    At(Point),
    /// Căn giữa vùng làm việc của màn hình chứa con trỏ, mức `pin::fit_zoom`.
    CenterOnCursorMonitor,
}

#[derive(Debug)]
pub enum PinError {
    Limit,
    Create(String),
}

/// Tạo một ảnh ghim (sở hữu `image`). Gọi trên luồng giao diện chính.
pub fn open(image: Image, place: PinPlace) -> Result<(), PinError> {
    let _ = (image, place);
    todo!()
}

pub fn count() -> usize {
    todo!()
}

/// Hủy mọi ảnh ghim (đồng bộ). Gọi được nhiều lần.
pub fn close_all() {
    todo!()
}

/// Hàm `main` đăng ký để Lưu/Chép từ ảnh ghim đi đúng đường `run_on_image` (thông báo, Lịch sử).
pub fn set_action_sink(f: fn(Action, Image)) {
    let _ = f;
    todo!()
}

// ---------------------------------------------------------------------------------------------
// Hình học và quy tắc thuần (không cần cửa sổ).

/// Quy đổi DIP sang pixel ở `dpi` (làm tròn gần nhất).
fn dip(v: i32, dpi: u32) -> i32 {
    let _ = (v, dpi);
    todo!()
}

/// Bề dày viền: 1 DIP, tối thiểu 1 px.
fn border_px(dpi: u32) -> i32 {
    let _ = dpi;
    todo!()
}

/// Nét dấu ×: 1,5 DIP làm tròn, tối thiểu 1 px.
fn x_pen_px(dpi: u32) -> i32 {
    let _ = dpi;
    todo!()
}

/// Khung cửa sổ = khung ảnh nới `border` px mỗi phía (kẹp vào i32).
fn window_rect(image_rect: Rect, border: i32) -> Rect {
    let _ = (image_rect, border);
    todo!()
}

/// Khung ảnh trong khung cửa sổ (ngược của `window_rect`).
fn image_rect_of(window: Rect, border: i32) -> Rect {
    let _ = (window, border);
    todo!()
}

/// Nút × trong vùng client: ô 20 DIP, cách mép trên và phải 4 DIP.
fn close_button_rect(client: Rect, dpi: u32) -> Rect {
    let _ = (client, dpi);
    todo!()
}

/// Nút × chỉ hiện khi ảnh hiển thị ≥ 48 DIP mỗi chiều.
fn close_button_visible(display_w: i32, display_h: i32, dpi: u32) -> bool {
    let _ = (display_w, display_h, dpi);
    todo!()
}

/// Khung nhãn mức/độ mờ giữa đáy `area` (khung ảnh trong client), cách đáy 8 DIP, đệm 8×4 DIP.
/// `None` khi ảnh nhỏ hơn nhãn cộng 8 DIP.
fn badge_rect(area: Rect, text_size: (i32, i32), dpi: u32) -> Option<Rect> {
    let _ = (area, text_size, dpi);
    todo!()
}

/// Cách chép ảnh nguồn vào khung ảnh.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Blit {
    /// 100%: chép thẳng.
    Copy,
    /// Thu nhỏ: `HALFTONE`.
    Halftone,
    /// Phóng to: `COLORONCOLOR` (điểm ảnh sắc).
    ColorOnColor,
}

fn stretch_mode(zoom: u16) -> Blit {
    let _ = zoom;
    todo!()
}

/// Cộng `delta` của bánh xe vào phần dư `accum`: số bậc trọn `WHEEL_DELTA` và phần dư mới.
/// Đổi chiều thì bỏ phần dư cũ.
fn wheel_steps(accum: i32, delta: i32) -> (i32, i32) {
    let _ = (accum, delta);
    todo!()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Hit {
    Close,
    Image,
}

/// Điểm client `pt` rơi vào nút × (khi đang hiện, khung `close`) hay ảnh.
fn hit(pt: Point, close: Option<Rect>) -> Hit {
    let _ = (pt, close);
    todo!()
}

/// Mục Thu phóng của menu: bật khi cỡ hiển thị mỗi chiều ≤ `MAX_SIDE`.
fn menu_zoom_enabled(w: u32, h: u32, zoom: u16) -> bool {
    let _ = (w, h, zoom);
    todo!()
}

/// Mức và khung ảnh ban đầu của `At(p)`: 100%, hoặc bậc lớn nhất dưới 100% vừa `MAX_SIDE`.
fn initial_at(p: Point, w: u32, h: u32) -> (u16, Rect) {
    let _ = (p, w, h);
    todo!()
}

/// Điểm mở menu ngữ cảnh: tọa độ màn hình trong `lp`, hoặc `fallback` khi mở bằng bàn phím (`lp` = -1).
fn context_point(lp: isize, fallback: Point) -> Point {
    let _ = (lp, fallback);
    todo!()
}

/// HWND của mọi ảnh ghim còn sống (bản chép của sổ).
fn handles() -> Vec<windows::Win32::Foundation::HWND> {
    todo!()
}

/// Tiêu đề cửa sổ: "darkshot — Ảnh ghim W × H".
fn title(lang: Lang, w: u32, h: u32) -> String {
    let _ = (t(lang, Msg::PinWindowTitle), w, h);
    todo!()
}

/// Chữ của nhãn: "125%" hoặc "Độ mờ 60%".
fn badge_text(lang: Lang, zoom: Option<u16>, opacity: u8) -> String {
    let _ = (lang, zoom, opacity, core_pin::MAX_PINS);
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }
    fn p(x: i32, y: i32) -> Point {
        Point { x, y }
    }

    #[test]
    fn border_and_pen_follow_dpi() {
        assert_eq!(border_px(96), 1);
        assert_eq!(border_px(120), 1);
        assert_eq!(border_px(144), 2);
        assert_eq!(border_px(192), 2);
        assert_eq!(border_px(0), 1);
        assert_eq!(x_pen_px(96), 2);
        assert_eq!(x_pen_px(120), 2);
        assert_eq!(x_pen_px(144), 2);
        assert_eq!(x_pen_px(192), 3);
    }

    #[test]
    fn window_rect_inflates_by_border() {
        assert_eq!(window_rect(r(10, 20, 100, 50), 1), r(9, 19, 102, 52));
        assert_eq!(window_rect(r(10, 20, 100, 50), 2), r(8, 18, 104, 54));
        assert_eq!(
            window_rect(r(-1920, -5, 640, 480), 1),
            r(-1921, -6, 642, 482)
        );
        assert_eq!(image_rect_of(r(8, 18, 104, 54), 2), r(10, 20, 100, 50));
        // Không tràn: kẹp vào i32.
        let w = window_rect(r(i32::MIN, 0, i32::MAX, 5), 2);
        assert_eq!((w.x, w.w), (i32::MIN, i32::MAX));
    }

    #[test]
    fn close_button_rect_per_dpi() {
        let c = r(0, 0, 300, 200);
        assert_eq!(close_button_rect(c, 96), r(276, 4, 20, 20));
        assert_eq!(close_button_rect(c, 120), r(270, 5, 25, 25));
        assert_eq!(close_button_rect(c, 144), r(264, 6, 30, 30));
        assert_eq!(close_button_rect(c, 192), r(252, 8, 40, 40));
    }

    #[test]
    fn close_button_visible_needs_48_dip() {
        assert!(close_button_visible(48, 48, 96));
        assert!(!close_button_visible(47, 48, 96));
        assert!(!close_button_visible(48, 47, 96));
        assert!(close_button_visible(60, 60, 120));
        assert!(!close_button_visible(59, 60, 120));
        assert!(close_button_visible(72, 72, 144));
        assert!(!close_button_visible(72, 71, 144));
        assert!(close_button_visible(96, 96, 192));
        assert!(!close_button_visible(95, 96, 192));
    }

    #[test]
    fn badge_rect_per_dpi_and_hidden_when_small() {
        let a = r(0, 0, 400, 300);
        assert_eq!(badge_rect(a, (40, 16), 96), Some(r(172, 268, 56, 24)));
        assert_eq!(badge_rect(a, (40, 16), 120), Some(r(170, 264, 60, 26)));
        assert_eq!(badge_rect(a, (40, 16), 144), Some(r(168, 260, 64, 28)));
        assert_eq!(badge_rect(a, (40, 16), 192), Some(r(164, 252, 72, 32)));
        // Theo gốc của khung ảnh.
        assert_eq!(
            badge_rect(r(1, 1, 400, 300), (40, 16), 96),
            Some(r(173, 269, 56, 24))
        );
        // Ảnh nhỏ hơn nhãn + 8 DIP: ẩn.
        assert_eq!(badge_rect(r(0, 0, 63, 300), (40, 16), 96), None);
        assert!(badge_rect(r(0, 0, 64, 300), (40, 16), 96).is_some());
        assert_eq!(badge_rect(r(0, 0, 400, 31), (40, 16), 96), None);
        assert_eq!(
            badge_rect(r(0, 0, 400, 32), (40, 16), 96),
            Some(r(172, 0, 56, 24))
        );
    }

    #[test]
    fn stretch_mode_by_zoom() {
        assert_eq!(stretch_mode(100), Blit::Copy);
        assert_eq!(stretch_mode(90), Blit::Halftone);
        assert_eq!(stretch_mode(10), Blit::Halftone);
        assert_eq!(stretch_mode(110), Blit::ColorOnColor);
        assert_eq!(stretch_mode(400), Blit::ColorOnColor);
    }

    #[test]
    fn wheel_steps_accumulate() {
        assert_eq!(wheel_steps(0, 120), (1, 0));
        assert_eq!(wheel_steps(0, -120), (-1, 0));
        assert_eq!(wheel_steps(0, 360), (3, 0));
        // Bánh xe mịn: cộng dồn đến đủ một bậc.
        assert_eq!(wheel_steps(0, 40), (0, 40));
        assert_eq!(wheel_steps(80, 40), (1, 0));
        assert_eq!(wheel_steps(100, 240), (2, 100));
        assert_eq!(wheel_steps(-80, -40), (-1, 0));
        // Đổi chiều: bỏ phần dư cũ.
        assert_eq!(wheel_steps(60, -120), (-1, 0));
        assert_eq!(wheel_steps(-60, 30), (0, 30));
        // Không tràn.
        assert_eq!(wheel_steps(i32::MAX, i32::MAX).1.abs() < 120, true);
    }

    #[test]
    fn hit_close_or_image() {
        let c = Some(r(0, 0, 20, 20));
        assert_eq!(hit(p(10, 10), c), Hit::Close);
        assert_eq!(hit(p(0, 0), c), Hit::Close);
        assert_eq!(hit(p(20, 10), c), Hit::Image);
        assert_eq!(hit(p(10, 20), c), Hit::Image);
        assert_eq!(hit(p(10, 10), None), Hit::Image);
    }

    #[test]
    fn menu_zoom_limits() {
        assert!(menu_zoom_enabled(1000, 1000, 200));
        assert!(menu_zoom_enabled(8192, 100, 200));
        assert!(!menu_zoom_enabled(8193, 100, 200));
        assert!(!menu_zoom_enabled(100, 8193, 200));
        assert!(menu_zoom_enabled(100, 100, 400));
    }

    #[test]
    fn initial_at_keeps_100_or_fits_max_side() {
        assert_eq!(
            initial_at(p(-1920, 5), 640, 480),
            (100, r(-1920, 5, 640, 480))
        );
        assert_eq!(initial_at(p(0, 0), 20000, 100), (75, r(0, 0, 15000, 75)));
    }

    #[test]
    fn context_point_mouse_or_keyboard() {
        let fb = p(7, 9);
        assert_eq!(context_point(-1, fb), fb);
        let lp = (((50u16 as u32) << 16) | (-100i16 as u16 as u32)) as i32 as isize;
        assert_eq!(context_point(lp, fb), p(-100, 50));
        let lp = (((-300i16 as u16 as u32) << 16) | 200u32) as i32 as isize;
        assert_eq!(context_point(lp, fb), p(200, -300));
    }

    #[test]
    fn title_and_badge_text() {
        assert_eq!(
            title(Lang::En, 640, 480),
            "darkshot — Pinned image 640 × 480"
        );
        assert_eq!(title(Lang::Vi, 1, 2), "darkshot — Ảnh ghim 1 × 2");
        assert_eq!(badge_text(Lang::En, Some(125), 100), "125%");
        assert_eq!(badge_text(Lang::En, None, 60), "Opacity 60%");
        assert_eq!(badge_text(Lang::Vi, None, 60), "Độ mờ 60%");
    }

    #[test]
    fn dip_rounds() {
        assert_eq!(dip(20, 96), 20);
        assert_eq!(dip(20, 120), 25);
        assert_eq!(dip(4, 144), 6);
        assert_eq!(dip(48, 192), 96);
        assert_eq!(dip(1, 0), 1);
    }
}

/// Kiểm thử mở cửa sổ thật. Chạy tay từng cái:
/// `cargo test -p darkshot-windows pin::live -- --ignored --test-threads=1`.
#[cfg(test)]
mod live {
    use super::*;
    use windows::Win32::Foundation::{POINT, RECT};
    use windows::Win32::Graphics::Gdi::ClientToScreen;
    use windows::Win32::System::Threading::{
        GetCurrentProcess, GetGuiResources, GR_GDIOBJECTS, GR_USEROBJECTS,
    };
    use windows::Win32::UI::HiDpi::{
        SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetClientRect, GetWindowRect, IsWindow, PeekMessageW, SendMessageW,
        TranslateMessage, MSG, PM_REMOVE, WM_MOUSEWHEEL,
    };

    fn p(x: i32, y: i32) -> Point {
        Point { x, y }
    }

    fn per_monitor() {
        // SAFETY: như ứng dụng thật (manifest PerMonitorV2), áp cho luồng kiểm thử.
        unsafe {
            SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
    }

    fn pump() {
        let mut msg = MSG::default();
        // SAFETY: bơm thông điệp của luồng kiểm thử.
        unsafe {
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    fn image(w: u32, h: u32) -> Image {
        let mut bgra = Vec::with_capacity((w * h * 4) as usize);
        for i in 0..w * h {
            bgra.extend_from_slice(&[(i % 251) as u8, (i % 13) as u8 * 19, 0x80, 0xFF]);
        }
        Image {
            width: w,
            height: h,
            bgra,
        }
    }

    fn counts() -> (u32, u32) {
        // SAFETY: chỉ đọc bộ đếm của tiến trình hiện tại.
        unsafe {
            (
                GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS),
                GetGuiResources(GetCurrentProcess(), GR_USEROBJECTS),
            )
        }
    }

    fn rect_of(h: windows::Win32::Foundation::HWND) -> RECT {
        let mut rc = RECT::default();
        // SAFETY: h là cửa sổ còn sống của luồng này.
        unsafe {
            let _ = GetWindowRect(h, &mut rc);
        }
        rc
    }

    #[test]
    #[ignore]
    fn pin_round_trip_without_gdi_leaks() {
        per_monitor();
        // Vòng khởi động: lớp cửa sổ, phông, chổi dùng chung được tạo lần đầu.
        open(image(64, 48), PinPlace::At(p(200, 200))).unwrap();
        pump();
        close_all();
        pump();
        let (gdi0, user0) = counts();
        for i in 0..50 {
            let place = if i % 2 == 0 {
                PinPlace::At(p(200 + i, 200))
            } else {
                PinPlace::CenterOnCursorMonitor
            };
            open(image(320, 240), place).unwrap();
            pump();
            close_all();
            pump();
        }
        let (gdi1, user1) = counts();
        assert!(gdi1 <= gdi0 + 2, "GDI {gdi0} -> {gdi1}");
        assert!(user1 <= user0 + 2, "USER {user0} -> {user1}");
        assert_eq!(count(), 0);
    }

    #[test]
    #[ignore]
    fn pin_close_all_destroys_every_window() {
        per_monitor();
        for i in 0..5 {
            open(image(100, 80), PinPlace::At(p(100 + i * 40, 100 + i * 40))).unwrap();
        }
        pump();
        let hs = handles();
        assert_eq!(hs.len(), 5);
        assert_eq!(count(), 5);
        close_all();
        assert_eq!(count(), 0);
        for h in hs {
            // SAFETY: IsWindow chỉ hỏi tính hợp lệ của handle.
            assert!(!unsafe { IsWindow(Some(h)) }.as_bool());
        }
        // Gọi lại vô hại.
        close_all();
        assert_eq!(count(), 0);
    }

    #[test]
    #[ignore]
    fn pin_limit() {
        per_monitor();
        for i in 0..core_pin::MAX_PINS as i32 {
            open(image(40, 30), PinPlace::At(p(50 + i * 10, 50 + i * 10))).unwrap();
        }
        assert_eq!(count(), core_pin::MAX_PINS);
        assert!(matches!(
            open(image(40, 30), PinPlace::At(p(10, 10))),
            Err(PinError::Limit)
        ));
        assert_eq!(count(), core_pin::MAX_PINS);
        close_all();
        assert_eq!(count(), 0);
    }

    #[test]
    #[ignore]
    fn pin_rejects_invalid_image() {
        per_monitor();
        let bad = Image {
            width: 0,
            height: 10,
            bgra: Vec::new(),
        };
        assert!(matches!(
            open(bad, PinPlace::At(p(10, 10))),
            Err(PinError::Create(_))
        ));
        let bad = Image {
            width: 10,
            height: 10,
            bgra: vec![0; 12],
        };
        assert!(matches!(
            open(bad, PinPlace::CenterOnCursorMonitor),
            Err(PinError::Create(_))
        ));
        assert_eq!(count(), 0);
    }

    #[test]
    #[ignore]
    fn pin_zoom_keeps_anchor() {
        per_monitor();
        let (iw, ih) = (200u32, 200u32);
        open(image(iw, ih), PinPlace::At(p(300, 300))).unwrap();
        pump();
        let h = handles()[0];
        let (ax, ay) = (350i32, 380i32);
        // Điểm ảnh dưới chuột trước khi phóng (100%).
        let before = rect_of(h);
        let b0 = ((before.right - before.left) - iw as i32) / 2;
        let (u0, v0) = (ax - (before.left + b0), ay - (before.top + b0));
        let wp = (120usize) << 16;
        let lp = (((ay as u32 & 0xFFFF) << 16) | (ax as u32 & 0xFFFF)) as isize;
        // SAFETY: gửi thông điệp cho cửa sổ ảnh ghim còn sống của luồng này.
        unsafe {
            SendMessageW(
                h,
                WM_MOUSEWHEEL,
                Some(windows::Win32::Foundation::WPARAM(wp)),
                Some(windows::Win32::Foundation::LPARAM(lp)),
            );
        }
        pump();
        let after = rect_of(h);
        let (dw, _) = core_pin::display_size(iw, ih, 110);
        let b1 = ((after.right - after.left) - dw) / 2;
        assert_eq!(after.right - after.left, dw + 2 * b1);
        let (ix, iy) = (after.left + b1, after.top + b1);
        // Điểm ảnh dưới chuột sau khi phóng (110%).
        let u1 = ((ax - ix) as f64 * 100.0 / 110.0).round() as i32;
        let v1 = ((ay - iy) as f64 * 100.0 / 110.0).round() as i32;
        assert!((u1 - u0).abs() <= 1, "x {u0} -> {u1}");
        assert!((v1 - v0).abs() <= 1, "y {v0} -> {v1}");
        close_all();
    }

    #[test]
    #[ignore]
    fn pin_at_places_image_exactly() {
        per_monitor();
        let at = p(240, 160);
        let (iw, ih) = (123u32, 77u32);
        open(image(iw, ih), PinPlace::At(at)).unwrap();
        pump();
        let h = handles()[0];
        let mut rc = RECT::default();
        // SAFETY: h là cửa sổ còn sống của luồng này; rc là biến cục bộ.
        unsafe {
            let _ = GetClientRect(h, &mut rc);
        }
        let b = ((rc.right - rc.left) - iw as i32) / 2;
        assert!(b >= 1);
        assert_eq!(rc.bottom - rc.top, ih as i32 + 2 * b);
        let mut corner = POINT { x: b, y: b };
        // SAFETY: như trên.
        unsafe {
            let _ = ClientToScreen(h, &mut corner);
        }
        assert_eq!((corner.x, corner.y), (at.x, at.y));
        close_all();
    }
}
