# Thiết kế: công cụ Làm mờ (che thông tin nhạy cảm)

Ngày: 2026-11-17. Trạng thái: chờ chủ dự án duyệt. Các chỗ ghi "(điều phối viên chọn)" là đề xuất; duyệt thiết kế này là duyệt luôn các chỗ đó.

## Ý định

Chủ dự án muốn thêm công cụ làm mờ vào thanh công cụ đánh dấu, đặt cạnh Bút dạ quang, để **che thông tin nhạy cảm sao cho không khôi phục được**, an toàn nhất có thể. Thiết kế đợt 5 đã để "làm mờ" ngoài phạm vi; thiết kế này bổ sung, không sửa thiết kế đợt 5.

Thành công khi: kéo một hình chữ nhật lên chữ (mật khẩu, email, số tài khoản), ảnh lưu, chép hoặc tải lên không còn điểm ảnh nào trong vùng che mang thông tin của chữ bên dưới; vùng che trông giống một mảng mờ hòa với nền.

## Mục tiêu, ngoài phạm vi, điều không được xảy ra

**Mục tiêu:** một công cụ mới "Làm mờ", kéo hình chữ nhật, tô kín bằng một màu trung bình duy nhất của vùng ("kính mờ giả").

**Ngoài phạm vi:** làm nhòe Gaussian, vỡ ô vuông (pixelate), cọ làm mờ tự do, chọn độ mờ, làm mờ cho hai phím tắt toàn màn hình.

**Không được xảy ra:**
- Bất kỳ điểm ảnh nào bên trong vùng che ở ảnh ra phụ thuộc vào nội dung vùng đó, ngoài một màu trung bình duy nhất.
- Ảnh ra khác thứ đã thấy trên màn hình.
- Có đường nào (lưu, chép, tải lên, JPEG hay PNG) xuất ra ảnh chưa in vùng che.
- Thay đổi hành vi của các công cụ hiện có.

## Các cách đã cân nhắc

1. **Làm nhòe Gaussian.** Chữ nhòe vẫn còn hình dạng và đoán ngược được. Loại.
2. **Vỡ ô vuông.** Mỗi ô giữ màu trung bình của phần chữ bên dưới; công cụ như Depix đã khôi phục được chữ từ ảnh vỡ ô. Loại.
3. **Hộp che đặc màu đang chọn.** An toàn tuyệt đối, đơn giản, nhưng không có cảm giác "làm mờ".
4. **Kính mờ giả: tô kín bằng một màu trung bình của vùng (đã chọn).** An toàn ngang cách 3 (chỉ lộ một màu trung bình của cả vùng, không suy ra được chữ), trông hòa với nền. Chủ dự án chọn cách này.

## Hành vi

- **Nút:** công cụ thứ bảy, ngay dưới Bút dạ quang, trên Chữ. Chú thích "Làm mờ (B)" / "Blur (B)", lấy từ `i18n`. Phím tắt B (đang trống). Biểu tượng tự vẽ bằng GDI+: một hình chữ nhật nét mảnh có vài vạch chéo bên trong, cùng kiểu với các nút khác. (điều phối viên chọn)
- **Thao tác:** như Hình chữ nhật: kéo theo đường chéo, giữ Shift thì thành hình vuông, kéo dưới 2 px ảnh thì không tạo hình. Ctrl+Z hoàn tác như mọi hình khác. Con trỏ chữ thập.
- **Màu che:** trung bình cộng R, G, B của **ảnh chụp gốc** (không tính nét đã vẽ) trên phần hình chữ nhật nằm trong ảnh chụp. Tính khi thả chuột và lưu luôn vào hình; trong lúc kéo, xem trước bằng cách tính tương tự (người làm được lấy mẫu thưa cho mượt, miễn là màu lúc thả chuột được lưu lại và dùng cho cả màn hình lẫn ảnh ra). Màu đang chọn ở bảng màu không ảnh hưởng tới công cụ này. (điều phối viên chọn)
- **Phủ:** lõi đục 100% (alpha 255) phủ đúng trọn hình chữ nhật đã kéo. Mép mềm (nếu có) chỉ nằm **phía ngoài** hình chữ nhật, rộng tối đa 3 DIP, chuyển dần từ màu che sang ảnh bên dưới; không được làm lộ điểm ảnh nào bên trong hình. (điều phối viên chọn)
- **Thứ tự:** vùng che là một hình như các hình khác, vẽ theo thứ tự tạo. Nét vẽ sau nằm trên vùng che; vùng che tạo sau phủ lên nét vẽ trước.
- **Đổi cỡ hoặc di chuyển vùng chọn:** theo quyết định 56: hình giữ yên trên màn hình, phần ngoài vùng chọn bị ẩn và không in ra.
- **Ảnh ra:** in qua đúng hàm vẽ dùng chung `annotate_draw::draw_shapes` (quyết định 57); in thất bại thì dừng hành động, không xuất ảnh thiếu vùng che.

## Ràng buộc

Như đợt 5: Rust thuần, không thêm crate; mô hình và phép tính màu trung bình (thuần) ở `darkshot-core`, vẽ ở lớp Windows; chữ từ `i18n` đủ hai ngôn ngữ; màu giao diện từ `theme`; `unsafe` nhỏ có `// SAFETY:`; không rò GDI/GDI+; release `panic = "abort"`. Quy trình gói task, nhánh `task/NN-...`, `scripts/check.ps1`.

## Kiểm thử bắt buộc

- Core: màu trung bình đúng trên ảnh mẫu (vùng cắt mép ảnh, vùng một điểm ảnh, vùng ngoài ảnh).
- Lớp Windows: in một vùng che lên ảnh có chữ, mọi điểm ảnh bên trong hình chữ nhật đều bằng đúng màu che; ảnh không có hình nào vẫn ra đúng từng byte như trước.

## Để lại cho người làm

Tên biến thể `Shape` (đề xuất `Shape::Blur { a, b, color }`), cách tính trung bình nhanh khi xem trước, hình học biểu tượng, có làm mép mềm phía ngoài hay không (bỏ được nếu khó giữ khớp màn hình và ảnh ra), cách tách task.
