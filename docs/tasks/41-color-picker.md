# Task 41: Bảng chọn màu tự do trong lớp phủ

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/41-color-picker` · **Rà soát:** `specialist`

Thuộc thiết kế `docs/superpowers/specs/2026-11-17-custom-colors-design.md` (đọc toàn bộ). Ràng buộc chung: mục "Global Constraints" của `docs/superpowers/plans/2026-11-17-annotate.md`. Chạy sau Task 40.

## Cần đọc

- Thiết kế trên; `crates/darkshot-core/src/color.rs` (Task 40: `Hls`, `rgb_to_hls`, `hls_to_rgb`, `CustomColors`, `picker_layout`, `spectrum_hs`, `spectrum_point`, `lum_at`, `lum_y`)
- `crates/darkshot-windows/src/overlay.rs`: bảng màu hiện có (`palette`, `palette_rect_for`, `Painter::palette`, `pick_color`, `escape`, `on_down`/`on_move`/`on_up`, `on_key`, `on_char`, `bar_chrome`, `update_bars`, kiểm thử bảng màu và `live::*`), `main.rs` (gọi `pick_region`, lưu cấu hình như cửa sổ Tùy chọn làm), `theme.rs` (vẽ nút, ô nhập dùng chung nếu tiện)

## Hành vi

Đúng mục "Hành vi" và "Nhìn" của thiết kế. Thêm:
- `pick_region(shot, initial, custom: &mut CustomColors)`; lớp phủ sửa trực tiếp `custom`. `main.rs` truyền `config.custom_colors`, sau khi lớp phủ đóng nếu khác trước thì cập nhật cấu hình trong bộ nhớ và ghi file bằng đúng đường lưu cấu hình hiện có (không làm mất hay ghi sai khóa R2); lỗi ghi thì bỏ qua.
- Bảng tự chọn là phần mở rộng của bảng màu hiện có (cùng cờ mở/đóng, cùng quy tắc Esc, bấm ngoài, nuốt cú bấm); dạng đủ nhớ trong lần chụp.
- Ô sắc độ: dựng một DIB cỡ ô cho mỗi DPI khi cần (lưu trong `State`, xóa ở `Drop`), không tính lại mỗi lần vẽ. Thanh độ sáng dựng lại khi Hue/Sat đổi.
- Phím khi một ô số đang sửa: chữ số, Backspace, Tab/Shift+Tab, lên/xuống, Enter (chốt ô; Enter lần nữa = OK), Esc (bỏ sửa ô; Esc lần nữa đóng bảng). Các phím này không đến công cụ vẽ hay ô chữ. Khi bảng mở mà không ô số nào sửa: phím chữ chọn công cụ không có tác dụng.
- Kéo trong ô sắc độ hoặc thanh độ sáng: bắt chuột (`SetCapture`) như kéo vùng chọn; nhả ngoài bảng không đóng bảng.
- Vùng làm mới: chỉ phần bảng đổi; mở rộng/thu gọn làm mới cả bảng cũ và mới.

## Các bước

- [ ] **1. Kiểm thử trước** (`test: colour picker`). Hàm thuần tách ra khi được; phần cần cửa sổ dùng `live::*` (`#[ignore]`, chạy từng cái với `--test-threads=1`):
  - `field_edit_rules` (thuần): gõ "2","5","9" vào Red → kẹp 255 khi chốt; Hue "300" → 239; Backspace; lên/xuống kẹp biên; sửa Red đổi Hue/Sat/Lum theo `rgb_to_hls`.
  - `picker_compact_click_selects_and_closes` (live): mở bảng, bấm ô cơ bản thứ 5 → màu vẽ là `PALETTE[5]`, bảng đóng, lớp phủ còn.
  - `picker_define_add_ok` (live): mở bảng, bấm "Định nghĩa…", bấm giữa ô sắc độ, kéo thanh độ sáng, "Thêm vào màu tự chọn", "OK", vẽ một đường, Lưu → `custom` có đúng một ô bằng màu của đường; màu đường khớp `hls_to_rgb` của điểm đã bấm ±2.
  - `picker_cancel_keeps_custom` (live): dạng đủ, thêm màu, Esc → màu vẽ không đổi, `custom` vẫn có màu đã thêm, lớp phủ còn.
  - `picker_typing_does_not_reach_tools` (live): sửa ô Red, gõ "P" và chữ số → công cụ không đổi.
  - Vòng chống rò GDI của lớp phủ thêm mở dạng đủ, kéo ô sắc độ, đóng (DIB ô sắc độ được xóa).
  - `main`: hàm thuần quyết định có ghi cấu hình không (`custom` đổi → có; không đổi → không).
- [ ] **2. Mã** theo "Hành vi".
- [ ] **3. Tài liệu.** README đoạn **Màu:** thêm phần tự chọn màu (dạng gọn, nút "Định nghĩa màu tự chọn >>", ô sắc độ, thanh độ sáng, sáu ô số, thêm vào màu tự chọn, OK/Hủy, màu tự chọn được lưu); quyết định mới trong `memory-bank/decisions.md` (số kế tiếp): bảng chọn màu tự vẽ giao diện tối theo mẫu hộp Windows, HSL thang 240 như Windows, màu tự chọn lưu trong cấu hình, chủ dự án chọn; ghi thêm mọi lựa chọn riêng của task này. Checklist mục Đợt 5 thêm dòng `[ ]`: dạng gọn và dạng đủ nằm trọn trong màn hình ở DPI 100% và 150%, hai ngôn ngữ chữ không bị cắt; chọn màu bằng ô sắc độ, thanh độ sáng, ô số; Thêm vào màu tự chọn; màu tự chọn còn sau khi thoát và mở lại darkshot; khóa R2 vẫn dùng được sau đó (Kiểm tra kết nối).
- [ ] **4.** `cargo fmt --all`; check qua. Commit `feat: custom colour picker in overlay`.

Quy trình chung: không thêm crate (có thể bật feature của crate `windows` nếu thiếu); không `unwrap`/`expect`/chỉ số có thể panic trên đường chạy thật; `unsafe` kèm `// SAFETY:`; giải phóng mọi đối tượng GDI/GDI+; kiểm `git branch --show-current` trước khi commit, không chuyển nhánh, không đẩy; không tắt tiến trình `darkshot` không do mình khởi động (build release thì `--target-dir target/task41`); không đọc hay ghi khóa R2 hay file cấu hình thật; `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.

**Xong khi:** check qua; báo cáo nêu commit, dòng cuối check, kết quả kiểm thử chạy tay, ảnh chụp bảng ở dạng đủ (cách chụp: kiểm thử tạm dùng bước chụp như phiên điều phối đã làm ở Task 39 — thêm tạm `Step::Snap` gọi `capture::capture_virtual_screen` rồi `image::encode_png` ghi ra `C:/Users/Client/AppData/Local/Temp/claude/9bda5e44761d6d15/a55dcc96-f74b-4a96-9ec7-65a267b4934d/scratchpad/t41_*.png`; không commit phần tạm), chỗ khác thiết kế và những gì chưa nhìn bằng mắt.
