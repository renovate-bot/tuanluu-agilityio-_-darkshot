# Task 36: Thanh công cụ vẽ trong lớp phủ

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/36-annotate-overlay` · **Rà soát:** `specialist`

Gói task tự chứa. Thuộc kế hoạch `docs/superpowers/plans/2026-11-17-annotate.md`. Chạy sau Task 35. Công cụ Chữ (soạn chữ) là Task 37: ở task này nút Chữ có trên thanh và chọn được, nhưng bấm vào vùng chọn khi đang chọn Chữ chưa làm gì.

## Cần đọc

- Thiết kế đợt 5 `docs/superpowers/specs/2026-11-17-annotate-design.md` mục 2, 5.1, 5.2, 5.3, 5.5, 5.6
- Kế hoạch đợt 5: "Global Constraints", "Interfaces chung", "Review Focus" dòng 1–3
- `crates/darkshot-windows/CLAUDE.md`
- `crates/darkshot-windows/src/overlay.rs` (toàn bộ), `main.rs` (quanh `overlay::pick_region`, `run_on_image`), `annotate_draw.rs`, `gdiplus.rs` (Task 35), `crates/darkshot-core/src/annotate.rs` (Task 34)

## Global Constraints

- Không thêm crate. `darkshot-core` không phụ thuộc crate `windows`. Mọi chữ hiển thị từ `darkshot_core::i18n`.
- Giao diện theo đợt 4 (màu từ `darkshot_core::theme`). Biểu tượng tự vẽ bằng GDI+ theo cách `Gp::icon`, lưới 24×24, nét 1,8; không chép biểu tượng của ứng dụng khác.
- Khi không chọn công cụ và chưa có hình nào, lớp phủ chạy đúng như hôm nay (kiểm thử lớp phủ hiện có phải qua không sửa khẳng định). Hai phím tắt toàn màn hình không đổi. Không hình thì ảnh ra đúng từng byte như hôm nay.
- Chỉ làm mới vùng bị ảnh hưởng (hộp bao đoạn mới, nút, bảng màu); không `InvalidateRect(None)` mỗi lần chuột di chuyển.
- Release `panic = "abort"`. `unsafe` nhỏ, kèm `// SAFETY:`. Giải phóng mọi đối tượng GDI/GDI+.
- Kiểm thử mới không khẳng định chữ đọc từ `lang::tr`.
- Commit kiểm thử trước (`test:`, thân hàm mới `todo!()`), rồi mã (`feat:`). `cargo fmt --all`. Chạy cargo qua PowerShell. Kiểm `git branch --show-current` trước khi commit; không chuyển nhánh. Không tắt tiến trình `darkshot` không do mình khởi động; exe bị khóa thì `--target-dir target/task36`. Không đẩy lên remote. Không đọc hay ghi khóa R2. Kiểm thử `#[ignore]` chạy từng cái với `--test-threads=1`. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.

## Interfaces

**Consumes:** `annotate::{Tool, Shape, Annotations, PressMode, press_mode, PALETTE, DEFAULT_COLOR, STROKE_DIP, MARKER_DIP, dip_to_px, add_point, constrain_45, constrain_square, too_short}`, `geometry::place_side_toolbar`, `annotate_draw::{draw_shapes, shape_bounds, burn}`, `gdiplus::{GdiPlus, Gp}`, `Msg::Tool*`, `Msg::AnnotateFailed*`.
**Produces:** `overlay::Picked`, `pick_region(...) -> Result<Option<Picked>, String>` như kế hoạch. Cho Task 37: trạng thái lớp phủ có `tool: Option<Tool>`, `marks: Annotations`, `color: Rgb`; một chỗ rõ ràng để Task 37 cắm ô chữ (ghi trong báo cáo).

## Hành vi cần làm

- Thanh dọc (mục 5.1): thứ tự Pen, Line, Arrow, Rect, Marker, Text, vạch ngăn, Color (ô vuông màu hiện tại, viền `OVERLAY_BORDER`), vạch ngăn, Undo (mờ `TEXT_DISABLED` khi rỗng); nút 36 DIP, vạch ngăn 1 px có lề; nút đang chọn: nền `OVERLAY_HOVER`, biểu tượng `ACCENT`, vạch `ACCENT` 2 DIP mép trái; chú thích khi trỏ dùng `Msg::Tool*` như chú thích thanh bốn nút. Vị trí `place_side_toolbar(sel, size, gap 6 DIP, màn hình chứa góc dưới phải, avoid = thanh bốn nút)`. Ẩn khi đang kéo khung mới, như thanh bốn nút.
- Chế độ (mục 5.2): bấm nút công cụ hoặc phím P/L/A/R/H/T (không Ctrl/Alt) chọn; chọn lại bỏ chọn. Bấm chuột dùng `press_mode`. Con trỏ: chữ thập cho công cụ vẽ trong vùng chọn, chữ I cho Text; tay nắm vẫn là con trỏ đổi cỡ.
- Vẽ (mục 5.3): bắt đầu nét lấy DPI của màn hình chứa điểm đầu (`dpi_at`), độ dày `dip_to_px(STROKE_DIP hoặc MARKER_DIP, dpi)`; Pen/Marker `add_point` mỗi `WM_MOUSEMOVE`; Line/Arrow/Rect cập nhật điểm cuối, Shift (`GetKeyState(VK_SHIFT)`) dùng `constrain_45` (Line, Arrow) hoặc `constrain_square` (Rect); nhả chuột: `too_short` thì bỏ, còn lại `push`. Nét đang vẽ hiển thị ngay. Hình (cả nét đang vẽ) vẽ trong `draw` sau ảnh sáng, trước viền vùng chọn, tay nắm và thanh, cắt theo vùng chọn.
- Màu (mục 5.5): bảng 2×4 ô cạnh nút Màu (bên trái thanh nếu thanh ở bên phải vùng chọn, ngược lại bên phải), ô 24 DIP, khe 6 DIP, nền `SURFACE`, viền `OVERLAY_BORDER`, ô đang chọn viền `TEXT` 2 DIP; bấm ô chọn màu và đóng; bấm ngoài hoặc Esc đóng (Esc lúc này không đóng lớp phủ, kể cả qua phím tắt Esc toàn cục). Màu nhớ qua các lần chụp trong tiến trình (`thread_local`), bắt đầu bằng `DEFAULT_COLOR`.
- Hoàn tác: nút Undo, Ctrl+Z; làm mới hộp bao hình bị bỏ.
- Kết quả (mục 5.6): `finish` trả `Picked { action, rect, shapes }`. `main.rs`: sau `crop`, nếu `shapes` không rỗng thì `annotate_draw::burn(&mut image, &shapes, rect)`; lỗi thì thông báo Error `AnnotateFailedTitle`/`AnnotateFailedBody` và dừng (không lưu, chép, tải ảnh thiếu nét). "Giữ vị trí vùng" dùng `rect` như cũ.

## Các bước

- [ ] **1. Kiểm thử trước.** Commit `test: annotate overlay`. Ưu tiên hàm thuần tách ra từ `overlay.rs`; phần cần cửa sổ dùng mẫu kiểm thử có sẵn (gửi `WM_LBUTTONDOWN/MOVE/UP` giả như kiểm thử Esc và nút Lưu hiện có), `#[ignore]` nếu cần màn hình thật, kèm lệnh chạy.
  - `side_bar_button_hit_testing`: điểm giữa mỗi nút trả đúng mục (6 công cụ, Màu, Hoàn tác); vạch ngăn và lề trả `None`.
  - `palette_hit_testing`: điểm giữa ô thứ i trả i; khe trả `None`.
  - `segment_invalidation_is_local` (Review Focus 2): hộp làm mới của một bước kéo Pen từ p tới q là hộp bao đoạn pq nới nửa độ dày + 2 px, không lớn hơn.
  - `drag_outside_with_shapes_keeps_everything` (Review Focus 3): lớp phủ có vùng chọn, chọn Pen, vẽ một nét, bỏ chọn công cụ, kéo ngoài vùng → vùng chọn không đổi, vẫn một hình; rồi Lưu → `Picked.shapes.len() == 1`.
  - `draw_line_and_save_returns_shape`: chọn Line, kéo trong vùng, Ctrl+S → `Picked` có đúng một `Shape::Line` với `a`, `b` theo tọa độ ảnh chụp và `width == dip_to_px(3.0, dpi)`.
  - `undo_removes_last_shape`: hai nét, Ctrl+Z, Lưu → một hình.
  - `no_tool_flow_unchanged`: không chọn công cụ, kéo vùng, Lưu → `Picked.shapes` rỗng, `rect` như trước.
  - `esc_closes_palette_not_overlay`: mở bảng màu, Esc → bảng đóng, lớp phủ còn; Esc lần nữa → `Ok(None)`.
  - `annotated_capture_writes_marked_image` (có thể đặt ở `main.rs` dưới dạng hàm thuần nhận `Picked` + ảnh): ảnh ra có pixel màu nét ở đúng chỗ, ảnh không hình giống hệt `crop`.
  - Kiểm thử chống rò của lớp phủ hiện có thêm vòng có vẽ hình (mở, vẽ, hoàn tác, mở bảng màu, đóng) và vẫn qua.
- [ ] **2. Mã** theo "Hành vi cần làm".
- [ ] **3. Checklist.** Thêm mục "Đợt 5: công cụ đánh dấu" vào `docs/testing/manual-checklist.md`, mỗi dòng `[ ]`: thanh dọc hiện đúng chỗ (phải, trái, trong vùng; hai màn hình; DPI 100% và 150%); năm công cụ vẽ đúng hình, Shift khóa góc và hình vuông; màu đổi, nhớ sang lần chụp sau; hoàn tác; kéo ngoài vùng khi có nét không mất nét; ảnh tải lên, lưu, chép có đúng nét như màn hình (mở file so); không công cụ thì mọi thứ như cũ; vẽ nét dài trên màn hình lớn không giật.
- [ ] **4.** `cargo fmt --all`; `powershell -File scripts/check.ps1` in `ALL CHECKS PASSED`. Commit `feat: annotation toolbar in overlay`.

**Xong khi:** check qua; báo cáo nêu các commit, dòng cuối của check, kiểm thử `#[ignore]` và kết quả khi chạy tay, chỗ Task 37 cắm ô chữ, và những gì chưa nhìn bằng mắt. Nếu chụp được ảnh lớp phủ (cách chụp ở `docs/tasks/30b-check-radio-aa.md` mục "Tự kiểm bằng mắt", dùng `scratchpad/cap.ps1`), để ảnh trong thư mục scratch và nêu đường dẫn.
