# Task 9: Dịch vụ Windows: khóa bí mật, clipboard, khởi động cùng hệ thống

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/09-win-services`

Gói task tự chứa. Chỉ đọc file này và các file ở mục "Cần đọc".

## Cần đọc

- `memory-bank/systemPatterns.md`
- `memory-bank/techContext.md`
- docs/superpowers/specs/2026-11-16-shotme-design.md mục 6
- `crates/shotme-windows/CLAUDE.md`
- `crates/shotme-core/src/config.rs`
- `crates/shotme-core/src/actions.rs`
- `crates/shotme-core/src/image.rs`

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
- Create: `crates/shotme-windows/src/{secret.rs,clipboard.rs,autostart.rs}`
- Modify: `crates/shotme-windows/src/main.rs` (khai báo module), `crates/shotme-windows/Cargo.toml`

**Interfaces:**
- Consumes: `shotme_core::config::SecretStore`, `shotme_core::actions::Clipboard`, `shotme_core::image::Image`.
- Produces:

```rust
pub struct DpapiStore;                         // secret.rs
impl SecretStore for DpapiStore { /* CryptProtectData / CryptUnprotectData, phạm vi người dùng hiện tại */ }
pub struct WinClipboard;                       // clipboard.rs
impl Clipboard for WinClipboard { /* set_text: CF_UNICODETEXT; set_image: CF_DIB 24 hoặc 32 bit */ }
pub fn is_enabled() -> bool;                   // autostart.rs
pub fn set_enabled(on: bool) -> Result<(), String>;
```

Quy tắc:
- DPAPI: không dùng cờ máy cục bộ; có cờ không hiện giao diện. Xóa trắng bộ đệm bản rõ sau khi dùng.
- Clipboard: `OpenClipboard` có thể thất bại vì ứng dụng khác đang giữ; thử lại tối đa 10 lần, mỗi lần cách 30 ms, rồi mới trả lỗi. Dùng được từ luồng nền.
- Khởi động cùng hệ thống: giá trị `shotme` trong `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, nội dung là đường dẫn file chạy hiện tại đặt trong dấu nháy kép. Hàm nội bộ nhận tên giá trị làm tham số để kiểm thử dùng tên khác.

- [ ] **Step 1: Viết kiểm thử thất bại:**

```rust
#[test] fn dpapi_round_trip() {
    let c = DpapiStore.protect(b"TOP-SECRET-VALUE").unwrap();
    assert!(!c.windows(16).any(|w| w == b"TOP-SECRET-VALUE"));
    assert_eq!(DpapiStore.unprotect(&c).unwrap(), b"TOP-SECRET-VALUE");
}
#[test] fn dpapi_rejects_garbage() { assert!(DpapiStore.unprotect(b"not a dpapi blob").is_err()); }
#[test] fn clipboard_text_round_trip() {
    // set_text("https://img.example.com/d30/Ab12.png") rồi đọc lại CF_UNICODETEXT => bằng nhau (có ký tự tiếng Việt trong một ca khác)
}
#[test] fn clipboard_image_round_trip() {
    // ảnh 3x2 với pixel đã biết => đọc lại CF_DIB: rộng 3, cao 2, pixel trên cùng bên trái đúng màu
}
#[test] fn autostart_round_trip_with_test_value_name() {
    // dùng tên giá trị "shotme-test-{pid}": bật => tồn tại và nội dung bắt đầu và kết thúc bằng dấu nháy kép; tắt => không còn
}
```

Hai kiểm thử clipboard dùng chung một khóa `Mutex` tĩnh để không chạy song song.

- [ ] **Step 2:** Chạy `cargo test -p shotme-windows secret clipboard autostart`. Mong đợi: lỗi biên dịch.
- [ ] **Step 3:** Hiện thực.
- [ ] **Step 4:** Chạy lại. Mong đợi: 5 test PASS.
- [ ] **Step 5:** Commit.
