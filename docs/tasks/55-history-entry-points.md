# Task 55: Lịch sử – hộp tích trong Tùy chọn, mục menu khay, khung cửa sổ

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/55-history-entry-points` · **Rà soát:** `reviewer`

(Đổi so với bảng kế hoạch: `settings.rs` bố cục tay phức tạp nên giao `specialist`.) Thiết kế mục 3.2, 7. Global Constraints: mục cùng tên của `docs/superpowers/plans/2026-11-18-screenshot-history.md`. Task 52–54 đã ở nhánh phát triển (chuỗi `ChkHistory`, `NoteHistory`, `TrayHistory`, `HistoryTitle`; `Config.history_enabled`; `history_service`; `WM_OPEN_HISTORY` và hàm `history_window::show` rỗng; `parse_args`).

## Cần đọc
`crates/darkshot-windows/src/settings.rs` (tab Thông thường: `ID_CHECK_UPDATES`, `ID_*` ghi chú 150/151, hàm bố cục, `Controls`, `FormValues`, `collect`/`populate`, kiểm thử bố cục), `tray.rs` (`MenuCommand`, `menu_layout`, `AppEvent`), `main.rs` (`handle_event`, `info_windows_stale`, nơi đóng Thông tin/Trợ giúp khi đổi ngôn ngữ), `about.rs` (mẫu cửa sổ phụ), `winutil.rs` (`create_popup`, `bring_to_front`), `theme.rs`.

## Files
Sửa `settings.rs`, `tray.rs`, `main.rs`; tạo `crates/darkshot-windows/src/history_window.rs` (KHUNG: chỉ lớp cửa sổ, cửa sổ rỗng nền `BG`).

## Yêu cầu
1. **Hộp tích** `ChkHistory` + dòng ghi chú `NoteHistory` (kiểu giống dòng ghi chú 150/151, chữ `TEXT_MUTED`, tự xuống dòng) ở tab Thông thường. ID mới 153 (hộp), 154 (ghi chú). Đặt SAU ô "Báo có bản mới" cùng nhóm "hệ thống", trước hàng nút. Nếu tab hết chỗ thì tăng chiều cao khung Tùy chọn (và các tab khác giữ nguyên bố cục); chữ Việt dài nhất không được bị cắt ở 100%, 150%, 200% DPI (kiểm tra phép tính bố cục bằng kiểm thử thuần như các kiểm thử bố cục hiện có). `FormValues.history_enabled` ↔ hộp tích; lưu thành công → `history_service::set_enabled(new)`. Hộp tích thuộc nhóm Tab/Phím tắt bàn phím đúng thứ tự như các hộp khác. Hộp tích vẽ bằng `theme::draw_check`.
2. **Khay:** `MenuCommand::History` đứng ngay sau `Capture`, trước `Options` (cập nhật `menu_layout` và kiểm thử của nó, cả hai nhánh có/không cập nhật); chữ `TrayHistory`; chọn thì mở cửa sổ Lịch sử (`WM_OPEN_HISTORY`/`history_window::show`). Menu khay mở được khi đang chụp? Giữ y như mục "Tùy chọn": chỉ cần theo cùng quy tắc `gate` của mục "Thông tin" (cửa sổ phụ, không chặn).
3. **Khung cửa sổ** `history_window`: một thể hiện (`OPEN` thread_local như `about.rs`), `show(owner)` mở hoặc đưa ra trước; kích thước mặc định 960×640 DIP, tối thiểu 760×520 (`WM_GETMINMAXINFO`), đổi cỡ được (`WS_THICKFRAME`, `WS_MAXIMIZEBOX`), căn giữa màn hình chứa con trỏ, tiêu đề `HistoryTitle`, thanh tiêu đề tối, biểu tượng ứng dụng (giống `about`), nền `BG` (`WM_ERASEBKGND`), DPI theo cửa sổ (`WM_DPICHANGED` đặt lại kích thước theo `RECT` đề nghị), Esc đóng (qua `pre_translate` như `about.rs`), `pub fn close_if_open()`, `pub fn is_open() -> bool`, `pub fn hwnd() -> isize` (để `history_service::set_window` được gọi khi mở và gọi lại 0 khi hủy). Chưa vẽ nội dung.
4. **Đổi ngôn ngữ/phím tắt:** thêm cửa sổ Lịch sử vào chỗ đóng cửa sổ phụ khi `info_windows_stale` (đọc `main.rs` để xem cách đóng Thông tin/Trợ giúp; ngôn ngữ đổi thì đóng cả Lịch sử; đổi phím tắt thì KHÔNG cần đóng Lịch sử – cập nhật hàm thuần và kiểm thử cho khớp: ghi rõ quyết định trong báo cáo).
5. `main()`: `--history` → mở cửa sổ sau khi khởi động (dùng `parse_args` của Task 54). `pre_translate` của `history_window` cắm vào vòng lặp thông điệp chính.

## Kiểm thử
Thuần: `menu_layout` cả hai nhánh có `History` sau `Capture`; bố cục tab Thông thường có hộp + ghi chú nằm trong khung và không đè hộp khác ở DPI 96/144/192 (theo mẫu kiểm thử bố cục có sẵn); `FormValues` ↔ `Config` giữ `history_enabled` cả hai chiều; hàm chọn kích thước mặc định/tối thiểu theo DPI; hàm căn giữa trên màn hình chứa con trỏ (thuần, nhận danh sách màn hình). `#[ignore]` (chạy tay): mở rồi đóng cửa sổ Lịch sử 20 lần không rò đối tượng GDI (đếm `GetGuiResources` như các kiểm thử rò hiện có); mở lần hai đưa cửa sổ cũ ra trước, không tạo thêm.

## Việc cần chủ dự án thử tay
Mở Tùy chọn xem hộp tích; mở menu khay xem mục mới; mở cửa sổ (rỗng) ở DPI khác nhau.

## Báo cáo
Theo mẫu `.claude/agents/specialist.md`.
