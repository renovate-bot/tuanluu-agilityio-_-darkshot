# Kế hoạch đợt 10 (Giai đoạn 3 sản phẩm): Lặp lại lần chụp trước

Thiết kế: `docs/superpowers/specs/2026-11-18-repeat-capture-design.md`. Nhánh phát triển `feature/repeat-capture` (tách từ `main` tại `985df73`), worktree `D:\2026 - Tuan\shotme-b10`; nhánh task trong worktree `D:\2026 - Tuan\shotme-task`. Mỗi task có nhánh `task/NN-...` tách từ nhánh phát triển và gộp lại vào đó (`--no-ff`). Xong đợt: đẩy nhánh, gộp `main`, cập nhật README và tài liệu AI, website (chủ dự án yêu cầu, quyết định 113). Không tag, không phát hành.

## Global Constraints

Như mục cùng tên của `docs/superpowers/plans/2026-11-18-pin-screenshot.md`, cộng:
- Tệp `last_capture.json` chỉ chứa tọa độ và hành động; ghi nguyên tử; lỗi đọc/ghi bị nuốt, không bao giờ làm hỏng luồng chụp; không ghi cấu hình chính khi lưu lần chụp trước.
- QA không giả lập chuột/phím thật (chủ dự án đang dùng máy): kiểm thử chạy tay chỉ gửi thông điệp tới cửa sổ của chính nó.

## Bảng task và phân bổ model

| Task | Nhánh | Agent | Model | Lý do chọn model | Nội dung | Phụ thuộc |
|---|---|---|---|---|---|---|
| 73 | task/73-repeat-core | builder | sonnet | Mô hình thuần + cấu hình + i18n, kiểm thử đơn vị | `last_capture.rs`, `HotkeyRole::RepeatLast`, `hotkey_repeat_last`, chuỗi, Trợ giúp | – |
| 74 | task/74-repeat-flow | specialist | opus | Luồng chụp, tọa độ màn hình/ảnh, DPI và tọa độ âm, đăng ký phím, menu khay, tệp trạng thái | Đọc/ghi tệp, `CaptureKind::RepeatLast`, khay, phím | 73 |
| 75 | task/75-repeat-options | builder | sonnet | Mẫu "Dòng phím tắt mới" đã có; bố cục có kiểm thử | Dòng thứ tư ở tab Phím tắt | 74 |
| 76 | task/76-repeat-docs | scaffolder | haiku | Tài liệu và chữ website cơ học | README, CHANGELOG, checklist, website, `scripts/demo-repeat.ps1` | 75 |

Rà soát (agent khác người làm): 73 → `reviewer` sonnet; 74 → `reviewer` fable (rủi ro cao: luồng chụp); 75 → `reviewer` opus (bố cục Tùy chọn 4.400 dòng); 76 → phiên điều phối đọc diff. Sau Task 75: rà soát toàn đợt `specialist` fable.

## Review Focus

1. Không hồi quy luồng chụp vùng, toàn màn hình, Ghim, Lịch sử, `keep_selection`, cổng chụp và hoãn khi menu mở.
2. Tọa độ: tệp lưu tọa độ MÀN HÌNH; đổi sang tọa độ ảnh trừ `origin` đúng một lần; màn hình tọa độ âm.
3. Tệp trạng thái: hỏng/lạ/không quyền ghi → không panic, không báo lỗi, không xóa; ghi nguyên tử.
4. Phím tắt: dòng thứ tư mặc định tắt, xung đột được báo, đăng ký/hủy đúng khi Tùy chọn mở/đóng.
5. Chữ i18n hai ngôn ngữ, không cắt chữ.
