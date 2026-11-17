# Thiết kế đợt 5: công cụ đánh dấu trên ảnh chụp

Ngày: 2026-11-17. Trạng thái: chờ chủ dự án duyệt.

## 1. Ý định

Chủ dự án muốn đánh dấu lên vùng vừa chọn trước khi tải lên, lưu hoặc chép, bằng một thanh công cụ dọc cạnh vùng chọn như ảnh họ gửi (bút vẽ, đường thẳng, mũi tên, hình chữ nhật, bút dạ quang, chữ, chọn màu, hoàn tác). Họ đồng ý làm đủ tám mục và giao phần còn lại cho điều phối viên quyết. Các chỗ ghi "(điều phối viên chọn)" là quyết định chưa được chủ dự án xác nhận; họ duyệt thiết kế này là duyệt luôn các chỗ đó.

Thành công khi: chọn vùng, vẽ vài nét, bấm Tải lên/Lưu/Chép, ảnh nhận được có đúng các nét đã thấy trên màn hình, ở cùng vị trí, cùng màu, cùng độ mịn; không chọn công cụ nào thì lớp phủ chạy như hôm nay.

## 2. Mục tiêu, ngoài phạm vi, điều không được xảy ra

**Mục tiêu**
- Thanh công cụ dọc, tám nút: Bút vẽ, Đường thẳng, Mũi tên, Hình chữ nhật, Bút dạ quang, Chữ, Màu, Hoàn tác.
- Nét vẽ được "in" vào ảnh đã cắt trước khi mã hóa, nên ảnh lưu, ảnh tải lên và ảnh chép giống nhau.
- Gõ được tiếng Việt trong công cụ Chữ bằng Unikey/EVKey và bộ gõ tiếng Việt của Windows.

**Ngoài phạm vi (đợt này)**
- Chọn, di chuyển, sửa, xóa từng hình đã vẽ (chỉ có hoàn tác hình cuối). Làm lại (redo).
- Chọn màu tự do (chỉ bảng màu có sẵn), chọn độ dày, chọn cỡ chữ, phông chữ, hình tròn, làm mờ (blur/pixelate), đánh số thứ tự.
- Công cụ vẽ cho hai phím tắt toàn màn hình (Lưu ảnh, Tải lên nhanh): chúng không mở lớp phủ, giữ nguyên.
- Nhớ nét vẽ giữa hai lần chụp; nhớ công cụ hay màu đã chọn sau khi đóng ứng dụng.

**Điều không được xảy ra**
- Ảnh đầu ra khác thứ đã thấy trên màn hình (lệch vị trí, khác màu, mất nét, thêm viền hay màu giao diện).
- Mất nét vẽ vì một cú kéo nhầm (ví dụ kéo ra ngoài vùng chọn tạo vùng mới).
- Lớp phủ chậm thấy rõ khi vẽ nét dài trên hai màn hình 4K.
- Rò GDI/GDI+, panic, hoặc chặn ứng dụng khi gõ chữ.
- Thay đổi hành vi hiện có khi không dùng công cụ vẽ: chọn vùng, kéo tay nắm, thanh bốn nút, Ctrl+D/S/C, Esc, "Giữ vị trí vùng".

## 3. Ràng buộc

- Rust thuần, không thêm crate. Vẽ bằng GDI+ (đã bật `Win32_Graphics_GdiPlus`, lớp phủ đã khởi động GDI+); chữ bằng GDI+ với phông hệ thống (Segoe UI).
- `darkshot-core` không phụ thuộc crate `windows`: mô hình dữ liệu, hoàn tác, hình học thuần nằm ở core; vẽ nằm ở lớp Windows.
- Mọi chữ hiển thị (chú thích nút, tên màu cho trình đọc màn hình nếu có) lấy từ `darkshot_core::i18n`, đủ hai ngôn ngữ.
- Giao diện tối của đợt 4: màu từ `darkshot_core::theme`, cặp chữ/nền mới vào `TEXT_PAIRS`.
- Release `panic = "abort"`; `unsafe` nhỏ kèm `// SAFETY:`; giải phóng mọi handle trên mọi đường đi.
- Quy trình dự án như các đợt trước: gói task, nhánh `task/NN-...`, `scripts/check.ps1`.

## 4. Các cách đã cân nhắc

1. **Vẽ trong lớp phủ, cùng một hàm vẽ cho màn hình và cho ảnh ra (chọn).** Mỗi hình lưu ở tọa độ ảnh chụp. Lớp phủ vẽ các hình lên bộ đệm sau; khi xong, đúng hàm đó vẽ lên ảnh đã cắt (dịch theo góc vùng chọn). Một đường vẽ duy nhất nên ảnh ra khớp màn hình. Tốn công nhất ở phần tương tác trong `overlay.rs`.
2. **Mở cửa sổ biên tập riêng sau khi chọn vùng.** Tách khỏi lớp phủ, dễ thêm tính năng về sau, nhưng thêm một bước và một cửa sổ, khác hẳn hình chủ dự án gửi (thanh nằm ngay cạnh vùng chọn). Loại.
3. **Vẽ thẳng vào ảnh chụp (raster) mỗi nét.** Hoàn tác phải giữ bản sao ảnh cho mỗi bước (tốn bộ nhớ trên ảnh 4K nhiều màn hình), đổi vùng chọn sau khi vẽ khó. Loại.

## 5. Hành vi

### 5.1 Thanh công cụ vẽ

- Hiện khi đã có vùng chọn và không đang kéo, cùng lúc với thanh bốn nút hiện có. Thanh dọc, cùng kiểu với thanh bốn nút (nền `SURFACE`, viền `OVERLAY_BORDER`, nút 36 DIP, biểu tượng nét mảnh `OVERLAY_ICON`, khi trỏ nền `OVERLAY_HOVER` biểu tượng `ACCENT`, có chú thích khi trỏ). Nút công cụ đang chọn: nền `OVERLAY_HOVER`, biểu tượng `ACCENT`, thêm vạch `ACCENT` 2 DIP ở mép trái. Biểu tượng tự vẽ bằng GDI+, không chép biểu tượng của ứng dụng khác.
- Thứ tự từ trên xuống: Bút vẽ, Đường thẳng, Mũi tên, Hình chữ nhật, Bút dạ quang, Chữ, vạch ngăn, Màu (ô vuông tô màu hiện tại), vạch ngăn, Hoàn tác (mờ khi chưa có gì để hoàn tác).
- Vị trí (điều phối viên chọn): bên phải vùng chọn, cách 6 DIP, mép dưới thẳng mép dưới vùng chọn; không đủ chỗ bên phải thì bên trái; không đủ cả hai thì nằm trong vùng chọn sát mép phải. Luôn nằm trọn trong màn hình chứa góc dưới phải vùng chọn; không chồng lên thanh bốn nút (nếu chồng thì dịch lên trên thanh bốn nút).
- Chú thích khi trỏ, kèm phím tắt: Bút vẽ (P), Đường thẳng (L), Mũi tên (A), Hình chữ nhật (R), Bút dạ quang (H), Chữ (T), Màu, Hoàn tác (Ctrl+Z).

### 5.2 Chế độ

- Mặc định không công cụ nào được chọn: lớp phủ chạy đúng như hôm nay.
- Bấm một nút công cụ (hoặc phím tắt một chữ khi không đang gõ chữ) thì chọn công cụ đó; bấm lại nút đang chọn thì bỏ chọn.
- Khi có công cụ đang chọn: kéo bên trong vùng chọn là vẽ; tay nắm ở viền và góc vẫn đổi cỡ vùng chọn (kiểm tay nắm trước); kéo giữa vùng chọn không di chuyển vùng (vì đang vẽ). Con trỏ: chữ thập cho công cụ vẽ, chữ I cho Chữ.
- Đã có ít nhất một hình hoặc đang chọn công cụ: kéo ở ngoài vùng chọn **không** tạo vùng mới (tránh mất nét vẽ). Muốn chọn lại từ đầu: hoàn tác hết rồi bỏ chọn công cụ, hoặc Esc.
- Vẽ chỉ có tác dụng trong vùng chọn: điểm bắt đầu phải nằm trong vùng chọn; nét kéo ra ngoài vẫn được ghi nhưng hiển thị và in ra bị cắt theo vùng chọn. Đổi cỡ hoặc di chuyển vùng chọn sau khi vẽ (di chuyển được khi không có công cụ nào đang chọn): hình giữ nguyên vị trí trên màn hình (lưu theo tọa độ ảnh chụp), phần nằm ngoài vùng bị ẩn và không in ra; hình không bị xóa, kéo vùng về lại thì hiện lại. (điều phối viên chọn)
- Esc: đang gõ chữ thì hủy ô chữ đang gõ; còn lại đóng lớp phủ như hôm nay (mất mọi nét vẽ). Ctrl+D/S/C và bốn nút hành động vẫn chạy khi có công cụ đang chọn; nếu đang gõ chữ thì chốt ô chữ trước.
- Ctrl+Z: hoàn tác hình cuối (cả khi đang gõ chữ: nếu ô chữ còn nội dung thì Ctrl+Z xóa ô chữ đó trước). Không có làm lại.

### 5.3 Công cụ

Độ dày tính theo DIP của màn hình chứa điểm đầu nét, đổi sang pixel ảnh lúc bắt đầu nét và lưu luôn trong hình (in ra đúng độ dày đã thấy). (điều phối viên chọn các số dưới đây)

| Công cụ | Hành vi | Hình |
|---|---|---|
| Bút vẽ | Kéo để vẽ tự do | Nét 3 DIP, đầu và chỗ nối tròn, mịn |
| Đường thẳng | Kéo từ điểm đầu tới điểm cuối; giữ Shift thì khóa góc theo bước 45° | Nét 3 DIP, đầu tròn |
| Mũi tên | Như đường thẳng, đầu mũi tên ở điểm cuối | Thân 3 DIP; đầu tam giác đặc dài 4 lần độ dày (tối thiểu 12 DIP), góc mở 2×28°; thân dừng ở gốc đầu mũi tên để không lộ đầu tròn |
| Hình chữ nhật | Kéo theo đường chéo; giữ Shift thì thành hình vuông | Viền 3 DIP, không tô, góc vuông |
| Bút dạ quang | Kéo tự do | Nét 16 DIP, đầu vuông, cùng màu đang chọn với độ phủ 40%; trong cùng một nét chỗ chồng không đậm thêm |
| Chữ | Bấm để đặt ô chữ ở đó rồi gõ | Phông Segoe UI đậm 18 DIP, màu đang chọn, có viền chữ 2 DIP màu tương phản (đen hoặc trắng theo độ sáng của màu) để đọc được trên mọi nền |

Kéo quá ngắn (dưới 2 px ảnh) với Đường thẳng, Mũi tên, Hình chữ nhật thì không tạo hình. Bút vẽ và Bút dạ quang bấm một cái tạo một chấm.

### 5.4 Chữ

- Bấm trong vùng chọn khi công cụ Chữ đang chọn: tạo ô chữ tại đó (nếu đang có ô chữ khác thì chốt ô đó trước). Ô đang gõ có khung nét đứt mảnh `ACCENT` và con trỏ nháy; khung không in ra ảnh.
- Enter xuống dòng. Ô chữ được chốt khi: bấm ra chỗ khác, đổi công cụ, bấm nút hành động hoặc Ctrl+D/S/C. Ô rỗng khi chốt thì bỏ.
- Phím đang gõ: chữ, Backspace, Delete, mũi tên trái/phải/lên/xuống, Home, End; Ctrl+V dán chữ thuần (bỏ ký tự điều khiển trừ xuống dòng). Không có chọn chữ bằng chuột hay Shift.
- Khi đang gõ, phím tắt một chữ của công cụ không có tác dụng (chữ đi vào ô).
- Tiếng Việt: nhận ký tự qua `WM_CHAR` (Unikey/EVKey gửi Backspace rồi ký tự); bộ gõ của Windows: để Windows hiện chuỗi đang soạn ở cửa sổ soạn mặc định đặt tại vị trí con trỏ (`ImmSetCompositionWindow`), nhận kết quả qua `WM_CHAR`. Phím tắt Esc toàn cục mà lớp phủ đăng ký không được nuốt Esc của bộ gõ khi đang soạn.
- Đổi màu khi đang gõ: đổi màu ô chữ đang gõ.

### 5.5 Màu

- Bấm nút Màu: mở bảng 8 ô màu (2 hàng × 4) cạnh nút, nền `SURFACE`, viền `OVERLAY_BORDER`; ô đang chọn có viền `TEXT` 2 DIP. Bấm một ô: chọn màu, đóng bảng. Bấm ra ngoài hoặc Esc: đóng bảng (Esc ở đây không đóng lớp phủ).
- Bảng màu (điều phối viên chọn): đỏ `#FF3B30` (mặc định), cam `#FF9500`, vàng `#FFCC00`, xanh lá `#34C759`, xanh lam `#0A84FF`, tím `#AF52DE`, trắng `#FFFFFF`, đen `#000000`.
- Màu dùng cho hình vẽ sau đó; không đổi màu hình đã vẽ (trừ ô chữ đang gõ). Màu nhớ trong lúc ứng dụng chạy (lần chụp sau mở với màu vừa dùng); công cụ không nhớ (mỗi lần mở lớp phủ đều không có công cụ nào được chọn).

### 5.6 Ảnh đầu ra

- `pick_region` trả thêm danh sách hình. Sau khi cắt ảnh, nếu danh sách không rỗng thì vẽ các hình lên ảnh đã cắt bằng đúng hàm vẽ của lớp phủ, dịch theo góc vùng chọn, cắt theo khung ảnh. Không có hình thì ảnh như hôm nay (đúng từng byte).
- Ảnh vẽ xong vẫn là BGRA đục (alpha 255 mọi điểm), như ảnh hiện nay.
- Ghi nhớ vùng chọn ("Giữ vị trí vùng") không đổi.

## 6. Để lại cho người làm

- Cấu trúc dữ liệu hình (đề xuất: `enum Shape { Pen{points,width,color}, Line{..}, Arrow{..}, Rect{..}, Marker{points,width,color}, Text{origin,lines,size,color} }` ở core, tọa độ ảnh `i32` hoặc `f32`), giản lược điểm cho nét tự do (bỏ điểm cách điểm trước dưới 1 px).
- Cách vẽ lại hiệu quả trong lớp phủ (chỉ vẽ lại vùng bị ảnh hưởng; hình đã chốt có thể vẽ sẵn vào một lớp đệm).
- Hình học biểu tượng các nút.
- Cách tách task: theo kế hoạch.
