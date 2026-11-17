//! Lịch sử ảnh chụp: phần thuần của cửa sổ (bố cục theo DPI, chạm, cuộn, bảng hành động, đệm ảnh nhỏ,
//! mã thế hệ, giữ mục chọn). Không gọi hệ điều hành nên kiểm thử được.

#[cfg(test)]
mod tests {
    use super::*;
    use darkshot_core::history::{Entry, Kind};
    use darkshot_core::i18n::Lang;
    use darkshot_core::image::Image;
    use darkshot_core::naming::LocalTime;
    use darkshot_core::theme::Rgb;
    use std::path::PathBuf;
    use windows::Win32::Foundation::{POINT, RECT};

    const DPIS: [u32; 7] = [96, 120, 144, 168, 192, 240, 288];

    fn area(r: &RECT) -> i64 {
        i64::from((r.right - r.left).max(0)) * i64::from((r.bottom - r.top).max(0))
    }

    fn overlap(a: &RECT, b: &RECT) -> bool {
        a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom
    }

    fn inside(inner: &RECT, outer: &RECT) -> bool {
        inner.left >= outer.left
            && inner.top >= outer.top
            && inner.right <= outer.right
            && inner.bottom <= outer.bottom
    }

    /// Mọi cỡ (pixel) từ tối thiểu tới 4K ở `dpi`.
    fn sizes(dpi: u32) -> Vec<(i32, i32)> {
        let (mw, mh) = crate::history_window::min_client_size(dpi);
        let mut out = Vec::new();
        for w in [mw, mw + 37, mw * 5 / 4, 1920, 2560, 3840] {
            for h in [mh, mh + 23, mh * 4 / 3, 1080, 1440, 2160] {
                if w >= mw && h >= mh {
                    out.push((w, h));
                }
            }
        }
        out
    }

    fn named(l: &Layout) -> Vec<(&'static str, RECT)> {
        let mut v = vec![
            ("search", l.search),
            ("chip0", l.chips[0]),
            ("chip1", l.chips[1]),
            ("chip2", l.chips[2]),
            ("chip3", l.chips[3]),
            ("clear", l.clear_btn),
            ("list", l.list),
            ("preview", l.preview_area),
            ("meta", l.meta_area),
            ("buttons", l.buttons_row),
            ("status", l.status_line),
        ];
        if area(&l.banner) > 0 {
            v.push(("banner", l.banner));
        }
        v
    }

    #[test]
    fn layout_never_overlaps_or_overflows() {
        for dpi in DPIS {
            for (w, h) in sizes(dpi) {
                for banner in [false, true] {
                    let l = layout(w, h, dpi, banner);
                    let client = RECT {
                        left: 0,
                        top: 0,
                        right: w,
                        bottom: h,
                    };
                    let parts = named(&l);
                    for (n, r) in &parts {
                        assert!(area(r) > 0, "{n} empty at {w}x{h}@{dpi} banner={banner}");
                        assert!(inside(r, &client), "{n} overflows at {w}x{h}@{dpi}");
                    }
                    for (i, (na, a)) in parts.iter().enumerate() {
                        for (nb, b) in parts.iter().skip(i + 1) {
                            assert!(
                                !overlap(a, b),
                                "{na} overlaps {nb} at {w}x{h}@{dpi} banner={banner}: {a:?} {b:?}"
                            );
                        }
                    }
                    // Thanh trên chứa ô tìm, bộ lọc, nút xóa; dải thông báo nằm dưới thanh trên.
                    for r in [l.search, l.chips[0], l.chips[3], l.clear_btn] {
                        assert!(inside(&r, &l.header), "{w}x{h}@{dpi}");
                    }
                    assert_eq!(area(&l.banner) > 0, banner);
                    if banner {
                        assert!(l.banner.top >= l.header.bottom);
                        assert!(inside(&l.banner_btn, &l.banner));
                    }
                    // Khung chi tiết chứa xem trước, thông tin, nút, dòng trạng thái theo thứ tự trên xuống.
                    for r in [l.preview_area, l.meta_area, l.buttons_row, l.status_line] {
                        assert!(inside(&r, &l.detail), "{w}x{h}@{dpi}");
                    }
                    assert!(l.preview_area.bottom <= l.meta_area.top);
                    assert!(l.meta_area.bottom <= l.buttons_row.top);
                    assert!(l.buttons_row.bottom <= l.status_line.top);
                    assert!(inside(&l.title, &l.meta_area));
                    // Cột danh sách 40% kẹp 320–420 DIP; khung chi tiết bên phải.
                    let lw = l.list.right - l.list.left;
                    let (lo, hi) = (scale(320, dpi), scale(420, dpi));
                    assert!(lw >= lo && lw <= hi, "list {lw} at {w}x{h}@{dpi}");
                    assert!(l.detail.left >= l.list.right);
                    assert_eq!(l.list.top, l.body.top);
                    assert_eq!(l.list.bottom, h);
                    // Thân nằm dưới thanh trên (+ dải thông báo).
                    let top = scale(56, dpi) + if banner { scale(44, dpi) } else { 0 };
                    assert_eq!(l.body.top, top, "{w}x{h}@{dpi}");
                    assert_eq!(l.row_h, scale(72, dpi));
                    assert_eq!(l.thumb, (scale(88, dpi), scale(56, dpi)));
                    // Ô tìm co từ 220 xuống 160 DIP.
                    let sw = l.search.right - l.search.left;
                    assert!(sw >= scale(160, dpi) - 1 && sw <= scale(220, dpi) + 1);
                    // Ô ảnh vừa trong dòng, chữ còn chỗ.
                    assert!(l.thumb.1 < l.row_h);
                    assert!(l.list_client_w > l.thumb.0 + scale(150, dpi));
                    // Hai hàng nút.
                    assert!(l.buttons_row.bottom - l.buttons_row.top >= 2 * scale(32, dpi));
                    assert!(l.preview_area.bottom - l.preview_area.top >= scale(80, dpi));
                }
            }
        }
    }

    #[test]
    fn default_layout_matches_spec_numbers() {
        let l = layout(960, 640, 96, false);
        assert_eq!(
            l.header,
            RECT {
                left: 0,
                top: 0,
                right: 960,
                bottom: 56
            }
        );
        assert_eq!(
            l.list,
            RECT {
                left: 0,
                top: 56,
                right: 384,
                bottom: 640
            }
        );
        assert_eq!(l.search.left, 16);
        assert_eq!(l.search.bottom - l.search.top, 32);
        assert_eq!(l.chips[0].bottom - l.chips[0].top, 28);
        assert_eq!(l.clear_btn.right, 960 - 16);
        assert_eq!(l.preview_area.left, 384 + 16);
        assert_eq!(l.preview_area.right, 960 - 16);
        assert_eq!(l.preview_area.top, 56 + 16);
        assert_eq!(l.status_line.bottom, 640 - 16);
        // Kẹp: hẹp thì 320, rộng thì 420.
        assert_eq!(layout(760, 520, 96, false).list.right, 320);
        assert_eq!(layout(1600, 900, 96, false).list.right, 420);
        assert_eq!(layout(1520, 1040, 192, false).list.right, 640);
        let b = layout(960, 640, 96, true);
        assert_eq!(b.body.top, 100);
        assert_eq!(b.banner.top, 56);
        assert_eq!((b.banner.left, b.banner.right), (16, 944));
    }

    #[test]
    fn rows_scroll_and_visibility() {
        let l = layout(960, 640, 96, false);
        let view = l.list.bottom - l.list.top; // 584
        assert_eq!(
            l.row_rect(0, 0),
            RECT {
                left: 0,
                top: 0,
                right: l.list_client_w,
                bottom: 72
            }
        );
        assert_eq!(l.row_rect(3, 100).top, 3 * 72 - 100);
        // Không cuộn khi vừa khung.
        assert_eq!(l.scroll_clamp(500, 5), 0);
        assert_eq!(l.scroll_clamp(-20, 50), 0);
        let max = 50 * 72 - view;
        assert_eq!(l.scroll_clamp(1_000_000, 50), max);
        assert_eq!(l.scroll_clamp(123, 50), 123);
        // ensure_visible: đã thấy thì giữ; dưới đáy thì cuộn tới vừa đáy; trên đỉnh thì cuộn lên.
        assert_eq!(l.ensure_visible(2, 0, 50), 0);
        assert_eq!(l.ensure_visible(20, 0, 50), 21 * 72 - view);
        assert_eq!(l.ensure_visible(1, 500, 50), 72);
        assert_eq!(l.ensure_visible(49, 0, 50), max);
        assert_eq!(l.ensure_visible(0, max, 50), 0);
        assert_eq!(l.visible_rows(), (view / 72) as usize);
        // Dòng giao vùng cần vẽ.
        assert_eq!(l.rows_in(0, 72, 0, 50), 0..1);
        assert_eq!(l.rows_in(70, 150, 0, 50), 0..3);
        assert_eq!(l.rows_in(0, view, 36, 50), 0..9);
        assert_eq!(l.rows_in(0, view, 0, 3), 0..3);
        assert_eq!(l.rows_in(0, view, 0, 0), 0..0);
    }

    #[test]
    fn hit_testing() {
        let mut l = layout(960, 640, 96, false);
        let p = |x, y| POINT { x, y };
        let mid = |r: RECT| p((r.left + r.right) / 2, (r.top + r.bottom) / 2);
        assert_eq!(l.hit_test(p(10, 56 + 10), 0, 50), Hit::Row(0));
        assert_eq!(l.hit_test(p(10, 56 + 72 + 1), 0, 50), Hit::Row(1));
        assert_eq!(l.hit_test(p(10, 56 + 10), 72, 50), Hit::Row(1));
        // Ngoài số dòng: không trúng.
        assert_eq!(l.hit_test(p(10, 56 + 72 * 2 + 5), 0, 2), Hit::None);
        // Thanh cuộn không phải dòng.
        assert_eq!(l.hit_test(p(l.list.right - 2, 60), 0, 50), Hit::None);
        for k in 0..4 {
            assert_eq!(l.hit_test(mid(l.chips[k]), 0, 50), Hit::Chip(k));
        }
        assert_eq!(l.hit_test(mid(l.clear_btn), 0, 50), Hit::Clear);
        assert_eq!(l.hit_test(mid(l.preview_area), 0, 50), Hit::Preview);
        assert_eq!(l.hit_test(mid(l.search), 0, 50), Hit::None);
        l.place_buttons([80, 110, 130, 150, 130], [true, true, false, true, true]);
        assert_eq!(l.hit_test(mid(l.buttons[0]), 0, 50), Hit::Button(0));
        assert_eq!(l.hit_test(mid(l.buttons[4]), 0, 50), Hit::Button(4));
        // Nút ẩn không trúng.
        assert_eq!(area(&l.buttons[2]), 0);
        assert_eq!(l.hit_test(p(-5, -5), 0, 50), Hit::None);
    }

    #[test]
    fn buttons_flow_in_two_lines_without_overlap() {
        for dpi in DPIS {
            for (w, h) in sizes(dpi) {
                let mut l = layout(w, h, dpi, false);
                // Bề rộng thật cỡ chữ tiếng Việt dài nhất + đệm.
                let wd = [72, 108, 128, 152, 124].map(|v| scale(v, dpi));
                for shown in [
                    [true, true, false, true, true],
                    [true, true, true, false, true],
                    [true, true, false, false, true],
                    [true, true, true, true, true],
                ] {
                    l.place_buttons(wd, shown);
                    let vis: Vec<RECT> =
                        (0..5).filter(|&k| shown[k]).map(|k| l.buttons[k]).collect();
                    for (i, a) in vis.iter().enumerate() {
                        assert!(inside(a, &l.buttons_row), "{a:?} at {w}x{h}@{dpi}");
                        assert_eq!(a.bottom - a.top, scale(32, dpi));
                        for b in vis.iter().skip(i + 1) {
                            assert!(!overlap(a, b), "{a:?} {b:?} at {w}x{h}@{dpi}");
                        }
                    }
                    for k in 0..5 {
                        if !shown[k] {
                            assert_eq!(area(&l.buttons[k]), 0);
                        }
                    }
                    // Nút Xóa ở mép phải.
                    assert_eq!(l.buttons[4].right, l.buttons_row.right);
                    // Mở là nút đầu tiên, sát lề trái.
                    assert_eq!(l.buttons[0].left, l.buttons_row.left);
                }
            }
        }
    }

    fn lt() -> LocalTime {
        LocalTime {
            year: 2026,
            month: 11,
            day: 18,
            hour: 9,
            minute: 5,
            second: 0,
        }
    }

    fn entry(n: u32, kind: Kind) -> Entry {
        Entry {
            id: format!("{n:021x}"),
            kind,
            unix: 1_700_000_000,
            local: lt(),
            width: 1920,
            height: 1080,
            bytes: Some(1000),
            path: matches!(kind, Kind::Saved | Kind::UploadFallback)
                .then(|| PathBuf::from("C:\\p\\a.png")),
            managed: matches!(kind, Kind::Uploaded | Kind::Copied),
            url: (kind == Kind::Uploaded).then(|| "https://x.example/a.png".to_string()),
            expires: None,
        }
    }

    #[test]
    fn action_table_by_kind_and_file() {
        use Avail::*;
        // Thứ tự: Mở, Sao chép ảnh, Sao chép liên kết, Mở thư mục, Xóa.
        let cases = [
            (
                Kind::Saved,
                true,
                [Enabled, Enabled, Hidden, Enabled, Enabled],
                false,
            ),
            (
                Kind::Saved,
                false,
                [Disabled, Disabled, Hidden, Disabled, Enabled],
                false,
            ),
            (
                Kind::UploadFallback,
                true,
                [Enabled, Enabled, Hidden, Enabled, Enabled],
                false,
            ),
            (
                Kind::UploadFallback,
                false,
                [Disabled, Disabled, Hidden, Disabled, Enabled],
                false,
            ),
            (
                Kind::Uploaded,
                true,
                [Enabled, Enabled, Enabled, Hidden, Enabled],
                true,
            ),
            // Tải lên: mở liên kết vẫn được khi bản sao mất.
            (
                Kind::Uploaded,
                false,
                [Enabled, Disabled, Enabled, Hidden, Enabled],
                true,
            ),
            (
                Kind::Copied,
                true,
                [Enabled, Enabled, Hidden, Hidden, Enabled],
                false,
            ),
            (
                Kind::Copied,
                false,
                [Disabled, Disabled, Hidden, Hidden, Enabled],
                false,
            ),
        ];
        for (kind, exists, want, link) in cases {
            let a = actions_for(&entry(1, kind), exists);
            assert_eq!(a.as_array(), want, "{kind:?} exists={exists}");
            assert_eq!(a.open_is_link, link, "{kind:?}");
        }
        // Liên kết không phải https: không mở được (vẫn sao chép được).
        let mut e = entry(2, Kind::Uploaded);
        e.url = Some("http://x/a.png".into());
        let a = actions_for(&e, true);
        assert_eq!(a.open, Disabled);
        assert_eq!(a.copy_link, Enabled);
        e.url = None;
        let a = actions_for(&e, true);
        assert_eq!((a.open, a.copy_link), (Disabled, Hidden));
    }

    #[test]
    fn thumbnail_fits_cell_without_upscaling() {
        let cell = RECT {
            left: 10,
            top: 20,
            right: 98,
            bottom: 76,
        }; // 88×56
           // 16:9 lớn: vừa bề ngang, căn giữa dọc.
        let r = fit_rect(1920, 1080, &cell);
        assert_eq!((r.right - r.left, r.bottom - r.top), (88, 50));
        assert_eq!((r.left, r.top), (10, 23));
        // Ảnh dọc: vừa bề cao.
        let r = fit_rect(500, 1000, &cell);
        assert_eq!((r.right - r.left, r.bottom - r.top), (28, 56));
        assert_eq!(r.left, 10 + 30);
        // Ảnh nhỏ: giữ nguyên cỡ, căn giữa.
        let r = fit_rect(20, 10, &cell);
        assert_eq!(
            r,
            RECT {
                left: 44,
                top: 43,
                right: 64,
                bottom: 53
            }
        );
        // Ô rỗng không panic.
        let z = RECT::default();
        let r = fit_rect(100, 100, &z);
        assert!(r.right >= r.left && r.bottom >= r.top);
        // Ghép lên nền: trong suốt ra màu nền, đục giữ màu.
        let img = Image {
            width: 2,
            height: 1,
            bgra: vec![0, 0, 255, 255, 0, 0, 255, 0],
        };
        let px = compose_over(&img, Rgb(0x15, 0x17, 0x1A));
        assert_eq!(px, vec![0x00FF_0000, 0x0015_171A]);
        let half = Image {
            width: 1,
            height: 1,
            bgra: vec![255, 255, 255, 128],
        };
        let p = compose_over(&half, Rgb(0, 0, 0))[0];
        assert!((0x7F..=0x81).contains(&(p & 0xFF)), "{p:x}");
        // Đích xem trước kẹp 2400.
        assert_eq!(preview_target(800, 500), (800, 500));
        assert_eq!(preview_target(5000, 3000), (2400, 2400));
        assert_eq!(preview_target(0, -4), (1, 1));
    }

    #[test]
    fn thumb_cache_is_lru_bounded_and_dedups() {
        let mut c: ThumbCache<u32> = ThumbCache::new(MAX_THUMBS);
        assert_eq!(MAX_THUMBS, 120);
        // Lần đầu: phải nạp; lần hai (đang nạp): không nạp lặp.
        assert!(c.request("a"));
        assert!(!c.request("a"));
        assert!(matches!(c.get("a"), Some(ThumbSlot::Loading)));
        assert!(c.fulfill("a", Some(1)).is_empty());
        assert!(matches!(c.get("a"), Some(ThumbSlot::Ready(1))));
        assert!(!c.request("a"));
        // Hỏng: nhớ hỏng, không thử lại.
        assert!(c.request("b"));
        assert!(c.fulfill("b", None).is_empty());
        assert!(matches!(c.get("b"), Some(ThumbSlot::Broken)));
        assert!(!c.request("b"));
        // Nạp lại: quên mục hỏng (sẽ thử lại), giữ mục đã có.
        c.forget_broken();
        assert!(c.get("b").is_none());
        assert!(matches!(c.get("a"), Some(ThumbSlot::Ready(1))));
        // Kết quả trùng cho mục đã có: trả bitmap cũ để xóa.
        assert_eq!(c.fulfill("a", Some(9)), vec![1]);

        // Vượt 120: loại đúng mục ít dùng nhất.
        let mut c: ThumbCache<u32> = ThumbCache::new(MAX_THUMBS);
        for i in 0..120u32 {
            assert!(c.request(&format!("id{i}")));
            assert!(c.fulfill(&format!("id{i}"), Some(i)).is_empty());
        }
        assert_eq!(c.len(), 120);
        // Dùng lại id0: id1 thành cũ nhất.
        assert!(c.get("id0").is_some());
        assert!(c.request("new"));
        let out = c.fulfill("new", Some(999));
        assert_eq!(out, vec![1]);
        assert_eq!(c.len(), 120);
        assert!(c.get("id1").is_none());
        assert!(c.get("id0").is_some());
        for i in 0..300u32 {
            c.request(&format!("more{i}"));
            let _ = c.fulfill(&format!("more{i}"), Some(1000 + i));
            assert!(c.len() <= 120);
        }
        // Xóa hết trả mọi bitmap còn giữ.
        let all = c.clear();
        assert_eq!(all.len(), 120);
        assert_eq!(c.len(), 0);
    }

    #[test]
    fn stale_generation_results_are_dropped() {
        let mut g = Generation::default();
        let a = g.next();
        assert!(g.is_current(a));
        let b = g.next();
        assert_ne!(a, b);
        assert!(!g.is_current(a), "kết quả cũ phải bị bỏ");
        assert!(g.is_current(b));
        assert_eq!(g.current(), b);
    }

    #[test]
    fn selection_survives_reload_by_id() {
        let old = ["a", "b", "c", "d"];
        // Mục mới chèn đầu: vẫn chọn "c".
        assert_eq!(
            reselect(&old, Some("c"), &["n", "a", "b", "c", "d"]),
            Some(3)
        );
        // Mục đang chọn biến mất: chọn mục kế (d), cuối thì mục trước.
        assert_eq!(reselect(&old, Some("c"), &["a", "b", "d"]), Some(2));
        assert_eq!(reselect(&old, Some("d"), &["a", "b", "c"]), Some(2));
        assert_eq!(reselect(&old, Some("b"), &["a", "d"]), Some(1));
        assert_eq!(reselect(&old, Some("b"), &["x", "y"]), Some(1));
        assert_eq!(reselect(&old, Some("b"), &[]), None);
        // Chưa chọn gì thì vẫn không chọn.
        assert_eq!(reselect(&old, None, &["a"]), None);
        // id lạ (không có ở danh sách cũ) và đã mất: không chọn.
        assert_eq!(reselect(&old, Some("zz"), &["a"]), None);
    }

    #[test]
    fn keyboard_navigation_steps() {
        use Nav::*;
        assert_eq!(step_selection(None, 10, Down, 4), Some(0));
        assert_eq!(step_selection(None, 10, Up, 4), Some(0));
        assert_eq!(step_selection(None, 0, Down, 4), None);
        assert_eq!(step_selection(Some(0), 10, Up, 4), Some(0));
        assert_eq!(step_selection(Some(3), 10, Down, 4), Some(4));
        assert_eq!(step_selection(Some(9), 10, Down, 4), Some(9));
        assert_eq!(step_selection(Some(5), 10, Home, 4), Some(0));
        assert_eq!(step_selection(Some(5), 10, End, 4), Some(9));
        assert_eq!(step_selection(Some(5), 10, PageDown, 4), Some(9));
        assert_eq!(step_selection(Some(1), 10, PageDown, 4), Some(5));
        assert_eq!(step_selection(Some(5), 10, PageUp, 4), Some(1));
        assert_eq!(step_selection(Some(2), 10, PageUp, 4), Some(0));
        // Số trang 0 vẫn đi ít nhất một dòng.
        assert_eq!(step_selection(Some(2), 10, PageDown, 0), Some(3));
        // Chỉ số cũ quá cuối (danh sách vừa ngắn đi): kẹp.
        assert_eq!(step_selection(Some(20), 10, Up, 4), Some(8));
    }

    #[test]
    fn expiry_dates_follow_language() {
        assert_eq!(expires_text(Lang::Vi, "2026-12-18"), "18/12/2026");
        assert_eq!(expires_text(Lang::En, "2026-12-18"), "2026-12-18");
        assert_eq!(expires_text(Lang::Vi, "rác"), "rác");
    }
}
