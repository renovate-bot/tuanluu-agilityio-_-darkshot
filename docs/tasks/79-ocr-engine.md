# Task 79: OCR cục bộ — bộ nhận dạng WinRT (`ocr.rs`)

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/79-ocr-engine` · **Rà soát:** `reviewer` (fable) · **Phụ thuộc:** 78

Thiết kế: `docs/superpowers/specs/2026-11-18-local-ocr-design.md` mục 2.4–2.7, 3, 4. Global Constraints: kế hoạch `docs/superpowers/plans/2026-11-18-local-ocr.md`.

## Cần đọc
`crates/darkshot-core/src/ocr_text.rs`, `image.rs` (`Image { width, height, bgra }`); `crates/darkshot-windows/Cargo.toml`; mã nguồn crate `windows` 0.62.2 và `windows-future` trong `~/.cargo/registry/src/*/` (`Media/Ocr`, `Graphics/Imaging` `SoftwareBitmap::CreateCopyFromBuffer` hoặc `Create` + `CopyFromBuffer`, `Storage/Streams` `DataWriter`/`IBuffer`, `IAsyncOperation::get`/`join`, `RoInitialize`/`CoInitializeEx`); `app.rs` (`spawn_upload` làm mẫu tạo luồng nền và báo về); `history_window.rs` `post_box` (mẫu `Box` qua `PostMessage`). Đối chiếu tài liệu Microsoft cho `OcrEngine`, `SoftwareBitmap` (OCR cần `Bgra8` hoặc `Gray8`).

## Files
Tạo `crates/darkshot-windows/src/ocr.rs`; sửa `Cargo.toml` (feature `windows`), `main.rs` (chỉ `mod ocr;`).

## Interfaces
```rust
pub struct OcrResult { pub text: String, pub lang_tag: String, pub lang_name: String, pub lines: usize }
pub enum OcrError { NoLanguage, Failed(String) }
/// Đồng bộ, gọi trên luồng nền. Tự khởi tạo/hủy WinRT apartment MTA cho luồng gọi.
pub fn recognize(image: &darkshot_core::image::Image) -> Result<OcrResult, OcrError>;
/// Tạo luồng `darkshot-ocr`, chạy `recognize`, gửi `Box<Result<OcrResult, OcrError>>` tới `hwnd` bằng `msg`
/// (LPARAM = con trỏ Box; WPARAM = `token` để bên nhận khớp yêu cầu). Gửi lỗi → tự giải phóng.
/// Không tạo được luồng → Err.
pub fn start(image: darkshot_core::image::Image, hwnd: HWND, msg: u32, token: usize) -> Result<(), String>;
/// Bên nhận lấy lại quyền sở hữu: gọi đúng một lần cho mỗi thông điệp nhận được.
pub unsafe fn take_result(lp: LPARAM) -> Box<Result<OcrResult, OcrError>>;
```

## Yêu cầu
1. Thu/phóng bằng `ocr_scale(w, h, OcrEngine::MaxImageDimension())` trước khi tạo `SoftwareBitmap` (co giãn bằng thuật toán đơn giản trong Rust: hộp trung bình khi thu, láng giềng gần nhất hoặc song tuyến khi phóng ×2 — đặt hàm thuần ở `ocr_text.rs` hay `ocr.rs` kèm kiểm thử).
2. Điểm ảnh: BGRA8, alpha 255 (ảnh chụp đã đục); `BitmapAlphaMode::Premultiplied` hoặc `Ignore` theo tài liệu OCR.
3. `TryCreateFromUserProfileLanguages()` lỗi/None → `NoLanguage`. `RecognitionLanguage` → `LanguageTag`, `DisplayName`.
4. Dòng: `result.Lines()` → `OcrLine::Text()` → `join_lines`; `lines = line_count`.
5. Không `unwrap`/`expect`; mọi lỗi WinRT → `Failed(message ngắn)` (chỉ dùng cho nhật ký nội bộ/kiểm thử; giao diện hiện câu i18n).
6. Đo kích thước `darkshot.exe` release trước/sau thêm feature (`--target-dir target/task79`), ghi trong báo cáo.
7. Thoát ứng dụng khi luồng OCR đang chạy: luồng tự kết thúc; không ai join nó; `PostMessage` tới HWND đã hủy thất bại → Box được giải phóng.

## Kiểm thử
- Thuần: hàm co giãn (cỡ, giá trị điểm ảnh ở ảnh nhỏ).
- Chạy tay `#[ignore]` (không cần chuột, không mở cửa sổ hiện): dựng ảnh trắng có chữ đen bằng GDI (`DrawTextW` "Hello darkshot 123" phông 32 px vào DIB) → `recognize` → chữ chứa "Hello" và "123" (không phân biệt hoa thường); ảnh trắng trơn → `text == ""`; `start` tới một cửa sổ message-only tạm, nhận đúng một kết quả, giải phóng. Ghi lệnh chạy; chạy một lượt và báo kết quả thật (nếu máy không có gói OCR → ghi rõ `NoLanguage`).

TDD `test:` rồi `feat:`. `cargo fmt --all`; `powershell -File scripts/check.ps1` → `ALL CHECKS PASSED`.

## Review focus
Apartment WinRT đúng luồng và được hủy; `.get()` không chạy trên luồng giao diện; sở hữu Box; không panic; feature tối thiểu; kích thước exe.
