# Task 72: Ghim ảnh — sửa sau rà soát toàn đợt 9

**Agent:** `specialist` · **Model:** opus (vòng lặp menu modal, phím tắt toàn cục, thứ tự cửa sổ topmost so với lớp phủ) · **Nhánh:** `task/72-pin-review-fixes` (tách từ `feature/pin-screenshot`) · **Rà soát:** `reviewer` (fable) · **Phụ thuộc:** 66–71

Nguồn: rà soát toàn đợt 9 (`specialist`, fable). Global Constraints: mục cùng tên của `docs/superpowers/plans/2026-11-18-pin-screenshot.md`.

## Cần đọc
`crates/darkshot-windows/src/main.rs` (WM_HOTKEY ~1089–1104, `MENU_OPEN`, `request`, `GateEvent::MenuCapture`, `capture_flow`, `pin_failure_notice` ~720–735, `with_app(|a| a.overlay_open)`), `pin.rs` (`MENU_OPEN` ~1027–1046, `badge_text` ~559, `open`, `PinError`, chỗ trả `Create` khi ảnh quá lớn ~220–243), `history_window.rs` (`on_pin_image` ~1839–1851, chỗ bấm Ghim ~1975–1989), `crates/darkshot-core/src/i18n.rs` (`pin_opacity_badge`, `PinClose`, `PinFailedBody`).

## Files
`crates/darkshot-windows/src/main.rs`, `pin.rs`, `history_window.rs`, `crates/darkshot-core/src/i18n.rs`.

## Yêu cầu
1. **Phím chụp khi menu ngữ cảnh của ảnh ghim đang mở** (nên sửa): thêm `pin::menu_open() -> bool`; ở WM_HOTKEY coi `MENU_OPEN || pin::menu_open()` như nhau: `EndMenu` rồi `request(hwnd, GateEvent::MenuCapture, kind)` (đường hoãn sẵn có của menu khay), để lớp phủ không chạy lồng trong vòng lặp modal của menu. Không đổi hành vi menu khay. (Menu ngữ cảnh của Lịch sử có cùng lớp lỗi từ v0.3.0; nếu cùng cờ dễ áp dụng thì làm luôn và ghi báo cáo, không bắt buộc.)
2. **Ảnh ghim từ Lịch sử tới khi lớp phủ đang mở**: `on_pin_image` khi lớp phủ đang mở (`overlay_open`) thì KHÔNG mở ngay; giữ ảnh (một chỗ chờ, ảnh mới thay ảnh cũ) và mở sau khi lớp phủ đóng (gọi từ `capture_flow` sau `pick_region` trả về, mọi nhánh). Thoát ứng dụng khi còn ảnh chờ thì bỏ ảnh.
3. **Kiểm giới hạn trước khi giải mã**: bấm Ghim trong Lịch sử khi `!darkshot_core::pin::can_add(pin::count())` → báo ngay qua `pin_failure_notice(…, &PinError::Limit)` trên luồng giao diện, không tạo luồng nền. Vẫn giữ kiểm trong `pin::open`.
4. **Chữ nhãn độ mờ**: `badge_text` dùng `i18n::pin_opacity_badge(lang, p)`; gỡ `Msg::PinClose` nếu không dùng (kiểm thử bảng i18n vẫn qua).
5. **Ảnh quá lớn**: thêm `PinError::TooLarge` (ảnh vượt `MAX_SIDE` ngay ở mức nhỏ nhất); `pin_failure_notice` → Error `PinFailedTitle` + `Msg::PinTooLargeBody` ("Ảnh quá lớn để ghim." / "The image is too large to pin."). `Create` giữ `PinFailedBody`.

## Kiểm thử (viết trước)
- Hàm thuần quyết định ở WM_HOTKEY (ví dụ `hotkey_defers(tray_menu_open, pin_menu_open) -> bool`) cho bốn tổ hợp.
- Chỗ chờ ảnh ghim: hàm thuần/kiểu nhỏ `PendingPin` (đặt, thay, lấy ra một lần) có kiểm thử.
- `pin_failure_notice` cho `TooLarge`; `pin_opacity_badge` được dùng (kiểm thử `badge_text` nếu có).
- `pin_rejects_invalid_image` (chạy tay) nếu đổi kết quả thì cập nhật đúng theo biến thể mới.

TDD: commit `test:` rồi `fix:`. `cargo fmt --all`; `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.

## Review focus
Không chạy lớp phủ trong vòng lặp modal; ảnh chờ không rò khi thoát; không đổi đường menu khay; chữ i18n.
