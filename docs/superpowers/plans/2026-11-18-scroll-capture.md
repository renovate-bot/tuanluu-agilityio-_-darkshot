# Kế hoạch đợt 13 (Giai đoạn 3 sản phẩm): Chụp cuộn

Thiết kế: `docs/superpowers/specs/2026-11-18-scroll-capture-design.md`. Nhánh phát triển `feature/scroll-capture` (tách từ `main` tại `6b63f2a`), worktree `D:\2026 - Tuan\shotme-b13`; nhánh task trong `D:\2026 - Tuan\shotme-task`. Xong đợt: đẩy nhánh, gộp `main`, cập nhật README, tài liệu AI, website (quyết định 114). Không tag, không phát hành.

## Global Constraints

Như mục cùng tên của `docs/superpowers/plans/2026-11-18-pin-screenshot.md`, cộng:
- Không crate mới. Không bao giờ gửi đầu vào (chuột/phím/thông điệp cuộn) tới ứng dụng khác.
- Ghép phải xác định, kiểm thử được bằng ảnh tổng hợp; mơ hồ thì báo, không đoán.
- Cửa sổ của darkshot trong lúc chụp cuộn không có trong ảnh (`WDA_EXCLUDEFROMCAPTURE`, kiểm bản dựng như thanh phiên đợt 12) và không lấy tiêu điểm.
- Bộ nhớ có giới hạn (30 000 px, 256 MiB). Không panic trong thủ tục cửa sổ.
- WM_APP đã dùng +1..+16 (quyết định 125): chọn số mới không trùng.
- QA không giả lập chuột/phím thật (quyết định 111, 114).

## Bảng task và phân bổ model

| Task | Nhánh | Agent | Model | Lý do chọn model | Nội dung | Phụ thuộc |
|---|---|---|---|---|---|---|
| 89 | task/89-scroll-core | specialist | opus | Thuật toán ghép là phần rủi ro cao nhất (đúng/sai âm thầm); cần suy luận kỹ về mơ hồ, dải tĩnh, giới hạn | `scroll_stitch.rs`, `Action::Scroll`, chuỗi | – |
| 90 | task/90-scroll-session | specialist | opus | Cửa sổ topmost loại khỏi ảnh chụp, hẹn giờ chụp, phím toàn cục tạm, cổng chụp, lớp phủ | `capture_rect`, `scroll_session.rs`, lớp phủ Ctrl+L, `main` | 89 |
| 91 | task/91-scroll-result | specialist | opus | Cửa sổ xem ảnh rất cao (thu nhỏ, cuộn), sở hữu ảnh lớn, nối `run_on_image`/Ghim | `scroll_result.rs` | 90 |
| 92 | task/92-scroll-docs | scaffolder | haiku | Tài liệu, website | README, CHANGELOG, checklist, website, demo | 91 |

Rà soát: 89, 90, 91 → `reviewer` fable; 92 → phiên điều phối. Sau Task 91: rà soát toàn đợt `specialist` fable.

## Review Focus

1. Ghép đúng hoặc báo không chắc chắn; không bao giờ lặp/mất hàng mà không báo.
2. Không hồi quy lớp phủ (9/10/11 ô theo phiên các bước), các luồng chụp khác, cổng chụp, thoát.
3. Cửa sổ chụp cuộn không vào ảnh, không lấy tiêu điểm, dọn sạch trên mọi đường (Xong, Hủy, Esc, giới hạn, màn hình đổi, thoát).
4. Bộ nhớ và hiệu năng hẹn giờ; GDI; DPI; i18n.
