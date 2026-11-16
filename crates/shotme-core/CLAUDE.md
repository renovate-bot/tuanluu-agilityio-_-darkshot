# shotme-core

Thư viện logic, không phụ thuộc hệ điều hành.

- Không dùng crate `windows` hay bất kỳ API riêng của hệ điều hành nào. `scripts/check.ps1` kiểm tra điều này.
- Không gọi API thời gian máy; nền tảng truyền `naming::LocalTime` vào.
- Mọi trait cho nền tảng có ràng buộc `Send + Sync`.
- HTTPS chỉ qua `rustls` với `webpki-roots`.
- Khóa bí mật không xuất hiện trong lỗi, `Debug` hay đầu ra kiểm thử.
- Mọi hàm công khai có kiểm thử tự động trong cùng file (`#[cfg(test)]`).
