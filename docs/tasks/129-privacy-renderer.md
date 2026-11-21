# Task 129: Privacy Shield — tô che và PreparedOutput

**Agent:** `builder` · **Model:** opus · **Nhánh:** `task/129-privacy-renderer` (từ `feature/privacy-shield`), worktree `D:\2026 - Tuan\shotme-privacy-r` · **Rà soát:** `reviewer` (fable)

Thiết kế: `docs/superpowers/specs/2026-11-22-sensitive-info-redaction-design.md` mục 6 (`Protection`, `PreparedOutput`), 7, 11, 14 (gạch "Renderer"), 15. Global Constraints: kế hoạch `docs/superpowers/plans/2026-11-22-sensitive-info-redaction.md`.

## Cần đọc
`crates/darkshot-core/src/image.rs` (`Image`, `encode_png`, `encode_jpeg`, `encode_for_upload`), `geometry.rs` (`Rect`), `privacy/mod.rs` + `privacy/model.rs` (Task 127), `scripts/check.ps1`.

## Files
Tạo `crates/darkshot-core/src/privacy/redact.rs`, `privacy/prepared.rs`; sửa `privacy/mod.rs`, `scripts/check.ps1`.

## Interfaces (hợp đồng với Task 130–132)
```rust
// redact.rs
pub const MIN_PAD: i32 = 3;
pub fn pad_for(r: Rect) -> i32;                                   // max(3, ceil(0.15 * h))
/// Nới, kẹp vào w×h, bỏ khung rỗng/âm, gộp khung giao/chạm. Kết quả rời nhau, trong ảnh.
pub fn normalize(rects: &[Rect], w: u32, h: u32) -> Vec<Rect>;
/// Ảnh mới: mọi điểm trong normalize(rects) = B0 G0 R0 A255; ngoài giữ nguyên. src không đổi.
pub fn flatten(src: &Image, rects: &[Rect]) -> Result<Image, RedactError>;
pub enum RedactError { InvalidImage, OutOfMemory }               // Debug, không chuỗi
// prepared.rs
pub enum Protection { Disabled, NoFindings, Redacted { regions: u32 }, ReviewedNoneApplied, SkippedAfterError { regions: u32 } }
impl Protection { pub fn scanned(&self) -> bool }                // NoFindings | Redacted | ReviewedNoneApplied
pub struct PreparedOutput { image: Image, protection: Protection } // trường riêng tư, không Clone
impl PreparedOutput {
    pub fn unscanned(image: Image) -> Self;                       // Disabled
    pub fn no_findings(image: Image) -> Self;
    pub fn reviewed_none(image: Image) -> Self;
    /// Làm phẳng plan; plan rỗng → lỗi lập trình: trả Err (dùng reviewed_none/skipped thay).
    pub fn redacted(image: Image, plan: &RedactionPlan) -> Result<Self, RedactError>;
    /// OCR lỗi, người dùng xác nhận đi tiếp; plan có thể rỗng (regions = 0).
    pub fn skipped_after_error(image: Image, plan: &RedactionPlan) -> Result<Self, RedactError>;
    pub fn image(&self) -> &Image;
    pub fn protection(&self) -> Protection;
    pub fn into_image(self) -> Image;
}
```
`Protection`: `Clone, Copy, Debug, PartialEq, Eq`. `PreparedOutput`: `Debug` tay chỉ in kích thước + protection.

## Yêu cầu
1. `normalize`: khung có `w <= 0 || h <= 0` bỏ; nới `pad_for` mỗi phía; kẹp; gộp lặp tới điểm cố định (giao hoặc chạm cạnh); phép tính dùng `i64`/`saturating` (không tràn với khung cực lớn/âm).
2. `flatten`: kiểm `src.is_valid()`; cấp bộ đệm bằng `try_reserve_exact` (→ `OutOfMemory`); chép theo hàng, ghi đen bằng `chunks_exact_mut(4)` theo hàng của khung (không vòng từng điểm toàn ảnh). Nguồn có alpha < 255 vẫn ra A=255 trong khung.
3. Không đổi `encode_*`. Kiểm thử khẳng định: PNG từ `encode_png(flatten(..))` giải mã (crate `png` đã có) ra đen tuyệt đối ở vùng che; không chunk `tEXt`/`iTXt`/`zTXt`/`eXIf` (duyệt chunk thô); JPEG (`encode_jpeg`, chất lượng 90 và 50): không có bộ giải JPEG trong phụ thuộc (không thêm), nên chỉ khẳng định byte JPEG không có đoạn APP1 (EXIF/XMP) hay COM; ghi giới hạn này trong báo cáo.
4. Kiểm thử thêm: mọi điểm trong khung = (0,0,0,255), ngoài khung không đổi so với src; nới/kẹp/gộp; khung rỗng/âm/ngoài ảnh/`i32::MAX`; ảnh 1×1; nhiều khung (1000 khung ngẫu nhiên có hạt giống cố định, so với tham chiếu từng điểm); ảnh lớn tổng hợp 8000×20000 chạy trong `#[ignore]` (ghi thời gian ở báo cáo, release); src không đổi (so hash/bản sao); khung rất nhỏ 1×1 vẫn che ≥ 7×7 nhờ nới.
5. `scripts/check.ps1`:
   a. Kiểm "no hard-coded Vietnamese strings" quét đệ quy `crates\*\src\**\*.rs` (giữ quy tắc bỏ qua `mod tests` và `i18n.rs`). Sửa nếu file cũ trong thư mục con vỡ (báo cáo).
   b. Mới "privacy boundary": các lời gọi `PreparedOutput::unscanned|no_findings|reviewed_none|redacted|skipped_after_error` ngoài `crates\darkshot-core\src\privacy\` và ngoài `crates\darkshot-windows\src\privacy_gate.rs` → thất bại (bỏ qua khối `mod tests` theo cùng cách). Hiện chưa có `privacy_gate.rs`: luật vẫn chạy (không có lời gọi nào ngoài lõi).
   c. Mới "no logging in privacy": `eprintln!|println!|dbg!` trong `crates\darkshot-core\src\privacy\**`, `privacy_gate.rs`, `review_window.rs` (nếu tồn tại) → thất bại, KỂ CẢ trong `mod tests`.
   Giữ file thuần ASCII như hiện nay.
6. Đo exe release trước/sau.

## Không làm
Cửa sổ, cổng, nối luồng xuất, OCR.
