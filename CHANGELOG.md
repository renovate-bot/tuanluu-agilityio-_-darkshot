# Nhật ký thay đổi

Mọi thay đổi đáng chú ý của darkshot, mới nhất ở trên. Phiên bản theo [SemVer](https://semver.org/lang/vi/): tăng số giữa khi có tính năng mới, số cuối khi chỉ sửa lỗi. Tải bản phát hành ở [trang Releases](https://github.com/tuanluu-agilityio/darkshot/releases).

## [0.4.0] - 2026-11-18

### Mới

- **Ghim ảnh lên màn hình.** Từ lớp phủ, bấm nút Ghim hoặc Ctrl+P để ghim vùng vừa chọn (đã đánh dấu) khít đúng chỗ của nó. Từ Lịch sử, nút Ghim mở ảnh giữa màn hình có chuột, không lớn hơn 80% vùng làm việc. Ảnh ghim luôn nằm trên cùng: kéo để dời, cuộn để phóng, Ctrl+cuộn để đổi độ mờ, bấm đúp để về 100%, chuột phải để mở menu, Esc để đóng. Phím: `+` `-` `0`, mũi tên, Ctrl+W, Ctrl+C, Ctrl+S. Tối đa 32 ảnh ghim; menu khay có "Đóng mọi ảnh ghim" khi đang có ảnh ghim. Thoát darkshot đóng mọi ảnh ghim.
  - Phóng: mỗi cạnh hiển thị tối đa 16.384 px; trên 100%, diện tích hiển thị tối đa 40 triệu px (ảnh 4K phóng tối đa 200%).

### Sửa lỗi

- Không có sửa lỗi sản phẩm trong đợt này.

### Đã biết

- Sau khi bấm chọn một cửa sổ ở Gợi ý cửa sổ, viên nhắc có thể còn hiện tới lần di chuột kế tiếp. Đã điều tra (Task 66), không tái hiện được trên một màn hình 200%; cần thử bằng chuột thật trên nhiều màn hình.
- Ảnh ghim chưa được thử tay trên Windows 10, ở các DPI 100% và 150%, và trên màn hình có tọa độ âm. Xem mục "Đợt 9" trong `docs/testing/manual-checklist.md`.

## [0.3.0] - 2026-11-18

### Mới

- **Gợi ý cửa sổ khi chọn vùng.** Khi chưa có vùng chọn, rê chuột lên một cửa sổ thì khung của nó sáng lên kèm nhãn kích thước; rê ra nền thì cả màn hình sáng lên. Bấm một lần (không kéo) hoặc Enter để chọn đúng khung đó, không lẫn viền vô hình. Kéo chuột vẫn chọn vùng tự do như trước. Viên nhắc ở mép trên màn hình nhắc thao tác và không nằm trong ảnh. Bật/tắt ở Tùy chọn → Thông thường → "Gợi ý cửa sổ khi chọn vùng" (mặc định bật). darkshot chỉ đọc khung cửa sổ, không đọc tiêu đề hay nội dung.
- **Lịch sử ảnh chụp** (mặc định tắt; bật ở Tùy chọn → Thông thường). Giữ 50 ảnh gần nhất đã Lưu, Chép hoặc Tải lên, chỉ trên máy (`%LOCALAPPDATA%\darkshot\history\`). Cửa sổ Lịch sử mở từ menu khay: ảnh nhỏ, xem trước, tìm theo tên/ngày/liên kết (không phân biệt dấu), lọc theo loại, Mở, Sao chép ảnh, Sao chép liên kết, Mở thư mục chứa ảnh, Xóa khỏi lịch sử, Xóa lịch sử (có xác nhận). Xóa trong lịch sử không bao giờ xóa ảnh bạn đã lưu hay ảnh trên R2.
- **Annotation Pro** cho công cụ đánh dấu:
  - Hoàn tác và làm lại nhiều bước (Ctrl+Z, Ctrl+Y hoặc Ctrl+Shift+Z), có nút trên thanh ngang.
  - Công cụ **Chọn** (phím V): chọn một hình để dời, đổi cỡ, đổi màu hoặc xóa (Delete).
  - **Hình elip** (phím E) và **Đánh số** (phím N, vòng số tự tăng, tự đánh lại khi xóa).
  - Bốn mức cỡ nét và cỡ chữ (bảng Cỡ, phím `[` và `]`).
- **Thanh công cụ gọn:** thanh dọc còn 9 mục; nút **Hình** gom Đường thẳng, Mũi tên, Hình chữ nhật, Hình elip và nhớ hình dùng gần nhất.
- **Báo có bản mới:** ứng dụng hỏi GitHub một lần sau khi khởi động và mỗi 24 giờ; có bản mới thì menu khay hiện mục "Có bản mới X.Y.Z". Chỉ gửi tên và phiên bản ứng dụng. Tắt được ở Tùy chọn.

### Sửa lỗi và độ tin cậy

- Lịch sử không còn mất mục khi thoát ngay sau Lưu, Chép hoặc Tải lên: các mục được ghi tuần tự trên một luồng nền và khi thoát (kể cả tắt máy) darkshot chờ tối đa 3 giây cho việc ghi xong.
- Một lượt tải lên không bao giờ tạo hai mục lịch sử, kể cả khi thoát đúng lúc lượt đó vừa xong.
- Chỉ mục lịch sử và ảnh được ghi chắc xuống đĩa trước khi đổi tên, để mất điện không làm hỏng chỉ mục.
- Khi thẻ "Xóa toàn bộ lịch sử?" đang mở, thanh cuộn danh sách mờ đi và không cuộn được.

### Đã biết

- Sau khi bấm chọn một cửa sổ, viên nhắc có thể còn hiện tới lần di chuột kế tiếp.
- Gợi ý cửa sổ và Lịch sử chưa được thử tay ở DPI 100%/150%, màn hình có tọa độ âm và Windows 10.
- Hạn hiển thị của ảnh đã tải lên là ngày "còn ít nhất tới": R2 có thể xóa trễ tới khoảng một ngày.

## [0.2.0] - 2026-11-17

### Mới

- **Công cụ Làm mờ** (phím B): kéo một hình chữ nhật lên thông tin nhạy cảm; vùng che được tô kín bằng một màu trung bình duy nhất, cạnh sắc, không khôi phục được chữ bên dưới. Giữ Shift để thành hình vuông.

### Sửa lỗi

- Khi vùng chọn hẹp và sát góc trên trái màn hình, thanh công cụ dọc không còn chồng lên thanh bốn nút.

Bản 0.1.1 từng được phát hành rồi rút lại cùng ngày (0 lượt tải) và đổi thành 0.2.0 vì có tính năng mới.

## [0.1.0] - 2026-11-17

Bản phát hành đầu tiên.

### Mới

- Chụp vùng màn hình bằng `PrintScreen` (đổi được), nhiều màn hình và DPI khác nhau; sau khi chọn: **Tải lên** Cloudflare R2 của bạn và chép link có hạn, **Lưu** PNG, **Chép** ảnh vào clipboard, hoặc **Hủy** (Ctrl+D, Ctrl+S, Ctrl+C, Esc).
- Phím tắt toàn màn hình: lưu và tải lên nhanh không cần chọn vùng.
- Đánh dấu ảnh trong lớp phủ: bút vẽ, đường thẳng, mũi tên, hình chữ nhật, bút dạ quang, chữ (gõ được tiếng Việt), hoàn tác; bảng màu 48 màu, 16 màu tự chọn được lưu, hộp chọn màu đủ (HSL, RGB).
- Tải lên R2 dạng PNG hoặc JPEG (chọn chất lượng); link hết hạn sau 1, 7, 30 ngày hoặc không hết hạn; khóa R2 mã hóa bằng DPAPI.
- Giữ vị trí vùng chọn, chụp kèm con trỏ chuột, bật/tắt thông báo, khởi động cùng Windows.
- Giao diện luôn tối; hai ngôn ngữ Tiếng Việt và English.
- Người dùng bản cũ tên shotme: lần chạy đầu tự chép cấu hình (kể cả khóa R2) và chuyển mục khởi động cùng Windows.

[Chưa phát hành]: https://github.com/tuanluu-agilityio/darkshot/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/tuanluu-agilityio/darkshot/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/tuanluu-agilityio/darkshot/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/tuanluu-agilityio/darkshot/releases/tag/v0.1.0
