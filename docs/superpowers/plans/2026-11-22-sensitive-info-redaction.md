# Kế hoạch đợt 21: Privacy Shield — che thông tin nhạy cảm trước khi xuất ảnh

Thiết kế: `docs/superpowers/specs/2026-11-22-sensitive-info-redaction-design.md` (chờ chủ dự án duyệt). Nhánh đợt `feature/privacy-shield` (tách từ `main` tại `b2ee232`), worktree `D:\2026 - Tuan\shotme-privacy`. Task gộp vào nhánh đợt trên máy; xong đợt: đẩy nhánh, PR vào `main` (quyết định 138). Không tag, không phát hành.

Số task: 115–121 giữ cho MCP, 122–126 đợt 20, nhánh macOS trên `origin` tới `task/115-…`; trước khi tạo mỗi nhánh vẫn `git fetch --prune origin` + `git branch -r` để không trùng. Quyết định mới từ 144. Gói task `docs/tasks/127-…` đến `134-…` viết lúc bắt đầu từng task (chép mục Global Constraints dưới đây).

Ánh xạ lời giao: Prompt 1 → Task 127, 128; Prompt 2 → Task 129; Prompt 3 → Task 130; Prompt 4 → Task 131, 132; Prompt 5 → Task 133, 134.

## Global Constraints

Như mục cùng tên của `docs/superpowers/plans/2026-11-21-ocr-pro.md` (và qua đó `2026-11-18-local-ocr.md`), cộng:
- Không crate mới (kể cả `regex`), không feature `windows` mới trừ khi thật cần (`Windows.Globalization.Language` cho `en-US` nếu chưa bật: báo phiên điều phối). Exe release đo trước/sau mỗi task (gốc `b2ee232`: 2.335.744 byte); cả đợt tăng quá 150 KB thì dừng, báo. Mục tiêu ≤ 80 KB.
- Chữ OCR và giá trị trùng khớp: chỉ trong bộ nhớ của luồng `darkshot-ocr`, không bao giờ vào `Finding`, `ScanReport`, `ScanError`, thông điệp lỗi, `Debug`, `eprintln!`, tệp, JSON, Lịch sử, metadata ảnh. Không `eprintln!`/log trong `privacy/`, `privacy_gate.rs`, `review_window.rs` (kể cả kiểm thử in chữ OCR).
- Không gọi `ocr::recognize`/quét trên luồng giao diện (quyết định 119). Cổng và bảng `Pending` chỉ trên luồng chính.
- Vùng che: điểm ảnh (0,0,0,255) làm phẳng vào ảnh mới; không dùng `Shape::Blur`, không lớp trong suốt.
- Khi Shield bật: không ảnh nào tới `run_on_image`, `pin::open`, `steps_store::add`, clipboard ảnh của Lịch sử mà không phải `PreparedOutput` do `privacy_gate` tạo. Khi Shield tắt: hành vi giống hệt trước.
- Dữ liệu kiểm thử: giả, tự sinh; chuỗi trông như khóa thật ghép lúc chạy. Không ảnh chụp thật, không khóa R2, không tải lên thật (dùng `Uploader` giả).
- WM_APP đã dùng +1..+18: `WM_PRIVACY_SCAN = WM_APP + 19`, `WM_PRIVACY_REVIEW = WM_APP + 20`.
- Cấu hình: thêm đúng một khóa `privacy_shield` (mặc định `false`) theo mẫu khóa mới; `session.json` của các bước thêm trường tùy chọn `scanned` (thiếu = `false`), vẫn v1.
- Mọi chữ qua `i18n` (vi, en); màu `theme::*`; DPI 96–192; đếm GDI/USER trong kiểm thử cửa sổ; `GDI_TEST_LOCK`.
- QA không giả lập chuột/phím thật (quyết định 111, 121).

## Bảng task và phân bổ model

| Task | Nhánh | Agent | Model | Lý do chọn model | Nội dung | Phụ thuộc |
|---|---|---|---|---|---|---|
| 127 | task/127-privacy-detect-core | builder | opus | Bảo mật: bộ dò quyết định gì bị lộ; luật nhận nhầm/bỏ sót tinh tế; không dùng model nhẹ | `darkshot-core/src/privacy/` : kiểu mục 6 (trừ `PreparedOutput`), 7 bộ dò, dựng chữ dòng + bảng ký tự→từ + ghép từ (mục 18.4), ưu tiên chồng nhau, hợp khung/dự phòng dòng, ghép dải; `OcrWord`, `Debug` che chữ cho `OcrLine`; nhãn loại/độ tin cậy trong `i18n`; kiểm thử mục 14 | – |
| 128 | task/128-privacy-ocr-scan | specialist | opus | WinRT, luồng nền, tọa độ khi co/dải, chữ không được rời luồng | `ocr.rs`: giữ khung từ; `start_scan(Arc<Image>, hwnd, msg, token)` chạy OCR + bộ dò trên `darkshot-ocr`, chỉ gửi `ScanReport`/`ScanError`; dải chồng 128 px khi vượt `MaxImageDimension`; ưu tiên `en-US`; 20 s mỗi dải; ghi bộ nhớ chữ; báo cáo mục 13 (exe portable) | 127 |
| 129 | task/129-privacy-renderer | builder | opus | Toàn vẹn dữ liệu: che phải không đảo ngược; ranh giới kiểu | `privacy/redact.rs` (`flatten`), `privacy/prepared.rs` (`PreparedOutput`, `Protection`), kiểm thử điểm ảnh/PNG/JPEG/chunk; luật grep trong `scripts/check.ps1` (hàm tạo chỉ trong `privacy_gate.rs` + kiểm thử; không log trong module privacy) | 127 |
| 130 | task/130-privacy-review-window | specialist | opus | Cửa sổ tự vẽ, DPI, kéo vùng, vòng đời GDI, phím | `review_window.rs` (+ `review_model` thuần, kiểm thử được): mục 3.3, 3.4, 18.1 (tiêu đề "Bước k/n"), 18.2 (dải k/n); trả `ReviewOutcome` qua `WM_PRIVACY_REVIEW`; chuỗi vi/en; không nối vào luồng xuất | 127, 129 |
| 131 | task/131-privacy-gate | specialist | fable | Rủi ro cao nhất: tách `capture_flow`, chạy đúng một lần, mọi đường xuất ảnh sống; bảo mật | `privacy_gate.rs`; khóa cấu hình + ô ở tab Chung; `run_on_image`/`pin::open`/`steps_store::add` nhận `PreparedOutput`; nối lớp phủ, phím toàn màn hình, Lặp lại, ảnh ghim (quét lại khi chưa quét), chụp cuộn (kể cả tự Lưu), hàng đợi 3, chờ lớp phủ đóng, thoát; bỏ `set_image` trực tiếp của pin.rs | 128, 129, 130 |
| 132 | task/132-privacy-steps-history | specialist | opus | Toàn vẹn phiên các bước (quyết định 123, 124), xuất thư mục, Lịch sử | Thêm bước qua cổng (không ảnh thô trên đĩa); trường `scanned`; xuất phiên = một lượt cổng (mục 18.1); Lịch sử Chép ảnh/Ghim quét lại; dòng "GIF không được quét" trên thanh ghi | 131 |
| 133 | task/133-privacy-hardening | specialist | opus | Rà soát bảo mật và đo đạc cần đọc toàn bộ đường xuất | Kiểm thử bổ sung (biên ảnh, nguồn trong suốt, khung cắt dở, ảnh lớn, nhiều DPI, mã hóa lại, metadata); bộ mẫu tổng hợp lớn hơn + đo precision/recall/tỉ lệ nhận nhầm (số thật); đo hiệu năng/bộ nhớ/khởi động/exe; hồi quy và demo `scripts/demo-privacy.ps1` (cách ly như `demo-ocr.ps1`); audit grep `set_image`/`encode_*`/`write`/`upload`/`Debug`/`eprintln!` | 132 |
| 134 | task/134-privacy-docs | scaffolder | sonnet | Văn bản phải đúng mã, không hứa quá; không đụng mã | README, CHANGELOG ("Chưa phát hành": Mới/Đã biết), Trợ giúp (`help.rs` chuỗi i18n nếu có), checklist thử tay "Đợt 21", website EN (`index.html`) + VI (`vi.json`), `npm ci && npm run build`; cam kết mục 12; memory bank, CLAUDE.md (dòng đợt 21) | 133 |

Rà soát:
- 127, 128, 129, 130, 132 → `reviewer` model fable (khác agent làm, mạnh hơn hoặc bằng).
- 131 → `reviewer` fable **và** rà soát bảo mật thứ hai `specialist` opus đọc riêng (không đưa kết luận của rà soát thứ nhất).
- 133 → `reviewer` fable. 134 → phiên điều phối.
- Sau Task 133: rà soát toàn đợt `specialist` fable (mục Review Focus), trước khi viết tài liệu.

Mỗi task: `powershell -File scripts/check.ps1` in `ALL CHECKS PASSED` trước khi gộp vào nhánh đợt; ghi token và model thật vào `progress.md`, cập nhật `activeContext.md`.

## Review Focus

1. Không đường nào xuất ảnh chưa qua cổng khi Shield bật: liệt kê mọi lời gọi `set_image`, `encode_png/jpeg/for_upload`, `write_atomic`, `save_png`, `upload`, `pin::open`, `steps_store::add`, `record_outcome` và chứng minh nguồn là `PreparedOutput`.
2. Chạy đúng một lần: token trùng/muộn, Hủy, đóng cửa sổ, thoát giữa chừng, hàng đợi đầy, lớp phủ mở lại khi đang duyệt.
3. OCR lỗi/quá hạn không bao giờ thành "sạch"; tiếp tục không quét cần hai lần bấm.
4. Chữ OCR không rời luồng nền; `Debug`/lỗi/log không có chữ; bộ đệm chữ được ghi đè trước khi thả.
5. Vùng che: alpha 255, nới, kẹp, gộp; ảnh gốc không đổi; PNG/JPEG không còn chữ ở vùng che; không chunk chữ/EXIF.
6. Chụp từng bước: không `.png`/`.tmp` thô trong `steps\current` khi bật; Hủy không đổi `session.json`; khôi phục phiên và `EXIT_BUDGET` không hồi quy.
7. Shield tắt: hồi quy bằng không (Lưu/Chép/Tải/Ghim/Lịch sử/Bước/Chụp cuộn/GIF).
8. Kích thước exe, không khởi tạo gì lúc khởi động khi Shield tắt, không chặn luồng giao diện với ảnh chụp cuộn lớn.
