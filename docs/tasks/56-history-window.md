# Task 56: Lịch sử – vẽ danh sách, ảnh nhỏ, xem trước, cuộn, bàn phím

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/56-history-window` · **Rà soát:** `specialist`

Thiết kế: `docs/superpowers/specs/2026-11-18-screenshot-history-design.md` mục 3.1–3.5, 3.7 (các trạng thái trống/tắt), 5. Global Constraints: mục cùng tên của `docs/superpowers/plans/2026-11-18-screenshot-history.md`. Task 52–55 đã ở nhánh phát triển. Task 57 làm tìm/lọc/hành động/xác nhận: task này dựng toàn bộ phần NHÌN và CHỌN, các nút hành động có mặt nhưng chưa nối (hoặc nối vào hàm rỗng do Task 57 thay).

## Cần đọc
`crates/darkshot-windows/src/history_window.rs` (khung Task 55), `theme.rs` (vẽ tay: `draw_button`, `custom_draw_button`, `round_rect`, `buffered`, `draw_text`, `draw_field`, `focus_frame`, `set_dark_control_theme`, `brush`), `about.rs` (phông, DPI, vẽ nút), `overlay.rs` (CHỈ cách quy đổi DIP→pixel và vẽ DIB: tìm `StretchDIBits`/`CreateDIBSection`), `clipboard.rs` (đọc, không đụng), `crates/darkshot-core/src/{history.rs,history_store.rs,thumb.rs,theme.rs}`.

## Files
Sửa/tạo: `history_window.rs` (có thể tách `history_layout.rs` thuần để kiểm thử và `history_paint.rs`), `main.rs` (chỉ nếu cần phân phối thông điệp), `history_service.rs` (đặt `set_window`).

## Yêu cầu

### Mô hình và nạp dữ liệu
- Khi mở: nạp `Store::load()` ở LUỒNG NỀN (`darkshot-history-load`), gửi kết quả về cửa sổ bằng `WM_APP+7` (con trỏ `Box` bị rò thành `LPARAM`, nhận lại bằng `Box::from_raw`; hủy cửa sổ thì bỏ qua kết quả) kèm mã thế hệ; trong lúc chờ vẽ trạng thái trống trơn không chữ. `WM_HISTORY_CHANGED` (từ dịch vụ) → nạp lại, giữ mục chọn theo `id`.
- Trạng thái trong `thread_local! State` (`RefCell`): `entries: Vec<Entry>`, `visible: Vec<usize>` (Task 57 điền theo bộ lọc; ở task này `visible` = tất cả), `selected: Option<String>` (id), `scroll: i32` (pixel), `thumbs: ThumbCache`, `preview: PreviewState`, `hover: Option<usize>`, `dpi`, `enabled: bool` (từ `Config.history_enabled` qua `with_app`; hàm tiện ích `history_enabled_now()` ở `main.rs`).

### Bố cục thuần (`history_layout.rs`, kiểm thử được, nhận DPI)
`layout(client_w, client_h, dpi, banner: bool) -> Layout { header, search, chips:[Rect;4], clear_btn, banner, list, detail, preview_area, meta_area, buttons_row, status_line }` theo mục 3.2–3.5: lề 16 DIP, thanh trên cao 56 (+44 nếu có dải thông báo), cột danh sách 40% kẹp 320–420 DIP, dòng 72 DIP, ô ảnh 88×56. `row_rect(i, scroll)`, `hit_test(pt) -> Hit::{Row(i), Chip(k), Clear, Preview, Button(k), None}`, `scroll_clamp`, `ensure_visible(i)`. Các ô không được chồng nhau ở mọi cỡ từ tối thiểu đến 4K và DPI 96–288.

### Vẽ (một cửa sổ, `buffered` hai lớp để không nháy; GDI thường, KHÔNG GDI+ ở lớp này)
- Danh sách: mỗi dòng theo mục 3.4, chỉ vẽ dòng giao vùng cần vẽ; trạng thái chọn/trỏ; mục tệp mất; nhãn loại có chấm màu; chữ cắt bằng `…`.
- Khung chi tiết: vùng xem trước, tiêu đề, lưới nhãn–giá trị, hàng nút. Nút là BUTTON con (HWND) tự vẽ bằng `theme::custom_draw_button` (ID 201–205: Mở, Sao chép ảnh, Sao chép liên kết, Mở thư mục, Xóa khỏi lịch sử); hiện/ẩn/vô hiệu theo bảng hành động mục 3.6 (hàm thuần `actions_for(&Entry, file_exists: bool) -> ActionSet` ở `history_layout.rs`, kiểm thử đủ 4 loại × tệp có/mất).
- Trạng thái đặc biệt: trống đang bật; tắt (và trống) với nút chính "Mở Tùy chọn" (ID 210, bấm gửi `WM_OPEN_SETTINGS` như tray); dải thông báo khi tắt mà còn mục. Biểu tượng khung ảnh/ảnh vỡ vẽ bằng GDI (đa giác, `round_rect`), không dùng tệp ảnh.
- Thanh cuộn: dùng thanh cuộn thật của cửa sổ con (`SCROLLBAR` điều khiển hoặc `WS_VSCROLL` với chủ đề `DarkMode_Explorer` qua `theme::set_dark_control_theme`), phải trông tối; cuộn chuột (`WM_MOUSEWHEEL`, cuộn mượt theo dòng), kéo thanh, phím. Giải thích lựa chọn trong báo cáo.

### Ảnh nhỏ
`ThumbCache`: `HashMap<String, Thumb>` + thứ tự LRU, tối đa 120 mục. Mỗi `Thumb` giữ `HBITMAP` DIB 32 bit đã ghép sẵn lên nền `INPUT` ở cỡ ô ảnh theo DPI (giữ tỉ lệ, không phóng to). Dòng nào cần mà chưa có → ghi nhận "đang nạp" (không nạp lặp) và nhờ luồng nền `darkshot-history-thumb` gọi `Store::thumbnail` rồi `PostMessage(WM_APP+8)` kèm mã thế hệ; thông điệp đến thì tạo HBITMAP trên luồng giao diện và làm mới đúng dòng đó. Lỗi → nhớ "hỏng" (vẽ biểu tượng ảnh vỡ), không thử lại cho tới khi nạp lại. Mọi HBITMAP bị `DeleteObject` khi loại khỏi đệm và khi cửa sổ hủy.

### Xem trước
Chọn mục (chuột, bàn phím) → tăng mã thế hệ, đặt `Loading`, nhờ luồng `darkshot-history-preview` giải mã `entry.image_path(root)` bằng `thumb::decode_png_file`, `downscale` về `min(khung xem trước theo pixel, 2400)`; kết quả `WM_APP+9` mang mã thế hệ, bị bỏ nếu cũ. Tệp không tồn tại (kiểm bằng `Path::exists` trên luồng nền) → `Missing`; giải mã lỗi → `Unreadable`. Trạng thái `Ready(HBITMAP)` giữ MỘT bản (xóa cái cũ). Không giữ ảnh gốc.

### Bàn phím và con trỏ
Lên/Xuống/Home/End/PageUp/PageDown di chuyển mục chọn và `ensure_visible`; nháy chuột chọn; con trỏ tay khi trỏ lên dòng; `WM_MOUSEMOVE` + `TrackMouseEvent` cho trỏ/rời. Tab đi qua: ô tìm (Task 57) → bộ lọc → danh sách → nút hành động. Danh sách có `WS_TABSTOP`-tương đương (cửa sổ chính nhận `WM_GETDLGCODE` hoặc dùng cửa sổ con danh sách riêng — chọn cách chạy đúng và ghi lại); khung tiêu điểm vẽ bằng `theme::focus_frame`.

### Chưa làm ở task này
Ô tìm, bộ lọc, xóa lịch sử, xác nhận, menu chuột phải, thực thi hành động (Task 57). Ô tìm và bốn viên thuốc lọc và nút "Xóa lịch sử" ĐƯỢC vẽ (chỗ của chúng trong bố cục đã đặt) nhưng có thể chưa hoạt động; đừng để chúng trông hỏng: nếu chưa nối thì vẽ trạng thái tĩnh đẹp. (Task 57 thay phần này.)

## Kiểm thử
Thuần: bố cục không chồng/không tràn (lưới kích thước × DPI), `hit_test`, `scroll_clamp`, `ensure_visible`, `actions_for`, `fit` ảnh nhỏ vào ô, LRU của `ThumbCache` (loại đúng cái cũ nhất, không vượt 120, không nạp lặp), logic mã thế hệ (kết quả cũ bị bỏ), chọn giữ theo `id` sau nạp lại, chọn mục kế khi mục đang chọn biến mất. Chạy tay `#[ignore]`: mở cửa sổ với kho tạm 60 mục tổng hợp rồi đóng không rò GDI (đếm trước/sau, dung sai như `options_no_gdi_leak` hiện có); mô phỏng cuộn/chọn 100 lần không rò.

## Quan sát bằng mắt (bắt buộc)
Dựng thư mục demo tổng hợp (ảnh PNG sinh bằng mã, ít nhất 12 mục đủ 4 loại và 1 tệp mất), chạy bản build cách ly (`DARKSHOT_PROFILE_DIR` trỏ vào thư mục tạm, `--history`), chụp màn hình thật bằng PowerShell (`System.Drawing.Graphics.CopyFromScreen` hoặc `PrintWindow`) lưu vào thư mục tạm của bạn (không commit), xem bằng công cụ Read, và chỉnh cho đến khi: không cắt chữ, thẳng hàng, khoảng cách đều, ảnh không méo. Ghi trong báo cáo những gì đã chỉnh. Không tắt tiến trình darkshot không do mình mở.

## Việc cần chủ dự án thử tay
Xem cửa sổ với nhiều mục; cuộn; chọn; DPI 100/150/200%.

## Báo cáo
Theo mẫu `.claude/agents/specialist.md`.
