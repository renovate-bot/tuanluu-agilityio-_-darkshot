# Task 45: Core – trúng hình, tay nắm, biến đổi, đổi kiểu, bố cục thanh dọc

**Agent:** `builder` · **Model:** sonnet · **Nhánh:** `task/45-pro-core-edit` · **Rà soát:** `specialist`

Đợt 6. Thiết kế: `docs/superpowers/specs/2026-11-17-annotation-pro-design.md` (đọc mục 3, 5, 7, 11). Kế hoạch: `docs/superpowers/plans/2026-11-17-annotation-pro.md`. Dựa trên Task 44 (đã ở `main`).

## Global Constraints

- Không thêm crate, kể cả `dev-dependencies`. `darkshot-core` không phụ thuộc crate `windows` hay API riêng của hệ điều hành.
- Làm mờ: màu che luôn tính trên ảnh chụp gốc (`redact_color`), cả khi xem trước lúc kéo lẫn khi chốt.
- Release `panic = "abort"`: không `unwrap`/`expect`/chỉ số có thể panic; số thực NaN/vô cực không được gây panic (dùng phép chia có kiểm mẫu số 0).
- Không sửa spec, kế hoạch đã duyệt, gói task cũ, `prototype/`.
- Quy trình: commit kiểm thử trước (`test:`, thân `todo!()`), rồi `feat:`. `cargo fmt --all`. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`. Kiểm `git branch --show-current`; không chuyển nhánh, không đẩy. Chạy cargo qua PowerShell.

## Cần đọc

- `crates/darkshot-core/src/annotate.rs`: `Shape`, `Shape::bounds`, `blur_pixel_rect`, `redact_color`, `constrain_46`, `constrain_square`, `arrow_geometry`, `arrow_min_head`, `dip_to_px`, `SizeLevel`, các hằng mới của Task 44.
- `crates/darkshot-core/src/geometry.rs`: `Rect`, `Handle`, `place_side_toolbar`.
- `crates/darkshot-core/src/image.rs`: `Image` (dựng ảnh thử).
- `crates/darkshot-core/src/lib.rs` (khai báo module).

## Files

- Tạo: `crates/darkshot-core/src/edit.rs` (`pub mod edit` trong `lib.rs`).
- Sửa: `crates/darkshot-core/src/geometry.rs`, `crates/darkshot-core/src/lib.rs`.

## Interfaces

```rust
// crates/darkshot-core/src/edit.rs
use crate::annotate::{Pt, Shape, SizeLevel};
use crate::geometry::{Handle, Rect};
use crate::image::Image;
use crate::theme::Rgb;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grip { Move, Box(Handle), End(usize) }   // End(0) = a, End(1) = b

#[derive(Clone, Debug, PartialEq)]
pub struct Gesture {
    pub index: usize,
    pub grip: Grip,
    pub start: Pt,              // điểm bấm
    pub original: Shape,
    pub original_bounds: Rect,  // mark_bounds lúc bấm (Chữ cần)
}

/// `p` trúng hình (mục 5.3 thiết kế). `bounds`: hộp bao đo của hình (dùng cho Text). `tol` px.
pub fn hit_shape(shape: &Shape, bounds: Rect, p: Pt, tol: f32) -> bool;
/// Vị trí hình trên cùng trúng `p` (duyệt từ cuối); bỏ nhanh hình có bounds nới tol không chứa p.
/// `bounds` song song với `shapes` (thiếu phần tử thì coi hình đó không trúng).
pub fn topmost_hit(shapes: &[Shape], bounds: &[Rect], p: Pt, tol: f32) -> Option<usize>;
/// Tay nắm và vị trí tâm của chúng.
/// Line/Arrow: [End(0) tại a, End(1) tại b].
/// Rect/Ellipse/Blur: 8 Box(..) trên hộp chuẩn hóa của a–b (NW, N, NE, E, SE, S, SW, W).
/// Pen/Marker: 4 góc (NW, NE, SE, SW) của hộp các điểm.
/// Text: 4 góc của `bounds`. Step: 4 góc hộp center ± radius.
pub fn handles(shape: &Shape, bounds: Rect) -> Vec<(Grip, Pt)>;
/// Tay nắm có tâm cách p (theo max |dx|, |dy|) ≤ radius; nhiều cái thì cái gần nhất.
pub fn grip_at(shape: &Shape, bounds: Rect, p: Pt, radius: f32) -> Option<Grip>;
/// Khung chọn (bounds nới 2 px); None với Line và Arrow.
pub fn frame(shape: &Shape, bounds: Rect) -> Option<Rect>;
/// Hình xem trước / chốt của cử chỉ `g` khi chuột ở `to` (đã kẹp vào vùng chọn bởi lớp gọi).
/// `min_extent` px. `img`: ảnh chụp gốc (màu che của Blur; ngoài ảnh thì giữ màu cũ).
pub fn transform(g: &Gesture, to: Pt, shift: bool, min_extent: f32, img: &Image) -> Shape;
/// Đổi kiểu: `color` (Blur bỏ qua), `level` đổi px theo `dpi` (bảng 5.6; Blur bỏ qua;
/// Text đặt size giữ origin; Step đặt radius giữ center). `n` giữ nguyên.
pub fn restyle(shape: &Shape, level: Option<SizeLevel>, color: Option<Rgb>, dpi: u32) -> Shape;

// crates/darkshot-core/src/geometry.rs
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SideLayout {
    pub size: (i32, i32),
    /// Ô của từng mục, tương đối góc trên trái thanh.
    pub items: Vec<Rect>,
    /// Ô vạch ngăn (cao `sep`, rộng `b`), tương đối góc trên trái thanh.
    pub seps: Vec<Rect>,
}
/// `n` mục vuông cạnh `b`; `seps`: chỉ số các mục có vạch ngăn ngay trên (tăng dần, < n); ô vạch cao `sep`.
/// Một cột nếu n·b + seps.len()·sep ≤ max_h; không thì hai cột: cột trái mục 0..k, cột phải k..n,
/// k = ceil(n/2); vạch ngăn ở đầu cột (chỉ số 0 hoặc k) bị bỏ; cao = cột cao hơn; rộng 2b.
pub fn side_layout(n: usize, seps: &[usize], b: i32, sep: i32, max_h: i32) -> SideLayout;
```

### Quy tắc biến đổi (thiết kế 5.5)

- `Grip::Move`: dời mọi điểm của hình bởi `to − start` (Text: origin; Step: center; Blur: a, b rồi tính lại màu).
- `Grip::End(i)` (Line/Arrow): đặt đầu i = `to` (Shift: `constrain_46(đầu kia, to)`); nếu dài < `MIN_DRAG_PX` thì giữ đầu i cũ.
- `Grip::Box(h)` với Rect/Ellipse/Blur: hộp chuẩn hóa x0<x1, y0<y1 của bản gốc; cạnh do h nắm đi theo `to`, cạnh đối đứng yên; kéo qua cạnh đối thì lật; mỗi chiều có |cạnh| ≥ `min_extent` (thiếu thì đặt cạnh động = cạnh đứng yên ± min_extent theo phía của `to`, bằng nhau thì phía cũ). Góc + Shift: hệ số s = max(|dx|/w0, |dy|/h0), áp cùng s cho hai chiều (giữ dấu từng chiều). Kết quả a = (min), b = (max). Blur: màu = `redact_color(img, blur_pixel_rect(a, b))`, `None` thì giữ màu gốc.
- `Grip::Box(góc)` với Pen/Marker: neo = góc đối của hộp các điểm; sx = (to.x − neo.x)/(góc.x − neo.x), chiều có mẫu 0 thì hệ số 1; |sx·w| < min_extent thì sx = dấu·min_extent/w; Shift: s = max(|sx|,|sy|) giữ dấu. Điểm' = neo + (điểm − neo)·(sx, sy). Độ dày giữ nguyên.
- `Grip::Box(góc)` với Text/Step: neo = góc đối (của `original_bounds` với Text, của hộp vòng với Step); c = góc nắm; k = ((to − neo)·(c − neo)) / |c − neo|², mẫu 0 thì k = 1; kẹp k để size trong [TEXT_SIZE_MIN_PX, TEXT_SIZE_MAX_PX] (Step: radius trong [STEP_RADIUS_MIN_PX, STEP_RADIUS_MAX_PX]). Text: origin' = neo + (origin − neo)·k, size' = size·k. Step: center' = neo + (center − neo)·k, radius' = radius·k.
- Grip không hợp loại hình (ví dụ End với Rect) → trả `original` không đổi.

### Trúng hình (thiết kế 5.3)
Pen/Marker: khoảng cách tới đường gấp khúc (một điểm: tới điểm) ≤ width/2 + tol. Line: tới đoạn ≤ width/2 + tol. Arrow: tới đoạn, hoặc trong tam giác đầu (`arrow_geometry`). Rect: tới viền hình chữ nhật ≤ width/2 + tol (trong lòng xa viền: không trúng). Ellipse: |√((dx/rx)²+(dy/ry)²) − 1|·min(rx, ry) ≤ width/2 + tol (rx hoặc ry < 1: coi như đoạn thẳng). Blur: trong `blur_pixel_rect` nới tol. Step: |p − center| ≤ radius + tol. Text: trong `bounds` nới tol.

## Các bước

- [ ] **1. Kiểm thử** (commit `test: annotation edit model`). Ảnh thử: dựng `Image` nhỏ tay (ví dụ 100×100, nửa trái đỏ thuần, nửa phải xanh thuần).
  - `hit_each_kind`: mỗi loại một điểm trúng và một điểm trượt; Rect 10..50 width 4 tol 4: (30,10) trúng, (30,30) trượt; Ellipse a(0,0) b(40,20): (20,0) trúng, (20,10) trượt; Step tâm (50,50) r 10: (58,50) trúng; Blur: điểm trong lòng trúng; Text: theo bounds.
  - `topmost_wins`: hai Blur chồng nhau → trả vị trí lớn hơn; điểm trúng chỉ hình dưới → trả hình dưới; không trúng → None; `bounds` ngắn hơn `shapes` không panic.
  - `handles_per_kind`: Line 2, Rect/Ellipse/Blur 8, Pen/Marker/Text/Step 4; vị trí NW của Rect a(30,40) b(10,20) là (10,20) (chuẩn hóa).
  - `grip_at_picks_nearest`: điểm cách NW 3 px, radius 5 → `Some(Box(NW))`; cách 6 → None.
  - `move_preserves_style`: Move mỗi loại bởi (+7, −3): màu, độ dày, chữ, cỡ, `n` giữ nguyên; điểm dời đúng.
  - `rect_resize_flip_and_min`: Rect a(10,10) b(50,50), Box(SE) tới (5,60) với min 2 → a(5,10) b(10,60); tới (10.5,30) với min 6 → bề rộng = 6; Shift Box(SE) tới (90,60) từ hộp 40×40 → hộp 80×80.
  - `line_end_shift`: End(1) tới (50,3) có Shift từ a(0,0) → b.y == 0; End(1) tới cách a 1 px → giữ b cũ.
  - `pen_scale_keeps_width`: Pen hộp 0..10 × 0..0 (nằm ngang), Box(SE) tới (20,40) → x nhân 2, y không đổi, width không đổi.
  - `text_resize_uniform`: Text origin(10,10) size 20, bounds (10,10,40,20), Box(SE) tới (90,50) → k = 2 (theo công thức), size 40, origin (10,10); k kẹp: kéo về gần neo → size == TEXT_SIZE_MIN_PX.
  - `step_resize_clamped`: tương tự, radius trong [6, 300].
  - `blur_move_recolors_from_original`: Blur màu đỏ nằm nửa trái; Move sang nửa phải → màu xanh; Gesture.original giữ đỏ.
  - `blur_resize_flip_stays_covered`: Box(E) kéo qua cạnh trái → a.x < b.x, `blur_pixel_rect` khác rỗng, màu tính lại; kéo ra ngoài ảnh hoàn toàn → giữ màu gốc.
  - `restyle_rules`: Pen Medium→Large ở dpi 192 → width 10; Marker Small ở 96 → 10; Text Huge ở 96 → size 36, origin giữ; Step Small ở 144 → radius 15; Blur không đổi gì; color áp cho mọi hình trừ Blur.
  - `wrong_grip_returns_original`: End(0) với Rect → bằng original.
  - geometry `side_layout_single_column`: n 14, seps [1,10,12], b 36, sep 9, max_h 1000 → size (36, 531); items[1].y == 36+9; seps.len()==3.
  - geometry `side_layout_wraps_when_short`: cùng tham số, max_h 500 → size (72, 270) (cột trái 7·36 + 9 = 261, cột phải 7·36 + 2·9 = 270); items[7].x == 36, items[7].y == 0; không vạch nào có y < 0; mọi ô nằm trong size.
- [ ] **2. Mã** theo Interfaces và quy tắc.
- [ ] **3.** `cargo fmt --all`; check → `ALL CHECKS PASSED`. Commit `feat: annotation edit model and side layout`.

**Xong khi:** check qua. Báo cáo: commit, số kiểm thử, dòng cuối check, chỗ khác gói.
