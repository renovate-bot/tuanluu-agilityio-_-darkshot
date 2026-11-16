# Task 12: Tài liệu và bản phát hành

**Agent:** `scaffolder` · **Model:** haiku · **Nhánh:** `task/12-docs-release`

Gói task tự chứa. Chỉ đọc file này và các file ở mục "Cần đọc".

## Cần đọc

- docs/superpowers/specs/2026-11-16-shotme-design.md mục 5, 6, 7
- `memory-bank/productContext.md`
- `docs/testing/manual-checklist.md`

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
- Create: `README.md`
- Modify: `docs/testing/manual-checklist.md`

- [ ] **Step 1:** Viết `README.md` bằng tiếng Việt gồm: shotme là gì; cách build (`cargo build --release`); cách dùng và bảng phím tắt; mục "Chuẩn bị Cloudflare R2" theo năm bước của spec mục 7, trong đó nêu rõ khóa API chỉ cấp quyền đọc và ghi đối tượng cho đúng một kho, ba quy tắc vòng đời (`d1/` 1 ngày, `d7/` 7 ngày, `d30/` 30 ngày), không bật liệt kê nội dung công khai; mục "Kiểm tra quy tắc xóa" (tải một ảnh với hạn 1 ngày, xem nó biến mất); mục "Phím PrintScreen" (tắt Lightshot, tắt việc gán PrintScreen cho Snipping Tool trong cài đặt Windows); mục "Bảo mật" tóm tắt spec mục 6. Các bước thao tác trên trang Cloudflare phải lấy từ tài liệu hiện hành của Cloudflare R2, kể cả độ trễ xóa của quy tắc vòng đời; không viết theo trí nhớ.
- [ ] **Step 2:** Thêm vào `docs/testing/manual-checklist.md` mục "Nghiệm thu với R2 thật" cho chủ dự án: nhập khóa, Kiểm tra kết nối thành công, tải lên và mở link ở trình duyệt khác, link có thư mục hạn đúng, và quan sát bằng Resource Monitor rằng `shotme.exe` chỉ kết nối tới hai địa chỉ R2.
- [ ] **Step 3:** `cargo build --release`. Ghi kích thước `target/release/shotme.exe` vào `README.md`.
- [ ] **Step 4:** Commit.

**Xong khi:** README đủ các mục trên; file `.exe` tồn tại và kích thước được ghi lại.
