# Task 70: Ghim ảnh — nút Ghim trong Lịch sử

**Agent:** `builder` · **Model:** sonnet (thêm một nút theo mẫu sẵn có; bố cục có kiểm thử thuần) · **Nhánh:** `task/70-pin-history` (tách từ `feature/pin-screenshot`) · **Rà soát:** `reviewer` (opus) · **Phụ thuộc:** 69

Thiết kế: `docs/superpowers/specs/2026-11-18-pin-screenshot-design.md` mục 3 (câu 2), 4.1 ("Từ Lịch sử"). Global Constraints: mục cùng tên của `docs/superpowers/plans/2026-11-18-pin-screenshot.md`.

## Cần đọc
`crates/darkshot-windows/src/history_layout.rs`: `ActionSet`, `as_array`, `actions_for`, bố cục nút của khung chi tiết (mảng `buttons`, logic xuống dòng quanh dòng 280–330), kiểm thử của chúng; `history_window.rs`: các `ID_*` của nút hành động, chỗ tạo nút, `on_action`, chỗ Sao chép ảnh đọc ảnh (`decode_png_file`? luồng nào?), chỗ cập nhật bật/tắt nút, `custom_draw_button`, phím tắt của cửa sổ Lịch sử (nếu có); `crates/darkshot-windows/src/pin.rs` (API `open`, `PinPlace::CenterOnCursorMonitor`, `PinError`); `crates/darkshot-core/src/i18n.rs` (`HistoryPin`, `PinLimitTitle`, `pin_limit_body`, `PinFailedTitle`, `pin_failed_body`).

## Files
`crates/darkshot-windows/src/history_layout.rs`, `history_window.rs` (và `history_paint.rs` chỉ nếu nút vẽ ở đó).

## Yêu cầu
1. `ActionSet.pin: Avail`; thứ tự nút: Mở, Sao chép ảnh, Sao chép liên kết, Mở thư mục, **Ghim**, Xóa khỏi lịch sử (`as_array` 6 phần tử). `actions_for`: `pin` = giống `copy_image` (Enabled khi tệp ảnh còn, Disabled khi không) cho mọi `Kind`.
2. Nút Ghim (`ID_PIN`, số chưa dùng), chữ `HistoryPin`, kiểu nút phụ như Sao chép ảnh. Bố cục: hàng nút xuống dòng như hiện có, không cắt chữ ở cả hai ngôn ngữ với cỡ cửa sổ tối thiểu 760×520 và DPI 100/150/200% (kiểm thử thuần trên hàm bố cục).
3. Bấm Ghim: đọc ảnh của mục theo đúng cách Sao chép ảnh đang làm (cùng luồng; nếu Sao chép ảnh đọc trên luồng nền thì Ghim cũng vậy và mở ảnh ghim khi nhận kết quả trên luồng giao diện) → `pin::open(image, PinPlace::CenterOnCursorMonitor)`. Lỗi đọc → hiện lỗi giống Sao chép ảnh khi tệp hỏng; `Limit` → thông báo Info `PinLimitTitle` + `pin_limit_body`; `Create` → Error `PinFailedTitle`. Cửa sổ Lịch sử giữ nguyên (không đóng).
4. Thứ tự Tab giữa các nút theo thứ tự mới; Enter/Space trên nút Ghim chạy như bấm.

## Kiểm thử (viết trước)
- `actions_for` cho `Saved`, `UploadFallback`, `Uploaded`, `Copied` có/không tệp → `pin` đúng; `as_array` thứ tự mới.
- Bố cục: với chữ dài nhất (Việt) và các DPI trên, không nút nào chồng nhau, mọi nút trong khung chi tiết, nút đủ rộng cho chữ (dùng hàm đo giả theo mẫu kiểm thử bố cục sẵn có).
- Kiểm thử cũ của bố cục cập nhật theo nút mới chỉ ở chỗ số nút/thứ tự đổi theo thiết kế.

## Tiêu chí nghiệm thu
Kiểm thử qua; `scripts/check.ps1` in `ALL CHECKS PASSED`; mọi hành động Lịch sử cũ giữ nguyên.

## Review focus
Chỉ số mảng `buttons` sau khi thêm phần tử (mọi chỗ dùng số cứng 4/5); bật/tắt khi đổi mục chọn; luồng đọc ảnh; không cắt chữ.

TDD: commit `test:` rồi `feat:`. Báo cáo theo mẫu `.claude/agents/builder.md`.
