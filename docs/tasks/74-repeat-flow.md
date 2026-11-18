# Task 74: Lặp lại lần chụp trước — luồng chụp, tệp trạng thái, phím, menu khay

**Agent:** `specialist` · **Model:** opus (luồng chụp, tọa độ màn hình/ảnh, DPI và tọa độ âm, đăng ký phím, khay) · **Nhánh:** `task/74-repeat-flow` · **Rà soát:** `reviewer` (fable) · **Phụ thuộc:** 73

Thiết kế: `docs/superpowers/specs/2026-11-18-repeat-capture-design.md` (toàn bộ). Global Constraints: kế hoạch `docs/superpowers/plans/2026-11-18-repeat-capture.md`.

## Cần đọc
`crates/darkshot-core/src/last_capture.rs`; `crates/darkshot-windows/src/main.rs`: `CaptureKind`, `kind_of`, `next_kind`, `remember_after`, `LAST_SELECTION`, `capture_flow` (nhánh Region/SaveFull/UploadFull, `picked_image`, `pin_origin`, `run_on_image`, `history_window::open_pending_pin`), `request`/`gate`, WM_HOTKEY, khởi động (`main`), `AppEvent`/`handle_event` cho menu khay; `hotkey.rs` (`register_enabled`, `id_of`, `role_of`); `tray.rs` (`MenuCommand`, `menu_layout`, `show_menu`, kiểm thử); `paths.rs` (`config_path_in`, `profile_override`); `capture.rs` (`to_screen_coords`, `Screenshot.monitors`).

## Files
`crates/darkshot-windows/src/main.rs`, `hotkey.rs`, `tray.rs`, `paths.rs` (và `lang.rs` nếu cần). Không sửa `settings.rs` ngoài mức tối thiểu để biên dịch (Task 75 làm giao diện).

## Yêu cầu
1. `paths::last_capture_path()` = thư mục chứa `config.json` + `last_capture.json` (theo `DARKSHOT_PROFILE_DIR`).
2. Khởi động: đọc tệp (`from_json`), đặt `LAST_CAPTURE` (`thread_local Cell<Option<LastCapture>>`) và `LAST_SELECTION = last.rect`. Lỗi đọc → None, im lặng.
3. Sau chụp vùng kết thúc bằng hành động (Save/Copy/Upload/Pin; Hủy thì không): `remember` → nếu đổi thì ghi tệp nguyên tử (tệp tạm cùng thư mục, `sync_all`, `rename`; tạo thư mục nếu thiếu). Lỗi ghi bị nuốt. Việc nhớ phải xảy ra trước khi chạy hành động (để tải lên lâu vẫn nhớ).
4. `CaptureKind::RepeatLast`: `kind_of(HotkeyRole::RepeatLast)`. Trong `capture_flow`: không có `LAST_CAPTURE` → chạy như `Region` (mở lớp phủ). Có → chụp (`include_cursor` theo cấu hình; không liệt kê cửa sổ), `repeat_rect(last, shot.origin, w, h, màn_hình_tọa_độ_màn_hình)`; None → thông báo Info `RepeatRegionGoneTitle`/`RepeatRegionGoneBody` rồi chạy như `Region` trên CÙNG ảnh chụp (không chụp lần hai); Some(r) → `picked_image(&shot.image, r, &[])` rồi: `Pin` → `pin::open(At(pin_origin(r, origin)))` + thông báo lỗi như Ghim thường; hành động khác → `run_on_image`. Lần chụp lặp lại KHÔNG đổi `LAST_CAPTURE`.
5. Phím: `hotkey::register_enabled` đăng ký dòng thứ tư; WM_HOTKEY dùng `role_of` như cũ (đi qua `request`).
6. Menu khay: `MenuCommand::RepeatLast` ngay sau `Capture`, chữ `MenuRepeatLast`, vô hiệu (`MF_GRAYED`) khi `LAST_CAPTURE` là None; chọn → đi qua cổng chụp như mục "Chụp" (hoãn 250 ms để menu đóng) với kiểu `RepeatLast`. `menu_layout(has_update, pins, can_repeat)` hoặc cách tương đương có kiểm thử.
7. Không đổi hành vi các loại chụp khác.

## Kiểm thử (viết trước)
- Hàm thuần mới (ví dụ `repeat_plan(last, shot_geom) -> RepeatPlan::{Overlay, OverlayWithNotice, Run(Rect, Action)}`) cho: chưa có, vùng mất, vùng hợp lệ, gốc âm.
- Ghi/đọc tệp trong thư mục tạm: khứ hồi; tệp hỏng → None; thư mục không có → tạo; ghi không để lại tệp tạm.
- `kind_of(RepeatLast)`; `menu_layout` có mục lặp, vô hiệu khi không có.
- Chạy tay (`#[ignore]`, không cần chuột): đặt `LAST_CAPTURE` giả với hành động Save vào thư mục tạm, gọi đường lặp → có tệp PNG đúng cỡ vùng.

TDD: `test:` rồi `feat:`. `cargo fmt --all`; `powershell -File scripts/check.ps1` → `ALL CHECKS PASSED`; build release `--target-dir target/task74`.

## Review focus
Tọa độ (trừ `origin` một lần), `restore`/`keep_selection` không đổi hành vi khi không có tệp; tệp ghi nguyên tử, không ghi khi Hủy; phím thứ tư đăng ký/hủy đúng; menu khay hoãn đúng; không panic.
