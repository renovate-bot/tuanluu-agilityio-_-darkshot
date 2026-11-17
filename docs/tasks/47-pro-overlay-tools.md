# Task 47: Lớp phủ – thanh 14 mục, bảng Cỡ, Làm lại, Hình elip, Đánh số

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/47-pro-overlay-tools` · **Rà soát:** `specialist`

Đợt 6. Thiết kế: `docs/superpowers/specs/2026-11-17-annotation-pro-design.md` (đọc mục 3, 4, 5.6, 6, 8, 9, 11). Kế hoạch: `docs/superpowers/plans/2026-11-17-annotation-pro.md`. Dựa trên Task 44–46 (đã ở `main`). Công cụ Chọn (chọn/kéo/xóa hình) là Task 48: ở task này nút Chọn bật/tắt được nhưng bấm trong vùng khi Chọn bật (`PressMode::Pick`) chưa làm gì.

## Global Constraints

- Không thêm crate. Không bật feature mới của crate `windows`.
- Mọi chữ người dùng thấy lấy từ bảng `darkshot_core::i18n`, đủ hai ngôn ngữ. Kiểm thử mới không khẳng định chữ đọc từ `lang::tr`; dùng `t(Lang::Vi, …)`.
- Màu giao diện từ `darkshot_core::theme`.
- Khi không chọn công cụ, lớp phủ chạy đúng như hôm nay. Không hình nào thì ảnh ra đúng từng byte như hôm nay. Mức cỡ mặc định (Vừa) cho đúng độ dày/cỡ chữ hôm nay.
- Hình lưu theo tọa độ ảnh chụp; độ dày, cỡ chữ, bán kính lưu bằng px, đổi từ DIP theo DPI màn hình chứa điểm đầu của hình.
- Một hàm vẽ duy nhất `draw_shapes` cho lớp phủ và ảnh ra.
- Hiệu năng: chỉ làm mới vùng bị ảnh hưởng; không vẽ lại cả màn hình mỗi lần chuột di chuyển.
- Release `panic = "abort"`: không `unwrap`/`expect`/chỉ số có thể panic. `unsafe` nhỏ, kèm `// SAFETY:`. Giải phóng mọi đối tượng GDI/GDI+; kiểm thử chống rò của lớp phủ phải qua.
- Không sửa spec, kế hoạch đã duyệt, gói task cũ, `prototype/`.
- Quy trình: commit kiểm thử trước (`test:`), rồi `feat:`. `cargo fmt --all`. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`. Kiểm `git branch --show-current`; không chuyển nhánh, không đẩy. Chạy cargo qua PowerShell. Không tắt tiến trình `darkshot` không do mình khởi động; build release với `--target-dir target/task47`. Không đọc hay ghi khóa R2, file cấu hình thật. Kiểm thử `#[ignore]` chạy từng cái với `--test-threads=1`.

## Cần đọc

- `crates/darkshot-windows/src/overlay.rs`: `SideItem`, `SIDE_ITEMS`, `COLOR_INDEX`, `seps_before`, `side_bar_size`, `side_item_rect`, `side_item_at`, `bars_for`, `palette_rect_for`, `State` (trường), `begin_stroke`/`extend_stroke`/`end_stroke`, `on_down`/`on_move`/`on_up`/`on_key`, `undo`, `push_mark`, `pop_mark`, `open_text`, `TextBox`, `choose_color`, `open_palette`/`close_palette`, `escape`, `activate`, `cursor_at`, `draw` (phần thanh dọc và bảng màu), `Painter::side_toolbar`, `Gp::side_icon`, `SideLook`, kiểm thử vị trí nút và `live::*`, kiểm thử chống rò GDI.
- `crates/darkshot-core/src/annotate.rs`: `Tool`, `SizeLevel`, `Annotations` (`add`, `remove`, `replace`, `undo`, `redo`, `Applied`, `Change`), `next_step_number`.
- `crates/darkshot-core/src/geometry.rs`: `side_layout`, `SideLayout`, `place_side_toolbar`.
- `crates/darkshot-windows/src/annotate_draw.rs`: `shape_bounds`.

## Files

- Sửa: `crates/darkshot-windows/src/overlay.rs`.

## Quyết định cho task này

- **Thanh dọc** theo thiết kế mục 3: `SIDE_ITEMS` = `Tool(Select)`, 9 công cụ vẽ theo `Tool::ALL[1..]`, `Size`, `Color`, `Undo`, `Redo` (14 mục); vạch ngăn trước chỉ số 1, 10, 12. Mọi hình học thanh dọc (cỡ, ô mục, vạch ngăn, mục dưới điểm, ô nút Màu/Cỡ cho bảng) lấy từ `geometry::side_layout(14, &[1, 10, 12], scaled(BUTTON), scaled(SIDE_SEP), mon.h − 2·scaled(GAP))` — bỏ `seps_before`, `side_bar_size`, `COLOR_INDEX` cố định. `bars_for` dùng cỡ từ bố cục; `place_side_toolbar` không đổi.
- **Biểu tượng mới** (cùng nét và lưới 24 của `side_icon`): Chọn = mũi tên con trỏ; Hình elip = elip rỗng; Đánh số = vòng tròn có số "1" dựng bằng nét (không dùng chữ); Cỡ = chấm tròn đặc đường kính theo mức (Small 6, Medium 9, Large 12, Huge 15 trên lưới 24); Làm lại = Hoàn tác lật ngang.
- **Bảng Cỡ**: ô rộng 4 ô vuông `BUTTON` ngang + lề `GAP`, đặt theo cùng quy tắc `palette_rect_for` nhưng canh theo nút Cỡ; nền `SURFACE`, viền `OVERLAY_BORDER`; ô mức hiện tại viền 2 px `ACCENT`; rê chuột dùng `OVERLAY_HOVER` và gợi ý `Msg::SizeSmall…SizeHuge`. Bấm một ô: đặt mức, đóng bảng. Bảng màu và bảng Cỡ không cùng mở. Bấm ngoài bảng chỉ đóng bảng (như bảng màu). Esc đóng bảng.
- **Mức cỡ**: `State.size: SizeLevel`, nhớ qua các lần mở bằng `thread_local LAST_SIZE` (như `LAST_COLOR`). `[` (`VK_OEM_4`) giảm, `]` (`VK_OEM_6`) tăng, cùng nhánh phím công cụ (không Ctrl/Alt, bỏ lặp). `begin_stroke` nhận mức: độ dày = `dip_to_px(level.stroke_dip()/marker_dip(), dpi)`. Ô chữ mới dùng `level.text_dip()`; đổi mức khi ô chữ mở thì đổi cỡ ô (đo lại, làm mới). (Đổi kiểu hình đang chọn là Task 48.)
- **Hình elip**: như Hình chữ nhật (`begin/extend/end_stroke`, Shift = `constrain_square`, `too_short`), con trỏ chữ thập.
- **Đánh số**: `on_down` trong vùng (`PressMode::Draw`) tạo ngay `Shape::Step { center: p, radius: dip_to_px(level.step_dip(), dpi), n: 0, color }` qua `marks.add` (không bắt chuột, không kéo); con trỏ chữ thập.
- **Sổ thay đổi**: một hàm áp `Applied` duy nhất cập nhật `mark_bounds` theo `Change` (Inserted: chèn `shape_bounds` tại index; Removed: bỏ tại index; Replaced: tính lại tại index), làm mới hộp cũ và mới, làm mới hộp các vị trí `renumbered`, làm mới thanh dọc (trạng thái nút). Mọi chỗ thêm hình (nét, chữ, vòng số) đi qua `marks.add` + hàm áp này.
- **Hoàn tác/Làm lại**: Ctrl+Z giữ quy tắc ô chữ hôm nay; Ctrl+Y và Ctrl+Shift+Z làm lại (ô chữ mở: không làm gì); đang kéo: không làm gì. Nút Làm lại trên thanh dọc mờ khi `!can_redo()`. Nút Hoàn tác mờ khi `!can_undo()` và không có ô chữ.
- `PressMode::Pick` (công cụ Chọn): chưa làm gì (`Effects::default()`); Task 48 thay.

## Các bước

- [ ] **1. Kiểm thử** (commit `test: overlay tools, size and redo`):
  - Đơn vị: `side_items_order` (14 mục, đúng thứ tự, tip đúng `Msg`); `side_bar_fits_low_screen`: màn hình 1920×1080 ở DPI 192 (540 DIP) → thanh hai cột, nằm trọn trong màn hình với vùng chọn ở giữa và ở sát góc trên trái, không giao thanh bốn nút; ở DPI 96 → một cột.
  - Đơn vị: `size_keys`: `[`/`]` đổi mức trong giới hạn; `begin_stroke` Pen ở Large dpi 96 → width 5; Marker Small → 10.
  - Đơn vị: `mark_bounds_follow_history` (Review Focus 1): dựng `State` thử nếu kiểm thử hiện có làm được, không thì tách hàm áp `Applied` thành hàm thuần `apply_bounds(bounds: &mut Vec<Rect>, shapes: &[Shape], a: &Applied) -> Vec<Rect>` (trả vùng cần làm mới) và kiểm: thêm 3, xóa giữa, hoàn tác, làm lại → `bounds[i] == shape_bounds(&shapes[i])` với mọi i sau mỗi bước.
  - `live::ellipse_draw_save` (`#[ignore]`): chọn vùng, `E`, kéo, Lưu → ảnh ra có điểm ảnh màu nét trên mép elip, tâm giữ nguyên.
  - `live::steps_number_and_undo_redo` (`#[ignore]`): `N`, bấm 3 chỗ, Ctrl+Z, Ctrl+Y, bấm thêm 1 chỗ, Lưu → 4 vòng; hình trả về `Picked.shapes` có `n` = 1, 2, 3, 4 theo thứ tự.
  - `live::undo_redo_multi` (`#[ignore]`): vẽ 3 nét, Ctrl+Z ×3, Ctrl+Y ×2, vẽ 1 nét, Ctrl+Y (không làm gì) → `Picked.shapes.len() == 3`.
  - `live::size_flyout` (`#[ignore]`): bấm nút Cỡ, bấm ô Large, vẽ Pen → width = 5·dpi/96; bấm nút Cỡ rồi bấm ngoài → bảng đóng, vùng chọn không đổi.
  - `live::text_ctrl_z_unchanged` (`#[ignore]`): ô chữ có chữ, Ctrl+Z → ô biến mất, hình cũ còn; Ctrl+Y khi ô mở → không đổi.
  - Thêm vào vòng chống rò GDI hiện có: mở bảng Cỡ, vẽ Ellipse, đặt Step.
- [ ] **2. Mã** theo quyết định trên.
- [ ] **3.** `cargo fmt --all`; check → `ALL CHECKS PASSED`; chạy từng kiểm thử `#[ignore]` mới và kiểm thử chống rò với `--test-threads=1`. Commit `feat: overlay ellipse, steps, size and redo`.

**Xong khi:** check qua và các kiểm thử chạy tay mới qua. Báo cáo: commit, dòng cuối check, kết quả từng kiểm thử chạy tay; ảnh chụp lớp phủ (thanh 14 mục, bảng Cỡ mở, elip, ba vòng số) theo cách của Task 41/42 (thêm tạm `Step::Snap`, ghi PNG vào `C:/Users/Client/AppData/Local/Temp/claude/9bda5e44761d6d15/b97deff2-6606-45cc-bdfe-d1822c907752/scratchpad/t47_*.png`, không commit phần tạm); những gì chưa nhìn bằng mắt; chỗ khác gói.
