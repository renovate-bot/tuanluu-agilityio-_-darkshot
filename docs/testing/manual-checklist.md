
## Vỏ ứng dụng

- [ ] Khởi động `shotme.exe`: biểu tượng nền xanh bo góc, có khung chọn trắng bốn góc chữ L (ở cỡ khay chỉ còn bốn góc, không chấm tròn) hiện ở khay hệ thống, chú thích "shotme"; nét sắc, không nhòe.
- [ ] Biểu tượng mới hiện đúng ở ba nơi: khay hệ thống, nút của cửa sổ Cài đặt trên thanh tác vụ (và thanh tiêu đề, Alt+Tab), và `shotme.exe` trong Explorer (thử các cỡ biểu tượng nhỏ, vừa, lớn, rất lớn). Thử thêm ở tỷ lệ hiển thị 100%, 125%, 150%.
- [ ] Bấm chuột phải (hoặc trái) vào biểu tượng: menu có ba mục "Chụp màn hình", "Cài đặt", "Thoát".
- [ ] Chọn "Cài đặt": cửa sổ Cài đặt mở (xem mục "Cài đặt").
- [ ] Chọn "Chụp màn hình" hoặc bấm PrintScreen: lớp phủ mở (xem mục "Lớp phủ").
- [ ] Khi Lightshot (hoặc ứng dụng khác giữ PrintScreen) đang chạy: shotme vẫn chạy và hiện thông báo "Phím PrintScreen đang bị ứng dụng khác chiếm".
- [ ] Chạy `shotme.exe` lần thứ hai: bản thứ hai thoát ngay, chỉ còn một tiến trình.
- [ ] Khởi động lại Explorer (Task Manager, Restart): biểu tượng khay hiện lại.
- [ ] Chọn "Thoát": biểu tượng biến mất khỏi khay và tiến trình kết thúc.
- [ ] Bật tùy chọn khởi động cùng Windows, đăng xuất rồi đăng nhập lại: biểu tượng shotme xuất hiện ở khay (kể cả khi Explorer khởi động chậm).

## Lớp phủ

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
- [ ] Bấm từng nút: ba nút đầu đóng lớp phủ và thực hiện đúng hành động (xem mục "Luồng đầy đủ"); nút X đóng lớp phủ không báo gì.
- [ ] Phím: `Ctrl+D`, `Ctrl+S`, `Ctrl+C` đóng lớp phủ và thực hiện đúng hành động khi đã có khung; khi chưa có khung thì không có tác dụng. `Esc` đóng lớp phủ ở mọi lúc, kể cả đang kéo.
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

## Cài đặt

- [ ] Mọi mục hiện đủ và đúng chữ: nhóm "Cloudflare R2" (Account ID, Access Key ID, Secret Access Key, Tên kho (bucket), Địa chỉ công khai, nút "Kiểm tra kết nối"), nhóm "Chia sẻ và lưu" (Hạn của link, Thư mục lưu, nút "Chọn…"), nhóm "Chung" (Phím tắt chụp với nút "Đổi…", "Khởi động cùng Windows"), nút "Lưu" và "Hủy". Chữ không bị cắt, các ô không chồng nhau.
- [ ] Phông chữ giống phông hệ thống; ở màn hình 125% và 150% cửa sổ vẫn cân đối, không bị cắt.
- [ ] Ô Secret Access Key hiện dấu chấm khi gõ; mở lại cửa sổ thì ô này rỗng, dòng chú thích dưới ô ghi "Đã lưu khóa; để trống nếu không đổi."; lưu với ô rỗng thì khóa cũ vẫn dùng được (Kiểm tra kết nối vẫn thành công).
- [ ] Phím Tab đi qua các ô theo thứ tự hợp lý; Enter ở ô nhập là Lưu; Esc là Hủy; Enter khi đang chọn nút "Chọn…" hoặc "Đổi…" là bấm nút đó.
- [ ] "Chọn…" mở hộp chọn thư mục; chọn xong đường dẫn hiện trong ô.
- [ ] Địa chỉ công khai nhập "http://x" rồi Lưu: lỗi màu đỏ hiện ngay trong cửa sổ, cửa sổ không đóng. Nhập "https://img.example.com/" thì lưu thành không có dấu "/" cuối.
- [ ] Xóa trống ô Thư mục lưu rồi Lưu: hiện lỗi, cửa sổ không đóng.
- [ ] "Đổi…": ô phím hiện "Nhấn tổ hợp phím mới…"; bấm PrintScreen thì ô hiện "PrintScreen"; bấm Ctrl+Shift+S thì hiện "Ctrl+Shift+S" (kể cả khi nhả Ctrl trước chữ S); bấm riêng chữ "S" hoặc Shift+S thì báo cần kèm Ctrl, Alt hoặc Win; F5 và Shift+F5 được nhận; Esc bỏ qua việc đổi.
- [ ] Bắt đầu "Đổi…" bằng phím Enter hoặc Space (Tab tới nút rồi bấm): ngay sau đó không hiện lỗi "cần kèm Ctrl…".
- [ ] Trong lúc "Đổi…": Alt+F4 đóng cửa sổ như thường (không thành tổ hợp); bấm chuột vào ô khác hoặc thanh tiêu đề thì thoát chế độ bắt và ô phím trở lại giá trị cũ; chuyển sang cửa sổ khác cũng thoát chế độ bắt.
- [ ] Phím mũi tên khi đang chọn nút "Lưu" chỉ nhảy sang "Hủy", không nhảy vào các ô nhập; Tab vẫn đi đủ các ô.
- [ ] "Kiểm tra kết nối": bấm liên tiếp nhiều lần chỉ chạy một lần; trong lúc nút mờ, Enter vẫn là Lưu chỉ khi bạn muốn (focus đã chuyển sang ô Account ID, không rơi vào trạng thái không phím nào hoạt động).
- [ ] Đổi phím tắt sang tổ hợp khác, Lưu: phím mới chụp được, phím cũ không còn tác dụng. Đổi về PrintScreen, Lưu: PrintScreen chụp được lại.
- [ ] Bấm "Đổi…" rồi bấm Hủy: phím tắt cũ vẫn chụp được.
- [ ] Chọn phím đang bị ứng dụng khác chiếm rồi Lưu: hiện lỗi "đang bị ứng dụng khác chiếm" trong cửa sổ, cửa sổ không đóng.
- [ ] "Kiểm tra kết nối" với khóa sai: nút mờ trong lúc chờ, cửa sổ vẫn kéo và bấm được, rồi hiện thông báo lỗi tiếng Việt (không chứa khóa). Với khóa đúng: "Thành công: tải lên được và link mở được."
- [ ] Đóng cửa sổ (nút X) lúc đang kiểm tra kết nối: ứng dụng không treo, không báo lỗi; mở lại cửa sổ thì dòng kết quả rỗng.
- [ ] Mở Cài đặt lần hai khi đã có một cửa sổ: không mở thêm, cửa sổ đang có được đưa ra trước.
- [ ] Khi có thông báo đầu cửa sổ (cấu hình hỏng hoặc mất khóa): dòng chữ hiện ở đầu, không đè lên nhóm "Cloudflare R2".

## Clipboard và khởi động cùng Windows

- [ ] "Chép ảnh": dán được vào Paint, Word, trình duyệt (ví dụ Gmail), Slack hoặc Telegram; ảnh đúng màu, không lật ngược.
- [ ] "Tải lên": link dán ra đúng nguyên văn.
- [ ] Chép khi một ứng dụng khác đang giữ clipboard: có thông báo lỗi "Clipboard đang bị ứng dụng khác giữ", ứng dụng vẫn chạy.
- [ ] Bật khởi động cùng Windows trong Cài đặt, khởi động lại máy: shotme tự chạy. Tắt đi, khởi động lại: không tự chạy.
- [ ] Bật lịch sử clipboard (Cài đặt Windows > Hệ thống > Clipboard), chép ảnh và tải lên bằng shotme, rồi bấm `Win+V`: không có ảnh hay link nào của shotme trong danh sách; dán (`Ctrl+V`) vẫn ra đúng ảnh/link.
- [ ] `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` không có giá trị nào tên `shotme-test-*` (sót lại từ kiểm thử bị ngắt giữa chừng).
- [ ] Sau khi chạy shotme một lần: `HKCU\Software\Microsoft\Windows\Windows Error Reporting\ExcludedApplications` có giá trị DWORD `shotme.exe` = 1 (loại khỏi Báo cáo lỗi của Windows); ngoài nó và khóa `Run` khi bật khởi động cùng Windows, shotme không ghi gì khác vào registry.

## Luồng đầy đủ

- [ ] Lần chạy đầu (xóa hoặc đổi tên `%APPDATA%\shotme\config.json` trước): ngay khi khởi động cửa sổ Cài đặt tự mở, không có dòng thông báo ở đầu cửa sổ. Bấm Hủy: không có file cấu hình nào được tạo.
- [ ] Bấm "Lưu" ở lần chạy đầu: file `%APPDATA%\shotme\config.json` được tạo; mở bằng Notepad thì không thấy Secret Access Key ở dạng đọc được.
- [ ] Chụp, chọn "Lưu vào máy": file `shotme_YYYY-MM-DD_HH-MM-SS.png` nằm đúng trong thư mục lưu đã cấu hình (mặc định `Pictures\shotme`), tên đúng giờ hiện tại, mở ra đúng vùng đã chọn; có thông báo "Đã lưu ảnh" kèm đường dẫn. Lưu hai ảnh trong cùng một giây: tên thứ hai có hậu tố `_2`.
- [ ] Đổi thư mục lưu trong Cài đặt rồi lưu ảnh: ảnh nằm trong thư mục mới (thư mục chưa có thì được tạo).
- [ ] Chụp, chọn "Chép ảnh": thông báo "Đã chép ảnh"; dán được vào Paint, ảnh đúng vùng chọn.
- [ ] Tải lên khi chưa cấu hình R2: ảnh được lưu vào thư mục lưu, thông báo lỗi nói rõ chưa cấu hình và nêu đường dẫn ảnh, cửa sổ Cài đặt tự mở.
- [ ] Tải lên khi đã cấu hình đúng: thông báo "Đã chép link", dán ra link mở được ảnh. Link nằm đúng thư mục hạn đã chọn (`d30/...` nếu mặc định).
- [ ] Tải lên khi rút mạng: ảnh được lưu vào thư mục lưu, thông báo lỗi tiếng Việt kèm đường dẫn; Cài đặt không tự mở.
- [ ] Hai lần tải lên liên tiếp (chụp lần hai ngay khi lần một chưa xong): có hai thông báo "Đã chép link" riêng, hai link khác nhau; ứng dụng không đứng trong lúc tải.
- [ ] Thoát ứng dụng (menu khay) khi một lượt tải lên còn chạy (ví dụ mạng chậm): ứng dụng thoát ngay, không treo, không báo lỗi lạ; ảnh của lượt đó có trong thư mục lưu dưới dạng PNG (có thể ảnh cũng đã lên kho).
- [ ] Đăng xuất hoặc tắt máy khi một lượt tải lên còn chạy: đăng nhập lại thì ảnh của lượt đó có trong thư mục lưu.
- [ ] File cấu hình hỏng (ghi `{not json` vào `config.json`): Cài đặt mở ngay với dòng "File cấu hình bị hỏng. Cài đặt mặc định đang được dùng; bấm Lưu để ghi lại."; trước khi bấm Lưu, nội dung file vẫn là `{not json`; bấm Lưu thì file được ghi lại hợp lệ.
- [ ] Mất khóa bí mật (chép `config.json` từ tài khoản Windows khác): Cài đặt mở với dòng "Không đọc được khóa bí mật đã lưu. Hãy nhập lại Secret Access Key."; các trường khác còn nguyên.
- [ ] Đổi phím tắt trong Cài đặt rồi Lưu: phím mới chụp được ngay, không cần khởi động lại; phím cũ không còn tác dụng. Khởi động lại ứng dụng: phím mới vẫn đúng.
- [ ] Khi cửa sổ Cài đặt đang mở: bấm phím chụp không mở lớp phủ, cửa sổ Cài đặt được đưa ra trước.
- [ ] Cửa sổ Cài đặt có nút riêng trên thanh tác vụ; Alt+Tab thấy nó.
- [ ] Che cửa sổ Cài đặt bằng cửa sổ khác (hoặc thu nhỏ nó), rồi bấm biểu tượng khay: cửa sổ Cài đặt hiện ra trước (khôi phục nếu đang thu nhỏ), không có menu khay và không mở cửa sổ thứ hai.
- [ ] Tải lên khi chưa cấu hình, nhưng lúc đó đang mở lớp phủ chụp lần khác: lớp phủ đóng xong thì Cài đặt mới mở (yêu cầu không bị mất).
- [ ] Menu khay đang mở mà một lượt tải lên báo cần Cài đặt: menu đóng rồi Cài đặt mở, không treo.
- [ ] Khi lưu cài đặt thất bại (ví dụ thư mục `%APPDATA%\shotme` bị khóa ghi) và phím cũ không đăng ký lại được: có thông báo "Phím … hiện không hoạt động" kèm hướng dẫn mở Cài đặt chọn lại.
- [ ] Bật "Khởi động cùng Windows" rồi Lưu: mục `shotme` xuất hiện trong `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`; tắt rồi Lưu thì mất.

## Nghiệm thu với R2 thật

Chủ dự án tự làm các bước này với kho thật. Agent không nhận và không ghi khóa.

- [ ] Nhập Account ID, Access Key ID, Secret Access Key, Tên kho và Địa chỉ công khai theo README (mục "Chuẩn bị Cloudflare R2"), bấm "Kiểm tra kết nối": hiện "Thành công: tải lên được và link mở được."
- [ ] Mở `%APPDATA%\shotme\config.json` bằng Notepad, tìm chuỗi Secret Access Key: không thấy khóa ở dạng đọc được.
- [ ] Chụp một vùng, bấm `Ctrl+D`: thông báo "Đã chép link", dán ra link nguyên văn.
- [ ] Mở link ở trình duyệt khác (cửa sổ ẩn danh, không đăng nhập Cloudflare): ảnh hiện đúng vùng đã chọn.
- [ ] Link của hạn mặc định có thư mục `/d30/`. Đổi "Hạn của link" sang "1 ngày", tải lên một ảnh: link có `/d1/`.
- [ ] Rút mạng rồi bấm `Ctrl+D`: ảnh được lưu vào thư mục lưu, thông báo "Không tải lên được" nêu lý do và đường dẫn ảnh; Cài đặt không tự mở. Nối mạng lại.
- [ ] Trong lúc `shotme.exe` chạy và tải lên, mở Resource Monitor (tab Network): các kết nối của `shotme.exe` chỉ tới địa chỉ `{Account ID}.r2.cloudflarestorage.com` và địa chỉ công khai của kho.
- [ ] Kiểm tra quy tắc xóa theo README (mục "Kiểm tra quy tắc xóa"): ảnh `d1/` biến mất sau hạn cộng độ trễ xóa.

## Đợt 1: cửa sổ Tùy chọn

- [ ] Ba tab đúng nội dung và không có ô thừa: Thông thường (Khởi động cùng Windows; Thư mục lưu + Chọn…; Ngôn ngữ), Phím tắt (Chụp vùng màn hình + ô chỉ đọc + Đổi…), Lưu trữ R2 (Account ID, Access Key ID, Secret Access Key + dòng gợi ý, Tên kho, Địa chỉ công khai, Hạn của link, Kiểm tra kết nối + dòng kết quả). Nền khung và nhãn trùng nền dải tab, không có ô xám trên nền trắng.
- [ ] Cửa sổ không đổi cỡ khi chuyển tab.
- [ ] Giá trị gõ ở một tab còn nguyên sau khi sang tab khác và quay lại.
- [ ] Tab/Shift+Tab đi vòng: dải tab → các ô của tab đang hiện → OK → Hủy → dải tab; không bao giờ vào ô của tab đang ẩn. Mũi tên trái/phải đổi tab khi dải tab giữ tiêu điểm. Ctrl+Tab và Ctrl+Shift+Tab đổi tab (vòng tròn) từ bất cứ ô nào, kể cả khi hộp chọn đang thả.
- [ ] Enter: bấm nút đang giữ tiêu điểm (Chọn…, Đổi…, Kiểm tra kết nối, Hủy), còn lại là OK. Esc và nút X: Hủy. Hộp chọn đang thả thì Enter/Esc chỉ đóng hộp chọn.
- [ ] Ô sai ở tab khác (ví dụ đang ở tab Phím tắt, Tên kho có dấu cách; hoặc Thư mục lưu trống khi đang ở tab R2): bấm OK nhảy về đúng tab, con trỏ ở đúng ô, dòng lỗi đỏ hiện câu lỗi.
- [ ] Đổi phím tắt: bấm Đổi…, rồi bấm chuột sang tab khác lúc đang bắt phím: thoát chế độ bắt, ô phím tắt hiện lại phím đang chọn.
- [ ] Ctrl+Tab lúc đang bắt phím được nhận làm phím tắt (ô hiện "Ctrl+Tab"), không đổi tab.
- [ ] Phím tắt mới bị ứng dụng khác chiếm: bấm OK chuyển sang tab Phím tắt và hiện lỗi; Hủy thì phím cũ hoạt động lại.
- [ ] Bấm Kiểm tra kết nối rồi sang tab khác và quay lại: kết quả nằm ở dòng kết quả của tab Lưu trữ R2, nút bật lại.
- [ ] Bấm Kiểm tra kết nối rồi Hủy ngay; mở lại Tùy chọn và bấm Kiểm tra kết nối: nó chạy (hiện "Đang kiểm tra…"), và kết quả của lần bị hủy không hiện ra trong cửa sổ mới.
- [ ] Lần chạy đầu (xóa `%APPDATA%\shotme\config.json`) cửa sổ mở ở tab Lưu trữ R2, con trỏ ở Account ID; mở từ menu "Tùy chọn..." thì ở tab Thông thường.
- [ ] Đổi ngôn ngữ rồi OK: menu khay, thông báo, chữ rê chuột trên thanh công cụ lớp phủ và cửa sổ Tùy chọn mở lại đều đổi theo.
- [ ] Đổi ngôn ngữ rồi Hủy: không đổi gì.
- [ ] File cấu hình của bản cũ (không có trường `language`): vẫn Tiếng Việt, mọi cài đặt còn nguyên.
- [ ] Chữ English không bị cắt ở ô nào (nhãn, nút, tab, ô tích, dòng gợi ý, thông báo cấu hình lỗi).
