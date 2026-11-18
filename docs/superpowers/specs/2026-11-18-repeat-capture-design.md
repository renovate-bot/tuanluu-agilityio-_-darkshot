# Thiết kế: Lặp lại lần chụp trước (Giai đoạn 3 sản phẩm, đợt 10 nội bộ)

Ngày: 2026-11-18. Người quyết: phiên điều phối (chủ dự án giao quyền và yêu cầu làm lần lượt các ứng viên Giai đoạn 3, mỗi đợt một tính năng; quyết định 113).

## 1. Mục đích

Người chụp lặp lại cùng một vùng (một bảng số liệu, một khung trình duyệt, một vùng ứng dụng đang theo dõi) phải bấm phím, chọn lại vùng, bấm hành động mỗi lần. `keep_selection` (đợt 2) đã đặt sẵn vùng cũ khi mở lớp phủ, nhưng vẫn phải mở lớp phủ và bấm hành động, và vùng mất khi khởi động lại. Tính năng này cho **một phím: chụp lại đúng vùng trước, làm lại đúng hành động trước**, không mở lớp phủ.

## 2. Phạm vi

Làm:
1. Dòng phím tắt thứ tư "Chụp lại vùng trước" ở tab Phím tắt, mặc định TẮT, tổ hợp mặc định Ctrl+Shift+PrintScreen.
2. Mục menu khay "Chụp lại vùng trước" ngay sau "Chụp"; mờ (vô hiệu) khi chưa có lần chụp nào để lặp.
3. "Lần chụp trước" = vùng (tọa độ MÀN HÌNH) và hành động (Lưu, Chép, Tải lên, Ghim) của lần chụp vùng gần nhất kết thúc bằng một hành động (không phải Hủy). Chụp toàn màn hình không đổi lần chụp trước.
4. Lặp lại: chụp màn hình (theo ô "kèm con trỏ"), cắt đúng vùng, chạy đúng hành động như khi bấm trong lớp phủ (thông báo, Lịch sử, lưu dự phòng, mở Tùy chọn khi thiếu R2, Ghim ở đúng chỗ). Không lặp lại hình đánh dấu.
5. Nhớ qua lần khởi động: tệp `%APPDATA%\darkshot\last_capture.json` (phiên bản 1: `{"version":1,"x":…,"y":…,"w":…,"h":…,"action":"save|copy|upload|pin"}`), ghi nguyên tử (tệp tạm + `sync_all` + đổi tên). Tệp hỏng hoặc phiên bản lạ → coi như chưa có, không báo lỗi, không xóa. Không ghi cấu hình chính (tránh đụng khóa R2 đã mã hóa).
6. Vùng nhớ cũng được dùng cho `keep_selection` sau khi khởi động lại (cùng một nguồn: `LAST_SELECTION` khởi tạo từ tệp).
7. Vùng không còn hợp lệ (màn hình đổi: vùng không giao màn hình nào, hoặc bị cắt dưới `MIN_SIZE` sau khi kẹp vào màn hình ảo) → mở lớp phủ chọn vùng như phím chụp vùng, kèm thông báo Info "Vùng trước không còn trên màn hình. Hãy chọn lại." Chưa có lần chụp trước → mở lớp phủ như phím chụp vùng (không thông báo).
8. Vùng một phần ra ngoài màn hình ảo nhưng còn ≥ `MIN_SIZE` → cắt phần còn trong ảnh (như `crop` hiện có). Không có phần đen.

Không làm (quyết định 114): mẫu chụp đặt tên/quản lý nhiều mẫu, chụp cỡ cố định, lặp lại theo hẹn giờ, lặp lại hình đánh dấu, nhớ lần chụp toàn màn hình.

## 3. Hành vi chi tiết

- Phím lặp đi qua cổng chụp hiện có (`gate`/`request`) như các phím khác: không chụp khi lớp phủ hay Tùy chọn đang mở; menu đang mở thì hoãn như phím khác (Task 72).
- Kiểu chụp mới `CaptureKind::RepeatLast`. `kind_of(HotkeyRole::RepeatLast) = RepeatLast`.
- Ghim lặp lại: ảnh ghim đặt tại góc vùng (như Ghim từ lớp phủ). Nếu đã đủ 32 ảnh ghim → thông báo giới hạn như thường.
- Tải lên lặp lại khi chưa cấu hình R2 → giống Tải lên từ lớp phủ (mở Tùy chọn, lưu dự phòng như hiện tại).
- Trợ giúp: thêm một câu về phím lặp lại và mục menu khay.

## 4. Kiến trúc

- `darkshot-core`:
  - `config.rs`: `HotkeyRole::RepeatLast` (thêm vào `ALL` cuối cùng), `Hotkeys.repeat_last`, khóa tệp `hotkey_repeat_last` (mặc định `enabled:false`, Ctrl+Shift+PrintScreen), `first_conflict` bao gồm dòng mới.
  - `last_capture.rs` (mới, thuần): `LastCapture { rect: Rect, action: Action }`, `to_json`, `from_json -> Option<LastCapture>` (phiên bản 1, `w,h ≥ MIN_SIZE`, action không phải Cancel), `repeat_rect(last, origin, img_w, img_h, monitors_screen) -> Option<Rect>` (tọa độ ảnh; dùng lại logic `restore_selection`), `remember(prev, action, rect) -> Option<LastCapture>` (Hủy giữ nguyên `prev`).
  - `i18n.rs`: `LabelHotkeyRepeatLast` ("Chụp lại vùng trước" / "Repeat last capture"), mục menu khay `MenuRepeatLast` (cùng chữ), `RepeatRegionGoneTitle` ("Không lặp lại được" / "Couldn't repeat"), `RepeatRegionGoneBody` ("Vùng trước không còn trên màn hình. Hãy chọn lại." / "The previous region is no longer on screen. Select it again."), câu Trợ giúp.
- `darkshot-windows`:
  - `paths.rs`: `last_capture_path()` (trong thư mục cấu hình, theo `DARKSHOT_PROFILE_DIR`).
  - `main.rs`: đọc tệp lúc khởi động → `LAST_CAPTURE` (`thread_local`) và `LAST_SELECTION`; sau mỗi lần chụp vùng kết thúc bằng hành động → cập nhật và ghi tệp (đồng bộ, nhỏ; lỗi ghi bị nuốt); `capture_flow` nhánh `RepeatLast`.
  - `hotkey.rs`: đăng ký dòng thứ tư như ba dòng cũ.
  - `tray.rs`: `MenuCommand::RepeatLast`, `menu_layout(has_update, pins, can_repeat)`.
  - `settings.rs`: dòng thứ tư ở tab Phím tắt (mẫu "Dòng phím tắt mới" trong `memory-bank/systemPatterns.md`), không cắt chữ ở hai ngôn ngữ.

## 5. Riêng tư và an toàn

Tệp chỉ chứa tọa độ và tên hành động; không ảnh, không đường dẫn. Chế độ cách ly (`DARKSHOT_PROFILE_DIR`) dời tệp theo. Không mạng mới.

## 6. Tiêu chí nghiệm thu

1. Chụp vùng + Lưu, rồi bấm phím lặp: tệp PNG mới cùng cỡ vùng, không mở lớp phủ; Lịch sử có mục mới.
2. Khởi động lại ứng dụng, phím lặp vẫn chụp đúng vùng; `keep_selection` bật thì lớp phủ mở với vùng đó.
3. Vùng nằm trên màn hình đã rút → lớp phủ mở kèm thông báo.
4. Tab Phím tắt có dòng thứ tư, không cắt chữ (Việt/English, DPI 100–200%), xung đột tổ hợp được báo như các dòng khác.
5. Menu khay có mục "Chụp lại vùng trước", mờ khi chưa có lần chụp.
6. Màn hình tọa độ âm: vùng lặp đúng chỗ (trừ gốc ảnh đúng một lần).
7. `scripts/check.ps1` in `ALL CHECKS PASSED`; không hồi quy đợt 1–9.
