# Task 108: ảnh chụp thật cho website (Ghim, Nhận dạng chữ, Các bước, Ảnh chụp cuộn)

**Agent:** `specialist` · **Model:** opus (mở cửa sổ thật trong kiểm thử; không được đụng hồ sơ thật, clipboard hay đầu vào của chủ dự án) · **Nhánh:** `task/108-website-shots` (tách từ `main`) · **Rà soát:** `reviewer` (sonnet, chỉ mã kiểm thử) · **Phụ thuộc:** không

Chủ dự án yêu cầu (2026-11-21): "cập nhật hình ảnh cho các tính năng: Ghim, Nhận dạng chữ, Từng bước và cuộn cho Website". Website đã có ảnh chụp thật cho Lịch sử và Tùy chọn (`website/public/img/*-en.webp`, `*-vi.webp`); các mục G (Ghim), H (Nhận dạng chữ), I (Chụp từng bước và chụp cuộn) chưa có hình. Task này chỉ tạo **công cụ chụp** (kiểm thử `#[ignore]`) và chạy nó để ra PNG; đưa ảnh vào website do phiên điều phối làm.

## Việc cần làm

Thêm kiểm thử chạy tay (`#[test] #[ignore = "ảnh cho website; chạy tay"]`, tên bắt đầu `webshot_`) trong `crates/darkshot-windows` mở cửa sổ thật với dữ liệu mẫu bịa, chụp bằng `PrintWindow(PW_RENDERFULLCONTENT)` (đã có mẫu `print_window` trong `ocr_window.rs` mục tests; được chuyển nó sang một mô-đun tiện ích kiểm thử chung nếu cần) và ghi PNG vào thư mục ở biến môi trường `DARKSHOT_WEBSHOTS` (thiếu biến thì kiểm thử bỏ qua, không ghi gì). Mỗi ảnh hai ngôn ngữ (`lang::set(Lang::En)` / `Lang::Vi`, đặt lại `Vi` khi xong), tên tệp `<tên>-en.png`, `<tên>-vi.png`:

1. `ocr` — cửa sổ "Chữ nhận dạng" cỡ mặc định, ảnh mẫu chữ Anh tổng hợp (như `ocr::tests::lines_image`, nội dung bịa kiểu "Quarterly report summary"…, 5–7 dòng, có hai đoạn), đã có kết quả nhận dạng thật, **hai dòng đang được chọn** (đặt trạng thái chọn trực tiếp hoặc qua đường không ghi clipboard; KHÔNG được ghi clipboard), công tắc mặc định.
2. `steps` — cửa sổ "Các bước" với 3 bước mẫu (ảnh bịa vẽ bằng mã: ví dụ một hộp thoại cài đặt giả với nút, chữ "Example settings"), mỗi bước có ghi chú ngắn bịa, bước 2 đang chọn để khung xem trước có ảnh. Thêm một ảnh `stepsbar` của thanh phiên ("Chụp từng bước · 3 bước").
3. `scroll` — cửa sổ "Ảnh chụp cuộn" với một ảnh dọc bịa cao khoảng 2400 px (trang bài viết giả: tiêu đề, các đoạn chữ, vài khối màu), `uncertain = 1` để thấy dải vàng cảnh báo.
4. `pin` — một ảnh ghim (ảnh mẫu bịa, ví dụ biểu đồ cột nhỏ có tiêu đề "Weekly sign-ups" và chú thích mũi tên/khung đỏ kiểu đánh dấu) ở mức 100%; chụp chính cửa sổ ghim. Một ảnh là đủ nếu ảnh ghim không có chữ giao diện (đặt tên `pin.png`); nếu có chữ giao diện thì hai ngôn ngữ.

## Ràng buộc (bắt buộc)

- Không bao giờ giả lập chuột/phím thật (`SendInput`, `mouse_event`, `keybd_event`, `SetCursorPos`); chỉ gọi hàm nội bộ hoặc `PostMessage`/`SendMessage` tới HWND của chính tiến trình kiểm thử.
- Không đụng clipboard.
- Không đụng hồ sơ thật: mọi kiểm thử đặt `DARKSHOT_PROFILE_DIR` tới một thư mục tạm riêng (dưới `std::env::temp_dir()`), dọn khi xong. Phiên Chụp từng bước thật của chủ dự án ở `%LOCALAPPDATA%\darkshot\steps\current` tuyệt đối không được đọc, ghi hay xóa.
- Không mạng, không tiến trình khác, không đụng `darkshot.exe` đang chạy.
- Dữ liệu mẫu bịa hoàn toàn: không ảnh màn hình thật, không đường dẫn máy, không tên người thật. Đường dẫn thư mục xuất (nếu cửa sổ hiện) không được lộ trong ảnh.
- Không đổi hành vi sản phẩm. Chỉ thêm/sắp lại mã `#[cfg(test)]`. Nếu buộc phải thêm hàm `pub(crate)` chỉ dùng cho kiểm thử thì đặt sau `#[cfg(test)]`.
- `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.

## Chạy và bàn giao

Chạy: `$env:DARKSHOT_WEBSHOTS = "<repo>\target\webshots"; cargo test -p darkshot-windows webshot_ -- --ignored --test-threads=1`. Mở từng PNG (công cụ Read xem được ảnh) và tự kiểm: đúng ngôn ngữ, không cắt xén, không cửa sổ khác lẫn vào, không đen. Commit trên nhánh (`test: website screenshot capture for pin, OCR, steps, scroll`). Báo lại: danh sách PNG và kích thước, DPI lúc chụp, chỗ nào không chụp được và vì sao, số token.
