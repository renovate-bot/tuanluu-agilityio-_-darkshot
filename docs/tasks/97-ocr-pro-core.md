# Task 97: OCR nâng cao — lõi (dòng, ghép chữ, chia đoạn, dò trúng), cấu hình, chuỗi

**Agent:** `builder` · **Model:** sonnet · **Nhánh:** `task/97-ocr-pro-core` (từ `feature/ocr-pro-15`) · **Rà soát:** `reviewer` (sonnet)

Thiết kế: `docs/superpowers/specs/2026-11-21-ocr-pro-design.md` mục 6.1, 6.2, 6.5, 6.6, 7b. Global Constraints: `docs/superpowers/plans/2026-11-21-ocr-pro.md`.

## Cần đọc
`crates/darkshot-core/src/ocr_text.rs`, `geometry.rs` (`Rect`), `config.rs` (mẫu `history_enabled`: trường, mặc định, đọc/ghi, kiểm thử), `i18n.rs` (khối `Ocr*`, `help` nếu có câu OCR, kiểm thử bảng chuỗi).

## Files
`crates/darkshot-core/src/ocr_text.rs`, `config.rs`, `i18n.rs`.

## Interfaces (Task 98, 99 dùng đúng tên này)
```rust
// ocr_text.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcrLine { pub text: String, pub rect: Rect }          // px ảnh gốc
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TextOptions { pub single_line: bool, pub paragraphs: bool }
pub fn paragraphs(lines: &[OcrLine]) -> Vec<usize>                // chỉ số dòng mở đoạn, luôn có 0 nếu không rỗng
pub fn compose(lines: &[OcrLine], idx: &[usize], o: TextOptions) -> String
pub fn compose_all(lines: &[OcrLine], o: TextOptions) -> String  // = compose mọi chỉ số
pub fn line_at(lines: &[OcrLine], x: i32, y: i32) -> Option<usize>
/// Đổi khung trong ảnh cỡ (fw,fh) về ảnh cỡ (tw,th): tỉ lệ riêng từng trục, làm tròn ra ngoài, kẹp trong (0,0,tw,th).
pub fn map_rect(r: Rect, fw: u32, fh: u32, tw: u32, th: u32) -> Rect
// config.rs: Config { ocr_single_line: bool, ocr_paragraphs: bool, ocr_auto_copy: bool }
// i18n.rs: Msg::{OcrCopyAll, OcrSingleLine, OcrParagraphs, OcrAutoCopy, OcrPickHint, OcrCopiedTip}
```

## Yêu cầu
1. `compose`: sắp `idx`, bỏ trùng và chỉ số ngoài dải; trim từng dòng; `single_line` → nối `" "` (bỏ qua `paragraphs`); ngược lại nối `"\r\n"`, `paragraphs` thêm một `"\r\n"` trước dòng mở đoạn (theo `paragraphs(lines)`) trừ dòng đầu tiên của kết quả.
2. `paragraphs` theo thiết kế 6.2 + 7b.9: trung vị chiều cao dòng `m`; dòng i mở đoạn khi `top(i) − bottom(i−1) > 0,8·m` hoặc `top(i) < top(i−1) − 0,5·m`.
3. `line_at`: khung nới 2 px mỗi phía; trả chỉ số nhỏ nhất chứa điểm.
4. `join_lines`: giữ nếu `ocr.rs` còn dùng; Task 98 sẽ chuyển sang `compose_all`. Kiểm thử cũ phải qua.
5. Cấu hình: ba khóa bool độc lập, thiếu/sai kiểu → `false`, ghi lại khi lưu, tệp bản cũ đọc được (theo đúng bộ kiểm thử của `history_enabled`).
6. Chuỗi theo bảng 6.6. Trợ giúp: thêm câu bấm khung/Ctrl+bấm và câu "Windows chưa có bộ nhận dạng tiếng Việt, chữ có dấu có thể bị mất dấu." (en: "Windows has no Vietnamese text recognizer yet, so accented letters may lose their marks."). `OcrCopy` để nguyên (Task 99 bỏ nếu không dùng).

## Kiểm thử (viết trước)
- `compose`: ba dòng `["a","b","c"]` không tùy chọn → `"a\r\nb\r\nc"`; `single_line` → `"a b c"`; idx `[2,0,0,9]` → `"a\r\nc"`; idx rỗng → `""`; hai đoạn (khe lớn giữa b và c) với `paragraphs` → `"a\r\nb\r\n\r\nc"`; chỉ chọn `[2]` → `"c"` (không dòng trống đầu).
- `paragraphs`: liền nhau → `[0]`; khe > 0,8·m → `[0,2]`; cột hai (top nhảy lên) → mở đoạn; lệch top 0,3·m trên cùng hàng → không mở; rỗng → `[]`; một dòng → `[0]`.
- `line_at`: trúng trong, trúng ở nới 2 px, trượt ở 3 px, chồng lấn → chỉ số nhỏ.
- `map_rect`: (4000×1000 → 2000×500) chia đôi; phóng ×2 ngược lại; làm tròn ra ngoài (khung 1×1 lẻ vẫn ≥ 1 px); kẹp mép.
- Cấu hình thiếu/sai kiểu/khứ hồi cho từng khóa.
- i18n: mọi khóa mới có hai ngôn ngữ, không rỗng.

TDD `test:` rồi `feat:`. `cargo fmt --all`; `powershell -File scripts/check.ps1` → `ALL CHECKS PASSED`.
