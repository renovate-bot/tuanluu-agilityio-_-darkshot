# Task 35: Vẽ hình đánh dấu bằng GDI+ và in vào ảnh

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/35-annotate-render` · **Rà soát:** `specialist`

Gói task tự chứa. Thuộc kế hoạch `docs/superpowers/plans/2026-11-17-annotate.md`. Chạy sau Task 34.

## Cần đọc

- Thiết kế đợt 5 `docs/superpowers/specs/2026-11-17-annotate-design.md` mục 5.3, 5.6
- Kế hoạch đợt 5: "Global Constraints", "Interfaces chung", "Review Focus" dòng 1, 5
- `crates/darkshot-windows/CLAUDE.md`
- `crates/darkshot-core/src/annotate.rs` (Task 34), `crates/darkshot-core/src/image.rs`
- `crates/darkshot-windows/src/overlay.rs`: `Gp`, `GdiPlus`, `Painter::toolbar`/`Gp::icon` (cách dùng GDI+ hiện có), `dib_info`, `BackBuffer`

## Global Constraints

- Không thêm crate. Feature `Win32_Graphics_GdiPlus` đã bật. `darkshot-core` không phụ thuộc crate `windows`.
- Một hàm vẽ duy nhất (`draw_shapes`) cho lớp phủ và ảnh ra.
- `burn` không đổi ảnh khi `shapes` rỗng (đúng từng byte); ảnh ra luôn alpha 255.
- Chuyển `GdiPlus` và `Gp` từ `overlay.rs` sang `gdiplus.rs` không đổi hành vi của lớp phủ (kiểm thử lớp phủ hiện có phải qua, kể cả `overlay_round_trip_without_gdi_leaks` nếu chạy được).
- Release `panic = "abort"`: không `unwrap`/`expect`/chỉ số có thể panic. `unsafe` nhỏ, kèm `// SAFETY:`. Mọi bút, chổi, đường, phông, họ phông, định dạng chuỗi, đồ họa GDI+ và DC/DIB được xóa trên mọi đường đi.
- Kiểm thử mới không khẳng định chữ đọc từ `lang::tr`.
- Commit kiểm thử trước (`test:`, thân hàm mới `todo!()`), rồi mã (`feat:`). `cargo fmt --all`. Chạy cargo qua PowerShell. Kiểm `git branch --show-current` trước khi commit; không chuyển nhánh. Không tắt tiến trình `darkshot` không do mình khởi động. Không đẩy lên remote. Không đọc hay ghi khóa R2. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.

## Interfaces

**Consumes:** `darkshot_core::annotate::*` (Task 34), `darkshot_core::image::Image`.
**Produces:** `gdiplus.rs` và `annotate_draw.rs` đúng như "Interfaces chung" (`GdiPlus`, `Gp`, `draw_shapes`, `shape_bounds`, `caret_rect`, `burn`).

## Cách vẽ (thiết kế mục 5.3)

- Pen: một đường nối các điểm (`GdipDrawLines`/path), đầu và chỗ nối tròn; một điểm → chấm tròn đường kính `width`.
- Arrow dùng `arrow_geometry(a, b, width, annotate::arrow_min_head(width))` (Task 34 thêm `arrow_min_head`; `Shape::bounds` dùng cùng công thức, nên hình vẽ khớp hộp bao). Thêm khẳng định `arrow_min_head(4.5) == 18.0` vào một kiểm thử của task này.
- Line: đầu tròn. Arrow: thân từ `a` tới `arrow_geometry(..).shaft_end` (đầu phẳng ở phía mũi), đầu là tam giác đặc `head`. Rect: viền `width`, góc vuông (nối Miter), hình chữ nhật chuẩn hóa từ a, b; viền nằm giữa trên đường a–b như CSS stroke.
- Marker: màu với alpha `MARKER_ALPHA`, đầu vuông, cả nét phủ một lần (chỗ tự cắt không đậm hơn): ví dụ vẽ nét đục vào mặt nạ rồi trộn một lần, hoặc `GdipWidenPath` + tô `FillModeWinding`.
- Text: Segoe UI đậm, em = `size` px; mỗi dòng cách nhau bằng chiều cao dòng của họ phông; chữ là path (`GdipAddPathString`), vẽ viền `outline_color(color)` dày 2 × `TEXT_OUTLINE_DIP` đổi ra px theo tỉ lệ `size / TEXT_DIP` (nối tròn) rồi tô `color`. `shape_bounds` và `caret_rect` đo bằng đúng phông và định dạng này.
- Khử răng cưa (`SmoothingModeAntiAlias`, `PixelOffsetModeHalf`) cho mọi hình; `TextRenderingHint` không dùng vì chữ là path.
- `burn`: DIB 32 bit cỡ ảnh, chép `bgra` vào, `draw_shapes(dc, shapes, crop.x, crop.y, crop)`, chép ra, đặt alpha 255.

## Các bước

- [ ] **1. Kiểm thử trước.** Commit `test: annotate rendering`. Vẽ lên DIB trong bộ nhớ (không cần màn hình), đọc pixel:
  - `empty_burn_is_identity`: ảnh ngẫu nhiên, `burn(&mut img, &[], crop)` → không đổi byte nào.
  - `line_pixels`: nền đen 40×20, Line (5,10)-(35,10) width 3 màu đỏ → pixel (20,10) đỏ (sai số 2), (20,14) đen, mọi alpha 255.
  - `rect_outline_not_filled`: tâm hình chữ nhật vẫn màu nền; điểm giữa cạnh trên mang màu nét.
  - `arrow_head_filled`: điểm trong tam giác đầu mũi tên mang màu nét.
  - `short_arrow_draws_head_only`: mũi tên dài 5 px không vẽ thân ra sau `a` (pixel phía sau `a` 3 px là nền).
  - `marker_is_translucent`: Marker trắng trên nền đen → kênh R khoảng 102 (±6).
  - `marker_self_overlap_not_darker`: Marker hình chữ X tự cắt, màu đen trên nền trắng → pixel tại giao điểm bằng pixel giữa một nhánh (±3).
  - `text_draws_outline_and_fill`: Text "H" đỏ size 40 trên nền xám giữa → có pixel đỏ và có pixel đen (viền); `shape_bounds` chứa mọi pixel khác nền.
  - `burn_offsets_by_crop_origin`: ảnh đầy đủ 200×100; hình ở tọa độ ảnh chụp; `burn` trên ảnh cắt (crop x=50,y=20) cho cùng pixel với vẽ lên ảnh đầy đủ rồi `crop` (so toàn bộ, sai số 0).
  - `clip_limits_drawing`: `draw_shapes` với `clip` nhỏ không đổi pixel ngoài clip.
  - `render_no_gdi_leak` (`#[ignore]` nếu số đếm GDI bị ảnh hưởng bởi kiểm thử song song, kèm lệnh chạy): `burn` 200 lần đủ loại hình → `GetGuiResources(GR_GDIOBJECTS)` không tăng.
- [ ] **2. Mã.** `gdiplus.rs` (chuyển, `overlay.rs` dùng lại), `annotate_draw.rs`, khai báo module trong `main.rs`.
- [ ] **3.** `cargo fmt --all`; `powershell -File scripts/check.ps1` in `ALL CHECKS PASSED`. Commit `feat: draw annotations with GDI+`.

**Xong khi:** check qua; báo cáo nêu các commit, dòng cuối của check, kiểm thử nào `#[ignore]` và lệnh chạy, cách làm bút dạ quang, và chỗ nào khác kế hoạch.
