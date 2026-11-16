# Task 5: Tải lên R2

**Agent:** `builder` · **Model:** sonnet · **Nhánh:** `task/05-upload`

Gói task tự chứa. Chỉ đọc file này và các file ở mục "Cần đọc".

## Cần đọc

- `memory-bank/systemPatterns.md`
- `memory-bank/techContext.md`
- docs/superpowers/specs/2026-11-16-shotme-design.md mục 5.5, 6
- `crates/shotme-core/CLAUDE.md`
- `crates/shotme-core/src/sigv4.rs`
- `crates/shotme-core/src/config.rs`
- `crates/shotme-core/src/naming.rs`

## Global Constraints

- Chỉ mở kết nối mạng đến hai nơi: `https://{account_id}.r2.cloudflarestorage.com` và địa chỉ công khai của kho do người dùng nhập. Không thống kê, không kiểm tra cập nhật, không báo lỗi tự động.
- HTTPS dùng `rustls` với `webpki-roots`. Không dùng `native-tls`, không dùng bộ kiểm chứng chứng chỉ của hệ điều hành.
- Secret Access Key không bao giờ xuất hiện ở dạng đọc được trên đĩa, trong log, trong thông báo lỗi hay trong đầu ra kiểm thử.
- Không ghi log ra file.
- `shotme-core` không được phụ thuộc crate `windows` hay bất kỳ API riêng của hệ điều hành nào. `cargo tree -p shotme-core` không chứa `windows`.
- Mọi trait nền tảng trong core có ràng buộc `Send + Sync`.
- Giao diện và thông báo bằng tiếng Việt. Câu cố định: tải lên thành công báo "Đã chép link".
- Tên ảnh trên kho: 16 ký tự `[A-Za-z0-9]` từ bộ sinh số ngẫu nhiên của hệ điều hành, không lệch phân phối.
- Tên file local: `shotme_YYYY-MM-DD_HH-MM-SS.png`, trùng thì thêm `_2`, `_3`.
- Thư mục hạn trên kho: `d1`, `d7`, `d30`, `keep`. Mặc định `d30`.
- Phím tắt mặc định: `PrintScreen` (VK `0x2C`), không phím bổ trợ.
- File cấu hình: `%APPDATA%\shotme\config.json`. Thư mục lưu mặc định: `Pictures\shotme`.
- Không xây tính năng ngoài phạm vi của spec (vẽ lên ảnh, lịch sử, xóa từ ứng dụng).
- `cargo test --workspace` và `cargo clippy --workspace -- -D warnings` phải qua trước khi gộp nhánh.

## Nội dung task
**Files:**
- Create: `crates/shotme-core/src/upload.rs`
- Modify: `crates/shotme-core/src/lib.rs`, `crates/shotme-core/Cargo.toml` (thêm `ureq` cấu hình chỉ dùng `rustls` với `webpki-roots`, tắt mọi tính năng TLS khác)

**Interfaces:**
- Consumes: `sigv4::{Credentials, SignInput, authorization_header, sha256_hex, amz_date_now}`, `config::{R2Config, Expiry}`, `naming::random_object_name`.
- Produces:

```rust
#[derive(Debug, PartialEq, Eq)]
pub enum UploadError { NotConfigured, Network(String), Auth, BucketNotFound, Http(u16), PublicUrl(String) }
impl UploadError { pub fn message_vi(&self) -> String; }      // câu tiếng Việt cho thông báo
pub trait Uploader: Send + Sync {
    fn upload(&self, key: &str, png: &[u8]) -> Result<String, UploadError>;   // trả link công khai
}
pub fn object_key(expiry: Expiry, name: &str) -> String;       // "d30/{name}.png"
pub fn public_url(base: &str, key: &str) -> String;            // "{base}/{key}"
pub struct R2Uploader { /* riêng tư */ }
impl R2Uploader {
    pub fn new(cfg: &R2Config) -> Self;                         // https://{account_id}.r2.cloudflarestorage.com
    pub fn with_endpoints(cfg: &R2Config, s3_endpoint: &str, public_base: &str) -> Self;  // cho kiểm thử
    pub fn test_connection(&self) -> Result<(), UploadError>;
}
impl Uploader for R2Uploader { /* ... */ }
```

Quy tắc:
- `upload`: `PUT {endpoint}/{bucket}/{key}`, header `content-type: image/png`, `x-amz-content-sha256`, `x-amz-date`, `authorization`. Vùng `auto`, dịch vụ `s3`.
- Ánh xạ lỗi: không kết nối được hoặc quá giờ là `Network`; HTTP 401 và 403 là `Auth`; 404 là `BucketNotFound`; mã khác ngoài 2xx là `Http(mã)`. `cfg.is_complete()` sai là `NotConfigured`, không gửi gì.
- Giới hạn thời gian: kết nối 10 giây, toàn bộ yêu cầu 60 giây.
- `test_connection`: PUT `d1/shotme-test-{random_object_name()}.txt` nội dung `shotme`; GET cùng key qua địa chỉ công khai, nội dung phải là `shotme`, nếu không thì `PublicUrl(...)`; cuối cùng DELETE qua S3, kể cả khi bước GET lỗi.
- `message_vi` và `Debug` không chứa khóa bí mật.

- [ ] **Step 1: Viết kiểm thử thất bại.** Viết máy chủ giả trong module kiểm thử: `std::net::TcpListener` trên `127.0.0.1:0` chạy ở một luồng, nhận danh sách phản hồi định sẵn, ghi lại mọi yêu cầu (dòng đầu, header, thân) vào `Arc<Mutex<Vec<Recorded>>>`.

```rust
#[test] fn key_and_url() {
    assert_eq!(object_key(Expiry::D30, "abc"), "d30/abc.png");
    assert_eq!(object_key(Expiry::Keep, "abc"), "keep/abc.png");
    assert_eq!(public_url("https://img.example.com", "d7/abc.png"), "https://img.example.com/d7/abc.png");
}
#[test] fn upload_sends_signed_put_and_returns_public_url() {
    // máy chủ giả trả 200
    // => Ok("https://img.example.com/d30/abc.png")
    // yêu cầu ghi lại: dòng đầu "PUT /bkt/d30/abc.png HTTP/1.1", thân == png,
    // content-type == image/png, authorization bắt đầu "AWS4-HMAC-SHA256 Credential=kid/",
    // x-amz-content-sha256 == sha256_hex(png)
}
#[test] fn status_mapping() {
    // 403 => Auth, 401 => Auth, 404 => BucketNotFound, 500 => Http(500)
}
#[test] fn incomplete_config_sends_nothing() {
    // R2Config thiếu bucket => Err(NotConfigured), máy chủ giả không nhận yêu cầu nào
}
#[test] fn connection_refused_is_network_error() {
    // endpoint là cổng đã đóng => Err(Network(_))
}
#[test] fn stalled_server_times_out() {                         // Review Focus 4
    // máy chủ giả nhận kết nối rồi không trả lời; dùng bộ tạo có thời hạn 1 giây cho kiểm thử
    // => Err(Network(_)) trong dưới 5 giây
}
#[test] fn test_connection_put_get_delete() {
    // ba yêu cầu theo thứ tự PUT, GET, DELETE trên cùng key bắt đầu bằng "d1/shotme-test-"
}
#[test] fn test_connection_reports_public_url_problem_and_still_deletes() {
    // GET trả 404 => Err(PublicUrl(_)), và vẫn có yêu cầu DELETE
}
#[test] fn error_text_never_contains_secret() {
    // với mọi biến thể lỗi: message_vi() và format!("{:?}") không chứa "TOP-SECRET-VALUE"
}
```

Để kiểm thử thời hạn, thêm hàm tạo chỉ dùng trong kiểm thử (`#[cfg(test)]`) nhận thời hạn.

- [ ] **Step 2:** Chạy `cargo test -p shotme-core upload`. Mong đợi: lỗi biên dịch.
- [ ] **Step 3:** Hiện thực.
- [ ] **Step 4:** Chạy lại (9 test PASS). Chạy `cargo tree -p shotme-core -e normal` và xác nhận không có `native-tls`, `schannel`, `rustls-platform-verifier`, `windows`. Nếu `ureq` kéo crate `windows-sys` qua phụ thuộc gián tiếp của socket thì chấp nhận được, ghi chú trong báo cáo.
- [ ] **Step 5:** Commit.
