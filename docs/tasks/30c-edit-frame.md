# Task 30c: Ô nhập chỉ có một viền bo tròn, mịn

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/30c-edit-frame` · **Rà soát:** `specialist`

Sửa lỗi sau Task 30/30b. Chủ dự án báo: "ô textbox dường như đang mixing giữa góc vuông và bo tròn". Thuộc kế hoạch `docs/superpowers/plans/2026-11-16-darkshot-dark-theme.md`.

## Hiện trạng (điều phối viên chụp ở DPI 200%, Windows 11, sau Task 30b)

Ảnh phóng to: `C:\Users\Client\AppData\Local\Temp\claude\9bda5e44761d6d15\a55dcc96-f74b-4a96-9ec7-65a267b4934d\scratchpad\z_edit_left.png` (Thư mục lưu và hộp chọn Ngôn ngữ) và `z_hk_left.png` (ba ô phím tắt). Ảnh đủ cửa sổ: thư mục `30b_after_vi\` cùng chỗ.

- Ô nhập đang bật (Thư mục lưu; ô phím tắt "PrintScreen" đang bật): viền ngoài bo 4 DIP do `paint_edit_frame` (`crates/darkshot-windows/src/settings.rs`) vẽ ở vùng ngoài client, **và thêm một khung vuông 1 px bên trong**, sát mép vùng client. Đó là chỗ "trộn góc vuông với bo tròn".
- Ô bị vô hiệu (hai ô phím tắt tắt, tự vẽ bởi `paint_disabled_edit`) và hộp chọn Ngôn ngữ chỉ có viền bo, không có khung vuông.
- Góc bo của viền ô nhập và hộp chọn vẽ bằng GDI thường nên răng cưa (cùng lỗi Task 30b đã sửa cho ô tích bằng hàm khoảng cách có dấu + `AlphaBlend` trong `theme.rs`).

Cần tìm đúng ai vẽ khung vuông bên trong (ví dụ: chủ đề hiển thị của ô nhập `set_dark_control_theme`/uxtheme vẽ viền trong client hoặc khi `WM_NCPAINT` bị gọi với vùng cập nhật; `WS_BORDER`/`WS_EX_CLIENTEDGE` còn sót; chính ô nhập vẽ khung trong `WM_PAINT`; `ExcludeClipRect` lệch một pixel; `edit_inset` không khớp `round_rect`...). Xác nhận nguyên nhân bằng thử nghiệm trước khi sửa (superpowers:systematic-debugging).

Prototype: `.superpowers/brainstorm/1649-1794846769/content/darkshot-theme.html` CSS `.in` (dòng ~60): nền `INPUT`, viền 1 px `INPUT_BORDER`, bo 4 px, mép dưới đậm hơn; có tiêu điểm thì gạch dưới 2 px `ACCENT`. Chỉ một viền, mọi góc bo đều mịn.

## Global Constraints

- Mục "Global Constraints" của kế hoạch đợt 4 vẫn áp dụng. Không thêm crate, không thêm feature nếu không cần.
- Kết quả: mọi ô nhập (thường, bí mật `ES_PASSWORD`, ô phím tắt), bật hay vô hiệu, có tiêu điểm hay không, chỉ có **một** viền bo 4 DIP mịn, không khung vuông nào bên trong; hộp chọn cũng bo mịn. Gạch dưới `ACCENT` khi có tiêu điểm/đang bắt phím giữ như cũ và không lòi ra ngoài góc bo.
- Dùng lại cách vẽ mịn của Task 30b trong `theme.rs` (tách hàm dùng chung nếu cần), không viết cách thứ hai. Lỗi thì rơi về GDI cũ, không panic.
- Không đổi bố cục, vị trí chữ trong ô, hành vi bàn phím, chọn chữ, con trỏ nháy, trợ năng. Ô `ES_PASSWORD` không bao giờ tự vẽ chữ.
- Giải phóng mọi DC/đối tượng trên mọi đường đi; `options_no_gdi_leak` vốn chập chờn từ trước (xem `memory-bank/activeContext.md`), không làm nó tệ hơn; chạy kiểm thử ignored từng cái một với `--test-threads=1`.
- Release `panic = "abort"`: không `unwrap`/`expect` trên đường chạy thật. `unsafe` nhỏ, kèm `// SAFETY:`.
- Commit kiểm thử trước (`test:`) cho hàm thuần mới nếu có, rồi `fix:`. `cargo fmt --all`. Chạy cargo qua PowerShell. Kiểm `git branch --show-current` trước khi commit; không chuyển nhánh. Không tắt tiến trình `darkshot` không do mình khởi động. Không đẩy lên remote. Không đọc hay ghi khóa R2. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.

## Tự kiểm bằng mắt

Như Task 30b: kiểm thử tạm `#[ignore]` trong `settings.rs` mở `show(...)`, `click_tab` 0..3, gọi `powershell -File C:/Users/Client/AppData/Local/Temp/claude/9bda5e44761d6d15/a55dcc96-f74b-4a96-9ec7-65a267b4934d/scratchpad/cap.ps1 -ProcId <pid> -Out <scratch>/30c_<tên>/tabN`. Chụp thêm trạng thái có tiêu điểm (đặt `SetFocus` vào ô Thư mục lưu, tab R2 các ô nhập và ô bí mật). Không commit kiểm thử tạm. Ảnh để trong scratch.

## Các bước

- [ ] 1. Chụp hiện trạng; tìm và xác nhận nguồn của khung vuông bên trong.
- [ ] 2. Kiểm thử cho hàm thuần mới nếu có; commit `test: edit frame`.
- [ ] 3. Sửa: bỏ khung vuông; viền ô nhập và hộp chọn vẽ mịn.
- [ ] 4. Chụp lại (bốn tab, có và không tiêu điểm, Tiếng Việt và English), phóng to góc ô; chạy `options_no_gdi_leak` và `options_keyboard_still_works` từng cái.
- [ ] 5. Thêm một dòng `[ ]` vào mục Đợt 4 của `docs/testing/manual-checklist.md`: ô nhập và hộp chọn chỉ một viền bo mịn, không khung vuông bên trong, khi bật, vô hiệu, có tiêu điểm. `cargo fmt --all`; check qua; commit `fix: single smooth rounded frame for text boxes`.

**Xong khi:** check qua; báo cáo nêu nguyên nhân đã xác nhận, các commit, dòng cuối của check, kết quả hai kiểm thử ignored, đường dẫn ảnh trước/sau và những gì chưa nhìn bằng mắt.
