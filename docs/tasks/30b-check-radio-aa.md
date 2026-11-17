# Task 30b: Ô tích và nút chọn vẽ mịn như prototype

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/30b-check-radio-aa` · **Rà soát:** `specialist`

Sửa lỗi sau Task 30, chủ dự án báo "checkbox và radio button không như mong đợi". Thuộc kế hoạch `docs/superpowers/plans/2026-11-16-darkshot-dark-theme.md`.

## Hiện trạng (điều phối viên chụp ở DPI 200%, Windows 11)

`theme::draw_check` (`crates/darkshot-windows/src/theme.rs`) vẽ bằng GDI thường (`ellipse`, `round_rect`, `polyline`), không khử răng cưa:

- Vòng nút chọn răng cưa rõ, có một điểm lồi ở đỉnh và ở cạnh trái của vòng (lỗi của `Ellipse` với bút dày).
- Góc bo của ô tích bị bậc thang; dấu tích thô, nét cạnh răng cưa.
- Khoảng hộp tới chữ 6 DIP, prototype 10 DIP.

Prototype: `.superpowers/brainstorm/1649-1794846769/content/darkshot-theme.html` (CSS `.chk`, `.box`, `.radio`, dòng 70–76): hộp 18×18 bo 4, viền 1,5 px `#8A9099`; bật thì nền và viền `ACCENT_BUTTON`, dấu tích trắng nét 2 px (chữ L xoay 45°: nét ngắn trái, nét dài phải); nút chọn tròn, bật thì viền `ACCENT` nền trong suốt, chấm `ACCENT` 8 px ở giữa; khoảng tới chữ 10 px. Mọi hình đều khử răng cưa.

## Global Constraints

- Mục "Global Constraints" của kế hoạch đợt 4 vẫn áp dụng. Không thêm crate. Feature `Win32_Graphics_GdiPlus` đã bật (có thể dùng GDI+ flat API: `GdipCreateFromHDC`, `GdipSetSmoothingMode(SmoothingModeAntiAlias8x8 hoặc AntiAlias)`, `GdipSetPixelOffsetMode(Half)`, path bo góc, `GdipFillEllipse`...). Xem `overlay.rs` có sẵn khởi tạo GDI+ (`GdiplusStartup`) chưa; dùng chung, không khởi tạo hai lần. Cách khác chấp nhận được: vẽ siêu lấy mẫu rồi thu nhỏ có lọc. Không dùng Direct2D.
- Chỉ đổi phần nhìn của ô tích và nút chọn (và núm thanh trượt nếu nó dùng cùng hàm vẽ tròn và cũng răng cưa). Màu giữ theo `check_colors`/`radio_colors`; bảng màu, bố cục, hành vi bàn phím, trợ năng, `NM_CUSTOMDRAW` + `CDRF_SKIPDEFAULT` giữ nguyên.
- Kích thước theo DIP, nhân DPI: hộp 18, bo 4, viền 1,5, dấu tích nét 2, chấm 8, khoảng chữ 10. Chữ không được bị cắt ở Tiếng Việt và English, DPI 100%/150%/200%; nếu 10 DIP làm cắt chữ thì giữ 6 và ghi lý do.
- Mọi đối tượng GDI/GDI+ tạo ra phải được giải phóng trên mọi đường đi. Kiểm thử `options_no_gdi_leak` (ignored) phải qua.
- Release `panic = "abort"`: không `unwrap`/`expect` trên đường chạy thật; GDI+ lỗi thì rơi về cách vẽ GDI cũ, không panic. `unsafe` nhỏ, kèm `// SAFETY:`.
- Commit kiểm thử trước (`test:`) nếu có hàm thuần mới (ví dụ hình học dấu tích theo kích thước), rồi `fix:`. `cargo fmt --all`. Chạy cargo qua PowerShell. Kiểm `git branch --show-current` trước khi commit; không chuyển nhánh. Không tắt tiến trình `darkshot` không do mình khởi động (chủ dự án đang chạy `target/release/darkshot.exe`; nếu exe bị khóa thì bỏ build release và báo lại). Không đẩy lên remote. Không đọc hay ghi khóa R2. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.

## Tự kiểm bằng mắt

Điều phối viên đã chụp được bằng cách: thêm tạm một kiểm thử `#[ignore]` trong `settings.rs` mở `show(...)` như `options_no_gdi_leak`, ở luồng phụ lần lượt `click_tab` 0..3, mỗi lần gọi `powershell -File <scratch>/cap.ps1 -ProcId <pid> -Out <scratch>/tabN` (script `PrintWindow`, có sẵn tại `C:/Users/Client/AppData/Local/Temp/claude/9bda5e44761d6d15/a55dcc96-f74b-4a96-9ec7-65a267b4934d/scratchpad/cap.ps1`), rồi đóng cửa sổ. Làm như vậy trước và sau khi sửa, phóng to vùng ô tích (tab 0, 1) và nút chọn (tab 2) để so với prototype. Không commit kiểm thử tạm này. Ảnh để trong thư mục scratch, không đưa vào repo.

## Các bước

- [ ] 1. Chụp hiện trạng; đọc `draw_check`, `ellipse`, `round_rect`, `polyline`, `buffered`.
- [ ] 2. Kiểm thử cho hàm thuần mới nếu có. Commit `test: smooth check and radio geometry`.
- [ ] 3. Vẽ hộp, dấu tích, vòng, chấm có khử răng cưa; khoảng chữ 10 DIP. Khung tiêu điểm giữ như cũ.
- [ ] 4. Chụp lại, so prototype; chạy `cargo test -p darkshot-windows -- --ignored options_no_gdi_leak options_keyboard_still_works`.
- [ ] 5. Thêm vào `docs/testing/manual-checklist.md` mục Đợt 4 một dòng `[ ]`: ô tích và nút chọn mịn, không răng cưa ở DPI 100% và 150%. `cargo fmt --all`; check qua. Commit `fix: anti-aliased check boxes and radio buttons`.

**Xong khi:** check qua; báo cáo nêu các commit, dòng cuối của check, kết quả hai kiểm thử ignored, đường dẫn ảnh trước/sau, khoảng chữ cuối cùng và những gì chưa nhìn bằng mắt (DPI nào, ngôn ngữ nào).
