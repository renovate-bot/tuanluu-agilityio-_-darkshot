# Task 57: Lịch sử – tìm, lọc, hành động, menu chuột phải, xóa, xác nhận

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/57-history-actions` · **Rà soát:** `specialist`

Thiết kế mục 3.3, 3.5 (dòng trạng thái), 3.6, 3.7, 6. Global Constraints: mục cùng tên của `docs/superpowers/plans/2026-11-18-screenshot-history.md`. Task 52–56 đã ở nhánh phát triển: cửa sổ đã có danh sách, ảnh nhỏ, xem trước, bố cục (`history_layout.rs`, có chỗ cho ô tìm, 4 viên lọc, nút "Xóa lịch sử"), `actions_for`, nút hành động ID 201–205.

## Cần đọc
`history_window.rs`, `history_layout.rs`, `history_service.rs`, `clipboard.rs` (`WinClipboard::set_image`, `set_text`), `theme.rs` (`edit_border`, `draw_field`, `round_rect`, `draw_button`, `enable_dark_menus`), `settings.rs` (CHỈ cách dựng ô EDIT tối: subclass `WM_NCPAINT`/`WM_NCCALCSIZE`, margin), `main.rs` (`open_release_url` làm mẫu ShellExecute, `post_open_settings`), `darkshot_core::history` (`filter_entries`, `Filter`).

## Yêu cầu

### Tìm và lọc
- Ô tìm: EDIT con (ID 220) dựng và vẽ như ô nhập tối của Tùy chọn (viền 2 DIP `edit_border`, gạch dưới `ACCENT` khi tiêu điểm), kính lúp vẽ tay bên trái, chữ gợi ý `HistorySearchPlaceholder` (vẽ khi trống và không tiêu điểm; hoặc `EM_SETCUEBANNER` nếu hiện đúng màu tối, chọn cái chạy), nút `×` (HistorySearchClear làm tooltip và nhãn) hiện khi có chữ. Gõ là lọc ngay (`EN_CHANGE`), không cần Enter; Ctrl+F đưa tiêu điểm vào ô và chọn hết; Esc: có chữ thì xóa chữ, không thì đóng cửa sổ.
- Bốn viên lọc (ID 230–233): BUTTON con tự vẽ dạng viên thuốc (cao 28 DIP, bo tròn hoàn toàn); mục chọn `ACCENT_BUTTON` chữ trắng, trỏ `TAB_HOVER`; kiểu nút radio về bàn phím (mũi tên trái/phải đổi và chọn; chỉ mục đang chọn có `WS_TABSTOP`).
- `visible` = `history::filter_entries(entries, filter, query, lang)`; đổi bộ lọc/ô tìm → tính lại, cuộn về đầu nếu mục chọn rời khỏi danh sách (giữ mục chọn nếu còn), chọn mục đầu nếu không còn mục nào được chọn và danh sách khác rỗng. Có hai trạng thái trống: lịch sử rỗng (mục 3.7) và không khớp (`HistoryNoMatch*` kèm nút "Xóa bộ lọc" ID 211 đặt lại bộ lọc Tất cả và ô tìm).
- Nút "Xóa lịch sử" (ID 212) vô hiệu khi `entries` rỗng.

### Hành động (bấm nút ID 201–205, menu chuột phải, phím tắt)
Theo bảng 3.6; điều kiện qua `actions_for`. Mọi việc nặng ở LUỒNG NỀN rồi gửi kết quả về cửa sổ bằng `WM_APP+10` (mã trạng thái: một giá trị enum trong `WPARAM`):
- **Mở:** Saved/UploadFallback/Copied → `ShellExecuteW("open", đường dẫn ảnh)`; Uploaded → `ShellExecuteW("open", url)` CHỈ KHI `url` bắt đầu `https://` (nếu không: bỏ và báo `HistoryStatusFailed`). Nháy đúp dòng và Enter = Mở. Với Uploaded có `url`, nhãn nút là `HistoryOpenLink`.
- **Sao chép ảnh:** luồng nền `decode_png_file(image_path)` → `WinClipboard.set_image`; báo `HistoryStatusImageCopied` hoặc `HistoryStatusFailed`. Ctrl+C khi tiêu điểm ở danh sách.
- **Sao chép liên kết:** `WinClipboard.set_text(url)` (nhanh, làm thẳng trên luồng giao diện); `HistoryStatusLinkCopied`. Ctrl+Shift+C.
- **Mở thư mục chứa ảnh:** `explorer.exe /select,"<đường dẫn>"` qua `ShellExecuteW` hoặc `SHOpenFolderAndSelectItems` (chọn cách an toàn với đường dẫn có dấu cách và Unicode); chỉ cho Saved/UploadFallback còn tệp.
- **Xóa khỏi lịch sử:** `Store::remove(id)` ở luồng nền; xong thì nạp lại, chọn mục kế (cùng vị trí, hoặc mục cuối), báo `HistoryStatusRemoved`. Delete khi tiêu điểm ở danh sách. Không xác nhận.
- Mục hiển thị là tệp mất: Mở, Sao chép ảnh, Mở thư mục bị vô hiệu (Sao chép liên kết và Xóa vẫn dùng được).
- Dòng trạng thái (3.5): chữ nhỏ `TEXT_MUTED` (lỗi `DANGER`) dưới hàng nút, hẹn giờ 2,5 giây (`SetTimer` ID riêng) tự tắt; thông báo mới thay cái cũ.
- Menu chuột phải (`WM_CONTEXTMENU` và Shift+F10/phím Menu): chọn dòng dưới con trỏ rồi `TrackPopupMenu` (menu tối qua `theme::enable_dark_menus`), các mục theo `actions_for` (ẩn mục không áp dụng, vô hiệu mục không làm được), có vạch ngăn trước "Xóa khỏi lịch sử". Phím tắt in bên phải mục (Enter, Ctrl+C, Ctrl+Shift+C, Del) cho các mục tương ứng.

### Xóa toàn bộ + xác nhận
Nút "Xóa lịch sử" → thẻ xác nhận vẽ trong cửa sổ (mục 3.7): lớp phủ `BG` mờ phủ phần thân (vẽ bằng `AlphaBlend` lên bộ đệm), thẻ `SURFACE` bo 8 giữa cửa sổ rộng 400 DIP có tiêu đề `HistoryClearConfirmTitle`, thân `history_clear_confirm_body(n)`, hai BUTTON con: "Hủy" (`HistoryCancel`, mặc định, tiêu điểm đầu) và "Xóa lịch sử" (`HistoryClearConfirmYes`, nút nguy hiểm đỏ theo `ButtonKind` có sẵn hoặc thêm một kiểu `Danger` vào `theme.rs` nếu chưa có — nếu thêm, thêm cả kiểm thử tương phản vào `TEXT_PAIRS`/`UI_PAIRS` ở lõi). Trong lúc thẻ mở: mọi điều khiển phía sau vô hiệu/không nhận chuột và bàn phím; Tab chỉ xoay trong hai nút; Esc = Hủy; Enter = nút đang có tiêu điểm. Đồng ý → `Store::clear()` ở luồng nền, nạp lại, báo `HistoryStatusCleared`.

### Cập nhật động
Khi cấu hình đổi lúc cửa sổ mở (ví dụ lưu Tùy chọn bật/tắt lịch sử): dải thông báo và trạng thái tắt/bật phải cập nhật (đọc `history_enabled_now()` khi nhận `WM_HISTORY_CHANGED` và khi cửa sổ được kích hoạt `WM_ACTIVATE`).

## Kiểm thử
Thuần: ánh xạ bộ lọc ↔ viên thuốc (chỉ số), `visible` theo tìm/lọc (nhiều tổ hợp), giữ mục chọn khi lọc, chọn kế sau khi xóa (đầu, giữa, cuối, cuối cùng còn lại), `actions_for` cho bảng hành động (đã có ở 56; thêm trường hợp url không https), điều kiện URL mở được (`https://` hợp lệ; `http://`, `file:`, `javascript:`, chuỗi rỗng, chuỗi có ký tự điều khiển → từ chối), dựng đối số explorer đúng với đường dẫn có dấu cách và Unicode, trạng thái thẻ xác nhận (mở/đóng/Esc/Enter, Tab xoay hai nút), `history_clear_confirm_body` với n=1 và n=50. `#[ignore]` chạy tay: gõ chữ vào ô tìm bằng `SendMessage(WM_CHAR)` thu hẹp danh sách; nháy các viên lọc; mở/đóng thẻ xác nhận 50 lần không rò GDI; xóa một mục trên kho tạm làm tệp `images/` của nó biến mất và tệp `path` người dùng còn (kiểm tra thật trên đĩa).

## Quan sát bằng mắt (bắt buộc)
Như Task 56: dựng demo tổng hợp, chụp màn hình thật các trạng thái: nhiều mục, đang tìm, mỗi bộ lọc, không khớp, trống, tắt, tắt kèm mục cũ, thẻ xác nhận, menu chuột phải, dòng trạng thái, ở tiếng Việt và English, DPI hiện tại; chỉnh cho đến khi tinh tế. Báo cáo nêu những gì đã chỉnh.

## Việc cần chủ dự án thử tay
Toàn bộ mục "Đợt 7" của checklist (Task 58 viết).

## Báo cáo
Theo mẫu `.claude/agents/specialist.md`. Nêu mọi quyết định trình bày bạn tự đưa ra.
