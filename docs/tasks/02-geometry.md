# Task 2: Hình học vùng chọn

**Agent:** `builder` · **Model:** sonnet · **Nhánh:** `task/02-geometry`

Gói task tự chứa. Chỉ đọc file này và các file ở mục "Cần đọc".

## Cần đọc

- `memory-bank/systemPatterns.md`
- `memory-bank/techContext.md`
- docs/superpowers/specs/2026-11-16-shotme-design.md mục 5.2
- `crates/shotme-core/CLAUDE.md`

## Global Constraints

- Chỉ mở kết nối mạng đến hai nơi: `https://{account_id}.r2.cloudflarestorage.com` và địa chỉ công khai của kho do người dùng nhập. Không thống kê, không kiểm tra cập nhật, không báo lỗi tự động.
- HTTPS dùng `rustls` với `webpki-roots`. Không dùng `native-tls`, không dùng bộ kiểm chứng chứng chỉ của hệ điều hành.
- Secret Access Key không bao giờ xuất hiện ở dạng đọc được trên đĩa, trong log, trong thông báo lỗi hay trong đầu ra kiểm thử.
- Không ghi log ra file.
- `shotme-core` không được phụ thuộc crate `windows` hay bất kỳ API riêng của hệ điều hành nào. `cargo tree -p shotme-core` không chứa `windows`.
- Mọi trait nền tảng trong core có ràng buộc `Send + Sync`.
- Giao diện và thông báo bằng tiếng Việt. Câu cố định: tải lên thành công báo "Đã chép link".
- Tên ảnh trên kho: 16 ký tự `[A-Za-z0-9]` từ bộ sinh số ngẫu nhiên của hệ điều hành, không lệch phân phối.
- Tên file local: `shotme_YYYY-MM-DD_HH-MM-SS.png`, trùng thì thêm `_2`, `_3`.
- Thư mục hạn trên kho: `d1`, `d7`, `d30`, `keep`. Mặc định `d30`.
- Phím tắt mặc định: `PrintScreen` (VK `0x2C`), không phím bổ trợ.
- File cấu hình: `%APPDATA%\shotme\config.json`. Thư mục lưu mặc định: `Pictures\shotme`.
- Không xây tính năng ngoài phạm vi của spec (vẽ lên ảnh, lịch sử, xóa từ ứng dụng).
- `cargo test --workspace` và `cargo clippy --workspace -- -D warnings` phải qua trước khi gộp nhánh.

## Nội dung task
**Files:**
- Create: `crates/shotme-core/src/geometry.rs`
- Modify: `crates/shotme-core/src/lib.rs` (thêm `pub mod geometry;`)

**Interfaces:**
- Produces:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub struct Point { pub x: i32, pub y: i32 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub struct Rect { pub x: i32, pub y: i32, pub w: i32, pub h: i32 }
impl Rect {
    pub fn from_points(a: Point, b: Point) -> Rect;   // w, h luôn >= 0
    pub fn contains(&self, p: Point) -> bool;         // cạnh phải và dưới không tính
    pub fn right(&self) -> i32; pub fn bottom(&self) -> i32;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Handle { N, S, E, W, NE, NW, SE, SW }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Hit { Outside, Inside, Handle(Handle) }
pub struct Selection { /* riêng tư */ }
impl Selection {
    pub fn new(bounds: Rect) -> Self;                 // bounds: toàn lớp phủ, gốc (0,0)
    pub fn rect(&self) -> Option<Rect>;
    pub fn is_dragging(&self) -> bool;
    pub fn hit_test(&self, p: Point, handle_radius: i32) -> Hit;
    pub fn press(&mut self, p: Point, handle_radius: i32);
    pub fn drag_to(&mut self, p: Point);
    pub fn release(&mut self);
}
pub fn monitor_containing(p: Point, monitors: &[Rect]) -> Rect;  // không màn hình nào chứa p thì trả màn hình gần nhất
pub fn place_toolbar(sel: Rect, size: (i32, i32), gap: i32, monitor: Rect) -> Rect;
```

Quy tắc hành vi (spec 5.2):
- `press` trên tay nắm: bắt đầu đổi kích thước. Trong khung: bắt đầu di chuyển. Ngoài khung hoặc chưa có khung: bắt đầu khung mới tại `p`.
- `drag_to`: điểm bị kẹp vào `bounds`. Di chuyển giữ nguyên kích thước và không ra ngoài `bounds`. Kéo tay nắm vượt qua cạnh đối diện thì khung lật lại, `w` và `h` không âm.
- `release`: khung nhỏ hơn 3×3 thì `rect()` thành `None`.
- `place_toolbar`: ưu tiên dưới khung, căn phải với khung, cách `gap`. Không đủ chỗ trong `monitor` thì đặt trên khung. Vẫn không đủ thì đặt trong khung, góc dưới phải, cách mép `gap`. Kết quả luôn nằm trọn trong `monitor` (kẹp theo chiều ngang nếu cần).

- [ ] **Step 1: Viết kiểm thử thất bại** trong `geometry.rs` (`#[cfg(test)]`):

```rust
const B: Rect = Rect { x: 0, y: 0, w: 1000, h: 800 };
fn p(x: i32, y: i32) -> Point { Point { x, y } }

#[test] fn new_selection_from_drag() {
    let mut s = Selection::new(B);
    s.press(p(300, 200), 6); s.drag_to(p(100, 50)); s.release();
    assert_eq!(s.rect(), Some(Rect { x: 100, y: 50, w: 200, h: 150 }));
}
#[test] fn tiny_selection_is_discarded() {
    let mut s = Selection::new(B);
    s.press(p(10, 10), 6); s.drag_to(p(12, 12)); s.release();
    assert_eq!(s.rect(), None);
}
#[test] fn drag_is_clamped_to_bounds() {
    let mut s = Selection::new(B);
    s.press(p(900, 700), 6); s.drag_to(p(5000, -40)); s.release();
    assert_eq!(s.rect(), Some(Rect { x: 900, y: 0, w: 100, h: 700 }));
}
#[test] fn move_keeps_size_and_stays_inside() {
    let mut s = sel(100, 100, 200, 100);            // helper: tạo khung bằng press/drag/release
    s.press(p(150, 150), 6); s.drag_to(p(2000, 2000)); s.release();
    assert_eq!(s.rect(), Some(Rect { x: 800, y: 700, w: 200, h: 100 }));
}
#[test] fn resize_east_handle_and_flip() {
    let mut s = sel(100, 100, 200, 100);
    assert_eq!(s.hit_test(p(300, 150), 6), Hit::Handle(Handle::E));
    s.press(p(300, 150), 6); s.drag_to(p(40, 150)); s.release();
    assert_eq!(s.rect(), Some(Rect { x: 40, y: 100, w: 60, h: 100 }));
}
#[test] fn press_outside_starts_new_selection() {
    let mut s = sel(100, 100, 200, 100);
    s.press(p(500, 500), 6); s.drag_to(p(600, 650)); s.release();
    assert_eq!(s.rect(), Some(Rect { x: 500, y: 500, w: 100, h: 150 }));
}
#[test] fn hit_test_corners_inside_outside() {
    let s = sel(100, 100, 200, 100);
    assert_eq!(s.hit_test(p(100, 100), 6), Hit::Handle(Handle::NW));
    assert_eq!(s.hit_test(p(200, 150), 6), Hit::Inside);
    assert_eq!(s.hit_test(p(50, 50), 6), Hit::Outside);
}
#[test] fn toolbar_below_right_aligned() {
    assert_eq!(place_toolbar(Rect { x: 100, y: 100, w: 300, h: 200 }, (160, 40), 6, B),
               Rect { x: 240, y: 306, w: 160, h: 40 });
}
#[test] fn toolbar_above_when_no_room_below() {
    assert_eq!(place_toolbar(Rect { x: 100, y: 500, w: 300, h: 290 }, (160, 40), 6, B),
               Rect { x: 240, y: 454, w: 160, h: 40 });
}
#[test] fn toolbar_inside_when_selection_fills_monitor() {
    assert_eq!(place_toolbar(B, (160, 40), 6, B), Rect { x: 834, y: 754, w: 160, h: 40 });
}
#[test] fn toolbar_clamped_horizontally_for_narrow_selection() {
    let r = place_toolbar(Rect { x: 10, y: 100, w: 50, h: 50 }, (160, 40), 6, B);
    assert_eq!(r, Rect { x: 0, y: 156, w: 160, h: 40 });
}
#[test] fn toolbar_stays_in_second_monitor_with_offset_origin() {   // Review Focus 5
    let m2 = Rect { x: 1000, y: 200, w: 800, h: 600 };
    let r = place_toolbar(Rect { x: 1100, y: 600, w: 300, h: 190 }, (160, 40), 6, m2);
    assert_eq!(r, Rect { x: 1240, y: 554, w: 160, h: 40 });
}
#[test] fn monitor_containing_picks_nearest_when_in_gap() {         // Review Focus 5
    let ms = [Rect { x: 0, y: 0, w: 1000, h: 800 }, Rect { x: 1000, y: 200, w: 800, h: 600 }];
    assert_eq!(monitor_containing(p(1500, 300), &ms), ms[1]);
    assert_eq!(monitor_containing(p(1500, 50), &ms), ms[1]);       // vùng trống phía trên màn hình 2
}
```

- [ ] **Step 2:** Chạy `cargo test -p shotme-core geometry`. Mong đợi: lỗi biên dịch, các kiểu chưa tồn tại.
- [ ] **Step 3:** Hiện thực các kiểu và hàm ở mục Interfaces.
- [ ] **Step 4:** Chạy lại. Mong đợi: 13 test PASS.
- [ ] **Step 5:** Commit.
