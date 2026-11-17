# Task 54: Lịch sử – cấu hình, đường dẫn, dịch vụ ghi nền, nối vào các hành động, chế độ cách ly

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/54-history-integration` · **Rà soát:** `specialist`

Thiết kế: `docs/superpowers/specs/2026-11-18-screenshot-history-design.md` mục 1, 2, 4, 7. Global Constraints: mục cùng tên của `docs/superpowers/plans/2026-11-18-screenshot-history.md`. Task 52 (`history`) và 53 (`history_store`, `thumb`) đã ở nhánh phát triển.

## Cần đọc
`crates/darkshot-core/src/config.rs` (struct `Config`, `defaults`, các hàm đọc/ghi JSON khoan dung, kiểm thử `check_updates_*` làm mẫu), `actions.rs` (`Outcome`), `crates/darkshot-windows/src/paths.rs`, `main.rs` (`run_on_image`, `save_pending_uploads`, `main()`), `app.rs` (`spawn_upload`, `save_pending`, `UploadJob`), `legacy.rs`, `wer.rs`, `settings.rs` (CHỈ chỗ dựng `Config` từ form: `FormValues` → `Config`, để `history_enabled` được giữ nguyên khi lưu).

## Files
Sửa: `darkshot-core/src/config.rs`, `darkshot-windows/src/{paths.rs, main.rs, app.rs, settings.rs (chỉ để giữ giá trị), legacy.rs/wer.rs chỉ nếu cần để bỏ qua}`. Tạo `darkshot-windows/src/history_service.rs`.

## Yêu cầu

### A. Cấu hình (lõi)
- `Config.history_enabled: bool`, mặc định `false`, JSON `"history_enabled"`. Thiếu hoặc sai kiểu → `false`. Ghi ra JSON luôn kèm trường này. Tệp cấu hình cũ (không có trường) vẫn nạp được và, khi ghi lại, các trường khác không đổi. Kiểm thử: vòng đủ, mặc định, sai kiểu (`"yes"`, `1`), tệp cũ nguyên văn không có trường.
- Cửa sổ Tùy chọn hiện chưa có hộp tích (Task 55): task này chỉ bảo đảm lưu từ Tùy chọn KHÔNG làm mất `history_enabled` (form lưu giá trị hiện tại; thêm trường `history_enabled` vào `FormValues` và vào các chỗ dựng/so sánh, mặc định lấy từ config hiện tại).

### B. Đường dẫn và chế độ cách ly (Windows)
- `paths::local_appdata_dir()` (FOLDERID_LocalAppData, dự phòng `%LOCALAPPDATA%`, rồi `temp_dir`), `paths::history_dir()` = `<local>\darkshot\history`.
- `paths::profile_override() -> Option<PathBuf>`: biến môi trường `DARKSHOT_PROFILE_DIR` (không rỗng). Khi có: `appdata_dir()` = `<dir>\Roaming`, `local_appdata_dir()` = `<dir>\Local` (tạo khi cần ghi, KHÔNG tạo ở đây), `pictures_dir()` = `<dir>\Pictures`. Một hàm thuần `resolve(override, known) -> PathBuf` để kiểm thử.
- Trong `main()`: khi có `profile_override`: mutex một-thể-hiện tên `Local\darkshot-single-instance-sandbox-<băm ngắn của đường dẫn>` (để chạy được song song với bản của chủ dự án); BỎ QUA `legacy::run_before_startup`, `legacy::sync_autostart`, `wer::exclude_from_error_reporting`, và không đăng ký hẹn giờ cập nhật (không gọi mạng ở chế độ cách ly). Không đổi gì khi không có biến. Kiểm thử hàm thuần quyết định chế độ.
- Cờ dòng lệnh `--history`: sau khi khởi động xong (cùng chỗ `open_settings_now`), đăng thông điệp mở cửa sổ Lịch sử. Task này chỉ cần định nghĩa `WM_OPEN_HISTORY = WM_APP + 5` và xử lý nó bằng một hàm `history_window::show(hwnd)` giả (tạm: không làm gì, `// TODO(task 55)` bị cấm ở mã cuối; để hàm rỗng có chú thích rõ Task 55 thay). Phân tích đối số thành hàm thuần `parse_args(&[String]) -> Args { open_history: bool }`.

### C. Dịch vụ ghi nền (`history_service.rs`)
```rust
pub fn set_enabled(on: bool);            // AtomicBool toàn tiến trình; main() gọi lúc khởi động và sau khi Tùy chọn lưu thành công
pub fn is_enabled() -> bool;
pub fn record_outcome(outcome: &Outcome, img: &Image, expiry: Expiry, now: LocalTime);   // không chặn
pub fn record_outcome_blocking(...)      // cùng đối số, đồng bộ, dùng khi thoát
pub fn failure_count() -> u32;           // bộ đếm lỗi trong bộ nhớ (kiểm thử, không ghi nhật ký)
```
- Ánh xạ: `Outcome::Saved(path)` → `Kind::Saved` + `path`; `Copied` → `Kind::Copied`; `Uploaded(link)` → `Kind::Uploaded` + `url` + `expires = expires_on(now, expiry)`; `UploadFailedSaved{path,..}` → `Kind::UploadFallback` + `path`; `Failed`/`Cancelled` → không ghi. Ánh xạ là hàm thuần `plan(&Outcome, expiry, &LocalTime, unix_millis) -> Option<NewEntry>` kiểm thử riêng (trong lõi nếu tiện: `history::plan_from_outcome` KHÔNG được vì lõi `history.rs` đã đóng; đặt trong `history_service.rs`).
- Không tắt → không làm gì cả (không clone ảnh, không tạo thư mục, không tạo luồng).
- Bật: clone ảnh MỘT LẦN (nó là `&Image`; chỉ clone khi bật), sinh luồng `darkshot-history` (tên đó) gọi `Store::new(paths::history_dir()).record(...)`; mọi `Err` và panic bị nuốt (`catch_unwind` hoặc `thread::spawn` + đếm lỗi); không panic ngược lên luồng gọi; không tạo được luồng thì tăng bộ đếm và bỏ.
- Sau khi ghi xong: nếu cửa sổ Lịch sử đang mở thì `PostMessageW` thông báo nạp lại (thông điệp `WM_HISTORY_CHANGED = WM_APP + 6` gửi tới HWND cửa sổ lịch sử; Task 56 xử lý; tạm thời nơi lưu HWND là một `AtomicIsize` công khai `history_service::set_window(isize)`).
- Chạy dưới nền nghĩa là: người dùng thoát ngay sau khi chụp có thể mất mục; chấp nhận cho Lưu/Chép/Tải lên (luồng nền) nhưng KHÔNG cho lưu dự phòng khi thoát: `save_pending_uploads` phải dùng `record_outcome_blocking`.

### D. Nối vào hành động
- `main::run_on_image`: sau mỗi `run_action` cho Lưu và Chép (cả nhánh lưu dự phòng khi không tạo được luồng tải lên) gọi `history_service::record_outcome` với `Outcome` trả về. Đọc `expiry` từ `config`.
- `app::spawn_upload`: thêm vào `UploadJob` một móc `on_outcome: Option<Box<dyn Fn(&Outcome, &Image) + Send>>` (hoặc trực tiếp gọi dịch vụ; chọn cách dễ kiểm thử, ví dụ móc), gọi nó với kết quả của `run_action` TRƯỚC khi rút khỏi sổ `PENDING`? Không: rút khỏi sổ trước (giữ nguyên thứ tự cũ), rồi gọi móc. Nếu thoát giữa chừng, lưu dự phòng đã lo bởi `save_pending`.
- `app::save_pending`: sau mỗi ảnh lưu dự phòng thành công, ghi lịch sử đồng bộ như `UploadFallback`. Cách làm: `save_pending` nhận thêm một `&dyn Fn(&Outcome,&Image)`; kiểm thử với móc giả.
- `set_enabled` lúc khởi động (từ `config.history_enabled`) và sau khi Tùy chọn lưu xong.
- Không đổi chữ ký `run_action`/`Ctx` của lõi.

## Kiểm thử (viết trước)
Lõi: như mục A. Windows: `plan` cho cả 4 `Outcome` hữu ích + 2 không ghi; hết hạn đúng cho D1/D7/D30/Keep; `resolve` đường dẫn cách ly; `parse_args`; chế độ cách ly (thuần); dịch vụ: tắt → thư mục không được tạo, bật → mục xuất hiện sau khi chờ luồng (dùng `Store` trên thư mục tạm qua một hàm nội bộ nhận `root`; chỉ kiểm thử hàm nhận `root`, không đụng `%LOCALAPPDATA%` thật); `record_outcome` với `root` không ghi được (đường dẫn là một TỆP) → không panic, `failure_count` tăng; hai kết quả tải lên đồng thời → hai mục; móc `on_outcome` của `spawn_upload` được gọi đúng một lần với `Uploaded` và với `UploadFailedSaved`; `save_pending` gọi móc cho mỗi ảnh.

## Việc cần chủ dự án thử tay
Không (chưa có giao diện).

## Báo cáo
Theo mẫu `.claude/agents/specialist.md`. Liệt kê mọi chỗ làm khác gói và hạn chế (đặc biệt: mục có thể mất nếu thoát ngay sau khi Lưu/Chép).
