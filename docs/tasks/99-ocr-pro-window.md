# Task 99: OCR nâng cao — cửa sổ hai khung, chọn dòng, công tắc

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/99-ocr-pro-window` (từ `feature/ocr-pro-15` sau Task 98) · **Rà soát:** `reviewer` (fable) · **Phụ thuộc:** 98

Thiết kế: `docs/superpowers/specs/2026-11-21-ocr-pro-design.md` mục 2, 6.3–6.7, 7, 7b (đọc hết). Global Constraints và Review Focus: `docs/superpowers/plans/2026-11-21-ocr-pro.md`.

## Cần đọc
`crates/darkshot-windows/src/ocr_window.rs` (toàn bộ: vòng đời, token, vét hàng đợi, `place_avoiding`, topmost quyết định 132, `pre_translate`, bố cục thuần + kiểm thử), `ocr.rs` (`OcrResult.lines_detail`), `crates/darkshot-core/src/ocr_text.rs` (Task 97), `history_window.rs` (cửa sổ con tự vẽ có `WS_VSCROLL` + `DarkMode_Explorer`, cuộn bánh xe), `history_paint.rs` (bộ đệm ngoài màn hình, `AlphaBlend`), `settings.rs` (ô tích tự vẽ `BS_AUTOCHECKBOX`), `main.rs` (cách mở cửa sổ OCR, cách các cửa sổ khác đọc/lưu cấu hình), `ocr.rs::scale_bgra`.

## Files
`ocr_window.rs`; có thể tạo `ocr_view.rs` (khung ảnh); `main.rs`; `i18n.rs` (bỏ `OcrCopy` nếu thôi dùng).

## Yêu cầu
Đúng thiết kế. Điểm chốt:
1. Bố cục thuần mới (cỡ mặc định 960×600, tối thiểu 640×400 DIP, khung ảnh ~58%, khung phải ≥ 280 DIP), kiểm thử không chồng/không cắt ở vi/en, DPI 96–192.
2. Khung ảnh: cửa sổ con `WS_TABSTOP | WS_VSCROLL`, `DarkMode_Explorer`, nền `INPUT`, ảnh vừa khít không quá 1:1 px vật lý, ảnh cao → khớp bề rộng + cuộn (bánh xe, thanh cuộn). DIB đã co lưu theo cỡ; vẽ qua bộ đệm. Ảnh giữ > 4096 px cạnh dài → co giữ (7b.7).
3. Khung dòng, hover, chọn (`AlphaBlend` ~30%), con trỏ bàn tay, nhãn "Đã chép" 1,2 s (timer), viền nét đứt cho dòng đang trỏ khi có tiêu điểm.
4. Chuột/phím theo 2.2–2.4, 6.4, 7b.4. Chép bằng `WinClipboard`; lỗi → thông báo `OcrCopyFailedTitle`.
5. Ba công tắc: giá trị đầu từ cấu hình; đổi → dựng lại ô chữ (`compose_all`), lưu cấu hình ngay qua đường lưu hiện có (`main.rs` đưa vào, ví dụ hàm đặt bằng `set_*` như `set_notifier`). "Tách đoạn" vô hiệu khi "Gộp một dòng" bật. Tự chép theo 7b.6 (thông báo như nút). "Chép tất cả" chép ô chữ và thông báo như nút cũ, nhưng cửa sổ ở lại (khác nút "Chép" cũ vốn đóng cửa sổ), để người dùng còn chọn tiếp trên ảnh. Tự chép cũng không đóng cửa sổ.
6. Giữ nguyên: token, `take_result` đúng một lần, vét hàng đợi, 8 cửa sổ, topmost khi mở từ ảnh ghim (132), Esc đóng (khi không có vùng chọn trong khung ảnh).

## Kiểm thử (viết trước)
- Bố cục thuần (trên).
- Hình học khung ảnh thuần: ảnh→khung (vừa khít, không phóng quá 1:1, ảnh cao + cuộn), đổi điểm khung→px ảnh (dùng cho `line_at`), ba lớp tọa độ khi co giữ.
- Trạng thái chọn thuần: bấm, Ctrl+bấm thêm/bỏ, bấm trống, Ctrl+A, ↑/↓/Enter/Space/Esc → (vùng chọn mới, chữ cần chép hay `None`).
- `live` `#[ignore]` (không đầu vào thật): mở với ảnh tổng hợp chữ Anh, chờ kết quả; gửi `WM_LBUTTONDOWN/UP` tới khung ảnh tại tâm dòng 2 → clipboard = dòng 2; `MK_CONTROL` dòng 1 rồi 3 → `"dòng1\r\ndòng2\r\ndòng3"` hoặc theo vùng chọn; clipboard "x" + Ctrl chọn/bỏ → vẫn "x"; đổi công tắc gửi `BM_CLICK` → ô chữ đổi, cấu hình lưu; Review Focus 1–5 của kế hoạch; mở/đóng 20 lần không rò GDI/USER.
- Kiểm thử cũ của `ocr_window` (sửa theo bố cục mới nếu cần, không bỏ ý).

TDD `test:` rồi `feat:`. `cargo fmt --all`; `powershell -File scripts/check.ps1` → `ALL CHECKS PASSED`; chạy `ocr_window::tests::manual` với `--ignored --test-threads=1`. Build release `--target-dir target/task99`, ghi kích thước (so 2.229.248 byte; > +60 KB thì dừng báo). Chụp PrintWindow cửa sổ có kết quả lưu ra thư mục tạm (không vào repo) để phiên điều phối xem.

## Bổ sung sau rà soát Task 97
Tùy chọn lưu ba khóa OCR từ ảnh chụp cấu hình lúc mở (`settings.rs` ~301). Nếu đổi công tắc OCR trong lúc Tùy chọn đang mở, bấm OK sẽ ghi đè. Sửa ở `main.rs` cạnh khối chép `update_notified` (~1429): trước `config::save`, chép `ocr_single_line`, `ocr_paragraphs`, `ocr_auto_copy` mới nhất từ `a.config` vào cấu hình sắp lưu. Kiểm thử nếu tách được hàm thuần.
