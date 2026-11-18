//! Thanh phiên chụp từng bước: cửa sổ nhỏ, tối, luôn trên cùng, không lên thanh tác vụ, không
//! giành tiêu điểm, không có trong ảnh chụp.

#[cfg(test)]
mod tests {
    use super::*;
    use darkshot_core::geometry::{Point, Rect};

    fn inside(outer: Rect, r: Rect) -> bool {
        r.x >= outer.x && r.y >= outer.y && r.right() <= outer.right() && r.bottom() <= outer.bottom()
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
        assert_eq!(
            hit(
                &l,
                Point {
                    x: l.size.0,
                    y: 1
                }
            ),
            Hit::None
        );
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
        DispatchMessageW, GetForegroundWindow, GetWindowDisplayAffinity, IsWindow,
        IsWindowVisible, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
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
