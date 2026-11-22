# Nhật ký thay đổi

Mọi thay đổi đáng chú ý của darkshot, mới nhất ở trên. Phiên bản theo [SemVer](https://semver.org/lang/vi/): tăng số giữa khi có tính năng mới, số cuối khi chỉ sửa lỗi. Tải bản phát hành ở [trang Releases](https://github.com/tuanluu-agilityio/darkshot/releases).

## [Chưa phát hành]

### Sửa lỗi

- Dải tab của Tùy chọn có khoảng cách: chữ các tab (Thông thường, Phím tắt, Định dạng, Lưu trữ R2) không còn liền nhau, chữ đậm của tab đang chọn không sát tab kế bên, ở cả hai ngôn ngữ và mọi mức DPI.
- Website: mục "Tự kiểm tra bản mới" ghi sai là mặc định tắt; thực tế mặc định bật (tắt được ở Tùy chọn → Thông thường). Ảnh Tùy chọn trên website được chụp lại.

## [0.6.0] - 2026-11-22

### Mới

- **Ghi GIF.** Chọn vùng rồi bấm nút **Ghi GIF** (giữa Chụp cuộn và Hủy) hoặc Ctrl+G để ghi vùng đó thành ảnh GIF động. Thanh ghi nổi (chấm đỏ, "Đang ghi 00:05 / 01:00", Dừng, Hủy) không lấy tiêu điểm; thanh và khung viền không nằm trong GIF trên Windows 10 bản 2004 trở lên. Dừng lưu tệp `darkshot_YYYY-MM-DD_HH-MM-SS.gif` vào thư mục Lưu (không ghi đè, trùng tên thì thêm `_2`…) và báo "Đã lưu GIF"; Hủy không tạo tệp. 10 khung hình mỗi giây, tối đa 60 giây, lặp vô hạn, không con trỏ chuột, không âm thanh, 256 màu mỗi khung; vùng lớn hơn 500 000 điểm ảnh được thu nhỏ, vùng quá 64 MP bị từ chối. Tự dừng và lưu khi đạt 60 giây, 256 MiB hoặc khi vùng không còn trên màn hình. Thoát ứng dụng khi đang ghi thì lưu phần đã ghi. Không tải lên, không vào Lịch sử, không đổi "lần chụp trước". Hình đánh dấu không được kèm. Chạy hoàn toàn trên máy, không mạng.
- **Privacy Shield (quét thông tin nhạy cảm).** Tùy chọn, mặc định tắt (Tùy chọn → Thông thường → "Quét thông tin nhạy cảm trước khi xuất ảnh"). Trước khi Lưu, Chép, Tải lên hoặc Ghim, darkshot nhận dạng chữ trên máy (OCR của Windows) và tìm email, số điện thoại, thẻ thanh toán, khóa API, token, mật khẩu, khóa riêng. Không có phát hiện thì không hỏi; có thì cửa sổ **Kiểm tra thông tin nhạy cảm** liệt kê (không hiện giá trị), cho bỏ chọn từng mục, thêm vùng che bằng chuột, rồi che bằng ô đặc. Lỗi quét: Thử lại hoặc Tiếp tục không quét (cần xác nhận). Áp cho Lưu, Chép, Tải lên, Ghim, Chép/Ghim từ Lịch sử, Chụp lại vùng trước, phím chụp toàn màn hình, chụp cuộn, Thêm bước và xuất phiên Chụp từng bước. Một cổng chung `privacy_gate` cho mọi đường xuất ảnh (`check.ps1` kiểm). Chạy hoàn toàn trên máy: ảnh và chữ không gửi cho AI hay máy chủ nào. Bảng có đường kẻ (ví dụ bảng tính) được quét thêm một lượt trên bản sao đã bỏ đường kẻ, vì đường kẻ sát chân chữ làm OCR bỏ `@` và chữ g/y/p/q (ảnh xuất không đổi); email bị OCR chèn khoảng trắng vào phần trước `@` được che cả phần bị tách khi khe giữa hai đoạn nhỏ (nhỏ hơn khoảng trắng thường). Chỉ là cố gắng hết sức, không bảo đảm. Số đo bộ dò trên bộ mẫu tổng hợp: `docs/testing/privacy-quality.md`. Exe release tăng khoảng 132 KiB.

### Đã biết

- Tự động có thể bỏ sót hoặc nhận nhầm (OCR đọc sai chữ nhỏ/phông lạ; ví dụ đã đo trong `docs/testing/privacy-quality.md`); nên tự xem lại và tự thêm vùng che.
- Privacy Shield không phát hiện khuôn mặt, mã QR, hình ảnh, chữ viết tay; không áp cho Ghi GIF và chữ chép từ Nhận dạng chữ.
- Mục Lịch sử và ảnh bước tạo khi Shield tắt không được che lại trên đĩa (chỉ được quét khi Chép/Ghim/xuất lúc Shield bật); "Mở"/"Hiện trong thư mục" của Lịch sử không quét.
- Lưu/Chép từ ảnh ghim hay Lịch sử khi Shield bật luôn quét lại: có thể thấy lại phát hiện đã bỏ chọn.
- Ảnh rộng hơn 10000 px (giới hạn OCR của Windows) bị co theo chiều rộng khi quét: chữ nhỏ có thể bị bỏ sót.
- Tệp cấu hình hỏng hoặc khóa `privacy_shield` sai kiểu → Shield về tắt (thông báo cấu hình hỏng không nhắc riêng Shield).
- Tắt Shield lúc bấm Xuất phiên rồi bật ngay: lượt xuất đó chép như khi tắt.
- Đang có 3 lượt chờ duyệt thì lượt mới bị bỏ (kể cả tự Lưu của chụp cuộn khi không mở được cửa sổ kết quả).
- Cửa sổ Kiểm tra bị ẩn khi chia sẻ màn hình/ghi màn hình (Windows 10 2004 trở lên).
- Trình đọc màn hình đọc cửa sổ Kiểm tra hạn chế (cửa sổ tự vẽ).
- Privacy Shield đã qua bản demo (2026-11-22) nhưng chưa được thử tay đầy đủ (DPI, nhiều màn hình, máy không có gói OCR en-US). Xem mục "Đợt 21: Privacy Shield" trong `docs/testing/manual-checklist.md`.
- Ghi GIF chưa được thử tay trên Windows 10, nhiều màn hình, DPI hỗn hợp và màn hình có tọa độ âm. Thông báo Windows hiện trong lúc ghi có thể lọt vào GIF. Xem mục "Đợt 20: Ghi GIF" trong `docs/testing/manual-checklist.md`.

## [0.5.0] - 2026-11-21

### Mới

- **Chụp lại vùng trước.** Phím mới (mặc định tắt, `Ctrl+Shift+PrintScreen`, bật ở Tùy chọn → Phím tắt) và mục menu khay cùng tên, ngay sau Chụp màn hình. Chụp lại đúng vùng của lần chụp vùng gần nhất (cùng tọa độ màn hình) và làm lại đúng hành động đã chọn (Lưu, Chép, Tải lên hoặc Ghim), không mở lớp phủ, không lặp nét đánh dấu. Lần chụp lặp lại không đổi "lần chụp trước". Chưa có lần chụp vùng nào thì mở lớp phủ như phím chụp vùng. Vùng không còn trên màn hình thì mở lớp phủ để chọn lại, kèm thông báo sau khi đóng. Nhớ qua khởi động lại trong `%APPDATA%\darkshot\last_capture.json`, tệp chỉ chứa tọa độ và hành động.
- **Giữ vị trí vùng chọn** nay nhớ qua khởi động lại (cùng tệp `last_capture.json`).
- **Nhận dạng chữ cục bộ.** Chọn vùng rồi bấm Ctrl+T hoặc nút **Nhận dạng chữ** (giữa Ghim và Hủy) để mở cửa sổ **Chữ nhận dạng** với chữ trong vùng. Ảnh nhận dạng gồm cả các nét đã vẽ, nên vùng đã làm mờ không đọc được chữ. Chữ sửa được trong ô; nút Chép (Ctrl+Enter, hoặc Enter khi không ở ô chữ) chép chữ rồi đóng cửa sổ. Ảnh ghim có mục "Nhận dạng chữ" trong menu chuột phải. Tối đa 8 cửa sổ chữ cùng lúc. Nhận dạng dùng OCR của Windows theo danh sách ngôn ngữ trong cài đặt Windows, hoàn toàn trên máy: không mạng, không lưu chữ ra tệp, không vào Lịch sử, không đổi "lần chụp trước". Tiếng Việt chỉ nhận dạng được nếu Windows có gói nhận dạng tiếng Việt.
- **Chữ nhận dạng nâng cao.** Cửa sổ **Chữ nhận dạng** nay có hai khung: trái là ảnh với khung quanh từng dòng, phải là ô chữ sửa được. Bấm khung để chép dòng (hiện "Đã chép"); Ctrl+bấm để gom hoặc bỏ dòng và chép các dòng đã chọn theo thứ tự đọc; bấm chỗ trống để bỏ chọn; trên khung ảnh dùng được ↑/↓, Enter, Space, Ctrl+A và Esc (bỏ chọn, rồi đóng). Ba công tắc **Gộp thành một dòng**, **Tách đoạn**, **Tự chép khi xong** (mặc định tắt, nhớ giữa các lần). Nút **Chép tất cả** chép mà không đóng cửa sổ (thay cho nút Chép đóng cửa sổ trước đây). Ảnh cao cuộn dọc; ảnh lớn hơn 16 MP được thu nhỏ để hiển thị. Vẫn hoàn toàn trên máy, không mạng. Windows chưa có bộ nhận dạng tiếng Việt nên chữ có dấu có thể mất dấu.
- **Chụp từng bước.** Mục menu khay "Chụp từng bước" (ngay sau Lịch sử ảnh chụp) mở một phiên gồm nhiều bước, tối đa 99. Khi có phiên, mục này thành "Xem lại các bước (N)". Thanh phiên nhỏ, tối, luôn nằm trên cùng, không lấy tiêu điểm và kéo được; có chấm đỏ, "Chụp từng bước · N bước" và các nút Chụp bước, Xem lại, Kết thúc. Thanh không xuất hiện trong ảnh chụp trên Windows 10 2004 trở lên; máy cũ hơn thì thanh tự ẩn trong lúc chụp. Khi có phiên, lớp phủ có thêm nút "Thêm bước (Ctrl+N)" ngay trước Hủy, và Ctrl+N thêm vùng chọn (đã đánh dấu) làm một bước. Ảnh bước không vào Lịch sử và không đổi "lần chụp trước". Cửa sổ "Các bước" cho đổi thứ tự (Lên, Xuống), ghi chú (tối đa 2000 ký tự), xóa bước (Delete) và xem trước. Xuất… tạo thư mục `darkshot-steps-YYYY-MM-DD_HH-mm` gồm `01.png`, `02.png`… và `steps.md`, không ghi vào thư mục có sẵn; xuất không kết thúc phiên. Kết thúc phiên khi còn bước chưa xuất sẽ hỏi trước. Phiên lưu ở `%LOCALAPPDATA%\darkshot\steps\current\`, thoát rồi mở lại vẫn tiếp tục; tệp phiên hỏng thì thư mục được giữ lại thành `current.corrupt-<thời gian>`. Chạy hoàn toàn trên máy, không mạng.
- **Chụp cuộn.** Chọn vùng rồi bấm nút **Chụp cuộn** (giữa Nhận dạng chữ và Hủy) hoặc Ctrl+L, rồi tự cuộn nội dung dọc xuống từ từ trong vùng cố định đã chọn; darkshot ghép các phần thành một ảnh dọc. Phạm vi: chỉ cuộn dọc, bạn tự cuộn (darkshot không gửi lệnh cuộn vào ứng dụng khác), không đảm bảo mọi ứng dụng. Trong lúc chụp có khung nét đứt quanh vùng và thanh điều khiển nổi (chấm đỏ, chiều cao px, trạng thái "Cuộn chậm hơn" khi khung không ghép được, nút Xong và Hủy; Esc hủy). Hình đánh dấu không được kèm. Khung không ghép được (nền một màu, nội dung lặp, cuộn quá nhanh hoặc cuộn ngược) bị bỏ; chỗ ghép không chắc chắn được báo bằng dải vàng "Có N chỗ ghép không chắc chắn. Kiểm tra ảnh trước khi dùng.". Cửa sổ "Ảnh chụp cuộn" xem trước cuộn được, ghi "W × H px", có nút Lưu (Enter), Chép, Tải lên, Ghim và Đóng (Esc); darkshot không bao giờ tự đóng cửa sổ này (từ 4 cửa sổ mở, mở thêm vẫn được và hiện "Có nhiều ảnh chụp cuộn chưa lưu"; thiếu bộ nhớ khi Lưu, Chép, Tải lên hoặc Ghim thì hiện "Không đủ bộ nhớ"). Giới hạn 30 000 px chiều cao hoặc 256 MB, đạt thì tự dừng và báo "Đã đạt giới hạn chiều cao". Chạy hoàn toàn trên máy, không mạng.

### Sửa lỗi

- Cửa sổ **Chữ nhận dạng** mở từ ảnh ghim không còn bị ảnh ghim che: khi không có chỗ bên cạnh ảnh ghim, cửa sổ nằm trên cùng, trên ảnh ghim.
- Ghim ảnh rất cao (hoặc rất rộng) mà ở mức 10% vẫn lớn hơn màn hình: phần đầu ảnh nay luôn thấy (mép trên/trái ảnh sát mép vùng làm việc) thay vì nằm ngoài màn hình; kéo để xem phần sau.
- Đánh dấu: Hoàn tác/Làm lại không còn làm lệch sổ thao tác nếu một bước không áp dụng được.

### Đã biết

- Chụp cuộn chưa được thử tay trên Windows 10 (trước và sau 2004), trình duyệt, tài liệu Word và PDF, danh sách trong Explorer, DPI 100%, 150% và 200%, nhiều màn hình. Các giới hạn đã biết của thuật toán ghép (khối lặp cao hơn 1/3 vùng, cuộn quá nhanh, lớp phủ mỏng ở đáy, thanh cuộn của trang) xem mục "Chụp cuộn" trong README. Xem mục "Đợt 13: Chụp cuộn" trong `docs/testing/manual-checklist.md`.
- Chụp từng bước chưa được thử tay trên Windows 10, ở các DPI 100% và 150%, với tiếng Việt, và trên màn hình có tọa độ âm. Xem mục "Đợt 12" trong `docs/testing/manual-checklist.md`.
- Chụp lại vùng trước chưa được thử tay trên Windows 10, ở các DPI 100% và 150%, trên màn hình có tọa độ âm, và khi rút màn hình. Xem mục "Đợt 10" trong `docs/testing/manual-checklist.md`.
- Chữ nhận dạng nâng cao chưa được thử tay ở DPI 100% và 150%, trên màn hình có tọa độ âm, và với đọc màn hình. Xem mục "Đợt 15" trong `docs/testing/manual-checklist.md`.
- Cửa sổ Chữ nhận dạng giữ ảnh ở độ phân giải gốc tới 16 MP (tới khoảng 64 MB bộ nhớ mỗi cửa sổ, tối đa 8 cửa sổ). Ảnh lớn hơn (ví dụ ảnh chụp cuộn rất dài) được thu nhỏ lúc mở cửa sổ, có thể khựng thoáng; kéo đổi cỡ cửa sổ với ảnh lớn có thể giật.
- Nhận dạng chữ cục bộ chưa được thử tay trên Windows 10, ở các DPI 100% và 150%, và với tiếng Việt (máy thử chỉ có English). Xem mục "Đợt 11" trong `docs/testing/manual-checklist.md`.

## [0.4.0] - 2026-11-18

### Mới

- **Ghim ảnh lên màn hình.** Từ lớp phủ, bấm nút Ghim hoặc Ctrl+P để ghim vùng vừa chọn (đã đánh dấu) khít đúng chỗ của nó. Từ Lịch sử, nút Ghim mở ảnh giữa màn hình có chuột, không lớn hơn 80% vùng làm việc. Ảnh ghim luôn nằm trên cùng: kéo để dời, cuộn để phóng, Ctrl+cuộn để đổi độ mờ, bấm đúp để về 100%, chuột phải để mở menu, Esc để đóng. Phím: `+` `-` `0`, mũi tên, Ctrl+W, Ctrl+C, Ctrl+S. Tối đa 32 ảnh ghim; menu khay có "Đóng mọi ảnh ghim" khi đang có ảnh ghim. Thoát darkshot đóng mọi ảnh ghim.
  - Phóng: mỗi cạnh hiển thị tối đa 16.384 px; trên 100%, diện tích hiển thị tối đa 40 triệu px (ảnh 4K phóng tối đa 200%).

### Sửa lỗi

- Không có sửa lỗi sản phẩm trong đợt này.

### Đã biết

- Sau khi bấm chọn một cửa sổ ở Gợi ý cửa sổ, viên nhắc có thể còn hiện tới lần di chuột kế tiếp. Đã điều tra (Task 66), không tái hiện được trên một màn hình 200%; cần thử bằng chuột thật trên nhiều màn hình.
- Ảnh ghim chưa được thử tay trên Windows 10, ở các DPI 100% và 150%, và trên màn hình có tọa độ âm. Xem mục "Đợt 9" trong `docs/testing/manual-checklist.md`.

## [0.3.0] - 2026-11-18

### Mới

- **Gợi ý cửa sổ khi chọn vùng.** Khi chưa có vùng chọn, rê chuột lên một cửa sổ thì khung của nó sáng lên kèm nhãn kích thước; rê ra nền thì cả màn hình sáng lên. Bấm một lần (không kéo) hoặc Enter để chọn đúng khung đó, không lẫn viền vô hình. Kéo chuột vẫn chọn vùng tự do như trước. Viên nhắc ở mép trên màn hình nhắc thao tác và không nằm trong ảnh. Bật/tắt ở Tùy chọn → Thông thường → "Gợi ý cửa sổ khi chọn vùng" (mặc định bật). darkshot chỉ đọc khung cửa sổ, không đọc tiêu đề hay nội dung.
- **Lịch sử ảnh chụp** (mặc định tắt; bật ở Tùy chọn → Thông thường). Giữ 50 ảnh gần nhất đã Lưu, Chép hoặc Tải lên, chỉ trên máy (`%LOCALAPPDATA%\darkshot\history\`). Cửa sổ Lịch sử mở từ menu khay: ảnh nhỏ, xem trước, tìm theo tên/ngày/liên kết (không phân biệt dấu), lọc theo loại, Mở, Sao chép ảnh, Sao chép liên kết, Mở thư mục chứa ảnh, Xóa khỏi lịch sử, Xóa lịch sử (có xác nhận). Xóa trong lịch sử không bao giờ xóa ảnh bạn đã lưu hay ảnh trên R2.
- **Annotation Pro** cho công cụ đánh dấu:
  - Hoàn tác và làm lại nhiều bước (Ctrl+Z, Ctrl+Y hoặc Ctrl+Shift+Z), có nút trên thanh ngang.
  - Công cụ **Chọn** (phím V): chọn một hình để dời, đổi cỡ, đổi màu hoặc xóa (Delete).
  - **Hình elip** (phím E) và **Đánh số** (phím N, vòng số tự tăng, tự đánh lại khi xóa).
  - Bốn mức cỡ nét và cỡ chữ (bảng Cỡ, phím `[` và `]`).
- **Thanh công cụ gọn:** thanh dọc còn 9 mục; nút **Hình** gom Đường thẳng, Mũi tên, Hình chữ nhật, Hình elip và nhớ hình dùng gần nhất.
- **Báo có bản mới:** ứng dụng hỏi GitHub một lần sau khi khởi động và mỗi 24 giờ; có bản mới thì menu khay hiện mục "Có bản mới X.Y.Z". Chỉ gửi tên và phiên bản ứng dụng. Tắt được ở Tùy chọn.

### Sửa lỗi và độ tin cậy

- Lịch sử không còn mất mục khi thoát ngay sau Lưu, Chép hoặc Tải lên: các mục được ghi tuần tự trên một luồng nền và khi thoát (kể cả tắt máy) darkshot chờ tối đa 3 giây cho việc ghi xong.
- Một lượt tải lên không bao giờ tạo hai mục lịch sử, kể cả khi thoát đúng lúc lượt đó vừa xong.
- Chỉ mục lịch sử và ảnh được ghi chắc xuống đĩa trước khi đổi tên, để mất điện không làm hỏng chỉ mục.
- Khi thẻ "Xóa toàn bộ lịch sử?" đang mở, thanh cuộn danh sách mờ đi và không cuộn được.

### Đã biết

- Sau khi bấm chọn một cửa sổ, viên nhắc có thể còn hiện tới lần di chuột kế tiếp.
- Gợi ý cửa sổ và Lịch sử chưa được thử tay ở DPI 100%/150%, màn hình có tọa độ âm và Windows 10.
- Hạn hiển thị của ảnh đã tải lên là ngày "còn ít nhất tới": R2 có thể xóa trễ tới khoảng một ngày.

## [0.2.0] - 2026-11-17

### Mới

- **Công cụ Làm mờ** (phím B): kéo một hình chữ nhật lên thông tin nhạy cảm; vùng che được tô kín bằng một màu trung bình duy nhất, cạnh sắc, không khôi phục được chữ bên dưới. Giữ Shift để thành hình vuông.

### Sửa lỗi

- Khi vùng chọn hẹp và sát góc trên trái màn hình, thanh công cụ dọc không còn chồng lên thanh bốn nút.

Bản 0.1.1 từng được phát hành rồi rút lại cùng ngày (0 lượt tải) và đổi thành 0.2.0 vì có tính năng mới.

## [0.1.0] - 2026-11-17

Bản phát hành đầu tiên.

### Mới

- Chụp vùng màn hình bằng `PrintScreen` (đổi được), nhiều màn hình và DPI khác nhau; sau khi chọn: **Tải lên** Cloudflare R2 của bạn và chép link có hạn, **Lưu** PNG, **Chép** ảnh vào clipboard, hoặc **Hủy** (Ctrl+D, Ctrl+S, Ctrl+C, Esc).
- Phím tắt toàn màn hình: lưu và tải lên nhanh không cần chọn vùng.
- Đánh dấu ảnh trong lớp phủ: bút vẽ, đường thẳng, mũi tên, hình chữ nhật, bút dạ quang, chữ (gõ được tiếng Việt), hoàn tác; bảng màu 48 màu, 16 màu tự chọn được lưu, hộp chọn màu đủ (HSL, RGB).
- Tải lên R2 dạng PNG hoặc JPEG (chọn chất lượng); link hết hạn sau 1, 7, 30 ngày hoặc không hết hạn; khóa R2 mã hóa bằng DPAPI.
- Giữ vị trí vùng chọn, chụp kèm con trỏ chuột, bật/tắt thông báo, khởi động cùng Windows.
- Giao diện luôn tối; hai ngôn ngữ Tiếng Việt và English.
- Người dùng bản cũ tên shotme: lần chạy đầu tự chép cấu hình (kể cả khóa R2) và chuyển mục khởi động cùng Windows.

[Chưa phát hành]: https://github.com/tuanluu-agilityio/darkshot/compare/v0.6.0...HEAD
[0.6.0]: https://github.com/tuanluu-agilityio/darkshot/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/tuanluu-agilityio/darkshot/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/tuanluu-agilityio/darkshot/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/tuanluu-agilityio/darkshot/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/tuanluu-agilityio/darkshot/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/tuanluu-agilityio/darkshot/releases/tag/v0.1.0
