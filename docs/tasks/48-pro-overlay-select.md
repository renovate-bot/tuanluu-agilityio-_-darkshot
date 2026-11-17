# Task 48: Lớp phủ – công cụ Chọn (chọn, di chuyển, đổi cỡ, xóa, đổi kiểu)

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/48-pro-overlay-select` · **Rà soát:** `specialist`

Đợt 6. Thiết kế: `docs/superpowers/specs/2026-11-17-annotation-pro-design.md` (đọc mục 4, 5, 7, 8, 11). Kế hoạch: `docs/superpowers/plans/2026-11-17-annotation-pro.md`. Dựa trên Task 44–47 (đã ở `main`).

## Global Constraints

- Không thêm crate. Không bật feature mới của crate `windows`.
- Mọi chữ người dùng thấy lấy từ `darkshot_core::i18n`. Màu giao diện từ `darkshot_core::theme` (khung chọn hình: `ACCENT`).
- Khi không chọn công cụ, lớp phủ chạy đúng như hôm nay. Không hình nào thì ảnh ra đúng từng byte như hôm nay.
- Khung chọn, tay nắm không bao giờ đi qua `draw_shapes`/`burn`.
- Làm mờ: màu che luôn tính trên ảnh chụp gốc (`edit::transform` nhận `self.shot_image()`), cả xem trước và chốt; hoàn tác/làm lại trả nguyên hình đã lưu.
- Một cử chỉ kéo là đúng một mục sổ; hủy (Esc) không ghi; thay bằng hình bằng hệt không ghi (`Annotations::replace` trả None).
- Hiệu năng: mỗi lần chuột di chuyển khi kéo chỉ làm mới hợp hộp bao cũ/mới của hình đang kéo cộng tay nắm; không chép ảnh cỡ màn hình.
- Release `panic = "abort"`: không `unwrap`/`expect`/chỉ số có thể panic (chỉ số chọn có thể lệch: luôn dùng `get`). `unsafe` nhỏ, kèm `// SAFETY:`. Giải phóng mọi đối tượng GDI/GDI+; kiểm thử chống rò phải qua.
- Không sửa spec, kế hoạch đã duyệt, gói task cũ, `prototype/`.
- Quy trình: commit kiểm thử trước (`test:`), rồi `feat:`. `cargo fmt --all`. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`. Kiểm `git branch --show-current`; không chuyển nhánh, không đẩy. Chạy cargo qua PowerShell. Không tắt tiến trình `darkshot` không do mình khởi động; build release với `--target-dir target/task48`. Không đọc hay ghi khóa R2, file cấu hình thật. Kiểm thử `#[ignore]` chạy từng cái với `--test-threads=1`.

## Cần đọc

- `crates/darkshot-core/src/edit.rs` (toàn bộ Interfaces: `Grip`, `Gesture`, `hit_shape`, `topmost_hit`, `handles`, `grip_at`, `frame`, `transform`, `restyle`).
- `crates/darkshot-core/src/annotate.rs`: `Annotations::{replace, remove, undo, redo}`, `Applied`, `PressMode::Pick`, `HIT_TOL_DIP`, `MIN_EXTENT_DIP`.
- `crates/darkshot-windows/src/overlay.rs`: như Task 47 (sau khi gộp), nhất là `on_down`/`on_move`/`on_up`/`on_key`, `lost_capture`, `escape`, hàm áp `Applied` của Task 47, `choose_color`, đặt mức cỡ, `toggle_tool`, `cursor_at`, `cursor_for_hit`, `draw`, `Painter::handle`, kiểm thử `live::*` và chống rò.

## Files

- Sửa: `crates/darkshot-windows/src/overlay.rs`.

## Quyết định cho task này

- `State.selected: Option<usize>`, `State.gesture: Option<Gesture>` cùng `State.preview: Option<Shape>` (hình đang kéo; `draw` vẽ `preview` thay cho `shapes[gesture.index]` ở đúng vị trí trong thứ tự, để thứ tự trên dưới khi xem trước giống ảnh ra).
- **Bấm** (Chọn bật, `PressMode::Pick` hoặc trước `press_mode` cho tay nắm hình): thứ tự thiết kế 5.2. Tay nắm hình đang chọn xét trước tay nắm vùng. `tol = dip_to_px(HIT_TOL_DIP, dpi_at(p))`, bán kính tay nắm = nửa `scaled(HANDLE_SIZE, dpi)` + `HANDLE_HIT`. Trúng hình → chọn, tạo `Gesture{grip: Move}`, bắt chuột. Không trúng → bỏ chọn, không tạo vùng mới.
- **Kéo**: điểm chuột kẹp vào vùng chọn trước khi gọi `transform` (Shift qua `key_held(VK_SHIFT)`; nhấn/nhả Shift khi đang kéo cập nhật như nét vẽ); `min_extent = dip_to_px(MIN_EXTENT_DIP, dpi)`. Hộp bao xem trước: `shape_bounds(&preview)`; với Text khi đổi cỡ được ước lượng (phóng hộp gốc cùng hệ số + 2 px) và đo thật lúc chốt.
- **Thả**: `marks.replace(index, preview)`; `Some(applied)` → hàm áp của Task 47; `None` (không đổi) → chỉ làm mới. Giữ chọn hình. Mất chuột → như thả.
- **Esc**: theo thiết kế 5.8 — bảng màu/Cỡ → ô chữ → đang kéo hình: bỏ `gesture`/`preview`, nhả chuột, làm mới, không ghi → có hình chọn: bỏ chọn → đóng lớp phủ.
- **Delete / Backspace** (không Ctrl, không ô chữ, có hình chọn, không đang kéo): `marks.remove(i)`, áp, bỏ chọn.
- **Đổi màu / mức khi có hình chọn**: ngoài việc đặt mặc định (như Task 47), `marks.replace(i, restyle(shape, level, color, dpi_at(tâm hộp bao)))` và áp.
- **Bỏ chọn khi**: đổi/bỏ công cụ, hoàn tác, làm lại, vùng chọn bắt đầu kéo khung mới, lớp phủ đóng.
- **Vẽ**: sau các hình và trước khung vùng chọn/thanh: `frame` (nếu có) viền 1 px `ACCENT`, tay nắm từ `handles` bằng `Painter::handle` cỡ `scaled(HANDLE_SIZE)`. Hình chọn có thể nằm ngoài phần đang vẽ: chỉ vẽ phần giao `area`. Khung/tay nắm chỉ trên màn hình.
- **Con trỏ** khi Chọn bật: trên tay nắm hình chọn → con trỏ đổi cỡ theo hướng (`End` dùng `IDC_SIZEALL`); trên hình (trúng) → `IDC_SIZEALL`; còn lại → mũi tên; tay nắm vùng như hôm nay.
- **Làm mới**: khi chọn/bỏ chọn làm mới `frame` nới bằng nửa cỡ tay nắm + 1; khi kéo làm mới hợp hộp cũ và mới (nới như trên).

## Các bước

- [ ] **1. Kiểm thử** (commit `test: overlay select tool`). Kiểm thử chạy tay (`#[ignore]`, kiểu `live::*`), mỗi cái kiểm cả `Picked.shapes` hoặc ảnh ra:
  - `live::select_move_one_undo`: vẽ Rect, `V`, kéo hình +40,+20 qua 5 bước Move, thả; Ctrl+Z → hình về vị trí cũ; Ctrl+Y → vị trí mới; Ctrl+Z ×2 → không còn hình (chứng minh kéo là đúng 1 mục).
  - `live::click_select_records_nothing` (Review Focus 3): vẽ 2 nét, `V`, bấm-thả trên nét thứ nhất, Ctrl+Z → còn 1 nét (nét thứ hai bị hoàn tác, không phải mục rỗng).
  - `live::esc_cancels_drag`: `V`, kéo hình giữa chừng, Esc → hình ở vị trí cũ, lớp phủ còn mở, chuột đã nhả; Ctrl+Z → bỏ hình (không có mục kéo).
  - `live::topmost_selected`: hai Blur chồng nhau, `V`, kéo ở chỗ chồng → chỉ Blur vẽ sau di chuyển.
  - `live::delete_and_undo`: 3 hình, chọn hình giữa, Delete → 2 hình; Ctrl+Z → 3 hình, hình giữa đúng vị trí 1 trong `Picked.shapes`.
  - `live::resize_rect_handle`: chọn Rect, kéo tay nắm SE → a/b mới đúng; Shift giữ tỉ lệ.
  - `live::blur_move_save_covered` (Review Focus 2): vẽ Blur, `V`, dời sang vùng khác màu, Lưu → mọi điểm ảnh trong `blur_pixel_rect` mới của ảnh ra bằng `redact_color(ảnh gốc, rect mới)`; không có điểm ảnh màu `ACCENT` trong ảnh ra.
  - `live::restyle_selected`: chọn Pen, chọn mức Huge → width = 8·dpi/96; chọn màu khác → màu đổi; Ctrl+Z ×2 → về như cũ; Blur chọn rồi đổi màu → không đổi.
  - `live::no_tool_behaves_as_before` (Review Focus 4): không công cụ, không hình: kéo ngoài vùng tạo vùng mới; Esc đóng lớp phủ.
  - `live::tool_switch_deselects`: chọn hình rồi `P` → không còn khung (đọc `selected` qua trạng thái thử nếu có, hoặc qua Delete không xóa gì).
  - Thêm vào vòng chống rò GDI: chọn, kéo, đổi cỡ, xóa, hoàn tác.
- [ ] **2. Mã** theo quyết định trên.
- [ ] **3.** `cargo fmt --all`; check → `ALL CHECKS PASSED`; chạy từng kiểm thử `#[ignore]` mới và chống rò với `--test-threads=1`. Build release `--target-dir target/task48`, ghi cỡ exe. Commit `feat: overlay select, move, resize, delete`.

**Xong khi:** check qua và các kiểm thử chạy tay mới qua. Báo cáo: commit, dòng cuối check, kết quả từng kiểm thử chạy tay, cỡ exe release; ảnh chụp lớp phủ (hình được chọn có khung và tay nắm; Ellipse đang đổi cỡ) ghi vào `C:/Users/Client/AppData/Local/Temp/claude/9bda5e44761d6d15/b97deff2-6606-44cc-bdfe-d1822c907752/scratchpad/t48_*.png` theo cách Task 47 (không commit phần tạm); những gì chưa nhìn bằng mắt; chỗ khác gói.
