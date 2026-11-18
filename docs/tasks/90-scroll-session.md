# Task 90: Chụp cuộn — phiên chụp (khung vùng, thanh điều khiển, hẹn giờ), lớp phủ Ctrl+L

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/90-scroll-session` · **Rà soát:** `reviewer` (fable) · **Phụ thuộc:** 89

Thiết kế: `docs/superpowers/specs/2026-11-18-scroll-capture-design.md` mục 2.1–2.4, 2.6, 4. Global Constraints: kế hoạch `docs/superpowers/plans/2026-11-18-scroll-capture.md`.

## Cần đọc
`crates/darkshot-core/src/scroll_stitch.rs`; `crates/darkshot-windows/src/`: `capture.rs` (BitBlt, DPI awareness luồng), `steps_bar.rs` (thanh nổi không lấy tiêu điểm, `WDA_EXCLUDEFROMCAPTURE` + kiểm bản dựng, kéo, DPI — dùng lại hàm kiểm loại trừ nếu có thể), `overlay.rs` (`bar_cells(steps)`, ô thanh ngang, phím Ctrl ở `on_key`/`text_key`, Esc toàn cục tạm khi lớp phủ mở), `main.rs` (`capture_flow_with`, cổng chụp `gate`/`request`, thoát `exit_cleanup`), `hotkey.rs`.

## Files
Sửa `capture.rs`, `overlay.rs`, `main.rs`; tạo `scroll_session.rs`. Kết quả cuối giao cho một hàm đăng ký `fn(Image, u32 /*uncertain*/)` (Task 91 mở cửa sổ kết quả; tạm thời mặc định: lưu như Lưu thường qua `run_on_image(Save)` để có thể thử).

## Yêu cầu
1. `capture::capture_rect(screen: Rect) -> windows::core::Result<Image>`: BitBlt chỉ vùng (PerMonitorV2 trên luồng gọi), alpha 255, không con trỏ.
2. Lớp phủ: ô `Act(Action::Scroll)` giữa Nhận dạng chữ và (Thêm bước nếu có phiên) Hủy; `bar_cells(steps)` → 10 ô không phiên / 11 ô có phiên; biểu tượng mũi tên xuống trong khung; Ctrl+L (0x4C) ở cả hai đường phím khi có vùng chọn; cập nhật kiểm thử thứ tự ô và hit-test theo thiết kế.
3. `main`: `Action::Scroll` → `scroll_session::start(rect_màn_hình)` (vùng đã chọn, đổi sang tọa độ màn hình đúng một lần). Trong lúc phiên chạy, `gate` coi như đang chụp (phím chụp khác bỏ qua; menu khay Chụp bị bỏ qua), thoát gọi `scroll_session::cancel()`.
4. `scroll_session`: bốn cửa sổ khung (topmost, `WS_EX_TRANSPARENT | LAYERED | TOOLWINDOW | NOACTIVATE`, loại khỏi ảnh chụp; không loại được thì không tạo khung) bao ngoài vùng 2 DIP; thanh điều khiển (thiết kế 2.2, vị trí dưới/trên/trong vùng làm việc, chữ và chiều cao cập nhật, trạng thái vàng 1,5 s khi `Uncertain`); nếu không loại được thanh khỏi ảnh chụp thì đặt thanh ngoài vùng (bắt buộc) — thanh không bao giờ nằm trong vùng chụp; hẹn giờ 100 ms: `capture_rect` → `Stitcher::push`; Esc toàn cục tạm (`RegisterHotKey` như lớp phủ; gỡ khi kết thúc); Xong / giới hạn / vùng mất (đổi cấu hình màn hình) → `finish()` → hàm kết quả; Hủy/Esc → bỏ. Đo thời gian mỗi lượt; > 80 ms liên tiếp thì giãn hẹn giờ lên 200 ms.
5. Dọn sạch trên mọi đường (hẹn giờ, cửa sổ, phím toàn cục, GDI); không panic trong thủ tục cửa sổ.

## Kiểm thử (viết trước)
`bar_cells` 10/11 ô; Ctrl+L trong `snap_state`; hàm thuần vị trí thanh (dưới/trên/trong, màn hình tọa độ âm) và khung (nới 2 DIP); trạng thái phiên (máy trạng thái thuần: Running → Done/Cancelled/Limit, giãn hẹn giờ); chạy tay (không chuột, không cuộn ứng dụng khác): tạo một cửa sổ thử của chính kiểm thử có nội dung vẽ được, bắt đầu phiên trên vùng của nó, kiểm thử tự cuộn nội dung CỦA CỬA SỔ THỬ (vẽ lại với offset) qua vài nhịp, Xong → ảnh ghép đúng chiều cao dự kiến; khung/thanh có `GetWindowDisplayAffinity`; mở/đóng 10 lần không rò GDI/USER.

TDD `test:` rồi `feat:`. `cargo fmt --all`; `powershell -File scripts/check.ps1` → `ALL CHECKS PASSED`; build release `--target-dir target/task90`.

## Review focus
Không lấy tiêu điểm; không vào ảnh; dọn sạch; cổng chụp; hồi quy lớp phủ; tọa độ/DPI; hiệu năng hẹn giờ.
