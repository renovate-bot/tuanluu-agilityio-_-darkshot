# Task 19b: Kiểm thử chập chờn và chữ "Đổi…" trong Trợ giúp

**Agent:** `builder` · **Model:** sonnet · **Nhánh:** `task/19b-flaky-test-help-text`

Gói task tự chứa. Hai điểm nhỏ còn lại sau rà soát đợt 1.

## Cần đọc

- `crates/shotme-core/CLAUDE.md`
- `crates/shotme-core/src/upload.rs` (phần `#[cfg(test)]`, nhất là `connection_refused_is_network_error` và `spawn_mock`)
- `crates/shotme-core/src/i18n.rs` (`help_text` và mục nhãn nút "Đổi…")

## Global Constraints

- Core không phụ thuộc crate `windows` hay API hệ điều hành. Không thêm crate mới (kể cả `socket2`).
- Mọi chữ người dùng thấy lấy từ bảng `i18n`, đủ hai ngôn ngữ.
- Không sửa mã chạy thật của `upload.rs`: chỉ sửa kiểm thử.
- `cargo fmt --all` trước khi commit. Chạy cargo qua PowerShell. Không chuyển nhánh. Không tắt tiến trình `shotme` nào. Không đẩy lên remote. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.

## Nội dung

### A. `upload::tests::connection_refused_is_network_error` chập chờn

Kiểm thử bind `127.0.0.1:0` để lấy một cổng rồi thả listener ngay, và giả định cổng đó từ chối kết nối. Các kiểm thử khác chạy song song cũng bind cổng 0 (`spawn_mock`), nên hệ điều hành có thể cấp lại đúng cổng vừa thả cho một máy chủ giả; khi đó kết nối thành công và kết quả không còn là `UploadError::Network`.

Sửa sao cho không kiểm thử nào khác lấy được cổng đó trong lúc kiểm thử này chạy, chỉ bằng thư viện chuẩn. Cách đề xuất: giữ một `TcpListener` sống suốt kiểm thử nhưng không bao giờ `accept`, và cho máy khách kết nối tới một địa chỉ chắc chắn bị từ chối theo cách khác; hoặc dùng một `Mutex` tĩnh chung mà mọi kiểm thử bind cổng 0 trong module này đều giữ trong lúc chúng lấy cổng và chạy. Chọn cách đơn giản nhất mà bạn chứng minh được là hết cuộc đua; kiểm thử vẫn phải khẳng định đúng `UploadError::Network(_)` cho trường hợp kết nối bị từ chối, và không được chậm hơn đáng kể.

- [ ] Giải thích trong báo cáo vì sao cách bạn chọn loại được cuộc đua.
- [ ] Chạy `cargo test -p shotme-core upload` 20 lần liên tiếp bằng một vòng lặp PowerShell; cả 20 lần phải qua. Ghi kết quả vào báo cáo.
- [ ] Commit `test: remove port race in connection_refused test`.

### B. Chữ "Đổi..." trong `help_text` bản Việt

Nhãn nút thật trong bảng là "Đổi…" (ký tự U+2026). `help_text(Lang::Vi, …)` đang viết "Đổi..." (ba dấu chấm).

- [ ] Thêm vào kiểm thử `help_text_names_the_hotkey_and_keys` (hoặc một kiểm thử mới cạnh nó): với mỗi ngôn ngữ, `help_text` chứa đúng nhãn của nút đổi phím tắt lấy từ bảng (`t(lang, Msg::<mục nút Đổi>)`). Chạy, thấy bản Việt thất bại. Commit `test: help text uses the real button label`.
- [ ] Sửa `help_text` để dùng đúng nhãn (tốt nhất chèn `t(lang, …)` vào câu thay vì chép lại chữ, cho cả hai ngôn ngữ). Kiểm thử qua. Commit `fix: help text names the Change button exactly`.

**Xong khi:** `powershell -File scripts/check.ps1` in `ALL CHECKS PASSED`; báo cáo có kết quả 20 lần chạy của mục A.
