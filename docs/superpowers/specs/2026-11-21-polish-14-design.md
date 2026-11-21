# Thiết kế: Đợt 14 — gia cố và hoàn thiện (sau Giai đoạn 3)

Ngày: 2026-11-21. Người quyết: phiên điều phối (chủ dự án: "cứ tiếp tục như những gì bạn cho là cần thiết"; hàng đợi tính năng đã hết). Không thêm tính năng; chỉ sửa ba điểm còn mở đã được ghi nhận ở các đợt trước.

## 1. Cửa sổ "Chữ nhận dạng" bị ảnh ghim che (quyết định 120)

Hiện tại: mở Nhận dạng chữ từ menu ảnh ghim, cửa sổ kết quả đặt cạnh ảnh ghim (`place_avoiding`); không đủ chỗ thì đặt giữa màn hình và ảnh ghim (`WS_EX_TOPMOST`) che lên.

Sửa: khi cửa sổ kết quả mở từ ảnh ghim (có khung cần tránh) và `place_avoiding` không tìm được chỗ không chồng, cửa sổ kết quả được đặt `HWND_TOPMOST` (luôn trên cùng) để nằm trên ảnh ghim; khi đã có chỗ không chồng thì giữ như cũ (không topmost). Cửa sổ topmost vẫn đóng/Esc/Chép như thường. Hàm thuần `place_avoiding` trả thêm cờ `overlaps: bool`.

## 2. Ghim ảnh rất cao: phần đầu nằm ngoài màn hình

Hiện tại: `PinPlace::CenterOnCursorMonitor` dùng `fit_zoom` (bậc nhỏ nhất 10%) rồi `centered`; ảnh vẫn cao hơn vùng làm việc ở 10% thì căn giữa, mép trên và mép dưới ra ngoài màn hình.

Sửa: `darkshot_core::pin::centered` (hoặc hàm mới `placed_in(img_w, img_h, zoom, area) -> Rect`): mỗi chiều, nếu cỡ hiển thị ≤ cỡ vùng thì căn giữa như cũ; nếu lớn hơn thì neo mép trên (dọc) / mép trái (ngang) vào mép vùng làm việc, để phần đầu ảnh luôn thấy được (người dùng kéo ảnh để xem phần sau). README "Đã biết" của Chụp cuộn cập nhật theo.

## 3. `Annotations::undo/redo` khi áp dụng thất bại

Hiện tại: `undo`/`redo` lấy mục khỏi một ngăn và đẩy sang ngăn kia kể cả khi `apply` trả `None`. Không xảy ra với sổ LIFO nhất quán, nhưng nếu xảy ra thì Ctrl+Z/Ctrl+Y sau đó lệch.

Sửa: `apply` trả `None` → đặt mục lại vào ngăn cũ, không đẩy sang ngăn kia; trả `None`. Kiểm thử bằng một sổ dựng tay không nhất quán.

## 4. Kiểm tra sức khỏe

Sau khi gộp: chạy một lượt mọi kiểm thử `#[ignore]` KHÔNG cần lớp phủ toàn màn hình hay chuột/phím thật (pin::live, ocr::tests::manual, ocr_window::tests::manual, steps_bar::live, steps_window::live, scroll_session::live, scroll_result::, repeat_last_saves_png_of_region), ghi kết quả vào checklist.

## 5. Tiêu chí

`scripts/check.ps1` → `ALL CHECKS PASSED`; không hồi quy; không chữ mới (nếu cần thì qua i18n); README/CHANGELOG ("Chưa phát hành" → "Sửa lỗi") cập nhật.
