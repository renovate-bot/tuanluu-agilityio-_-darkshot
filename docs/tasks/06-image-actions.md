# Task 6: Ảnh PNG và luồng hành động

**Agent:** `builder` · **Model:** sonnet · **Nhánh:** `task/06-image-actions`

Gói task tự chứa. Chỉ đọc file này và các file ở mục "Cần đọc".

## Cần đọc

- `memory-bank/systemPatterns.md`
- `memory-bank/techContext.md`
- docs/superpowers/specs/2026-11-16-shotme-design.md mục 5.3
- `crates/shotme-core/CLAUDE.md`
- `crates/shotme-core/src/geometry.rs`
- `crates/shotme-core/src/naming.rs`
- `crates/shotme-core/src/config.rs`
- `crates/shotme-core/src/upload.rs`

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
- Create: `crates/shotme-core/src/image.rs`, `crates/shotme-core/src/actions.rs`
- Modify: `crates/shotme-core/src/lib.rs`, `crates/shotme-core/Cargo.toml` (thêm `png`)

**Interfaces:**
- Consumes: `geometry::Rect`, `naming::*`, `config::Expiry`, `upload::{Uploader, UploadError, object_key}`.
- Produces:

```rust
// image.rs
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image { pub width: u32, pub height: u32, pub bgra: Vec<u8> }   // từ trên xuống, 4 byte mỗi pixel: B, G, R, A
impl Image { pub fn crop(&self, r: Rect) -> Image; }                       // r được kẹp vào ảnh
pub fn encode_png(img: &Image) -> Vec<u8>;                                 // RGB 8 bit, bỏ kênh alpha

// actions.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Action { Upload, Save, Copy, Cancel }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum NoticeKind { Info, Error }
pub trait Notifier: Send + Sync { fn notify(&self, kind: NoticeKind, title: &str, body: &str); }
pub trait Clipboard: Send + Sync {
    fn set_text(&self, s: &str) -> Result<(), String>;
    fn set_image(&self, img: &Image) -> Result<(), String>;
}
pub struct Ctx<'a> {
    pub save_dir: &'a Path, pub expiry: Expiry, pub now: LocalTime,
    pub uploader: Option<&'a dyn Uploader>,          // None khi R2 chưa cấu hình
    pub clipboard: &'a dyn Clipboard, pub notifier: &'a dyn Notifier,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Cancelled, Saved(PathBuf), Copied, Uploaded(String),
    UploadFailedSaved { path: PathBuf, needs_settings: bool },
    Failed,
}
pub fn run_action(ctx: &Ctx, action: Action, img: &Image) -> Outcome;
```

Luồng `run_action` (spec 5.3):
- `Cancel`: không làm gì, không thông báo.
- `Save`: tạo `save_dir` nếu thiếu, ghi PNG vào `unique_path(save_dir, local_file_name(now))`, thông báo Info có đường dẫn.
- `Copy`: `clipboard.set_image`, thông báo Info.
- `Upload` với `uploader` có: mã hóa PNG, `upload(object_key(expiry, random_object_name()), png)`. Thành công: `set_text(link)`, thông báo Info tiêu đề "Đã chép link", thân là link. Thất bại: lưu như `Save`, thông báo Error gồm `message_vi()` và đường dẫn đã lưu, trả `UploadFailedSaved { needs_settings: false }`.
- `Upload` với `uploader` là `None`: lưu như `Save`, thông báo Error, trả `UploadFailedSaved { needs_settings: true }`.
- Bất kỳ lần lưu nào thất bại (kể cả lưu dự phòng): `clipboard.set_image(img)`, thông báo Error nói rõ không lưu được và ảnh đã ở trong clipboard, trả `Failed`.

- [ ] **Step 1: Viết kiểm thử thất bại.** Dùng `FakeClipboard` và `FakeNotifier` ghi lại lời gọi vào `Mutex<Vec<_>>`, `FakeUploader` trả kết quả định sẵn và ghi lại key.

```rust
// image.rs
#[test] fn crop_takes_the_right_pixels() {
    // ảnh 4x3, pixel (x,y) có B = x, G = y; crop Rect{x:1,y:1,w:2,h:2}
    // => ảnh 2x2 với bgra bắt đầu [1,1,_,_, 2,1,_,_, 1,2,_,_, 2,2,_,_]
}
#[test] fn crop_is_clamped() { /* Rect vượt mép => kích thước bị cắt, không panic */ }
#[test] fn png_round_trip() {
    // encode_png rồi giải mã bằng crate png: cùng kích thước, kiểu màu RGB, pixel đầu (R,G,B) đúng thứ tự
    // 8 byte đầu == [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]
}
// actions.rs
#[test] fn cancel_does_nothing() { /* Outcome::Cancelled, không thông báo, không file */ }
#[test] fn save_creates_dir_and_file() {
    // save_dir chưa tồn tại => Outcome::Saved(dir/shotme_2026-11-16_14-03-05.png), file bắt đầu bằng chữ ký PNG, 1 thông báo Info
}
#[test] fn copy_puts_image_on_clipboard() { /* set_image nhận đúng ảnh, Outcome::Copied */ }
#[test] fn upload_success_copies_link() {
    // FakeUploader trả Ok("https://img.example.com/d7/x.png"), expiry D7
    // => key ghi lại bắt đầu "d7/" và kết thúc ".png", dài 3 + 16 + 4
    // => set_text("https://img.example.com/d7/x.png"), thông báo Info tiêu đề "Đã chép link"
    // => không có file nào trong save_dir
}
#[test] fn upload_failure_saves_locally() {
    // FakeUploader trả Err(Network("x")) => UploadFailedSaved { needs_settings: false }, file tồn tại,
    // thông báo Error có chứa đường dẫn file, clipboard không bị đụng tới
}
#[test] fn upload_without_config_saves_and_asks_for_settings() {
    // uploader None => UploadFailedSaved { needs_settings: true }, file tồn tại
}
#[test] fn unwritable_save_dir_keeps_image_on_clipboard() {     // Review Focus 1
    // save_dir là đường dẫn tới một FILE đang tồn tại (không tạo được thư mục)
    // Save => Outcome::Failed, set_image được gọi, 1 thông báo Error
    // Upload thất bại + lưu dự phòng thất bại => Outcome::Failed, set_image được gọi
}
```

- [ ] **Step 2:** Chạy `cargo test -p shotme-core image actions`. Mong đợi: lỗi biên dịch.
- [ ] **Step 3:** Hiện thực.
- [ ] **Step 4:** Chạy lại. Mong đợi: 10 test PASS.
- [ ] **Step 5:** Commit.
