# Thiết kế: Ghim ảnh lên màn hình (Giai đoạn 3 sản phẩm, đợt 9 nội bộ)

Ngày: 2026-11-18. Người quyết: phiên điều phối (chủ dự án giao toàn quyền chọn tính năng, thiết kế giao diện, agent và model; chỉ duyệt bản demo cuối).

Số hiệu: **Giai đoạn 3 — Mở rộng năng suất** (lộ trình sản phẩm). Nội bộ: đợt 9, Task 66–71, nhánh phát triển `feature/pin-screenshot`. Giai đoạn sản phẩm, số đợt nội bộ, số task và phiên bản là bốn dãy số tách biệt.

## 1. Lộ trình sản phẩm

| Giai đoạn | Nội dung | Trạng thái |
|---|---|---|
| 1 — Chụp cơ bản | Chụp vùng, lưu PNG, clipboard, tải lên R2, khay hệ thống, Tùy chọn, phím tắt | Xong (v0.1.0) |
| 2 — Năng suất chụp | 2A Lịch sử ảnh chụp, 2B Bắt cửa sổ, Annotation Pro, thanh gọn | Xong (v0.3.0) |
| 3 — Mở rộng năng suất | **Ghim ảnh lên màn hình** (đợt này); các ứng viên khác để sau | Đang làm |

Chụp hẹn giờ không vào lộ trình: lớp phủ chụp màn hình đóng băng ngay lúc bấm phím, nên menu mở, chú thích (tooltip), hộp thoại modal đều chụp được (chủ dự án đã kiểm). Phiên này không tìm thấy tình huống nào mà luồng hiện tại không chụp được.

## 2. Chọn tính năng

### 2.1 Chấm điểm (1–5, cao là tốt; "công sức" và "rủi ro" cao là tốt = ít công sức, ít rủi ro)

| Tiêu chí | A Ghim ảnh | B Lặp lại vùng / mẫu | C Chụp cuộn | D OCR cục bộ | E Chụp từng bước |
|---|---|---|---|---|---|
| Giá trị người dùng | 5 | 3 | 4 | 4 | 3 |
| Tần suất thực tế | 4 | 3 | 2 | 3 | 2 |
| Tác động trải nghiệm | 5 | 2 | 4 | 3 | 3 |
| Hợp kiến trúc hiện tại | 5 | 5 | 2 | 3 | 3 |
| Khả thi kỹ thuật | 5 | 5 | 2 | 3 | 4 |
| Rủi ro tin cậy | 4 | 5 | 1 | 4 | 4 |
| Riêng tư | 5 | 5 | 5 | 5 | 5 |
| Hiệu năng / kích thước | 5 | 5 | 2 | 2 | 4 |
| Công sức | 4 | 5 | 1 | 3 | 2 |
| Khác biệt sản phẩm | 4 | 2 | 4 | 4 | 3 |
| **Cộng** | **46** | 40 | 27 | 34 | 33 |

### 2.2 Lý do

- **A Ghim ảnh (chọn).** Dùng lại thẳng đầu ra của lớp phủ (ảnh đã cắt, đã in hình đánh dấu) và kho Lịch sử (đường dẫn ảnh). Cửa sổ Win32 thuần, không crate mới, vài trăm KB mã. Việc thường gặp: giữ một đoạn thông số, một lỗi, một bản thiết kế trước mắt khi gõ ở ứng dụng khác; so hai trạng thái giao diện cạnh nhau. Vừa nhẹ vừa "một chạm" như triết lý darkshot.
- **B Lặp lại vùng.** `keep_selection` (đợt 2) đã giữ vị trí vùng trong phiên; phần còn lại (nhớ qua lần khởi động, mẫu cỡ cố định) có giá trị nhỏ hơn so với công sức quản lý mẫu.
- **C Chụp cuộn.** Cần cuộn cửa sổ của ứng dụng khác bằng thông điệp/SendInput, ghép ảnh theo vùng trùng, xử lý tiêu đề dính, nội dung động: rủi ro ghép sai cao, không hứa được tính tương thích. Để sau, cần thiết kế riêng có phạm vi hẹp.
- **D OCR cục bộ.** `Windows.Media.Ocr` qua WinRT là cục bộ, nhưng cần thêm feature WinRT của crate `windows` (tăng kích thước và thời gian build), gói ngôn ngữ nhận dạng tiếng Việt thường không cài sẵn: nhiều người sẽ gặp "không có ngôn ngữ". Ứng viên tốt cho giai đoạn sau.
- **E Chụp từng bước.** Cần trình sửa phiên nhiều ảnh và xuất tài liệu: một sản phẩm con, quá lớn cho một đợt.

### 2.3 Ngoài phạm vi (không làm ở đợt này)

Ghim nhóm/gộp nhiều ảnh, lưu ghim qua lần khởi động lại, vẽ thêm lên ảnh đã ghim, cắt ảnh ghim, xoay/lật, ghim chữ từ clipboard, xuyên chuột (click-through), phím tắt toàn cục "ghim lần chụp gần nhất", bóng đổ ngoài cửa sổ.

## 3. Câu chuyện người dùng

1. Chụp vùng → bấm **Ghim** (hoặc Ctrl+P) → ảnh nổi đúng chỗ vừa chụp, luôn ở trên, mình tiếp tục làm việc ở ứng dụng khác.
2. Mở Lịch sử → chọn một mục → **Ghim** → ảnh nổi giữa màn hình đang có chuột.
3. Kéo ảnh ghim đi chỗ khác, cuộn chuột để phóng/thu, Ctrl+cuộn để làm mờ cho nhìn xuyên.
4. Chuột phải → Sao chép ảnh / Lưu ảnh / Thu phóng / Độ mờ / Đóng / Đóng mọi ảnh ghim.
5. Esc hoặc nút × khi rê chuột để đóng; menu khay "Đóng mọi ảnh ghim (N)" để dọn hết.

## 4. Hành vi

### 4.1 Tạo ảnh ghim

- **Từ lớp phủ:** mục mới **Ghim** trên thanh ngang, giữa Chép và Hủy: `Hoàn tác, Làm lại | Tải lên, Lưu, Chép, Ghim, Hủy`. Phím Ctrl+P. Ảnh = đúng ảnh mà Lưu/Chép sẽ dùng (`picked_image`, có hình đánh dấu). Vị trí: ảnh hiện 1:1 đúng tọa độ màn hình của vùng đã chọn (cửa sổ = vùng nới ra một viền), nên lúc lớp phủ đóng, ảnh nằm khít chỗ cũ. Vùng được nhớ như Lưu (`remember_after`). Không thông báo (chính ảnh ghim là phản hồi). Không ghi Lịch sử (ghim không phải kết quả lưu/chép/tải).
- **Từ Lịch sử:** nút **Ghim** trong khung chi tiết (thứ tự: Mở, Sao chép ảnh, Sao chép liên kết, Mở thư mục, **Ghim**, Xóa khỏi lịch sử). Bật khi tệp ảnh của mục còn trên đĩa (như Sao chép ảnh), vô hiệu khi không. Ảnh đọc bằng `thumb::decode_png_file` (đã kiểm cỡ từ header) theo cùng đường và cùng luồng mà Sao chép ảnh của Lịch sử đang dùng (người làm kiểm và ghi lại; nếu Sao chép ảnh đọc trên luồng nền thì Ghim cũng vậy và báo về bằng `PostMessage`). Vị trí: giữa vùng làm việc của màn hình chứa con trỏ; ảnh lớn hơn 80% vùng làm việc thì thu về mức phóng lớn nhất trong bảng mức (mục 4.3) vừa 80%.
- **Giới hạn:** tối đa 32 ảnh ghim cùng lúc. Ghim thứ 33 → thông báo loại Info "Đã đủ 32 ảnh ghim. Đóng bớt rồi thử lại." và không tạo.
- **Lỗi tạo cửa sổ hoặc đọc ảnh:** thông báo Error "Không ghim được ảnh" + lý do ngắn; không ảnh hưởng luồng chụp.

### 4.2 Cửa sổ ảnh ghim

- `WS_POPUP`, `WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED` (không lên thanh tác vụ, không Alt+Tab, độ mờ qua `SetLayeredWindowAttributes(LWA_ALPHA)`). Hiện bằng `SW_SHOWNA` (không giành tiêu điểm của ứng dụng đang dùng); bấm vào thì được kích hoạt để nhận phím.
- Viền 1 DIP (tối thiểu 1 px) bao ngoài ảnh: `theme::ACCENT` khi cửa sổ đang kích hoạt, `theme::OVERLAY_BORDER` khi không. Ảnh nằm trong viền, không có thanh tiêu đề.
- Tiêu đề cửa sổ (cho trình đọc màn hình và công cụ hệ thống): "darkshot — Ảnh ghim W × H" (i18n).
- Vẽ: đệm ngoài màn hình (back buffer) cỡ cửa sổ; ảnh co giãn bằng `StretchDIBits`, chế độ `HALFTONE` khi mức < 100%, `COLORONCOLOR` (điểm ảnh sắc) khi ≥ 100%. Mức 100% chép thẳng.
- Con trỏ: `IDC_SIZEALL` trên ảnh, mũi tên trên nút ×.

### 4.3 Thao tác

| Thao tác | Kết quả |
|---|---|
| Kéo chuột trái | Dời cửa sổ (tự xử lý bằng `SetCapture`, không dùng `HTCAPTION` để tránh Aero Snap/phóng to) |
| Bấm đúp chuột trái | Về 100% (giữ điểm dưới chuột) |
| Cuộn chuột | Phóng/thu một bậc, giữ điểm dưới chuột đứng yên |
| Ctrl + cuộn | Độ mờ ±10%, trong [20%, 100%] |
| Chuột phải | Menu ngữ cảnh tối (mục 4.4) |
| Rê chuột | Hiện nút × 20 DIP ở góc trên phải (cách mép 4 DIP), chỉ khi ảnh hiển thị ≥ 48 DIP mỗi chiều; bấm × đóng |
| Esc, Ctrl+W | Đóng ảnh ghim này |
| Ctrl+C | Sao chép ảnh gốc (100%, không phụ thuộc mức phóng/độ mờ) |
| Ctrl+S | Lưu ảnh gốc như hành động Lưu (thư mục lưu, thông báo, Lịch sử như Lưu thường) |
| `+`/`=`, `-` (kể cả bàn phím số) | Phóng/thu một bậc, giữ tâm cửa sổ |
| `0` | Về 100% |
| Mũi tên | Dời 1 px; Shift + mũi tên 10 px |

Bảng mức phóng (%): 10, 25, 33, 50, 67, 75, 90, 100, 110, 125, 150, 175, 200, 250, 300, 400. Cỡ hiển thị mỗi chiều ≥ 1 px và ≤ 16384 px (bậc nào vượt thì bỏ qua). Mỗi lần đổi mức hoặc độ mờ: một nhãn nhỏ giữa đáy ảnh ("125%" hoặc "Độ mờ 60%") hiện 1,2 giây (hẹn giờ), nền `OVERLAY_LABEL`, chữ `TEXT`, bo 4 DIP; không hiện khi ảnh hiển thị nhỏ hơn nhãn.

Giữ trong màn hình: sau khi dời, phóng hoặc đổi DPI, ít nhất 32 px × 32 px của cửa sổ phải nằm trong một màn hình (dùng danh sách màn hình hiện tại). Kéo thả tự do giữa các màn hình.

### 4.4 Menu ngữ cảnh

```
Sao chép ảnh           Ctrl+C
Lưu ảnh                Ctrl+S
─────────────────────────────
Thu phóng            ▸  50% · 100% · 150% · 200%   (mức hiện tại có dấu tích)
Độ mờ                ▸  100% · 80% · 60% · 40%       (mức hiện tại có dấu tích)
─────────────────────────────
Đóng                   Esc
Đóng mọi ảnh ghim
```

Menu tối nhờ `theme::enable_dark_menus` đã có. Chữ theo ngôn ngữ hiện hành lúc mở menu.

### 4.5 Menu khay

Khi có ≥ 1 ảnh ghim: thêm mục "Đóng mọi ảnh ghim (N)" ngay sau "Lịch sử" (`tray::menu_layout(has_update, pins)`). Không có ảnh ghim: menu y như v0.3.0.

### 4.6 DPI và nhiều màn hình

- Ảnh chụp là điểm ảnh vật lý; 100% = một điểm ảnh ảnh trên một điểm ảnh màn hình, ở mọi DPI. Vì vậy ảnh ghim từ lớp phủ khít chỗ cũ trên mọi màn hình, kể cả màn hình tọa độ âm.
- `WM_DPICHANGED`: KHÔNG dùng khung gợi ý của hệ thống (sẽ phóng ảnh theo DPI); giữ góc trên trái và cỡ điểm ảnh, chỉ tính lại viền, nút ×, nhãn, phông theo DPI mới.
- Màn hình bị rút ra (`WM_DISPLAYCHANGE`): kéo ảnh ghim về theo luật "giữ trong màn hình".

### 4.7 Vòng đời và tài nguyên

- Mọi ảnh ghim sống trên luồng giao diện chính (luồng khay). Lớp cửa sổ đăng ký một lần. Mỗi cửa sổ sở hữu trạng thái `Box<Pin>` (ảnh BGRA gốc, DIB/đệm, mức, độ mờ) qua `GWLP_USERDATA`, giải phóng ở `WM_NCDESTROY`; mọi đối tượng GDI tạo ra được xóa trên mọi đường.
- Sổ ảnh ghim (`thread_local`) chỉ giữ HWND; đếm số lượng cho menu khay và giới hạn 32.
- Thoát ứng dụng (Thoát, `WM_ENDSESSION`): hủy mọi ảnh ghim trước khi thoát vòng lặp.
- Lớp phủ đang mở: ảnh ghim vẫn vẽ (vòng lặp của lớp phủ bơm thông điệp); ảnh ghim có mặt trong ảnh chụp mới (như mọi cửa sổ khác) — chủ ý.
- Bộ nhớ: ảnh 4K = ~33 MB mỗi ảnh ghim; 32 ảnh 4K ~1 GB là trường hợp cực đoan chấp nhận được, người dùng tự đóng.

## 5. Kiến trúc

- `darkshot-core/src/pin.rs` (thuần, kiểm thử đơn vị): `ZOOM_STEPS`, `zoom_in/zoom_out`, `display_size`, `zoom_about(rect, anchor, old, new)`, `fit_zoom`, `initial_rect_centered`, `keep_visible(rect, monitors, min)`, `opacity_step`, `MAX_PINS`. Chuỗi i18n mới trong `i18n.rs`, câu Trợ giúp về Ghim.
- `darkshot-core/src/actions.rs`: `Action::Pin`. `run_action(Pin)` không làm gì, trả `Outcome::Cancelled` (lớp Windows xử lý Ghim trước khi gọi `run_action`).
- `darkshot-windows/src/pin.rs`: cửa sổ ảnh ghim, sổ, API `open(image, at: PinPlace) -> Result<(), PinError>`, `count()`, `close_all()`; hành động Lưu/Chép gọi lại `main` qua một hàm đăng ký (`set_action_sink`), để `pin.rs` không phụ thuộc `main`.
- `overlay.rs`: `BarItem::Act(Action::Pin)`, biểu tượng đinh ghim vẽ như các biểu tượng khác của thanh, Ctrl+P, chú thích "Ghim lên màn hình (Ctrl+P)".
- `main.rs`: nhánh `Action::Pin` trong `capture_flow` → `pin::open`; mục menu khay; dọn khi thoát.
- `history_layout.rs`/`history_window.rs`: nút Ghim thứ 6, bố cục xuống dòng không cắt chữ.

Không crate mới. Feature `windows` mới chỉ khi thiếu (kiểm `Cargo.toml`).

## 6. Hồi quy phải giữ

Chụp vùng, bắt cửa sổ, toàn màn hình, menu/modal (màn hình đóng băng), Lưu, Tải lên, lưu dự phòng, Chép, Annotation Pro, Hoàn tác/Làm lại, Lịch sử, Tùy chọn, khay, kiểm tra cập nhật, hai ngôn ngữ. Thanh ngang thêm một ô: kiểm thử `bar_layout_order` cập nhật theo thiết kế này (không phải nới kiểm thử).

## 7. Nợ kỹ thuật cùng đợt

Task 66: viên nhắc của Bắt cửa sổ còn đọng sau khi BẤM chọn (quyết định 100). Tìm nguyên nhân gốc; chỉ sửa khi có nguyên nhân; thêm kiểm thử hồi quy; nếu không tái hiện được thì ghi rõ và giữ mục thử tay.

## 8. Tiêu chí nghiệm thu

1. Ghim từ lớp phủ: ảnh nằm khít chỗ vùng đã chọn ở DPI 100/125/150/200% (người làm kiểm bằng hàm thuần + bản demo ở DPI của máy).
2. Ghim từ Lịch sử: ảnh hiện giữa màn hình có chuột, không vượt 80% vùng làm việc.
3. Kéo, cuộn phóng/thu giữ điểm dưới chuột, Ctrl+cuộn độ mờ, bấm đúp về 100%, phím tắt, menu ngữ cảnh, nút ×, menu khay đều chạy.
4. Sao chép/Lưu từ ảnh ghim cho ảnh gốc đủ cỡ.
5. 32 ảnh ghim tối đa; mở/đóng 50 lần không rò GDI/USER (kiểm thử chạy tay).
6. Thoát ứng dụng khi còn ảnh ghim: không treo, không rò.
7. Mọi chữ hai ngôn ngữ, không cắt chữ.
8. `scripts/check.ps1` in `ALL CHECKS PASSED`; không hồi quy mục 6.
