# Task 39: Bảng màu 48 ô

**Agent:** `builder` · **Model:** sonnet · **Nhánh:** `task/39-palette-48` · **Rà soát:** `specialist`

Thay đổi nhỏ sau đợt 5. Chủ dự án gửi ảnh hộp chọn màu của Windows và muốn bảng màu đa dạng hơn; họ chọn phương án "48 màu" (2026-11-17): lưới 6 hàng × 8 cột, đúng 48 màu cơ bản của hộp chọn màu Windows, vẽ trong lớp phủ theo giao diện tối (không mở hộp thoại của Windows: lớp phủ tự đóng khi mất tiêu điểm và hộp đó sáng). Không có màu tự chọn, không có hàng màu gần đây. Thiết kế đợt 5 đã duyệt không sửa; ghi quyết định mới vào `memory-bank/decisions.md`.

## Cần đọc

- Kế hoạch đợt 5 `docs/superpowers/plans/2026-11-17-annotate.md`: "Global Constraints"
- `crates/darkshot-core/src/annotate.rs` (`PALETTE`, `DEFAULT_COLOR`, kiểm thử `palette_and_constants_match_plan`)
- `crates/darkshot-windows/src/overlay.rs`: `CELL`, `palette_size`, `palette_cell_rect`, `palette_cell_at`, `palette_rect_for`, `Painter::palette`, `LAST_COLOR`, kiểm thử `palette_hit_testing`, `draw_line_and_save_returns_shape`, `esc_closes_palette_not_overlay`
- `README.md` mục "Đánh dấu ảnh" (đoạn **Màu:**), `docs/testing/manual-checklist.md` mục "Đợt 5"

## Giá trị

`PALETTE: [Rgb; 48]`, theo hàng từ trên xuống, mỗi hàng trái sang phải:

```
#FF8080 #FFFF80 #80FF80 #00FF80 #80FFFF #0080FF #FF80C0 #FF80FF
#FF0000 #FFFF00 #80FF00 #00FF40 #00FFFF #0080C0 #8080C0 #FF00FF
#804040 #FF8040 #00FF00 #008080 #004080 #8080FF #800040 #FF0080
#800000 #FF8000 #008000 #008040 #0000FF #0000A0 #800080 #8000FF
#400000 #804000 #004000 #004040 #000080 #000040 #400040 #400080
#000000 #808000 #808040 #808080 #408080 #C0C0C0 #400040 #FFFFFF
```

(`#400040` có hai lần, giống hộp của Windows; giữ nguyên.) `DEFAULT_COLOR = PALETTE[8]` (`#FF0000`, ô đỏ đang được chọn trong ảnh của chủ dự án). Thêm `pub const PALETTE_COLS: usize = 8;`.

Bố cục: ô 20 DIP, khe và lề 4 DIP (bảng 196 × 148 DIP), nền `SURFACE`, viền `OVERLAY_BORDER`; mỗi ô có viền 1 px `OVERLAY_BORDER` để ô tối (đen, `#000040`) vẫn thấy trên nền tối; ô đang chọn viền `TEXT` 2 DIP như hiện nay. Vị trí bảng giữ cách tính hiện có (cạnh nút Màu, kẹp trong màn hình).

## Global Constraints

- Như "Global Constraints" của kế hoạch đợt 5. Không thêm crate. Không đổi hành vi nào khác của bảng màu (mở/đóng, Esc, cú bấm ngoài, nhớ màu qua `LAST_COLOR`, đổi màu ô chữ đang gõ).
- Không `unwrap`/`expect`/chỉ số có thể panic; lấy màu bằng `PALETTE.get(i)`.
- Commit kiểm thử trước (`test:`), rồi mã (`feat:`). `cargo fmt --all`. Chạy cargo qua PowerShell. Kiểm `git branch --show-current` trước khi commit; không chuyển nhánh. Không tắt tiến trình `darkshot` không do mình khởi động. Không đẩy lên remote. Không đọc hay ghi khóa R2. Kiểm thử `#[ignore]` chạy từng cái với `--test-threads=1`. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.

## Các bước

- [ ] **1. Kiểm thử trước.** Commit `test: 48-colour palette`.
  - `palette_and_constants_match_plan`: 48 màu đúng thứ tự và giá trị như bảng trên; `DEFAULT_COLOR == PALETTE[8] == Rgb::hex(0xFF0000)`; `PALETTE_COLS == 8`.
  - `palette_hit_testing`: bảng 8 × 6; điểm giữa ô i trả i với mọi i trong 0..48 (ô 7 ở góc trên phải, ô 40 ở góc dưới trái, ô 47 góc dưới phải); khe và lề trả `None`; cỡ bảng `palette_size(20, 4) == (196, 148)`.
  - Sửa khẳng định màu trong các kiểm thử overlay đang dùng chỉ số bảng 8 màu (ví dụ chọn ô cam) cho khớp chỉ số mới, giữ ý nghĩa.
- [ ] **2. Mã** theo "Giá trị".
- [ ] **3. Tài liệu.** README đoạn **Màu:** đổi thành bảng 48 màu (6 hàng × 8 cột như hộp chọn màu của Windows, mặc định đỏ `#FF0000`); thêm quyết định mới vào `memory-bank/decisions.md` (số kế tiếp): bảng màu 48 ô theo hộp chọn màu Windows, mặc định `#FF0000`, thay bảng 8 màu của quyết định 55, chủ dự án chọn; không mở hộp thoại Windows vì lớp phủ đóng khi mất tiêu điểm. Checklist mục Đợt 5 thêm một dòng `[ ]`: bảng 48 màu nằm trọn trong màn hình ở DPI 100% và 150%, ô tối vẫn thấy rõ, chọn được từng ô.
- [ ] **4.** Chạy tay `draw_line_and_save_returns_shape`, `esc_closes_palette_not_overlay`, `overlay_round_trip_without_gdi_leaks` (từng cái). `cargo fmt --all`; check qua. Commit `feat: 48-colour annotation palette`.

**Xong khi:** check qua; báo cáo nêu các commit, dòng cuối của check, kết quả ba kiểm thử chạy tay, và ảnh chụp bảng màu nếu chụp được (cách chụp như Task 36: kiểm thử tạm dùng `capture_virtual_screen`, ảnh để trong `C:/Users/Client/AppData/Local/Temp/claude/9bda5e44761d6d15/a55dcc96-f74b-4a96-9ec7-65a267b4934d/scratchpad/`, không commit kiểm thử tạm).
