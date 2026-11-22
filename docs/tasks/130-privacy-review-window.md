# Task 130: Privacy Shield — cửa sổ Duyệt

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/130-privacy-review-window` (từ `feature/privacy-shield`), worktree `D:\2026 - Tuan\shotme-privacy-w` · **Rà soát:** `reviewer` (fable)

Thiết kế: `docs/superpowers/specs/2026-11-22-sensitive-info-redaction-design.md` mục 3.2 (bước 4–7), 3.3, 3.4, 10, 14 (gạch "Cửa sổ Duyệt"), 18.1 (tiêu đề "Bước k/n"), 18.2 (dải k/n), 18.5. Global Constraints: kế hoạch `docs/superpowers/plans/2026-11-22-sensitive-info-redaction.md`. Quyết định 120, 121, 133 trong `memory-bank/decisions.md`.

## Cần đọc
`crates/darkshot-windows/src/ocr_window.rs` (mẫu cửa sổ tối đổi cỡ, DPI, `pre_translate`, Box trong LPARAM, đặt vị trí, đếm GDI trong kiểm thử, PrintWindow), `ocr_view.rs` (vẽ ảnh co + khung), `theme.rs`, `winutil.rs`, `history_paint.rs` (`Bmp`), `darkshot-core/src/theme.rs`, `i18n.rs`, `privacy/` (Task 127, 129). Chỉ đọc phần cần.

## Files
Tạo `crates/darkshot-core/src/privacy/review.rs` (mô hình thuần), `crates/darkshot-windows/src/review_window.rs`; sửa `privacy/mod.rs`, `i18n.rs`, `crates/darkshot-windows/src/main.rs` (chỉ `mod review_window;` + nối `pre_translate` vào vòng thông điệp như `ocr_window::pre_translate`, nếu cần), `help.rs` nếu Trợ giúp liệt kê phím.

## Interfaces (hợp đồng với Task 131, 132)
```rust
// darkshot-core/src/privacy/review.rs — thuần, kiểm thử được
pub enum ReviewPhase { Scanning { tile: Option<(u32, u32)> }, Findings, Error(ScanError) }
pub enum Confirm { None, EmptySelection, Unscanned }
pub enum ReviewOutcome { Apply(RedactionPlan), ContinueNone, ContinueUnscanned(RedactionPlan), Retry, Cancel }
pub struct ReviewModel { /* findings → Redaction (mọi mục selected), manual, phase, confirm, hover, drag */ }
impl ReviewModel {
    pub fn new(phase: ReviewPhase, findings: Vec<Finding>, image_w: u32, image_h: u32) -> Self;
    pub fn set_result(&mut self, r: Result<Vec<Finding>, ScanError>);   // Scanning → Findings/Error; giữ vùng tự thêm
    pub fn items(&self) -> &[Redaction];
    pub fn toggle(&mut self, i: usize); pub fn select_all(&mut self); pub fn deselect_all(&mut self);
    pub fn add_manual(&mut self, r: Rect) -> bool;    // kẹp vào ảnh; < 4×4 → false
    pub fn remove(&mut self, i: usize) -> bool;       // chỉ mục Manual
    pub fn hit(&self, x: i32, y: i32) -> Option<usize>;  // px ảnh; mục trên cùng/nhỏ nhất
    /// Nút chính: Findings → Apply(plan) | Confirm::EmptySelection; Error → Confirm::Unscanned.
    pub fn primary(&mut self) -> Option<ReviewOutcome>;
    pub fn confirm_yes(&mut self) -> Option<ReviewOutcome>; // EmptySelection → ContinueNone; Unscanned → ContinueUnscanned(plan đang chọn)
    pub fn confirm_back(&mut self);
    pub fn selected_rects(&self) -> Vec<Rect>;        // cho vẽ ô đen trên ảnh xem trước
}
// darkshot-windows/src/review_window.rs
pub struct ReviewRequest { pub token: usize, pub image: Arc<Image>, pub action: Msg, pub step: Option<(u32, u32)>, pub phase: ReviewPhase, pub findings: Vec<Finding> }
/// Mở cửa sổ; kết quả người dùng gửi `Box<(usize, ReviewOutcome)>` bằng PostMessage(notify, msg, token, ptr).
pub fn open(req: ReviewRequest, notify: HWND, msg: u32) -> Result<(), String>;
pub unsafe fn take_outcome(lp: LPARAM) -> Box<ReviewOutcome>;
pub fn update(token: usize, r: Result<Vec<Finding>, ScanError>) -> bool; // false nếu không có cửa sổ
pub fn progress(token: usize, tile: u32, tiles: u32);
pub fn close(token: usize);          // đóng không gửi outcome
pub fn close_all();                  // thoát ứng dụng
pub fn pre_translate(msg: &MSG) -> bool;
```
Cửa sổ gửi outcome đúng MỘT lần (Cancel khi đóng bằng ✕/Alt+F4/Esc nếu chưa gửi gì); `close(token)` từ cổng không gửi. Sau `Retry` cửa sổ ở lại, về `Scanning`, chờ `update`.

## Yêu cầu
1. Bố cục mục 3.3/3.4 (wireframe là gợi ý, số đo tự chọn), tối, `theme::*`, mặc định 960×640 DIP kẹp vùng làm việc, tối thiểu 640×420 DIP, giữa màn hình có con trỏ, topmost + lên trước. DPI 96/120/144/192 và `WM_DPICHANGED`.
2. Ảnh xem trước co vừa (≤ 100%), vùng chọn ô đen ĐẶC, vùng bỏ chọn viền đứt nét, mục trỏ/chọn viền `ACCENT`; bấm vùng trên ảnh = toggle. Chế độ Thêm vùng che: kéo vẽ khung xem trước, thả → `add_manual`; Esc thoát chế độ kéo. Không bao giờ hiện chữ/giá trị; nhãn = `Category::label`, `Confidence::label`, "Vùng tự thêm".
3. Danh sách tự vẽ (không dùng ListView nếu tự vẽ gọn hơn): ô chọn, tên, độ tin cậy, nút Xóa cho mục tự thêm; cuộn khi dài; ↑/↓/Space/Ctrl+A/Tab; Enter = nút chính, Esc = Hủy.
4. Thanh xác nhận trong cửa sổ (không MessageBox) cho `Confirm::EmptySelection` và `Confirm::Unscanned` (chữ mục 3.3, 3.4).
5. Trạng thái Scanning: "Đang quét thông tin nhạy cảm…" (+ "dải k/n" khi có), chỉ Hủy; Error: lý do = `ScanError::message`, nút Hủy / Thử lại / Thêm vùng che / Tiếp tục không quét. Tiêu đề có "Bước k/n" khi `step` có; tóm tắt có tên hành động (`action`).
6. Chuỗi mới vi + en trong `i18n.rs` (tiêu đề, tóm tắt "Tìm thấy N mục", nút, xác nhận, "Đang quét…", ghi chú "Tự động có thể bỏ sót. Hãy xem lại ảnh.", "Vùng tự thêm", tên hành động nếu chưa có).
7. Tài nguyên: một bitmap xem trước co sẵn (tạo lại khi đổi cỡ), font/brush giải phóng ở `WM_DESTROY`; ảnh `Arc` thả khi đóng. Vét hàng đợi Box khi hủy (quyết định 120). Không `eprintln!`.
8. Kiểm thử: `ReviewModel` (chọn mặc định, toggle, chọn/bỏ hết, thêm/xóa tay, khung nhỏ/ngoài ảnh, `hit`, `primary`/`confirm_*` mọi nhánh, `set_result` giữ vùng tay, Retry) — kiểm thử thường; cửa sổ: mở/đóng 20 lần không rò GDI/USER (đếm như ocr_window), outcome gửi đúng một lần khi `WM_CLOSE`, `update` chuyển trạng thái, bố cục 4 DPI không chồng nút, chuỗi hai ngôn ngữ, ảnh PrintWindow (lưu vào scratchpad của agent, KHÔNG vào repo) xem bằng mắt. Không giả lập chuột/phím thật: gửi `WM_LBUTTONDOWN/UP`, `WM_KEYDOWN` thẳng tới HWND của cửa sổ.
9. Chưa nối vào luồng xuất (Task 131). Đo exe trước/sau (cửa sổ chưa được gọi thì LTO có thể bỏ: ghi rõ).
