# Bối cảnh hiện tại

Cập nhật: 2026-11-16

## Đang ở đâu

- Giai đoạn: 5. Xây dựng.
- Task vừa xong: Task 8 (chụp màn hình và lớp phủ), đã gộp vào `main`. Lõi 57 kiểm thử; lớp Windows 10 kiểm thử và 1 kiểm thử chạy tay (`overlay_round_trip_without_gdi_leaks`, `-- --ignored`). Rà soát hai lượt (lượt hai chỉ rà phần sửa).
- Task 9 (DPAPI, clipboard, khởi động cùng Windows) đã gộp vào `main`; lớp Windows 16 kiểm thử, 3 kiểm thử chạy tay (2 clipboard, 1 lớp phủ).
- Task 10 (cửa sổ Cài đặt) đã gộp vào `main`; lớp Windows 31 kiểm thử, 4 kiểm thử chạy tay. Cửa sổ mới chỉ được mở qua kiểm thử chạy tay, chưa ai nhìn.
- Task 11 (nối toàn bộ) đã gộp vào `main`; lớp Windows 41 kiểm thử. Ứng dụng đã đủ ba tính năng theo mã; chưa ai thử bằng tay, chưa thử với R2 thật. Rà soát Task 11 kết luận CẦN SỬA (không lỗi chặn), bốn điểm đã sửa nhưng CHƯA được rà soát lại: rà soát toàn bộ cuối cùng phải xem kỹ `gate_step` (trạng thái `PendingMenuSettings`, `CapturingSettings`, hành động `FocusSettings`) và `settings::focus_existing`.
- Task 12 (README, mục nghiệm thu R2) đã gộp vào `main`. Cả 12 task đã xong. Bản phát hành `target/release/shotme.exe` 1.513.472 byte.
- Rà soát toàn bộ (specialist) đã xong: DUYỆT, không lỗi chặn; phần bảo mật (hai đích mạng, đường đi của khóa, tên ngẫu nhiên) đã xác nhận. Sáu điểm nên sửa và bốn điểm nhỏ thành gói `docs/tasks/13-hardening.md`.
- Task 13 (gia cố) đã gộp vào `main`: lõi 64 kiểm thử, lớp Windows 49 kiểm thử và 4 kiểm thử chạy tay. Tổng kết token đã lập trong `progress.md`.
- Task 14 (biểu tượng mới, tám cỡ, sinh bằng `crates/shotme-windows/res/make_icon.py`, cần Pillow; ảnh xem trước `docs/icon-preview.png`) đã gộp vào `main`. Phần mã (khay nạp cỡ nhỏ bằng `LoadImageW`, `WM_SETICON` cho Cài đặt) chưa qua agent rà soát. Chủ dự án đã duyệt hình (2026-11-16).
- Task đang làm: không. Nghiệm thu bản đầu với R2 thật vẫn chờ chủ dự án, nhưng KHÔNG chặn đợt 1 (quyết định 32).
- **Đợt 1** (menu mới, Tùy chọn dạng tab, Thông tin, Trợ giúp, hai ngôn ngữ): thiết kế `docs/superpowers/specs/2026-11-16-options-tabs-i18n-design.md` đã duyệt; kế hoạch `docs/superpowers/plans/2026-11-16-options-tabs-i18n.md`; gói task `docs/tasks/15-...` đến `19-...`. Task 15 đã gộp vào `main` (rà soát DUYỆT; lõi 79 kiểm thử): bảng `i18n` có 22 mục `Msg` và 7 hàm có tham số; lớp Windows đang truyền cứng `Lang::Vi`, Task 16 thay bằng ngôn ngữ hiện hành. `normalize_public_base_url("https://")` nay trả `UrlError::MissingHost` (trước là lỗi thiếu https). Agent Task 15 đưa mã thật vào commit `test:` (chỉ `i18n.rs` là `todo!()`): lời giao các task sau nhắc lại yêu cầu này. Task 16 đã gộp vào `main` (rà soát DUYỆT, năm điểm nhỏ đã sửa; lớp Windows 55 kiểm thử): mọi chữ của lớp Windows lấy từ bảng; `lang.rs` có `current`, `set`, `tr`, `tr_code`, `lang_from_langid`, `os_default`; `app::ConfigNotice` thay ba hằng `NOTICE_*`; `UploadJob` có trường `lang` (chốt lúc tạo lượt); crate `windows` bật thêm feature `Win32_Globalization`; `scripts/check.ps1` có bước `no hard-coded Vietnamese strings` (dừng quét ở `#[cfg(test)]` đứng trước `mod`; không bắt chuỗi English viết cứng, chuỗi nhiều dòng). Kiểm thử mới ở lớp Windows không được khẳng định chữ lấy từ `lang::tr` (biến toàn cục, `tr_follows_current_language` tạm đặt En): truyền `Lang` tường minh. Chưa ai nhìn giao diện English bằng mắt. Task 17 đã gộp vào `main` (rà soát `specialist` DUYỆT, ba điểm đã sửa; lớp Windows 64 kiểm thử): cửa sổ Tùy chọn ba tab (`SysTabControl32`, ba khung con, khung ẩn thì vừa ẩn vừa `EnableWindow(false)`, dải tab ở đáy thứ tự Z; `frame_proc` chuyển tiếp `WM_COMMAND`/`WM_CTLCOLORSTATIC`); `settings::{Tab, Field, FormError, OpenReason, KeyRoute, tab_of, initial_tab, key_route, show_form_error}`; `show(parent, current, notice, reason)`; `main::{merge_reason, lang_after_save}`; lý do mở giữ trong `thread_local`; crate `windows` bật thêm `Win32_UI_Controls`; chữ "OK" là hằng `i18n::OK_LABEL` ngoài `Msg` (bảng đòi hai bản khác nhau). Sửa kèm lỗi có từ bản đầu: `TEST_RUNNING` kẹt sau khi Hủy lúc đang Kiểm tra kết nối. Hàng nút trống từ x = 12 đến 300 DIP cho nút Trợ giúp. Chưa ai nhìn cửa sổ thật (nền tab, thứ tự Tab, chữ English bị cắt): chờ thử tay sau Task 18. **Task 18 đang làm** trên nhánh `task/18-menu-about-help` (agent `builder`). Đợt 2 (chụp) và đợt 3 (định dạng) chưa có thiết kế: brainstorm riêng sau khi đợt 1 xong. Proxy không làm (quyết định 28).
- Agent không được tự tắt tiến trình `shotme` đang chạy sẵn (có thể chủ dự án đang dùng): chỉ tắt tiến trình do chính nó khởi động; nếu exe bị khóa thì bỏ qua build release và báo lại.
- shotme ghi `shotme.exe`=1 (DWORD) vào `HKCU\Software\Microsoft\Windows\Windows Error Reporting\ExcludedApplications` mỗi lần khởi động (đã có trên máy chủ dự án từ lượt chạy thử Task 13). Chưa thử bằng sự cố thật.
- Biết mà để lại: thoát đúng lúc một lượt đang tự lưu dự phòng có thể để lại một file PNG 0 byte (ảnh không mất, bản đủ nằm ở tên `_2`). Việc còn lại sau đó: sửa lỗi họ báo (nhánh `task/13-...`, agent theo độ khó), rà soát toàn bộ bằng `specialist`, tổng kết token theo task/model/giai đoạn trong `progress.md`. Chủ dự án bảo "tiếp tục" mà chưa báo kết quả thử tay Task 7 và 8.

## Bước kế tiếp

Chạy `/next-task` nối tiếp từ Task 15, không hỏi lại chủ dự án giữa các task (quyết định 18). Dừng sau Task 18 để chủ dự án thử tay theo các mục "Đợt 1" của checklist và đọc lời văn Trợ giúp; Task 19 chạy sau khi họ báo kết quả. Ngoài ra chỉ dừng khi có việc chỉ chủ dự án quyết được.

## Đang chờ chủ dự án

- Thử `target/release/shotme.exe` theo `docs/testing/manual-checklist.md` (mọi mục), rồi làm theo `README.md` mục "Chuẩn bị Cloudflare R2" và mục "Nghiệm thu với R2 thật" của checklist. Họ tự nhập khóa vào ứng dụng; phiên điều phối và agent không nhận, đọc hay ghi khóa. Cần họ xác nhận thêm: quyền "Object Read & Write" có cho xóa file thử của "Kiểm tra kết nối" không. Lỗi họ báo: giao agent `specialist` sửa trên nhánh mới `task/08b-...` trước khi sang Task 9.

## Điểm dừng sắp tới cần chủ dự án

- Sau Task 8: thử lớp phủ chọn vùng theo mục "Lớp phủ" của `docs/testing/manual-checklist.md`.
- Sau Task 12: nghiệm thu với kho R2 thật.
- Sau Task 18: thử tay đợt 1 (hai mục "Đợt 1" của `docs/testing/manual-checklist.md`). Lỗi họ báo: nhánh `task/18b-...`, agent theo độ khó.

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
- Task 9: `secret::DpapiStore`, `clipboard::WinClipboard` (ảnh là `CF_DIB` 24 bit, không kèm "PNG"; mỗi lần chép tạo một cửa sổ message-only tạm nên gọi được từ luồng nền), `autostart::{is_enabled, set_enabled}`. Ba module đang `#[allow(dead_code)]`, Task 10 và 11 nối vào rồi bỏ. `is_enabled` chỉ kiểm giá trị tồn tại: Task 11 ghi lại đường dẫn lúc khởi động nếu đang bật.
- Task 10: `settings::show(parent, &Config, notice) -> Option<SettingsResult>`; `None` là Hủy hoặc đã có cửa sổ. `show` tự hủy và đăng ký lại phím tắt cho `parent` (khi trả `Some`, phím mới ĐÃ được đăng ký; không gọi `hotkey::register` lần nữa). `show` không ghi đĩa, không gọi DPAPI, không đổi autostart. Phím thường phải kèm Ctrl, Alt hoặc Win. Để sau bản đầu: `WM_DPICHANGED` của cửa sổ Cài đặt, zeroize khóa.
- Chưa kiểm được bằng máy: phím PrintScreen thật (Lightshot và Snipping Tool trên máy giữ phím), lớp phủ khi không giành được tiêu điểm, mọi phần hiển thị.
- Sau khi chạy thử `shotme.exe`, luôn kết thúc mọi tiến trình `shotme`, nếu không lần build sau không ghi đè được file exe.
- Việc để sau (ngoài bản đầu): xóa trắng vùng nhớ chứa khóa sau khi ký (cần crate `zeroize`).
- Sửa nhỏ sau rà soát: gọi tiếp chính agent đã làm bằng SendMessage (còn nguyên ngữ cảnh), không tạo agent mới.
- Task 2, lựa chọn của agent đã được rà soát chấp nhận: tay nắm nhận trên cả dải cạnh; `release` bỏ khung khi một chiều nhỏ hơn 3; `place_toolbar` kẹp cả hai chiều. Task 8 (lớp phủ) cần biết: khung nhỏ hơn hai lần bán kính tay nắm thì không còn vùng "Inside" để di chuyển.
- Cargo báo cảnh báo "unused dependency shotme-core" ở `shotme-windows`; sẽ hết khi Task 7 dùng core. Không cần sửa.
- Chủ dự án có thể điều khiển phiên từ claude.ai bằng `/remote-control`.
