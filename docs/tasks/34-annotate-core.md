# Task 34: Mô hình đánh dấu ở core

**Agent:** `builder` · **Model:** sonnet · **Nhánh:** `task/34-annotate-core` · **Rà soát:** `reviewer`

Gói task tự chứa. Thuộc kế hoạch `docs/superpowers/plans/2026-11-17-annotate.md`. Task đầu của đợt 5.

## Cần đọc

- Thiết kế đợt 5 `docs/superpowers/specs/2026-11-17-annotate-design.md` mục 5
- Kế hoạch đợt 5: "Global Constraints", "Interfaces chung" (phần core, geometry, i18n), "Review Focus" dòng 2–4
- `crates/darkshot-core/src/{lib.rs, geometry.rs (Hit, Rect, place_toolbar và kiểm thử của nó), theme.rs (Rgb, contrast_ratio), i18n.rs (cách khai báo Msg và kiểm thử bảng)}`

## Global Constraints

- Không thêm crate, kể cả `dev-dependencies`. `darkshot-core` không phụ thuộc crate `windows`.
- Mọi chữ người dùng thấy lấy từ `darkshot_core::i18n`, đủ hai ngôn ngữ, đúng chữ trong kế hoạch.
- Release `panic = "abort"`: không `unwrap`/`expect`/chỉ số có thể panic trên dữ liệu đầu vào (chuỗi rỗng, con trỏ ở đầu/cuối, điểm trùng nhau, NaN không cần xử lý).
- Không sửa spec, kế hoạch, gói task cũ.
- Commit kiểm thử trước (`test:`, thân hàm mới `todo!()`), rồi mã (`feat:`). `cargo fmt --all`. Chạy cargo qua PowerShell. Kiểm `git branch --show-current` trước khi commit; không chuyển nhánh. Không tắt tiến trình `darkshot` không do mình khởi động. Không đẩy lên remote. Không đọc hay ghi khóa R2. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.

## Interfaces

**Produces:** đúng khối `annotate.rs`, `place_side_toolbar` và các `Msg` trong "Interfaces chung" của kế hoạch (tên, kiểu, hằng, giá trị). Task 35–37 dùng nguyên văn.

## Các bước

- [ ] **1. Kiểm thử trước.** Commit `test: annotate core`. Ít nhất:
  - `palette_and_constants_match_plan`: tám màu đúng thứ tự và giá trị; `DEFAULT_COLOR == PALETTE[0]`; các hằng đúng giá trị.
  - `tool_from_key`: `0x50`→Pen, `0x4C`→Line, `0x41`→Arrow, `0x52`→Rect, `0x48`→Marker, `0x54`→Text, phím khác `None`; `Tool::ALL` đúng thứ tự.
  - `dip_to_px_scales`: `dip_to_px(3.0, 96) == 3.0`, `(3.0, 144) == 4.5`, `(16.0, 192) == 32.0`.
  - `add_point_skips_sub_pixel_moves`: điểm cách điểm cuối 0,5 px bị bỏ; 1,0 px được thêm; danh sách rỗng luôn thêm.
  - `constrain_45_snaps`: (0,0)→(10,1) thành (10,0); (0,0)→(10,9) thành đường chéo 45° (|dx| == |dy|); (0,0)→(-1,-10) thành (0,-10).
  - `constrain_square_keeps_sign`: (5,5)→(1,12) thành (−2,12)… tức cạnh 7 theo dấu (dx âm, dy dương) — viết khẳng định đúng với định nghĩa `cạnh = max(|dx|,|dy|)`.
  - `too_short_rules`: Line (0,0)-(1,1) đúng; (0,0)-(2,0) sai; Rect (0,0)-(10,1) đúng; Pen luôn sai.
  - `arrow_geometry_normal_and_short`: mũi tên dài 100, width 3, min_head 12 → đầu dài 12 (vì 4×3=12), `head[0] == b`, `shaft_end` cách b đúng 12·cos(28°) dọc trục (sai số 0,01); mũi tên dài 5 → `shaft_end == a`, đầu dài 5.
  - `outline_color_picks_contrast`: đỏ `#FF3B30` → đen; đen → trắng; trắng → đen; xanh lam `#0A84FF` → đen nếu tương phản với đen cao hơn (tính bằng `contrast_ratio`), khẳng định theo kết quả hàm `contrast_ratio`, không đoán.
  - `press_mode_table`: đủ các tổ hợp (công cụ Some/None × has_shapes × Outside/Inside/Handle), đặc biệt `(None, true, Outside) == Ignore`, `(None, false, Outside) == Selection`, `(Some(Pen), _, Handle(SE)) == Selection`, `(Some(Pen), _, Inside) == Draw`, `(Some(Text), _, Outside) == Ignore`.
  - `bounds_include_width_and_head`: Line (10,10)-(20,10) width 4 → hộp chứa (8,8)-(22,12) và không lớn hơn mỗi phía quá 2 px; Arrow chứa cả ba đỉnh đầu; Text → `None`; Pen một điểm → hộp quanh điểm đó.
  - `annotations_undo`: push hai hình, undo trả hình thứ hai, còn một; undo khi rỗng trả `None`.
  - `text_edit_basic`: gõ "ab", Enter ("\n"), "c"; `line_col() == (1,1)`; up → (0,1); end → (0,2); down → (1,1) (kẹp); home → (1,0); backspace ở đầu dòng nối dòng.
  - `text_edit_unikey_sequence`: insert "Vie", backspace, insert "ê", insert "t", backspace, insert "ệ", insert "t" → "Việt" (kiểm theo `char`, có ký tự ngoài ASCII).
  - `text_edit_to_shape`: chỉ khoảng trắng và "\n" → `None`; "Hi" → `Shape::Text` đúng origin, size, màu; `set_color` đổi màu.
  - `sanitize_paste_normalizes_newlines`: "a\r\nb\rc\td\u{0007}" → "a\nb\nc d".
  - `side_toolbar_placement`: bên phải khi đủ chỗ (x = sel.right()+gap, bottom = sel.bottom()); bên trái khi sát mép phải màn hình; bên trong khi vùng chọn kín màn hình; kẹp dọc khi vùng chọn cao hơn thanh nhưng sát mép trên; tránh `avoid` khi giao; màn hình thứ hai có gốc âm.
  - `i18n`: mười `Msg` mới có trong bảng, hai bản khác nhau, đúng chữ kế hoạch (dùng `t(Lang::Vi/En, …)`).
- [ ] **2. Mã.** `crates/darkshot-core/src/annotate.rs` (`pub mod annotate` trong `lib.rs`), `place_side_toolbar` trong `geometry.rs`, `Msg` mới trong `i18n.rs`. `TextEdit` lưu `Vec<char>`. `constrain_45`: góc làm tròn tới bội 45°, độ dài = độ dài hình chiếu của ab lên hướng đó (không âm).
- [ ] **3.** `cargo fmt --all`; `powershell -File scripts/check.ps1` in `ALL CHECKS PASSED`. Commit `feat: annotate model in core`.

**Xong khi:** check qua; báo cáo nêu các commit, dòng cuối của check, số kiểm thử mới và chỗ nào làm khác kế hoạch (tên, kiểu) kèm lý do.
