# Task 63: Bắt cửa sổ – ô tích "Gợi ý cửa sổ khi chọn vùng" trong Tùy chọn

**Agent:** `builder` · **Model:** sonnet (mẫu "thêm một ô" đã có) · **Nhánh:** `task/63-snap-options` (tách từ `feature/window-snap`) · **Rà soát:** `reviewer` (sonnet)

Thiết kế: `docs/superpowers/specs/2026-11-18-window-snap-design.md` mục 3.6. Global Constraints: mục cùng tên của `docs/superpowers/plans/2026-11-18-window-snap.md`. Task 60–62 đã ở nhánh phát triển (`Config.snap_windows`, `Msg::ChkSnapWindows`).

## Cần đọc
`memory-bank/systemPatterns.md` mục "Thêm một ô hoặc một tab"; `crates/darkshot-windows/src/settings.rs`: mọi chỗ của `ChkKeepSelection`/`ChkIncludeCursor`/`keep_selection`/`include_cursor` (ID, `FormValues`, dựng form từ config, đọc form về config, bố cục tab Chụp, thứ tự Tab, kiểm thử bố cục và kiểm thử "không cắt chữ" nếu có), `history_enabled` làm mẫu ô gần nhất.

## Files
Sửa `darkshot-windows/src/settings.rs` (và `main.rs` chỉ nếu cần áp giá trị mới ngay sau khi lưu – không cần vì `capture_flow` đọc config mỗi lần chụp).

## Yêu cầu
- Ô tích mới ngay dưới "Chụp cả con trỏ" (hoặc cuối nhóm chụp của tab Chụp), ID mới chưa dùng (tìm ID lớn nhất hiện có), chữ `Msg::ChkSnapWindows`, trạng thái từ `config.snap_windows`, lưu về `snap_windows`.
- Thứ tự Tab đi qua ô mới đúng vị trí; phím tắt chữ gạch dưới (nếu các ô khác có) không trùng.
- Bố cục tab Chụp cao thêm đúng một hàng; cửa sổ Tùy chọn không bị cắt ở DPI 100–250%, Việt và English (dùng hàm đo/bố cục sẵn có; nếu có kiểm thử bố cục theo chữ dài nhất, thêm chữ mới vào).
- Vẽ tối như các ô tích khác (NM_CUSTOMDRAW sẵn có tự áp nếu ô được tạo qua builder chung).

## Kiểm thử (viết trước)
- Form ↔ config: `snap_windows` true/false đi vòng không đổi; lưu Tùy chọn không đổi các trường khác.
- Bố cục: tab Chụp có ô mới, các ô không chồng nhau, đáy ô cuối ≤ đáy khung tab ở cả hai ngôn ngữ (theo kiểu kiểm thử bố cục có sẵn).
- Chạy tay có sẵn `options_no_gdi_leak`, `options_keyboard_still_works` phải qua (`cargo test -p darkshot-windows -- --ignored --test-threads=1 <tên>`); ghi kết quả thật.

## Báo cáo
Theo mẫu `.claude/agents/builder.md`; kèm ảnh chụp cửa sổ Tùy chọn tab Chụp nếu tự chụp được (lưu vào `target/task63/`, không commit).
