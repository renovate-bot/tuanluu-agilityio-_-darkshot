# Task 44: Core – hình mới, công cụ mới, mức cỡ, sổ hoàn tác/làm lại

**Agent:** `builder` · **Model:** sonnet · **Nhánh:** `task/44-pro-core-model` · **Rà soát:** `reviewer`

Đợt 6 (Annotation Pro). Thiết kế: `docs/superpowers/specs/2026-11-17-annotation-pro-design.md` (đọc mục 2, 4, 5.6, 6, 11). Kế hoạch: `docs/superpowers/plans/2026-11-17-annotation-pro.md`.

## Global Constraints

- Không thêm crate, kể cả `dev-dependencies`; `Cargo.lock` không thêm gói. `darkshot-core` không phụ thuộc crate `windows` hay API riêng của hệ điều hành.
- Mọi chữ người dùng thấy lấy từ bảng `darkshot_core::i18n`, đủ hai ngôn ngữ; không viết cứng.
- Khi không chọn công cụ, lớp phủ chạy đúng như hôm nay. Không hình nào thì ảnh ra đúng từng byte như hôm nay. Mức cỡ mặc định (Vừa) cho đúng độ dày/cỡ chữ hôm nay.
- Release `panic = "abort"`: mã chạy thật không `unwrap`/`expect`/chỉ số có thể panic.
- Không sửa spec, kế hoạch đã duyệt, gói task cũ, `prototype/`.
- Quy trình: commit kiểm thử trước (`test:`, thân hàm mới là `todo!()`; chỉ thêm đủ mã để biên dịch), rồi commit mã (`feat:`). `cargo fmt --all` trước khi commit. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`. Kiểm `git branch --show-current` trước khi commit; không chuyển nhánh, không đẩy. Chạy cargo qua PowerShell. Không đọc hay ghi khóa R2, file cấu hình thật.

## Cần đọc

- `crates/darkshot-core/src/annotate.rs` (toàn bộ, gồm kiểm thử).
- `crates/darkshot-core/src/i18n.rs`: `Msg::ToolPen`…`Msg::ToolUndo` và bảng kiểm thử hai ngôn ngữ.
- `crates/darkshot-windows/src/overlay.rs`: chỉ những chỗ dùng `Tool`, `Shape`, `marks`, `push_mark`, `pop_mark`, `undo`, `SideItem::tip`, `begin_stroke` (grep).
- `crates/darkshot-windows/src/annotate_draw.rs`: `draw_one`, `shape_bounds` (chỉ các `match` trên `Shape`).

## Files

- Sửa: `crates/darkshot-core/src/annotate.rs`, `crates/darkshot-core/src/i18n.rs`.
- Sửa tối thiểu cho biên dịch: `crates/darkshot-windows/src/overlay.rs`, `crates/darkshot-windows/src/annotate_draw.rs`.

## Interfaces

```rust
// crates/darkshot-core/src/annotate.rs
pub enum Tool { Select, Pen, Line, Arrow, Rect, Ellipse, Marker, Blur, Text, Step }
impl Tool {
    /// Thứ tự trên thanh: Select, Pen, Line, Arrow, Rect, Ellipse, Marker, Blur, Text, Step.
    pub const ALL: [Tool; 10];
    /// Thêm: 0x56 'V' => Select, 0x46 'E' => Ellipse, 0x4E 'N' => Step (giữ P L A R H B T).
    pub fn from_key(vk: u16) -> Option<Tool>;
}

pub enum Shape {
    /* Pen, Line, Arrow, Rect, Marker, Blur, Text như cũ */
    Ellipse { a: Pt, b: Pt, width: f32, color: Rgb },
    /// Vòng đánh số; `n` do `Annotations` đánh lại (1 + số Step đứng trước trong danh sách).
    Step { center: Pt, radius: f32, n: u32, color: Rgb },
}
// Shape::bounds: Ellipse = bounds_of(&[a, b], width / 2.0 + 1.0);
//                Step = bounds_of(&[center], radius + radius * STEP_RING / 2.0 + 1.0).

pub const STEP_RING: f32 = 0.12;          // độ dày viền vòng số / bán kính
pub const MIN_EXTENT_DIP: f32 = 6.0;      // cạnh tối thiểu khi đổi cỡ
pub const HIT_TOL_DIP: f32 = 4.0;         // dung sai trúng hình
pub const TEXT_SIZE_MIN_PX: f32 = 8.0;
pub const TEXT_SIZE_MAX_PX: f32 = 400.0;
pub const STEP_RADIUS_MIN_PX: f32 = 6.0;
pub const STEP_RADIUS_MAX_PX: f32 = 300.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SizeLevel { Small, #[default] Medium, Large, Huge }
impl SizeLevel {
    pub const ALL: [SizeLevel; 4];             // Small, Medium, Large, Huge
    pub fn smaller(self) -> SizeLevel;         // Small giữ Small
    pub fn larger(self) -> SizeLevel;          // Huge giữ Huge
    pub fn stroke_dip(self) -> f32;            // 2, 3, 5, 8   (Pen, Line, Arrow, Rect, Ellipse)
    pub fn marker_dip(self) -> f32;            // 10, 16, 24, 32
    pub fn text_dip(self) -> f32;              // 14, 18, 26, 36
    pub fn step_dip(self) -> f32;              // bán kính 10, 13, 17, 22
}
// Medium phải bằng STROKE_DIP, MARKER_DIP, TEXT_DIP hiện có.

// too_short: Ellipse như Rect. Step không bao giờ quá ngắn.
pub enum PressMode { Draw, Selection, Ignore, Pick }
// press_mode: (Some(Tool::Select), Inside | Outside) => Pick; (Some(Select), Handle) => Selection;
// mọi tổ hợp cũ giữ nguyên.

#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    /// Hình mới nằm ở `index` (sau thêm, hoặc hoàn tác một lần xóa).
    Inserted { index: usize },
    /// Hình `shape` đã bị lấy khỏi `index`.
    Removed { index: usize, shape: Shape },
    /// Hình ở `index` đã được thay; `old` là hình trước đó.
    Replaced { index: usize, old: Shape },
}
#[derive(Clone, Debug, PartialEq)]
pub struct Applied {
    pub change: Change,
    /// Vị trí (sau thay đổi) các Step khác đổi `n`, tăng dần; không gồm `index` của `change`.
    pub renumbered: Vec<usize>,
}

impl Annotations {
    pub fn shapes(&self) -> &[Shape];
    /// Thêm vào cuối, ghi sổ, xóa ngăn làm lại.
    pub fn add(&mut self, s: Shape) -> Applied;
    /// None khi `index` ngoài danh sách.
    pub fn remove(&mut self, index: usize) -> Option<Applied>;
    /// None khi `index` ngoài danh sách hoặc `s` (sau khi đánh số theo vị trí) bằng hệt hình cũ.
    pub fn replace(&mut self, index: usize, s: Shape) -> Option<Applied>;
    pub fn undo(&mut self) -> Option<Applied>;
    pub fn redo(&mut self) -> Option<Applied>;
    pub fn can_undo(&self) -> bool;
    pub fn can_redo(&self) -> bool;
    /// Số mà một Step thêm vào cuối sẽ mang (1 + số Step hiện có).
    pub fn next_step_number(&self) -> u32;
    pub fn is_empty(&self) -> bool;
    pub fn into_shapes(self) -> Vec<Shape>;
}
// Bỏ `push` và `undo() -> Option<Shape>` cũ. Sổ: Vec<Edit> hoàn tác và làm lại, với Edit riêng tư
// Add { index, shape } / Remove { index, shape } / Replace { index, before, after }.
// Hoàn tác Add → Removed; hoàn tác Remove → Inserted; hoàn tác Replace → Replaced { old: after }.
// Sau mọi lần áp: đánh lại n của mọi Step theo thứ tự (1, 2, 3…), trả các vị trí đổi n.
```

i18n (`Msg` mới, Việt / English):
- `ToolSelect` "Chọn (V)" / "Select (V)"
- `ToolEllipse` "Hình elip (E)" / "Ellipse (E)"
- `ToolStep` "Đánh số (N)" / "Numbered steps (N)"
- `ToolSize` "Cỡ ([ ])" / "Size ([ ])"
- `ToolRedo` "Làm lại (Ctrl+Y)" / "Redo (Ctrl+Y)"
- `SizeSmall` "Nhỏ" / "Small"; `SizeMedium` "Vừa" / "Medium"; `SizeLarge` "Lớn" / "Large"; `SizeHuge` "Rất lớn" / "Extra large"

## Các bước

- [ ] **1. Kiểm thử** (commit `test: annotation pro core model`; thân hàm mới `todo!()`). Trong `annotate.rs` mod tests:
  - `tool_keys_and_order`: `Tool::ALL` đúng thứ tự trên; `from_key(0x56)==Some(Select)`, `0x46`→Ellipse, `0x4E`→Step; P L A R H B T giữ nguyên.
  - `size_level_values`: bảng bốn mức đúng như Interfaces; `SizeLevel::default()==Medium`; `Medium.stroke_dip()==STROKE_DIP`, `marker_dip()==MARKER_DIP`, `text_dip()==TEXT_DIP`; `Small.smaller()==Small`, `Huge.larger()==Huge`, `Small.larger()==Medium`.
  - `ellipse_and_step_bounds`: Ellipse a(10,10) b(30,20) width 4 → `Rect{x:7,y:7,w:26,h:16}`; Step center(50,50) radius 10 → pad 10+0.6+1=11.6 → `Rect{x:38,y:38,w:24,h:24}`.
  - `too_short_ellipse_like_rect`.
  - `press_mode_select`: `(Some(Select), Inside)`→Pick, `(Some(Select), Outside)`→Pick, `(Some(Select), Handle(N))`→Selection; bảng cũ `press_mode_table` vẫn qua.
  - `add_undo_redo_multi`: add A, B, C; undo ×3 trả lần lượt `Removed{2,C}`, `Removed{1,B}`, `Removed{0,A}`; undo lần 4 None; redo ×3 trả `Inserted{0}`, `Inserted{1}`, `Inserted{2}` và `shapes()==[A,B,C]`; `can_undo`/`can_redo` đúng ở mỗi bước.
  - `new_edit_clears_redo`: add A, B; undo; add C → `can_redo()==false`, shapes `[A, C]`.
  - `remove_middle_then_undo_restores_position`: [A,B,C], remove(1) → `Removed{1,B}`, shapes [A,C]; undo → `Inserted{1}`, shapes [A,B,C]; redo → [A,C].
  - `replace_and_undo`: replace(0, A2) → `Replaced{0, old:A}`; undo → `Replaced{0, old:A2}` và shapes[0]==A; redo → shapes[0]==A2.
  - `replace_identical_is_not_recorded`: replace(0, A.clone()) → None, `can_undo()` chỉ còn mục add; replace(5, X) → None; remove(5) → None.
  - `steps_renumber`: add Step×3 (n truyền vào 0) → n = 1,2,3; `next_step_number()==4`; remove(0) → n của hai Step còn lại 1,2, `renumbered==[0,1]`; undo → 1,2,3, `renumbered==[1,2]`; chèn hình khác giữa không làm đổi số Step.
  - `replace_step_keeps_number`: replace một Step bằng bản dời tâm, `n` truyền 99 → lưu với n đúng vị trí; replace bằng bản giống hệt trừ `n` → None.
  - `history_memory_is_per_shape` (không cần đo bộ nhớ): sau 100 lần replace một Pen 10.000 điểm, `shapes().len()==1` (chống nhầm chép cả danh sách vào sổ chỉ bằng cách đọc mã; kiểm thử chỉ khẳng định đúng đắn: undo 100 lần trả đúng bản đầu).
  - Sửa `annotations_undo` cũ theo API mới.
  - i18n: thêm 9 dòng vào bảng kiểm thử hai ngôn ngữ.
- [ ] **2. Mã** theo Interfaces. Lớp Windows chỉ sửa để biên dịch và giữ hành vi hôm nay:
  - `push_mark` gọi `marks.add(shape)`; `pop_mark` thành hoàn tác qua `marks.undo()`: với `Removed{index, shape}` thì bỏ `mark_bounds[index]` (dùng `remove` an toàn khi index hợp lệ) và trả hộp bao của nó. Hành vi Ctrl+Z hôm nay không đổi (chỉ có thêm và hoàn tác).
  - `match` trên `Tool`/`Shape` chưa phủ: `Select`, `Ellipse`, `Step` không làm gì (`begin_stroke` trả None cho Select và Step; Ellipse có thể tạo như Rect nhưng chưa có nút nên không cần), tip tạm dùng `Msg::ToolSelect`/`ToolEllipse`/`ToolStep`; `draw_one` trả `true` cho hai hình mới (Task 46 vẽ thật). Ghi `// Task 46` / `// Task 47` ở các chỗ tạm. `SIDE_ITEMS` không đổi.
- [ ] **3.** `cargo fmt --all`; `powershell -File scripts/check.ps1` → `ALL CHECKS PASSED`. Commit `feat: annotation pro core model`.

**Xong khi:** check qua. Báo cáo: các commit, số kiểm thử lõi trước/sau, dòng cuối của check, chỗ làm khác gói.
