# Task 139: Tùy chọn — dải tab có khoảng cách

**Agent:** `builder` · **Model:** sonnet (đổi bố cục một điều khiển tự vẽ, rủi ro thấp) · **Nhánh:** `task/139-options-tab-spacing` (từ `main`), worktree `D:\2026 - Tuan\shotme-139` · **Rà soát:** `reviewer` (opus; không dùng fable, quyết định 154)

Thiết kế liên quan: `docs/superpowers/specs/2026-11-16-options-tabs-i18n-design.md` (Tùy chọn dạng tab), `docs/superpowers/specs/2026-11-16-darkshot-dark-theme-design.md` (tab tự vẽ). Không sửa thiết kế đã duyệt; ghi thay đổi thành quyết định mới.

## Lỗi (chủ dự án báo, 2026-11-22)

"UI của các tab trên Tùy chọn đang liền nhau." Ảnh PrintWindow của phiên điều phối (150 % DPI): chữ các tab "Thông thường  Phím tắt  Định dạng  Lưu trữ R2" / "General  Hotkeys  Format  R2 storage" cách nhau chỉ ~12–20 px; tab đang chọn in đậm nên chữ đậm gần chạm tab kế bên. Nguyên nhân: dải tab (`SysTabControl32`, `settings.rs` ~2778) dùng đệm ngang mặc định của hệ thống (6 px mỗi bên) và đo chiều rộng mục bằng phông thường, còn `paint_tabs` (~997) vẽ tab chọn bằng phông đậm.

## Files

`crates/darkshot-windows/src/settings.rs` (tạo dải tab, `paint_tabs`, kiểm thử), có thể `theme.rs`; `website/public/img/options-en.webp`, `options-vi.webp`, `website/index.html` + `website/src/vi.json` (chỉ `alt` của ảnh Tùy chọn, và kích thước `width`/`height` nếu đổi), `website/README.md` (cách tạo lại ảnh), `memory-bank/decisions.md` (quyết định mới, số kế tiếp sau mục cuối), `docs/testing/manual-checklist.md` (một mục thử tay).

## Yêu cầu

1. **Khoảng cách tab:** mỗi mục tab có đệm ngang đủ rộng để giữa chữ hai tab liền kề có khoảng trống rõ ràng (gợi ý `TCM_SETPADDING` khoảng 14–16 DIP mỗi bên, đặt trước khi chèn mục; giữ nguyên đệm dọc/chiều cao dải để bố cục khung không xê dịch, hoặc nếu đổi thì mọi điều khiển bên dưới phải theo `TCM_ADJUSTRECT` như hiện nay). Chiều rộng mục phải đủ cho chữ **đậm** (tab đang chọn) — không để chữ đậm tràn ra ngoài ô hay sát tab kế. Nền hover và gạch dưới tab chọn không chạm sang tab bên cạnh.
2. Tổng chiều rộng dải tab vừa trong vùng client ở cả hai ngôn ngữ ở 100/125/150/200 % (không xuống dòng, không mũi tên cuộn). Ghi phép tính/số đo trong báo cáo.
3. Bàn phím (Ctrl+Tab, mũi tên), khung tiêu điểm, hover, bấm chuột vào tab (hit-test theo ô mới) vẫn đúng. Chạy tay các `#[ignore]` sẵn có của `settings` (`options_no_gdi_leak`, `options_keyboard_still_works`, …) và ghi kết quả.
4. **Kiểm thử ảnh:** thêm kiểm thử `#[ignore]` `webshot_options` theo mẫu `webshot.rs` (PrintWindow cửa sổ Tùy chọn của tiến trình kiểm thử, hồ sơ tạm `DARKSHOT_PROFILE_DIR`, thư mục lưu giả, mỗi ngôn ngữ một ảnh, không chuột/phím thật, không clipboard). Kèm một kiểm thử (thường, nếu làm được không cần màn hình; không thì `#[ignore]`) khẳng định khoảng trống giữa chữ hai tab liền kề ≥ một ngưỡng DIP, đo từ `TCM_GETITEMRECT` và độ rộng chữ đậm.
5. **Website:** ảnh `options-en.webp` / `options-vi.webp` là ảnh thật của tab Thông thường và đã cũ (chưa có ô "Quét thông tin nhạy cảm trước khi xuất ảnh"). Tạo lại từ `webshot_options` (cắt giống ảnh cũ: phần đầu cửa sổ gồm dải tab và các ô tích, rộng ~776 px), đổi sang WebP bằng công cụ có sẵn trên máy (ffmpeg hoặc Pillow đã cài; không cài gì mới), chất lượng 86 như README website; cập nhật `width`/`height` nếu đổi và `alt` EN/VI (thêm mục quét thông tin nhạy cảm). Không dữ liệu cá nhân trong ảnh (thư mục lưu giả). `npm ci` rồi `npm run build` trong `website/` phải qua.
6. Quyết định mới trong `memory-bank/decisions.md` (ngày 2026-11-22; đệm tab; ảnh website). Một mục thử tay trong checklist (DPI 100–200 %, hai ngôn ngữ). CHANGELOG: thêm mục "Chưa phát hành" → "Sửa lỗi": dải tab của Tùy chọn có khoảng cách.
7. `cargo fmt --all`; `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`. Không giả lập chuột/phím thật (chủ dự án đang dùng máy). Commit `fix:` / `test:` / `docs:` trên nhánh, không gộp, không đẩy. Báo cáo: thay đổi, số đo, đường dẫn ảnh trước/sau (PNG trong `target\webshots`), token nếu biết.
