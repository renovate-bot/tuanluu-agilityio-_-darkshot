# Bối cảnh sản phẩm

Nguồn đầy đủ: spec mục 5. Hình mẫu giao diện: `prototype/index.html` (mở bằng trình duyệt).

## Người dùng

Một người: chủ dự án, trên Windows 11, đang quen dùng Lightshot.

## Trải nghiệm mong muốn

- Ứng dụng nằm ở khay hệ thống, không có cửa sổ chính. Menu khay có năm mục: Chụp màn hình, Tùy chọn..., Thông tin..., Trợ giúp, Thoát (không đường kẻ).
- Cửa sổ Thông tin và Trợ giúp không chặn ứng dụng: lúc mở vẫn chụp, dùng menu và mở Tùy chọn được. Mở lại khi đang mở thì chỉ đưa ra trước. Thông tin gồm biểu tượng, tên, phiên bản và một dòng mô tả, không có link. Trợ giúp là văn bản thuần, nêu phím tắt đang đặt và các bước chuẩn bị R2, không có địa chỉ web.
- Tùy chọn có ba tab, nút OK, Hủy và Trợ giúp:
  - Thông thường: Khởi động cùng Windows, ba ô tích (Hiện các thông báo lưu và sao chép; Giữ vị trí vùng đã chọn; Bao gồm con trỏ chuột vào ảnh chụp màn hình), Thư mục lưu (nút Chọn…), Ngôn ngữ.
  - Phím tắt: ba dòng, mỗi dòng có ô tích bật/tắt, ô phím chỉ đọc và nút Đổi…: Chụp vùng màn hình (bật, PrintScreen), Lưu ảnh (toàn màn hình) (tắt, Shift+PrintScreen), Tải lên nhanh (toàn màn hình) (tắt, Ctrl+PrintScreen).
  - Lưu trữ R2: Account ID, Access Key ID, Secret Access Key, Tên kho, Địa chỉ công khai, Hạn của link, nút Kiểm tra kết nối.
- Hai ngôn ngữ: Tiếng Việt và English. Đổi ở Tùy chọn › Thông thường, có hiệu lực ngay khi bấm OK. Mặc định: chưa có file cấu hình hoặc file hỏng thì theo ngôn ngữ giao diện Windows (tiếng Việt nếu là tiếng Việt, còn lại English); file cũ không có mục ngôn ngữ thì là Tiếng Việt.
- Đợt 2 đã có trong mã: phím lưu và tải lên toàn màn hình (màn hình đang có con trỏ chuột, không mở lớp phủ), chụp kèm con trỏ chuột, giữ vị trí vùng chọn (trong lúc ứng dụng chạy), bật/tắt thông báo lưu và sao chép. Chủ dự án đã nghiệm thu đợt 2 qua thử tay (2026-11-16).
- Chưa làm (đợt 3): tải lên bằng PNG hoặc JPEG, chất lượng JPEG. Chưa có thiết kế. Proxy không làm ở mọi đợt.
- Bấm `PrintScreen` (đổi được): màn hình đứng yên và tối đi, kéo chuột chọn vùng, có nhãn kích thước. Chỉnh khung bằng 8 tay nắm, kéo bên trong để di chuyển, kéo bên ngoài để chọn lại. Ảnh nền có con trỏ chuột nếu ô "Bao gồm con trỏ chuột" đang bật.
- Phím toàn màn hình (tắt mặc định): `Shift+PrintScreen` lưu, `Ctrl+PrintScreen` tải lên; không mở lớp phủ, không có vùng chọn.
- Thanh công cụ bốn nút chỉ có biểu tượng; rê chuột hiện tên và phím tắt:
  - Tải lên (`Ctrl+D`): tải lên R2, chép link vào clipboard, thông báo "Đã chép link".
  - Lưu (`Ctrl+S`): lưu PNG vào `Pictures\shotme`, tên `shotme_YYYY-MM-DD_HH-MM-SS.png`, không hỏi lại.
  - Chép (`Ctrl+C`): chép ảnh vào clipboard.
  - Hủy (`Esc`).
- Tải lên thất bại hoặc R2 chưa cấu hình: ảnh được lưu local, thông báo nêu lý do và nơi lưu. Chưa cấu hình thì mở Tùy chọn.
- Tùy chọn: thông tin R2 và nút Kiểm tra kết nối, hạn của link, phím tắt, thư mục lưu, khởi động cùng Windows, ngôn ngữ (vị trí từng ô xem mục trên). Ô khóa bí mật không hiện lại giá trị đã lưu.
- Phím tắt bị ứng dụng khác chiếm: ứng dụng vẫn chạy, báo rõ, vẫn chụp được qua menu.
- Giao diện tiếng Việt và English.

## Link chia sẻ

- Dạng `{địa chỉ công khai}/{d1|d7|d30|keep}/{16 ký tự ngẫu nhiên}.png`.
- Ai có link đều xem được. Riêng tư dựa vào tên không đoán được và hạn của link.
- Cloudflare xóa ảnh theo quy tắc vòng đời của từng thư mục; chủ dự án tự tạo ba quy tắc một lần.

## Việc của chủ dự án

Tạo kho R2, bật truy cập công khai, tạo khóa API chỉ đọc và ghi trong kho đó, tạo ba quy tắc xóa, nhập thông tin vào Tùy chọn, tự chạy nghiệm thu với kho thật. Agent không được nhận khóa.
