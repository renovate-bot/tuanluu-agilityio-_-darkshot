# Task 11: Nối toàn bộ

**Agent:** `builder` · **Model:** sonnet · **Nhánh:** `task/11-integration`

Gói task tự chứa. Chỉ đọc file này và các file ở mục "Cần đọc".

## Cần đọc

- `memory-bank/systemPatterns.md`
- `memory-bank/techContext.md`
- docs/superpowers/specs/2026-11-16-shotme-design.md mục 5
- `crates/shotme-windows/CLAUDE.md`
- mọi file trong crates/shotme-windows/src/
- crates/shotme-core/src/lib.rs và các module nó khai báo (chỉ phần chữ ký công khai)

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
- Modify: `crates/shotme-windows/src/main.rs`
- Create: `crates/shotme-windows/src/app.rs`

**Interfaces:**
- Consumes: mọi thứ từ Task 2 đến Task 10.
- Produces:

```rust
// app.rs
pub struct AppState { pub config: Config, pub config_notice: Option<String>, pub overlay_open: bool }
pub fn startup(path: &Path, store: &dyn SecretStore, default_save_dir: &Path) -> AppState;   // thuần với hệ thống file
pub fn needs_settings_on_start(s: &AppState, config_file_exists: bool) -> bool;
```

Hành vi:
- Khởi động: `config::load`. File chưa có: mở Cài đặt (spec 5.1). `Err(Corrupt)`: dùng mặc định, `config_notice` = "File cấu hình bị hỏng. Cài đặt mặc định đang được dùng; bấm Lưu để ghi lại.", mở Cài đặt, không ghi file. `secret_lost`: `config_notice` = "Không đọc được khóa bí mật đã lưu. Hãy nhập lại Secret Access Key.", mở Cài đặt.
- Đăng ký phím tắt theo cấu hình; áp dụng `autostart::set_enabled` khi lưu cài đặt.
- Capture: nếu lớp phủ đang mở thì bỏ qua. Chụp, `pick_region`, cắt ảnh bằng `Image::crop`. Lưu ý sau Task 8: `pick_region` trả `Result<Option<(Action, Rect)>, String>` (`Err` thì báo lỗi "Không mở được lớp phủ chọn vùng"), và việc "bỏ qua khi lớp phủ đang mở" đã do cổng `gate()`/`capture_flow` trong `main.rs` đảm nhận: nối `run_action` vào `capture_flow`, không viết lại cổng.
- `Save` và `Copy` chạy ngay trên luồng chính. `Upload` chạy `run_action` trên một luồng mới với ảnh đã cắt; mỗi lần tải lên một luồng riêng, không chờ nhau.
- `uploader` là `Some(R2Uploader::new(..))` khi `r2.is_complete()`, ngược lại `None`.
- Kết quả `UploadFailedSaved { needs_settings: true }`: gửi sự kiện mở Cài đặt về luồng chính.
- Lưu cài đặt: `config::save`, đăng ký lại phím tắt; phím mới bị chiếm thì thông báo Error và giữ phím cũ.
- Gỡ bỏ các thông báo tạm "Chưa có" và "Đã chọn …" của Task 7 và Task 8.

- [ ] **Step 1: Viết kiểm thử thất bại** (dùng `XorStore` và `FailStore` như Task 3, định nghĩa lại trong module kiểm thử):

```rust
#[test] fn first_run_opens_settings() {
    let s = startup(&missing_path, &XorStore, Path::new("C:/pics"));
    assert!(needs_settings_on_start(&s, false) && s.config_notice.is_none());
}
#[test] fn corrupt_config_uses_defaults_and_does_not_overwrite() {   // Review Focus 3
    // file chứa "{not json" => s.config == Config::defaults(..), s.config_notice.is_some(),
    // needs_settings_on_start == true, nội dung file vẫn là "{not json"
}
#[test] fn lost_secret_opens_settings_with_notice() {               // Review Focus 3
    // lưu bằng XorStore, startup bằng FailStore => notice có chữ "Secret Access Key", needs_settings_on_start == true
}
#[test] fn complete_config_starts_quietly() { /* needs_settings_on_start == false */ }
#[test] fn concurrent_uploads_each_notify() {                       // Review Focus 4
    // hai luồng gọi run_action(Upload) với FakeUploader chậm 200 ms và hai link khác nhau
    // => FakeNotifier nhận đúng 2 thông báo "Đã chép link", mỗi link xuất hiện một lần
}
```

- [ ] **Step 2:** Chạy `cargo test -p shotme-windows app`. Mong đợi: lỗi biên dịch.
- [ ] **Step 3:** Hiện thực `app.rs` và nối trong `main.rs`.
- [ ] **Step 4:** Chạy `cargo test --workspace` (mọi test PASS) và `cargo clippy --workspace -- -D warnings`.
- [ ] **Step 5:** Thêm vào `docs/testing/manual-checklist.md` mục "Toàn bộ": lần chạy đầu mở Cài đặt; Lưu tạo file đúng tên trong thư mục; Chép dán được vào Paint; Tải lên khi chưa cấu hình thì lưu local và mở Cài đặt; Tải lên khi rút mạng thì lưu local và báo lỗi; hai lần tải lên liên tiếp có hai thông báo; khởi động cùng Windows sau khi đăng nhập lại.
- [ ] **Step 6:** Commit.
