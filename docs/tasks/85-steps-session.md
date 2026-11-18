# Task 85: Chụp từng bước — lưu phiên, thanh phiên, lớp phủ, khay

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/85-steps-session` · **Rà soát:** `reviewer` (fable) · **Phụ thuộc:** 84

Thiết kế: `docs/superpowers/specs/2026-11-18-step-capture-design.md` mục 2.1–2.5, 2.8–2.10, 3, 4. Global Constraints: kế hoạch `docs/superpowers/plans/2026-11-18-step-capture.md`.

## Cần đọc
`crates/darkshot-core/src/steps.rs`; `crates/darkshot-windows/src/`: `history_service.rs` (luồng ghi có hàng đợi + `flush` khi thoát — làm mẫu), `paths.rs`, `pin.rs` (cửa sổ topmost toolwindow, kéo, DPI — mẫu cho thanh phiên), `overlay.rs` (`BAR_CELLS`, `bar_layout`, `bar_item_at`, `stroke_icon`, phím Ctrl ở `on_key`/`text_key`, cách `pick_region` nhận tham số), `main.rs` (`capture_flow_with`, `request`, `run_on_image`, khay `menu_layout`/`show_menu`, thoát), `tray.rs`, `theme.rs` (nút tự vẽ).

## Files
Tạo `crates/darkshot-windows/src/steps_store.rs`, `steps_bar.rs`; sửa `overlay.rs`, `main.rs`, `tray.rs`, `paths.rs`.

## Yêu cầu
1. `paths::steps_dir()` = `<LOCALAPPDATA>\darkshot\steps\current`.
2. `steps_store`: trạng thái phiên trên luồng giao diện (`StepSession` + cờ `dirty_since_export`); `start()`, `active() -> bool`, `count()`, `add(image) -> Result<(), Full>` (đặt tên `next_file_name`, thêm vào danh sách, ghi `session.json` SAU KHI PNG đã ghi xong — luồng `darkshot-steps` có hàng đợi mã hóa PNG rồi báo về bằng `PostMessage`; khi báo xong mới thêm bước vào danh sách và ghi json nguyên tử; lỗi ghi PNG → thông báo Error, không thêm bước), `restore()` lúc khởi động (thiết kế 2.5), `end()` (xóa thư mục), `flush(3 s)` khi thoát, API cho Task 86: `steps()`, `image_path(i)`, `remove(i)`, `move_up(i)`, `move_down(i)`, `set_caption(i, s)`, `mark_exported()`, `dirty()`; mỗi thay đổi ghi `session.json` nguyên tử. Đăng ký người nghe thay đổi (hàm `fn()`) để thanh phiên và cửa sổ Xem lại cập nhật.
3. `steps_bar`: thiết kế 2.2; `SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE)` (kiểm hằng có trong `windows` 0.62; lỗi → ẩn/hiện quanh lúc chụp trong `capture_flow_with`); nút Chụp bước → `request(GateEvent::MenuCapture? hoặc tương đương không hoãn menu, kind Region)`; Xem lại → `steps_window::open()` (Task 86 — tạm thời một hàm `fn()` đăng ký từ `main`, mặc định không làm gì); Kết thúc → nếu `dirty()` và có bước thì mở Xem lại ở chế độ xác nhận (Task 86; tạm gọi hàm đăng ký), không thì `end()`. Không giành tiêu điểm khi hiện (`SW_SHOWNA`); DPI theo màn hình; i18n; `relabel()` khi đổi ngôn ngữ.
4. Lớp phủ: `pick_region` nhận thêm cờ `steps_active` (hoặc qua trạng thái) → thanh ngang thêm ô `Act(Action::AddStep)` trước Hủy (10 ô); không có phiên → 9 ô như cũ. Biểu tượng dấu cộng trong ô vuông (`stroke_icon`). Ctrl+N (0x4E) ở cả hai đường phím, chỉ khi có phiên và có vùng chọn. Hàm thuần `bar_cells(steps: bool)` + kiểm thử; `bar_layout_order` cũ giữ cho trường hợp không phiên.
5. `main`: `Action::AddStep` → `picked_image` → `steps_store::add`; Full → thông báo Info `StepsFullTitle/Body`. Khay: không phiên → mục "Chụp từng bước" (sau "Lịch sử"); có phiên → "Xem lại các bước (N)". Thoát: `steps_store::flush`, giữ thư mục; đóng thanh phiên.
6. Khởi động có phiên khôi phục → hiện thanh phiên.

## Kiểm thử (viết trước)
`bar_cells` hai trường hợp; Ctrl+N trong `snap_state` (có phiên/không); khay `menu_layout` có/không phiên; `steps_store` trên thư mục tạm: thêm 3 bước (ảnh nhỏ) → tệp 001–003 + json đúng; khôi phục; json hỏng → thư mục đổi tên `current.corrupt-*`, ảnh còn; PNG thiếu → bỏ bước; xóa/di chuyển cập nhật json; `end` xóa thư mục; ghi json không để tệp tạm. Chạy tay (không chuột): mở thanh phiên, kiểm `GetWindowDisplayAffinity`, đóng không rò GDI.

TDD `test:` rồi `feat:`. `cargo fmt --all`; `powershell -File scripts/check.ps1` → `ALL CHECKS PASSED`; build release `--target-dir target/task85`.

## Review focus
Thứ tự ghi PNG/json, khôi phục và tệp hỏng; luồng nền và thoát; lớp phủ không phiên y như cũ; thanh phiên không vào ảnh chụp, không giành tiêu điểm.
