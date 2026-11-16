# Task 7: Vỏ ứng dụng Windows

**Agent:** `builder` · **Model:** sonnet · **Nhánh:** `task/07-win-shell`

Gói task tự chứa. Chỉ đọc file này và các file ở mục "Cần đọc".

## Cần đọc

- `memory-bank/systemPatterns.md`
- `memory-bank/techContext.md`
- docs/superpowers/specs/2026-11-16-shotme-design.md mục 5.1, 5.6
- `crates/shotme-windows/CLAUDE.md`
- `crates/shotme-core/src/actions.rs`
- `crates/shotme-core/src/config.rs`
- `crates/shotme-core/src/naming.rs`

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
- Create: `crates/shotme-windows/build.rs`, `crates/shotme-windows/res/app.manifest`, `crates/shotme-windows/res/shotme.ico`, `crates/shotme-windows/src/{tray.rs,hotkey.rs,paths.rs}`
- Modify: `crates/shotme-windows/src/main.rs`, `crates/shotme-windows/Cargo.toml` (thêm `windows` với các tính năng cần dùng, và một crate nhúng tài nguyên như `embed-resource`)

**Interfaces:**
- Consumes: `shotme_core::actions::{Notifier, NoticeKind}`, `shotme_core::config::Hotkey`, `shotme_core::naming::LocalTime`.
- Produces:

```rust
// tray.rs
pub const WM_TRAY: u32;                     // thông điệp riêng của biểu tượng khay
pub enum MenuCommand { Capture, Settings, Quit }
pub struct Tray { /* ... */ }
impl Tray {
    pub fn new(hwnd: HWND) -> windows::core::Result<Tray>;
    pub fn show_menu(&self) -> Option<MenuCommand>;     // gọi khi bấm chuột phải hoặc trái vào biểu tượng
    pub fn readd(&self);                                 // gọi khi nhận "TaskbarCreated"
    pub fn notifier(&self) -> TrayNotifier;
}
#[derive(Clone)] pub struct TrayNotifier { /* giữ HWND dưới dạng isize */ }
impl Notifier for TrayNotifier { /* an toàn khi gọi từ luồng khác: PostMessage về luồng chính */ }
// hotkey.rs
pub const HOTKEY_ID: i32 = 1;
pub fn register(hwnd: HWND, hk: &Hotkey) -> bool;        // RegisterHotKey với MOD_NOREPEAT
pub fn unregister(hwnd: HWND);
pub fn describe(hk: &Hotkey) -> String;                  // "PrintScreen", "Ctrl+Shift+S"
// paths.rs
pub fn config_path() -> PathBuf;                         // %APPDATA%\shotme\config.json
pub fn default_save_dir() -> PathBuf;                    // Pictures\shotme
pub fn local_now() -> LocalTime;
// main.rs
pub enum AppEvent { Capture, OpenSettings, Quit }        // nội bộ, Task 11 mở rộng
```

Hành vi (spec 5.1, 5.6):
- Manifest khai báo `PerMonitorV2` và Common Controls v6.
- Một bản chạy: mutex có tên `Local\shotme-single-instance`; bản thứ hai thoát ngay.
- Cửa sổ ẩn nhận thông điệp. Biểu tượng khay có chú thích "shotme". Menu: "Chụp màn hình", "Cài đặt", "Thoát".
- Đăng ký phím tắt mặc định. Thất bại: thông báo Error "Phím {describe} đang bị ứng dụng khác chiếm" kèm gợi ý đổi phím trong Cài đặt; ứng dụng vẫn chạy.
- Ở task này "Chụp màn hình" (menu hoặc phím tắt) và "Cài đặt" chỉ hiện thông báo Info tạm "Chưa có" để kiểm tra đường đi của sự kiện. "Thoát" gỡ biểu tượng và kết thúc.
- `shotme.ico`: biểu tượng tạm, nền xanh `#2563eb` với hình khung chọn trắng, có cỡ 16, 32, 48.

- [ ] **Step 1: Viết kiểm thử thất bại** (chạy được không cần giao diện):

```rust
#[test] fn describe_hotkey() {
    assert_eq!(describe(&Hotkey { ctrl: false, alt: false, shift: false, win: false, vk: 0x2C }), "PrintScreen");
    assert_eq!(describe(&Hotkey { ctrl: true, alt: false, shift: true, win: false, vk: 0x53 }), "Ctrl+Shift+S");
    assert_eq!(describe(&Hotkey { ctrl: false, alt: true, shift: false, win: false, vk: 0x71 }), "Alt+F2");
}
#[test] fn paths_end_correctly() {
    assert!(config_path().ends_with("shotme/config.json"));
    assert!(default_save_dir().ends_with("shotme"));
}
#[test] fn local_now_is_plausible() { let t = local_now(); assert!(t.year >= 2026 && (1..=12).contains(&t.month)); }
```

- [ ] **Step 2:** Chạy `cargo test -p shotme-windows`. Mong đợi: lỗi biên dịch.
- [ ] **Step 3:** Hiện thực.
- [ ] **Step 4:** Chạy lại (3 test PASS). Chạy `cargo build --release`, khởi động `target/release/shotme.exe`, xác nhận bằng lệnh rằng tiến trình còn sống sau 3 giây và lần khởi động thứ hai thoát ngay với một tiến trình `shotme` duy nhất; rồi kết thúc tiến trình.
- [ ] **Step 5:** Thêm vào `docs/testing/manual-checklist.md` mục "Vỏ ứng dụng": biểu tượng hiện ở khay; menu có ba mục; Thoát gỡ biểu tượng; bấm PrintScreen hiện thông báo; khi Lightshot đang chạy thì có thông báo phím bị chiếm.
- [ ] **Step 6:** Commit.
