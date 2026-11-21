# Task 96: Gia cố — cửa sổ OCR trên ảnh ghim, ghim ảnh rất cao, undo/redo

**Agent:** `specialist` · **Model:** opus (chạm thứ tự cửa sổ topmost, hình học đặt ảnh ghim, sổ hoàn tác — lỗi ở đây làm mất thao tác người dùng) · **Nhánh:** `task/96-polish-pin-ocr-undo` (từ `feature/polish-14`) · **Rà soát:** `reviewer` (fable)

Thiết kế: `docs/superpowers/specs/2026-11-21-polish-14-design.md` mục 1–3. Global Constraints: mục cùng tên của `docs/superpowers/plans/2026-11-18-scroll-capture.md` (không crate mới, i18n, theme, không panic, khóa GDI cho kiểm thử, kiểm thử lớp phủ đặt `dpis`, QA không giả lập đầu vào).

## Cần đọc
`crates/darkshot-windows/src/ocr_window.rs` (`open_avoiding`, `place_avoiding` ~1107 và kiểm thử `placement_avoids_rect`), `crates/darkshot-core/src/pin.rs` (`fit_zoom`, `centered`, kiểm thử), `crates/darkshot-windows/src/pin.rs` (`initial_at`, nơi dùng `centered` cho `CenterOnCursorMonitor`), `crates/darkshot-core/src/annotate.rs` (`undo`, `redo`, `apply`, kiểm thử sổ).

## Files
`ocr_window.rs`, `crates/darkshot-core/src/pin.rs`, `crates/darkshot-windows/src/pin.rs`, `crates/darkshot-core/src/annotate.rs`, `README.md`, `CHANGELOG.md`.

## Yêu cầu
Như thiết kế mục 1–3. README: bỏ/sửa dòng "Đã biết" về ghim ảnh rất cao (nay phần đầu ảnh luôn thấy; ảnh vẫn có thể dài hơn màn hình, kéo để xem); CHANGELOG mục "Chưa phát hành" thêm nhóm "Sửa lỗi" (ba ý ngắn).

## Kiểm thử (viết trước)
- `place_avoiding` trả `overlaps` đúng (có chỗ → false; không chỗ → true); hàm quyết định topmost.
- Đặt ảnh ghim: ảnh vừa → căn giữa như cũ; ảnh cao hơn vùng ở 10% → mép trên = mép trên vùng, ngang căn giữa; ảnh rộng hơn → mép trái = mép trái vùng; vùng tọa độ âm.
- `undo`/`redo` với sổ dựng tay mà `apply` trả `None` → ngăn không đổi, trả `None`; sổ nhất quán → hành vi cũ (kiểm thử cũ giữ nguyên).
- Chạy tay (không chuột): mở cửa sổ OCR với khung cần tránh phủ kín vùng làm việc → cửa sổ có `WS_EX_TOPMOST`; khung nhỏ → không topmost.

TDD `test:` rồi `fix:`. `cargo fmt --all`; `powershell -File scripts/check.ps1` → `ALL CHECKS PASSED`.
