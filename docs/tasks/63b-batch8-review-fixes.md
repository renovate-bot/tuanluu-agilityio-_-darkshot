# Task 63b: Sửa sau rà soát toàn đợt 8

**Agent:** `specialist` · **Model:** opus (đồng thời lúc thoát + lớp phủ + Win32) · **Nhánh:** `task/63b-batch8-review-fixes` (tách từ `feature/window-snap`) · **Rà soát:** `specialist` (opus)

Nguồn: rà soát toàn đợt 8 (model fable), kết luận DUYỆT kèm điểm nên sửa. Global Constraints: mục cùng tên của `docs/superpowers/plans/2026-11-18-window-snap.md`.

## Files
`darkshot-windows/src/{main.rs, history_service.rs, overlay.rs, snap.rs}`; `darkshot-core/src/snap.rs` nếu cần hàm thuần.

## Yêu cầu
1. **Thứ tự mục lịch sử khi thoát** (`main.rs` `save_pending_uploads`): mục dự phòng đang được ghi đồng bộ TRƯỚC khi `flush` hàng đợi, nên mục Lưu/Chép còn trong hàng hiện "mới hơn" mục dự phòng. Sửa: `flush` hàng đợi trước, rồi mới ghi dự phòng (đồng bộ), rồi `flush` lần nữa nếu cần; hoặc đẩy mục dự phòng vào hàng đợi rồi `flush` — chọn cách đúng thứ tự và vẫn trong giới hạn thời gian (tổng chờ hàng đợi ≤ 3 s). Kiểm thử: dùng `Writer` trên thư mục tạm + hàm thuần/nội bộ của trình tự thoát (tách nếu cần) → mục đẩy trước có thời gian/vị trí cũ hơn mục dự phòng.
2. **Khẳng định yếu** (`history_service.rs`: `writer_panic_does_not_wedge_flush`, `unwritable_root_counts_failure_without_panic`): không dựa vào bộ đếm `FAILURES` toàn tiến trình (kiểm thử song song làm nó tăng). Cho `Writer` một bộ đếm lỗi riêng của thể hiện (bộ đếm toàn cục vẫn tăng như cũ) và khẳng định trên bộ đếm đó; với đường đồng bộ, khẳng định qua giá trị trả về hoặc bộ đếm truyền vào.
3. **Chớp tối khi mở lớp phủ** (`overlay.rs` `pick_region`): khung đầu được vẽ khi `snap_hover` còn `None`. Sửa: tính `snap_hover` từ vị trí chuột (đổi sang tọa độ ảnh) trong/ngay sau `State::new`, TRƯỚC `ShowWindow`/`UpdateWindow`. Kiểm thử trạng thái: `State::new` với chuột trong một ứng viên → `snap_hover` đã có giá trị trước khi có `on_move`.
4. **Cửa sổ phủ vô hình** (`snap.rs`): loại cửa sổ `WS_EX_LAYERED` có độ mờ 0 (`GetLayeredWindowAttributes` với `LWA_ALPHA` và alpha 0); lỗi đọc thì giữ cửa sổ. Thêm tham số vào `accept` (ví dụ `invisible: bool`) và cập nhật bảng kiểm thử; kiểm thử thật: popup `WS_EX_LAYERED` alpha 0 không có trong danh sách, alpha 255 thì có.

## Kiểm thử
Viết trước, commit `test:` rồi `fix:`. `cargo fmt --all`; `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.
