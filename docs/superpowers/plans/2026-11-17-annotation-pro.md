# darkshot đợt 6 (Annotation Pro) Implementation Plan

> **For agentic workers:** kế hoạch này chạy bằng quy trình của dự án: `/next-task` giao từng gói `docs/tasks/NN-ten-ngan.md` cho agent ghi trong `memory-bank/progress.md`, rồi rà soát, kiểm tra, gộp. Các bước chi tiết (dạng `- [ ]`) nằm trong gói task; file này giữ phần chung mà mọi gói dựa vào.

**Goal:** Hoàn tác/làm lại nhiều bước; công cụ Chọn (chọn, di chuyển, đổi cỡ, xóa, đổi kiểu từng hình); công cụ Hình elip và Đánh số; bốn mức cỡ dùng chung — mà ảnh ra vẫn khớp màn hình và vùng che không lộ.

**Architecture:** `darkshot-core::annotate` thêm hai loại hình, ba công cụ, `SizeLevel` và sổ thay đổi trong `Annotations` (thêm/xóa/thay tại vị trí, đánh lại số vòng). Module mới `darkshot-core::edit` giữ phần thuần của việc sửa: trúng hình, tay nắm, biến đổi theo loại hình (Làm mờ tính lại màu trên ảnh gốc), đổi kiểu. `geometry::side_layout` xếp thanh dọc một hoặc hai cột. Lớp Windows: `annotate_draw` vẽ hai hình mới; `overlay.rs` nối thanh dọc mới, bảng Cỡ, sổ thay đổi, hai công cụ mới (Task 47) rồi công cụ Chọn (Task 48).

**Tech Stack:** như các đợt trước. Không thêm crate, không bật feature mới của crate `windows` trừ khi gói task nói.

**Spec:** `docs/superpowers/specs/2026-11-17-annotation-pro-design.md` (gọi là "thiết kế đợt 6"); mục 11 của nó là các quyết định chốt. Thiết kế đợt 5 và Làm mờ vẫn đúng ở mọi chỗ thiết kế đợt 6 không nói khác.

## Global Constraints

- Không thêm crate, kể cả `dev-dependencies`; `Cargo.lock` không thêm gói. `darkshot-core` không phụ thuộc crate `windows` hay API riêng của hệ điều hành.
- Mọi chữ người dùng thấy lấy từ bảng `darkshot_core::i18n`, đủ hai ngôn ngữ; không viết cứng. Kiểm thử mới ở crate Windows không khẳng định chữ đọc từ `lang::tr`; dùng `t(Lang::Vi, …)`.
- Màu giao diện từ `darkshot_core::theme` (khung chọn hình và viền ô Cỡ đang dùng: `ACCENT`). Màu nét vẽ là `annotate::PALETTE`/màu tự chọn.
- Khi không chọn công cụ, lớp phủ chạy đúng như hôm nay. Không hình nào thì ảnh ra đúng từng byte như hôm nay. Mức cỡ mặc định (Vừa) cho đúng độ dày/cỡ chữ hôm nay.
- Hình lưu theo tọa độ ảnh chụp (`f32`); độ dày, cỡ chữ, bán kính lưu bằng px, đổi từ DIP theo DPI màn hình chứa điểm đầu của hình (hình mới) hoặc tâm hộp bao (đổi kiểu).
- Một hàm vẽ duy nhất (`annotate_draw::draw_shapes`) cho lớp phủ và ảnh ra. Khung chọn, tay nắm, viền ô chữ không bao giờ đi qua `draw_shapes`/`burn`.
- Làm mờ: màu che luôn tính trên ảnh chụp gốc (`redact_color`), cả khi xem trước lúc kéo lẫn khi chốt; hoàn tác/làm lại trả nguyên hình đã lưu.
- Một cử chỉ kéo (di chuyển, đổi cỡ) là đúng một mục sổ; hủy (Esc) không ghi; thay bằng hình bằng hệt không ghi.
- Hiệu năng: mỗi lần chuột di chuyển khi kéo chỉ làm mới hợp hộp bao cũ/mới (cộng tay nắm); không vẽ lại cả màn hình, không chép ảnh cỡ màn hình.
- Hỗ trợ Windows 10 và 11. Release `panic = "abort"`: mã chạy thật không `unwrap`/`expect`/chỉ số có thể panic. `unsafe` nhỏ, kèm `// SAFETY:`. Giải phóng mọi đối tượng GDI và GDI+ trên mọi đường đi; kiểm thử chống rò của lớp phủ phải qua.
- Không sửa spec, kế hoạch đã duyệt, gói task cũ (`docs/tasks/01` đến `43`), `prototype/`.
- Quy trình: commit kiểm thử trước (`test:`, thân hàm mới là `todo!()`; chỉ thêm đủ mã để biên dịch), rồi commit mã (`feat:`). `cargo fmt --all` trước khi commit. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`. Kiểm `git branch --show-current` trước khi commit; không chuyển nhánh. Chạy cargo qua PowerShell. Không tắt tiến trình `darkshot` không do mình khởi động; exe bị khóa thì build release với `--target-dir target/taskNN`. Không đẩy lên remote. Không đọc hay ghi khóa R2, file cấu hình thật. Kiểm thử chạy tay (`#[ignore]`) chạy từng cái với `--test-threads=1`.

## Phân bổ agent và model

| Task | Việc | Agent | Model | Rà soát | Lý do |
|---|---|---|---|---|---|
| 44 | Core: hai hình mới, ba công cụ, `SizeLevel`, sổ thay đổi, đánh số, i18n; chỉnh tối thiểu lớp Windows cho biên dịch | builder | sonnet | reviewer | Rust thuần, đặc tả và kiểm thử rõ |
| 45 | Core: `edit` (trúng hình, tay nắm, biến đổi, đổi kiểu), `geometry::side_layout` | builder | sonnet | specialist | Thuần nhưng nhiều trường hợp biên; biến đổi Làm mờ liên quan riêng tư |
| 46 | Vẽ Hình elip và vòng số, hộp bao, `burn` | specialist | opus | specialist | GDI+, đo chữ, khớp màn hình/ảnh ra |
| 47 | Lớp phủ: thanh 14 mục (hai cột), bảng Cỡ, sổ thay đổi, Làm lại, Hình elip, Đánh số | specialist | opus | specialist | Nhiều trạng thái trong `overlay.rs`, vẽ lại theo vùng, DPI |
| 48 | Lớp phủ: công cụ Chọn (chọn, kéo, đổi cỡ, xóa, đổi kiểu, Esc) | specialist | opus | specialist | Tương tác khó nhất, quyền riêng tư của vùng che |
| 49 | Tài liệu đợt 6 | scaffolder | haiku | reviewer | Việc máy móc |

Chạy tuần tự 44 → 49, không dừng giữa chừng (quyết định 75).

## Interfaces chung

Chi tiết đầy đủ trong từng gói; tóm tắt để các task khớp tên:

```rust
// darkshot-core/src/annotate.rs
pub enum Tool { Select, Pen, Line, Arrow, Rect, Ellipse, Marker, Blur, Text, Step }
pub enum Shape { /* cũ */ Ellipse { a: Pt, b: Pt, width: f32, color: Rgb },
                 Step { center: Pt, radius: f32, n: u32, color: Rgb } }
pub enum SizeLevel { Small, Medium, Large, Huge }
pub enum PressMode { Draw, Selection, Ignore, Pick }
pub enum Change { Inserted { index: usize }, Removed { index: usize, shape: Shape },
                  Replaced { index: usize, old: Shape } }
pub struct Applied { pub change: Change, pub renumbered: Vec<usize> }
impl Annotations { add, remove, replace, undo, redo, can_undo, can_redo, next_step_number, shapes, is_empty, into_shapes }

// darkshot-core/src/edit.rs
pub enum Grip { Move, Box(Handle), End(usize) }
pub struct Gesture { index, grip, start: Pt, original: Shape, original_bounds: Rect }
pub fn hit_shape, topmost_hit, handles, grip_at, frame, transform, restyle

// darkshot-core/src/geometry.rs
pub struct SideLayout { pub size: (i32, i32), pub items: Vec<Rect>, pub seps: Vec<Rect> }
pub fn side_layout(n: usize, seps: &[usize], b: i32, sep: i32, max_h: i32) -> SideLayout
```

## Review Focus

1. **Chỉ số lệch giữa `marks` và `mark_bounds` sau xóa giữa danh sách rồi hoàn tác/làm lại** → vẽ sót hoặc chọn nhầm hình. Task 47 kiểm `mark_bounds_follow_history` (thêm 3, xóa giữa, hoàn tác, làm lại, so từng hộp với `shape_bounds` của hình cùng vị trí).
2. **Vùng che sau khi dời ra mép ảnh / đổi cỡ qua cạnh đối diện vẫn kín và màu lấy từ ảnh gốc**, kể cả khi có nét vẽ đè bên dưới trong danh sách. Task 45 kiểm `blur_move_recolors_from_original` và `blur_resize_flip_stays_covered`; Task 48 kiểm `live::blur_move_save_covered` (ảnh ra: mọi điểm ảnh trong vùng mới bằng `redact_color` ảnh gốc).
3. **Bấm-thả không kéo trên hình khi công cụ Chọn bật** sinh mục sổ rỗng làm Ctrl+Z "không làm gì". Task 44 kiểm `replace_identical_is_not_recorded`; Task 48 kiểm `live::click_select_records_nothing`.
4. **Không chọn công cụ thì kéo vẫn tạo vùng mới, Esc vẫn đóng lớp phủ** (đường cũ không bị bậc Esc mới chặn). Task 48 kiểm `live::no_tool_behaves_as_before`.
5. **Màn hình thấp (thanh 14 mục không vừa)**: thanh tràn ra ngoài màn hình hoặc hai cột chồng thanh bốn nút. Task 45 kiểm `side_layout_wraps_when_short`; Task 47 kiểm vị trí thanh trong màn hình 1920×1080 ở 200% (540 DIP).

## Task

Gói task: `docs/tasks/44-pro-core-model.md`, `45-pro-core-edit.md`, `46-pro-render.md`, `47-pro-overlay-tools.md`, `48-pro-overlay-select.md`, `49-docs-batch6.md`.
