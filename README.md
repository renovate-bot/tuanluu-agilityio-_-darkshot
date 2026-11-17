# darkshot

darkshot là ứng dụng Windows 11 chạy ở khay hệ thống. Bấm `PrintScreen`, kéo chọn vùng màn hình, rồi chọn một trong bốn việc: tải ảnh lên Cloudflare R2 và chép link, lưu PNG vào máy, chép ảnh vào clipboard, hoặc hủy. Ứng dụng viết bằng Rust thuần, không có máy chủ trung gian.

## Build

Yêu cầu: Rust (cài qua rustup), Windows 11.

```powershell
cargo build --release
```

File chạy nằm tại `target\release\darkshot.exe`. Kích thước bản build đo ngày 2026-11-17 (commit `f67268a`): **1.686.528 byte** (khoảng 1,61 MiB, bản `--release` trên Windows 11).

Biểu tượng ứng dụng nằm ở `crates\darkshot-windows\res\darkshot.ico`, sinh bằng `python crates\darkshot-windows\res\make_icon.py` (cần Pillow). Script ghi đè `darkshot.ico` cạnh nó.

## Chuyển từ shotme

darkshot là tên mới của shotme. Nếu máy đang có shotme:

1. Thoát shotme (menu khay, **Thoát**) trước khi chạy `darkshot.exe`. Hai bản cùng chạy sẽ tranh phím tắt; khi đó darkshot báo "Phím tắt chưa hoạt động".
2. Lần chạy đầu, darkshot tự chép `%APPDATA%\shotme\config.json` sang `%APPDATA%\darkshot\config.json`, kể cả khóa R2 đã mã hóa. Không phải nhập lại R2. Nếu `%APPDATA%\darkshot\config.json` đã có thì darkshot giữ nguyên file đó, không ghi đè.
3. Khởi động cùng Windows theo cấu hình của darkshot, và mục khởi động của shotme được gỡ. Khi cấu hình được chép từ shotme ở lần chạy đầu, shotme đang bật thì darkshot cũng bật. Nếu darkshot đã có cấu hình riêng với khởi động tắt thì vẫn tắt. Ngoại lệ: nếu `%APPDATA%\darkshot\config.json` hỏng hoặc không đọc được, darkshot không đụng tới mục khởi động nào (của cả hai bản) để không ai mất khởi động cùng Windows.
4. Thư mục ảnh cũ vẫn được dùng: nếu shotme lưu vào `Pictures\shotme` (mặc định cũ), darkshot tiếp tục lưu vào đó. Cấu hình mới của darkshot dùng `Pictures\darkshot`.
5. `%APPDATA%\shotme` không bị xóa. Sau khi thấy darkshot chạy đúng, có thể xóa thư mục này bằng tay.

Nếu một bước chuyển bị lỗi, darkshot vẫn chạy với các thiết lập đang có và hiện thông báo "Chưa chuyển được hết thiết lập từ phiên bản cũ".

## Cách dùng

1. Chạy `darkshot.exe`. Biểu tượng xuất hiện ở khay hệ thống, chú thích "darkshot".
2. Bấm chuột trái hoặc phải vào biểu tượng để mở menu: **Chụp màn hình**, **Tùy chọn...**, **Thông tin...**, **Trợ giúp**, **Thoát**.
3. Lần chạy đầu, khi chưa có cấu hình, cửa sổ **Tùy chọn** tự mở ở tab **Lưu trữ R2**. Điền thông tin R2 (xem mục bên dưới) rồi bấm **OK**.
4. Bấm `PrintScreen` hoặc chọn **Chụp màn hình**. Màn hình tối đi và đứng yên.
5. Kéo chuột để chọn vùng. Vùng chọn sáng, có nhãn kích thước `{rộng} × {cao}` tính bằng pixel.
6. Chỉnh vùng chọn nếu cần: kéo 8 tay nắm ở góc và cạnh để đổi kích thước, kéo bên trong để di chuyển, kéo ở ngoài khung để chọn lại từ đầu.
7. Chọn hành động bằng nút trên thanh công cụ hoặc phím tắt.

### Thanh công cụ

Thanh có bốn nút chỉ có biểu tượng. Rê chuột vào nút để xem tên và phím tắt.

| Nút | Phím | Việc làm |
|---|---|---|
| Tải lên và chép link | `Ctrl+D` | Tải ảnh lên kho R2, chép link vào clipboard, hiện thông báo "Đã chép link". |
| Lưu vào máy | `Ctrl+S` | Lưu PNG vào thư mục lưu, không hỏi lại. |
| Chép ảnh | `Ctrl+C` | Chép ảnh vào clipboard. |
| Hủy | `Esc` | Đóng lớp phủ, không làm gì. |

- `Ctrl+D`, `Ctrl+S`, `Ctrl+C` chỉ có tác dụng khi đã có vùng chọn. Khi chưa có vùng chọn, các phím này không làm gì.
- `Esc` đóng lớp phủ bất cứ lúc nào, kể cả khi đang kéo chuột.

### Lưu vào máy

- Định dạng PNG, thư mục mặc định `Pictures\darkshot` (đổi được trong Tùy chọn, tab Thông thường; thư mục chưa có thì được tạo).
- Tên file: `darkshot_YYYY-MM-DD_HH-MM-SS.png` theo giờ máy. Nếu trùng tên thì thêm `_2`, `_3`.
- Ảnh lưu vào máy luôn là PNG, kể cả khi chọn JPEG cho ảnh tải lên và kể cả bản lưu dự phòng sau khi tải lên thất bại.

### Tải lên

- Ảnh được đưa vào thư mục hạn trên kho. Link có dạng `{địa chỉ công khai}/{thư mục hạn}/{tên}.png`, ví dụ `https://img.example.com/d30/k3X9f2abQ7mZp1Lw.png`. Khi chọn JPEG trong tab Định dạng, đuôi là `.jpg`.
- Tên ảnh gồm 16 ký tự chữ và số, sinh từ bộ sinh số ngẫu nhiên của Windows.
- Tải lên thành công: link được chép vào clipboard, thông báo "Đã chép link" (có thể tắt, xem mục "Thông báo").
- Tải lên thất bại (mất mạng, khóa sai, kho không tồn tại, địa chỉ công khai không mở được): ảnh vẫn được lưu vào thư mục lưu. Thông báo "Không tải lên được" nêu lý do và đường dẫn ảnh đã lưu.
- Chưa cấu hình R2 mà bấm tải lên: ảnh được lưu vào thư mục lưu, thông báo nói rõ chưa cấu hình, và cửa sổ Tùy chọn tự mở ở tab Lưu trữ R2.

### Hạn của link

Mục **Hạn của link** nằm trong tab **Lưu trữ R2** của Tùy chọn, có bốn lựa chọn:

| Lựa chọn | Thư mục trên kho | Ghi chú |
|---|---|---|
| 1 ngày | `d1/` | |
| 7 ngày | `d7/` | |
| 30 ngày | `d30/` | Mặc định. |
| Không hết hạn | `keep/` | Không có quy tắc xóa cho thư mục này. |

Hạn áp dụng cho mọi lần tải lên sau khi bấm OK. Không chọn hạn riêng cho từng ảnh.

### Tùy chọn

Cửa sổ Tùy chọn có bốn tab:

- **Thông thường:** **Khởi động cùng Windows**, ba ô tích **Hiện các thông báo lưu và sao chép**, **Giữ vị trí vùng đã chọn**, **Bao gồm con trỏ chuột vào ảnh chụp màn hình**, Thư mục lưu (nút **Chọn…**), Ngôn ngữ.
- **Phím tắt:** ba dòng phím tắt, mỗi dòng có ô tích bật/tắt, ô phím chỉ đọc và nút **Đổi…** (xem mục "Phím tắt chụp").
- **Định dạng:** hai nút chọn **PNG** (mặc định) và **JPEG**, thanh trượt **Chất lượng JPEG** từ 50 đến 100 (mặc định 90). Thanh trượt và số mờ khi chọn PNG. Chỉ áp dụng cho ảnh tải lên; ảnh lưu vào máy luôn là PNG. JPEG vẫn là nén có mất mát, kể cả ở mức 100.
- **Lưu trữ R2:** Account ID, Access Key ID, Secret Access Key, Tên kho (bucket), Địa chỉ công khai, Hạn của link, nút **Kiểm tra kết nối**.

Nút **OK** ghi cài đặt rồi đóng cửa sổ. Nút **Hủy** bỏ thay đổi. Nút **Trợ giúp** mở cửa sổ Trợ giúp và không đóng Tùy chọn.

- **Địa chỉ công khai** phải bắt đầu bằng `https://`, không có khoảng trắng. Dấu `/` cuối được bỏ tự động khi lưu.
- **Secret Access Key** không hiện lại giá trị đã lưu. Ô hiện dòng "Đã lưu khóa; để trống nếu không đổi." Để trống ô này khi bấm OK thì khóa cũ vẫn được dùng.
- Ô sai (ví dụ địa chỉ công khai sai, thư mục lưu trống) làm OK không đóng cửa sổ. Cửa sổ chuyển đến tab chứa ô đó và báo lỗi ngay trong cửa sổ.

### Ngôn ngữ

Giao diện có hai ngôn ngữ: Tiếng Việt và English. Đổi ở Tùy chọn, tab Thông thường, mục Ngôn ngữ; có hiệu lực ngay khi bấm OK.

- Khi chưa có file cấu hình (lần chạy đầu) hoặc file cấu hình bị hỏng: theo ngôn ngữ giao diện của Windows. Windows dùng tiếng Việt thì là Tiếng Việt, còn lại là English.
- File cấu hình của bản cũ không có mục ngôn ngữ: Tiếng Việt.
- **Kiểm tra kết nối** tải lên một file thử nhỏ, tải lại qua địa chỉ công khai để chắc link mở được, rồi xóa file thử. Kết quả hiện ngay bên cạnh nút. Thông báo lỗi phân biệt lỗi tải lên với lỗi địa chỉ công khai. File thử nằm ở `d1/darkshot-test-{16 ký tự}.txt`. Nếu bước xóa thất bại, kết quả báo lỗi quyền dù tải lên đã được, và file thử ở lại trên kho đến khi quy tắc `d1/` xóa nó.

### Phím tắt chụp

Tab **Phím tắt** của Tùy chọn có ba dòng. Mỗi dòng có ô tích bật/tắt, ô phím chỉ đọc và nút **Đổi…**.

| Dòng | Mặc định | Việc làm |
|---|---|---|
| Chụp vùng màn hình | Bật, `PrintScreen` | Mở lớp phủ để chọn vùng. |
| Lưu ảnh (toàn màn hình) | Tắt, `Shift+PrintScreen` | Lưu PNG của màn hình đang có con trỏ chuột, không mở lớp phủ. |
| Tải lên nhanh (toàn màn hình) | Tắt, `Ctrl+PrintScreen` | Tải lên ảnh của màn hình đang có con trỏ chuột và chép link, không mở lớp phủ. |

- "Toàn màn hình" là màn hình đang có con trỏ chuột lúc bấm phím. Nếu con trỏ không nằm trên màn hình nào thì lấy màn hình gần nhất. Ảnh là cả màn hình: không có vùng chọn và không có thanh công cụ.
- Hai phím toàn màn hình không đổi vùng đã nhớ của mục "Giữ vị trí vùng chọn".
- Bỏ tích một dòng thì phím của dòng đó không còn được đăng ký; ô phím và nút **Đổi…** mờ đi. Tổ hợp của dòng vẫn được giữ, tích lại là dùng được.
- Hai dòng đang bật không được trùng tổ hợp: bấm OK khi trùng thì cửa sổ báo lỗi và không đóng. Dòng đang tắt có thể trùng với dòng khác.
- Để đổi: bấm **Đổi…** rồi nhấn tổ hợp mới. Bấm **Esc** để bỏ qua việc đổi.
- Phím thường phải kèm **Ctrl**, **Alt** hoặc **Win**. Ví dụ `Ctrl+Shift+S` được nhận, còn chữ `S` một mình thì không.
- Các phím được đứng riêng: `PrintScreen`, `Pause`, `ScrollLock`, `F1` đến `F24`.
- Shift một mình không đủ. `Alt+F4` vẫn là lệnh đóng cửa sổ của Windows.
- Nếu tổ hợp đang bị ứng dụng khác giữ (ví dụ Lightshot giữ `PrintScreen`), Tùy chọn báo "... đang được ứng dụng khác dùng. Hãy chọn phím khác." và không đóng cửa sổ. Ứng dụng vẫn chạy, và vẫn chụp được qua menu khay.
- Khi khởi động, dòng bật nào không đăng ký được thì biểu tượng khay hiện thông báo "Phím tắt chưa hoạt động", nêu tổ hợp bị chiếm và gợi ý mở Tùy chọn. Mọi dòng lỗi gom vào một thông báo.

### Thông báo

- Ô **Hiện các thông báo lưu và sao chép** (mặc định bật) quyết định ba thông báo thành công: "Đã lưu ảnh", "Đã chép ảnh" và "Đã chép link". Tắt ô này thì ba thông báo đó không hiện.
- Thông báo lỗi luôn hiện, kể cả khi tắt ô.

### Giữ vị trí vùng chọn

- Bật **Giữ vị trí vùng đã chọn** (mặc định tắt): lần chụp vùng sau mở lớp phủ với khung của lần trước, đã chọn sẵn, có tay nắm, nhãn kích thước và thanh công cụ.
- Khung được nhớ khi lượt chụp vùng kết thúc bằng Lưu, Chép hoặc Tải lên. Bấm Esc thì giữ khung cũ.
- Khung vẫn được nhớ khi ô đang tắt, nên bật ô lên là dùng được ngay.
- Khung chỉ dùng lại khi nằm trọn trong ảnh chụp hiện tại và chạm ít nhất một màn hình đang có. Nếu không, lớp phủ mở trống.
- Chỉ nhớ tọa độ, không lưu ảnh. Vị trí chỉ nhớ trong lúc ứng dụng chạy: thoát rồi mở lại thì lớp phủ mở trống.

### Con trỏ chuột trong ảnh

- Bật **Bao gồm con trỏ chuột vào ảnh chụp màn hình** (mặc định tắt): con trỏ được vẽ vào ảnh ngay lúc chụp, nên ảnh nền của lớp phủ cũng có con trỏ đứng yên.
- Con trỏ đang ẩn thì không được vẽ.
- Con trỏ được vẽ ở cỡ mà Windows trả về lúc chụp. Trên màn hình có DPI khác 100%, hoặc khi con trỏ được phóng to trong Cài đặt Windows, con trỏ trong ảnh có thể nhỏ hơn trên màn hình.

### Menu khay

| Mục | Việc làm |
|---|---|
| Chụp màn hình | Mở lớp phủ chụp. |
| Tùy chọn... | Mở cửa sổ Tùy chọn. Nếu đã mở thì đưa cửa sổ đó ra trước. |
| Thông tin... | Mở cửa sổ Thông tin: biểu tượng, tên, phiên bản và mô tả ngắn. |
| Trợ giúp | Mở cửa sổ Trợ giúp: các bước dùng darkshot và chuẩn bị Cloudflare R2, theo phím tắt đang đặt. |
| Thoát | Thoát ứng dụng và xóa biểu tượng khỏi khay. |

Thông tin và Trợ giúp không chặn ứng dụng: lúc hai cửa sổ này mở, vẫn chụp, dùng menu và mở Tùy chọn được. Mở lại khi đang mở thì chỉ đưa cửa sổ ra trước. Đóng bằng Enter, Esc, nút X hoặc nút Đóng.

Chỉ một bản darkshot chạy cùng lúc. Mở bản thứ hai thì bản mới thoát ngay.

## Giao diện

- darkshot luôn dùng giao diện tối, không theo chế độ sáng hoặc tối của Windows.
- Thông báo ở khay và hộp chọn thư mục vẫn theo chế độ của Windows, sáng hoặc tối.
- Thanh tiêu đề tối cần Windows 10 bản 20H1 trở lên. Bản cũ hơn vẫn chạy, với thanh tiêu đề sáng.
- Menu khay dùng nền tối khi Windows cho phép. Nếu không, menu vẫn chạy với nền sáng.
- Biểu tượng: nền than chì, viền xám nhạt, bốn góc khung màu xanh `#4C8DFF`, chấm sáng ở giữa (cỡ 16 đến 24 px không có chấm). Nếu khay, thanh tác vụ hoặc Explorer vẫn hiện biểu tượng xanh cũ sau khi cập nhật, khởi động lại Explorer (Task Manager, **Windows Explorer**, **Restart**) để xóa bộ đệm biểu tượng.
- Ô tích, nút chọn, thanh trượt, ô nhập và hộp chọn trong Tùy chọn được vẽ lại theo màu tối, cạnh khử răng cưa; bàn phím, Tab và Trình tường thuật vẫn dùng như điều khiển gốc của Windows.

## Chuẩn bị Cloudflare R2

Các bước dưới đây theo tài liệu R2 của Cloudflare, đối chiếu ngày 2026-11-16. Giao diện Cloudflare có thể đổi tên nút. Nếu tên không khớp, hãy tìm theo mô tả trong tài liệu.

Tài liệu tham khảo:

- Tạo bucket: https://developers.cloudflare.com/r2/buckets/create-buckets/
- Truy cập công khai: https://developers.cloudflare.com/r2/buckets/public-buckets/
- API token: https://developers.cloudflare.com/r2/api/tokens/
- Quy tắc vòng đời: https://developers.cloudflare.com/r2/buckets/object-lifecycles/

### Bước 1: Tạo tài khoản và bucket

1. Tạo tài khoản Cloudflare, đăng nhập trang quản lý, rồi mở mục **R2 object storage**.
2. Tạo một bucket (kho) mới. Tên bucket chỉ gồm chữ thường `a-z`, số `0-9` và dấu gạch ngang `-`, dài từ 3 đến 63 ký tự, không bắt đầu hoặc kết thúc bằng `-`. Đây là quy tắc của Cloudflare; darkshot không tự kiểm độ dài hay chữ hoa.
3. Ghi lại **Account ID** (hiện trong trang tổng quan R2). Đây là giá trị mẫu, không phải khóa: `1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d`.

### Bước 2: Bật truy cập công khai

Ảnh cần mở được bằng trình duyệt, nên bucket phải bật truy cập công khai. Có hai cách:

- **Tên miền riêng (khuyến nghị cho dùng lâu dài):** mở bucket, chọn tab **Settings**, mục **Custom Domains**, bấm **Add**, nhập tên miền, bấm **Continue** rồi **Connect Domain**. Tên miền phải đã là một zone trong cùng tài khoản Cloudflare. Địa chỉ công khai là `https://{tên miền}`.
- **r2.dev:** mở bucket, tab **Settings**, mục **Public Development URL**, bấm **Enable**, gõ `allow` vào hộp xác nhận rồi bấm **Allow**. Địa chỉ công khai có dạng `https://pub-{mã}.r2.dev`. Cloudflare ghi rõ cách này chỉ dùng cho phát triển.

Kho không được bật liệt kê nội dung công khai. Tài liệu Cloudflare cho biết hiện tại truy cập theo từng đối tượng vẫn hoạt động, còn liệt kê thư mục gốc của tên miền thì không.

### Bước 3: Tạo API token chỉ cho một bucket

1. Trong trang **R2 object storage**, mục **Overview**, phần **Account Details**, bấm **Manage** bên cạnh **API Tokens**.
2. Chọn **Create User API token** (gắn với tài khoản cá nhân) hoặc **Create Account API token** (gắn với tài khoản Cloudflare).
3. Ở mục **Permissions**, chọn **Object Read & Write**.
4. Ở phạm vi bucket, chọn đúng bucket của bước 1. Nếu bỏ qua, token có quyền trên mọi bucket của tài khoản.
5. Bấm nút **Create Account API token** hoặc **Create User API token**, tùy loại đã chọn. Cloudflare hiện **Access Key ID** và **Secret Access Key** một lần duy nhất. Hãy lưu cả hai ngay lập tức.

Endpoint của kho có dạng `https://{Account ID}.r2.cloudflarestorage.com`. Ứng dụng tự ghép địa chỉ này từ Account ID.

Khóa này chỉ đọc và ghi đối tượng trong đúng một kho. Nó không có quyền quản lý tài khoản hay kho khác.

### Bước 4: Tạo ba quy tắc vòng đời

Quy tắc vòng đời do bạn tự tạo trên Cloudflare. Ứng dụng không tạo quy tắc và không kiểm tra được quy tắc có tồn tại hay không.

**Nếu không tạo quy tắc, ảnh sẽ không tự hết hạn, dù bạn đã chọn 1 hoặc 7 ngày trong ứng dụng.**

Tạo ba quy tắc:

| Prefix | Số ngày | Ảnh bị xóa sau |
|---|---|---|
| `d1/` | 1 | 1 ngày |
| `d7/` | 7 | 7 ngày |
| `d30/` | 30 | 30 ngày |

Cách tạo:

1. Mở bucket, chọn tab **Settings**, mục **Object Lifecycle Rules**, bấm **Add rule**.
2. Điền tên quy tắc, prefix (ví dụ `d1/`) và số ngày hết hạn. Tên các ô nhập có thể khác trên giao diện hiện tại, nên đối chiếu với tài liệu Cloudflare.
3. Bấm **Save changes**.
4. Lặp lại cho `d7/` và `d30/`. Không tạo quy tắc cho `keep/`.

Sau khi tạo, mở lại mục **Object Lifecycle Rules** và kiểm tra đủ ba quy tắc, prefix đúng từng chữ và số ngày đúng.

Độ trễ xóa: theo tài liệu Cloudflare, đối tượng thường bị xóa trong vòng 24 giờ kể từ thời điểm hết hạn. Ảnh có thể còn trên kho thêm khoảng một ngày sau hạn.

### Bước 5: Nhập vào darkshot

1. Mở Tùy chọn, tab Lưu trữ R2, nhập Account ID, Access Key ID, Secret Access Key, Tên kho và Địa chỉ công khai (ví dụ `https://img.example.com`).
2. Bấm **Kiểm tra kết nối**. Chờ thông báo "Thành công: tải lên được và link mở được."
3. Bấm **OK**.

## Kiểm tra quy tắc xóa

Ba quy tắc chỉ hoạt động nếu đã tạo đúng. Kiểm tra như sau:

1. Trong Tùy chọn, tab Lưu trữ R2, chọn **Hạn của link** là **1 ngày**, bấm **OK**.
2. Chụp một vùng bất kỳ, bấm `Ctrl+D`. Chép link, ví dụ `https://img.example.com/d1/{16 ký tự}.png`. Link phải có `/d1/`.
3. Mở link trong trình duyệt và thấy ảnh.
4. Đợi qua hạn 1 ngày, cộng thêm độ trễ xóa tối đa khoảng 24 giờ. Mở lại link: phải thấy lỗi không tìm thấy.
5. Nếu ảnh vẫn còn sau thời gian này, kiểm tra lại quy tắc `d1/` trong trang Cloudflare.

Sau khi kiểm tra xong, đặt lại **Hạn của link** theo ý bạn.

## Phím PrintScreen

Nếu `PrintScreen` không mở được lớp phủ, một ứng dụng khác đang giữ phím này.

- **Lightshot:** thoát hẳn Lightshot trước khi dùng darkshot. Cách tắt việc Lightshot giữ `PrintScreen` tùy phiên bản; chưa đối chiếu với tài liệu của Lightshot.
- **Snipping Tool của Windows 11:** tắt việc gán `PrintScreen` cho công cụ cắt. Vào **Settings** > **Accessibility** > **Keyboard** rồi tắt mục dùng nút Print Screen để mở công cụ cắt. Tên mục có thể khác theo bản Windows. Tham khảo: https://support.microsoft.com/en-gb/accessibility/windows/make-your-mouse-keyboard-and-other-input-devices-easier-to-use

Nếu không muốn tắt, đổi phím tắt chụp trong Tùy chọn của darkshot (tab Phím tắt) sang tổ hợp khác, ví dụ `Ctrl+Shift+S`. Bỏ tích dòng "Chụp vùng màn hình" cũng trả `PrintScreen` về cho Windows; menu khay vẫn chụp được.

## Bảo mật

- Cấu hình nằm ở `%APPDATA%\darkshot\config.json`.
- **Secret Access Key** được mã hóa bằng DPAPI của Windows, gắn với tài khoản Windows hiện tại, trước khi ghi vào file. Các mục khác ghi dạng văn bản thường. Nếu chép file sang tài khoản Windows khác thì khóa không đọc được, và Tùy chọn sẽ yêu cầu nhập lại.
- Ứng dụng tải ảnh thẳng lên địa chỉ S3 của R2 qua HTTPS, yêu cầu được ký theo AWS Signature V4. Không có máy chủ trung gian.
- Ứng dụng chỉ mở kết nối đến hai địa chỉ: `https://{Account ID}.r2.cloudflarestorage.com` và địa chỉ công khai của kho do bạn nhập. Tên miền của hai địa chỉ này vẫn được hỏi qua máy chủ DNS mà Windows đang dùng, nên máy chủ DNS đó thấy được tên miền.
- Ảnh và link do darkshot chép vào clipboard được đánh dấu bằng các định dạng `ExcludeClipboardContentFromMonitorProcessing`, `CanIncludeInClipboardHistory` = 0 và `CanUploadToCloudClipboard` = 0 theo tài liệu của Microsoft, để Windows không đưa chúng vào lịch sử clipboard (`Win+V`) và không đồng bộ sang thiết bị khác. Đây là quy ước của Windows; trình quản lý clipboard của hãng khác có thể không tuân theo.
- Lúc khởi động, darkshot tự loại mình khỏi Báo cáo lỗi của Windows (WER) cho tài khoản hiện tại, để khi ứng dụng sự cố Windows không gửi bản kết xuất bộ nhớ đi. Việc này ghi một giá trị DWORD `darkshot.exe` = 1 vào `HKEY_CURRENT_USER\Software\Microsoft\Windows\Windows Error Reporting\ExcludedApplications`. Muốn gỡ, xóa giá trị đó (ví dụ `reg delete "HKCU\Software\Microsoft\Windows\Windows Error Reporting\ExcludedApplications" /v darkshot.exe /f`). Chính sách WER do quản trị viên đặt cho cả máy vẫn có thể ghi đè. Việc loại trừ có hiệu lực từ sau lần darkshot khởi động đầu tiên, và chưa được thử bằng một sự cố thật.
- Nếu bạn chọn Thoát (hoặc tắt máy, đăng xuất) lúc một lượt tải lên còn chạy, ảnh của lượt đó được lưu PNG vào thư mục lưu trước khi thoát. Có thể ảnh vừa được lưu vừa đã lên kho.
- HTTPS dùng thư viện `rustls` với danh sách chứng chỉ gốc đóng kèm trong file, không dùng bộ chứng chỉ của Windows.
- Cửa sổ Thông tin và Trợ giúp không mở trang web và không gọi mạng.
- Ứng dụng không ghi log ra file. Không gửi thống kê, không kiểm tra cập nhật.
- Ảnh chụp chỉ nằm trong bộ nhớ đến khi bạn chọn Lưu hoặc Tải lên.
- **Ai có link đều xem được ảnh.** Sự riêng tư dựa vào tên ngẫu nhiên khó đoán và hạn của link. Không bật liệt kê nội dung công khai cho kho.
- **Ứng dụng không xóa được ảnh chụp đã tải lên.** (Nó chỉ xóa file thử của nút Kiểm tra kết nối.) Muốn xóa, vào trang Cloudflare R2, mở bucket và xóa đối tượng đó.
- Link "Không hết hạn" nằm trong thư mục `keep/`. Ảnh ở đó vẫn nằm trên kho cho đến khi bạn tự xóa trong trang Cloudflare.

## Phạm vi

Ứng dụng làm một việc: chụp vùng màn hình, lưu hoặc tải lên và chép link. Không có chức năng vẽ lên ảnh, lịch sử ảnh, hay xóa ảnh từ ứng dụng.

## Thư viện bên thứ ba

Ảnh JPEG được nén bằng crate `jpeg-encoder`; phần mềm này dựa một phần trên công trình của Independent JPEG Group.
