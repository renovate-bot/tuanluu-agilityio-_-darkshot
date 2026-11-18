# Task 80: OCR cục bộ — cửa sổ kết quả, nút lớp phủ, menu ảnh ghim

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/80-ocr-window` · **Rà soát:** `reviewer` (fable) · **Phụ thuộc:** 79

Thiết kế: `docs/superpowers/specs/2026-11-18-local-ocr-design.md` mục 2, 3, 5, 6. Global Constraints: kế hoạch `docs/superpowers/plans/2026-11-18-local-ocr.md`.

## Cần đọc
`crates/darkshot-windows/src/ocr.rs`; `help.rs`/`about.rs` (mẫu cửa sổ không chặn: `show`, `close`, `pre_translate`, dọn ở `WM_NCDESTROY`), `winutil.rs`, `theme.rs` (nút tự vẽ `custom_draw_button`, `ctl_color`, `set_dark_control_theme`, ô EDIT tối: `Builder::edit`, `draw_field`), `settings.rs` (mẫu ô EDIT tối có viền), `overlay.rs` (`BAR_CELLS`, `bar_layout_order`, `pin_glyph`/`stroke_icon`, phím Ctrl ở `on_key` và `text_key`), `pin.rs` (menu ngữ cảnh, `set_action_sink`), `main.rs` (`capture_flow_with`, nhánh `Action::Pin`, `pin_action`, thoát: Quit/`WM_ENDSESSION`/`WM_DESTROY`, vòng lặp thông điệp và `pre_translate`), `clipboard.rs` (`set_text`).

## Files
Tạo `crates/darkshot-windows/src/ocr_window.rs`; sửa `overlay.rs`, `pin.rs`, `main.rs`.

## Yêu cầu
1. `ocr_window::open(image: Image) -> Result<(), String>`: tạo cửa sổ (thiết kế 2.3), hiện "Đang nhận dạng chữ…" rồi gọi `ocr::start(image, hwnd, WM_OCR_RESULT, token)`. Tối đa 8 cửa sổ (đóng cái cũ nhất). `close_all()`; `pre_translate(&MSG) -> bool` (Esc, Enter, Ctrl+Enter, Tab giữa ô chữ và nút; Ctrl+A trong ô chữ chọn hết). Gọi `pre_translate` ở vòng lặp chính như Help/About.
2. Nhận `WM_OCR_RESULT`: `take_result` đúng một lần; token khớp cửa sổ; Ok → điền chữ (EDIT `\r\n`), chọn hết, cho sửa, dòng trạng thái `ocr_status`; chữ rỗng → `OcrEmpty`; `NoLanguage` → `OcrNoLanguage`; `Failed` → `OcrFailed` (không hiện chuỗi lỗi kỹ thuật).
3. Chép: lấy chữ trong ô (độ dài tùy ý), `WinClipboard::set_text`, thông báo Success `OcrCopiedTitle` (thông báo Success tuân ô "hiện thông báo lưu và sao chép"), đóng cửa sổ. Lỗi clipboard → thông báo Error dùng chuỗi sẵn có của Chép.
4. Giao diện tối: nền `BG`, ô chữ nền `INPUT` chữ `TEXT` phông hệ thống 10 pt theo DPI, nút như Tùy chọn (Chép là nút chính `ACCENT_BUTTON`), `WM_DPICHANGED` dùng khung gợi ý và tính lại bố cục/phông, đổi cỡ tối thiểu 420×260 DIP. Hàm bố cục thuần + kiểm thử (không chồng, không cắt nút ở Việt/English, DPI 96–192).
5. Lớp phủ: `BAR_CELLS` 9 ô `Hoàn tác, Làm lại | Tải lên, Lưu, Chép, Ghim, Nhận dạng chữ, Hủy`; biểu tượng mới (khung bốn góc + chữ "T", cùng hệ nét `stroke_icon`); Ctrl+T (0x54) ở cả hai đường phím (gõ chữ thì chốt ô trước như Ctrl+P); cập nhật `bar_layout_order` và kiểm thử hit-test theo thiết kế.
6. `main::capture_flow_with`: `Action::Ocr` → `picked_image` (có hình đánh dấu) → `ocr_window::open`. Không ghi Lịch sử, không đổi lần chụp trước (Task 78). Lặp lại lần chụp không bao giờ chạy OCR.
7. Ảnh ghim: menu ngữ cảnh thêm "Nhận dạng chữ" sau "Lưu ảnh" → sink với `Action::Ocr` (bản sao ảnh gốc) → `ocr_window::open`.
8. Thoát: `ocr_window::close_all()` trước khi thoát vòng lặp (cùng chỗ `pin::close_all`).

## Kiểm thử (viết trước)
Bố cục thuần; `bar_layout_order` 9 ô; Ctrl+T trong `snap_state` (có vùng → `Picked{action: Ocr}`, không vùng → không gì); khớp token (kết quả token cũ bị bỏ); chạy tay `#[ignore]`: mở cửa sổ với ảnh tổng hợp có chữ, chờ kết quả, đọc chữ ô EDIT qua `GetWindowTextW` chứa "Hello"; mở/đóng 20 lần không rò GDI/USER; đóng ngay khi đang nhận dạng → không panic, không rò.

TDD `test:` rồi `feat:`. `cargo fmt --all`; `powershell -File scripts/check.ps1` → `ALL CHECKS PASSED`; build release `--target-dir target/task80`.

## Review focus
Vòng đời cửa sổ và kết quả đến muộn; `pre_translate` không nuốt phím của cửa sổ khác; hồi quy lớp phủ; i18n; EDIT tối.
