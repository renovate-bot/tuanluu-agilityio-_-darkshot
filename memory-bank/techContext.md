# Bối cảnh kỹ thuật

Cập nhật: 2026-11-16

## Môi trường phát triển

- Windows 11, 64-bit. Thư mục dự án: `D:\2026 - Tuan\shotme` (đường dẫn có dấu cách, luôn đặt trong dấu nháy).
- Rust 1.100.0 stable, target `x86_64-pc-windows-msvc`, cài qua rustup. Có sẵn `clippy` và `rustfmt`.
- Visual Studio 2022 Build Tools, workload C++ (cung cấp linker MSVC).
- Git 2.47. Nhánh chính `main`. Remote `origin`: repo GitHub riêng tư `tuanluu-agilityio/shotme`.
- `cargo` nằm ở `%USERPROFILE%\.cargo\bin`. Nếu shell chưa thấy `cargo`, thêm thư mục đó vào `PATH` của phiên.

## Lệnh

| Việc | Lệnh |
|---|---|
| Cổng kiểm tra trước khi gộp | `powershell -File scripts/check.ps1` (phải in `ALL CHECKS PASSED`) |
| Kiểm thử một crate | `cargo test -p shotme-core <bộ lọc>` |
| Build bản phát hành | `cargo build --release` (ra `target/release/shotme.exe`) |
| Thêm crate | `cargo add -p <crate của dự án> <tên>` |

## Thư viện dự kiến

| Crate | Dùng ở | Để làm gì |
|---|---|---|
| `serde`, `serde_json`, `base64` | core | file cấu hình |
| `getrandom` | core | tên ngẫu nhiên |
| `sha2`, `hmac` | core | chữ ký SigV4 |
| `ureq` (chỉ `rustls` + `webpki-roots`) | core | HTTPS tới R2 |
| `png` | core | mã hóa ảnh |
| `windows` | windows | Win32 |
| `embed-resource` hoặc tương đương | windows | nhúng biểu tượng và manifest |

Dùng bản mới nhất tại thời điểm thêm. Phiên bản thực tế nằm trong `Cargo.lock`.

## Ràng buộc kỹ thuật

- `shotme-core` không phụ thuộc crate `windows`, `native-tls`, `schannel`, `rustls-platform-verifier`.
- Không dùng TLS của hệ điều hành.
- Profile release: `opt-level = "z"`, `lto = true`, `codegen-units = 1`, `panic = "abort"`, `strip = true`.
- R2: địa chỉ S3 `https://{account_id}.r2.cloudflarestorage.com`, vùng `auto`, dịch vụ `s3`, đường dẫn `/{bucket}/{key}`.
- Phiên đám mây của Claude Code chạy Ubuntu nên không build được phần Windows; dự án làm trên máy local.

## Phát sinh

(Ghi thêm ở đây khi thêm phụ thuộc hoặc đổi công cụ.)
