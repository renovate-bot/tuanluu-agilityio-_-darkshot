# Thanh công cụ gọn (sau đợt 6)

Ngày: 2026-11-17. Trạng thái: chủ dự án chọn phương án "Gộp nhóm + dời" (2026-11-17).

## Vì sao

Sau đợt 6, thanh dọc của lớp phủ có 14 nút, cao khoảng 610 DIP. Chủ dự án thấy quá dài: thanh che ảnh, và ở màn hình thấp hoặc DPI cao thì thanh phải xếp hai cột. Mục tiêu là rút gọn thanh mà không mất công cụ hay phím tắt nào.

## Các phương án đã xét

| Phương án | Kết quả | Đánh đổi |
|---|---|---|
| **Gộp nhóm Hình + dời Hoàn tác/Làm lại sang thanh ngang** (chọn) | 9 nút, cao ~390 DIP | Đổi giữa các hình cần thêm một cú bấm (phím tắt vẫn đi thẳng) |
| Chỉ gộp nhóm Hình | 11 nút, ~480 DIP | Gọn ít hơn |
| Luôn hai cột | 7 hàng, ~300 DIP | Rộng gấp đôi, che ảnh theo chiều ngang |
| Thu nhỏ nút 36→30, khe 6→4 | 14 nút, ~500 DIP | Nút nhỏ, khó bấm |

## Thiết kế

### Thanh dọc (9 mục)

Chọn | Bút, **Hình**, Bút dạ quang, Làm mờ, Chữ, Đánh số | Cỡ, Màu.

Vạch ngăn trước mục 1 và mục 7. Bố cục vẫn đi qua `geometry::side_layout`, nên khi một cột không vừa màn hình thì thanh vẫn xếp hai cột như đợt 6.

### Nút Hình

- Nhóm Hình gồm Đường thẳng, Mũi tên, Hình chữ nhật, Hình elip, theo thứ tự này.
- Nút hiện biểu tượng của **hình nhớ gần nhất**, kèm một tam giác nhỏ ở góc dưới phải để báo có bảng bật ra.
- Hình nhớ mặc định là Mũi tên. Hình nhớ được giữ qua các lần mở lớp phủ trong cùng tiến trình, giống màu và mức cỡ.
- Vạch "đang chọn" bật khi công cụ hiện hành là một trong bốn hình.
- Bấm nút khi công cụ hiện hành **không** phải hình: bật hình nhớ, không mở bảng. Như vậy chỉ một cú bấm là vẽ tiếp được.
- Bấm nút khi công cụ hiện hành **là** hình: mở bảng Hình, hoặc đóng nếu đang mở.
- Bảng Hình gồm 4 ô vuông `BUTTON` xếp ngang. Bảng đặt và trông giống bảng Cỡ (`flyout_rect`, canh theo nút Hình). Ô của công cụ hiện hành có viền `ACCENT`. Rê chuột lên ô thì hiện gợi ý tên công cụ kèm phím.
- Bấm một ô: bật hình đó, nhớ nó, đóng bảng.
- Bảng Hình, bảng Cỡ và bảng màu không bao giờ mở cùng lúc. Bấm ngoài bảng chỉ đóng bảng. Esc đóng bảng; đây là bậc đầu của Esc năm bậc.
- Phím L, A, R, E vẫn chọn thẳng công cụ và cập nhật hình nhớ. Bấm lại phím của công cụ đang bật thì vẫn tắt như hôm nay.
- Gợi ý của nút: "Hình (L, A, R, E)" / "Shapes (L, A, R, E)". Đây là mục `Msg` mới.

### Thanh ngang

Hoàn tác, Làm lại | Tải lên, Lưu, Chép, Hủy.

- Hoàn tác và Làm lại rời thanh dọc sang đầu thanh ngang, có vạch ngăn trước Tải lên.
- Biểu tượng, gợi ý, trạng thái mờ và hành vi giữ nguyên như ở thanh dọc.
- Thanh ngang vẫn đặt bằng `place_toolbar`, chỉ đổi chiều rộng.
- Việc tính ô của từng mục và tìm mục dưới chuột là hàm thuần, có kiểm thử. Hiện mã đang chia đều chiều rộng cho 4 nút; cách này phải bỏ.

### Không đổi

Phím tắt, sổ hoàn tác, công cụ Chọn, bảng Cỡ, bảng màu, ảnh ra.

## Ngoài phạm vi

Gộp các nhóm khác, cho người dùng tự sắp thanh, thanh nằm ngang cho công cụ vẽ.

## Để lại cho người làm

Hình vẽ tam giác góc, tên trường trạng thái, cách tách hàm thuần cho thanh ngang.
