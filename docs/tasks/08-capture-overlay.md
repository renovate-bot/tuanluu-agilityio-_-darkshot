# Task 8: Chụp màn hình và lớp phủ

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/08-capture-overlay`

Gói task tự chứa. Chỉ đọc file này và các file ở mục "Cần đọc".

## Cần đọc

- `memory-bank/systemPatterns.md`
- `memory-bank/techContext.md`
- docs/superpowers/specs/2026-11-16-shotme-design.md mục 5.2, 5.3
- `crates/shotme-windows/CLAUDE.md`
- prototype/index.html (hình mẫu giao diện, không dùng lại mã)
- `crates/shotme-core/src/geometry.rs`
- `crates/shotme-core/src/image.rs`
- `crates/shotme-core/src/actions.rs`
- `crates/shotme-windows/src/main.rs`
- `crates/shotme-windows/src/tray.rs`

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
- Create: `crates/shotme-windows/src/capture.rs`, `crates/shotme-windows/src/overlay.rs`
- Modify: `crates/shotme-windows/src/main.rs` (khai báo module), `crates/shotme-windows/Cargo.toml`

**Interfaces:**
- Consumes: `shotme_core::geometry::*`, `shotme_core::image::Image`, `shotme_core::actions::Action`.
- Produces:

```rust
// capture.rs
pub struct Screenshot {
    pub image: Image,            // toàn bộ màn hình ảo, pixel vật lý
    pub origin: Point,           // góc trên trái của màn hình ảo trong tọa độ màn hình (có thể âm)
    pub monitors: Vec<Rect>,     // từng màn hình, đã đổi sang tọa độ ảnh (trừ origin)
}
pub fn capture_virtual_screen() -> windows::core::Result<Screenshot>;
pub fn to_image_coords(screen: Rect, origin: Point) -> Rect;
// overlay.rs
pub fn pick_region(shot: &Screenshot) -> Option<(Action, Rect)>;   // Rect trong tọa độ ảnh; None khi hủy
```

Quyết định kỹ thuật:
- Chụp bằng `BitBlt` (có `CAPTUREBLT`) từ DC màn hình vào DIB 32 bit từ trên xuống, kích thước lấy từ `SM_XVIRTUALSCREEN`, `SM_YVIRTUALSCREEN`, `SM_CXVIRTUALSCREEN`, `SM_CYVIRTUALSCREEN`. Con trỏ chuột không có trong ảnh.
- Lớp phủ là một cửa sổ `WS_POPUP`, `WS_EX_TOPMOST | WS_EX_TOOLWINDOW`, phủ đúng màn hình ảo. `pick_region` chạy vòng lặp thông điệp riêng cho đến khi có kết quả.
- Vẽ hai lớp đệm chuẩn bị sẵn: ảnh gốc và ảnh đã làm tối 50%. Mỗi lần vẽ: ảnh tối toàn màn hình, ảnh gốc trong khung chọn, viền khung, 8 tay nắm, nhãn kích thước `{w} × {h}`, thanh công cụ. Vẽ qua DC bộ nhớ để không nháy.
- Toàn bộ trạng thái kéo thả dùng `geometry::Selection`; vị trí thanh công cụ dùng `place_toolbar` với `monitor_containing(góc dưới phải của khung, monitors)`.
- Thanh công cụ do lớp phủ tự vẽ: bốn nút vuông chỉ có biểu tượng theo thứ tự tải lên, lưu, chép, hủy. Rê chuột vào nút hiện chú thích: "Tải lên và chép link (Ctrl+D)", "Lưu vào máy (Ctrl+S)", "Chép ảnh (Ctrl+C)", "Hủy (Esc)". Kích thước nút 36 pixel nhân với tỉ lệ DPI của màn hình chứa nó. Ẩn thanh công cụ trong lúc đang kéo khung mới.
- Con trỏ: chữ thập ngoài khung, mũi tên bốn hướng trong khung, mũi tên đổi kích thước phù hợp trên tay nắm, mũi tên thường trên thanh công cụ.
- Phím: `Esc` hủy ở mọi lúc. `Ctrl+D`, `Ctrl+S`, `Ctrl+C` chỉ có tác dụng khi đã có khung. Mất tiêu điểm (ví dụ Alt+Tab) thì hủy.
- Kết quả: `(Action, khung chọn)`; với `Cancel` trả `None`.

- [ ] **Step 1: Viết kiểm thử thất bại:**

```rust
#[test] fn image_coords_with_negative_origin() {                // Review Focus 5
    let r = to_image_coords(Rect { x: -1920, y: -200, w: 1920, h: 1080 }, Point { x: -1920, y: -200 });
    assert_eq!(r, Rect { x: 0, y: 0, w: 1920, h: 1080 });
    let r = to_image_coords(Rect { x: 0, y: 0, w: 2560, h: 1440 }, Point { x: -1920, y: -200 });
    assert_eq!(r, Rect { x: 1920, y: 200, w: 2560, h: 1440 });
}
#[test] fn capture_matches_virtual_screen() {
    let s = capture_virtual_screen().unwrap();
    assert!(s.image.width > 0 && s.image.height > 0);
    assert_eq!(s.image.bgra.len(), (s.image.width * s.image.height * 4) as usize);
    assert!(!s.monitors.is_empty());
    // mọi màn hình nằm trọn trong ảnh
    assert!(s.monitors.iter().all(|m| m.x >= 0 && m.y >= 0
        && m.right() <= s.image.width as i32 && m.bottom() <= s.image.height as i32));
}
#[test] fn toolbar_button_hit_testing() {
    // hàm thuần button_at(toolbar: Rect, p: Point) -> Option<Action>:
    // thanh 144x36 tại (100,100): điểm (110,110) => Upload, (150,110) => Save, (190,110) => Copy, (230,110) => Cancel, (300,110) => None
}
```

- [ ] **Step 2:** Chạy `cargo test -p shotme-windows capture overlay`. Mong đợi: lỗi biên dịch.
- [ ] **Step 3:** Hiện thực `capture.rs`.
- [ ] **Step 4:** Hiện thực `overlay.rs`.
- [ ] **Step 5:** Chạy lại (3 test PASS).
- [ ] **Step 6:** Nối tạm vào `main.rs`: sự kiện Capture gọi `capture_virtual_screen` rồi `pick_region`, hiện thông báo Info "Đã chọn {w} × {h}, hành động {tên}". Bấm phím tắt khi lớp phủ đang mở thì bỏ qua.
- [ ] **Step 7:** Thêm vào `docs/testing/manual-checklist.md` mục "Lớp phủ": màn hình đứng yên và tối đi; kéo chọn, đổi kích thước 8 hướng, di chuyển, chọn lại; nhãn kích thước đúng; thanh công cụ dưới, trên, trong khung; chú thích khi rê chuột; bốn phím tắt; kéo qua hai màn hình; màn hình phụ bên trái màn hình chính; hai màn hình khác tỉ lệ phóng to; ảnh không bị mờ hay co giãn.
- [ ] **Step 8:** Commit.

**Điểm dừng cho chủ dự án:** sau khi gộp task này, chủ dự án chạy `shotme.exe` và thử mục "Lớp phủ" của danh sách kiểm tra. Phản hồi được xử lý trước Task 11.
