# Thiết kế: Chụp từng bước (Giai đoạn 3 sản phẩm, đợt 12 nội bộ)

Ngày: 2026-11-18. Người quyết: phiên điều phối (quyết định 114).

## 1. Mục đích

Viết hướng dẫn, báo lỗi theo từng bước, tài liệu quy trình: người dùng cần chụp nhiều ảnh liên tiếp, đánh dấu, ghi chú ngắn cho mỗi ảnh rồi gửi cả bộ. Hôm nay họ phải Lưu từng ảnh, tự đặt tên, tự ghép tài liệu. Tính năng này gom các lần chụp thành một **phiên các bước**, đánh số tự động, cho ghi chú và sắp xếp, rồi **xuất ra một thư mục** gồm ảnh đánh số và tài liệu Markdown. Dùng lại lớp phủ và Annotation Pro; không làm trình soạn tài liệu.

## 2. Phạm vi

Làm:
1. **Bắt đầu phiên**: menu khay "Chụp từng bước" (khi chưa có phiên). Bắt đầu = tạo phiên trống, hiện **thanh phiên**.
2. **Thanh phiên**: cửa sổ nhỏ, tối, luôn trên cùng, không lên thanh tác vụ, kéo được (kéo ở vùng chữ), mặc định giữa mép trên vùng làm việc của màn hình chính, cách mép 12 DIP. Nội dung: chấm đỏ + "Chụp từng bước · N bước", nút **Chụp bước** (chạy chụp vùng như phím chụp vùng), **Xem lại**, **Kết thúc**. Thanh **không xuất hiện trong ảnh chụp** (`SetWindowDisplayAffinity(WDA_EXCLUDEFROMCAPTURE)`; nếu hàm lỗi — Windows cũ — thì ẩn thanh trong lúc chụp rồi hiện lại).
3. **Thêm bước** trong lớp phủ: khi có phiên, thanh ngang của lớp phủ có thêm mục **Thêm bước** (biểu tượng dấu cộng trong ô vuông) ngay trước Hủy, phím **Ctrl+N**. Ảnh bước = ảnh vùng chọn có hình đánh dấu. Lớp phủ đóng; thanh phiên cập nhật số bước. Không ghi Lịch sử (bước là dữ liệu của phiên), không đổi lần chụp trước. Khi không có phiên, thanh ngang như đợt 11.
4. **Lưu trên đĩa**: `%LOCALAPPDATA%\darkshot\steps\current\` gồm `001.png`, `002.png`… (tên theo thứ tự thêm, không đổi khi sắp xếp) và `session.json` (phiên bản 1: danh sách `{file, caption}` theo thứ tự hiển thị). PNG mã hóa trên luồng nền; `session.json` ghi nguyên tử. Tối đa 99 bước (thêm bước thứ 100 → thông báo Info, không thêm).
5. **Khôi phục**: khởi động mà thư mục `current` có `session.json` hợp lệ → phiên tiếp tục, hiện thanh phiên. `session.json` hỏng → đổi tên thư mục thành `current.corrupt-<thời gian>` (giữ ảnh), không có phiên. Tệp PNG thiếu → bỏ bước đó khỏi danh sách.
6. **Cửa sổ Xem lại** ("Các bước"): tối, đổi cỡ được, mặc định 860×600 DIP, tối thiểu 640×440. Trái: danh sách bước (ảnh nhỏ + "Bước k" + dòng đầu ghi chú). Phải: xem trước ảnh bước đang chọn + ô **Ghi chú** nhiều dòng (tối đa 2000 ký tự, lưu khi rời ô/đổi bước/đóng). Nút: **Lên**, **Xuống**, **Xóa bước** (xóa ngay, không xác nhận; tệp ảnh bị xóa), **Xuất…**, **Kết thúc phiên**, **Đóng**. Danh sách rỗng → trạng thái trống "Chưa có bước nào. Bấm Chụp bước trên thanh phiên."
7. **Xuất…**: hộp chọn thư mục của Windows (như "Chọn…" ở Tùy chọn, mặc định thư mục lưu ảnh) → tạo thư mục con `darkshot-steps-YYYY-MM-DD_HH-mm` (thêm `_2`, `_3`… nếu trùng) gồm `01.png`, `02.png`… theo thứ tự hiển thị và `steps.md`:
   ```
   # Các bước            (English: # Steps)
   _darkshot, 2026-11-18 14:05_

   ## Bước 1             (## Step 1)
   <ghi chú, nếu có>

   ![Bước 1](01.png)
   ```
   Xong → thông báo Success "Đã xuất N bước" và mở thư mục trong Explorer (`explorer.exe /select` như Lịch sử). Lỗi ghi → thông báo Error, phiên giữ nguyên. Xuất không kết thúc phiên.
8. **Kết thúc phiên** (thanh phiên hoặc cửa sổ Xem lại): nếu còn bước chưa xuất kể từ thay đổi cuối → thẻ xác nhận trong cửa sổ Xem lại (mẫu thẻ "Xóa lịch sử?"): "Kết thúc mà không xuất? N bước sẽ bị xóa." [Hủy] [Kết thúc]. Kết thúc = xóa thư mục `current`, đóng thanh phiên và cửa sổ Xem lại. Đã xuất (không đổi gì sau đó) hoặc 0 bước → kết thúc ngay.
9. Menu khay khi có phiên: "Xem lại các bước (N)" thay cho "Chụp từng bước".
10. Thoát ứng dụng khi có phiên: giữ thư mục (khôi phục lần sau), đợi ghi PNG đang dở tối đa 3 giây.

Không làm (quyết định 122): tự vẽ số bước lên ảnh (người dùng dùng công cụ Đánh số), xuất HTML/PDF/Word, ghép một ảnh dài, nhiều phiên song song, phím tắt toàn cục riêng, ghi lại thao tác chuột/bàn phím, chỉnh sửa lại hình đánh dấu của bước đã thêm.

## 3. Kiến trúc

- `darkshot-core/src/steps.rs` (thuần): `StepSession { steps: Vec<Step> }`, `Step { file: String, caption: String }`; `add(file) -> Result<usize, Full>`, `remove(i)`, `move_up(i)`, `move_down(i)`, `set_caption(i, s)` (cắt 2000 ký tự theo ký tự Unicode), `next_file_name()` ("001.png"…, không trùng tên đang có), `to_json`/`from_json` (version 1, kiểm tên tệp chỉ `\d{3}\.png`), `export_names(n) -> Vec<String>` ("01.png"… hoặc "001.png" khi n ≥ 100 — tối đa 99 nên luôn 2 chữ số), `markdown(lang, when: LocalTime, steps) -> String`, `export_dir_name(when) -> String`. `MAX_STEPS = 99`, `MAX_CAPTION = 2000`. `Action::AddStep` (`run_action` no-op như Pin/Ocr; `last_capture::remember` giữ prev).
- `darkshot-windows`:
  - `steps_store.rs`: thư mục phiên, ghi PNG nền (một luồng `darkshot-steps` có hàng đợi, giống mẫu `history_service`), `session.json` nguyên tử, khôi phục, xóa, xuất (sao chép tệp + ghi `steps.md`).
  - `steps_bar.rs`: thanh phiên.
  - `steps_window.rs`: cửa sổ Xem lại.
  - `overlay.rs`: ô thanh ngang động theo cờ phiên, Ctrl+N; `main.rs`: nối, khay, thoát.
- Chữ mới qua `i18n` (Việt/English): tên mục menu, thanh phiên, cửa sổ, nút, trạng thái trống, thẻ xác nhận, thông báo, tiêu đề Markdown.

## 4. Riêng tư

Ảnh bước chỉ trên máy (LOCALAPPDATA) cho tới khi người dùng xuất ra thư mục họ chọn. Không mạng, không ghi Lịch sử. Chế độ cách ly `DARKSHOT_PROFILE_DIR` dời thư mục theo.

## 5. Tiêu chí nghiệm thu

1. Bắt đầu phiên, chụp 3 bước có đánh dấu, thanh phiên ghi "3 bước", thanh không có trong ảnh.
2. Xem lại: đổi thứ tự, ghi chú, xóa một bước; xuất ra thư mục: ảnh đánh số đúng thứ tự, `steps.md` đúng ghi chú và liên kết ảnh.
3. Thoát và mở lại darkshot: phiên còn, đủ bước và ghi chú.
4. Kết thúc khi chưa xuất → thẻ xác nhận; kết thúc → thư mục phiên bị xóa.
5. Không có phiên: lớp phủ, khay như đợt 11. Hai ngôn ngữ, DPI, không cắt chữ; `scripts/check.ps1` → `ALL CHECKS PASSED`.
