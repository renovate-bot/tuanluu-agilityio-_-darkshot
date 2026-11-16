
## Vỏ ứng dụng

- [ ] Khởi động `shotme.exe`: biểu tượng xanh có khung chọn trắng hiện ở khay hệ thống, chú thích "shotme".
- [ ] Bấm chuột phải (hoặc trái) vào biểu tượng: menu có ba mục "Chụp màn hình", "Cài đặt", "Thoát".
- [ ] Chọn "Cài đặt": hiện thông báo "Chưa có".
- [ ] Chọn "Chụp màn hình" hoặc bấm PrintScreen: lớp phủ mở (xem mục "Lớp phủ").
- [ ] Khi Lightshot (hoặc ứng dụng khác giữ PrintScreen) đang chạy: shotme vẫn chạy và hiện thông báo "Phím PrintScreen đang bị ứng dụng khác chiếm".
- [ ] Chạy `shotme.exe` lần thứ hai: bản thứ hai thoát ngay, chỉ còn một tiến trình.
- [ ] Khởi động lại Explorer (Task Manager, Restart): biểu tượng khay hiện lại.
- [ ] Chọn "Thoát": biểu tượng biến mất khỏi khay và tiến trình kết thúc.
- [ ] Bật tùy chọn khởi động cùng Windows, đăng xuất rồi đăng nhập lại: biểu tượng shotme xuất hiện ở khay (kể cả khi Explorer khởi động chậm).

## Lớp phủ

Tạm thời (đến Task 11) chọn xong một hành động chỉ hiện thông báo "Đã chọn {w} × {h}, hành động {tên}"; hủy thì không có thông báo.

- [ ] Bấm PrintScreen khi đang xem một video hoặc trang có chuyển động: màn hình đứng yên và tối đi, không nháy, không thấy con trỏ chuột trong ảnh nền.
- [ ] Mở lớp phủ từ menu khay: menu khay không lọt vào ảnh nền.
- [ ] Bấm PrintScreen khi đang ở trong một ứng dụng khác (trình duyệt, Explorer): bấm ngay `Esc` (không bấm chuột trước) thì lớp phủ đóng. Nếu phải bấm chuột vào lớp phủ trước thì phím mới có tác dụng, ghi lại: lớp phủ chưa lấy được tiêu điểm.
- [ ] Con trỏ: chữ thập ở ngoài khung; mũi tên bốn hướng trong khung; mũi tên đổi kích thước đúng hướng trên 4 góc và 4 cạnh (cả dải dọc cạnh, không chỉ ô vuông); mũi tên thường trên thanh công cụ.
- [ ] Kéo chọn một vùng: vùng chọn sáng như ảnh gốc, phần còn lại tối; viền nét đứt; 8 tay nắm vuông trắng ở 4 góc và giữa 4 cạnh.
- [ ] Nhãn kích thước `{w} × {h}` ở trên góc trái khung, cập nhật khi kéo; khung sát mép trên màn hình thì nhãn nằm trong khung. So với kích thước thật (ví dụ chọn đúng một cửa sổ đã biết kích thước): đúng pixel vật lý.
- [ ] Đổi kích thước bằng cả 8 hướng; kéo qua cạnh đối diện thì khung lật đúng.
- [ ] Kéo bên trong khung: khung di chuyển, giữ nguyên kích thước, không ra ngoài màn hình ảo.
- [ ] Kéo ở ngoài khung: chọn lại từ đầu; thanh công cụ ẩn trong lúc kéo và hiện lại khi thả.
- [ ] Bấm rồi thả gần như tại chỗ (khung nhỏ hơn 3 pixel): không còn khung, không có thanh công cụ.
- [ ] Thanh công cụ: nằm dưới, sát phải khung; khung sát đáy màn hình thì nằm trên; khung phủ cả màn hình thì nằm bên trong góc dưới phải. Thanh không bao giờ tràn ra ngoài màn hình chứa nó.
- [ ] Bốn biểu tượng theo thứ tự: đám mây có mũi tên lên, đĩa mềm, hai ô chồng nhau, dấu X; nét mịn (khử răng cưa), không bị cắt.
- [ ] Rê chuột vào từng nút: nút đổi nền xanh nhạt, hiện chú thích "Tải lên và chép link (Ctrl+D)", "Lưu vào máy (Ctrl+S)", "Chép ảnh (Ctrl+C)", "Hủy (Esc)"; chú thích chữ tiếng Việt có dấu đầy đủ, phông hệ thống, không tràn ra ngoài màn hình.
- [ ] Bấm từng nút: ba nút đầu đóng lớp phủ và báo đúng hành động; nút X đóng lớp phủ không báo gì.
- [ ] Phím: `Ctrl+D`, `Ctrl+S`, `Ctrl+C` đóng lớp phủ và báo đúng hành động khi đã có khung; khi chưa có khung thì không có tác dụng. `Esc` đóng lớp phủ ở mọi lúc, kể cả đang kéo.
- [ ] Alt+Tab (hoặc phím Windows) khi lớp phủ đang mở: lớp phủ đóng.
- [ ] Bấm PrintScreen khi lớp phủ đang mở: không có gì xảy ra (không mở lớp phủ thứ hai).
- [ ] Bấm PrintScreen khi đang ở một ứng dụng khác rồi bấm `Esc` ngay, không bấm chuột: lớp phủ đóng (phím Esc toàn cục tạm thời).
- [ ] Chọn "Chụp màn hình" ở menu khay rồi mở lại menu khay ngay: menu không mở, lớp phủ mở một lần sau khoảng 1/4 giây.
- [ ] Bấm PrintScreen khi menu khay đang mở: menu đóng, lớp phủ mở đúng một lần và menu không lọt vào ảnh nền.
- [ ] Hai màn hình: lớp phủ phủ cả hai; kéo một khung qua ranh giới hai màn hình được.
- [ ] Màn hình phụ đặt bên trái (hoặc phía trên) màn hình chính: lớp phủ khớp đúng từng màn hình, khung chọn và nhãn đúng vị trí dưới con trỏ.
- [ ] Hai màn hình khác tỉ lệ phóng to (ví dụ 100% và 150%): nút, nhãn, chú thích, tay nắm trên mỗi màn hình to theo tỉ lệ của màn hình đó; ảnh nền không bị lệch.
- [ ] Ảnh nền và vùng chọn không bị mờ hay co giãn (so từng pixel chữ nhỏ với màn hình thật), ở cả 100% và 150%.
- [ ] Chụp khi có hộp thoại UAC hoặc ngay sau khi mở khóa máy: nếu không chụp được thì hiện thông báo lỗi "Không chụp được màn hình", ứng dụng vẫn chạy.
- [ ] Màn hình 4K hoặc hai màn hình lớn: kéo và di chuyển khung lớn vẫn mượt; mở rồi đóng lớp phủ 10 lần, bộ nhớ của shotme (Task Manager) trở về mức cũ, số GDI objects (cột "GDI objects" trong Task Manager, tab Details) không tăng dần.

## Clipboard và khởi động cùng Windows (thử được sau Task 11)

- [ ] "Chép ảnh": dán được vào Paint, Word, trình duyệt (ví dụ Gmail), Slack hoặc Telegram; ảnh đúng màu, không lật ngược.
- [ ] "Tải lên": link dán ra đúng nguyên văn.
- [ ] Chép khi một ứng dụng khác đang giữ clipboard: có thông báo lỗi "Clipboard đang bị ứng dụng khác giữ", ứng dụng vẫn chạy.
- [ ] Bật khởi động cùng Windows trong Cài đặt, khởi động lại máy: shotme tự chạy. Tắt đi, khởi động lại: không tự chạy.
- [ ] `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` không có giá trị nào tên `shotme-test-*` (sót lại từ kiểm thử bị ngắt giữa chừng).
