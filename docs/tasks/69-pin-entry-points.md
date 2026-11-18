# Task 69: Ghim ảnh — nút Ghim trong lớp phủ, Ctrl+P, luồng chụp, menu khay, dọn khi thoát

**Agent:** `specialist` · **Model:** opus (lớp phủ 9.000 dòng, luồng chụp và vòng đời thoát; hồi quy rủi ro cao) · **Nhánh:** `task/69-pin-entry-points` (tách từ `feature/pin-screenshot`) · **Rà soát:** `reviewer` (fable) · **Phụ thuộc:** 68

Thiết kế: `docs/superpowers/specs/2026-11-18-pin-screenshot-design.md` mục 4.1, 4.5, 4.7, 5, 6. Global Constraints: mục cùng tên của `docs/superpowers/plans/2026-11-18-pin-screenshot.md`. Task 67 (lõi) và 68 (`darkshot-windows/src/pin.rs`) đã ở nhánh phát triển.

## Cần đọc
`crates/darkshot-windows/src/pin.rs` (API công khai), `overlay.rs`: `BarItem`, `BAR_CELLS`, `bar_layout`, `bar_item_at`, cách vẽ biểu tượng thanh ngang (tìm nơi vẽ biểu tượng Chép/Lưu), chú thích (`tip`), `finish`, nhánh phím Ctrl (khoảng dòng 3300) và nhánh phím khi đang gõ chữ (khoảng dòng 2559), kiểm thử `bar_layout_order` và các kiểm thử bố cục thanh ngang/`place_toolbar`; `main.rs`: `capture_flow`, `remember_after`, `run_on_image`, `handle_event` (Quit, `WM_ENDSESSION`), cách mở menu khay; `tray.rs`: `MenuCommand`, `menu_layout`, chỗ dựng menu và kiểm thử của nó; `lang.rs`; `crates/darkshot-core/src/geometry.rs` (`place_toolbar` nếu cỡ thanh ngang ảnh hưởng vị trí).

## Files
`crates/darkshot-windows/src/overlay.rs`, `main.rs`, `tray.rs`. (Biểu tượng vẽ bằng mã, không thêm tệp tài nguyên.)

## Yêu cầu
1. Thanh ngang: `Hoàn tác, Làm lại | Tải lên, Lưu, Chép, Ghim, Hủy` (`BAR_CELLS` 8 ô). Cập nhật kiểm thử `bar_layout_order` theo đúng thứ tự mới (đây là thay đổi theo thiết kế, không phải nới kiểm thử) và mọi kiểm thử cỡ thanh ngang suy ra từ số ô. Chú thích `Msg::OverlayPin`.
2. Biểu tượng đinh ghim: cùng hệ nét, cỡ, màu với các biểu tượng thanh ngang hiện có (đầu đinh hình thang/tròn + thân + mũi kim chéo 45°), rõ ở DPI 100/150/200%. Trạng thái rê/nhấn như nút khác.
3. Phím: Ctrl+P (0x50) khi có vùng chọn → `finish(Action::Pin)`, ở cả hai đường phím (bình thường và đang gõ chữ — đang gõ chữ thì chốt ô trước như Ctrl+S/Ctrl+C đang làm). Không có vùng chọn → như các phím hành động khác hiện nay.
4. `main::capture_flow`: `Action::Pin` → ảnh = `picked_image` (có hình đánh dấu, như Lưu) → `pin::open(image, PinPlace::At(góc trên trái vùng theo TỌA ĐỘ MÀN HÌNH))` (dùng `capture::to_screen_coords(rect_đã_kẹp, shot.origin)`; vùng bị kẹp vào ảnh thì góc là góc sau khi kẹp, khớp gốc ảnh cắt của `picked_image`). Lỗi: `Limit` → thông báo Info `PinLimitTitle` + `pin_limit_body`; `Create(e)` → Error `PinFailedTitle` + `pin_failed_body`. Không gọi `run_on_image`, không ghi Lịch sử. `remember_after` nhớ vùng như Lưu. Toàn màn hình (`SaveFull`/`UploadFull`) không đổi.
5. `pin::set_action_sink` đăng ký lúc khởi động một hàm gọi `run_on_image(hwnd_khay, action, image, notifier)` (Lưu/Chép từ ảnh ghim đi đúng đường thường: thông báo, Lịch sử).
6. Menu khay: `menu_layout(has_update, pins: usize)`; `pins > 0` → thêm `MenuCommand::ClosePins` ngay sau `History`, chữ `tray_close_pins(lang, pins)`; chọn → `pin::close_all()`. Kiểm thử `menu_layout` cho 0 và 3 ảnh ghim, có/không có bản mới.
7. Thoát (`MenuCommand::Quit`, `WM_ENDSESSION`, đường thoát khác nếu có): `pin::close_all()` trước khi thoát vòng lặp / trước khi lưu dự phòng; không treo.
8. Không đổi gì khác của lớp phủ: Lưu/Chép/Tải lên/Hủy, Enter, phím công cụ, bắt cửa sổ.

## Kiểm thử (viết trước)
- `bar_layout_order` mới; `bar_item_at` trúng ô Ghim; cỡ thanh ngang = 7 nút + 1 vạch ngăn.
- Phím: dựng `State` trong kiểm thử (mẫu `snap_state`, `dpis` tường minh) có vùng chọn, Ctrl+P → kết quả `Picked { action: Pin, .. }` (hoặc trạng thái kết thúc tương đương có sẵn trong kiểm thử).
- `remember_after(prev, Action::Pin, r) == Some(r)`.
- Hàm thuần mới ở `main` nếu có (ví dụ `pin_origin(rect, origin) -> Point`) cho gốc âm (màn hình trái) và vùng bị kẹp.
- `menu_layout` như mục 6.
- Chạy tay (`#[ignore]`): mở lớp phủ thật với ảnh giả, chọn vùng, gửi Ctrl+P → một ảnh ghim tồn tại với góc ảnh đúng vị trí; dọn.

## Tiêu chí nghiệm thu
Kiểm thử qua; `scripts/check.ps1` in `ALL CHECKS PASSED`; bản release build được (`--target-dir target/task69`); không hồi quy các hành động cũ.

## Review focus
Tọa độ màn hình/ảnh (trừ `origin` đúng một lần), vùng bị kẹp; thoát khi còn ảnh ghim và khi lớp phủ đang mở; Ctrl+P khi đang gõ chữ/IME; hình đánh dấu có trong ảnh ghim; menu khay đúng ngôn ngữ và số đếm.

TDD: commit `test:` rồi `feat:`. Báo cáo theo mẫu `.claude/agents/specialist.md`.
