# Task 131: Privacy Shield — cổng xuất và nối các đường xuất ảnh

**Agent:** `specialist` · **Model:** fable · **Nhánh:** `task/131-privacy-gate` (từ `feature/privacy-shield`), worktree `D:\2026 - Tuan\shotme-privacy` · **Rà soát:** `reviewer` (fable) + rà soát bảo mật thứ hai `specialist` (opus)

Thiết kế: `docs/superpowers/specs/2026-11-22-sensitive-info-redaction-design.md` mục 3.1, 3.2, 8, 8.1, 10, 11, 15, 16, 18.2, 18.5, 18.6 (đọc toàn bộ mục 3, 8, 10, 18). Global Constraints: `docs/superpowers/plans/2026-11-22-sensitive-info-redaction.md`. Quyết định 99, 116, 120, 124 trong `memory-bank/decisions.md`.

## Cần đọc
`crates/darkshot-windows/src/main.rs` (`capture_flow_with`, `picked_image`, `run_on_image`, `pin_action`, `scroll_action`, `on_scroll_result`, `open_pending_pin`, thủ tục cửa sổ chính và vòng thông điệp `pre_translate`, thoát/`WM_ENDSESSION`, `save_pending_uploads`), `pin.rs` (`open`, `copy_or_save`, sink, nhánh `set_image` ~pin.rs:1001), `scroll_result.rs` (`open`, tự Lưu ~518, `on_action`), `steps_store.rs` (`add`), `app.rs` (`spawn_upload`, `save_pending`), `ocr.rs` (`start_scan`, Task 128), `review_window.rs` (Task 130), `darkshot-core/src/privacy/` (127, 129, 130), `config.rs` (mẫu khóa mới như `ocr_auto_copy`), `settings.rs` (ô `history_enabled` ở tab Chung), `i18n.rs`. `grep -rn "run_on_image\|pin::open\|steps_store::add\|set_image" crates/darkshot-windows/src`.

## Files
Tạo `crates/darkshot-windows/src/privacy_gate.rs`. Sửa `main.rs`, `pin.rs`, `scroll_result.rs` (nếu cần cho tự Lưu), `steps_store.rs` (chỉ chữ ký `add(PreparedOutput)`; trường `scanned` là Task 132), `settings.rs`, `darkshot-core/src/config.rs`, `i18n.rs`, `help.rs` nếu Trợ giúp mô tả Tùy chọn. Không sửa `history_window.rs`, `steps_window.rs`, `steps_export.rs`, `record_session.rs` (Task 132) ngoài mức biên dịch.

## Interfaces (hợp đồng với Task 132)
```rust
// privacy_gate.rs (luồng chính)
pub const MAX_PENDING: usize = 3;
pub const SCANNING_WINDOW_DELAY_MS: u32 = 800;
pub enum OutputAction { Save, Copy, Upload, Pin { place: pin::PinPlace }, AddStep,
                        HistoryCopy, HistoryPin, StepExport { step: u32, steps: u32 } }
pub type Done = Box<dyn FnOnce(Option<PreparedOutput>)>; // chỉ cho StepExport (Task 132): None = Hủy
/// Nhận ảnh đã phẳng. Shield tắt → dispatch ngay với `PreparedOutput::unscanned`.
/// Shield bật → luôn quét (mọi OutputAction, kể cả HistoryCopy/HistoryPin và ảnh ghim chưa quét).
pub fn submit(action: OutputAction, image: Image) -> Result<(), SubmitError>;
pub fn submit_with(action: OutputAction, image: Image, done: Done) -> Result<(), SubmitError>; // StepExport
pub enum SubmitError { QueueFull, WindowFailed }
pub fn handle_message(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> Option<LRESULT>; // gọi từ thủ tục cửa sổ chính
pub fn on_overlay_closed();   // mở cửa sổ Duyệt đang chờ
pub fn shutdown();            // thoát: bỏ mọi lượt chờ, close_all cửa sổ Duyệt, không ghi gì
pub fn enabled() -> bool;     // đọc config.privacy_shield
// thay đổi chữ ký
fn run_on_image(hwnd, action, prepared: PreparedOutput, notifier);   // main.rs
pub fn pin::open(prepared: PreparedOutput, place: PinPlace) -> Result<..>;   // ảnh ghim giữ Protection
pub fn steps_store::add(prepared: PreparedOutput) -> Result<..>;
```
`WM_PRIVACY_SCAN = WM_APP + 19`, `WM_PRIVACY_REVIEW = WM_APP + 20` (+ `WM_PRIVACY_PROGRESS = WM_APP + 21` nếu Task 128 có tiến độ). Kiểm thử hằng như main.rs:2518.

## Yêu cầu
1. Cấu hình `privacy_shield: bool` (mặc định false) theo đúng mẫu khóa mới (`ocr_auto_copy`): thiếu khóa → false, ghi không đè khóa khác, kiểm thử di chuyển cấu hình cũ. Ô ở tab Chung dưới Lịch sử với nhãn + mô tả mục 3.1 (vi, en).
2. `capture_flow_with`: sau `picked_image`, mọi nhánh Pin/AddStep/Save/Copy/Upload → `submit`. OCR (Ctrl+T) không đổi. SaveFull/UploadFull, RepeatLast đi cùng đường. `scroll_action` → `submit` (Pin: `CenterOnCursorMonitor`); tự Lưu khi không mở được cửa sổ kết quả → `submit(Save)`. `pin_action` Save/Copy: Shield bật và `protection.scanned() == false` → `submit` (quét lại); còn lại `run_on_image` với `PreparedOutput` giữ nguyên Protection (ảnh ghim giữ `PreparedOutput`; OCR của ảnh ghim dùng `image()`). Bỏ nhánh `WinClipboard.set_image` trực tiếp của pin.rs khi không có sink (không làm gì).
3. Bảng `Pending` (`thread_local!`, token tăng dần): Scanning → (800 ms hẹn giờ → mở cửa sổ "Đang quét") → kết quả: không phát hiện → đóng cửa sổ nếu có, `no_findings`, dispatch; có phát hiện/lỗi → `review_window::open` hoặc `update`. Outcome: Apply → `redacted`; ContinueNone → `reviewed_none`; ContinueUnscanned → `skipped_after_error`; Retry → `start_scan` lại cùng `Arc<Image>`, token mới hoặc thế hệ mới (kết quả cũ bị bỏ); Cancel → xóa, thả ảnh (AddStep: không thêm bước). Lấy `Pending` ra khỏi bảng TRƯỚC khi dispatch; thông điệp trùng/muộn/không khớp bị bỏ (Box vẫn được giải phóng). Mỗi lúc một cửa sổ Duyệt; lượt khác chờ trong hàng (tối đa `MAX_PENDING` tính cả lượt đang mở); đầy → `QueueFull` + thông báo "Đang có ảnh chờ kiểm tra" (vi/en), ảnh bị bỏ. Lớp phủ đang mở (`overlay_open`) → hoãn mở cửa sổ tới `on_overlay_closed`.
4. Làm phẳng ảnh > 4 MP trên luồng nền (`darkshot-privacy-redact`) rồi PostMessage về; nhỏ hơn làm tại chỗ. Lỗi làm phẳng → không xuất, thông báo lỗi.
5. Không tạo được cửa sổ Duyệt → không xuất, thông báo "Không mở được cửa sổ kiểm tra; ảnh chưa được xuất". Không tạo được luồng quét → cửa sổ ở trạng thái lỗi `Failed`.
6. Thoát/`WM_ENDSESSION`: `shutdown()` trước `save_pending_uploads`; không thêm thời gian vào `EXIT_BUDGET`. `save_pending`, `fallback`, `save_failed` dùng ảnh của `PreparedOutput` (đã qua cổng). `record_outcome` nhận `prepared.image()`.
7. Không `eprintln!` trong `privacy_gate.rs`; luật grep của Task 129 phải qua (hàm tạo `PreparedOutput` chỉ trong `privacy_gate.rs`).
8. Kiểm thử: tách logic cổng thành máy trạng thái thuần (`GateCore` trong `privacy_gate.rs` hoặc `darkshot-core/src/privacy/gate.rs`, không Win32) với các sự kiện `Submit/ScanDone/Outcome/Timer/OverlayClosed/Shutdown` → các lệnh `StartScan/OpenReview/Update/Dispatch/Drop/Notify`: kiểm mọi đường mục 14 gạch "Cổng" (tắt; không phát hiện; có phát hiện che hết/một phần/bỏ hết; Hủy; lỗi; Thử lại; tiếp tục không quét; token trùng/muộn; hàng đợi đầy; thoát khi chờ; lớp phủ mở; đúng một Dispatch mỗi Submit hoặc không có). Kiểm thử tích hợp có HWND ẩn: `submit` với Shield tắt gọi `run_on_image` giả (sink tiêm được) đúng một lần. Tải lên dùng `Uploader` giả. Không giả lập chuột/phím thật.
9. Audit trong báo cáo: liệt kê mọi lời gọi `set_image`, `encode_png/jpeg/for_upload`, `save_png`, `upload`, `pin::open`, `steps_store::add`, `record_outcome` sau khi sửa và nguồn ảnh của mỗi lời gọi.
10. Exe release trước/sau (giờ mã privacy được gọi thật: đây là con số thật).

## Không làm
`scanned` của bước, xuất phiên, Lịch sử Chép/Ghim, dòng GIF (Task 132).

## Ghi chú từ Task 127–130 (hợp đồng thật đã gộp)
- Task 128: `ocr::start_scan(image, hwnd, msg, token)` và `ocr::start_scan_with_progress(image, hwnd, msg, progress_msg: Option<u32>, token)`; tiến độ = `PostMessage(hwnd, progress_msg, token, (k<<16)|n)` (không Box). Gỡ các `#[cfg_attr(not(test), allow(dead_code))]` khi đã dùng. Luồng quét không `catch_unwind` và bản release `panic = "abort"`: không thêm đường panic. Sau `Timeout` WinRT có thể còn chạy dải cũ (chỉ bỏ kết quả theo token/thế hệ).
- Task 129: `PreparedOutput::redacted` trả `Err(InvalidImage)` khi plan rỗng HOẶC mọi khung nằm ngoài ảnh → cổng KHÔNG được rơi về ảnh thô: nếu outcome là Apply mà plan không phủ điểm nào, coi như `ContinueNone` chỉ khi người dùng đã xác nhận; nếu không, báo lỗi và không xuất. `Redacted.regions` = số khung plan còn phủ ≥ 1 điểm sau nới+kẹp. `check.ps1` đã có luật "privacy boundary" (hàm tạo chỉ trong `privacy_gate.rs`, chặn bí danh `PreparedOutput as`, `<PreparedOutput>::`) và "no logging" (`e?print(ln)?!`, `dbg!`) cho `privacy_gate.rs`, `review_window.rs`, `privacy/`.
- Task 130: `review_window::WM_PRIVACY_REVIEW = WM_APP + 20` đã định nghĩa ở `review_window.rs`; outcome là `Box<ReviewOutcome>` với token ở WPARAM (`take_outcome(lp)`), có `drain_outcomes(notify, msg)` để vét khi hủy (quyết định 120). Nếu `PostMessageW` tới `notify` lỗi thì cửa sổ đã coi là gửi: cổng phải tự dọn `Pending` (ví dụ khi cửa sổ Duyệt bị hủy mà không có outcome tới — kiểm `IsWindow`/hẹn giờ, hoặc `review_window` gọi lại cổng). Gỡ `#![cfg_attr(not(test), allow(dead_code))]` ở đầu `review_window.rs`. `ReviewRequest.action: Msg` dùng các `Msg::PrivacyAction{Save,Copy,Upload,Pin,AddStep,ExportSteps}`; `step: Option<(u32,u32)>`.
