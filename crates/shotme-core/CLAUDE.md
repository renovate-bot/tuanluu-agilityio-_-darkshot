# shotme-core

Thư viện logic, không phụ thuộc hệ điều hành.

- Không dùng crate `windows` hay bất kỳ API riêng của hệ điều hành nào. `scripts/check.ps1` kiểm tra điều này.
- Không gọi API giờ địa phương của hệ điều hành; nền tảng truyền `naming::LocalTime` vào. Giờ UTC qua `std::time::SystemTime` thì được dùng (ví dụ `sigv4::amz_date_now`).
- Mọi trait cho nền tảng có ràng buộc `Send + Sync`.
- HTTPS chỉ qua `rustls` với `webpki-roots`.
- Chữ cho người dùng nằm trong `i18n.rs` (có từ Task 15), mỗi chuỗi đủ Tiếng Việt và English. Core không giữ ngôn ngữ hiện hành: hàm nào sinh chữ thì nhận `Lang` từ bên gọi.
- Khóa bí mật không xuất hiện trong lỗi, `Debug` hay đầu ra kiểm thử.
- Mọi hàm công khai có kiểm thử tự động trong cùng file (`#[cfg(test)]`).
