# Task 10: Cửa sổ Cài đặt

**Agent:** `builder` · **Model:** sonnet · **Nhánh:** `task/10-settings`

Gói task tự chứa. Chỉ đọc file này và các file ở mục "Cần đọc".

## Cần đọc

- `memory-bank/systemPatterns.md`
- `memory-bank/techContext.md`
- docs/superpowers/specs/2026-11-16-shotme-design.md mục 5.5
- `crates/shotme-windows/CLAUDE.md`
- prototype/index.html (hình mẫu giao diện, không dùng lại mã)
- `crates/shotme-core/src/config.rs`
- `crates/shotme-core/src/upload.rs`
- `crates/shotme-windows/src/hotkey.rs`
- `crates/shotme-windows/src/main.rs`

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
- Create: `crates/shotme-windows/src/settings.rs`
- Modify: `crates/shotme-windows/src/main.rs` (khai báo module), `crates/shotme-windows/Cargo.toml`

**Interfaces:**
- Consumes: `shotme_core::config::*`, `shotme_core::upload::R2Uploader`, `hotkey::{register, unregister, describe}`.
- Produces:

```rust
pub struct SettingsResult { pub config: Config }
pub fn show(parent: HWND, current: &Config, notice: Option<&str>) -> Option<SettingsResult>;  // None khi bấm Hủy
pub fn apply_form(current: &Config, form: &Form) -> Result<Config, String>;                    // thuần, kiểm thử được
pub struct Form { pub account_id: String, pub access_key_id: String, pub secret_access_key: String,
                  pub bucket: String, pub public_base_url: String, pub expiry: Expiry,
                  pub hotkey: Hotkey, pub save_dir: String, pub autostart: bool }
```

Hành vi (spec 5.5):
- Các mục và nhãn như prototype (`prototype/index.html`): nhóm "Cloudflare R2" (Account ID, Access Key ID, Secret Access Key, Tên kho (bucket), Địa chỉ công khai, nút "Kiểm tra kết nối" và dòng kết quả), nhóm "Chia sẻ và lưu" (Hạn của link: "1 ngày", "7 ngày", "30 ngày", "Không hết hạn"; Thư mục lưu với nút "Chọn…"), nhóm "Chung" (Phím tắt chụp, "Khởi động cùng Windows"), nút "Lưu" và "Hủy".
- Ô Secret Access Key luôn mở ra rỗng, kiểu mật khẩu. `apply_form`: ô rỗng thì giữ khóa hiện có.
- `apply_form` chuẩn hóa địa chỉ công khai bằng `normalize_public_base_url` (chỉ khi ô không rỗng); cắt khoảng trắng các ô chữ; thư mục lưu rỗng là lỗi. Lỗi hiện ngay trong cửa sổ, cửa sổ không đóng.
- "Kiểm tra kết nối": dựng cấu hình từ biểu mẫu hiện tại, chạy `R2Uploader::test_connection` ở luồng nền, vô hiệu nút trong lúc chờ, hiện "Thành công: tải lên được và link mở được." hoặc `message_vi()`.
- Phím tắt: ô chỉ đọc hiện `describe(hotkey)` và nút "Đổi…". Bấm "Đổi…" thì gọi `hotkey::unregister`, tổ hợp phím kế tiếp trở thành phím tắt mới. Lưu ý `PrintScreen` không sinh `WM_KEYDOWN`; bắt nó ở `WM_KEYUP` với `VK_SNAPSHOT`. Phím tắt cũ được đăng ký lại nếu người dùng bấm Hủy.
- `notice`: dòng chữ hiện ở đầu cửa sổ khi có (dùng cho trường hợp cấu hình hỏng hoặc mất khóa).
- Chỉ một cửa sổ Cài đặt mở cùng lúc; mở lần nữa thì đưa cửa sổ đang có lên trước.

- [ ] **Step 1: Viết kiểm thử thất bại:**

```rust
#[test] fn empty_secret_keeps_existing() {
    // current có secret "OLD", form.secret_access_key == "" => kết quả giữ "OLD"; form có "NEW" => "NEW"
}
#[test] fn url_is_normalized_and_bad_url_rejected() {
    // "https://img.example.com/" => "https://img.example.com"; "http://x" => Err
}
#[test] fn empty_url_allowed_when_r2_not_filled() { /* mọi ô R2 rỗng => Ok, r2.is_complete() == false */ }
#[test] fn fields_are_trimmed() { /* " bkt " => "bkt" */ }
#[test] fn empty_save_dir_rejected() { /* Err */ }
```

- [ ] **Step 2:** Chạy `cargo test -p shotme-windows settings`. Mong đợi: lỗi biên dịch.
- [ ] **Step 3:** Hiện thực `apply_form`, rồi cửa sổ.
- [ ] **Step 4:** Chạy lại (5 test PASS).
- [ ] **Step 5:** Thêm vào `docs/testing/manual-checklist.md` mục "Cài đặt": mọi mục hiện đúng; ô khóa bí mật rỗng khi mở lại; đổi phím tắt sang tổ hợp khác và về lại PrintScreen; nút Chọn… mở hộp chọn thư mục; lỗi địa chỉ công khai hiện trong cửa sổ; Kiểm tra kết nối báo lỗi khi sai khóa.
- [ ] **Step 6:** Commit.
