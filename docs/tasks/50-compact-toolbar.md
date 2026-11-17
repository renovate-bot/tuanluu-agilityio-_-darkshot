# Task 50: Thanh công cụ gọn – nút Hình, Hoàn tác/Làm lại sang thanh ngang

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/50-compact-toolbar` · **Rà soát:** `specialist`

Thiết kế: `docs/superpowers/specs/2026-11-17-compact-toolbar-design.md`. Đọc hết file này, nó ngắn. Task dựa trên đợt 6 (Task 44–49), tất cả đã ở `main`. Quyết định 84.

## Global Constraints

Dùng mục cùng tên của `docs/superpowers/plans/2026-11-17-annotation-pro.md`, tóm tắt như sau:

- **Phụ thuộc:** không thêm crate, không bật feature mới của crate `windows`.
- **Chữ hiển thị:** mọi chữ người dùng thấy lấy từ `darkshot_core::i18n`, đủ hai ngôn ngữ. Kiểm thử dùng `t(Lang::Vi, …)`.
- **Màu:** lấy từ `darkshot_core::theme`.
- **Hành vi cũ:** khi không chọn công cụ nào, lớp phủ chạy như cũ.
- **Hiệu năng:** chỉ làm mới vùng bị ảnh hưởng.
- **An toàn:**
  - Không `unwrap`/`expect` hay chỉ số có thể panic.
  - Mỗi khối `unsafe` có `// SAFETY:`.
  - Giải phóng mọi đối tượng GDI/GDI+. Kiểm thử chống rò phải qua.
- **Không sửa:** spec và kế hoạch đã duyệt, gói task cũ, `prototype/`, các dòng cũ của `decisions.md`.
- **Quy trình:**
  - Commit kiểm thử trước (`test:`), rồi `feat:`, rồi `docs:`.
  - `git add` từng file, `git commit` là lệnh riêng.
  - Chạy `cargo fmt --all`. Check phải in `ALL CHECKS PASSED`.
  - Không chuyển nhánh, không đẩy.
  - Build release với `--target-dir target/task50`.
  - Không tắt tiến trình `darkshot` mà mình không khởi động.
  - Không đọc khóa R2 hay cấu hình thật.
  - Kiểm thử `#[ignore]` chạy từng cái với `--test-threads=1`.

## Cần đọc

`crates/darkshot-windows/src/overlay.rs`, các phần sau:
- **Thanh dọc:** `SideItem`, `SIDE_ITEMS`, `SIDE_SEPS`, `side_index`, `side_layout_for`, `side_item_rect`/`side_item_at`, `bars_for`, `flyout_rect`, `size_flyout_rect`, `palette_rect_for`.
- **Thanh ngang:** `ACTIONS` và các chỗ chia đều thanh ngang (khoảng dòng 513, 1047, 1077, 3883, 3899).
- **Trạng thái:** `State` (`sizes_open`, `size_hover`, `size_pressed`, `hover`, `pressed`), `LAST_COLOR`/`LAST_SIZE`.
- **Xử lý:** `activate`, `escape`, `on_key`, `cursor_at`.
- **Vẽ:** `Painter::side_toolbar`, `size_flyout`, `Gp::side_icon`.
- **Kiểm thử:** `side_items_order`, `size_flyout_geometry`, các `live::*` chạm thanh dọc hoặc Hoàn tác/Làm lại, `overlay_round_trip_without_gdi_leaks`.

`crates/darkshot-core/src/i18n.rs`: `Msg::Tool*`.

## Files

- **Sửa:**
  - `crates/darkshot-windows/src/overlay.rs`
  - `crates/darkshot-core/src/i18n.rs` (thêm `Msg::ToolShapes`)
- **Tài liệu:**
  - `README.md`: mục Đánh dấu ảnh.
  - `docs/testing/manual-checklist.md`: thêm mục "Thanh gọn (Task 50)".
  - `memory-bank/decisions.md`: thêm dòng 84.
  - `memory-bank/systemPatterns.md`: dòng về thanh dọc.

## Quyết định cho task này

### Thanh dọc

- `SIDE_ITEMS` có 9 mục: `Tool(Select)`, `Tool(Pen)`, `Shapes`, `Tool(Marker)`, `Tool(Blur)`, `Tool(Text)`, `Tool(Step)`, `Size`, `Color`.
- `SIDE_SEPS = [1, 7]`.
- `SideItem` thêm biến thể `Shapes` và bỏ `Undo`/`Redo`.
- Gợi ý của `Shapes` là `Msg::ToolShapes`: ("Hình (L, A, R, E)", "Shapes (L, A, R, E)").

### Hình nhớ

- Hằng `SHAPE_TOOLS: [Tool; 4] = [Line, Arrow, Rect, Ellipse]`.
- Hình nhớ lưu trong `thread_local LAST_SHAPE: Cell<Tool>`, mặc định `Arrow`, và trường `State.shape_tool`.
- Mọi đường bật một công cụ trong `SHAPE_TOOLS` đều cập nhật hình nhớ: phím L, A, R, E và ô của bảng Hình.

### Nút Hình

- Biểu tượng là `side_icon` của hình nhớ, cộng một tam giác đặc nhỏ ở góc dưới phải ô. Tam giác cạnh khoảng 4–5 trên lưới 24, cùng màu biểu tượng.
- Vạch "đang chọn" bật khi `tool ∈ SHAPE_TOOLS`.
- `activate(Shapes)`:
  - Nếu `tool ∉ SHAPE_TOOLS`: đặt `tool = shape_tool`, đóng mọi bảng, không mở bảng Hình.
  - Ngược lại: bật/tắt bảng Hình. Bảng màu và bảng Cỡ đóng nếu đang mở.

### Bảng Hình

- Trạng thái `shapes_open`, `shape_hover: Option<Tool>`, `shape_pressed: Option<Tool>`.
- Hình học: hàm `shapes_flyout_rect(side, btn, sel, mon, dpi)` giống `size_flyout_rect` (4 ô `BUTTON` ngang, lề `GAP`), canh theo nút Hình.
- Vẽ:
  - Nền `SURFACE`, viền `OVERLAY_BORDER`.
  - Mỗi ô vẽ `side_icon` của công cụ.
  - Ô của công cụ hiện hành có viền 2 px `ACCENT`.
  - Ô đang rê chuột dùng nền `OVERLAY_HOVER` và hiện gợi ý `Msg::ToolLine` / `ToolArrow` / `ToolRect` / `ToolEllipse`.
- Bấm rồi nhả trên cùng một ô: bật công cụ đó, nhớ nó, đóng bảng.
- Bấm ngoài bảng: chỉ đóng bảng, cú bấm bị nuốt như bảng Cỡ.
- Esc: thêm bảng Hình vào bậc 1 cùng bảng Cỡ và bảng màu.
- Ba bảng không bao giờ mở cùng lúc. Viết một chỗ đóng chung, để ba bảng không lặp code.

### Thanh ngang

- Thứ tự: `Undo`, `Redo`, vạch ngăn, `Upload`, `Save`, `Copy`, `Cancel`.
- Tách hàm thuần:
  - `bar_layout(dpi) -> (size, cells)`, hoặc hình dạng tương đương.
  - `bar_item_at(bar, p, dpi) -> Option<BarItem>`, với `enum BarItem { Undo, Redo, Act(Action) }`.
  - Vạch ngăn rộng `SIDE_SEP`.
- Bỏ phép chia đều `p.x - toolbar.x` cho `ACTIONS.len()`.
- `bars_for` lấy chiều rộng thanh ngang từ bố cục mới.
- Hoàn tác và Làm lại giữ nguyên:
  - Biểu tượng (vẽ bằng `side_icon` như trước).
  - Trạng thái mờ (`!can_undo()` và không ô chữ; `!can_redo()`).
  - Gợi ý `Msg::ToolUndo` / `ToolRedo`.
  - Hành vi khi bấm, kể cả bỏ chọn hình.
- `Picked.action` vẫn chỉ là `Action`. Undo/Redo không bao giờ đóng lớp phủ.
- Phím Enter, Ctrl+C, Ctrl+S và các phím hành động khác không đổi.

### Không đổi

Phím tắt, mọi thứ khác của đợt 6, ảnh ra.

## Các bước

- [ ] **1. Kiểm thử** (commit `test: compact toolbar`)
  - Kiểm thử đơn vị (sửa theo bố cục mới):
    - `side_items_order`: 9 mục, đúng thứ tự, `SIDE_SEPS == [1, 7]`, gợi ý đúng.
    - `side_bar_fits_low_screen` nếu có.
    - `bar_layout_order`: 7 ô theo thứ tự trên. Các ô không chồng nhau, nằm trong thanh. `bar_item_at` đúng ở tâm từng ô và trả `None` ở vạch ngăn.
    - `shapes_flyout_geometry`: giống `size_flyout_geometry`. Bảng nằm trong màn hình, không đè thanh dọc, ở các DPI 96, 144, 192.
    - Chiều cao một cột của thanh dọc ở DPI 96 ≤ 9·36 + 8·6 + 2·9 DIP (tính theo cách `side_layout` thực sự cộng).
  - `live::shapes_button` (`#[ignore]`), các bước nối tiếp nhau:
    1. Chọn vùng, bấm nút Hình. Kết quả: công cụ là Mũi tên (mặc định) và không có bảng nào mở.
    2. Bấm nút Hình lần nữa: bảng Hình mở.
    3. Bấm ô Hình elip: công cụ là Ellipse, bảng đóng. Kéo để vẽ.
    4. Bấm `P`, rồi bấm nút Hình: công cụ là Ellipse (hình nhớ).
    5. Bấm `L`, rồi `P`, rồi nút Hình: công cụ là Line.
    6. Bấm Lưu: ảnh có hình elip.
  - `live::shapes_flyout_esc_and_outside` (`#[ignore]`):
    - Bảng Hình mở, bấm Esc: bảng đóng, lớp phủ vẫn mở.
    - Mở lại bảng, bấm ngoài bảng: bảng đóng, vùng chọn không đổi, không có hình mới.
    - Mở bảng Cỡ rồi bấm nút Hình hai lần: chỉ bảng Hình mở.
  - `live::undo_redo_on_action_bar` (`#[ignore]`):
    1. Vẽ 2 nét.
    2. Bấm nút Hoàn tác trên thanh ngang: còn 1 hình.
    3. Bấm nút Làm lại: lại 2 hình.
    4. Bấm Hoàn tác 2 lần: nút Hoàn tác mờ, bấm thêm không làm gì.
    5. Bấm Lưu: `Picked.shapes.len() == 0` và `action == Save`.
  - Sửa các `live::*` cũ đang bấm nút Hoàn tác/Làm lại hoặc nút công cụ Line/Arrow/Rect/Ellipse trên thanh dọc, cho chúng dùng vị trí mới. Không đổi khẳng định.
  - Thêm vào vòng chống rò GDI: mở và đóng bảng Hình, bấm Hoàn tác và Làm lại trên thanh ngang.
- [ ] **2. Mã** theo các quyết định trên. Commit `feat: compact toolbar`.
- [ ] **3. Tài liệu** (commit `docs: compact toolbar`)
  - README: mục thanh dọc 9 mục, nút Hình, Hoàn tác/Làm lại trên thanh ngang.
  - Checklist: mục "Thanh gọn (Task 50)" với các dòng `[ ]`.
  - Thêm dòng 84 vào `decisions.md`: thanh dọc 9 mục; nút Hình nhớ hình gần nhất, mặc định Mũi tên; Hoàn tác/Làm lại sang thanh ngang; lý do là thanh 14 mục quá dài.
  - Sửa dòng thanh dọc trong `systemPatterns.md`.
- [ ] **4.** Chạy `cargo fmt --all` rồi check, phải in `ALL CHECKS PASSED`. Chạy từng kiểm thử `#[ignore]` mới hoặc đã sửa, và kiểm thử chống rò, với `--test-threads=1`.

## Xong khi

Check qua và các kiểm thử chạy tay mới hoặc đã sửa đều qua.

Báo cáo gồm:
- Các commit.
- Dòng cuối của check.
- Kết quả từng kiểm thử chạy tay.
- Chiều cao thanh dọc ở DPI 96 trước và sau, tính bằng số.
- Ảnh chụp theo cách của Task 47: thêm tạm `Step::Snap`, ghi ra `C:/Users/Client/AppData/Local/Temp/claude/9bda5e44761d6d15/b97deff2-6606-44cc-bdfe-d1822c907752/scratchpad/t50_*.png`, không commit phần tạm. Cần ba ảnh: thanh dọc 9 mục với thanh ngang 7 ô; bảng Hình đang mở; nút Hình khi hình nhớ là Ellipse.
- Những gì chưa nhìn bằng mắt.
- Chỗ khác gói.
