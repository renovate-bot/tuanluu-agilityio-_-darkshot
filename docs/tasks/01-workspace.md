# Task 1: Khung workspace

**Agent:** `scaffolder` · **Model:** haiku · **Nhánh:** `task/01-workspace`

Gói task tự chứa. Chỉ đọc file này và các file ở mục "Cần đọc".

## Cần đọc

- `memory-bank/techContext.md`

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
- Create: `Cargo.toml`, `crates/shotme-core/Cargo.toml`, `crates/shotme-core/src/lib.rs`, `crates/shotme-windows/Cargo.toml`, `crates/shotme-windows/src/main.rs`

**Interfaces:**
- Produces: package `shotme-core` (lib), package `shotme-windows` với `[[bin]] name = "shotme"`, phụ thuộc `shotme-core` theo đường dẫn.

- [ ] **Step 1:** `Cargo.toml` gốc: `[workspace] members = ["crates/*"]`, `resolver = "2"`, và

```toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

- [ ] **Step 2:** `shotme-core/src/lib.rs` có một kiểm thử `workspace_builds` khẳng định `2 + 2 == 4`. `shotme-windows/src/main.rs` có `#![windows_subsystem = "windows"]` và `fn main() {}`.
- [ ] **Step 3:** Chạy `cargo test --workspace` (PASS, 1 test) và `cargo build --release` (tạo `target/release/shotme.exe`).
- [ ] **Step 4:** Commit.

**Xong khi:** hai lệnh ở Step 3 thành công.
