//! Phiên chụp cuộn: khung vùng (bốn cửa sổ mảnh), thanh điều khiển nổi, hẹn giờ chụp vùng và ghép
//! bằng `darkshot_core::scroll_stitch::Stitcher`, Esc toàn cục tạm.

#[cfg(test)]
mod tests {
    use super::*;

    fn r(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    fn overlaps(a: Rect, b: Rect) -> bool {
        a.x < b.right() && b.x < a.right() && a.y < b.bottom() && b.y < a.bottom()
    }

    #[test]
    fn frame_is_two_dip_outside_region() {
        assert_eq!(frame_thickness(96), 2);
        assert_eq!(frame_thickness(144), 3);
        assert_eq!(frame_thickness(192), 4);
        let f = frame_rects(r(100, 100, 400, 300), 2);
        assert_eq!(
            f,
            [
                r(98, 98, 404, 2),
                r(98, 400, 404, 2),
                r(98, 100, 2, 300),
                r(500, 100, 2, 300),
            ]
        );
        // Màn hình bên trái (tọa độ âm), DPI 200%.
        let region = r(-1000, -150, 300, 200);
        let f = frame_rects(region, 4);
        assert_eq!(f[0], r(-1004, -154, 308, 4));
        assert_eq!(f[3], r(-700, -150, 4, 200));
        // Không ô khung nào chồng lên vùng chụp.
        for part in f {
            assert!(!overlaps(part, region), "{part:?}");
        }
    }

    #[test]
    fn bar_goes_below_then_above_then_inside() {
        let work = r(0, 0, 1920, 1040);
        let size = (300, 40);
        // Dưới vùng, căn giữa theo vùng.
        assert_eq!(
            bar_origin(r(100, 100, 400, 300), size, 8, work, &[], true),
            Some(Point { x: 150, y: 408 })
        );
        // Không đủ chỗ dưới: trên.
        assert_eq!(
            bar_origin(r(100, 700, 400, 330), size, 8, work, &[], true),
            Some(Point { x: 150, y: 652 })
        );
        // Sát mép phải: kẹp vào vùng làm việc.
        assert_eq!(
            bar_origin(r(1800, 100, 120, 300), size, 8, work, &[], true),
            Some(Point { x: 1620, y: 408 })
        );
        // Vùng chiếm cả vùng làm việc, thanh loại khỏi ảnh chụp được: trong vùng làm việc, đáy.
        assert_eq!(
            bar_origin(work, size, 8, work, &[], true),
            Some(Point { x: 810, y: 992 })
        );
    }

    #[test]
    fn bar_on_negative_monitor() {
        let work = r(-1280, -200, 1280, 1000);
        assert_eq!(
            bar_origin(r(-1000, 0, 400, 300), (300, 40), 8, work, &[], false),
            Some(Point { x: -950, y: 308 })
        );
        // Không đủ chỗ dưới (đáy vùng làm việc ở 800): trên.
        assert_eq!(
            bar_origin(r(-1000, 400, 400, 380), (300, 40), 8, work, &[], false),
            Some(Point { x: -950, y: 352 })
        );
    }

    /// Thanh không loại khỏi ảnh chụp được: không bao giờ nằm trong vùng chụp.
    #[test]
    fn bar_never_inside_region_when_not_excluded() {
        let work = r(0, 0, 1920, 1040);
        let size = (300, 40);
        // Không có chỗ trên/dưới: bên phải, rồi bên trái.
        let region = r(0, 0, 1500, 1040);
        let at = bar_origin(region, size, 8, work, &[], false).unwrap();
        assert_eq!(at, Point { x: 1508, y: 1000 });
        let region = r(400, 0, 1520, 1040);
        let at = bar_origin(region, size, 8, work, &[], false).unwrap();
        assert_eq!(at, Point { x: 92, y: 1000 });
        // Vùng phủ kín màn hình: sang màn hình khác (đáy, giữa).
        let other = r(1920, 0, 1280, 1024);
        let at = bar_origin(work, size, 8, work, &[other], false).unwrap();
        assert_eq!(at, Point { x: 2410, y: 976 });
        // Không còn chỗ nào ngoài vùng: không đặt thanh.
        assert_eq!(bar_origin(work, size, 8, work, &[], false), None);
        // Màn hình khác cũng nằm trọn trong vùng: không đặt.
        let both = r(0, 0, 3200, 1040);
        assert_eq!(bar_origin(both, size, 8, work, &[other], false), None);
        // Nhiều vị trí ngẫu nhiên: luôn ngoài vùng và trong một vùng làm việc.
        for (x, y, w, h) in [
            (0, 0, 1920, 1000),
            (10, 10, 1900, 1020),
            (500, 300, 900, 700),
            (0, 30, 1920, 980),
        ] {
            let region = r(x, y, w, h);
            if let Some(p) = bar_origin(region, size, 8, work, &[other], false) {
                let bar = r(p.x, p.y, size.0, size.1);
                assert!(!overlaps(bar, region), "{region:?} -> {bar:?}");
                let inside = |wa: Rect| {
                    bar.x >= wa.x
                        && bar.y >= wa.y
                        && bar.right() <= wa.right()
                        && bar.bottom() <= wa.bottom()
                };
                assert!(inside(work) || inside(other), "{bar:?}");
            }
        }
    }

    #[test]
    fn region_valid_only_when_fully_on_screens() {
        let a = r(0, 0, 1920, 1080);
        let b = r(1920, 0, 1280, 1024);
        let left = r(-1280, 0, 1280, 1024);
        assert!(region_valid(r(100, 100, 400, 300), &[a, b]));
        // Vắt qua hai màn hình liền nhau.
        assert!(region_valid(r(1800, 100, 300, 300), &[a, b]));
        assert!(region_valid(r(-200, 100, 400, 300), &[left, a]));
        // Màn hình phải đã rút: phần vùng nằm ngoài.
        assert!(!region_valid(r(1800, 100, 300, 300), &[a]));
        // Phần dưới màn hình phải (cao 1024) nằm ngoài.
        assert!(!region_valid(r(1900, 1000, 100, 60), &[a, b]));
        assert!(!region_valid(r(0, 0, 0, 10), &[a]));
        assert!(!region_valid(r(0, 0, 10, 10), &[]));
    }

    #[test]
    fn machine_ends_once() {
        use darkshot_core::scroll_stitch::{Push, Reason};
        let mut m = Machine::new();
        assert_eq!(m.phase(), Phase::Running);
        assert!(!m.input(Input::Push(Push::Appended(300)), 0));
        assert!(!m.input(Input::Push(Push::NoChange), 100));
        assert!(m.input(Input::Done, 200));
        assert_eq!(m.phase(), Phase::Done);
        assert!(m.phase().delivers());
        // Đã kết thúc: mọi đầu vào sau bị bỏ.
        assert!(!m.input(Input::Cancel, 300));
        assert!(!m.input(Input::Push(Push::Limit), 300));
        assert_eq!(m.phase(), Phase::Done);

        let mut m = Machine::new();
        assert!(m.input(Input::Cancel, 0));
        assert_eq!(m.phase(), Phase::Cancelled);
        assert!(!m.phase().delivers());
        assert!(!m.input(Input::Done, 0));

        let mut m = Machine::new();
        assert!(m.input(Input::Push(Push::Limit), 0));
        assert_eq!(m.phase(), Phase::Limit);
        assert!(m.phase().delivers());

        let mut m = Machine::new();
        assert!(m.input(Input::RegionLost, 0));
        assert_eq!(m.phase(), Phase::Lost);
        assert!(m.phase().delivers());

        // Khung không ghép được: trạng thái vàng 1,5 s, vẫn chạy.
        let mut m = Machine::new();
        assert!(!m.warning(0));
        assert!(!m.input(Input::Push(Push::Uncertain(Reason::NoOverlap)), 1000));
        assert_eq!(m.phase(), Phase::Running);
        assert!(m.warning(1000));
        assert!(m.warning(2499));
        assert!(!m.warning(2500));
        // Lần nữa: gia hạn.
        m.input(Input::Push(Push::Uncertain(Reason::Backward)), 2000);
        assert!(m.warning(3499));
        // Nối được không tắt trạng thái vàng sớm.
        m.input(Input::Push(Push::Appended(10)), 2100);
        assert!(m.warning(3000));
    }

    #[test]
    fn machine_slows_timer_after_slow_ticks() {
        let mut m = Machine::new();
        assert_eq!(m.interval_ms(), INTERVAL_MS);
        assert_eq!(INTERVAL_MS, 100);
        assert_eq!(SLOW_INTERVAL_MS, 200);
        // Lượt chậm lẻ tẻ: không giãn.
        assert_eq!(m.tick_took(90), None);
        assert_eq!(m.tick_took(20), None);
        assert_eq!(m.tick_took(90), None);
        assert_eq!(m.tick_took(80), None, "80 ms chưa phải chậm");
        assert_eq!(m.interval_ms(), 100);
        // Đủ lượt chậm liên tiếp: giãn lên 200 ms đúng một lần.
        for _ in 1..SLOW_STREAK {
            assert_eq!(m.tick_took(81), None);
        }
        assert_eq!(m.tick_took(81), Some(SLOW_INTERVAL_MS));
        assert_eq!(m.interval_ms(), 200);
        assert_eq!(m.tick_took(120), None);
        // Nhanh trở lại đủ lâu: về 100 ms.
        for _ in 1..FAST_STREAK {
            assert_eq!(m.tick_took(10), None);
        }
        assert_eq!(m.tick_took(10), Some(INTERVAL_MS));
        assert_eq!(m.interval_ms(), 100);
        // Một lượt chậm cắt chuỗi nhanh.
        for _ in 0..SLOW_STREAK {
            m.tick_took(100);
        }
        assert_eq!(m.interval_ms(), 200);
        for _ in 1..FAST_STREAK {
            m.tick_took(10);
        }
        assert_eq!(m.tick_took(60), None);
        assert_eq!(m.tick_took(10), None);
        assert_eq!(m.interval_ms(), 200);
    }

    #[test]
    fn bar_layout_fits_text_at_every_dpi() {
        for dpi in [96u32, 120, 144, 192] {
            let k = |v: i32| v * dpi as i32 / 96;
            let t = Texts {
                title: k(200),
                height: k(60),
                status: k(90),
                buttons: [k(40), k(45)],
                line: k(16),
            };
            let l = layout(dpi, &t);
            let win = r(0, 0, l.size.0, l.size.1);
            let inside = |x: Rect| {
                x.x >= 0 && x.y >= 0 && x.right() <= win.right() && x.bottom() <= win.bottom()
            };
            assert!(l.size.1 >= k(BAR_H));
            for part in [l.dot, l.title, l.height, l.status, l.buttons[0], l.buttons[1]] {
                assert!(inside(part), "dpi {dpi} {part:?}");
            }
            assert!(l.title.w >= t.title && l.height.w >= t.height && l.status.w >= t.status);
            // Trái sang phải, không chồng nhau.
            let order = [l.dot, l.title, l.height, l.status, l.buttons[0], l.buttons[1]];
            for pair in order.windows(2) {
                assert!(pair[0].right() <= pair[1].x, "dpi {dpi} {pair:?}");
            }
            for (b, tw) in l.buttons.iter().zip(t.buttons) {
                assert!(b.w >= tw + 2 * k(BTN_PAD));
                assert!(b.h >= t.line);
            }
            // Dò ô.
            let c = |x: Rect| Point {
                x: x.x + x.w / 2,
                y: x.y + x.h / 2,
            };
            assert_eq!(hit(&l, c(l.buttons[0])), Some(BarButton::Done));
            assert_eq!(hit(&l, c(l.buttons[1])), Some(BarButton::Cancel));
            assert_eq!(hit(&l, c(l.title)), None);
        }
        assert_eq!(BUTTONS, [BarButton::Done, BarButton::Cancel]);
    }

    /// Thanh và khung chỉ được tạo khi loại khỏi ảnh chụp được (Windows 10 2004 trở lên).
    #[test]
    fn exclusion_needs_windows_10_2004() {
        assert!(!exclude_supported(None));
        assert!(!exclude_supported(Some(19040)));
        assert!(exclude_supported(Some(19041)));
        assert!(os_build().is_some_and(|b| b > 7600));
    }
}

/// Chạy tay một lượt (mở cửa sổ thật của kiểm thử, không dời chuột hay phím, không cuộn ứng dụng
/// khác): `cargo test -p darkshot-windows scroll_session::live -- --ignored --test-threads=1`.
#[cfg(test)]
mod live {
    use super::*;
    use std::cell::RefCell;
    use windows::Win32::Graphics::Dwm::DwmFlush;
    use windows::Win32::Graphics::Gdi::{
        SetDIBitsToDevice, UpdateWindow, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    };
    use windows::Win32::System::Threading::{
        GetCurrentProcess, GetGuiResources, GR_GDIOBJECTS, GR_USEROBJECTS,
    };
    use windows::Win32::UI::HiDpi::{
        SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetClientRect, GetForegroundWindow, GetWindowDisplayAffinity, IsWindow,
        PeekMessageW, TranslateMessage, MSG, PM_REMOVE, SW_SHOWNOACTIVATE,
    };

    /// Nội dung của cửa sổ thử: cao `PAGE_H`, vẽ từ hàng `OFFSET`.
    const TEST_W: i32 = 360;
    const TEST_H: i32 = 240;
    const PAGE_H: i32 = 2000;

    thread_local! {
        static OFFSET: Cell<i32> = const { Cell::new(0) };
        static PAGE: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
        static RESULT: RefCell<Option<(Image, u32)>> = const { RefCell::new(None) };
    }

    fn keep_result(img: Image, uncertain: u32) {
        RESULT.with(|r| *r.borrow_mut() = Some((img, uncertain)));
    }

    /// Trang thử: mỗi hàng một mẫu riêng (băm của chỉ số hàng), có chữ giả ở nhiều cột.
    fn make_page() -> Vec<u32> {
        let mut v = vec![0u32; (TEST_W * PAGE_H) as usize];
        for y in 0..PAGE_H {
            let mut s = (y as u32).wrapping_mul(2_654_435_761) ^ 0x9E37_79B9;
            for x in 0..TEST_W {
                s ^= s << 13;
                s ^= s >> 17;
                s ^= s << 5;
                let on = (x / 6 + y / 3) % 7 != 0 && s % 3 == 0;
                v[(y * TEST_W + x) as usize] = if on { s & 0x00FF_FFFF } else { 0x00F0_F0F0 };
            }
        }
        v
    }

    fn pump_for(ms: u64) {
        let end = std::time::Instant::now() + std::time::Duration::from_millis(ms);
        while std::time::Instant::now() < end {
            let mut msg = MSG::default();
            // SAFETY: vòng bơm thông điệp chuẩn của luồng kiểm thử.
            unsafe {
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    unsafe extern "system" fn test_proc(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        if msg == WM_PAINT {
            let mut ps = PAINTSTRUCT::default();
            // SAFETY: cặp BeginPaint/EndPaint; trang thử sống trong thread_local của luồng này.
            unsafe {
                let dc = BeginPaint(h, &mut ps);
                let off = OFFSET.with(|o| o.get());
                PAGE.with(|p| {
                    let p = p.borrow();
                    let rows = &p[(off * TEST_W) as usize..((off + TEST_H) * TEST_W) as usize];
                    let bmi = BITMAPINFO {
                        bmiHeader: BITMAPINFOHEADER {
                            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                            biWidth: TEST_W,
                            biHeight: -TEST_H,
                            biPlanes: 1,
                            biBitCount: 32,
                            biCompression: BI_RGB.0,
                            ..Default::default()
                        },
                        ..Default::default()
                    };
                    SetDIBitsToDevice(
                        dc,
                        0,
                        0,
                        TEST_W as u32,
                        TEST_H as u32,
                        0,
                        0,
                        0,
                        TEST_H as u32,
                        rows.as_ptr().cast(),
                        &bmi,
                        DIB_RGB_COLORS,
                    );
                });
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

    /// Cửa sổ thử luôn trên cùng, không lấy tiêu điểm; trả cửa sổ và vùng client (tọa độ màn hình).
    fn test_window() -> (HWND, Rect) {
        // SAFETY: đăng ký lớp và tạo cửa sổ popup của luồng kiểm thử.
        unsafe {
            let hinst = GetModuleHandleW(None).unwrap();
            let class = w!("darkshot.scroll-test");
            let wc = WNDCLASSW {
                lpfnWndProc: Some(test_proc),
                hInstance: hinst.into(),
                lpszClassName: class,
                ..Default::default()
            };
            RegisterClassW(&wc);
            let h = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                class,
                w!("scroll test"),
                WS_POPUP,
                200,
                150,
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
            let mut cr = RECT::default();
            let _ = GetClientRect(h, &mut cr);
            let mut wr = RECT::default();
            let _ = GetWindowRect(h, &mut wr);
            (h, r_of(wr.left, wr.top, cr.right, cr.bottom))
        }
    }

    fn r_of(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    fn scroll_to(h: HWND, off: i32) {
        OFFSET.with(|o| o.set(off));
        // SAFETY: vẽ lại cửa sổ thử của chính kiểm thử rồi chờ DWM trình bày.
        unsafe {
            let _ = InvalidateRect(Some(h), None, false);
            let _ = UpdateWindow(h);
            let _ = DwmFlush();
        }
    }

    fn gui(kind: windows::Win32::System::Threading::GET_GUI_RESOURCES_FLAGS) -> u32 {
        // SAFETY: chỉ đọc bộ đếm của tiến trình hiện tại.
        unsafe { GetGuiResources(GetCurrentProcess(), kind) }
    }

    fn dpi_aware() {
        // SAFETY: đặt chế độ DPI cho luồng kiểm thử (như manifest của ứng dụng).
        unsafe {
            SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
    }

    #[test]
    #[ignore]
    fn session_stitches_test_window_scroll() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        dpi_aware();
        PAGE.with(|p| *p.borrow_mut() = make_page());
        OFFSET.with(|o| o.set(0));
        let (tw, region) = test_window();
        pump_for(300);
        set_on_result(keep_result);
        // SAFETY: chỉ đọc cửa sổ đang có tiêu điểm.
        let fg = unsafe { GetForegroundWindow() };
        assert!(start(region));
        assert!(active());
        // Bắt đầu lại khi đang chạy: từ chối.
        assert!(!start(region));
        // SAFETY: chỉ đọc cửa sổ đang có tiêu điểm.
        assert_eq!(unsafe { GetForegroundWindow() }, fg, "không lấy tiêu điểm");
        pump_for(350);
        let step = 60;
        let steps = 8;
        for i in 1..=steps {
            scroll_to(tw, i * step);
            pump_for(300);
        }
        done();
        assert!(!active());
        let (img, uncertain) = RESULT.with(|r| r.borrow_mut().take()).expect("có kết quả");
        assert_eq!(uncertain, 0);
        assert_eq!(img.width, region.w as u32);
        assert_eq!(img.height, (region.h + steps * step) as u32);
        // Ảnh ghép đúng nội dung trang (vài hàng mẫu).
        PAGE.with(|p| {
            let p = p.borrow();
            for y in [0, 100, 239, 300, 500, 719] {
                for x in [0, 77, 359] {
                    let i = ((y * region.w + x) * 4) as usize;
                    let got = u32::from_le_bytes([img.bgra[i], img.bgra[i + 1], img.bgra[i + 2], 0]);
                    assert_eq!(got, p[(y * TEST_W + x) as usize], "({x}, {y})");
                }
            }
        });
        // SAFETY: hủy cửa sổ thử của chính kiểm thử.
        unsafe {
            let _ = DestroyWindow(tw);
        }
        pump_for(50);
    }

    #[test]
    #[ignore]
    fn windows_are_excluded_and_session_does_not_leak() {
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        dpi_aware();
        PAGE.with(|p| *p.borrow_mut() = make_page());
        let (tw, region) = test_window();
        pump_for(200);
        set_on_result(keep_result);
        let supported = exclude_supported(os_build());
        let (mut gdi0, mut user0) = (0, 0);
        for i in 0..11 {
            if i == 1 {
                gdi0 = gui(GR_GDIOBJECTS);
                user0 = gui(GR_USEROBJECTS);
            }
            assert!(start(region));
            pump_for(150);
            let ws = session_windows();
            if supported {
                // Bốn khung và thanh, tất cả loại khỏi ảnh chụp.
                assert_eq!(ws.len(), 5);
                for h in &ws {
                    let mut aff = 0u32;
                    // SAFETY: h là cửa sổ của tiến trình này; aff là biến cục bộ.
                    unsafe { GetWindowDisplayAffinity(*h, &mut aff) }.unwrap();
                    assert_eq!(aff, WDA_EXCLUDEFROMCAPTURE.0);
                }
            }
            // Xen kẽ Hủy (bỏ) và Xong (giao kết quả).
            if i % 2 == 0 {
                cancel();
                assert!(RESULT.with(|r| r.borrow_mut().take()).is_none());
            } else {
                done();
                assert!(RESULT.with(|r| r.borrow_mut().take()).is_some());
            }
            pump_for(30);
            assert!(!active());
            for h in ws {
                // SAFETY: chỉ hỏi cửa sổ đã hủy có còn không.
                assert!(!unsafe { IsWindow(Some(h)) }.as_bool());
            }
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
    }
}
