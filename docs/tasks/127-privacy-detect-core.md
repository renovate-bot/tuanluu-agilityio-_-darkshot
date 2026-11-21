# Task 127: Privacy Shield — lõi dò thông tin nhạy cảm

**Agent:** `builder` · **Model:** opus · **Nhánh:** `task/127-privacy-detect-core` (từ `feature/privacy-shield`), worktree `D:\2026 - Tuan\shotme-privacy` · **Rà soát:** `reviewer` (fable)

Thiết kế: `docs/superpowers/specs/2026-11-22-sensitive-info-redaction-design.md` mục 4, 5, 6 (trừ `PreparedOutput`/`Protection` — Task 129), 11 (hàng "Lộ chữ OCR"), 14 (gạch "Lõi bộ dò"), 18.4. Global Constraints: kế hoạch `docs/superpowers/plans/2026-11-22-sensitive-info-redaction.md` (đọc mục đó).

## Cần đọc
`crates/darkshot-core/src/ocr_text.rs` (`OcrLine`, `map_rect`), `geometry.rs` (`Rect`), `i18n.rs` (macro `messages!`, cách thêm `Msg`, kiểm thử đủ hai ngôn ngữ), `lib.rs`. `grep -rn "OcrLine {" crates/` để sửa mọi chỗ dựng `OcrLine` (thêm trường `words`). Lớp Windows chỉ sửa tối thiểu cho biên dịch: `ocr.rs` dựng `OcrLine` với `words: Vec::new()` hoặc từ đã có (Task 128 làm phần thật).

## Files
Tạo `crates/darkshot-core/src/privacy/mod.rs`, `privacy/model.rs`, `privacy/text.rs` (dựng chữ dòng + bảng ký tự→từ + ghép từ), `privacy/detect/` (một tệp mỗi bộ dò + `mod.rs` đăng ký), `privacy/geom.rs` (hợp khung, dự phòng dòng, ghép phát hiện giữa dải). Sửa `lib.rs`, `ocr_text.rs` (`OcrWord`, trường `words`, `Debug` che chữ), `i18n.rs`, và tối thiểu `crates/darkshot-windows/src/ocr.rs`, `ocr_window.rs`/`ocr_view.rs` nếu chúng dựng `OcrLine`.

## Interfaces (hợp đồng với Task 128–130)
```rust
// ocr_text.rs
pub struct OcrWord { pub text: String, pub rect: Rect }          // Debug tay: không in text
pub struct OcrLine { pub text: String, pub rect: Rect, pub words: Vec<OcrWord> } // Debug tay: "<redacted N chars>"
// privacy/model.rs
pub enum Category { Email, Phone, CreditCard, ApiKey, Token, Password, PrivateKey }
pub enum Confidence { High, Medium }
pub enum Mapping { Word, Line }
pub enum RuleId { /* một biến thể mỗi luật */ }
pub struct Finding { pub id: u32, pub category: Category, pub confidence: Confidence,
                     pub rects: Vec<Rect>, pub rule: RuleId, pub mapping: Mapping }
pub struct ScanReport { pub findings: Vec<Finding>, pub tiles: u32, pub elapsed_ms: u32 }
pub enum ScanError { NoLanguage, Failed, Timeout, TooLarge }
impl Category { pub fn label(self, lang: Lang) -> &'static str }
impl Confidence { pub fn label(self, lang: Lang) -> &'static str }
impl ScanError { pub fn message(self, lang: Lang) -> &'static str }
pub struct Redaction { pub rect: Rect, pub source: RedactionSource, pub selected: bool }
pub enum RedactionSource { Detected { finding: u32, category: Category }, Manual }
pub struct RedactionPlan { pub rects: Vec<Rect> }
// privacy/mod.rs
/// Dò trên các dòng OCR (tọa độ px ảnh gốc). Không giữ chữ trong kết quả.
pub fn detect(lines: &[OcrLine]) -> Vec<Finding>;
/// Dịch phát hiện của một dải (độ dời dy) và gộp với phát hiện đã có (cùng loại, khung giao nhau).
pub fn merge_tile(acc: &mut Vec<Finding>, tile: Vec<Finding>, dy: i32);
pub trait Detector { fn scan(&self, ctx: &LineCtx<'_>, out: &mut Vec<RawMatch>); }  // tên phụ tùy bạn
```
`Category`, `Confidence`, `Mapping`, `RuleId`: `Clone, Copy, Debug, PartialEq, Eq`. `Finding`, `ScanReport`: `Clone, Debug, PartialEq` (an toàn vì không có chữ). `id` đánh lại 0..n sau khi gộp.

## Yêu cầu
1. Luật đúng bảng mục 4 (bảy loại, Cao/Trung bình, tránh nhận nhầm, ưu tiên chồng nhau, chuẩn hóa nhãn bỏ dấu, chuẩn hóa nhầm OCR chỉ trong chuỗi số thẻ). Bộ so khớp viết tay, KHÔNG crate mới. Mỗi bộ dò một tệp, độc lập; nhãn ngữ cảnh "cùng dòng hoặc dòng trên" cho điện thoại.
2. Dựng chữ dòng theo mục 5 + 18.4: hai bản (có khoảng trắng; ghép từ khi khe < 0,35 × cao dòng và hai ký tự biên thuộc bộ token), bảng `(start, end, word)` theo chỉ số `char`, không theo byte. Khung = hợp khung các từ giao khoảng trùng; từ không có khung hợp lệ hoặc `words` rỗng → khung dòng, `Mapping::Line`. PEM: một `Finding` nhiều `rects` (mỗi dòng một khung).
3. Mật khẩu: chỉ che giá trị (trừ nhãn và giá trị cùng một từ). JWT: giải mã base64url đoạn đầu trong biến cục bộ (tự viết, hoặc `base64` đã có trong `darkshot-core`), không giữ lại.
4. Không trường chữ nào trong kiểu kết quả. Không `eprintln!`/`println!` trong `privacy/` (kể cả kiểm thử; dùng `assert!` với thông điệp không chứa chữ mẫu). Bộ đệm chữ tạm (`String`/`Vec<char>`) ghi đè bằng 0 trước khi thả trong `detect` (hàm nhỏ `wipe(&mut String)` dùng `unsafe { as_bytes_mut }` có SAFETY hoặc `into_bytes` rồi `fill(0)`).
5. Chuỗi `i18n` (vi + en): tên 7 loại ("Email", "Số điện thoại"/"Phone number", "Thẻ thanh toán"/"Payment card", "Khóa API"/"API key", "Token truy cập"/"Access token", "Mật khẩu"/"Password", "Khóa riêng"/"Private key"), "Cao"/"High", "Trung bình"/"Medium", 4 lỗi `ScanError` (không ngôn ngữ OCR / không nhận dạng được / quét quá lâu / ảnh quá lớn).
6. `OcrLine`/`OcrWord` giữ `PartialEq` (kiểm thử cũ), `Debug` tay không in chữ; kiểm thử khẳng định `format!("{:?}", line)` không chứa chữ.
7. Kiểm thử (mục 14, chỉ dữ liệu giả; chuỗi dạng khóa thật ghép lúc chạy bằng `concat!`/`format!` để GitHub secret scanning không báo): mỗi loại đúng/sai; VN di động/cố định, `+84`, quốc tế E.164; Luhn đúng (`4111 1111 1111 1111`, Amex thử `3782 822463 10005`) / sai; nhầm OCR `4111 1111 1111 111l`; JWT (`eyJhbGciOiJIUzI1NiJ9` + payload giả + chữ ký giả) và `eyJ` hỏng; khóa API từng tiền tố; `password: hunter2`, `Password` một mình, `password: ******`; PEM có/không END; nhiều phát hiện một dòng; nhiều dòng; ưu tiên chồng nhau (JWT trong `Authorization: Bearer`); ánh xạ ký tự→từ, ghép từ bị tách, hợp khung, dự phòng dòng; `merge_tile` gộp phát hiện ở vùng chồng 128 px; bộ dương tính giả (ngày, giờ, IP, phiên bản, số tiền, mã băm git 40 hex, UUID, `@mention`, `x@2x.png`, số 16 chữ số không nhóm không tiền tố).
8. Đo kích thước exe release trước/sau (`cargo build --release -p darkshot-windows`), ghi vào báo cáo. Tăng > 50 KB ở task này thì báo.

## Không làm
OCR thật, dải, luồng (Task 128); `PreparedOutput`, tô che (129); cửa sổ (130); nối luồng xuất (131).
