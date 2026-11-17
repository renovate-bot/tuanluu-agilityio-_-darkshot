# Task 42: Công cụ Làm mờ (che thông tin nhạy cảm)

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/42-blur-redact` · **Rà soát:** `specialist`

Thay đổi nhỏ sau đợt 5. Chủ dự án muốn che thông tin nhạy cảm sao cho không khôi phục được và đã chọn cách "kính mờ giả": tô kín vùng che bằng **một màu trung bình duy nhất** của ảnh chụp gốc. Thiết kế đã duyệt: `docs/superpowers/specs/2026-11-17-redact-blur-design.md` (đọc toàn bộ, ngắn). Ràng buộc chung: mục "Global Constraints" của `docs/superpowers/plans/2026-11-17-annotate.md`. Thiết kế đợt 5 không sửa.

## Quyết định của điều phối viên (thiết kế để lại cho người làm)

- **Không có mép mềm.** Vùng che có cạnh sắc, đúng bằng hình chữ nhật điểm ảnh `blur_pixel_rect(a, b)`. Lý do: giữ ảnh ra khớp màn hình và bảo đảm mọi điểm ảnh bên trong vùng đều đúng bằng màu che.
- **Vẽ không khử răng cưa.** Điểm ảnh ở mép không được trộn với nội dung bên dưới. Mọi điểm ảnh của `blur_pixel_rect` (giao với vùng cắt) phải bằng đúng màu che, và điểm ảnh ngay bên ngoài phải giữ nguyên.
- **Màu che lấy theo lưới mẫu cố định.** Dùng cùng một hàm cho bản xem trước và cho màu lưu lúc thả chuột. Màu lúc thả được lưu vào hình, và cả màn hình lẫn ảnh ra đều dùng màu đó.
- Một task duy nhất: phần core quá nhỏ để rà soát riêng, giá trị nằm ở lớp phủ.

## Cần đọc

- Thiết kế trên.
- `crates/darkshot-core/src/annotate.rs`: `Tool`, `Tool::ALL`, `Tool::from_key`, `Shape`, `Shape::bounds`, `bounds_of`, `too_short`, `constrain_square`, kiểm thử.
- `crates/darkshot-core/src/image.rs` (`Image`, `crop`); `crates/darkshot-core/src/i18n.rs` (`Msg::ToolMarker`, bảng kiểm thử hai ngôn ngữ).
- `crates/darkshot-windows/src/annotate_draw.rs`: `draw_shapes`, `draw_one`, `shape_bounds`, `burn`, kiểm thử in ảnh.
- `crates/darkshot-windows/src/overlay.rs`: `SideItem`, `SIDE_ITEMS`, `COLOR_INDEX`, `seps_before`, `side_bar_size`, `begin_stroke`, `extend_stroke`, `end_stroke`, `shift_changed`, `side_icon`, con trỏ theo công cụ, kiểm thử vị trí nút (`at(100 + k * 36 + 18)`), kiểm thử `live::*`.
- `crates/darkshot-windows/src/main.rs`: `picked_image`.

## Interfaces

```rust
// crates/darkshot-core/src/annotate.rs
pub enum Tool { Pen, Line, Arrow, Rect, Marker, Blur, Text }
// Tool::ALL: [Pen, Line, Arrow, Rect, Marker, Blur, Text]  (7 phần tử)
// Tool::from_key: thêm 0x42 ('B') => Blur

pub enum Shape { /* … như cũ … */ Blur { a: Pt, b: Pt, color: Rgb } }

/// Hình chữ nhật điểm ảnh của vùng che: min làm tròn xuống (floor), max làm tròn lên (ceil).
pub fn blur_pixel_rect(a: Pt, b: Pt) -> Rect;

/// Số mẫu tối đa khi tính màu che.
pub const REDACT_MAX_SAMPLES: u64 = 65_536;

/// Bước lưới mẫu cho vùng w×h: số nguyên nhỏ nhất s ≥ 1 sao cho ceil(w/s)·ceil(h/s) ≤ REDACT_MAX_SAMPLES.
pub fn redact_step(w: i32, h: i32) -> i32;

/// Trung bình cộng B, G, R (làm tròn gần nhất) của các điểm ảnh (x, y) với
/// x = r'.x + i·s, y = r'.y + j·s nằm trong r', trong đó r' = r ∩ ảnh và s = redact_step(r'.w, r'.h).
/// None khi r' rỗng hoặc ảnh không hợp lệ.
pub fn redact_color(img: &Image, r: Rect) -> Option<Rgb>;
```

- `Shape::Blur.bounds()` = `blur_pixel_rect(a, b)` nới 1 px mỗi phía.
- `too_short(Tool::Blur, a, b)` giống Rect. Khi giữ Shift, `constrain_square` áp cho Blur giống Rect.
- i18n: `Msg::ToolBlur` → "Làm mờ (B)" / "Blur (B)".

Lớp Windows:

- `draw_one`: `Shape::Blur` tô đặc `blur_pixel_rect` bằng `color`, alpha 255, không khử răng cưa. Trả lại chế độ vẽ cũ của `Graphics` sau khi vẽ để các hình sau không bị ảnh hưởng.
- `SIDE_ITEMS` có 9 mục: Pen, Line, Arrow, Rect, Marker, **Blur**, Text, Color, Undo.
  - `COLOR_INDEX = 7`.
  - `seps_before`: `0..=6 => 0`, `7 => 1`, còn lại `2`.
  - Tip của Blur là `Msg::ToolBlur`.
- Biểu tượng: hình chữ nhật (như Rect) có ba vạch chéo song song bên trong, cắt trong khung. Cùng nét và lưới 24 với các biểu tượng khác.
- Con trỏ khi chọn Blur: chữ thập, như các công cụ vẽ khác.
- Nét đang kéo (`begin_stroke`/`extend_stroke`/`shift_changed`):
  - Mỗi khi `b` đổi, gán `color = redact_color(&shot.image, blur_pixel_rect(a, b))`.
  - Nếu kết quả là `None` thì giữ màu cũ. Màu ban đầu là màu của điểm ảnh tại `p`; nếu không lấy được thì `Rgb(128,128,128)`.
  - Vùng làm mới là hợp của bounds cũ và mới, như Rect.
- `end_stroke`: quá ngắn thì không tạo hình. Nếu không, tính lại `color` một lần nữa bằng `redact_color` trên hình chữ nhật cuối cùng rồi chốt hình.
- Màu đang chọn ở bảng màu không dùng cho Blur. Đổi màu khi đang chọn Blur không ảnh hưởng gì.
- `main.rs` và `burn` không đổi gì: Blur đi qua đúng đường in hiện có.

## Review Focus

1. **Vùng che vượt ra ngoài ảnh hoặc ngoài vùng chọn** (kéo ra mép màn hình ảo, hoặc vùng chọn bị thu nhỏ sau khi che). Mong đợi:
   - Màu tính trên phần nằm trong ảnh, không panic.
   - Phần ngoài vùng chọn không in ra.
   - Phần còn trong vùng chọn vẫn che kín.
2. **Vùng che rất lớn** (kéo hết hai màn hình 4K): bản xem trước vẫn mượt. Số mẫu không bao giờ vượt `REDACT_MAX_SAMPLES`.
3. **Nét vẽ trước bị che sau**, và Ctrl+Z:
   - Vùng che phủ lên nét bên dưới.
   - Ctrl+Z gỡ vùng che và hiện lại ảnh gốc trên màn hình (đúng như mong đợi). Ảnh ra không còn vùng che đó.
4. **Màu che không bị ảnh hưởng bởi nét vẽ.** Che lên chỗ đã có nét vẽ thì màu vẫn tính từ ảnh chụp gốc, không từ nét vẽ.
5. **Thanh dọc 9 nút** ở DPI 100% và 150%, trên màn hình thấp (ví dụ 1366×768):
   - Thanh vẫn nằm trọn trong màn hình.
   - Không chồng thanh bốn nút.
   - Thứ tự và chú thích đúng.

## Các bước

- [ ] **1. Kiểm thử trước** (commit `test: blur redact tool`, thân hàm mới `todo!()`):
  - Core `blur_pixel_rect_rounds_outward`:
    - `(1.2,2.7)–(5.1,3.0)` → `Rect{x:1,y:2,w:5,h:1}`.
    - Hai điểm đảo thứ tự cho cùng kết quả.
  - Core `redact_step_caps_samples`:
    - `redact_step(256,256) == 1`; `redact_step(257,256) == 2`.
    - Với (7680, 2160), (1, 10_000_000) và (3840, 2160): `ceil(w/s)*ceil(h/s) <= 65_536` và `s-1` không thỏa (khi `s > 1`).
  - Core `redact_color_average`:
    - Ảnh 4×2, nửa trái đen, nửa phải trắng: cả ảnh → `Rgb(128,128,128)` (làm tròn 127,5 lên).
    - Chỉ nửa trái → đen.
    - Ảnh một màu `#336699` → đúng `#336699`.
    - Vùng tràn ra ngoài ảnh → bằng vùng đã cắt.
    - Vùng hoàn toàn ngoài ảnh → `None`.
    - Vùng 1×1 → đúng điểm ảnh đó.
  - Core `tool_blur_key_order`: `Tool::from_key(0x42) == Some(Tool::Blur)`; `Tool::ALL[5] == Tool::Blur`; `too_short(Tool::Blur, …)` giống Rect.
  - Core `blur_bounds`: `Shape::Blur` từ `(10.5,10.5)` tới `(20.2,30.0)` → bounds `Rect{x:9,y:9,w:13,h:22}`.
  - Core i18n: `Msg::ToolBlur` đúng chữ hai bản ở bảng kiểm thử hai ngôn ngữ.
  - Windows `burn_blur_covers_every_pixel`:
    - Ảnh 64×48 có hoa văn caro đen trắng 1 px (giả chữ). In một `Shape::Blur` `(8.4,6.6)–(40.2,30.9)` với màu `#5A6B7C`.
    - Mọi điểm ảnh trong `blur_pixel_rect` (8,6,33,25) bằng đúng `#5A6B7C` với alpha 255.
    - Mọi điểm ảnh ngoài hình chữ nhật đó giữ nguyên từng byte.
  - Windows `burn_blur_then_pen_order`: Pen vẽ trước rồi Blur → điểm của Pen trong vùng che bị phủ. Blur trước rồi Pen → nét Pen nằm trên.
  - Windows `burn_no_shapes_unchanged`: nếu đã có kiểm thử tương đương thì giữ nguyên; nếu chưa thì thêm (ảnh không hình → đúng từng byte).
  - Overlay (thuần): cập nhật kiểm thử vị trí nút cho 9 mục.
    - `at(100 + 5*36 + 18) == Some(SideItem::Tool(Tool::Blur))`; Text ở chỉ số 6.
    - Màu ở chỉ số 7, sau một vạch ngăn; Hoàn tác sau hai vạch.
    - Kiểm thử đặt thanh hiện có vẫn qua với chiều cao mới; thêm trường hợp màn hình 1366×768 ở 150%.
  - Overlay `live::blur_save_redacts` (`#[ignore]`, chạy với `--test-threads=1`):
    - Chọn vùng, nhấn `B`, kéo một hình chữ nhật trong vùng, Lưu.
    - Mọi điểm ảnh của vùng che trong ảnh ra bằng nhau và bằng `redact_color` của ảnh chụp trên hình chữ nhật đó.
    - Thêm vào vòng chống rò GDI hiện có một lần kéo Blur.
- [ ] **2. Mã** theo "Interfaces".
- [ ] **3. Tài liệu.**
  - README: bảng công cụ thêm dòng `| Làm mờ | B | Tô kín vùng kéo bằng một màu trung bình của ảnh; không khôi phục được chữ bên dưới. |` ngay sau Bút dạ quang.
  - `memory-bank/decisions.md` thêm quyết định số kế tiếp:
    - Công cụ Làm mờ là "kính mờ giả": một màu trung bình, lấy mẫu tối đa 65.536 điểm, cạnh sắc, không khử răng cưa, phím B, nằm giữa Bút dạ quang và Chữ.
    - Lý do: Gaussian và pixelate khôi phục được. Chủ dự án chọn.
    - Ghi kèm mọi lựa chọn riêng của task này.
  - Checklist (file có mục "Đợt 5") thêm các dòng `[ ]`:
    - Che một dòng mật khẩu hoặc email, rồi Lưu, Chép, Tải lên (PNG và JPEG): vùng che là một màu phẳng, không đọc được gì.
    - Nét vẽ sau nằm trên vùng che.
    - Ctrl+Z gỡ vùng che.
    - Thanh 9 nút nằm trọn trong màn hình ở 100% và 150%.
- [ ] **4.** Chạy `cargo fmt --all`. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`. Commit `feat: blur redact tool`.

Quy trình chung:
- Không thêm crate. Core không phụ thuộc `windows`.
- Không `unwrap`/`expect`/chỉ số có thể panic trên đường chạy thật. `unsafe` kèm `// SAFETY:`. Giải phóng mọi đối tượng GDI/GDI+.
- Kiểm `git branch --show-current` trước khi commit. Không chuyển nhánh, không đẩy.
- Không tắt tiến trình `darkshot` không do mình khởi động. Build release thì dùng `--target-dir target/task42`.
- Không đọc hay ghi khóa R2 hay file cấu hình thật.

**Xong khi:** check qua. Báo cáo nêu:
- commit và dòng cuối của check;
- kết quả kiểm thử chạy tay;
- ảnh chụp lớp phủ có vùng che và thanh 9 nút. Cách chụp như Task 41: thêm tạm `Step::Snap` gọi `capture::capture_virtual_screen` rồi `image::encode_png`, ghi ra `C:/Users/Client/AppData/Local/Temp/claude/9bda5e44761d6d15/b42772f7-7c2f-4a6b-bb6f-a48e834c0eaf/scratchpad/t42_*.png`. Không commit phần tạm;
- chỗ khác gói task, và những gì chưa nhìn bằng mắt.
