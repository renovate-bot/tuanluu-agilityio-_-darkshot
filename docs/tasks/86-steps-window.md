# Task 86: Chụp từng bước — cửa sổ Xem lại, xuất

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/86-steps-window` · **Rà soát:** `reviewer` (fable) · **Phụ thuộc:** 85

Thiết kế: `docs/superpowers/specs/2026-11-18-step-capture-design.md` mục 2.6–2.8, 3, 5. Global Constraints: kế hoạch `docs/superpowers/plans/2026-11-18-step-capture.md`.

## Cần đọc
`crates/darkshot-windows/src/steps_store.rs`, `steps_bar.rs`; `history_window.rs`, `history_layout.rs`, `history_paint.rs` (mẫu cửa sổ danh sách ảnh nhỏ, xem trước, thẻ xác nhận, nút tự vẽ, ảnh nhỏ nạp nền, mở thư mục bằng explorer `/select`); `settings.rs` (`IFileOpenDialog` chọn thư mục ~1766); `ocr_window.rs` (EDIT nhiều dòng tối, `pre_translate`, `relabel`); `crates/darkshot-core/src/steps.rs` (`markdown`, `export_names`, `export_dir_name`), `thumb.rs`.

## Files
Tạo `crates/darkshot-windows/src/steps_window.rs` (và `steps_export.rs` nếu tách gọn); sửa `main.rs`, `steps_bar.rs`, `steps_store.rs` (chỉ API cần thêm).

## Yêu cầu
1. `steps_window::open()`, `open_confirm_end()`, `close()`, `pre_translate`, `relabel()`; một cửa sổ duy nhất (mở lại thì đưa ra trước). Bố cục thiết kế 2.6; hàm bố cục thuần + kiểm thử (không chồng/cắt chữ Việt/English, DPI 96–192, cỡ tối thiểu).
2. Danh sách: ảnh nhỏ nạp trên luồng nền (dùng `thumb` của core), "Bước k" + dòng đầu ghi chú; chọn bằng chuột/phím mũi tên; giữ chọn sau khi lên/xuống/xóa.
3. Ghi chú: EDIT nhiều dòng tối, `EM_LIMITTEXT` 2000; lưu vào `steps_store::set_caption` khi mất tiêu điểm, đổi bước, đóng cửa sổ, xuất, kết thúc.
4. Lên/Xuống/Xóa bước: vô hiệu khi không áp dụng; Delete trên danh sách = Xóa bước.
5. Xuất…: hộp chọn thư mục (mặc định thư mục lưu ảnh của cấu hình); sao chép tệp và ghi `steps.md` trên luồng nền; tạo thư mục con theo `export_dir_name` (+ `_2`… nếu có sẵn — không bao giờ ghi vào thư mục đã có); xong → `mark_exported`, thông báo Success `steps_exported`, mở Explorer chọn thư mục mới; lỗi → thông báo Error `StepsExportFailedTitle` (+ lý do ngắn qua i18n `reason_try_again` nếu có), không nửa vời (xóa thư mục con đã tạo dở).
6. Kết thúc phiên: `dirty()` và có bước → thẻ xác nhận trong cửa sổ (mẫu thẻ của Lịch sử: phủ mờ, Hủy/Kết thúc, Esc = Hủy); xác nhận hoặc không cần → `steps_store::end()`, đóng cửa sổ và thanh phiên. Thanh phiên "Kết thúc" và "Xem lại" gọi các hàm này (thay các hàm tạm của Task 85).
7. Trạng thái trống (thiết kế 2.6). Đổi ngôn ngữ → `relabel`. Thoát ứng dụng → lưu ghi chú đang sửa rồi đóng.

## Kiểm thử (viết trước)
Bố cục thuần; trạng thái nút theo vị trí chọn; tên thư mục xuất không trùng (thư mục tạm có sẵn `darkshot-steps-…`); xuất trên thư mục tạm: đủ ảnh theo thứ tự hiển thị, `steps.md` đúng (so với `steps::markdown`); lỗi giữa chừng → không để thư mục dở; chạy tay (không chuột): mở cửa sổ với phiên 3 bước tổng hợp, đọc chữ danh sách/ghi chú qua thông điệp, đóng không rò GDI/USER.

TDD `test:` rồi `feat:`. `cargo fmt --all`; `powershell -File scripts/check.ps1` → `ALL CHECKS PASSED`; build release `--target-dir target/task86`.

## Review focus
Mất ghi chú (các đường lưu), xuất an toàn (không ghi đè, không nửa vời, tên tệp), luồng nền và sở hữu dữ liệu, thẻ xác nhận, vòng đời, i18n, DPI.
