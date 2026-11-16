# Bối cảnh sản phẩm

Nguồn đầy đủ: spec mục 5. Hình mẫu giao diện: `prototype/index.html` (mở bằng trình duyệt).

## Người dùng

Một người: chủ dự án, trên Windows 11, đang quen dùng Lightshot.

## Trải nghiệm mong muốn

- Ứng dụng nằm ở khay hệ thống, không có cửa sổ chính. Menu chuột phải: Chụp màn hình, Cài đặt, Thoát.
- Bấm `PrintScreen` (đổi được): màn hình đứng yên và tối đi, kéo chuột chọn vùng, có nhãn kích thước. Chỉnh khung bằng 8 tay nắm, kéo bên trong để di chuyển, kéo bên ngoài để chọn lại.
- Thanh công cụ bốn nút chỉ có biểu tượng; rê chuột hiện tên và phím tắt:
  - Tải lên (`Ctrl+D`): tải lên R2, chép link vào clipboard, thông báo "Đã chép link".
  - Lưu (`Ctrl+S`): lưu PNG vào `Pictures\shotme`, tên `shotme_YYYY-MM-DD_HH-MM-SS.png`, không hỏi lại.
  - Chép (`Ctrl+C`): chép ảnh vào clipboard.
  - Hủy (`Esc`).
- Tải lên thất bại hoặc R2 chưa cấu hình: ảnh được lưu local, thông báo nêu lý do và nơi lưu. Chưa cấu hình thì mở Cài đặt.
- Cài đặt: thông tin R2 và nút Kiểm tra kết nối, hạn của link, phím tắt, thư mục lưu, khởi động cùng Windows. Ô khóa bí mật không hiện lại giá trị đã lưu.
- Phím tắt bị ứng dụng khác chiếm: ứng dụng vẫn chạy, báo rõ, vẫn chụp được qua menu.
- Giao diện tiếng Việt.

## Link chia sẻ

- Dạng `{địa chỉ công khai}/{d1|d7|d30|keep}/{16 ký tự ngẫu nhiên}.png`.
- Ai có link đều xem được. Riêng tư dựa vào tên không đoán được và hạn của link.
- Cloudflare xóa ảnh theo quy tắc vòng đời của từng thư mục; chủ dự án tự tạo ba quy tắc một lần.

## Việc của chủ dự án

Tạo kho R2, bật truy cập công khai, tạo khóa API chỉ đọc và ghi trong kho đó, tạo ba quy tắc xóa, nhập thông tin vào Cài đặt, tự chạy nghiệm thu với kho thật. Agent không được nhận khóa.
