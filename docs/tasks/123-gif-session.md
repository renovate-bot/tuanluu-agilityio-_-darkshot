# Task 123: Ghi GIF — phiên ghi, thanh ghi, lớp phủ, nối ứng dụng

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/123-gif-session` (từ `feature/gif-recording` sau khi gộp 122) · **Rà soát:** `reviewer` (fable)

Thiết kế: `docs/superpowers/specs/2026-11-21-gif-recording-design.md` mục 1, 3.2, 3.3, 3.4, 5, 6. Global Constraints: kế hoạch `docs/superpowers/plans/2026-11-21-gif-recording.md`.

## Cần đọc
`crates/darkshot-core/src/gif_record.rs` (Task 122: `Recorder`, `RecordConfig`, `Outcome`, `GifError`, `sweep_stale`), `crates/darkshot-windows/src/scroll_session.rs` (mẫu: thanh, khung, `placement`/`place`, `exclude_supported`/`os_build`, `relayout_bar`, `check_region`, quy tắc tái nhập `SESSION`, mô-đun `live`), `capture.rs` (`capture_rect`, `to_screen_coords`), `main.rs` (nhánh `Action::Scroll` trong luồng lớp phủ, `scroll_region`, `scroll_blocks`, `request`, `steps_start_allowed`, `STEPS_BAR_HELD`, `exit_cleanup`, `WM_ENDSESSION`, `WM_DESTROY`, nơi đăng ký `scroll_session::set_on_*`, thông báo `run_on_image` cho Lưu), `overlay.rs` (`BAR_CELLS`, `bar_cells`, `tip_text`, `text_key`, phím một chữ, `icon`, kiểm thử bố cục thanh), `steps_bar.rs` (loại khỏi ảnh), `help.rs`.

## Files
Tạo `crates/darkshot-windows/src/record_session.rs`; sửa `main.rs`, `overlay.rs`, `help.rs` (nếu bảng phím ở đó), `scroll_session.rs` (CHỈ đổi vài hàm thuần thành `pub(crate)` để dùng lại; không đổi hành vi), `crates/darkshot-core/src/i18n.rs` (chỉ khi thiếu chuỗi).

## Interfaces
```rust
// record_session.rs
pub const WM_RECORD_DONE: u32 = WM_APP + 18;
pub enum StartError { Busy, Empty, NoRoom, Failed, Recorder(GifError) }
pub fn start(region: Rect, save_dir: PathBuf) -> Result<(), StartError>; // region: tọa độ màn hình vật lý
pub fn active() -> bool;            // true từ start tới khi đã dọn xong (kể cả lúc đang lưu)
pub fn stop();  pub fn cancel();    // an toàn khi gọi nhiều lần hoặc không có phiên
pub fn finish_on_exit(deadline: Instant);
pub fn set_on_outcome(f: fn(Outcome));  // main thông báo theo Outcome
```

## Yêu cầu
1. Vùng: như `scroll_region` (kẹp vào ảnh, cộng `origin` đúng một lần). Kiểm thử với `origin` âm (màn hình phụ bên trái/trên).
2. Thanh ghi: `● Đang ghi 00:05 / 00:15  [Dừng] [Hủy]`, chấm đỏ `theme` DANGER, chữ từ `i18n`, đo chữ theo DPI (bố cục thuần + kiểm thử chạm ở 96/120/144/192 DPI, hai ngôn ngữ); sau Dừng: "Đang lưu…", nút mờ, không nhận bấm. Không lấy tiêu điểm (`WS_EX_NOACTIVATE`, `MA_NOACTIVATE`), luôn trên cùng, `WDA_EXCLUDEFROMCAPTURE` khi được; khung quanh vùng cũng thế. Bản dựng cũ: không khung, thanh ngoài vùng hoặc `NoRoom`.
3. Luồng: `Recorder::start(cfg, move || capture::capture_rect(region).map_err(..), move || PostMessageW(host, WM_RECORD_DONE, ..))`. HWND chuyển sang luồng nền dưới dạng `isize` (HWND không `Send`); ghi SAFETY. Cửa sổ chủ nhận `WM_RECORD_DONE` → `try_outcome()` → dọn cửa sổ (lấy trạng thái khỏi `SESSION` trước khi `DestroyWindow`) → callback. `WM_RECORD_DONE` tới sau khi phiên đã dọn: bỏ qua.
4. Thời gian hiện: hẹn giờ 250 ms của thanh, chỉ `InvalidateRect`. Không chụp trên luồng giao diện.
5. `WM_DISPLAYCHANGE`/`WM_DPICHANGED`: đặt lại thanh; vùng không nằm trọn trên các màn hình → `stop()`.
6. Lớp phủ: ô `RecordGif` giữa `Scroll` và `Cancel`, phím `G` và `Ctrl+G` (cả khi đang gõ chữ, như `L`), tooltip "… · Không gồm hình đánh dấu"; biểu tượng tự vẽ. Cập nhật mọi kiểm thử bố cục/chạm của thanh ngang (số ô 10→11, có/không phiên các bước), kiểm thanh vẫn vừa màn hình nhỏ ở 96–192 DPI như kiểm thử hiện có.
7. `main.rs`: nhánh `RecordGif` (bỏ ảnh tĩnh + hình đánh dấu như `Scroll`; `save_dir` từ cấu hình như Lưu; tên `naming::local_gif_name(now)`); cổng bận khi đang ghi (phím tắt, mục Chụp của khay, nút Chụp bước, Chụp lại vùng trước đều bị bỏ qua); không bắt đầu chụp từng bước hay chụp cuộn khi đang ghi và ngược lại không ghi khi đang chụp cuộn (cổng đã chặn lớp phủ); thanh các bước không loại được → giữ ẩn tới khi ghi xong. Thông báo theo `Outcome` (thiết kế 1, 3.3): Saved/Stopped → Success "Đã lưu GIF" + tên tệp; MaxDuration/MaxSize/SourceLost → Info + tên tệp; Failed → Error + `GifError::message`; Cancelled → không. `StartError::Recorder(InvalidRegion|TooLarge)` → chuỗi tương ứng; `NoRoom` → dùng lại thông báo không có chỗ (hoặc chuỗi riêng).
8. Thoát: `exit_cleanup`, `WM_ENDSESSION`, `WM_DESTROY` gọi `record_session::finish_on_exit(deadline)` (lưu phần đã ghi trong `EXIT_BUDGET` chung) TRƯỚC khi hủy các cửa sổ khác; khởi động: `gif_record::sweep_stale(save_dir)` một lần (sau khi đọc cấu hình, không chặn đáng kể).
9. Không dùng chung trạng thái với `scroll_session`/`steps_bar`.

## Kiểm thử
Viết trước: bố cục thanh ghi (đo chữ giả), chạm nút, chuỗi thời gian `00:05 / 00:15`, `remember_after(.., RecordGif, ..)` giữ khung cũ, cổng bận (`scroll_blocks` mở rộng), vùng với `origin` âm, chuyển `Outcome` → (loại, tiêu đề, nội dung) thông báo, bố cục thanh ngang lớp phủ 11 ô. Kiểm thử chạy tay `#[ignore]` (mẫu `scroll_session::live`): tạo thanh trên vùng, `GetWindowDisplayAffinity == WDA_EXCLUDEFROMCAPTURE`, `capture_rect(region)` sau `DwmFlush` không chứa màu thanh; một lượt ghi thật 1 s ra GIF giải mã được rồi xóa. Chạy các kiểm thử `#[ignore]` đó một lần và báo kết quả (KHÔNG giả lập chuột/phím thật, quyết định 111).

`cargo fmt --all`; `powershell -File scripts/check.ps1` → `ALL CHECKS PASSED`; `cargo build --release` và báo kích thước exe trước/sau.

## Review focus
Tái nhập và dọn cửa sổ trên mọi đường (Dừng, Hủy, tự dừng, lỗi, màn hình đổi, thoát); HWND/luồng; thanh không vào ảnh, không lấy tiêu điểm; tọa độ màn hình âm; hồi quy lớp phủ và cổng chụp.
