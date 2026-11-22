# Task 128: Privacy Shield — quét OCR trên luồng nền

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/128-privacy-ocr-scan` (từ `feature/privacy-shield`), worktree `D:\2026 - Tuan\shotme-privacy` · **Rà soát:** `reviewer` (fable)

Thiết kế: `docs/superpowers/specs/2026-11-22-sensitive-info-redaction-design.md` mục 5, 10 (OCR), 13, 14 (gạch "OCR"), 18.2, 18.3, 18.5. Global Constraints: kế hoạch `docs/superpowers/plans/2026-11-22-sensitive-info-redaction.md`. Quyết định 119, 120 trong `memory-bank/decisions.md`.

## Cần đọc
`crates/darkshot-windows/src/ocr.rs` (toàn bộ), `crates/darkshot-core/src/ocr_text.rs` (`ocr_scale`, `map_rect`, `OcrWord`/`OcrLine` mới của Task 127), `crates/darkshot-core/src/privacy/` (Task 127: `detect`, `merge_tile`, `ScanReport`, `ScanError`), `crates/darkshot-windows/Cargo.toml` (feature `windows`).

## Files
Sửa `crates/darkshot-windows/src/ocr.rs` (và `Cargo.toml` nếu cần feature `Globalization`: được phép, ghi báo cáo). Không sửa cửa sổ OCR ngoài mức biên dịch.

## Interfaces (hợp đồng với Task 131)
```rust
// ocr.rs
pub const SCAN_TILE_OVERLAP: u32 = 128;
pub const SCAN_TILE_TIMEOUT: Duration = Duration::from_secs(20);
/// Tạo luồng `darkshot-ocr`, quét ảnh: OCR (en-US nếu có, không thì hồ sơ) theo dải, `privacy::detect`
/// trên luồng nền, gửi `Box<Result<ScanReport, ScanError>>` tới `hwnd` bằng `msg` (WPARAM = token).
/// Chữ OCR không bao giờ rời luồng nền.
pub fn start_scan(image: Arc<Image>, hwnd: HWND, msg: u32, token: usize) -> Result<(), String>;
pub unsafe fn take_scan_result(lp: LPARAM) -> Box<Result<ScanReport, ScanError>>;
/// Thông báo tiến độ tùy chọn: PostMessage(hwnd, msg_progress, token, (k << 16) | n) trước mỗi dải.
/// (Nếu làm: tham số `progress_msg: Option<u32>`; ghi rõ trong báo cáo.)
```

## Yêu cầu
1. `recognize_in_mta` giữ khung TỪNG TỪ (đã đọc ở ocr.rs:154–160): `OcrLine.words` có chữ + khung từng từ (px ảnh gốc qua `map_rect`); `OcrLine.rect` = hợp như cũ. Từ có khung hỏng: giữ chữ, khung rỗng (Task 127 sẽ rơi về khung dòng). Cửa sổ OCR vẫn hoạt động như cũ (kiểm thử cũ qua).
2. Dải: nếu `height > MaxImageDimension` (hay `width`), cắt dải ngang cao ≤ max, chồng `SCAN_TILE_OVERLAP`; chiều rộng > max thì co theo tỉ lệ như `ocr_scale` (ghi giới hạn). Mỗi dải: sao chép điểm ảnh dải (một dải trong bộ nhớ một lúc), OCR, `detect`, dịch tọa độ, `merge_tile`. Ảnh vừa: một dải, không sao chép thừa nếu được.
3. Ngôn ngữ: `Language::CreateLanguage("en-US")` + `OcrEngine::IsLanguageSupported` → `TryCreateFromLanguage`; không được thì `TryCreateFromUserProfileLanguages`; không có → `ScanError::NoLanguage`. Tạo engine một lần mỗi lượt quét.
4. Thời hạn mỗi dải `SCAN_TILE_TIMEOUT`: `RecognizeAsync` + chờ có hạn (ví dụ `IAsyncOperation` + `Status()`/`Cancel()` vòng ngủ ngắn, hoặc `SetCompleted` + kênh có `recv_timeout`) — đối chiếu tài liệu crate `windows` 0.6x thật (docs.rs / mã nguồn trong `~/.cargo/registry`), không dựa trí nhớ. Quá hạn → `Cancel()` nếu được, trả `ScanError::Timeout`.
5. Mọi lỗi WinRT → `ScanError::Failed` (không chuỗi, không HRESULT trong kết quả gửi đi). Chữ OCR: bộ đệm chữ dòng/từ ghi đè 0 trước khi thả (dùng hàm `wipe` của Task 127 nếu có); không `eprintln!` chữ. `ScanReport.tiles`, `elapsed_ms` điền đúng.
6. Luồng: theo mẫu `start` (quyết định 119: `Mta::enter`, `keep_mta_alive`); `post_result` tương tự cho kiểu mới, gửi lỗi → tự giải phóng Box. Không gọi trên luồng giao diện.
7. Kiểm thử: hàm thuần cắt dải (biên, chồng, ảnh đúng max, ảnh 3×max+1, chiều rộng > max) — kiểm thử thường; `#[ignore]` (chạy tay, máy có OCR): ảnh tổng hợp vẽ chữ bằng GDI (`GDI_TEST_LOCK`) gồm email/khóa giả → `start_scan` qua cửa sổ thông điệp ẩn (mẫu `MsgWindow` ở ocr.rs:378) → có phát hiện đúng loại, khung nằm trong ảnh, khung từ nằm trong khung dòng; ảnh cao 3×max có chữ ở mỗi dải → đủ phát hiện. Chạy các kiểm thử `#[ignore]` này trên máy (release, nếu OCR có) và ghi kết quả thật.
8. Báo cáo mục 13: phiên bản Windows (`ver`), `MaxImageDimension`, ngôn ngữ engine được chọn, thời gian quét ảnh 1920×1080 và 1000×20000 tổng hợp, exe portable không package identity chạy được hay không. Exe release trước/sau.

## Không làm
Cổng, cửa sổ Duyệt, tô che.
