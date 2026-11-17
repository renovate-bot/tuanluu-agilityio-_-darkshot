# Thiết kế: chọn màu tự do cho công cụ đánh dấu

Ngày: 2026-11-17. Trạng thái: đã duyệt (chủ dự án chọn "Tự vẽ, giao diện tối" sau khi xem hai phương án; gửi ảnh hộp chọn màu Windows đã mở phần Define Custom Colors làm mẫu).

## Ý định

Bảng màu của công cụ đánh dấu (hiện 48 màu cơ bản, Task 39) có thêm phần tự chọn màu giống hộp chọn màu của Windows khi đã mở "Define Custom Colors": ô sắc độ, thanh độ sáng, ô xem màu, sáu ô số Hue/Sat/Lum/Red/Green/Blue, 16 ô màu tự chọn và nút thêm vào màu tự chọn. Vẽ trong lớp phủ, giao diện tối (quyết định 45, 69: không mở hộp thoại của Windows).

Ngoài phạm vi: nhập mã hex, ống hút màu từ màn hình, độ trong suốt.

## Hành vi

**Dạng gọn (mặc định khi bấm nút Màu):** nhãn "Màu cơ bản", lưới 48 màu (như Task 39), nhãn "Màu tự chọn", lưới 16 ô (2 hàng × 8), nút rộng "Định nghĩa màu tự chọn >>". Bấm ô màu cơ bản hoặc ô tự chọn đã có màu: chọn màu đó và đóng bảng (như hôm nay). Bấm ô tự chọn trống: không làm gì. Bấm nút "Định nghĩa…": mở rộng bảng sang dạng đủ, nút đó mờ đi.

**Dạng đủ:** thêm bên phải:
- Ô sắc độ: trục ngang là Hue 0→239 (trái → phải), trục dọc là Sat 240→0 (trên → dưới), vẽ ở Lum 120. Dấu chữ thập nhỏ ở (Hue, Sat) hiện tại.
- Thanh độ sáng dọc bên phải ô sắc độ: màu (Hue, Sat) hiện tại với Lum 240 ở trên → 0 ở dưới; tam giác chỉ vị trí Lum hiện tại ở bên phải thanh.
- Ô xem màu hiện tại dưới ô sắc độ, cạnh sáu ô số: Hue, Sat, Lum (cột trái), Red, Green, Blue (cột phải).
- Nút "Thêm vào màu tự chọn" ở đáy cột phải; hàng nút "OK" và "Hủy" ở đáy cột trái.
- Kéo hoặc bấm trong ô sắc độ đổi Hue, Sat (giữ Lum); kéo hoặc bấm trên thanh đổi Lum. Sửa một ô số đổi các ô còn lại và hai vùng trên.
- Ô số: bấm để chọn ô (viền `ACCENT`), gõ chữ số (tối đa 3), Backspace; Tab/Shift+Tab sang ô kế; mũi tên lên/xuống tăng/giảm 1; giá trị kẹp vào khoảng (Hue 0–239, Sat/Lum 0–240, RGB 0–255) khi rời ô hoặc Enter. Không có con trỏ nháy; ô đang chọn hiện số đang gõ.
- Ở dạng đủ, bấm ô màu cơ bản hoặc ô tự chọn có màu: đặt màu đó làm màu hiện tại của phần tự chọn (không đóng). Bấm ô tự chọn (trống hoặc không) cũng đánh dấu nó là ô đích (viền `TEXT`).
- "Thêm vào màu tự chọn": ghi màu hiện tại vào ô đích nếu có; không có thì vào ô trống đầu tiên; hết ô trống thì ghi đè lần lượt từ ô 0 (vòng tròn). Ô đích chuyển sang ô kế tiếp.
- "OK" hoặc Enter (khi không có ô số đang sửa): dùng màu hiện tại cho công cụ vẽ (và ô chữ đang gõ), đóng bảng. "Hủy", Esc hoặc bấm ra ngoài: đóng bảng, màu vẽ không đổi. Màu tự chọn đã thêm vẫn được giữ dù đóng bằng cách nào.
- Lần mở bảng sau trong cùng lần chụp mở lại ở dạng đã dùng lần trước; mở lớp phủ mới thì luôn ở dạng gọn.

**Màu tự chọn được lưu** vào file cấu hình (trường mới, file cũ không có trường vẫn đọc được; bản cũ của darkshot đọc file mới vẫn chạy), giữ sau khi khởi động lại. Ghi file khi lớp phủ đóng và màu tự chọn đã đổi; ghi lỗi thì bỏ qua, không thông báo (mất màu tự chọn không đáng làm phiền).

**HSL theo cách của Windows:** thang 0–240 (`HLSMAX = 240`), Hue hiển thị 0–239, xám có Hue 160 và Sat 0; chuyển đổi theo thuật toán `ColorRGBToHLS`/`ColorHLSToRGB` của Windows, viết bằng Rust thuần ở core.

## Nhìn

Nền `SURFACE`, viền `OVERLAY_BORDER`, chữ `TEXT` (nhãn), `TEXT_MUTED` (nhãn ô số), ô số nền `INPUT` viền `INPUT_BORDER` (đang chọn `ACCENT`), nút như nút phụ của Tùy chọn (`BUTTON`, viền `BUTTON_BORDER`; OK là nút chính `ACCENT_BUTTON` chữ trắng), ô tự chọn trống nền `INPUT` viền `OVERLAY_BORDER`. Phông như chú thích của lớp phủ. Kích thước tính theo DPI; cả bảng nằm trọn trong màn hình chứa nút Màu (kẹp; nếu màn hình quá hẹp thì đặt bảng sát mép trái màn hình).

Bố cục (DIP, ở 100%): lề 8; ô màu 20, khe 4; cột trái rộng 8 ô = 188; nhãn cao 18; nút cao 26. Cột phải: ô sắc độ 200 × 180, thanh độ sáng rộng 14 cách 8, tam giác 8; dưới đó ô xem màu 56 × 40 và hai cột ô số (nhãn 40, ô 40 × 22, hàng cách 4); nút "Thêm vào màu tự chọn" rộng hết cột phải. (Điều phối viên chọn; người làm được chỉnh vài DIP để chữ hai ngôn ngữ vừa, ghi lại.)

## Chữ (i18n)

`PickerBasic` "Màu cơ bản"/"Basic colors"; `PickerCustom` "Màu tự chọn"/"Custom colors"; `PickerDefine` "Định nghĩa màu tự chọn >>"/"Define custom colors >>"; `PickerAdd` "Thêm vào màu tự chọn"/"Add to custom colors"; `PickerHue` "Sắc độ"/"Hue"; `PickerSat` "Bão hòa"/"Sat"; `PickerLum` "Độ sáng"/"Lum"; `PickerRed` "Đỏ"/"Red"; `PickerGreen` "Lục"/"Green"; `PickerBlue` "Lam"/"Blue"; nút OK dùng `i18n::OK_LABEL`; Hủy dùng `Msg` hủy đã có (người làm tìm; không có thì thêm `PickerCancel` "Hủy"/"Cancel").

## Không được xảy ra

Mất nét vẽ hay đóng lớp phủ khi thao tác trong bảng; chuyển đổi màu sai lệch so với Windows quá 1 đơn vị; file cấu hình cũ không đọc được; khóa R2 bị ghi lại sai khi lưu màu tự chọn (dùng đúng đường lưu cấu hình hiện có); vẽ ô sắc độ chậm thấy rõ (dựng ảnh ô sắc độ một lần cho mỗi DPI, không tính lại mỗi lần vẽ).
