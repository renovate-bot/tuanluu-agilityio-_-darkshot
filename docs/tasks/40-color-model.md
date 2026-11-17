# Task 40: Mô hình màu tự chọn ở core

**Agent:** `builder` · **Model:** sonnet · **Nhánh:** `task/40-color-model` · **Rà soát:** `reviewer`

Thuộc thiết kế `docs/superpowers/specs/2026-11-17-custom-colors-design.md` (đọc toàn bộ, ngắn). Ràng buộc chung: mục "Global Constraints" của `docs/superpowers/plans/2026-11-17-annotate.md`. Task 41 làm giao diện dựa trên task này.

## Cần đọc

- Thiết kế trên; `crates/darkshot-core/src/{annotate.rs (PALETTE, PALETTE_COLS), theme.rs (Rgb), config.rs (Config, các struct File*, load, save, kiểm thử định dạng file), i18n.rs}`

## Interfaces (Produces, Task 41 dùng nguyên văn)

```rust
// crates/darkshot-core/src/color.rs   (pub mod color)
pub const HLS_MAX: u16 = 240;
pub const HUE_MAX: u16 = 239;   // hiển thị
pub const RGB_MAX: u16 = 255;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hls { pub h: u16, pub l: u16, pub s: u16 }   // h 0..=239, l,s 0..=240
pub fn rgb_to_hls(c: Rgb) -> Hls;                       // theo ColorRGBToHLS của Windows (xám: h = 160, s = 0); h = 240 đổi thành 0
pub fn hls_to_rgb(v: Hls) -> Rgb;                       // theo ColorHLSToRGB của Windows
pub const CUSTOM_SLOTS: usize = 16;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CustomColors { pub slots: [Option<Rgb>; CUSTOM_SLOTS] }
impl CustomColors {
    /// Ghi `c` vào `target` nếu có, không thì ô trống đầu tiên, hết ô trống thì ô `next_overwrite`
    /// (vòng tròn, bắt đầu 0, tăng sau mỗi lần ghi đè). Trả chỉ số đã ghi.
    pub fn add(&mut self, c: Rgb, target: Option<usize>) -> usize;
}
pub fn parse_hex(s: &str) -> Option<Rgb>;   // "#RRGGBB" (hoa hoặc thường), khác thì None
pub fn to_hex(c: Rgb) -> String;            // "#RRGGBB" chữ hoa

/// Bố cục bảng chọn màu (tọa độ tương đối góc trên trái bảng, pixel), `k` = dpi/96.
pub struct PickerLayout {
    pub size: (i32, i32),
    pub basic_label: Rect, pub basic: [Rect; 48],
    pub custom_label: Rect, pub custom: [Rect; CUSTOM_SLOTS],
    pub define_btn: Rect,
    // chỉ có nghĩa khi expanded:
    pub spectrum: Rect, pub lum_bar: Rect, pub lum_arrow_x: i32, pub preview: Rect,
    pub fields: [(Rect, Rect); 6],    // (nhãn, ô) theo thứ tự Hue, Sat, Lum, Red, Green, Blue
    pub add_btn: Rect, pub ok_btn: Rect, pub cancel_btn: Rect,
}
pub fn picker_layout(dpi: u32, expanded: bool) -> PickerLayout;   // số đo theo thiết kế mục "Nhìn"
pub fn spectrum_hs(spectrum: Rect, x: i32, y: i32) -> (u16, u16);  // điểm (kẹp vào ô) → (hue 0..=239, sat 240..=0)
pub fn spectrum_point(spectrum: Rect, h: u16, s: u16) -> (i32, i32);
pub fn lum_at(bar: Rect, y: i32) -> u16;                          // trên = 240, dưới = 0, kẹp
pub fn lum_y(bar: Rect, l: u16) -> i32;
```

`Config` thêm `pub custom_colors: CustomColors` (mặc định trống). File: trường `custom_colors`, mảng 16 phần tử, mỗi phần tử `"#RRGGBB"` hoặc `null`; thiếu trường, mảng ngắn hơn, phần tử sai định dạng → ô trống; dài hơn → bỏ phần thừa. Trường mới phải được giữ khi `save` rồi `load`.

i18n: các `Msg` trong mục "Chữ" của thiết kế, đúng chữ.

## Các bước

- [ ] **1. Kiểm thử trước** (`test: custom colour model`, thân hàm `todo!()`):
  - `windows_hls_reference`: đỏ (255,0,0) → (h0,l120,s240); lục (0,255,0) → (80,120,240); lam (0,0,255) → (160,120,240); trắng → l240 s0 h160; đen → l0 s0 h160; xám (128,128,128) → (160,120,0)±1 ở l; vàng (255,255,0) → (40,120,240); (0x80,0x40,0x40) → giá trị tính theo thuật toán Windows (ghi kết quả tính tay trong chú thích).
  - `hls_round_trip`: với mọi màu trong `PALETTE` và 1000 màu giả ngẫu nhiên cố định hạt, `hls_to_rgb(rgb_to_hls(c))` lệch mỗi kênh ≤ 2 (đúng mức lệch của Windows).
  - `hls_to_rgb_reference`: (0,120,240) → đỏ; (160,240,0) → trắng; (80,60,240) → (0,128,0)±1.
  - `custom_add_rules`: ghi vào ô đích; không đích → ô trống đầu; đầy → ghi đè 0, rồi 1 …, quay lại 0 sau 15.
  - `hex_parse_format`: "#ff3b30" và "#FF3B30" → đúng; "FF3B30", "#FFF", "#GG0000", "" → None; `to_hex` chữ hoa.
  - `picker_layout_fits_and_disjoint`: ở 96 và 144 DPI, gọn và đủ: mọi ô nằm trong `size`; 48 + 16 ô không chồng nhau và không chồng nút; dạng đủ rộng hơn dạng gọn, cùng chiều cao hoặc cao hơn; các ô số không chồng nhau.
  - `spectrum_mapping`: góc trên trái → (0,240), góc dưới phải → (239,0); `spectrum_point(spectrum_hs(p))` trở lại p ±1; điểm ngoài kẹp vào ô. `lum_at`/`lum_y` tương tự.
  - `config_custom_colors_round_trip`: config có vài ô → save → load trả đúng; file không có trường → trống; mảng có `"xyz"` và 20 phần tử → ô đó trống, bỏ phần thừa; file cũ mẫu trong kiểm thử config hiện có vẫn load được.
  - `i18n`: các `Msg` mới đúng chữ, hai bản khác nhau.
- [ ] **2. Mã.** `color.rs` (`pub mod color`), sửa `config.rs`, `i18n.rs`. Thuật toán HLS: chép logic của `ColorRGBToHLS`/`ColorHLSToRGB` (công khai, ví dụ trong tài liệu KB29240 của Microsoft) bằng số nguyên như bản gốc, `HLSMAX = 240`, `RGBMAX = 255`, `UNDEFINED = HLSMAX*2/3`.
- [ ] **3.** `cargo fmt --all`; `powershell -File scripts/check.ps1` in `ALL CHECKS PASSED`. Commit `feat: custom colour model`.

Quy trình chung: không thêm crate; core không phụ thuộc `windows`; không `unwrap`/`expect`/chỉ số có thể panic ngoài kiểm thử; kiểm `git branch --show-current` trước khi commit, không chuyển nhánh, không đẩy; không đọc hay ghi khóa R2 hay file cấu hình thật (kiểm thử dùng thư mục tạm).

**Xong khi:** check qua; báo cáo nêu commit, dòng cuối check, giá trị tham chiếu Windows đã dùng và nguồn, chỗ khác thiết kế.
