# Task 13: Gia cố sau rà soát toàn bộ

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/13-hardening`

Gói task tự chứa. Nguồn: lượt rà soát toàn bộ ngày 2026-11-16 (kết luận DUYỆT, không lỗi chặn). Các mục dưới đây là những điểm "nên sửa trước khi dùng thật" và vài điểm nhỏ.

## Cần đọc

- `crates/shotme-core/CLAUDE.md`, `crates/shotme-windows/CLAUDE.md`, `memory-bank/systemPatterns.md`
- Spec `docs/superpowers/specs/2026-11-16-shotme-design.md` mục 2, 5.3, 5.6, 6
- Các file nêu ở từng mục

## Global Constraints

Như các gói task trước (xem đầu `docs/tasks/11-integration.md`). Nhắc lại: chỉ hai đích mạng; khóa bí mật không lọt ra đâu; core không phụ thuộc hệ điều hành; release dùng `panic = "abort"` nên không `unwrap`/`expect`/chỉ số có thể panic trên đường chạy thật; viết kiểm thử trước.

## Nội dung

Mỗi mục: kiểm thử thất bại trước (commit `test:`), rồi sửa (commit `fix:`). Mục nào không tách được logic thuần để kiểm thử thì nêu trong báo cáo.

### A. Không làm mất ảnh khi thoát lúc đang tải lên (spec mục 2)

`crates/shotme-windows/src/app.rs`, `main.rs`. Giữ một sổ các lượt tải lên đang chạy (ảnh `Arc<Image>` và thư mục lưu của lượt đó); lượt nào kết thúc (thành công hoặc đã tự lưu dự phòng) thì rút khỏi sổ. Khi chọn Thoát, hoặc nhận `WM_QUERYENDSESSION`/`WM_ENDSESSION` (tắt máy, đăng xuất), lưu PNG của mọi lượt còn trong sổ vào thư mục lưu trước khi thoát. Một ảnh có thể vừa được lưu vừa lên kho xong: chấp nhận. Phần sổ là logic thuần, có kiểm thử: thêm, rút, rút hai lần, `drain` trả đúng các lượt còn lại; `Mutex` poison không panic.

### B. Thông báo không bị mất khi biểu tượng khay chưa có (spec mục 5.6)

`tray.rs`. Khi chưa thêm được biểu tượng, xếp thông báo vào hàng đợi (tối đa 8, đầy thì bỏ cái cũ nhất); khi `try_add` thành công thì hiện cái mới nhất mức Error nếu có, không thì cái mới nhất. Logic hàng đợi là hàm thuần có kiểm thử. `readd` (TaskbarCreated): nếu `NIM_ADD` thất bại thì thử `NIM_MODIFY` trước khi coi là chưa thêm.

### C. Cửa sổ Cài đặt không bị khuất

`settings.rs`, `main.rs`. Thêm `WS_EX_APPWINDOW` để cửa sổ Cài đặt có nút trên thanh tác vụ. Trong `gate_step`: `(SettingsOpen, Hotkey)` và `(SettingsOpen, MenuCapture)` → `FocusSettings` (trước là `Ignore`). Cập nhật kiểm thử cổng.

### D. Ảnh và link chép vào clipboard không bị Windows đưa vào lịch sử hay đồng bộ đám mây

`clipboard.rs`. Trong cùng phiên `OpenClipboard`, đặt thêm các định dạng loại trừ của Windows (tra tài liệu Microsoft "Cloud Clipboard and Clipboard History Formats" để lấy đúng tên và kiểu dữ liệu; rà soát nhớ là `ExcludeClipboardContentFromMonitorProcessing`, `CanIncludeInClipboardHistory` = DWORD 0, `CanUploadToCloudClipboard` = DWORD 0, nhưng CHƯA đối chiếu). Áp dụng cho cả ảnh lẫn link. Đặt các định dạng phụ thất bại thì không làm hỏng việc chép chính. Ghi trong báo cáo đường dẫn tài liệu đã dùng. Cập nhật kiểm thử `#[ignore]` của clipboard để kiểm các định dạng này có mặt, chạy tay một lần.

### E. Báo cáo lỗi của Windows (WER)

`main.rs` (hoặc module nhỏ mới). Lúc khởi động, loại `shotme.exe` khỏi WER để không có bản kết xuất bộ nhớ nào được gửi đi khi ứng dụng sự cố: tra tài liệu Microsoft và chọn cách phù hợp cho một tiến trình không có quyền quản trị (`WerAddExcludedApplication` với `bAllUsers = FALSE`, và/hoặc `SetErrorMode`/`WerSetFlags`). Lỗi của lời gọi này bị bỏ qua. Nêu trong báo cáo cách đã chọn, lý do, và điều nó không che được.

### F. Lệch giờ máy không bị báo thành "khóa sai"

`crates/shotme-core/src/upload.rs`. Khi S3 trả 403, đọc thân phản hồi (giới hạn 4 KB); nếu chứa mã `RequestTimeTooSkewed` thì trả biến thể lỗi mới `UploadError::ClockSkew`, `message_vi()` = "Giờ máy không đúng. Hãy chỉnh lại ngày giờ của Windows rồi thử lại." Kiểm thử bằng máy chủ giả sẵn có: 403 với thân có mã đó → `ClockSkew`; 403 thân khác → lỗi quyền như cũ. Thân phản hồi không được đưa vào thông báo.

### G. Điểm nhỏ

1. `crates/shotme-core/src/actions.rs` (lưu PNG): hai lần lưu trùng giây không được ghi đè nhau. Chiếm tên đích bằng `OpenOptions::create_new`, gặp `AlreadyExists` thì thử hậu tố kế tiếp; vẫn ghi qua file tạm rồi đổi tên. Kiểm thử: hai lần lưu cùng `LocalTime` cho hai file khác nhau, cả hai còn nguyên.
2. `actions.rs`: `needs_settings` đúng khi lỗi là `UploadError::NotConfigured`, dù `uploader` là `Some`. Kiểm thử.
3. `crates/shotme-core/src/config.rs` `normalize_public_base_url`: từ chối `?`, `#`, `@`, và tên máy chủ rỗng. Kiểm thử.
4. `main.rs`: bấm biểu tượng khay khi menu đang mở không được hạ cờ `MENU_OPEN` sớm (bỏ qua `WM_TRAY` khi `MENU_OPEN`).

### H. Tài liệu

`README.md`: mục "Bảo mật" thêm (đúng theo những gì đã làm ở D và E) rằng ảnh/link đã chép được đánh dấu không đưa vào lịch sử và đồng bộ clipboard của Windows; rằng tên miền vẫn được hỏi qua máy chủ DNS của hệ điều hành; rằng thoát lúc đang tải lên thì ảnh được lưu vào máy. `docs/testing/manual-checklist.md`: sửa dòng nói về thoát khi đang tải lên; thêm dòng thử Win+V (lịch sử clipboard) không có ảnh/link của shotme; thêm dòng cửa sổ Cài đặt có nút trên thanh tác vụ.

**Xong khi:** `powershell -File scripts/check.ps1` in `ALL CHECKS PASSED`; mỗi mục A đến G có kiểm thử hoặc lý do không kiểm thử được trong báo cáo.
