# Task 4: Chữ ký AWS Signature V4

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/04-sigv4`

Gói task tự chứa. Chỉ đọc file này và các file ở mục "Cần đọc".

## Cần đọc

- `memory-bank/systemPatterns.md`
- `memory-bank/techContext.md`
- `crates/shotme-core/CLAUDE.md`
- Tài liệu AWS nêu trong Step 1

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
- Create: `crates/shotme-core/src/sigv4.rs`
- Modify: `crates/shotme-core/src/lib.rs`, `crates/shotme-core/Cargo.toml` (thêm `sha2`, `hmac`)

**Interfaces:**
- Produces:

```rust
pub struct Credentials<'a> { pub access_key_id: &'a str, pub secret_access_key: &'a str }
pub struct SignInput<'a> {
    pub method: &'a str,
    pub path: &'a str,                    // đã mã hóa URI, bắt đầu bằng '/'
    pub query: &'a str,                   // chuỗi truy vấn chuẩn tắc, "" nếu không có
    pub headers: &'a [(&'a str, &'a str)],// mọi header được ký, gồm host, x-amz-date, x-amz-content-sha256
    pub payload_sha256_hex: &'a str,
    pub amz_date: &'a str,                // "YYYYMMDDTHHMMSSZ"
    pub region: &'a str,
    pub service: &'a str,
}
pub fn sha256_hex(data: &[u8]) -> String;
pub fn authorization_header(c: &Credentials, i: &SignInput) -> String;
pub fn amz_date_now() -> String;          // giờ UTC từ SystemTime, không thêm crate thời gian
```

`authorization_header` trả đúng dạng `AWS4-HMAC-SHA256 Credential={key}/{yyyymmdd}/{region}/{service}/aws4_request, SignedHeaders={a;b;c}, Signature={hex}` (trong ví dụ của AWS các phần cách nhau bằng dấu phẩy không có khoảng trắng; so sánh từng phần). Tên header được hạ chữ thường và sắp xếp; giá trị được cắt khoảng trắng hai đầu.

- [ ] **Step 1: Viết kiểm thử thất bại** dùng hai ví dụ chính thức trong tài liệu AWS "Signature Calculations for the Authorization Header: Transferring Payload in a Single Chunk (AWS Signature Version 4)": ví dụ **GET Object** và ví dụ **PUT Object**. Mở trang tài liệu đó và chép nguyên văn khóa, ngày, header và chữ ký mong đợi vào kiểm thử; không chép từ trí nhớ. Để đối chiếu: ví dụ GET dùng khóa `AKIAIOSFODNN7EXAMPLE`, ngày `20130524T000000Z`, vùng `us-east-1`, đường dẫn `/test.txt`, header `range: bytes=0-9`, và chữ ký trong tài liệu là `f0e8bdb87c964420e857bd35b5d6ed310bd44f0170aba48dd91039c6036bdb41`. Nếu trang tài liệu khác con số này thì theo trang tài liệu và ghi chú lại trong báo cáo.

```rust
#[test] fn aws_doc_example_get_object() { /* Signature == chữ ký trong tài liệu */ }
#[test] fn aws_doc_example_put_object() { /* Signature == chữ ký trong tài liệu */ }
#[test] fn sha256_of_empty() {
    assert_eq!(sha256_hex(b""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
}
#[test] fn headers_sorted_and_lowercased() {
    // cùng đầu vào, thứ tự và kiểu chữ của headers khác nhau => cùng kết quả
}
#[test] fn amz_date_shape() {
    let d = amz_date_now();
    assert!(d.len() == 16 && d.as_bytes()[8] == b'T' && d.ends_with('Z'));
}
#[test] fn civil_date_known_values() {
    // hàm nội bộ đổi giây Unix sang ngày giờ UTC: 0 => 19700101T000000Z,
    // 951782400 => 20000229T000000Z, 1369353600 => 20130524T000000Z
}
```

- [ ] **Step 2:** Chạy `cargo test -p shotme-core sigv4`. Mong đợi: lỗi biên dịch.
- [ ] **Step 3:** Hiện thực. Thông báo lỗi và `Debug` không được in `secret_access_key`.
- [ ] **Step 4:** Chạy lại. Mong đợi: 6 test PASS.
- [ ] **Step 5:** Commit.
