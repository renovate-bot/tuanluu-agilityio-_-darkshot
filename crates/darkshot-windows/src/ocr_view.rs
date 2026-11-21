//! Khung ảnh của cửa sổ "Chữ nhận dạng": hình học thuần (ảnh gốc → bản giữ → hiển thị, cuộn, dò
//! điểm), trạng thái chọn dòng thuần, và các hàm vẽ GDI nhỏ (DIB, khung dòng, nhãn "Đã chép").

use darkshot_core::geometry::Rect;
use darkshot_core::ocr_text::{OcrLine, TextOptions};

/// Ảnh có cạnh dài hơn mức này thì cửa sổ chỉ giữ bản đã co về cạnh dài này (thiết kế 7b.7).
pub const KEEP_MAX: u32 = 4096;

/// Cỡ bản giữ của ảnh `w`×`h`: cạnh dài > `KEEP_MAX` thì co giữ tỉ lệ, còn lại giữ nguyên.
pub fn kept_size(w: u32, h: u32) -> (u32, u32) {
    let _ = (w, h);
    todo!()
}

/// Hình học khung ảnh. Tọa độ "nội dung": gốc ở góc trên trái vùng cuộn (cuộn = 0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Geom {
    /// Cỡ ảnh gốc (khung dòng của OCR tính theo cỡ này).
    pub orig: (u32, u32),
    /// Cỡ bản giữ.
    pub kept: (u32, u32),
    /// Chỗ ảnh hiển thị trong tọa độ nội dung (w, h = cỡ hiển thị).
    pub img: Rect,
    /// Chiều cao nội dung (≥ chiều cao khung khi không cần cuộn).
    pub content_h: i32,
    /// Chiều cao khung nhìn.
    pub view_h: i32,
}

/// Hình học cho khung nhìn `vw`×`vh` px, đệm `pad` px: ảnh khớp bề rộng (không phóng quá 1:1),
/// căn giữa ngang; vừa khung thì căn giữa dọc, cao hơn thì đặt sát trên và cuộn dọc.
pub fn geom(orig: (u32, u32), kept: (u32, u32), vw: i32, vh: i32, pad: i32) -> Geom {
    let _ = (orig, kept, vw, vh, pad);
    todo!()
}

impl Geom {
    /// Vị trí cuộn lớn nhất.
    pub fn max_scroll(&self) -> i32 {
        todo!()
    }

    /// Khung dòng `r` (px ảnh gốc) trong tọa độ nội dung: gốc → bản giữ → hiển thị.
    pub fn line_rect(&self, r: Rect) -> Rect {
        let _ = r;
        todo!()
    }

    /// Điểm (tọa độ nội dung) → px ảnh gốc; ngoài ảnh hiển thị thì `None`.
    pub fn to_image(&self, x: i32, y: i32) -> Option<(i32, i32)> {
        let _ = (x, y);
        todo!()
    }

    /// Vị trí cuộn mới để khung `r` (tọa độ nội dung) hiện đủ, cách mép `pad`; đã kẹp.
    pub fn scroll_to_show(&self, scroll: i32, r: Rect, pad: i32) -> i32 {
        let _ = (scroll, r, pad);
        todo!()
    }
}

/// Vùng chọn dòng trong khung ảnh.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pick {
    /// Chỉ số các dòng đang chọn (tăng dần, không trùng).
    pub sel: Vec<usize>,
    /// Dòng đang trỏ bằng bàn phím (viền nét đứt).
    pub cursor: Option<usize>,
}

/// Thao tác của người dùng trên khung ảnh (dòng trúng đã dò sẵn).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickInput {
    Click(Option<usize>),
    CtrlClick(Option<usize>),
    Up,
    Down,
    Enter,
    Space,
    SelectAll,
    Escape,
}

/// Việc cần làm sau một thao tác.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PickOutcome {
    Nothing,
    /// Chép `text`; nhãn "Đã chép" đặt cạnh dòng `at`.
    Copy { text: String, at: usize },
    /// Đóng cửa sổ (Esc khi không có vùng chọn).
    Close,
}

impl Pick {
    pub fn is_selected(&self, i: usize) -> bool {
        self.sel.binary_search(&i).is_ok()
    }
}

/// Áp `input` lên vùng chọn `p` của `lines`; trả việc cần làm (chữ cần chép ghép theo `o`).
pub fn pick(p: &mut Pick, lines: &[OcrLine], o: TextOptions, input: PickInput) -> PickOutcome {
    let _ = (p, lines, o, input);
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkshot_core::ocr_text::line_at;

    fn r(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    fn line(text: &str, rect: Rect) -> OcrLine {
        OcrLine {
            text: text.into(),
            rect,
        }
    }

    fn three() -> Vec<OcrLine> {
        vec![
            line("one", r(10, 10, 100, 20)),
            line("two", r(10, 40, 100, 20)),
            line("three", r(10, 70, 100, 20)),
        ]
    }

    #[test]
    fn kept_size_caps_long_edge() {
        assert_eq!(kept_size(1920, 1080), (1920, 1080));
        assert_eq!(kept_size(4096, 100), (4096, 100));
        assert_eq!(kept_size(800, 12_000), (273, 4096));
        assert_eq!(kept_size(5000, 3000), (4096, 2458));
        assert_eq!(kept_size(100_000, 10), (4096, 1));
        // Ảnh nhỏ không bao giờ bị phóng (khác `ocr_scale`).
        assert_eq!(kept_size(30, 20), (30, 20));
        assert_eq!(kept_size(0, 0), (0, 0));
    }

    #[test]
    fn wide_image_fits_width_and_centres() {
        let g = geom((1920, 1080), (1920, 1080), 600, 400, 8);
        assert_eq!(g.img, r(8, 35, 584, 329));
        assert_eq!(g.content_h, 400);
        assert_eq!(g.view_h, 400);
        assert_eq!(g.max_scroll(), 0);
    }

    #[test]
    fn small_image_not_upscaled() {
        let g = geom((200, 50), (200, 50), 600, 400, 8);
        assert_eq!(g.img, r(200, 175, 200, 50));
        assert_eq!(g.max_scroll(), 0);
    }

    #[test]
    fn tall_image_fits_width_and_scrolls() {
        // Ảnh chụp cuộn 800×12000: bản giữ 273×4096, hẹp hơn khung nên giữ 1:1 và cuộn.
        let g = geom((800, 12_000), (273, 4096), 600, 400, 8);
        assert_eq!(g.img, r(163, 8, 273, 4096));
        assert_eq!(g.content_h, 4112);
        assert_eq!(g.max_scroll(), 3712);
        // Rộng hơn khung: khớp bề rộng.
        let g = geom((2000, 12_000), (683, 4096), 600, 400, 8);
        assert_eq!(g.img.w, 584);
        assert_eq!(g.img.h, 3502);
        assert_eq!(g.img.x, 8);
        assert_eq!(g.content_h, 3518);
    }

    #[test]
    fn degenerate_view_or_image_is_empty() {
        for g in [
            geom((0, 0), (0, 0), 600, 400, 8),
            geom((100, 100), (100, 100), 10, 10, 8),
            geom((100, 100), (100, 100), 0, 0, 0),
        ] {
            assert_eq!((g.img.w, g.img.h), (0, 0), "{g:?}");
            assert_eq!(g.max_scroll(), 0, "{g:?}");
            assert_eq!(g.to_image(5, 5), None);
            let lr = g.line_rect(r(10, 10, 20, 20));
            assert_eq!((lr.w, lr.h), (0, 0), "{lr:?}");
        }
    }

    #[test]
    fn line_rect_maps_through_three_layers() {
        // 1:1 cả ba lớp: chỉ dời theo chỗ ảnh.
        let g = geom((200, 50), (200, 50), 600, 400, 8);
        assert_eq!(g.line_rect(r(10, 10, 50, 20)), r(210, 185, 50, 20));
        // Co giữ (12000 → 4096) rồi hiển thị 1:1: tỉ lệ 4096/12000 theo dọc, 273/800 theo ngang.
        let g = geom((800, 12_000), (273, 4096), 600, 400, 8);
        let lr = g.line_rect(r(20, 11_900, 300, 40));
        assert_eq!(lr, r(163 + 6, 8 + 4061, 104, 15));
    }

    /// Tâm khung hiển thị của mọi dòng đổi ngược về px gốc thì trúng đúng dòng đó, ở mọi tổ hợp co
    /// giữ và co hiển thị (ba lớp tọa độ).
    #[test]
    fn display_centre_hits_same_line() {
        let cases = [
            ((1920u32, 1080u32), 600, 400),
            ((800, 12_000), 600, 400),
            ((2000, 12_000), 600, 400),
            ((5000, 3000), 900, 700),
            ((300, 80), 600, 400),
        ];
        for (orig, vw, vh) in cases {
            let (ow, oh) = (orig.0 as i32, orig.1 as i32);
            let lines: Vec<OcrLine> = (0..5)
                .map(|i| {
                    let h = (oh / 12).max(4);
                    line(&format!("l{i}"), r(ow / 10, i * oh / 5 + 1, ow / 2, h))
                })
                .collect();
            let g = geom(orig, kept_size(orig.0, orig.1), vw, vh, 8);
            for (i, l) in lines.iter().enumerate() {
                let d = g.line_rect(l.rect);
                assert!(d.w > 0 && d.h > 0, "{orig:?} {d:?}");
                let (cx, cy) = (d.x + d.w / 2, d.y + d.h / 2);
                let (px, py) = g.to_image(cx, cy).expect("inside image");
                assert_eq!(line_at(&lines, px, py), Some(i), "{orig:?} line {i} {d:?}");
            }
        }
    }

    #[test]
    fn to_image_outside_is_none_and_corners_map() {
        let g = geom((1920, 1080), (1920, 1080), 600, 400, 8);
        assert_eq!(g.to_image(7, 100), None);
        assert_eq!(g.to_image(100, 34), None);
        assert_eq!(g.to_image(8 + 584, 100), None);
        assert_eq!(g.to_image(100, 35 + 329), None);
        assert_eq!(g.to_image(8, 35), Some((0, 0)));
        let (x, y) = g.to_image(8 + 583, 35 + 328).unwrap();
        assert!((1915..1920).contains(&x) && (1074..1080).contains(&y), "{x},{y}");
    }

    /// Khung rỗng (`w == 0` hoặc `h == 0`): không panic, không chia 0.
    #[test]
    fn degenerate_line_boxes_do_not_panic() {
        let g = geom((800, 12_000), (273, 4096), 600, 400, 8);
        for b in [r(0, 0, 0, 0), r(100, 100, 0, 30), r(100, 100, 30, 0)] {
            let d = g.line_rect(b);
            assert!(d.w >= 0 && d.h >= 0);
        }
    }

    #[test]
    fn scroll_to_show_clamps() {
        let g = geom((800, 12_000), (273, 4096), 600, 400, 8);
        // Đã hiện: giữ.
        assert_eq!(g.scroll_to_show(0, r(0, 100, 10, 20), 8), 0);
        // Ở dưới: cuộn để mép dưới cách đáy 8.
        assert_eq!(g.scroll_to_show(0, r(0, 1000, 10, 20), 8), 1000 + 20 + 8 - 400);
        // Ở trên: cuộn để mép trên cách đỉnh 8.
        assert_eq!(g.scroll_to_show(2000, r(0, 1000, 10, 20), 8), 992);
        // Kẹp trong [0, max].
        assert_eq!(g.scroll_to_show(500, r(0, 2, 10, 20), 8), 0);
        assert_eq!(g.scroll_to_show(0, r(0, 4100, 10, 20), 8), g.max_scroll());
    }

    fn opts() -> TextOptions {
        TextOptions::default()
    }

    fn copy(text: &str, at: usize) -> PickOutcome {
        PickOutcome::Copy {
            text: text.into(),
            at,
        }
    }

    #[test]
    fn click_selects_one_and_copies() {
        let ls = three();
        let mut p = Pick::default();
        assert_eq!(pick(&mut p, &ls, opts(), PickInput::Click(Some(1))), copy("two", 1));
        assert_eq!(p.sel, vec![1]);
        assert_eq!(p.cursor, Some(1));
        assert_eq!(pick(&mut p, &ls, opts(), PickInput::Click(Some(2))), copy("three", 2));
        assert_eq!(p.sel, vec![2]);
        // Ngoài dải: như bấm chỗ trống.
        assert_eq!(pick(&mut p, &ls, opts(), PickInput::Click(Some(9))), PickOutcome::Nothing);
        assert!(p.sel.is_empty());
    }

    #[test]
    fn click_blank_clears_without_copy() {
        let ls = three();
        let mut p = Pick {
            sel: vec![0, 2],
            cursor: Some(2),
        };
        assert_eq!(pick(&mut p, &ls, opts(), PickInput::Click(None)), PickOutcome::Nothing);
        assert!(p.sel.is_empty());
        assert_eq!(p.cursor, Some(2));
    }

    #[test]
    fn ctrl_click_toggles_and_copies_in_reading_order() {
        let ls = three();
        let mut p = Pick::default();
        assert_eq!(pick(&mut p, &ls, opts(), PickInput::CtrlClick(Some(2))), copy("three", 2));
        assert_eq!(
            pick(&mut p, &ls, opts(), PickInput::CtrlClick(Some(0))),
            copy("one\r\nthree", 0)
        );
        assert_eq!(p.sel, vec![0, 2]);
        // Bỏ một dòng: chép phần còn lại.
        assert_eq!(pick(&mut p, &ls, opts(), PickInput::CtrlClick(Some(2))), copy("one", 2));
        // Bỏ dòng cuối: không chép.
        assert_eq!(pick(&mut p, &ls, opts(), PickInput::CtrlClick(Some(0))), PickOutcome::Nothing);
        assert!(p.sel.is_empty());
        // Ctrl+bấm chỗ trống: giữ nguyên vùng chọn.
        p.sel = vec![1];
        assert_eq!(pick(&mut p, &ls, opts(), PickInput::CtrlClick(None)), PickOutcome::Nothing);
        assert_eq!(p.sel, vec![1]);
        // Tùy chọn ghép áp dụng cho chữ chép.
        let one = TextOptions {
            single_line: true,
            paragraphs: false,
        };
        assert_eq!(pick(&mut p, &ls, one, PickInput::CtrlClick(Some(2))), copy("two three", 2));
    }

    #[test]
    fn arrows_move_cursor_without_copy() {
        let ls = three();
        let mut p = Pick::default();
        assert_eq!(pick(&mut p, &ls, opts(), PickInput::Down), PickOutcome::Nothing);
        assert_eq!(p.cursor, Some(0));
        pick(&mut p, &ls, opts(), PickInput::Down);
        pick(&mut p, &ls, opts(), PickInput::Down);
        pick(&mut p, &ls, opts(), PickInput::Down);
        assert_eq!(p.cursor, Some(2));
        pick(&mut p, &ls, opts(), PickInput::Up);
        assert_eq!(p.cursor, Some(1));
        let mut q = Pick::default();
        pick(&mut q, &ls, opts(), PickInput::Up);
        assert_eq!(q.cursor, Some(2));
        pick(&mut q, &ls, opts(), PickInput::Up);
        pick(&mut q, &ls, opts(), PickInput::Up);
        pick(&mut q, &ls, opts(), PickInput::Up);
        assert_eq!(q.cursor, Some(0));
        assert!(p.sel.is_empty() && q.sel.is_empty());
    }

    #[test]
    fn enter_and_space_act_on_cursor() {
        let ls = three();
        let mut p = Pick::default();
        assert_eq!(pick(&mut p, &ls, opts(), PickInput::Enter), PickOutcome::Nothing);
        assert_eq!(pick(&mut p, &ls, opts(), PickInput::Space), PickOutcome::Nothing);
        p.cursor = Some(1);
        assert_eq!(pick(&mut p, &ls, opts(), PickInput::Enter), copy("two", 1));
        p.cursor = Some(2);
        assert_eq!(pick(&mut p, &ls, opts(), PickInput::Space), copy("two\r\nthree", 2));
        assert_eq!(pick(&mut p, &ls, opts(), PickInput::Space), copy("two", 2));
        p.cursor = Some(0);
        assert_eq!(pick(&mut p, &ls, opts(), PickInput::Enter), copy("one", 0));
        assert_eq!(p.sel, vec![0]);
    }

    #[test]
    fn select_all_and_escape() {
        let ls = three();
        let mut p = Pick::default();
        assert_eq!(
            pick(&mut p, &ls, opts(), PickInput::SelectAll),
            copy("one\r\ntwo\r\nthree", 0)
        );
        assert_eq!(p.sel, vec![0, 1, 2]);
        assert_eq!(pick(&mut p, &ls, opts(), PickInput::Escape), PickOutcome::Nothing);
        assert!(p.sel.is_empty());
        assert_eq!(pick(&mut p, &ls, opts(), PickInput::Escape), PickOutcome::Close);
        p.cursor = Some(2);
        assert_eq!(
            pick(&mut p, &ls, opts(), PickInput::SelectAll),
            copy("one\r\ntwo\r\nthree", 2)
        );
    }

    #[test]
    fn no_lines_only_escape_does_anything() {
        let mut p = Pick::default();
        for i in [
            PickInput::Click(Some(0)),
            PickInput::CtrlClick(Some(0)),
            PickInput::Click(None),
            PickInput::Up,
            PickInput::Down,
            PickInput::Enter,
            PickInput::Space,
            PickInput::SelectAll,
        ] {
            assert_eq!(pick(&mut p, &[], opts(), i), PickOutcome::Nothing, "{i:?}");
            assert_eq!(p, Pick::default());
        }
        assert_eq!(pick(&mut p, &[], opts(), PickInput::Escape), PickOutcome::Close);
    }
}
