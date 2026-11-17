# Task 46: Vẽ Hình elip và vòng Đánh số

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/46-pro-render` · **Rà soát:** `specialist`

Đợt 6. Thiết kế: `docs/superpowers/specs/2026-11-17-annotation-pro-design.md` (đọc mục 6, 8). Kế hoạch: `docs/superpowers/plans/2026-11-17-annotation-pro.md`. Dựa trên Task 44–45 (đã ở `main`).

## Global Constraints

- Không thêm crate. Không bật feature mới của crate `windows`.
- Một hàm vẽ duy nhất (`annotate_draw::draw_shapes`) cho lớp phủ và ảnh ra.
- Hỗ trợ Windows 10 và 11. Release `panic = "abort"`: không `unwrap`/`expect`/chỉ số có thể panic. `unsafe` nhỏ, kèm `// SAFETY:`. Giải phóng mọi đối tượng GDI và GDI+ trên mọi đường đi.
- Không sửa spec, kế hoạch đã duyệt, gói task cũ, `prototype/`.
- Quy trình: commit kiểm thử trước (`test:`), rồi `feat:`. `cargo fmt --all`. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`. Kiểm `git branch --show-current`; không chuyển nhánh, không đẩy. Chạy cargo qua PowerShell. Không tắt tiến trình `darkshot` không do mình khởi động.

## Cần đọc

- `crates/darkshot-windows/src/annotate_draw.rs` (toàn bộ, gồm kiểm thử: cách so điểm ảnh, `TextStyle`, `Pen`, `Brush`, `text_bounds`).
- `crates/darkshot-core/src/annotate.rs`: `Shape::Ellipse`, `Shape::Step`, `Shape::bounds`, `STEP_RING`, `outline_color`.

## Files

- Sửa: `crates/darkshot-windows/src/annotate_draw.rs`.

## Interfaces

Không đổi chữ ký công khai (`draw_shapes`, `shape_bounds`, `burn`, `caret_rect`). `draw_one` thay hai nhánh tạm của Task 44:

- `Shape::Ellipse { a, b, width, color }`: `GdipDrawEllipse` bút tròn đầu (như Line), màu alpha 255, trên hộp chuẩn hóa a–b; khử răng cưa như các hình khác.
- `Shape::Step { center, radius, n, color }`:
  1. Tô hình tròn tâm `center` bán kính `radius` bằng `color`.
  2. Viền tròn cùng bán kính, độ dày `radius * STEP_RING`, màu `outline_color(color)`.
  3. Số `n` (thập phân) ở giữa, màu `outline_color(color)`, Segoe UI đậm (cùng họ phông `TextStyle`), dựng bằng đường (path) như chữ: cỡ em ban đầu `radius * 1.1`; đo hộp đường; nếu rộng > `1.5 * radius` thì thu cỡ theo tỉ lệ để rộng = `1.5 * radius`; đặt tâm hộp đường trùng `center` (căn theo hộp đo được, không theo dòng chữ).
  4. `n == 0`: chỉ vẽ vòng, không số.
- `shape_bounds` của hai hình mới dùng `Shape::bounds` (lõi đã tính).

## Các bước

- [ ] **1. Kiểm thử** (commit `test: render ellipse and step`), theo kiểu so điểm ảnh của các kiểm thử `burn` hiện có:
  - `ellipse_drawn_inside_bounds`: `burn` một Ellipse lên ảnh trắng; có điểm ảnh khác trắng trên trục ngang tại mép trái của elip; điểm tâm vẫn trắng; mọi điểm ảnh đổi màu nằm trong `shape_bounds`.
  - `ellipse_circle_symmetric`: elip có hộp vuông cho số điểm ảnh đổi màu ở bốn góc phần tư lệch nhau ≤ 2%.
  - `step_fill_ring_and_number`: Step radius 20 màu `#FF0000` n 1 trên ảnh xám `#808080`: điểm ở (center.x + 0.6r, center.y) = màu đỏ; điểm ngay trong mép (center.x + r − r·STEP_RING/2, center.y) gần `outline_color(đỏ)` (sai số kênh ≤ 40); có ít nhất một điểm ảnh màu `outline_color(đỏ)` trong hình vuông cạnh r quanh tâm (số); mọi điểm đổi màu nằm trong `shape_bounds`.
  - `step_number_fits`: n 888 radius 12: mọi điểm ảnh màu số nằm trong hình tròn bán kính r (không tràn ra ngoài vòng).
  - `step_readable_on_light_and_dark`: vòng màu `#FFFF00` (vàng) → số đen; vòng `#000080` → số trắng (đọc điểm ảnh số).
  - `screen_matches_burn_for_new_shapes`: vẽ [Ellipse, Step] bằng `draw_shapes` lên DIB rồi so với `burn` trên ảnh cùng nền: giống từng byte (như kiểm thử khớp hiện có, nếu có; không thì dùng cùng đường `with_dib`).
- [ ] **2. Mã** theo Interfaces. Không tạo đối tượng GDI+ dư mỗi hình ngoài bút/chổi/đường cần thiết; giải phóng trên mọi đường lỗi.
- [ ] **3.** `cargo fmt --all`; check → `ALL CHECKS PASSED`. Commit `feat: render ellipse and numbered step`.

**Xong khi:** check qua. Báo cáo: commit, số kiểm thử, dòng cuối check, ảnh PNG thử (ghi `burn` ra file trong scratchpad của phiên nếu muốn xem, không commit), chỗ khác gói.
