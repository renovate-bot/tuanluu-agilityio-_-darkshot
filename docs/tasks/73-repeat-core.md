# Task 73: Lặp lại lần chụp trước — lõi

**Agent:** `builder` · **Model:** sonnet (mô hình thuần, cấu hình, i18n; kiểm thử đơn vị) · **Nhánh:** `task/73-repeat-core` (tách từ `feature/repeat-capture`) · **Rà soát:** `reviewer` (sonnet)

Thiết kế: `docs/superpowers/specs/2026-11-18-repeat-capture-design.md` mục 2–4. Global Constraints: kế hoạch `docs/superpowers/plans/2026-11-18-repeat-capture.md`.

## Cần đọc
`crates/darkshot-core/src/config.rs` (`HotkeyRole`, `ALL`, `label`, `default_slot`, `Hotkeys` get/set/first_conflict, `FileConfig`/`FileSlot` và `hotkey_save_full` để làm theo mẫu, `load`/`save` và kiểm thử của chúng), `geometry.rs` (`Rect`, `MIN_SIZE`, `restore_selection`), `actions.rs` (`Action`), `i18n.rs` (bảng `Msg`, `help_text`, `HelpKeys`), `memory-bank/systemPatterns.md` mục "Dòng phím tắt mới". Tìm mọi `match` trên `HotkeyRole` trong workspace (`grep -rn "HotkeyRole::UploadFull" crates/`).

## Files
Tạo `crates/darkshot-core/src/last_capture.rs`; sửa `lib.rs`, `config.rs`, `i18n.rs`. Sửa tối thiểu ở `crates/darkshot-windows/src/*.rs` CHỈ để biên dịch (nhánh `RepeatLast` trong các `match`: `kind_of` có thể tạm trả `CaptureKind::Region` kèm `// Task 74`; nếu `settings.rs` có mảng cỡ 3 theo `HotkeyRole::ALL` thì làm cho nó chạy với 4 dòng mà không đổi giao diện khác — nếu không làm được gọn thì báo lại, Task 75 làm giao diện).

## Interfaces
```rust
// darkshot_core::last_capture
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LastCapture { pub rect: Rect /* tọa độ MÀN HÌNH */, pub action: Action }
pub const FILE_VERSION: u32 = 1;
pub fn to_json(l: &LastCapture) -> String;
/// None khi JSON hỏng, version khác 1, w/h < MIN_SIZE, action lạ hoặc "cancel".
pub fn from_json(s: &str) -> Option<LastCapture>;
/// Lần chụp trước sau một lần chụp vùng kết thúc bằng `action` với vùng `rect` (tọa độ màn hình): Cancel → giữ `prev`.
pub fn remember(prev: Option<LastCapture>, action: Action, rect: Rect) -> Option<LastCapture>;
/// Vùng cắt (tọa độ ẢNH) để lặp lại; None khi vùng không giao màn hình nào hoặc phần còn trong ảnh < MIN_SIZE.
pub fn repeat_rect(last: &LastCapture, origin: Point, img_w: i32, img_h: i32, monitors_screen: &[Rect]) -> Option<Rect>;
```
Action dạng chữ trong JSON: "save", "copy", "upload", "pin". `config`: `HotkeyRole::RepeatLast` cuối `ALL`, `label` → `Msg::LabelHotkeyRepeatLast`, mặc định `enabled:false`, Ctrl+Shift+PrintScreen (`Hotkey { ctrl: true, shift: true, ..pre }`), khóa tệp `hotkey_repeat_last` theo mẫu `hotkey_save_full`.

## Chuỗi i18n
`LabelHotkeyRepeatLast`, `MenuRepeatLast` ("Chụp lại vùng trước" / "Repeat last capture"), `RepeatRegionGoneTitle` ("Không lặp lại được" / "Couldn't repeat"), `RepeatRegionGoneBody` ("Vùng trước không còn trên màn hình. Hãy chọn lại." / "The previous region is no longer on screen. Select it again."). `help_text`: một câu về phím "Chụp lại vùng trước" (mặc định tắt, bật ở tab Phím tắt) và mục menu khay; `HelpKeys` thêm phím của dòng mới nếu `help_text` liệt kê phím các dòng khác.

## Kiểm thử (viết trước, commit `test:` với `todo!()`)
- JSON khứ hồi cho 4 hành động; tọa độ âm; `from_json` trả None cho: chuỗi rác, version 2, w < MIN_SIZE, "cancel", thiếu trường.
- `remember`: Cancel giữ prev (kể cả None); Save thay prev.
- `repeat_rect`: vùng trong màn hình chính (origin 0) → đúng vùng; gốc ảo âm (màn hình trái −1920) → trừ đúng một lần; vùng trên màn hình đã rút (không giao màn hình nào) → None; vùng tràn mép phải còn ≥ MIN_SIZE → cắt; còn < MIN_SIZE → None.
- Cấu hình: thiếu khóa → mặc định tắt Ctrl+Shift+PrtScn; khứ hồi `save`/`load`; `first_conflict` bắt trùng với dòng mới; tệp cũ (chỉ ba dòng) đọc được.
- i18n: mục mới hai bản khác nhau; `help_text` hai ngôn ngữ chứa nhãn mới.

TDD: `test:` rồi `feat:`. `cargo fmt --all`; `powershell -File scripts/check.ps1` → `ALL CHECKS PASSED`.

## Review focus
Tương thích tệp cấu hình cũ; JSON không panic với đầu vào lạ; tràn số i64 trong `repeat_rect`.
