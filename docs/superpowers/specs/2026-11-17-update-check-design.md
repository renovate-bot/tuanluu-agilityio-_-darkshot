# Thiết kế: báo có bản mới

Ngày: 2026-11-17. Trạng thái: đã duyệt (chủ dự án chọn "cách 2" và đồng ý thiết kế ngắn, 2026-11-17).

## Ý định

darkshot không có trình cài đặt và hiện không có cách nào để người dùng biết đã có bản mới. Chủ dự án muốn ứng dụng tự hỏi GitHub, rồi báo khi có bản mới hơn. Người dùng vẫn tự tải và tự thay exe.

## Các cách đã cân nhắc

1. **Theo dõi release trên GitHub (Watch).** Không cần sửa mã, nhưng người dùng phải tự bật và phải có tài khoản GitHub. Không đủ.
2. **Ứng dụng tự kiểm tra và báo, người dùng tự tải (đã chọn).**
3. **Tự cập nhật hoàn toàn (tự tải và thay exe).** Phức tạp. Exe chưa ký số nên tự tải rồi chạy là không an toàn. Loại.

## Hành vi

- **Nguồn:** `GET https://api.github.com/repos/tuanluu-agilityio/darkshot/releases/latest`.
  - Chỉ gửi header `User-Agent: darkshot/<phiên bản>` và `Accept: application/vnd.github+json`, ngoài ra không gửi gì.
  - Không có khóa hay token.
  - Repo riêng tư thì GitHub trả 404, và ứng dụng coi như không có bản mới.
- **Lịch:** kiểm tra lần đầu 60 giây sau khi khởi động, sau đó 24 giờ một lần trong lúc ứng dụng còn chạy. Mạng chạy ở luồng nền và không chặn giao diện.
- **So sánh:**
  - Đọc `tag_name` dạng `vX.Y.Z` hoặc `X.Y.Z`, so với `CARGO_PKG_VERSION` theo số (X, Y, Z).
  - Bỏ qua bản `draft` hoặc `prerelease`, tag không đọc được, và bản không mới hơn.
- **Link:** dùng `html_url` của release, chỉ khi nó bắt đầu đúng bằng `https://github.com/tuanluu-agilityio/darkshot/`. Link khác thì coi như không có bản mới (không mở link lạ).
- **Có bản mới:**
  - Menu khay có mục "Có bản mới X.Y.Z" ở trên cùng, có vạch ngăn bên dưới. Bấm vào thì mở link bằng trình duyệt mặc định.
  - Mỗi phiên bản chỉ được báo một lần bằng thông báo khay, bấm vào thông báo cũng mở link. Số phiên bản đã báo lưu vào cấu hình (trường `update_notified`), theo đúng luật ghi cấu hình của quyết định 72. Không ghi được thì chỉ nhớ trong bộ nhớ.
  - Thông báo này hiện cả khi người dùng đã tắt "thông báo thành công".
- **Lỗi** (không mạng, hết thời gian, mã khác 200, JSON lạ): im lặng, đợi lần sau. Không có thông báo lỗi.
- **Tùy chọn, tab Thông thường:** ô "Tự kiểm tra bản mới", mặc định bật (trường cấu hình `check_updates`). Tắt thì không gửi yêu cầu nào và mục menu biến mất. Bật lại thì kiểm tra sau 60 giây.

## Ràng buộc

- Không thêm crate: dùng `ureq` (rustls) đã có ở core và `serde_json`.
- Mã mạng và phần phân tích kết quả nằm ở `darkshot-core`. Hẹn giờ, menu, thông báo và mở trình duyệt (`ShellExecuteW`) nằm ở lớp Windows.
- Mọi chữ hiển thị lấy từ `i18n`, có đủ hai ngôn ngữ.
- Release dùng `panic = "abort"`: phân tích JSON không được panic.
- File cấu hình cũ vẫn đọc được. Bản darkshot cũ đọc file mới vẫn chạy.
- Thay phần "không gọi mạng, chỉ hai đích mạng" của quyết định 31. Đích mạng thứ ba là `api.github.com`, và chỉ được gọi khi ô kiểm tra bản mới đang bật.

## Ngoài phạm vi

Tự tải hoặc thay exe; nút "Kiểm tra ngay"; hiển thị nội dung thay đổi của bản mới; kênh bản thử nghiệm.
