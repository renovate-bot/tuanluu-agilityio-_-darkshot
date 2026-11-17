//! Lịch sử ảnh chụp: phần thuần của cửa sổ (bố cục theo DPI, chạm, cuộn, bảng hành động, đệm ảnh nhỏ,
//! mã thế hệ, giữ mục chọn). Không gọi hệ điều hành nên kiểm thử được.
//!
//! Mọi số đo là DIP (thiết kế mục 3.2–3.5) và được quy đổi sang pixel theo `dpi`.

use crate::winutil::scale;
use darkshot_core::history::{format_dims, format_size, Entry, Kind};
use darkshot_core::i18n::Lang;
use darkshot_core::image::Image;
use darkshot_core::theme::Rgb;
use darkshot_core::thumb::fit_within;
use std::collections::{HashMap, VecDeque};
use std::ops::Range;
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
/// Ô tìm: cao 32, rộng 220 co xuống 160.
const SEARCH_H: i32 = 32;
const SEARCH_MIN: i32 = 160;
const SEARCH_MAX: i32 = 220;
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
            open: file,
            open_is_link: false,
            copy_image: file,
            copy_link: Avail::Hidden,
            show_folder: file,
            remove: Avail::Enabled,
        },
        Kind::Uploaded => ActionSet {
            open: if e.url.as_deref().is_some_and(|u| u.starts_with("https://")) {
                Avail::Enabled
            } else {
                Avail::Disabled
            },
            open_is_link: true,
            copy_image: file,
            copy_link: if e.url.is_some() {
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
}
