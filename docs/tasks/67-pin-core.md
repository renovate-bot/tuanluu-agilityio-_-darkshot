# Task 67: Ghim ảnh — lõi: hình học, mức phóng, độ mờ, `Action::Pin`, chuỗi i18n

**Agent:** `builder` · **Model:** sonnet (hàm thuần có đặc tả rõ, kiểm thử đơn vị) · **Nhánh:** `task/67-pin-core` (tách từ `feature/pin-screenshot`) · **Rà soát:** `reviewer` (sonnet) · **Phụ thuộc:** 66 (thứ tự gộp)

Thiết kế: `docs/superpowers/specs/2026-11-18-pin-screenshot-design.md` mục 4.1–4.6, 5. Global Constraints: mục cùng tên của `docs/superpowers/plans/2026-11-18-pin-screenshot.md`.

## Cần đọc
`crates/darkshot-core/src/geometry.rs` (`Rect`, `Point`), `actions.rs` (`Action`, `run_action`, các `match` trên `Action`), `i18n.rs` (macro bảng `Msg`, cách thêm mục, `help_text`, kiểm thử của bảng), `lib.rs`. Tìm mọi `match` trên `Action` trong cả workspace (`grep -rn "Action::Cancel" crates/`) để biết chỗ phải thêm nhánh.

## Files
Tạo `crates/darkshot-core/src/pin.rs`; sửa `crates/darkshot-core/src/lib.rs`, `actions.rs`, `i18n.rs`. Sửa tối thiểu ở `crates/darkshot-windows/src/*.rs` CHỈ để workspace còn biên dịch sau khi thêm `Action::Pin` (thêm nhánh `Action::Pin` tương đương `Cancel`/không làm gì; Task 69 nối thật). Không làm giao diện.

## Interfaces (hợp đồng với Task 68–70)
```rust
// darkshot_core::pin
pub const ZOOM_STEPS: [u16; 16] = [10, 25, 33, 50, 67, 75, 90, 100, 110, 125, 150, 175, 200, 250, 300, 400];
pub const MAX_PINS: usize = 32;
pub const OPACITY_MIN: u8 = 20;   // phần trăm
pub const OPACITY_STEP: u8 = 10;
pub const MAX_SIDE: i32 = 16384;  // cỡ hiển thị tối đa mỗi chiều (px)
pub const MIN_VISIBLE: i32 = 32;  // phần tối thiểu còn trong một màn hình (px)
pub const FIT_PERCENT: i32 = 80;  // ảnh từ Lịch sử chiếm tối đa 80% vùng làm việc

/// Cỡ hiển thị của ảnh w×h ở mức `zoom` (%), làm tròn gần nhất, mỗi chiều ≥ 1.
pub fn display_size(w: u32, h: u32, zoom: u16) -> (i32, i32);
/// Bậc kế tiếp lớn hơn/nhỏ hơn `zoom` trong ZOOM_STEPS mà cỡ hiển thị mỗi chiều ≤ MAX_SIDE;
/// `None` khi đã ở đầu/cuối hoặc bậc kế vượt MAX_SIDE. `zoom` không nằm trong bảng thì lấy bậc kề.
pub fn zoom_in(w: u32, h: u32, zoom: u16) -> Option<u16>;
pub fn zoom_out(zoom: u16) -> Option<u16>;
/// Khung ẢNH mới (không gồm viền) khi đổi mức từ `old` sang `new`, giữ điểm `anchor`
/// (tọa độ màn hình, thường là chuột) đứng yên trên cùng điểm ảnh. Tính bằng i64, kẹp vào i32.
pub fn zoom_about(image_rect: Rect, anchor: Point, img_w: u32, img_h: u32, new_zoom: u16) -> Rect;
/// Bậc lớn nhất của ZOOM_STEPS, ≤ 100, để ảnh vừa FIT_PERCENT% của `area` mỗi chiều; ảnh đã vừa thì 100;
/// không bậc nào vừa thì bậc nhỏ nhất.
pub fn fit_zoom(img_w: u32, img_h: u32, area: Rect) -> u16;
/// Khung ảnh căn giữa `area` ở mức `zoom`.
pub fn centered(img_w: u32, img_h: u32, zoom: u16, area: Rect) -> Rect;
/// Dời `r` (khung CỬA SỔ) ít nhất có thể sao cho ≥ min(MIN_VISIBLE, cạnh của r) px mỗi chiều
/// nằm trong ít nhất một màn hình; ưu tiên màn hình giao nhiều nhất, không giao thì màn hình gần nhất.
/// `monitors` rỗng thì trả nguyên `r`.
pub fn keep_visible(r: Rect, monitors: &[Rect]) -> Rect;
/// Độ mờ kế tiếp (phần trăm) khi cuộn lên (`up`, đục hơn) hoặc xuống; kẹp [OPACITY_MIN, 100].
pub fn opacity_step(cur: u8, up: bool) -> u8;
/// Phần trăm → alpha 0..=255 cho SetLayeredWindowAttributes (làm tròn).
pub fn opacity_alpha(percent: u8) -> u8;
/// Có được tạo thêm ảnh ghim khi đang có `count`.
pub fn can_add(count: usize) -> bool;
```
`actions::Action::Pin` (thêm sau `Copy`). `run_action(ctx, Action::Pin, img)` không lưu, không chép, không thông báo, trả `Outcome::Cancelled`. Các hàm khác có `match` trên `Action` (ví dụ `title_for`, `remember_after` ở lớp Windows) phải có nhánh hợp lý: `remember_after` coi `Pin` như `Save` (nhớ vùng).

## Chuỗi i18n (Việt / English) — thêm vào bảng `Msg`
| Msg | Việt | English |
|---|---|---|
| `OverlayPin` | Ghim lên màn hình (Ctrl+P) | Pin to screen (Ctrl+P) |
| `PinWindowTitle` | darkshot — Ảnh ghim | darkshot — Pinned image |
| `PinMenuCopy` | Sao chép ảnh\tCtrl+C | Copy image\tCtrl+C |
| `PinMenuSave` | Lưu ảnh\tCtrl+S | Save image\tCtrl+S |
| `PinMenuZoom` | Thu phóng | Zoom |
| `PinMenuOpacity` | Độ mờ | Opacity |
| `PinMenuClose` | Đóng\tEsc | Close\tEsc |
| `PinMenuCloseAll` | Đóng mọi ảnh ghim | Close all pinned images |
| `PinClose` | Đóng | Close |
| `HistoryPin` | Ghim | Pin |
| `PinLimitTitle` | Đã đủ ảnh ghim | Too many pinned images |
| `PinFailedTitle` | Không ghim được ảnh | Couldn't pin the image |
Hàm có tham số: `pin_limit_body(lang, max) -> String` ("Đã đủ {max} ảnh ghim. Đóng bớt rồi thử lại." / "You already have {max} pinned images. Close some and try again."), `tray_close_pins(lang, n) -> String` ("Đóng mọi ảnh ghim ({n})" / "Close all pinned images ({n})"), `pin_opacity_badge(lang, p) -> String` ("Độ mờ {p}%" / "Opacity {p}%"), `pin_failed_body(lang, reason) -> String` (theo mẫu `reason_try_again` sẵn có nếu hợp). Nhãn mức phóng "125%" không cần dịch (tạo bằng `format!` ở lớp Windows là chấp nhận được vì không có chữ).
`help_text`: thêm một đoạn ngắn về Ghim (Ctrl+P trong lớp phủ, nút Ghim trong Lịch sử, kéo để dời, cuộn để phóng, Ctrl+cuộn để làm mờ, chuột phải để có menu, Esc để đóng), đặt sau đoạn về lớp phủ. Giữ đúng các kiểm thử cũ của `help_text`.

## Kiểm thử (viết trước, commit `test:` với thân hàm `todo!()`)
- `display_size`: 100% giữ nguyên; 50% của 3×3 → (2,2) (làm tròn gần nhất, 1,5 → 2); 10% của 5×5 → (1,1) (≥ 1).
- `zoom_in/zoom_out`: 100→110, 100→90; 400 → None; 10 → None khi out; 105 (ngoài bảng) → in 110, out 100; ảnh 8000×100: in từ 200 → None (250% = 20000 > MAX_SIDE).
- `zoom_about`: anchor ở tâm ảnh 100×100 tại (0,0), 100%→200% → khung (−50,−50,200,200); anchor ở góc trên trái → góc giữ nguyên; tọa độ âm (ảnh tại (−1920, 100)); số lớn không tràn (kẹp).
- `fit_zoom`: ảnh 800×600 trong vùng 1920×1040 → 100; ảnh 3840×2160 trong 1920×1040 → 33 (80% = 1536×832 → 3840×0.33=1267 ≤ 1536, 2160×0.33=713 ≤ 832; 50% → 1080 > 832); ảnh 100000×10 → 10.
- `centered`: tâm khớp tâm vùng (làm tròn xuống), vùng có tọa độ âm.
- `keep_visible`: khung trong màn hình giữ nguyên; khung ra hẳn bên phải → dời để còn đúng 32 px; khung nhỏ hơn 32 px → cần toàn bộ khung; hai màn hình (một ở x âm): chọn màn hình giao nhiều nhất; không giao màn hình nào → màn hình gần nhất; `monitors` rỗng → nguyên.
- `opacity_step`: 100 lên → 100; 100 xuống → 90; 20 xuống → 20; 25 xuống → 20; `opacity_alpha(100)=255`, `(20)=51`, `(60)=153`.
- `can_add(31)=true`, `can_add(32)=false`.
- `run_action(Pin)` → `Outcome::Cancelled`, không gọi clipboard/notifier, không tạo tệp.
- i18n: mọi mục mới có hai bản khác nhau (theo kiểm thử bảng sẵn có); `tray_close_pins(Vi, 3)` chứa "(3)"; `help_text` cả hai ngôn ngữ chứa "Ctrl+P".

## Tiêu chí nghiệm thu
Mọi kiểm thử trên qua; không đổi hành vi hành động khác; `scripts/check.ps1` in `ALL CHECKS PASSED`.

## Review focus
Tràn số i32/i64 ở `zoom_about`, `display_size`; làm tròn nhất quán; `keep_visible` đúng với màn hình tọa độ âm; chuỗi Việt tự nhiên, không dấu "..." ASCII ở nhãn (dùng "…" nếu cần).

Báo cáo theo mẫu `.claude/agents/builder.md`.
