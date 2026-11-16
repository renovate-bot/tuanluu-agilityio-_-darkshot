# Tóm tắt dự án

Nguồn đầy đủ: `docs/superpowers/specs/2026-11-16-shotme-design.md`, mục 1 và 2.

## Mục đích

shotme thay Lightshot cho riêng chủ dự án. Lightshot tải ảnh lên máy chủ của hãng; chủ dự án muốn ảnh chia sẻ nằm trong kho lưu trữ đứng tên mình, vì lý do an toàn và bảo mật.

## Mục tiêu bản đầu

1. Chụp một vùng màn hình bằng phím tắt.
2. Lưu ảnh ra file trên máy.
3. Tải ảnh lên Cloudflare R2 của chủ dự án và nhận link chia sẻ.
4. Link tự hết hạn: mặc định 30 ngày, chọn được 1 ngày, 7 ngày, không hết hạn.
5. Gọn nhẹ: file nhỏ, ít RAM khi chạy ngầm, phản hồi ngay khi bấm phím tắt.

## Ngoài phạm vi bản đầu

- Vẽ lên ảnh (mũi tên, chữ, khung, làm mờ).
- Lịch sử ảnh đã tải lên; xóa ảnh trên kho từ ứng dụng.
- Chụp theo cửa sổ, chụp cuộn trang, quay video.
- Bản chạy trên macOS và Linux (kiến trúc phải sẵn sàng, nhưng chưa xây).

## Điều phải tránh

- Gửi ảnh hoặc dữ liệu đến bất kỳ máy chủ nào ngoài hai địa chỉ R2 đã cấu hình. Không thống kê, không kiểm tra cập nhật.
- Ghi khóa bí mật R2 ra đĩa hoặc log ở dạng đọc được.
- Tạo link đoán được.
- Làm mất ảnh vừa chụp khi tải lên thất bại.

## Yêu cầu về quy trình (chủ dự án đặt ra)

- Theo các giai đoạn của một SDLC, có prototype.
- AI làm là chính; task tối ưu theo loại model.
- Lưu tiến trình để mở lại làm tiếp mà không đọc lại toàn bộ (memory bank).
- Mỗi task một nhánh riêng.
- Ghi số token của từng task để thống kê cuối dự án.
- Dễ mở rộng; có thể hỗ trợ macOS và Linux sau này.
