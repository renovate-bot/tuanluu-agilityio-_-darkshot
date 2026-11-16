# Bối cảnh hiện tại

Cập nhật: 2026-11-16

## Đang ở đâu

- Giai đoạn: 5. Xây dựng.
- Task vừa xong: Task 5 (tải lên R2), đã gộp vào `main`.
- Task đang làm: **Task 6, `docs/tasks/06-image-actions.md`**, nhánh `task/06-image-actions`, agent `builder`, trạng thái `đang làm`.

## Bước kế tiếp

Chạy `/next-task` nối tiếp, không hỏi lại chủ dự án giữa các task (quyết định 18). Chỉ dừng sau Task 8, sau Task 12, hoặc khi có việc chỉ chủ dự án quyết được.

## Đang chờ chủ dự án

- Chưa có.

## Điểm dừng sắp tới cần chủ dự án

- Sau Task 8: thử lớp phủ chọn vùng theo mục "Lớp phủ" của `docs/testing/manual-checklist.md`.
- Sau Task 12: nghiệm thu với kho R2 thật.

## Ghi chú cho phiên sau

- Agent trong `.claude/agents/` được nạp khi khởi động phiên. Nếu phiên hiện tại chưa thấy chúng, giao việc bằng agent `general-purpose` với model tương ứng và dán nội dung file agent vào lời giao.
- Luôn kiểm tra `git branch --show-current` trước khi commit: một agent con từng để kho ở nhánh task sau khi phiên chính đã về `main`. Trong lời giao, ghi rõ "không chuyển nhánh".
- Agent `builder` ở Task 2 viết mã và kiểm thử cùng lúc, bỏ bước thấy kiểm thử thất bại. Từ Task 3, lời giao yêu cầu commit kiểm thử trước (commit `test:` riêng, thân hàm `todo!()`), rồi mới commit mã (`feat:`). Task 3 đã làm đúng như vậy.
- Task 3: `R2Config` tự viết `Debug` để che khóa; `load` dùng mặc định cho trường JSON thiếu và `expiry` lạ; `save` ghi file tạm, `sync_all`, rồi đổi tên.
- Task 4: trang tài liệu AWS chứa ví dụ SigV4 không còn truy cập được. Hai ví dụ GET và PUT trong kiểm thử đã được đối chiếu bằng một hiện thực Python độc lập (ra đúng hai chữ ký). `authorization_header` nối các phần bằng dấu phẩy không khoảng trắng, gộp khoảng trắng trong giá trị header, gộp header trùng tên. Người gọi phải truyền `path` đã mã hóa URI.
- Task 5: `R2Uploader` trả `NotConfigured` (không gửi gì) khi `account_id` có ký tự ngoài `[A-Za-z0-9-]`, tên kho ngoài `[A-Za-z0-9._-]`, hoặc địa chỉ công khai không qua `normalize_public_base_url`. Chưa thử với R2 thật.
- Từ Task 5, `scripts/check.ps1` chạy thêm `cargo fmt --all --check`; agent phải chạy `cargo fmt --all` trước khi commit.
- Trong bash của máy này `cargo` không có trong PATH; chạy cargo qua PowerShell hoặc qua `scripts/check.ps1`.
- Việc để sau (ngoài bản đầu): xóa trắng vùng nhớ chứa khóa sau khi ký (cần crate `zeroize`).
- Sửa nhỏ sau rà soát: gọi tiếp chính agent đã làm bằng SendMessage (còn nguyên ngữ cảnh), không tạo agent mới.
- Task 2, lựa chọn của agent đã được rà soát chấp nhận: tay nắm nhận trên cả dải cạnh; `release` bỏ khung khi một chiều nhỏ hơn 3; `place_toolbar` kẹp cả hai chiều. Task 8 (lớp phủ) cần biết: khung nhỏ hơn hai lần bán kính tay nắm thì không còn vùng "Inside" để di chuyển.
- Cargo báo cảnh báo "unused dependency shotme-core" ở `shotme-windows`; sẽ hết khi Task 7 dùng core. Không cần sửa.
- Chủ dự án có thể điều khiển phiên từ claude.ai bằng `/remote-control`.
