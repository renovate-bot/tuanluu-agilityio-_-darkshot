//! Cửa sổ "Các bước" (Xem lại) của phiên chụp từng bước. Stub: hiện thực ở commit sau.

use crate::history_layout::Nav;
use darkshot_core::geometry::Rect;
use darkshot_core::steps::Step;
use windows::Win32::UI::WindowsAndMessaging::WM_APP;

/// Kết quả nạp ảnh nhỏ/xem trước của luồng nền đã sẵn (hàng đợi trong trạng thái cửa sổ).
pub const WM_STEPS_WORK: u32 = WM_APP + 15;
/// Gửi tới cửa sổ chính khi một lượt xuất xong (kết quả trong hàng đợi của tiến trình).
pub const WM_STEPS_EXPORTED: u32 = WM_APP + 16;

/// Vùng client mặc định và tối thiểu (DIP).
const DEFAULT_W: i32 = 860;
const DEFAULT_H: i32 = 600;
const MIN_W: i32 = 640;
const MIN_H: i32 = 440;

const BTN_PAD: i32 = 16;

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
    body: Rect,
    list: Rect,
    preview: Rect,
    label: Rect,
    field: Rect,
    edit: Rect,
    buttons: [Rect; 6],
    row_h: i32,
    thumb: (i32, i32),
}

fn layout(cw: i32, ch: i32, dpi: u32, m: &Measure) -> Layout {
    let _ = (cw, ch, dpi, m);
    todo!()
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

fn card_rects(body: Rect, dpi: u32, title_h: i32, text_h: i32, btn_w: [i32; 2]) -> Card {
    let _ = (body, dpi, title_h, text_h, btn_w);
    todo!()
}

fn button_enabled(sel: Option<usize>, count: usize, exporting: bool) -> [bool; 6] {
    let _ = (sel, count, exporting);
    todo!()
}

fn reselect(old_file: Option<&str>, old_index: Option<usize>, new: &[Step]) -> Option<usize> {
    let _ = (old_file, old_index, new);
    todo!()
}

fn first_line(caption: &str) -> &str {
    let _ = caption;
    todo!()
}

fn to_crlf(s: &str) -> String {
    let _ = s;
    todo!()
}

fn from_crlf(s: &str) -> String {
    let _ = s;
    todo!()
}

fn scroll_clamp(scroll: i32, count: usize, row_h: i32, view_h: i32) -> i32 {
    let _ = (scroll, count, row_h, view_h);
    todo!()
}

fn ensure_visible(i: usize, scroll: i32, row_h: i32, view_h: i32) -> i32 {
    let _ = (i, scroll, row_h, view_h);
    todo!()
}

fn row_at(y: i32, scroll: i32, row_h: i32, count: usize) -> Option<usize> {
    let _ = (y, scroll, row_h, count);
    todo!()
}

fn nav_of(vk: u16) -> Option<Nav> {
    let _ = vk;
    todo!()
}

type Notify = fn(darkshot_core::actions::NoticeKind, &str, &str);

pub fn set_notifier(f: Notify) {
    let _ = f;
}

pub fn set_default_dir(f: fn() -> Option<std::path::PathBuf>) {
    let _ = f;
}

pub fn set_main_window(h: windows::Win32::Foundation::HWND) {
    let _ = h;
}

pub fn open() {}

pub fn open_confirm_end() {}

pub fn close() {}

pub fn relabel() {}

pub fn on_store_changed() {}

pub fn on_exported() {}

pub fn pre_translate(msg: &windows::Win32::UI::WindowsAndMessaging::MSG) -> bool {
    let _ = msg;
    false
}

fn start_export(parent: std::path::PathBuf) -> bool {
    let _ = parent;
    false
}

#[cfg(test)]
fn wait_export() {}

#[cfg(test)]
fn hwnd() -> Option<windows::Win32::Foundation::HWND> {
    None
}

#[cfg(test)]
fn row_texts() -> Vec<(String, String)> {
    Vec::new()
}

#[cfg(test)]
fn edit_hwnd() -> windows::Win32::Foundation::HWND {
    Default::default()
}

#[cfg(test)]
fn button_hwnd(b: Btn) -> windows::Win32::Foundation::HWND {
    let _ = b;
    Default::default()
}

#[cfg(test)]
fn card_button_hwnds() -> [windows::Win32::Foundation::HWND; 2] {
    Default::default()
}

#[cfg(test)]
fn confirm_open() -> bool {
    false
}

#[cfg(test)]
fn select(i: usize) {
    let _ = i;
}

#[cfg(test)]
fn selected() -> Option<usize> {
    None
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
        // Kết thúc phiên đã xuất (không cần hỏi): cửa sổ đang mở thì đóng theo.
        open();
        pump();
        crate::steps_store::mark_exported();
        open_confirm_end();
        pump();
        assert!(!crate::steps_store::active());
        assert!(hwnd().is_none());
        let _ = std::fs::remove_dir_all(&base);
    }
}
