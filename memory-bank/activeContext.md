# Bối cảnh hiện tại

Cập nhật: 2026-11-16

## Đang ở đâu

- Giai đoạn: 5. Xây dựng.
- Task vừa xong: Task 8 (chụp màn hình và lớp phủ), đã gộp vào `main`. Lõi 57 kiểm thử; lớp Windows 10 kiểm thử và 1 kiểm thử chạy tay (`overlay_round_trip_without_gdi_leaks`, `-- --ignored`). Rà soát hai lượt (lượt hai chỉ rà phần sửa).
- Task đang làm: không. **ĐANG DỪNG** chờ chủ dự án thử tay. Task kế tiếp sau khi họ báo: Task 9, `docs/tasks/09-win-services.md`, agent `specialist`.

## Bước kế tiếp

Chạy `/next-task` nối tiếp, không hỏi lại chủ dự án giữa các task (quyết định 18). Chỉ dừng sau Task 8, sau Task 12, hoặc khi có việc chỉ chủ dự án quyết được.

## Đang chờ chủ dự án

- Thử `target/release/shotme.exe` theo `docs/testing/manual-checklist.md`, mục "Vỏ ứng dụng" và "Lớp phủ". Lỗi họ báo: giao agent `specialist` sửa trên nhánh mới `task/08b-...` trước khi sang Task 9.

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
- Task 6, khác gói task: `image::encode_png` trả `Result<Vec<u8>, String>`; có thêm `Image::is_valid()`. `run_action` với ảnh không hợp lệ trả `Failed` kèm một thông báo Error. Sao chép lỗi clipboard trả `Failed`; tải lên xong mà không chép được link vẫn trả `Uploaded(link)` với thông báo Error chứa link. Bản release dùng `panic = "abort"`: mã chạy thật không được `unwrap`/`expect` trên dữ liệu đầu vào.
- Task 7: `windows` 0.62.2, `embed-resource` 3.0.12; có thêm `res/app.rc`. Cửa sổ ẩn là top-level `WS_EX_TOOLWINDOW` (để nhận "TaskbarCreated"). `Tray` có `try_add()`, `is_added()`; thêm biểu tượng thất bại thì thử lại mỗi 2 giây, tối đa 30 lần. `mod paths` đang có `#[allow(dead_code)]`, Task 11 bỏ đi. Phím tắt đang dùng `Hotkey::default()`, Task 11 đọc từ cấu hình. Chưa ai nhìn thấy giao diện bằng mắt: chờ chủ dự án thử theo checklist.
- Task 8: `overlay::pick_region(&Screenshot) -> Result<Option<(Action, Rect)>, String>` (khác kế hoạch: `Err` là lỗi mở lớp phủ, `Ok(None)` là hủy). `main.rs` có cổng chụp `CaptureGate`/`gate_step` (Idle, PendingMenu, Capturing) thay cho cờ; mọi lối chụp phải đi qua `gate()`. Chụp từ menu khay hoãn 250 ms. Esc là phím tắt toàn cục tạm trong lúc lớp phủ mở. Lớp phủ dùng bộ đệm vẽ cố định cỡ màn hình ảo (thêm w×h×4 byte). Hiện sau khi chọn vùng chỉ có thông báo "Đã chọn ..."; Task 11 nối vào `run_action`.
- Chưa kiểm được bằng máy: phím PrintScreen thật (Lightshot và Snipping Tool trên máy giữ phím), lớp phủ khi không giành được tiêu điểm, mọi phần hiển thị.
- Sau khi chạy thử `shotme.exe`, luôn kết thúc mọi tiến trình `shotme`, nếu không lần build sau không ghi đè được file exe.
- Việc để sau (ngoài bản đầu): xóa trắng vùng nhớ chứa khóa sau khi ký (cần crate `zeroize`).
- Sửa nhỏ sau rà soát: gọi tiếp chính agent đã làm bằng SendMessage (còn nguyên ngữ cảnh), không tạo agent mới.
- Task 2, lựa chọn của agent đã được rà soát chấp nhận: tay nắm nhận trên cả dải cạnh; `release` bỏ khung khi một chiều nhỏ hơn 3; `place_toolbar` kẹp cả hai chiều. Task 8 (lớp phủ) cần biết: khung nhỏ hơn hai lần bán kính tay nắm thì không còn vùng "Inside" để di chuyển.
- Cargo báo cảnh báo "unused dependency shotme-core" ở `shotme-windows`; sẽ hết khi Task 7 dùng core. Không cần sửa.
- Chủ dự án có thể điều khiển phiên từ claude.ai bằng `/remote-control`.
