//! Lịch sử ảnh chụp: phần thuần của cửa sổ (bố cục theo DPI, chạm, cuộn, bảng hành động, đệm ảnh nhỏ,
//! mã thế hệ, giữ mục chọn). Không gọi hệ điều hành nên kiểm thử được.
//!
//! Mọi số đo là DIP (thiết kế mục 3.2–3.5) và được quy đổi sang pixel theo `dpi`.

use crate::winutil::scale;
use darkshot_core::history::{filter_entries, format_dims, format_size, Entry, Filter, Kind};
use darkshot_core::i18n::{Lang, Msg};
use darkshot_core::image::Image;
use darkshot_core::theme::Rgb;
use darkshot_core::thumb::fit_within;
use std::collections::{HashMap, VecDeque};
use std::ops::Range;
use std::path::Path;
use windows::Win32::Foundation::{POINT, RECT};

/// Lề ngoài và đệm khung chi tiết.
pub const MARGIN: i32 = 16;
/// Thanh trên; dải thông báo (khi tắt mà còn mục) cộng thêm.
pub const HEADER_H: i32 = 56;
pub const BANNER_H: i32 = 44;
/// Dải thông báo thật cao 36, phần còn lại của 44 là khoảng cách với thân.
const BANNER_STRIP: i32 = 36;
const BANNER_BTN_W: i32 = 120;
const BANNER_BTN_H: i32 = 28;
/// Cột danh sách: 40% bề rộng, kẹp 320–420.
const LIST_PERCENT: i32 = 40;
const LIST_MIN: i32 = 320;
const LIST_MAX: i32 = 420;
/// Dòng danh sách và ô ảnh nhỏ.
pub const ROW_H: i32 = 72;
pub const THUMB_W: i32 = 88;
pub const THUMB_H: i32 = 56;
/// Ô tìm: cao 32, rộng tối đa 260 (thiết kế ghi 220; nới để chữ gợi ý tiếng Việt hiện đủ), co xuống 160.
const SEARCH_H: i32 = 32;
const SEARCH_MIN: i32 = 160;
const SEARCH_MAX: i32 = 260;
/// Viên thuốc lọc: cao 28; bề rộng cố định đủ cho nhãn dài nhất của hai ngôn ngữ.
const CHIP_H: i32 = 28;
const CHIP_W: [i32; 4] = [68, 76, 92, 80];
const CHIP_GAP: i32 = 6;
/// Nút "Xóa lịch sử" ở mép phải thanh trên.
const CLEAR_W: i32 = 112;
const CLEAR_H: i32 = 32;
/// Khoảng giữa ô tìm, nhóm lọc và nút xóa.
const HEAD_GAP: i32 = 16;
/// Chỗ dành cho thanh cuộn dọc (cửa sổ thay bằng bề rộng thật sau khi tạo).
pub const SCROLLBAR_W: i32 = 17;
/// Nút hành động: cao 32, cách nhau 8, tối đa hai hàng.
pub const BTN_H: i32 = 32;
pub const BTN_GAP: i32 = 8;
/// Khung chi tiết: tiêu đề, sáu dòng nhãn–giá trị, dòng trạng thái.
const TITLE_H: i32 = 28;
const TITLE_GAP: i32 = 6;
pub const META_ROW: i32 = 22;
pub const META_ROWS: i32 = 6;
const STATUS_H: i32 = 20;
const STATUS_GAP: i32 = 8;
const SECTION_GAP: i32 = 12;
/// Đệm ảnh nhỏ tối đa; cạnh dài của bản xem trước.
pub const MAX_THUMBS: usize = 120;
pub const PREVIEW_MAX: i32 = 2400;

/// Vị trí các phần của cửa sổ (pixel, tọa độ client của cửa sổ chính).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layout {
    pub dpi: u32,
    pub header: RECT,
    pub search: RECT,
    pub chips: [RECT; 4],
    pub clear_btn: RECT,
    /// Rỗng khi không có dải thông báo.
    pub banner: RECT,
    pub banner_btn: RECT,
    /// Phần dưới thanh trên (và dải thông báo): danh sách + khung chi tiết.
    pub body: RECT,
    /// Cửa sổ con danh sách (gồm cả thanh cuộn).
    pub list: RECT,
    /// Bề rộng vùng client của danh sách (trừ thanh cuộn).
    pub list_client_w: i32,
    pub detail: RECT,
    pub preview_area: RECT,
    pub meta_area: RECT,
    /// Dòng tiêu đề (ngày giờ đầy đủ), nằm đầu `meta_area`.
    pub title: RECT,
    pub buttons_row: RECT,
    /// Năm nút hành động (Mở, Sao chép ảnh, Sao chép liên kết, Mở thư mục, Xóa); rỗng khi ẩn.
    pub buttons: [RECT; 5],
    pub status_line: RECT,
    pub row_h: i32,
    /// Cỡ ô ảnh nhỏ (pixel).
    pub thumb: (i32, i32),
}

/// Chỗ con trỏ trúng.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    Row(usize),
    Chip(usize),
    Clear,
    Preview,
    Button(usize),
    None,
}

fn rc(left: i32, top: i32, right: i32, bottom: i32) -> RECT {
    RECT {
        left,
        top,
        right,
        bottom,
    }
}

fn contains(r: &RECT, p: POINT) -> bool {
    p.x >= r.left && p.x < r.right && p.y >= r.top && p.y < r.bottom
}

/// Bố cục cho vùng client `client_w` x `client_h` pixel ở `dpi`; `banner` = có dải "Lịch sử đang tắt".
pub fn layout(client_w: i32, client_h: i32, dpi: u32, banner: bool) -> Layout {
    let s = |v: i32| scale(v, dpi);
    let (w, h) = (client_w.max(0), client_h.max(0));
    let m = s(MARGIN);
    let head_h = s(HEADER_H);
    let header = rc(0, 0, w, head_h);

    // Thanh trên: ô tìm bên trái, bốn viên thuốc, nút xóa ở mép phải.
    let chip_w = CHIP_W.map(s);
    let chips_total: i32 = chip_w.iter().sum::<i32>() + 3 * s(CHIP_GAP);
    let clear_w = s(CLEAR_W);
    let avail = w - 2 * m - chips_total - clear_w - 2 * s(HEAD_GAP);
    let search_w = avail.clamp(s(SEARCH_MIN), s(SEARCH_MAX));
    let st = (head_h - s(SEARCH_H)) / 2;
    let search = rc(m, st, m + search_w, st + s(SEARCH_H));
    let ct = (head_h - s(CHIP_H)) / 2;
    let mut chips = [RECT::default(); 4];
    let mut x = search.right + s(HEAD_GAP);
    for (k, cw) in chip_w.iter().enumerate() {
        chips[k] = rc(x, ct, x + cw, ct + s(CHIP_H));
        x += cw + s(CHIP_GAP);
    }
    let bt = (head_h - s(CLEAR_H)) / 2;
    let clear_btn = rc(w - m - clear_w, bt, w - m, bt + s(CLEAR_H));

    // Dải thông báo dưới thanh trên.
    let (banner_rc, banner_btn, body_top) = if banner {
        let b = rc(m, head_h, w - m, head_h + s(BANNER_STRIP));
        let bh = s(BANNER_BTN_H);
        let top = b.top + (s(BANNER_STRIP) - bh) / 2;
        let btn = rc(
            b.right - s(4) - s(BANNER_BTN_W),
            top,
            b.right - s(4),
            top + bh,
        );
        (b, btn, head_h + s(BANNER_H))
    } else {
        (RECT::default(), RECT::default(), head_h)
    };
    let body = rc(0, body_top, w, h);

    // Danh sách bên trái, chi tiết bên phải.
    let list_w = (w * LIST_PERCENT / 100).clamp(s(LIST_MIN), s(LIST_MAX));
    let list = rc(0, body_top, list_w, h);
    let detail = rc(list_w, body_top, w, h);
    let (x0, x1) = (list_w + m, w - m);
    let (top, bottom) = (body_top + m, h - m);
    let status_line = rc(x0, bottom - s(STATUS_H), x1, bottom);
    let btn_rows_h = 2 * s(BTN_H) + s(BTN_GAP);
    let buttons_row = rc(
        x0,
        status_line.top - s(STATUS_GAP) - btn_rows_h,
        x1,
        status_line.top - s(STATUS_GAP),
    );
    let meta_h = s(TITLE_H) + s(TITLE_GAP) + META_ROWS * s(META_ROW);
    let meta_area = rc(
        x0,
        buttons_row.top - s(SECTION_GAP) - meta_h,
        x1,
        buttons_row.top - s(SECTION_GAP),
    );
    let title = rc(x0, meta_area.top, x1, meta_area.top + s(TITLE_H));
    let preview_area = rc(x0, top, x1, meta_area.top - s(SECTION_GAP));

    Layout {
        dpi,
        header,
        search,
        chips,
        clear_btn,
        banner: banner_rc,
        banner_btn,
        body,
        list,
        list_client_w: (list_w - s(SCROLLBAR_W)).max(0),
        detail,
        preview_area,
        meta_area,
        title,
        buttons_row,
        buttons: [RECT::default(); 5],
        status_line,
        row_h: s(ROW_H).max(1),
        thumb: (s(THUMB_W), s(THUMB_H)),
    }
}

impl Layout {
    fn view_h(&self) -> i32 {
        (self.list.bottom - self.list.top).max(0)
    }

    /// Dòng `i` trong tọa độ client của danh sách khi đã cuộn `scroll` pixel.
    pub fn row_rect(&self, i: usize, scroll: i32) -> RECT {
        let top = i as i32 * self.row_h - scroll;
        rc(0, top, self.list_client_w, top + self.row_h)
    }

    /// Số dòng trọn vẹn thấy được (ít nhất 1): bước của PageUp/PageDown.
    pub fn visible_rows(&self) -> usize {
        (self.view_h() / self.row_h).max(1) as usize
    }

    /// Chiều cao toàn bộ `count` dòng.
    pub fn content_h(&self, count: usize) -> i32 {
        (count as i32).saturating_mul(self.row_h)
    }

    /// Kẹp độ cuộn vào [0, nội dung − khung].
    pub fn scroll_clamp(&self, scroll: i32, count: usize) -> i32 {
        let max = (self.content_h(count) - self.view_h()).max(0);
        scroll.clamp(0, max)
    }

    /// Độ cuộn mới ít thay đổi nhất để dòng `i` nằm trọn trong khung.
    pub fn ensure_visible(&self, i: usize, scroll: i32, count: usize) -> i32 {
        let top = i as i32 * self.row_h;
        let bottom = top + self.row_h;
        let view = self.view_h();
        let s = if top < scroll {
            top
        } else if bottom > scroll + view {
            bottom - view
        } else {
            scroll
        };
        self.scroll_clamp(s, count)
    }

    /// Các dòng giao dải [`top`, `bottom`) của client danh sách.
    pub fn rows_in(&self, top: i32, bottom: i32, scroll: i32, count: usize) -> Range<usize> {
        if count == 0 || bottom <= top {
            return 0..0;
        }
        let first = ((top + scroll).max(0) / self.row_h) as usize;
        let last = ((((bottom + scroll - 1).max(0)) / self.row_h) as usize + 1).min(count);
        first.min(last)..last
    }

    /// Chỗ trúng của điểm `pt` (tọa độ client cửa sổ chính).
    pub fn hit_test(&self, pt: POINT, scroll: i32, count: usize) -> Hit {
        if let Some(k) = self.chips.iter().position(|r| contains(r, pt)) {
            return Hit::Chip(k);
        }
        if contains(&self.clear_btn, pt) {
            return Hit::Clear;
        }
        if let Some(k) = self.buttons.iter().position(|r| contains(r, pt)) {
            return Hit::Button(k);
        }
        if contains(&self.preview_area, pt) {
            return Hit::Preview;
        }
        let rows = rc(
            self.list.left,
            self.list.top,
            self.list.left + self.list_client_w,
            self.list.bottom,
        );
        if contains(&rows, pt) {
            let i = ((pt.y - self.list.top + scroll) / self.row_h) as usize;
            if i < count {
                return Hit::Row(i);
            }
        }
        Hit::None
    }

    /// Xếp năm nút hành động (bề rộng pixel `widths`, `shown` = hiện) trong `buttons_row`: bốn nút đầu
    /// chảy từ trái, xuống hàng khi hết chỗ; "Xóa khỏi lịch sử" sát mép phải của hàng cuối (hoặc hàng
    /// dưới nếu không đủ chỗ). Tối đa hai hàng.
    pub fn place_buttons(&mut self, widths: [i32; 5], shown: [bool; 5]) {
        let r = self.buttons_row;
        let (gap, bh) = (scale(BTN_GAP, self.dpi), scale(BTN_H, self.dpi));
        let row_w = (r.right - r.left).max(0);
        let line_top = |line: i32| r.top + line.min(1) * (bh + gap);
        let (mut x, mut line) = (r.left, 0);
        for k in 0..4 {
            if !shown[k] {
                self.buttons[k] = RECT::default();
                continue;
            }
            let bw = widths[k].clamp(0, row_w);
            if x > r.left && x + bw > r.right {
                line += 1;
                x = r.left;
            }
            let t = line_top(line);
            self.buttons[k] = rc(x, t, x + bw, t + bh);
            x += bw + gap;
        }
        if shown[4] {
            let bw = widths[4].clamp(0, row_w);
            if x > r.left && x + bw > r.right {
                line += 1;
            }
            let t = line_top(line);
            self.buttons[4] = rc(r.right - bw, t, r.right, t + bh);
        } else {
            self.buttons[4] = RECT::default();
        }
        // Đủ một hàng: đặt ở hàng dưới, sát dòng trạng thái (nút luôn neo đáy khung chi tiết).
        if line == 0 {
            for (k, b) in self.buttons.iter_mut().enumerate() {
                if shown[k] {
                    b.top += bh + gap;
                    b.bottom += bh + gap;
                }
            }
        }
    }
}

/// Một hành động: ẩn (không áp dụng), vô hiệu (áp dụng nhưng không làm được), hoặc dùng được.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Avail {
    Hidden,
    Disabled,
    Enabled,
}

/// Bảng hành động của một mục (thiết kế mục 3.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionSet {
    pub open: Avail,
    /// Nút Mở mở liên kết trên trình duyệt (nhãn "Mở liên kết").
    pub open_is_link: bool,
    pub copy_image: Avail,
    pub copy_link: Avail,
    pub show_folder: Avail,
    pub remove: Avail,
}

impl ActionSet {
    /// Theo thứ tự nút: Mở, Sao chép ảnh, Sao chép liên kết, Mở thư mục, Xóa khỏi lịch sử.
    pub fn as_array(&self) -> [Avail; 5] {
        [
            self.open,
            self.copy_image,
            self.copy_link,
            self.show_folder,
            self.remove,
        ]
    }
}

/// Hành động áp dụng cho `e`; `file_exists` = tệp ảnh (`image_path`) còn trên đĩa.
pub fn actions_for(e: &Entry, file_exists: bool) -> ActionSet {
    let file = if file_exists {
        Avail::Enabled
    } else {
        Avail::Disabled
    };
    match e.kind {
        Kind::Saved | Kind::UploadFallback => ActionSet {
            open: if e.path.as_deref().is_some_and(user_png_openable) {
                file
            } else {
                Avail::Disabled
            },
            open_is_link: false,
            copy_image: file,
            copy_link: Avail::Hidden,
            show_folder: file,
            remove: Avail::Enabled,
        },
        Kind::Uploaded => ActionSet {
            open: if e.url.as_deref().is_some_and(url_openable) {
                Avail::Enabled
            } else {
                Avail::Disabled
            },
            open_is_link: true,
            copy_image: file,
            copy_link: if e.url.as_deref().is_some_and(|u| !u.is_empty()) {
                Avail::Enabled
            } else {
                Avail::Hidden
            },
            show_folder: Avail::Hidden,
            remove: Avail::Enabled,
        },
        Kind::Copied => ActionSet {
            open: file,
            open_is_link: false,
            copy_image: file,
            copy_link: Avail::Hidden,
            show_folder: Avail::Hidden,
            remove: Avail::Enabled,
        },
    }
}

/// Khung ảnh `img_w` x `img_h` vừa trong `cell`, giữ tỉ lệ, không phóng to, căn giữa.
pub fn fit_rect(img_w: u32, img_h: u32, cell: &RECT) -> RECT {
    let cw = (cell.right - cell.left).max(0) as u32;
    let ch = (cell.bottom - cell.top).max(0) as u32;
    let (fw, fh) = fit_within(img_w, img_h, cw, ch);
    let (fw, fh) = (fw as i32, fh as i32);
    let left = cell.left + (cell.right - cell.left - fw) / 2;
    let top = cell.top + (cell.bottom - cell.top - fh) / 2;
    rc(left, top, left + fw, top + fh)
}

/// Điểm ảnh `0x00RRGGBB` của `img` đã trộn alpha lên nền `bg` (đục hoàn toàn).
pub fn compose_over(img: &Image, bg: Rgb) -> Vec<u32> {
    let mix = |s: u8, b: u8, a: u32| (u32::from(s) * a + u32::from(b) * (255 - a) + 127) / 255;
    img.bgra
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| {
            let a = u32::from(p[3]);
            (mix(p[2], bg.0, a) << 16) | (mix(p[1], bg.1, a) << 8) | mix(p[0], bg.2, a)
        })
        .collect()
}

/// Cỡ đích (pixel) khi thu nhỏ bản xem trước cho vùng `w` x `h`: kẹp 1..=2400 mỗi cạnh.
pub fn preview_target(w: i32, h: i32) -> (u32, u32) {
    (
        w.clamp(1, PREVIEW_MAX) as u32,
        h.clamp(1, PREVIEW_MAX) as u32,
    )
}

/// Ô của đệm ảnh nhỏ.
#[derive(Debug, PartialEq, Eq)]
pub enum ThumbSlot<B> {
    /// Đã nhờ luồng nền nạp, chưa có kết quả.
    Loading,
    Ready(B),
    /// Nạp lỗi: vẽ biểu tượng ảnh vỡ, không thử lại cho tới lần nạp lại.
    Broken,
}

/// Đệm ảnh nhỏ theo id, loại mục ít dùng nhất khi vượt `cap`. `B` là bitmap; mọi bitmap bị loại hay
/// bị thay đều được trả về cho bên gọi giải phóng.
pub struct ThumbCache<B> {
    cap: usize,
    map: HashMap<String, ThumbSlot<B>>,
    /// Đầu = ít dùng nhất.
    order: VecDeque<String>,
}

impl<B> ThumbCache<B> {
    pub fn new(cap: usize) -> ThumbCache<B> {
        ThumbCache {
            cap: cap.max(1),
            map: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Số ô đang chờ luồng nền.
    #[allow(dead_code)]
    pub fn loading(&self) -> usize {
        self.map
            .values()
            .filter(|v| matches!(v, ThumbSlot::Loading))
            .count()
    }

    fn touch(&mut self, id: &str) {
        if let Some(pos) = self.order.iter().position(|k| k == id) {
            if let Some(k) = self.order.remove(pos) {
                self.order.push_back(k);
            }
        }
    }

    /// `true` khi bên gọi phải nhờ nạp `id` (chưa có gì); ghi nhận "đang nạp" để không nạp lặp.
    pub fn request(&mut self, id: &str) -> bool {
        if self.map.contains_key(id) {
            self.touch(id);
            return false;
        }
        self.map.insert(id.to_string(), ThumbSlot::Loading);
        self.order.push_back(id.to_string());
        true
    }

    /// Ô của `id` (đánh dấu vừa dùng).
    pub fn get(&mut self, id: &str) -> Option<&ThumbSlot<B>> {
        if self.map.contains_key(id) {
            self.touch(id);
        }
        self.map.get(id)
    }

    /// Ghi kết quả nạp (`None` = lỗi). Trả các bitmap bị thay hoặc bị loại.
    pub fn fulfill(&mut self, id: &str, bmp: Option<B>) -> Vec<B> {
        let slot = match bmp {
            Some(b) => ThumbSlot::Ready(b),
            None => ThumbSlot::Broken,
        };
        let mut out = Vec::new();
        match self.map.insert(id.to_string(), slot) {
            Some(ThumbSlot::Ready(old)) => {
                out.push(old);
                self.touch(id);
            }
            Some(_) => self.touch(id),
            None => self.order.push_back(id.to_string()),
        }
        while self.map.len() > self.cap {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            if oldest == id {
                self.order.push_back(oldest);
                if self.order.len() <= 1 {
                    break;
                }
                continue;
            }
            if let Some(ThumbSlot::Ready(b)) = self.map.remove(&oldest) {
                out.push(b);
            }
        }
        out
    }

    /// Quên các ô hỏng (lần nạp lại sẽ thử lại).
    pub fn forget_broken(&mut self) {
        self.map.retain(|_, v| !matches!(v, ThumbSlot::Broken));
        let map = &self.map;
        self.order.retain(|k| map.contains_key(k));
    }

    /// Xóa hết; trả mọi bitmap còn giữ.
    pub fn clear(&mut self) -> Vec<B> {
        self.order.clear();
        self.map
            .drain()
            .filter_map(|(_, v)| match v {
                ThumbSlot::Ready(b) => Some(b),
                _ => None,
            })
            .collect()
    }
}

/// Mã thế hệ: mỗi yêu cầu mới tăng mã; kết quả mang mã cũ bị bỏ.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Generation(u64);

impl Generation {
    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(1);
        self.0
    }

    pub fn current(&self) -> u64 {
        self.0
    }

    pub fn is_current(&self, g: u64) -> bool {
        g == self.0
    }
}

/// Chỉ số mục chọn sau khi danh sách đổi từ `old` sang `new` (đều là id theo thứ tự hiển thị):
/// còn thì giữ theo id; mất thì chọn mục kế còn lại (hết thì mục trước); không còn gì để dựa thì
/// mục cùng vị trí. Chưa chọn gì thì vẫn không chọn.
pub fn reselect(old: &[&str], selected: Option<&str>, new: &[&str]) -> Option<usize> {
    let sel = selected?;
    let pos = |id: &str| new.iter().position(|x| *x == id);
    if let Some(i) = pos(sel) {
        return Some(i);
    }
    let oi = old.iter().position(|x| *x == sel)?;
    if let Some(i) = old[oi + 1..].iter().find_map(|id| pos(id)) {
        return Some(i);
    }
    if let Some(i) = old[..oi].iter().rev().find_map(|id| pos(id)) {
        return Some(i);
    }
    (!new.is_empty()).then(|| oi.min(new.len() - 1))
}

/// Phím di chuyển trong danh sách.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Nav {
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
}

/// Mục chọn sau phím `nav` (danh sách `count` mục, trang `page` dòng).
pub fn step_selection(cur: Option<usize>, count: usize, nav: Nav, page: usize) -> Option<usize> {
    if count == 0 {
        return None;
    }
    let last = count - 1;
    let Some(c) = cur else {
        return Some(if nav == Nav::End { last } else { 0 });
    };
    let c = c.min(last);
    let p = page.max(1);
    Some(match nav {
        Nav::Up => c.saturating_sub(1),
        Nav::Down => (c + 1).min(last),
        Nav::Home => 0,
        Nav::End => last,
        Nav::PageUp => c.saturating_sub(p),
        Nav::PageDown => (c + p).min(last),
    })
}

/// Ngày hết hạn `YYYY-MM-DD` theo ngôn ngữ (tiếng Việt `dd/mm/yyyy`); dạng lạ giữ nguyên.
pub fn expires_text(lang: Lang, s: &str) -> String {
    let b = s.as_bytes();
    let ok = b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit());
    match (lang, ok) {
        (Lang::Vi, true) => format!("{}/{}/{}", &s[8..10], &s[5..7], &s[0..4]),
        _ => s.to_string(),
    }
}

/// Trạng thái đặc biệt (trống, tắt) vẽ giữa thân: biểu tượng, tiêu đề, đoạn giải thích, nút (nếu có).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EmptyRects {
    pub icon: RECT,
    pub title: RECT,
    pub body: RECT,
    /// Rỗng khi không có nút.
    pub button: RECT,
}

const EMPTY_TEXT_MAX: i32 = 420;
const EMPTY_ICON: (i32, i32) = (64, 48);
const EMPTY_TITLE_H: i32 = 28;

/// Bề rộng đoạn giải thích của trạng thái đặc biệt trong thân `body`.
pub fn empty_text_width(body: &RECT, dpi: u32) -> i32 {
    ((body.right - body.left) - 2 * scale(32, dpi)).clamp(1, scale(EMPTY_TEXT_MAX, dpi))
}

/// Xếp khối trạng thái đặc biệt giữa `body`, hơi lệch lên; `text_h` = chiều cao đoạn giải thích đã đo,
/// `button_w` = bề rộng nút (pixel) nếu có nút.
pub fn empty_state_rects(body: &RECT, dpi: u32, text_h: i32, button_w: Option<i32>) -> EmptyRects {
    let s = |v: i32| scale(v, dpi);
    let (iw, ih) = (s(EMPTY_ICON.0), s(EMPTY_ICON.1));
    let btn_h = if button_w.is_some() {
        s(20) + s(BTN_H)
    } else {
        0
    };
    let total = ih + s(16) + s(EMPTY_TITLE_H) + s(6) + text_h.max(0) + btn_h;
    let top = (body.top + (body.bottom - body.top - total) / 2 - s(16)).max(body.top + s(16));
    let cx = (body.left + body.right) / 2;
    let tw = empty_text_width(body, dpi);
    let icon = rc(cx - iw / 2, top, cx - iw / 2 + iw, top + ih);
    let title_top = icon.bottom + s(16);
    let title = rc(
        cx - tw / 2,
        title_top,
        cx - tw / 2 + tw,
        title_top + s(EMPTY_TITLE_H),
    );
    let body_top = title.bottom + s(6);
    let text = rc(title.left, body_top, title.right, body_top + text_h.max(0));
    let button = match button_w {
        Some(bw) => {
            let t = text.bottom + s(20);
            rc(cx - bw / 2, t, cx - bw / 2 + bw, t + s(BTN_H))
        }
        None => RECT::default(),
    };
    EmptyRects {
        icon,
        title,
        body: text,
        button,
    }
}

/// Dòng phụ của một dòng danh sách: `rộng × cao · dung lượng`.
pub fn row_subtitle(lang: Lang, e: &Entry) -> String {
    let dims = format_dims(e.width, e.height);
    match e.bytes {
        Some(b) => format!("{dims} \u{b7} {}", format_size(lang, b)),
        None => dims,
    }
}

// ---------------------------------------------------------------------------
// Tìm và lọc (thiết kế mục 3.3, 6)
// ---------------------------------------------------------------------------

/// Bộ lọc theo thứ tự bốn viên thuốc: Tất cả, Đã lưu, Đã tải lên, Đã chép.
pub const CHIP_FILTERS: [Filter; 4] =
    [Filter::All, Filter::Saved, Filter::Uploaded, Filter::Copied];

/// Bộ lọc của viên thứ `k` (`None` khi ngoài bốn viên).
pub fn chip_filter(k: usize) -> Option<Filter> {
    CHIP_FILTERS.get(k).copied()
}

/// Viên thuốc của bộ lọc `f`.
pub fn filter_chip(f: Filter) -> usize {
    CHIP_FILTERS.iter().position(|x| *x == f).unwrap_or(0)
}

/// Viên nào có `WS_TABSTOP`: chỉ viên đang chọn (như nhóm nút radio).
pub fn chip_tabstops(f: Filter) -> [bool; 4] {
    let sel = filter_chip(f);
    std::array::from_fn(|k| k == sel)
}

/// Bộ lọc sau phím mũi tên (`dir` âm = trái), dừng ở hai đầu.
pub fn chip_step(f: Filter, dir: i32) -> Filter {
    let k = filter_chip(f) as i32 + dir.signum();
    chip_filter(k.clamp(0, 3) as usize).unwrap_or(f)
}

/// Chỉ số vào `entries` (theo thứ tự hiển thị) của các mục qua `history::filter_entries`.
pub fn visible_indices(entries: &[Entry], filter: Filter, query: &str, lang: Lang) -> Vec<usize> {
    let base = entries.as_ptr() as usize;
    let size = std::mem::size_of::<Entry>().max(1);
    filter_entries(entries, filter, query, lang)
        .into_iter()
        .map(|e| (e as *const Entry as usize - base) / size)
        .collect()
}

/// Mục chọn sau khi đổi bộ lọc hoặc ô tìm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Refiltered {
    /// Vị trí trong danh sách hiển thị mới.
    pub selected: Option<usize>,
    /// Cuộn về đầu (mục chọn cũ đã rời khỏi danh sách).
    pub scroll_top: bool,
}

/// Giữ mục chọn nếu còn trong `new` (id theo thứ tự hiển thị); không thì chọn mục đầu và cuộn về đầu.
pub fn after_filter(selected: Option<&str>, new: &[&str]) -> Refiltered {
    if let Some(p) = selected.and_then(|id| new.iter().position(|x| *x == id)) {
        return Refiltered {
            selected: Some(p),
            scroll_top: false,
        };
    }
    Refiltered {
        selected: (!new.is_empty()).then_some(0),
        scroll_top: true,
    }
}

/// Mục chọn sau khi nạp lại mà mục đang chọn là `removed`: còn thì giữ theo id; đã bị xóa thì mục kế ở
/// cùng vị trí, hết thì mục cuối; không còn gì thì `None`.
pub fn select_after_remove(old: &[&str], removed: &str, new: &[&str]) -> Option<usize> {
    reselect(old, Some(removed), new)
}

// ---------------------------------------------------------------------------
// Hành động (thiết kế mục 3.6)
// ---------------------------------------------------------------------------

/// Tệp của người dùng mà nút Mở được phép chạy: đường dẫn tuyệt đối đuôi `.png` (ảnh Lưu và Lưu dự
/// phòng luôn là PNG). Chỉ mục bị sửa tay trỏ tới `.exe`, `.lnk`... thì không mở.
pub fn user_png_openable(p: &Path) -> bool {
    p.is_absolute()
        && p.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("png"))
}

/// Liên kết mở được trên trình duyệt: đúng tiền tố `https://` (chữ thường), có máy chủ, không có ký
/// tự điều khiển hay khoảng trắng.
pub fn url_openable(u: &str) -> bool {
    let Some(rest) = u.strip_prefix("https://") else {
        return false;
    };
    !rest.is_empty()
        && !rest.starts_with('/')
        && !u.chars().any(|c| c.is_control() || c.is_whitespace())
}

/// Đối số của `explorer.exe` để mở thư mục và chọn sẵn tệp `p`: đường dẫn trong ngoặc kép, nên dấu
/// cách và dấu phẩy không cắt đường dẫn. `None` (không gọi explorer) khi đường dẫn không tuyệt đối, có
/// dấu `"` hay ký tự điều khiển (không thể là tệp thật, và có thể chèn thêm đối số).
pub fn explorer_select_args(p: &Path) -> Option<String> {
    let s = p.to_str()?;
    if !p.is_absolute() || s.chars().any(|c| c == '"' || c.is_control()) {
        return None;
    }
    Some(format!("/select,\"{s}\""))
}

/// Đường dẫn đầy đủ của `explorer.exe` trong thư mục Windows `windir`; không biết (hoặc giá trị lạ) thì
/// dự phòng tên trần `explorer.exe`.
pub fn explorer_exe(windir: Option<&str>) -> String {
    let Some(d) = windir.map(|d| d.trim_end_matches('\\')) else {
        return "explorer.exe".to_string();
    };
    let exe = format!("{d}\\explorer.exe");
    if Path::new(&exe).is_absolute() && !d.chars().any(|c| c == '"' || c.is_control()) {
        exe
    } else {
        "explorer.exe".to_string()
    }
}

/// Phím mũi tên trên nhóm viên lọc.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChipKey {
    Left,
    Right,
    Up,
    Down,
}

/// Hướng đổi bộ lọc của phím mũi tên: Lên như Trái, Xuống như Phải (như nhóm nút radio).
pub fn chip_key_dir(k: ChipKey) -> i32 {
    match k {
        ChipKey::Left | ChipKey::Up => -1,
        ChipKey::Right | ChipKey::Down => 1,
    }
}

/// Kết quả một hành động, gửi về cửa sổ trong `WPARAM` của `WM_APP+10`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Status {
    /// Xong, không cần báo (mở tệp, mở thư mục).
    None = 0,
    ImageCopied = 1,
    LinkCopied = 2,
    Removed = 3,
    Cleared = 4,
    Failed = 5,
}

impl Status {
    pub fn from_wparam(v: usize) -> Status {
        match v {
            1 => Status::ImageCopied,
            2 => Status::LinkCopied,
            3 => Status::Removed,
            4 => Status::Cleared,
            5 => Status::Failed,
            _ => Status::None,
        }
    }

    /// Chữ của dòng trạng thái.
    pub fn msg(self) -> Option<Msg> {
        Some(match self {
            Status::None => return None,
            Status::ImageCopied => Msg::HistoryStatusImageCopied,
            Status::LinkCopied => Msg::HistoryStatusLinkCopied,
            Status::Removed => Msg::HistoryStatusRemoved,
            Status::Cleared => Msg::HistoryStatusCleared,
            Status::Failed => Msg::HistoryStatusFailed,
        })
    }

    /// Lỗi: vẽ bằng `DANGER`.
    pub fn is_error(self) -> bool {
        self == Status::Failed
    }
}

/// Một mục của menu chuột phải.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuItem {
    /// Chỉ số hành động (0–4, như thứ tự nút).
    pub action: usize,
    pub label: Msg,
    pub enabled: bool,
    /// Phím tắt in bên phải (tên phím, giống nhau ở mọi ngôn ngữ).
    pub shortcut: Option<&'static str>,
    pub separator_before: bool,
}

/// Các mục menu theo bảng hành động: ẩn mục không áp dụng, vô hiệu mục không làm được, vạch ngăn trước
/// "Xóa khỏi lịch sử".
pub fn menu_items(a: &ActionSet) -> Vec<MenuItem> {
    let labels = [
        if a.open_is_link {
            Msg::HistoryOpenLink
        } else {
            Msg::HistoryOpen
        },
        Msg::HistoryCopyImage,
        Msg::HistoryCopyLink,
        Msg::HistoryShowInFolder,
        Msg::HistoryRemove,
    ];
    let keys = [
        Some("Enter"),
        Some("Ctrl+C"),
        Some("Ctrl+Shift+C"),
        None,
        Some("Del"),
    ];
    a.as_array()
        .into_iter()
        .enumerate()
        .filter(|(_, av)| *av != Avail::Hidden)
        .map(|(k, av)| MenuItem {
            action: k,
            label: labels[k],
            enabled: av == Avail::Enabled,
            shortcut: keys[k],
            separator_before: k == 4,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Thẻ xác nhận "Xóa toàn bộ lịch sử?" (thiết kế mục 3.7)
// ---------------------------------------------------------------------------

/// Hai nút của thẻ xác nhận.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ConfirmBtn {
    #[default]
    Cancel,
    Yes,
}

/// Phím thẻ xác nhận tự xử lý.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfirmKey {
    Esc,
    Enter,
    Tab,
    ShiftTab,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfirmOutcome {
    /// Thẻ vẫn mở (hoặc đang đóng sẵn).
    Stay,
    Cancel,
    Confirm,
}

/// Trạng thái thẻ xác nhận: mở/đóng và nút đang có tiêu điểm (Hủy là mặc định).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Confirm {
    open: bool,
    focus: ConfirmBtn,
}

impl Confirm {
    pub fn open(&mut self) {
        self.open = true;
        self.focus = ConfirmBtn::Cancel;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn focus(&self) -> ConfirmBtn {
        self.focus
    }

    pub fn set_focus(&mut self, b: ConfirmBtn) {
        self.focus = b;
    }

    /// Esc = Hủy; Enter = nút đang có tiêu điểm; Tab/Shift+Tab xoay giữa hai nút.
    pub fn key(&mut self, k: ConfirmKey) -> ConfirmOutcome {
        if !self.open {
            return ConfirmOutcome::Stay;
        }
        match k {
            ConfirmKey::Esc => {
                self.open = false;
                ConfirmOutcome::Cancel
            }
            ConfirmKey::Enter => {
                self.open = false;
                match self.focus {
                    ConfirmBtn::Cancel => ConfirmOutcome::Cancel,
                    ConfirmBtn::Yes => ConfirmOutcome::Confirm,
                }
            }
            ConfirmKey::Tab | ConfirmKey::ShiftTab => {
                self.focus = match self.focus {
                    ConfirmBtn::Cancel => ConfirmBtn::Yes,
                    ConfirmBtn::Yes => ConfirmBtn::Cancel,
                };
                ConfirmOutcome::Stay
            }
        }
    }
}

const CARD_W: i32 = 400;
const CARD_PAD: i32 = 24;
const CARD_TITLE_GAP: i32 = 8;
const CARD_BTN_TOP: i32 = 24;
pub const CARD_RADIUS: i32 = 8;

/// Vị trí thẻ xác nhận và các phần của nó (pixel, tọa độ client cửa sổ chính).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CardRects {
    pub card: RECT,
    pub title: RECT,
    pub text: RECT,
    pub cancel: RECT,
    pub yes: RECT,
}

/// Bề rộng đoạn giải thích của thẻ xác nhận (để đo chiều cao chữ xuống dòng).
pub fn card_text_width(dpi: u32) -> i32 {
    scale(CARD_W - 2 * CARD_PAD, dpi)
}

/// Thẻ rộng 400 DIP giữa thân; `title_h`, `text_h` = chiều cao chữ đã đo; `btn_w` = bề rộng nút Hủy và
/// nút xóa. Hai nút cùng hàng ở góc dưới phải, Hủy bên trái.
pub fn confirm_card(l: &Layout, title_h: i32, text_h: i32, btn_w: [i32; 2]) -> CardRects {
    let s = |v: i32| scale(v, l.dpi);
    let w = s(CARD_W);
    let h = 2 * s(CARD_PAD) + title_h + s(CARD_TITLE_GAP) + text_h + s(CARD_BTN_TOP) + s(BTN_H);
    let b = &l.body;
    let left = b.left + (b.right - b.left - w) / 2;
    let top = (b.top + (b.bottom - b.top - h) / 2).max(b.top);
    let card = rc(left, top, left + w, top + h);
    let (x0, x1) = (left + s(CARD_PAD), left + w - s(CARD_PAD));
    let title = rc(x0, top + s(CARD_PAD), x1, top + s(CARD_PAD) + title_h);
    let tt = title.bottom + s(CARD_TITLE_GAP);
    let text = rc(x0, tt, x1, tt + text_h);
    let bt = text.bottom + s(CARD_BTN_TOP);
    let yw = btn_w[1].min(x1 - x0);
    let yes = rc(x1 - yw, bt, x1, bt + s(BTN_H));
    let cw = btn_w[0].min((yes.left - s(BTN_GAP) - x0).max(0));
    let cancel = rc(
        yes.left - s(BTN_GAP) - cw,
        bt,
        yes.left - s(BTN_GAP),
        bt + s(BTN_H),
    );
    CardRects {
        card,
        title,
        text,
        cancel,
        yes,
    }
}

// ---------------------------------------------------------------------------
// Vùng xem trước, ô tìm
// ---------------------------------------------------------------------------

/// Cách vẽ thông báo giữa vùng xem trước: cỡ biểu tượng (0 = không vẽ) và có dòng gợi ý hay không.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MsgPlan {
    pub icon: i32,
    pub hint: bool,
}

const MSG_TITLE_H: i32 = 22;
const MSG_HINT_GAP: i32 = 6;

impl MsgPlan {
    /// Chiều cao cả khối (biểu tượng, tiêu đề, gợi ý cao `hint_h`).
    pub fn height(&self, hint_h: i32, dpi: u32) -> i32 {
        let s = |v: i32| scale(v, dpi);
        let icon = if self.icon > 0 {
            self.icon + if self.icon >= s(36) { s(12) } else { s(8) }
        } else {
            0
        };
        let hint = if self.hint {
            s(MSG_HINT_GAP) + hint_h
        } else {
            0
        };
        icon + s(MSG_TITLE_H) + hint
    }
}

/// Chọn cách vẽ vừa vùng cao `inner_h`: đủ chỗ thì biểu tượng 36 DIP và gợi ý; thiếu thì co biểu
/// tượng còn 24, rồi bỏ biểu tượng, rồi mới bỏ dòng gợi ý (chữ quan trọng hơn hình).
pub fn center_msg_plan(inner_h: i32, hint_h: i32, dpi: u32) -> MsgPlan {
    let s = |v: i32| scale(v, dpi);
    let plans = [
        MsgPlan {
            icon: s(36),
            hint: true,
        },
        MsgPlan {
            icon: s(24),
            hint: true,
        },
        MsgPlan {
            icon: 0,
            hint: true,
        },
        MsgPlan {
            icon: s(24),
            hint: false,
        },
    ];
    plans
        .into_iter()
        .find(|p| p.height(hint_h, dpi) <= inner_h)
        .unwrap_or(MsgPlan {
            icon: 0,
            hint: false,
        })
}

/// Các phần của ô tìm: kính lúp, ô EDIT con (không viền, căn giữa dọc), nút `×`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SearchParts {
    pub lens: RECT,
    pub edit: RECT,
    pub clear: RECT,
}

/// Xếp ô tìm `search`; `text_h` = chiều cao một dòng chữ của phông thường.
pub fn search_parts(search: &RECT, dpi: u32, text_h: i32) -> SearchParts {
    let s = |v: i32| scale(v, dpi);
    let cy = (search.top + search.bottom) / 2;
    let d = s(12);
    let lens = rc(
        search.left + s(10),
        cy - d / 2 - s(1),
        search.left + s(10) + d,
        cy + d / 2 - s(1),
    );
    let cs = s(24);
    let clear = rc(
        search.right - s(4) - cs,
        cy - cs / 2,
        search.right - s(4),
        cy - cs / 2 + cs,
    );
    let max_h = (search.bottom - search.top - 2 * s(3)).max(1);
    let eh = (text_h + s(2)).clamp(1, max_h);
    let top = cy - eh / 2;
    let edit = rc(lens.right + s(10), top, clear.left - s(2), top + eh);
    SearchParts { lens, edit, clear }
}

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
                    // Rộng tối đa 260 DIP (nới từ 220 để chữ gợi ý tiếng Việt hiện đủ).
                    assert!(sw >= scale(160, dpi) - 1 && sw <= scale(260, dpi) + 1);
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
    fn empty_state_block_is_centered_and_stacked() {
        for dpi in DPIS {
            for (w, h) in sizes(dpi) {
                let l = layout(w, h, dpi, false);
                let th = scale(40, dpi);
                let e = empty_state_rects(&l.body, dpi, th, Some(scale(140, dpi)));
                for r in [e.icon, e.title, e.body, e.button] {
                    assert!(inside(&r, &l.body), "{r:?} at {w}x{h}@{dpi}");
                }
                assert!(e.icon.bottom <= e.title.top);
                assert!(e.title.bottom <= e.body.top);
                assert!(e.body.bottom <= e.button.top);
                assert_eq!(e.body.bottom - e.body.top, th);
                let cx = (l.body.left + l.body.right) / 2;
                assert!(((e.button.left + e.button.right) / 2 - cx).abs() <= 1);
                assert!(e.body.right - e.body.left <= scale(420, dpi));
                let none = empty_state_rects(&l.body, dpi, th, None);
                assert_eq!(area(&none.button), 0);
            }
        }
    }

    #[test]
    fn row_subtitle_shows_dims_and_size() {
        let mut e = entry(1, Kind::Saved);
        e.bytes = Some(421_888);
        assert_eq!(row_subtitle(Lang::En, &e), "1920 × 1080 · 412 KB");
        e.bytes = None;
        assert_eq!(row_subtitle(Lang::En, &e), "1920 × 1080");
    }

    #[test]
    fn expiry_dates_follow_language() {
        assert_eq!(expires_text(Lang::Vi, "2026-12-18"), "18/12/2026");
        assert_eq!(expires_text(Lang::En, "2026-12-18"), "2026-12-18");
        assert_eq!(expires_text(Lang::Vi, "rác"), "rác");
    }

    // ---- Task 57: tìm, lọc, hành động, xác nhận ----

    use darkshot_core::history::Filter;

    #[test]
    fn chips_map_to_filters_and_back() {
        let want = [Filter::All, Filter::Saved, Filter::Uploaded, Filter::Copied];
        for (k, f) in want.iter().enumerate() {
            assert_eq!(chip_filter(k), Some(*f));
            assert_eq!(filter_chip(*f), k);
        }
        assert_eq!(chip_filter(4), None);
        assert_eq!(chip_filter(usize::MAX), None);
        // Chỉ viên đang chọn nằm trong vòng Tab.
        assert_eq!(chip_tabstops(Filter::All), [true, false, false, false]);
        assert_eq!(chip_tabstops(Filter::Copied), [false, false, false, true]);
        // Mũi tên trái/phải đổi bộ lọc, dừng ở hai đầu.
        assert_eq!(chip_step(Filter::All, 1), Filter::Saved);
        assert_eq!(chip_step(Filter::Saved, -1), Filter::All);
        assert_eq!(chip_step(Filter::All, -1), Filter::All);
        assert_eq!(chip_step(Filter::Copied, 1), Filter::Copied);
        assert_eq!(chip_step(Filter::Uploaded, 1), Filter::Copied);
    }

    fn sample() -> Vec<Entry> {
        let mut v = Vec::new();
        let mut mk = |n: u32, kind: Kind, file: Option<&str>, day: u8| {
            let mut e = entry(n, kind);
            e.local.day = day;
            e.path = file.map(|f| PathBuf::from(format!("C:\\Users\\Tuấn\\Ảnh\\{f}")));
            if kind == Kind::Uploaded {
                e.url = Some(format!("https://img.example.com/{n}/shot.png"));
            }
            v.push(e);
        };
        mk(0, Kind::Saved, Some("darkshot_meo.png"), 18);
        mk(1, Kind::Uploaded, None, 18);
        mk(2, Kind::Copied, None, 17);
        mk(3, Kind::UploadFallback, Some("darkshot_cho.png"), 16);
        mk(4, Kind::Saved, Some("bao cao.png"), 16);
        mk(5, Kind::Uploaded, None, 15);
        v
    }

    #[test]
    fn visible_follows_filter_and_query() {
        let e = sample();
        let v = |f, q: &str, lang| visible_indices(&e, f, q, lang);
        assert_eq!(v(Filter::All, "", Lang::Vi), vec![0, 1, 2, 3, 4, 5]);
        // Đã lưu = Lưu + Lưu dự phòng.
        assert_eq!(v(Filter::Saved, "", Lang::Vi), vec![0, 3, 4]);
        assert_eq!(v(Filter::Uploaded, "", Lang::Vi), vec![1, 5]);
        assert_eq!(v(Filter::Copied, "", Lang::Vi), vec![2]);
        // Tên tệp, không phân biệt hoa thường và dấu.
        assert_eq!(v(Filter::All, "MEO", Lang::Vi), vec![0]);
        assert_eq!(v(Filter::All, "darkshot", Lang::Vi), vec![0, 3]);
        assert_eq!(
            v(Filter::Uploaded, "darkshot", Lang::Vi),
            Vec::<usize>::new()
        );
        // Mọi từ phải khớp.
        assert_eq!(v(Filter::All, "darkshot cho", Lang::Vi), vec![3]);
        assert_eq!(v(Filter::All, "bao cao", Lang::Vi), vec![4]);
        // Ngày theo hai dạng.
        assert_eq!(v(Filter::All, "2026-11-16", Lang::En), vec![3, 4]);
        assert_eq!(v(Filter::All, "17/11/2026", Lang::Vi), vec![2]);
        assert_eq!(v(Filter::Saved, "2026-11-16", Lang::En), vec![3, 4]);
        // Liên kết.
        assert_eq!(v(Filter::All, "img.example.com/5", Lang::En), vec![5]);
        // Tên loại theo ngôn ngữ.
        let vi_label = Kind::Copied.label(Lang::Vi);
        assert_eq!(v(Filter::All, vi_label, Lang::Vi), vec![2]);
        let en_label = Kind::Copied.label(Lang::En);
        assert_eq!(v(Filter::All, en_label, Lang::En), vec![2]);
        // Khoảng trắng thừa không đổi kết quả; không khớp thì rỗng.
        assert_eq!(v(Filter::All, "   ", Lang::Vi), vec![0, 1, 2, 3, 4, 5]);
        assert_eq!(v(Filter::All, "zzzz", Lang::Vi), Vec::<usize>::new());
        assert_eq!(
            visible_indices(&[], Filter::All, "", Lang::Vi),
            Vec::<usize>::new()
        );
    }

    #[test]
    fn selection_is_kept_when_filtering() {
        // Mục chọn còn trong kết quả: giữ, không cuộn về đầu.
        let r = after_filter(Some("c"), &["a", "c", "d"]);
        assert_eq!(
            r,
            Refiltered {
                selected: Some(1),
                scroll_top: false
            }
        );
        // Mục chọn rời khỏi danh sách: chọn mục đầu, cuộn về đầu.
        let r = after_filter(Some("b"), &["a", "c"]);
        assert_eq!(
            r,
            Refiltered {
                selected: Some(0),
                scroll_top: true
            }
        );
        // Chưa chọn gì, danh sách khác rỗng: chọn mục đầu.
        assert_eq!(after_filter(None, &["x"]).selected, Some(0));
        // Danh sách rỗng: không chọn gì.
        let r = after_filter(Some("b"), &[]);
        assert_eq!(r.selected, None);
        assert!(r.scroll_top);
        assert_eq!(after_filter(None, &[]).selected, None);
    }

    #[test]
    fn next_selection_after_remove() {
        let old = ["a", "b", "c", "d"];
        // Đầu: mục kế (cùng vị trí 0).
        assert_eq!(select_after_remove(&old, "a", &["b", "c", "d"]), Some(0));
        // Giữa: cùng vị trí.
        assert_eq!(select_after_remove(&old, "b", &["a", "c", "d"]), Some(1));
        // Cuối: mục cuối mới.
        assert_eq!(select_after_remove(&old, "d", &["a", "b", "c"]), Some(2));
        // Mục cuối cùng còn lại: không còn gì để chọn.
        assert_eq!(select_after_remove(&["a"], "a", &[]), None);
    }

    #[test]
    fn action_table_rejects_non_https_links() {
        use Avail::*;
        for bad in [
            "http://x.example/a.png",
            "file:///C:/a.png",
            "javascript:alert(1)",
            "",
            "https://",
            "https://x.example/a\u{7}.png",
            "https://x.example/\na.png",
            "HTTPS://x.example/a.png",
            " https://x.example/a.png",
        ] {
            assert!(!url_openable(bad), "{bad:?}");
            let mut e = entry(3, Kind::Uploaded);
            e.url = Some(bad.to_string());
            let a = actions_for(&e, true);
            assert_eq!(a.open, Disabled, "{bad:?}");
            // Liên kết vẫn chép được (trừ chuỗi rỗng: không có gì để chép).
            let want = if bad.is_empty() { Hidden } else { Enabled };
            assert_eq!(a.copy_link, want, "{bad:?}");
        }
        for good in [
            "https://x.example/a.png",
            "https://pub-1.r2.dev/darkshot/2026/11/a%20b.png?X-Amz-Expires=3600&X-Amz-Signature=ab",
        ] {
            assert!(url_openable(good), "{good}");
            let mut e = entry(4, Kind::Uploaded);
            e.url = Some(good.to_string());
            assert_eq!(actions_for(&e, false).open, Enabled);
        }
    }

    #[test]
    fn explorer_select_args_quote_paths() {
        use std::path::Path;
        assert_eq!(
            explorer_select_args(Path::new("C:\\Users\\Tuấn\\Ảnh chụp\\a b.png")).as_deref(),
            Some("/select,\"C:\\Users\\Tuấn\\Ảnh chụp\\a b.png\"")
        );
        assert_eq!(
            explorer_select_args(Path::new("D:\\x,y\\日本.png")).as_deref(),
            Some("/select,\"D:\\x,y\\日本.png\"")
        );
        assert_eq!(
            explorer_select_args(Path::new("\\\\server\\share\\a.png")).as_deref(),
            Some("/select,\"\\\\server\\share\\a.png\"")
        );
        // Từ chối: có dấu ngoặc kép, ký tự điều khiển, đường dẫn tương đối hoặc rỗng.
        for bad in [
            "C:\\a\"b.png",
            "C:\\a\nb.png",
            "C:\\a\u{1}b.png",
            "a\\b.png",
            "\\a.png",
            "C:a.png",
            "",
        ] {
            assert_eq!(explorer_select_args(Path::new(bad)), None, "{bad:?}");
        }
    }

    #[test]
    fn explorer_runs_from_windows_directory() {
        assert_eq!(
            explorer_exe(Some("C:\\Windows")),
            "C:\\Windows\\explorer.exe"
        );
        assert_eq!(explorer_exe(Some("D:\\WIN\\")), "D:\\WIN\\explorer.exe");
        // Không biết thư mục Windows (hoặc giá trị lạ): dự phòng tên trần.
        assert_eq!(explorer_exe(None), "explorer.exe");
        assert_eq!(explorer_exe(Some("")), "explorer.exe");
        assert_eq!(explorer_exe(Some("Windows")), "explorer.exe");
        assert_eq!(explorer_exe(Some("C:\\W\"in")), "explorer.exe");
    }

    #[test]
    fn chips_treat_up_down_like_left_right() {
        // Lên = trái, Xuống = phải (như nhóm nút radio).
        assert_eq!(chip_key_dir(ChipKey::Up), -1);
        assert_eq!(chip_key_dir(ChipKey::Left), -1);
        assert_eq!(chip_key_dir(ChipKey::Down), 1);
        assert_eq!(chip_key_dir(ChipKey::Right), 1);
    }

    #[test]
    fn vietnamese_search_hint_fits_at_default_size() {
        let hint =
            darkshot_core::i18n::t(Lang::Vi, darkshot_core::i18n::Msg::HistorySearchPlaceholder);
        for dpi in DPIS {
            let (w, h) = crate::history_window::default_client_size(dpi);
            let l = layout(w, h, dpi, false);
            let font = crate::winutil::system_font(dpi);
            let (tw, th) = crate::history_paint::with_screen_dc(|dc| {
                crate::history_paint::measure(dc, font, hint)
            });
            if let Some(f) = font {
                // SAFETY: phông vừa tạo cho kiểm thử, không còn dùng.
                unsafe {
                    let _ = windows::Win32::Graphics::Gdi::DeleteObject(
                        windows::Win32::Graphics::Gdi::HGDIOBJ(f.0),
                    );
                }
            }
            let p = search_parts(&l.search, dpi, th);
            // Ô trống: ô EDIT kéo tới gần nút `×` (đang ẩn), trừ 4 DIP.
            let room = p.clear.right - scale(4, dpi) - p.edit.left;
            assert!(tw <= room, "hint {tw} > {room} @{dpi}");
        }
    }

    #[test]
    fn confirm_card_state_machine() {
        use ConfirmBtn::*;
        use ConfirmKey::*;
        let mut c = Confirm::default();
        assert!(!c.is_open());
        // Đóng thì phím không làm gì.
        assert_eq!(c.key(Enter), ConfirmOutcome::Stay);
        assert_eq!(c.key(Esc), ConfirmOutcome::Stay);
        c.open();
        assert!(c.is_open());
        assert_eq!(c.focus(), Cancel, "Hủy là mặc định");
        // Tab xoay giữa hai nút, hai chiều.
        assert_eq!(c.key(Tab), ConfirmOutcome::Stay);
        assert_eq!(c.focus(), Yes);
        assert_eq!(c.key(Tab), ConfirmOutcome::Stay);
        assert_eq!(c.focus(), Cancel);
        assert_eq!(c.key(ShiftTab), ConfirmOutcome::Stay);
        assert_eq!(c.focus(), Yes);
        assert_eq!(c.key(ShiftTab), ConfirmOutcome::Stay);
        assert_eq!(c.focus(), Cancel);
        // Enter trên Hủy: đóng, không xóa.
        assert_eq!(c.key(Enter), ConfirmOutcome::Cancel);
        assert!(!c.is_open());
        // Esc = Hủy, dù tiêu điểm ở nút xóa.
        c.open();
        c.set_focus(Yes);
        assert_eq!(c.key(Esc), ConfirmOutcome::Cancel);
        assert!(!c.is_open());
        // Mở lại: tiêu điểm về Hủy.
        c.open();
        assert_eq!(c.focus(), Cancel);
        c.key(Tab);
        assert_eq!(c.key(Enter), ConfirmOutcome::Confirm);
        assert!(!c.is_open());
        c.open();
        c.close();
        assert!(!c.is_open());
    }

    #[test]
    fn confirm_body_text_for_one_and_fifty() {
        use darkshot_core::i18n::history_clear_confirm_body as body;
        assert_eq!(
            body(Lang::En, 1),
            "1 entry will be removed from history. Screenshots you saved and links you uploaded are not deleted."
        );
        assert_eq!(
            body(Lang::En, 50),
            "50 entries will be removed from history. Screenshots you saved and links you uploaded are not deleted."
        );
        assert!(body(Lang::Vi, 1).starts_with("1 mục sẽ bị gỡ khỏi lịch sử."));
        assert!(body(Lang::Vi, 50).starts_with("50 mục sẽ bị gỡ khỏi lịch sử."));
    }

    #[test]
    fn confirm_card_is_centered_and_fits() {
        for dpi in DPIS {
            for (w, h) in sizes(dpi) {
                for banner in [false, true] {
                    let l = layout(w, h, dpi, banner);
                    let c = confirm_card(
                        &l,
                        scale(26, dpi),
                        scale(40, dpi),
                        [scale(72, dpi), scale(120, dpi)],
                    );
                    assert_eq!(c.card.right - c.card.left, scale(400, dpi), "{w}x{h}@{dpi}");
                    assert!(inside(&c.card, &l.body), "{:?} at {w}x{h}@{dpi}", c.card);
                    let cx = (l.body.left + l.body.right) / 2;
                    assert!(((c.card.left + c.card.right) / 2 - cx).abs() <= 1);
                    for r in [c.title, c.text, c.cancel, c.yes] {
                        assert!(inside(&r, &c.card), "{r:?} at {w}x{h}@{dpi}");
                    }
                    assert!(c.title.bottom <= c.text.top);
                    assert!(c.text.bottom <= c.cancel.top);
                    assert!(!overlap(&c.cancel, &c.yes));
                    // Hủy bên trái nút xóa, cùng hàng, nút xóa sát lề phải trong thẻ.
                    assert!(c.cancel.right <= c.yes.left);
                    assert_eq!(c.cancel.top, c.yes.top);
                    assert_eq!(c.yes.bottom - c.yes.top, scale(32, dpi));
                    assert_eq!(c.text.bottom - c.text.top, scale(40, dpi));
                }
            }
        }
    }

    #[test]
    fn preview_message_shrinks_to_fit() {
        for dpi in DPIS {
            let s = |v| scale(v, dpi);
            // Đủ chỗ: biểu tượng đầy đủ và dòng gợi ý.
            let p = center_msg_plan(s(300), s(34), dpi);
            assert_eq!(
                p,
                MsgPlan {
                    icon: s(36),
                    hint: true
                }
            );
            assert!(p.height(s(34), dpi) <= s(300));
            // Mọi chiều cao: khối luôn nằm trọn, ưu tiên bỏ biểu tượng lớn rồi dòng gợi ý.
            for inner in 0..s(300) {
                let p = center_msg_plan(inner, s(34), dpi);
                let need = p.height(s(34), dpi);
                assert!(
                    need <= inner.max(s(22)),
                    "inner {inner}@{dpi}: {p:?} needs {need}"
                );
                // Chữ được ưu tiên hơn biểu tượng: chỉ bỏ dòng gợi ý khi không biểu tượng cũng không vừa.
                if !p.hint {
                    assert!(s(22) + s(6) + s(34) > inner, "{inner}@{dpi}: {p:?}");
                }
                assert!(p.icon == 0 || p.icon == s(36) || p.icon == s(24));
            }
            // Cỡ tối thiểu 760×520 (thực tế, có và không có dải thông báo): chữ không tràn.
            for banner in [false, true] {
                let (w, h) = crate::history_window::min_client_size(dpi);
                let l = layout(w, h, dpi, banner);
                let inner = l.preview_area.bottom - l.preview_area.top - 2 * s(16);
                let p = center_msg_plan(inner, s(36), dpi);
                assert!(
                    p.height(s(36), dpi) <= inner,
                    "{dpi} banner={banner}: {p:?}"
                );
            }
        }
    }

    #[test]
    fn search_box_parts_fit_inside_field() {
        for dpi in DPIS {
            for (w, h) in sizes(dpi) {
                let l = layout(w, h, dpi, false);
                let th = scale(16, dpi);
                let p = search_parts(&l.search, dpi, th);
                for r in [p.lens, p.edit, p.clear] {
                    assert!(inside(&r, &l.search), "{r:?} at {w}x{h}@{dpi}");
                }
                assert!(p.lens.right <= p.edit.left);
                assert!(p.edit.right <= p.clear.left);
                assert!(p.edit.bottom - p.edit.top >= th);
                assert!(p.edit.right - p.edit.left >= scale(80, dpi));
                // Căn giữa dọc.
                let cy = (l.search.top + l.search.bottom) / 2;
                assert!(((p.edit.top + p.edit.bottom) / 2 - cy).abs() <= 1);
            }
        }
    }

    #[test]
    fn status_codes_round_trip() {
        for s in [
            Status::None,
            Status::ImageCopied,
            Status::LinkCopied,
            Status::Removed,
            Status::Cleared,
            Status::Failed,
        ] {
            assert_eq!(Status::from_wparam(s as usize), s);
        }
        assert_eq!(Status::from_wparam(999), Status::None);
        assert_eq!(Status::None.msg(), None);
        assert_eq!(
            Status::ImageCopied.msg(),
            Some(darkshot_core::i18n::Msg::HistoryStatusImageCopied)
        );
        assert!(Status::Failed.is_error());
        assert!(!Status::Removed.is_error());
    }

    #[test]
    fn context_menu_follows_action_table() {
        use darkshot_core::i18n::Msg;
        // Đã lưu, còn tệp: Mở, Sao chép ảnh, Mở thư mục, vạch, Xóa (không có Sao chép liên kết).
        let m = menu_items(&actions_for(&entry(1, Kind::Saved), true));
        let ids: Vec<usize> = m.iter().map(|i| i.action).collect();
        assert_eq!(ids, vec![0, 1, 3, 4]);
        assert!(m.iter().all(|i| i.enabled));
        assert_eq!(m[0].label, Msg::HistoryOpen);
        assert_eq!(m[0].shortcut, Some("Enter"));
        assert_eq!(m[1].shortcut, Some("Ctrl+C"));
        assert_eq!(m[2].shortcut, None);
        assert_eq!(m[3].shortcut, Some("Del"));
        assert!(m[3].separator_before);
        assert!(!m[0].separator_before && !m[1].separator_before && !m[2].separator_before);
        // Tệp mất: Mở, Sao chép ảnh, Mở thư mục vô hiệu; Xóa vẫn dùng được.
        let m = menu_items(&actions_for(&entry(1, Kind::Saved), false));
        let en: Vec<bool> = m.iter().map(|i| i.enabled).collect();
        assert_eq!(en, vec![false, false, false, true]);
        // Tải lên: Mở liên kết, Sao chép liên kết có Ctrl+Shift+C.
        let m = menu_items(&actions_for(&entry(1, Kind::Uploaded), true));
        let ids: Vec<usize> = m.iter().map(|i| i.action).collect();
        assert_eq!(ids, vec![0, 1, 2, 4]);
        assert_eq!(m[0].label, Msg::HistoryOpenLink);
        assert_eq!(m[2].label, Msg::HistoryCopyLink);
        assert_eq!(m[2].shortcut, Some("Ctrl+Shift+C"));
    }
}

#[cfg(test)]
mod open_guard_tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn only_absolute_png_files_open() {
        assert!(user_png_openable(Path::new(r"C:\pics\a.png")));
        assert!(user_png_openable(Path::new(r"C:\pics\A.PNG")));
        assert!(!user_png_openable(Path::new(r"C:\pics\a.exe")));
        assert!(!user_png_openable(Path::new(r"C:\pics\a.lnk")));
        assert!(!user_png_openable(Path::new(r"C:\pics\a")));
        assert!(!user_png_openable(Path::new("a.png")));
        assert!(!user_png_openable(Path::new(r"\a.png")));
    }
}
