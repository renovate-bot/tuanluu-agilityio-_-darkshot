# Lịch sử ảnh chụp – thiết kế (đợt 7)

Ngày: 2026-11-18. Chủ dự án giao toàn quyền quyết định giao diện và cách làm, chỉ xem bản demo cuối. Mọi quyết định "(tôi chọn)" dưới đây là của phiên điều phối, ghi lại để chủ dự án rà khi xem demo.

## 1. Mục tiêu và nguyên tắc

Người dùng xem lại ảnh đã chụp: xem trước, mở lại, chép lại ảnh hoặc liên kết, mở thư mục chứa ảnh, tìm và lọc, xóa từng mục, xóa hết. Dữ liệu nằm trên máy, tồn tại qua lần khởi động lại. Không tài khoản, không máy chủ mới, không thư viện xử lý ảnh ngoài.

Không làm (ngoài phạm vi): lịch sử đám mây, đồng bộ nhiều máy, OCR, phân tích AI, chia sẻ mạng xã hội, xóa đối tượng trên R2, cấu hình vòng đời R2, thẻ, bộ sưu tập, quay phim, chụp cửa sổ, chụp hẹn giờ, sửa ảnh trong Lịch sử.

Lỗi của Lịch sử không bao giờ làm hỏng hành động chụp chính (Lưu, Chép, Tải lên): ghi lịch sử chạy sau khi hành động đã xong và mọi lỗi của nó bị nuốt (chỉ ghi vào bộ đếm lỗi trong bộ nhớ, không in ảnh hay khóa).

## 2. Hiện trạng đã xác minh (mã thật, 2026-11-18)

`main` ở `afaea38`, cây làm việc sạch. Annotation Pro có đủ trong mã: hoàn tác/làm lại nhiều bước, chọn/dời/đổi cỡ/xóa hình, Hình elip, Đánh số, bốn mức cỡ nét và chữ, làm mờ bằng ô tô đặc. Giữ nguyên, không đụng. Tài liệu `memory-bank/activeContext.md` còn nhiều đoạn cũ (đầu file nói Task 8) nhưng các đoạn "Bước kế tiếp" đã cập nhật đến Task 51; việc đồng bộ nằm ở Task 58. Repo GitHub đang CÔNG KHAI, khác dòng "riêng tư" trong CLAUDE.md: vì vậy nhánh đợt này không đẩy lên `origin` trước khi chủ dự án duyệt.

Điểm nối trong mã:
- `darkshot_core::actions::run_action` trả `Outcome::{Saved(path), Copied, Uploaded(link), UploadFailedSaved{path,..}, Failed, Cancelled}`. Nó được gọi ở hai chỗ: `main::run_on_image` (Lưu, Chép, chạy trên luồng giao diện) và `app::spawn_upload` (Tải lên, luồng riêng `darkshot-upload`). Lưu dự phòng khi thoát đi qua `app::save_pending`.
- `Config` ở `darkshot_core::config` có tương thích tiến: trường lạ hoặc sai kiểu rơi về mặc định.
- Đường dẫn: `paths.rs` (`%APPDATA%\darkshot`), chưa có `%LOCALAPPDATA%`.
- Tray: `tray::menu_layout` + `MenuCommand`. Cửa sổ phụ theo mẫu `about.rs` (`winutil::create_popup`, một cửa sổ, `pre_translate`). Giao diện tối: `theme.rs` (vẽ tay, `draw_button`, `round_rect`, `buffered`...) và bảng màu `darkshot_core::theme`.
- `png` đã có ở core (mã hóa; giải mã dùng cùng crate). Không cần crate mới.

## 3. Quyết định sản phẩm và giao diện

### 3.1 Bố cục: danh sách + khung chi tiết (tôi chọn)

Đã cân nhắc ba kiểu: (a) lưới ảnh thu nhỏ, (b) danh sách gọn, (c) danh sách bên trái cộng khung xem trước bên phải. Chọn (c): ảnh chụp màn hình có tên vô nghĩa nên cần ảnh nhỏ để nhận ra, nhưng cũng cần ngày giờ, kích thước, liên kết; lưới thì chật chữ, danh sách thuần thì không thấy ảnh. Khung chi tiết cố định bên phải cho xem trước và hành động ngay tại chỗ, nên không cần cửa sổ phụ, không cần bấm nhiều.

```
┌ Lịch sử ảnh chụp ───────────────────────────────────────────── ─ □ ✕ ┐
│ [🔍 Tìm theo tên, ngày hoặc liên kết ]  (Tất cả)(Đã lưu)(Đã tải lên)(Đã chép)      [Xóa lịch sử] │
│ ⓘ Lịch sử đang tắt: ảnh mới không được ghi lại.            [Mở Tùy chọn]  ← chỉ khi tắt │
├───────────────────────────────┬─────────────────────────────────────────┤
│ ┌────┐ Hôm nay 14:32       ●Đã lưu │  ┌───────────────────────────────┐  │
│ │ ▒▒ │ 1920 × 1080 · 412 KB         │  │                               │  │
│ └────┘                              │  │      xem trước, giữ tỉ lệ     │  │
│ ┌────┐ Hôm qua 09:10   ●Đã tải lên  │  │                               │  │
│ │ ▒▒ │ 1280 × 720 · 188 KB          │  └───────────────────────────────┘  │
│ ...                                 │  Hôm nay 14:32                      │
│                                     │  Loại  Đã lưu     Kích thước 1920×1080 │
│                                     │  Tệp   C:\Users\…\darkshot_2026….png│
│                                     │  [Mở] [Sao chép ảnh] [Mở thư mục]   │
│                                     │                  [Xóa khỏi lịch sử] │
└───────────────────────────────┴─────────────────────────────────────────┘
```

### 3.2 Kích thước và co giãn
- Khung làm việc mặc định 960×640 DIP, tối thiểu 760×520, căn giữa màn hình chứa con trỏ, đổi cỡ được. Ghi nhớ cỡ không làm (mở lại về mặc định).
- Cột danh sách rộng 40% (kẹp 320–420 DIP); khung chi tiết phần còn lại. Tiêu đề thanh trên tối (`apply_dark_title_bar`).
- Vẽ theo DPI cửa sổ, tính lại khi `WM_DPICHANGED`.

### 3.3 Thanh trên
- Ô tìm (EDIT con, viền theo `theme::edit_border`, biểu tượng kính lúp vẽ tay, chữ gợi ý khi trống, nút xóa chữ `×` khi có chữ), cao 32 DIP, co từ 220 xuống 160.
- Bốn nút lọc dạng "viên thuốc" (cao 28, bo tròn): Tất cả, Đã lưu, Đã tải lên, Đã chép. Mục chọn tô `ACCENT_BUTTON` chữ trắng; trỏ `TAB_HOVER`. Một lần chọn một bộ lọc, phím mũi tên trái phải đổi khi nhóm đang có tiêu điểm. Không đếm số trên nhãn (gọn).
- Nút "Xóa lịch sử" ở mép phải, kiểu nút phụ, chữ chuyển `DANGER` khi trỏ. Vô hiệu khi lịch sử trống.

### 3.4 Danh sách
- Mỗi dòng cao 72 DIP: ô ảnh nhỏ 88×56 (nền `INPUT`, bo 4, ảnh vừa khung giữ tỉ lệ, không phóng to ảnh nhỏ); dòng chính: ngày giờ ("Hôm nay 14:32", "Hôm qua 09:10", hoặc `dd/mm/yyyy HH:mm`); dòng phụ: `rộng × cao · dung lượng` (`TEXT_MUTED`); nhãn loại cuối dòng chính: chấm màu + chữ (Đã lưu = `TEXT_MUTED`, Đã tải lên = `ACCENT`, Đã chép = `SUCCESS`, Lưu dự phòng = `DANGER`).
- Dòng đang chọn: nền `SURFACE`, vạch trái 3 DIP `ACCENT`; trỏ: `TAB_HOVER`. Tệp mất: ô ảnh hiện biểu tượng ảnh vỡ, dòng phụ chữ `DANGER` "Không tìm thấy tệp ảnh".
- Cuộn chuột, PageUp/Down, thanh cuộn tối (theme `DarkMode_Explorer`) hoặc tự vẽ mảnh; chọn giữ đúng mục khi lọc đổi nếu mục còn trong kết quả.
- Mục mới xuất hiện khi cửa sổ đang mở (tải lên xong) thì danh sách nạp lại, giữ mục đang chọn theo `id`.

### 3.5 Khung chi tiết
- Vùng xem trước trên cùng (nền `INPUT`, đệm 16, bo 6): ảnh vừa khung, giữ tỉ lệ, không phóng to quá 100%. Đang giải mã: chữ "Đang tải bản xem trước…". Tệp mất: biểu tượng ảnh vỡ + "Không tìm thấy tệp ảnh" + gợi ý "Tệp đã bị di chuyển hoặc xóa. Vẫn có thể xóa mục này khỏi lịch sử.". Không đọc được: "Không đọc được ảnh này".
- Dưới là tiêu đề (ngày giờ đầy đủ), lưới nhãn–giá trị (Loại, Kích thước, Dung lượng, Tệp, Liên kết, Hết hạn); đường dẫn và liên kết cắt giữa bằng `…` (`DT_PATH_ELLIPSIS`/`DT_END_ELLIPSIS`), tooltip hiện đủ. "Hết hạn": ngày; đã qua ngày thì chữ `DANGER` "Đã hết hạn"; "Không hết hạn" với `Keep`.
- Hàng nút (BUTTON con tự vẽ bằng `theme::custom_draw_button`, nên có nhãn truy cập): Mở (nút chính), Sao chép ảnh, Sao chép liên kết, Mở thư mục chứa ảnh; "Xóa khỏi lịch sử" ở bên phải, kiểu nguy hiểm nhẹ. Nút không áp dụng thì ẩn, nút áp dụng nhưng không làm được (tệp mất) thì vô hiệu.
- Dòng trạng thái nhỏ dưới nút ("Đã sao chép ảnh"), tự tắt sau 2,5 giây (hẹn giờ).
- Chưa chọn gì: "Chọn một ảnh để xem trước".

### 3.6 Hành động áp dụng theo loại

| Loại | Mở | Sao chép ảnh | Sao chép liên kết | Mở thư mục | Xóa khỏi lịch sử |
|---|---|---|---|---|---|
| Đã lưu | tệp gốc | tệp gốc | – | tệp gốc | có |
| Đã tải lên | mở liên kết trên trình duyệt | bản sao do ứng dụng giữ | có | – | có |
| Đã chép | bản sao do ứng dụng giữ | bản sao do ứng dụng giữ | – | – | có |
| Lưu dự phòng | tệp lưu dự phòng | tệp | – | tệp | có |

"Mở liên kết" chỉ cho `https://`. Mở tệp bằng `ShellExecuteW` "open"; Mở thư mục bằng `explorer /select,"path"`-tương đương (`SHOpenFolderAndSelectItems` hoặc `ShellExecuteW` với `/select,`). Sao chép ảnh giải mã trên luồng nền rồi đặt lên clipboard (`clipboard::WinClipboard`). Cũng có menu chuột phải (cùng các mục, menu tối), Enter = Mở, Delete = Xóa khỏi lịch sử, Ctrl+C = sao chép ảnh, Ctrl+Shift+C = sao chép liên kết, Ctrl+F = vào ô tìm, Esc = xóa ô tìm nếu đang có chữ, không thì đóng; Lên/Xuống/Home/End/PageUp/PageDown di chuyển; nháy đúp dòng = Mở.

### 3.7 Trạng thái đặc biệt (vẽ giữa vùng thân)
- Trống, đang bật: biểu tượng khung ảnh, "Chưa có ảnh nào", "Ảnh bạn chụp, lưu hoặc tải lên từ bây giờ sẽ xuất hiện ở đây.".
- Tắt (và trống): "Lịch sử đang tắt" + câu giải thích + nút chính "Mở Tùy chọn". Tắt nhưng còn mục cũ: vẫn hiện mục cũ (không xóa gì) kèm dải thông báo màu `SURFACE` dưới thanh trên với nút "Mở Tùy chọn".
- Không khớp tìm/lọc: "Không tìm thấy ảnh nào" + "Thử từ khóa khác hoặc chọn bộ lọc Tất cả." + nút "Xóa bộ lọc".
- Chỉ mục hỏng: khi nạp, dòng hỏng bị bỏ; cả file hỏng thì đổi tên thành `index.corrupt.json` và bắt đầu danh sách trống (không sập, không im lặng mất dữ liệu).
- Xóa toàn bộ: thẻ xác nhận vẽ ngay trong cửa sổ (nền mờ phủ thân, thẻ giữa): "Xóa toàn bộ lịch sử?", "{n} mục sẽ bị gỡ khỏi lịch sử. Ảnh bạn đã lưu và các liên kết đã tải lên không bị xóa.", nút "Hủy" (mặc định, tiêu điểm) và "Xóa lịch sử" (đỏ). Esc = Hủy. (Không dùng `MessageBox` vì nó sáng, phá giao diện tối.) Xóa một mục không xác nhận (nhẹ, không xóa tệp của người dùng).

### 3.8 Lựa chọn khác với quy ước Darkshot
- Cửa sổ phụ đầu tiên đổi cỡ được và có thanh cuộn; các cửa sổ cũ (Thông tin, Trợ giúp) cố định.
- Cửa sổ này tự vẽ danh sách trong một cửa sổ (không dùng ListView) để kiểm soát hoàn toàn dáng vẻ tối. Hệ quả: danh sách không có cây truy cập UIA riêng. Nút, ô tìm, bộ lọc là điều khiển thật nên có nhãn; thanh cuộn dùng điều khiển thật. Ghi là hạn chế đã biết.

## 4. Lưu trữ và quyền riêng tư

Gốc: `%LOCALAPPDATA%\darkshot\history\` (máy-cục bộ, không roaming).
```
history\
  index.json            {"version":1,"entries":[…]}  (mới nhất trước)
  images\<id>.png       bản sao do ứng dụng giữ (chỉ loại Tải lên và Chép)
  thumbs\<id>.png       ảnh nhỏ tối đa 320×200, không phóng to
```
- Mặc định TẮT. Cấu hình `history_enabled: bool` (JSON `history_enabled`), thiếu hoặc sai kiểu = false. Tắt: không tạo mục, không giữ bản sao, không xóa mục cũ, không đụng gì tới hành động chụp.
- Giữ tối đa 50 mục. Quá thì bỏ mục cũ nhất và chỉ xóa `images\<id>.png`, `thumbs\<id>.png` của nó. Không bao giờ xóa tệp của người dùng (tệp Lưu, Lưu dự phòng) và không bao giờ đụng R2.
- Lưu: ghi tham chiếu tệp gốc (không chép lại). Tải lên: ghi liên kết + bản sao PNG. Chép: bản sao PNG. Lưu dự phòng: đường dẫn tệp thật vừa lưu. Lưu dự phòng khi thoát (pending) ghi đồng bộ vì tiến trình sắp thoát.
- Mục: `id` (mili giây UNIX 13 chữ số hex + 8 hex ngẫu nhiên), `kind` (`saved|uploaded|copied|upload_fallback`), `unix` (giây UTC), `local` (`YYYY-MM-DD HH:MM:SS` giờ địa phương lúc chụp), `width`, `height`, `bytes` (nếu biết), `path` (tệp gốc nếu có), `managed` (có `images\<id>.png`), `url`, `expires` (`YYYY-MM-DD` hoặc bỏ trống khi `Keep`).
- Ghi chỉ mục nguyên tử: ghi `index.json.tmp` rồi `rename` đè. Mọi thay đổi chạy dưới một khóa toàn tiến trình: khóa → đọc chỉ mục mới nhất từ đĩa → sửa → ghi → mở khóa; mã hóa PNG và ảnh nhỏ làm TRƯỚC khi lấy khóa. Hai lượt tải lên xong cùng lúc không mất cập nhật, không trùng mục (id ngẫu nhiên, mỗi kết quả ghi đúng một lần).
- Mã hóa PNG và ảnh nhỏ chạy trên luồng nền (`darkshot-history`), không bao giờ trên luồng giao diện. Dịch vụ ghi sao ảnh vào bộ nhớ (một bản sao) rồi giao cho luồng.
- Không ghi nhật ký nội dung ảnh, đường dẫn đầy đủ hoặc khóa. Không dùng mạng.
- Ghi chú trình bày: nhóm Tùy chọn có chú thích "Lưu lịch sử ảnh chụp trên thiết bị này. Ảnh không được gửi đến máy chủ bổ sung." / "Keep a local history of screenshots on this device. Images are never sent to an additional server."

Chế độ cách ly để chạy thử (không phải tính năng cho người dùng): biến môi trường `DARKSHOT_PROFILE_DIR=<thư mục>` đưa `%APPDATA%` và `%LOCALAPPDATA%` về `<thư mục>\Roaming` và `<thư mục>\Local`, dùng mutex một-thể-hiện khác tên, bỏ qua chuyển dữ liệu cũ, khóa Run, WER. Cờ dòng lệnh `--history` mở cửa sổ Lịch sử ngay khi khởi động (hữu ích cho demo và kiểm thử).

## 5. Ảnh nhỏ và xem trước

- Giải mã PNG bằng crate `png` hiện có (mở rộng bảng màu, 16 → 8 bit, mọi kiểu màu → BGRA). Giới hạn chống bom giải mã: mỗi cạnh ≤ 16384, tổng điểm ảnh ≤ 100 triệu, vượt thì từ chối.
- Thu nhỏ bằng lọc trung bình diện tích (box), không phóng to. Ảnh nhỏ sinh từ ảnh trong bộ nhớ lúc ghi (không giải mã lại); chỉ khi thiếu `thumbs\<id>.png` mới giải mã tệp để làm lại (luồng nền).
- Giao diện giữ bộ nhớ đệm HBITMAP cho các ảnh nhỏ đã nạp (tối đa 120, loại bỏ cũ nhất), không giải mã lặp.
- Xem trước: luồng nền giải mã tệp, thu nhỏ về vừa kích thước khung (kẹp 2400 cạnh dài), gửi `WM_APP` kèm mã thế hệ; kết quả cũ bị bỏ. Không bao giờ giữ ảnh gốc 4K trong bộ nhớ giao diện.
- Xóa mục thì xóa luôn ảnh nhỏ và bản sao của mục đó. Quét mồ côi (tệp trong `images\`, `thumbs\` không có mục) khi ghi mục mới.

## 6. Tìm và lọc

- Lọc: Tất cả; Đã lưu = `saved` + `upload_fallback`; Đã tải lên = `uploaded`; Đã chép = `copied`.
- Tìm: mọi từ (cách nhau khoảng trắng) phải khớp (AND), không phân biệt hoa thường và dấu tiếng Việt, trên: tên tệp (từ `path`), ngày (`2026-11-18` và `18/11/2026`), tên loại theo ngôn ngữ hiện tại, liên kết. Áp dụng khi gõ (không cần Enter).

## 7. Cấu hình, di chuyển, nội địa hóa

- `Config.history_enabled` + JSON `history_enabled`; tệp cũ không có trường = tắt (di chuyển không cần ghi lại). Hộp tích ở tab Thông thường của Tùy chọn kèm dòng chú thích; không thêm tab.
- Mục "Lịch sử ảnh chụp" trong menu khay đứng sau "Chụp màn hình" (trước "Tùy chọn"). Cửa sổ chỉ một thể hiện, mở lại thì đưa ra trước.
- Mọi chữ qua `darkshot_core::i18n` hai ngôn ngữ; bảng chuỗi ở Task 52. Đổi ngôn ngữ khi cửa sổ đang mở thì đóng cửa sổ (như Thông tin và Trợ giúp, `info_windows_stale`).
- Trợ giúp (`help_text`) thêm một câu về Lịch sử.

## 8. Kiểm thử và nghiệm thu

Tự động (lõi): mô hình/JSON, nạp khoan dung, chỉ mục hỏng, ghi nguyên tử, giới hạn 50, khóa và ghi đồng thời (nhiều luồng), tệp mất, ảnh nhỏ (kích thước, không phóng to, tệp hỏng), tích hợp Lưu/Chép/Tải lên/Lưu dự phòng, tắt, xóa một mục/xóa hết (chỉ tệp do ứng dụng giữ), tìm/lọc, định dạng ngày/dung lượng, `history_enabled` đi vòng và di chuyển. Tự động (Windows): bố cục (hàm thuần), bảng hành động theo loại, kiểm thử chạy tay chống rò GDI, mở/đóng cửa sổ. Chạy tay: mục "Đợt 7" của `docs/testing/manual-checklist.md`. Chỉ báo "đã qua" những gì thật sự chạy.

Demo: bản build cách ly `target/demo7/release/darkshot.exe`, dữ liệu tổng hợp trong thư mục tạm, không khóa R2, không đụng tiến trình của chủ dự án. Ảnh chụp màn hình thật của ứng dụng chạy: cửa sổ chính, xem trước, trống, tắt, Tùy chọn, tiếng Việt, tiếng Anh, tìm/lọc, xác nhận, nhiều mục.

## 9. Phương án đã cân nhắc (cách làm)

1. Móc ghi lịch sử vào `Ctx` của `run_action` (trait `HistorySink`) – bị loại: phải sửa mọi chỗ dựng `Ctx` và kiểm thử, và lõi sẽ phải biết về luồng.
2. **Chọn:** ghi dựa trên `Outcome` ở lớp Windows (hai điểm: `run_on_image` và `spawn_upload`, cộng `save_pending`), logic thuần ở lõi (`history`, `history_store`). Không đổi chữ ký lõi hiện có, kiểm thử được hoàn toàn ở lõi.
3. Lưu bằng SQLite – bị loại: thêm phụ thuộc lớn, 50 mục không cần.
