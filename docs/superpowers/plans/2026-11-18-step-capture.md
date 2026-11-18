# Kế hoạch đợt 12 (Giai đoạn 3 sản phẩm): Chụp từng bước

Thiết kế: `docs/superpowers/specs/2026-11-18-step-capture-design.md`. Nhánh phát triển `feature/step-capture` (tách từ `main` tại `2908658`), worktree `D:\2026 - Tuan\shotme-b12`; nhánh task trong `D:\2026 - Tuan\shotme-task`. Xong đợt: đẩy nhánh, gộp `main`, cập nhật README, tài liệu AI, website (quyết định 114). Không tag, không phát hành.

## Global Constraints

Như mục cùng tên của `docs/superpowers/plans/2026-11-18-pin-screenshot.md`, cộng:
- Không crate mới; feature `windows` mới chỉ khi cần (ví dụ `SetWindowDisplayAffinity` đã ở `Win32_UI_WindowsAndMessaging`).
- Không mã hóa PNG hay sao chép tệp lớn trên luồng giao diện; luồng nền báo về bằng `PostMessage`.
- Dữ liệu phiên không bao giờ mất do lỗi của app: ghi nguyên tử, khôi phục khi khởi động, tệp hỏng được giữ lại.
- QA không giả lập chuột/phím thật (quyết định 111, 114).

## Bảng task và phân bổ model

| Task | Nhánh | Agent | Model | Lý do chọn model | Nội dung | Phụ thuộc |
|---|---|---|---|---|---|---|
| 84 | task/84-steps-core | builder | sonnet | Mô hình thuần + i18n | `steps.rs`, `Action::AddStep`, chuỗi, Markdown | – |
| 85 | task/85-steps-session | specialist | opus | Lưu trữ nền + khôi phục (toàn vẹn dữ liệu), lớp phủ động, thanh phiên topmost loại khỏi ảnh chụp, khay, thoát | `steps_store.rs`, `steps_bar.rs`, lớp phủ, `main` | 84 |
| 86 | task/86-steps-window | specialist | opus | Cửa sổ Win32 lớn (danh sách ảnh nhỏ, EDIT ghi chú, thẻ xác nhận, hộp chọn thư mục COM, xuất tệp nền) | `steps_window.rs`, xuất | 85 |
| 87 | task/87-steps-docs | scaffolder | haiku | Tài liệu, website | README, CHANGELOG, checklist, website, `scripts/demo-steps.ps1` | 86 |

Rà soát: 84 → `reviewer` sonnet; 85, 86 → `reviewer` fable; 87 → phiên điều phối. Sau Task 86: rà soát toàn đợt `specialist` fable.

## Review Focus

1. Không hồi quy khi không có phiên (lớp phủ 9 ô, khay, Lặp lại, OCR, Ghim, Lịch sử).
2. Toàn vẹn dữ liệu phiên: thứ tự ghi PNG/`session.json`, khôi phục, thoát giữa chừng, tệp hỏng.
3. Thanh phiên không có trong ảnh chụp; không giành tiêu điểm khi chụp.
4. Vòng đời cửa sổ, GDI, DPI, i18n, không cắt chữ.
5. Xuất: tên tệp/thư mục an toàn, không ghi đè thư mục có sẵn, Markdown đúng.
