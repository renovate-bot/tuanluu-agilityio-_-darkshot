# shotme-windows

File chạy `shotme.exe`: hiện thực các trait của core bằng Win32.

- Chỉ vẽ, nhận sự kiện và gọi core. Logic tính toán thuộc về `shotme-core`.
- Ứng dụng khai báo DPI `PerMonitorV2`; mọi tọa độ là pixel vật lý.
- Cửa sổ và vòng lặp thông điệp chạy ở luồng chính. Luồng nền chỉ báo về bằng `PostMessage`.
- `unsafe` nhỏ gọn, kèm một dòng chú thích vì sao an toàn. Giải phóng mọi handle trên mọi đường đi.
- Chuỗi hiển thị bằng tiếng Việt, dùng API Unicode (hậu tố `W`).
- Phần nào kiểm thử tự động được thì tách thành hàm thuần và kiểm thử; phần còn lại ghi vào `docs/testing/manual-checklist.md`.
