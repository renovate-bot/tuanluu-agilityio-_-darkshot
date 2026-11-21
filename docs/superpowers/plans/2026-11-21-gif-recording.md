# Kế hoạch đợt 20: Ghi màn hình thành GIF

Thiết kế: `docs/superpowers/specs/2026-11-21-gif-recording-design.md`. Nhánh đợt `feature/gif-recording` (tách từ `main` tại `f055b99`), worktree `D:\2026 - Tuan\shotme-gif`. Task gộp vào nhánh đợt trên máy; xong đợt: đẩy nhánh, PR vào `main` (quyết định 138). Không tag, không phát hành.

Đợt 16–19 và task 115–121 thuộc kế hoạch MCP (nhánh `task/115-mcp-design`, chưa gộp), nên đợt này là đợt 20, task 122–124.

## Global Constraints

Như mục cùng tên của `docs/superpowers/plans/2026-11-21-ocr-pro.md`, trừ "không crate mới", cộng:
- Đúng một crate mới: `gif` 0.14.x (thuần Rust, phụ thuộc `color_quant`, `weezl`) trong `darkshot-core`. Không FFmpeg, không chương trình ngoài, không mạng. Không feature `windows` mới (trừ khi thật cần: báo phiên điều phối). Exe release đo trước/sau; tăng quá 150 KB thì dừng, báo.
- Luồng giao diện không bao giờ chụp liên tục hay mã hóa GIF. Luồng nền không gọi hàm giao diện Win32 nào (chỉ `PostMessageW`).
- Không bao giờ giữ mọi khung trong bộ nhớ: mỗi lượt tối đa một khung thô + một khung chờ (thiết kế mục 4).
- Không bao giờ để lại GIF dở ở tên cuối, không ghi đè tệp có sẵn, không để lại tệp `.part` sau Dừng/Hủy/lỗi.
- Không panic trong thủ tục cửa sổ và luồng nền (lỗi → `Outcome::Failed`). `unsafe` nhỏ, có chú thích SAFETY.
- WM_APP đã dùng +1..+17: `WM_RECORD_DONE = WM_APP + 18`.
- Không đổi định dạng cấu hình, hành vi PNG/JPEG, tải lên, Lịch sử.

## Bảng task và phân bổ model

| Task | Nhánh | Agent | Model | Lý do chọn model | Nội dung | Phụ thuộc |
|---|---|---|---|---|---|---|
| 122 | task/122-gif-core | specialist | opus | Đồng thời (luồng nền, trạng thái nguyên tử) và toàn vẹn tệp (tạm → tên cuối không ghi đè): không dùng model nhẹ | `gif_record.rs`, crate `gif`, `Action::RecordGif`, tên tệp, chuỗi | – |
| 123 | task/123-gif-session | specialist | opus | Cửa sổ topmost loại khỏi ảnh chụp, DPI nhiều màn hình, cổng chụp, thoát | `record_session.rs`, lớp phủ, `main` | 122 |
| 124 | task/124-gif-docs | scaffolder | sonnet | Tài liệu và website phải khớp mã (không hứa quá) | README, CHANGELOG, checklist, website | 123 |

Rà soát: 122, 123 → `reviewer` model fable (khác agent làm, mạnh bằng hoặc hơn); 124 → phiên điều phối. Sau Task 123: rà soát toàn đợt `specialist` fable (luồng, bộ nhớ, GIF hợp lệ, dọn tài nguyên, tọa độ nhiều màn hình, hồi quy).

## Review Focus

1. Không đua giữa Dừng/Hủy/tự dừng và chốt tệp; Dừng/Hủy gọi nhiều lần an toàn; một phiên tối đa.
2. GIF hợp lệ (giải mã lại được), lặp vô hạn, độ trễ đúng; không GIF dở, không ghi đè, không `.part` sót.
3. Bộ nhớ có giới hạn; tính kích thước có kiểm tra tràn.
4. Thanh/khung không vào ảnh, không lấy tiêu điểm; đúng vùng trên màn hình phụ tọa độ âm.
5. Không hồi quy lớp phủ (số ô thanh ngang, DPI), cổng chụp, chụp cuộn, chụp từng bước, thoát.
