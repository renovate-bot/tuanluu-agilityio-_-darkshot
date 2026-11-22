# Task 133: Privacy Shield — gia cố, đo đạc, hồi quy, demo

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/133-privacy-hardening` (từ `feature/privacy-shield`), worktree `D:\2026 - Tuan\shotme-privacy` · **Rà soát:** `reviewer` (fable), sau đó rà soát toàn đợt `specialist` (fable)

Thiết kế: `docs/superpowers/specs/2026-11-22-sensitive-info-redaction-design.md` (toàn bộ, nhất là 4, 11, 12, 14, 15, 16). Global Constraints và Review Focus: `docs/superpowers/plans/2026-11-22-sensitive-info-redaction.md`. Không thêm loại phát hiện mới.

## Cần đọc
`crates/darkshot-core/src/privacy/` (toàn bộ), `crates/darkshot-windows/src/privacy_gate.rs`, `review_window.rs`, `ocr.rs` (phần quét), các sink trong `main.rs`/`actions.rs`/`app.rs`/`history_window.rs`/`steps_*`, `scripts/check.ps1`, `scripts/demo-ocr.ps1` (mẫu demo cách ly).

## Files
Sửa `crates/darkshot-core/src/privacy/**` (kiểm thử + tinh chỉnh luật đã nêu dưới), `scripts/check.ps1`; tạo `scripts/demo-privacy.ps1`, `docs/testing/privacy-quality.md` (số đo thật); sửa mã khác chỉ khi sửa lỗi tìm được (ghi rõ trong báo cáo).

## Yêu cầu
1. **Audit bảo mật** (ghi bảng trong báo cáo): chữ OCR chỉ trong bộ nhớ luồng `darkshot-ocr`; `Finding`/`ScanReport`/lỗi/`Debug` không chữ; không log; không ghi JSON/cấu hình/Lịch sử/metadata; không tệp tạm chứa ảnh gốc; không đường xuất vòng qua `PreparedOutput` (liệt kê lại mọi sink); không ảnh bước thô; Lịch sử cũ quét lại; Hủy không xuất; OCR lỗi không thành sạch; clipboard/R2 chỉ nhận ảnh đã duyệt; vùng che đặc và phẳng. Xem cả thông điệp panic, `expect` có chuỗi chứa dữ liệu.
2. **Siết `scripts/check.ps1`** (giữ ASCII):
   a. Bỏ chú thích `//` mà KHÔNG xóa phần trong chuỗi (bỏ nội dung chuỗi `"..."` trước, rồi mới cắt `//`).
   b. `Test-TestMod` không khớp dòng chú thích (`// #[cfg(test)]`, `/// …`).
   c. Bắt `type X = PreparedOutput`, `impl … for PreparedOutput` ngoài `privacy/`.
   d. Luật mới "raw image sinks": lời gọi `\.set_image\(`, `run_action\(`, `spawn_upload\(`, `record_outcome(_arc)?\(`, `encode_for_upload\(` chỉ được phép trong danh sách tệp cho phép (`clipboard.rs`, `actions.rs`, `app.rs`, `history_service.rs`, `main.rs` — chỉ trong `run_on_image`/`gate_output`/`save_pending_uploads` thì khó grep: cho phép theo tệp và in danh sách lời gọi trong `main.rs` để rà soát đọc), và `history_window.rs` chỉ trong nhánh dispatch của cổng (Task 132). Thử cài lỗi tạm cho từng luật rồi hoàn lại.
3. **Chất lượng bộ dò** (synthetic, `privacy/quality_tests.rs` hoặc tương tự): bộ mẫu dương tính ≥ 20 mỗi loại (biến thể định dạng, nhóm, nhãn vi/en, lỗi OCR phổ biến `I/l/1`, `O/0`, `S/5`) và bộ âm tính ≥ 150 dòng (ngày, giờ, IP, phiên bản, tiền, mã băm, UUID, đường dẫn, `env` thường gặp như `PWD=/home/x`, `SECRETARY=…`, văn bản thường vi/en). Tính precision, recall, tỉ lệ nhận nhầm MỖI loại và tổng — số đo thật từ kiểm thử, ghi vào `docs/testing/privacy-quality.md` kèm danh sách ví dụ bỏ sót/nhận nhầm còn lại. Kiểm thử khẳng định ngưỡng tối thiểu = số đo hiện tại (chặn hồi quy), không bịa ngưỡng.
4. **Tinh chỉnh đã biết** (từ rà soát 127/128): OCR đọc `I` thành `l` trong khóa có tiền tố (ví dụ `AKIA…`): chấp nhận `l`/`|` ở vị trí chữ hoa khi kiểm tiền tố và thân khóa AWS/`ghp_` (chuẩn hóa trong bản sao cục bộ); `PWD=/home/…` và `OLDPWD` không phải mật khẩu (giá trị là đường dẫn tuyệt đối → bỏ); `SECRETARY` không khớp (đòi `SECRET` là đoạn hoặc theo sau `_`/`KEY`/hết tên); `MASTERKEY`/`SSHKEY` nhận (đuôi `KEY` với tiền tố ≥ 3 chữ, trừ danh sách loại `MONKEY`, `HOTKEY`, `TURKEY`, `DONKEY`, `KEYBOARD`); `+84(0)912345678` nhận. Mỗi điều chỉnh có kiểm thử và cập nhật số đo mục 3.
5. **Renderer** bổ sung kiểm thử: biên ảnh, nguồn trong suốt (alpha 0) → vùng che A=255, khung cắt dở ở bốn cạnh, ảnh lớn, mã hóa lại PNG→giải mã→mã hóa lại vẫn đen; JPEG: thêm kiểm thử giải mã bằng WIC trong `darkshot-windows` (`#[ignore]` nếu cần, `IWICImagingFactory` đã có feature?) để đo vùng che sau JPEG chất lượng 90/50: mọi kênh ≤ 8 trong vùng co vào 4 px — nếu feature WIC chưa có thì KHÔNG thêm, ghi giới hạn.
6. **Hiệu năng** (release, ghi vào `privacy-quality.md`, số thật): OCR+dò cho ảnh tổng hợp 1920×1080, 3840×2160, 1000×20000; `detect` riêng trên 1000 dòng; `flatten` 4K và 8000×20000; bộ nhớ đỉnh (Working Set qua `GetProcessMemoryInfo`) cho 1000×20000; thời gian khởi động không đổi khi Shield tắt (không khởi tạo gì: kiểm mã); exe release so `b2ee232` (2.335.744) — phân tích phần tăng theo module nếu được (`cargo bloat` KHÔNG cài mới; dùng so sánh build có/không gọi).
7. **Demo cách ly** `scripts/demo-privacy.ps1` theo mẫu `demo-ocr.ps1`: hồ sơ tạm, `privacy_shield: true`, `history_enabled: true`, không R2, không mạng; tệp mẫu Notepad chứa dữ liệu GIẢ của cả 7 loại (email `@example.com`, số thẻ thử, khóa giả có thân `EXAMPLE`, JWT giả, PEM giả rút gọn), in hướng dẫn thử từng đường (Lưu, Chép, Tải → lưu dự phòng, Ghim, Lặp lại, toàn màn hình, chụp cuộn, Thêm bước/xuất, Lịch sử Chép/Ghim) và tham số `-Lang`, `-Off` (Shield tắt để so). Chuỗi dạng khóa thật ghép lúc chạy để GitHub không báo. Chạy thử script tạo hồ sơ (không mở ứng dụng nếu tham số `-NoLaunch`) và ghi kết quả.
8. **Hồi quy**: `check.ps1`; các `#[ignore]` liên quan (ocr, review_window, privacy_gate, pin, settings, history_window, steps_*, scroll_*, record_session nếu có) chạy tay và ghi số thật; GDI/USER trước–sau. Không giả lập chuột/phím thật.
9. Sửa lỗi tìm được (nhỏ) ngay; lỗi lớn: dừng và báo.

## Ghi chú từ rà soát Task 131–132 (đưa vào audit/đo)
- Lịch sử Chép ảnh đã qua cổng ở `history_window::copy_prepared` (gọi từ `main::gate_output`); đó là chỗ `set_image` hợp lệ duy nhất trong `history_window.rs`.
- Đo bộ nhớ xuất phiên: `ExportScan.job.replacements` giữ mọi ảnh đã che tới khi ghi xong (99 bước 4K ≈ 3,3 GB lý thuyết): đo 10 bước 4K, đề xuất (không bắt buộc sửa) nếu quá lớn.
- Shield tắt lúc bấm Xuất rồi bật ngay: luồng xuất vẫn chép thô (đúng spec "Shield bật lúc xuất"): ghi vào "Đã biết".
- Cửa sổ Duyệt ĐÃ mở trước khi bắt đầu chụp cuộn/ghi GIF vẫn có thể lọt vào ảnh: cân nhắc ẩn/loại khỏi ảnh (`WDA_EXCLUDEFROMCAPTURE` như thanh phiên, quyết định 122) — làm nếu nhỏ.
- `config.privacy_shield` sai kiểu/tệp cấu hình hỏng → Shield về tắt: kiểm thông báo cấu hình hỏng hiện có nói gì; ghi "Đã biết".
