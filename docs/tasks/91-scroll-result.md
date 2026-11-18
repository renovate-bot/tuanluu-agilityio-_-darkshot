# Task 91: Chụp cuộn — cửa sổ kết quả

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/91-scroll-result` · **Rà soát:** `reviewer` (fable) · **Phụ thuộc:** 90

Thiết kế: `docs/superpowers/specs/2026-11-18-scroll-capture-design.md` mục 2.5, 4, 5. Global Constraints: kế hoạch `docs/superpowers/plans/2026-11-18-scroll-capture.md`.

## Cần đọc
`crates/darkshot-windows/src/scroll_session.rs` (hàm kết quả đăng ký), `ocr_window.rs`/`steps_window.rs` (mẫu cửa sổ tối không chặn, nút tự vẽ, `pre_translate`, `relabel`, DPI), `history_window.rs`/`history_paint.rs` (vẽ ảnh thu nhỏ chất lượng), `pin.rs` (`open(image, CenterOnCursorMonitor)`), `main.rs` (`run_on_image`, `pin_failure_notice`, thoát).

## Files
Tạo `crates/darkshot-windows/src/scroll_result.rs`; sửa `main.rs`, `scroll_session.rs` (đăng ký hàm kết quả thật).

## Yêu cầu
1. `scroll_result::open(image, uncertain: u32)`: một cửa sổ mỗi kết quả, tối đa 4. Khi đã có 4: nếu có cửa sổ cũ nhất đã được Lưu/Chép/Tải lên/Ghim ít nhất một lần thì đóng nó để mở cái mới; nếu cả 4 chưa dùng thì vẫn mở cái mới KHÔNG đóng cái nào (không bao giờ bỏ một ảnh chưa dùng) và báo Info "Có nhiều ảnh chụp cuộn chưa lưu" (chuỗi i18n mới `ScrollManyOpenTitle`). Kích thước mặc định 70% chiều cao vùng làm việc, rộng theo tỉ lệ (tối thiểu 480×400 DIP), giữa màn hình chứa con trỏ, đổi cỡ được.
2. Xem trước: ảnh thu theo bề rộng khung xem (HALFTONE, dựng sẵn bitmap thu nhỏ một lần trên luồng nền nếu ảnh > 4096 px cao; trong lúc dựng hiện "Đang chuẩn bị…"), cuộn dọc bằng con lăn/thanh cuộn/PgUp/PgDn/Home/End.
3. Dòng thông tin "W × H px"; dải cảnh báo vàng khi `uncertain > 0` (`scroll_uncertain`).
4. Nút Lưu, Chép, Tải lên, Ghim, Đóng: Lưu/Chép/Tải lên → `run_on_image(action, bản sao ảnh)` (Tải lên khi thiếu R2 như thường); Ghim → `pin::open(bản sao, CenterOnCursorMonitor)` + `pin_failure_notice`; Đóng/Esc → đóng, bỏ ảnh. Cửa sổ không tự đóng sau Lưu/Chép.
5. Đổi ngôn ngữ → `relabel`; thoát → đóng hết.

## Kiểm thử (viết trước)
Bố cục thuần (Việt/English, DPI 96–192, không chồng/cắt); tính tỉ lệ thu và vùng cuộn; chính sách tối đa 4 cửa sổ; chạy tay (không chuột): mở với ảnh tổng hợp 900×12000, đợi dựng xong, gửi WM_MOUSEWHEEL/WM_VSCROLL tới cửa sổ, kiểm vị trí cuộn; dải cảnh báo khi uncertain>0; mở/đóng 10 lần không rò GDI/USER.

TDD `test:` rồi `feat:`. `cargo fmt --all`; `powershell -File scripts/check.ps1` → `ALL CHECKS PASSED`; build release `--target-dir target/task91`.
