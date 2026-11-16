# shotme

shotme là ứng dụng Windows 11 chạy ở khay hệ thống. Bấm `PrintScreen`, kéo chọn vùng màn hình, rồi chọn một trong bốn việc: tải ảnh lên Cloudflare R2 và chép link, lưu PNG vào máy, chép ảnh vào clipboard, hoặc hủy. Ứng dụng viết bằng Rust thuần, không có máy chủ trung gian.

## Build

Yêu cầu: Rust (cài qua rustup), Windows 11.

```powershell
cargo build --release
```

File chạy nằm tại `target\release\shotme.exe`. Kích thước bản build `shotme.exe` kiểm tra ngày 2026-11-16: **1.505.792 byte** (khoảng 1,44 MiB, bản `--release` trên Windows 11).

## Cách dùng

1. Chạy `shotme.exe`. Biểu tượng xuất hiện ở khay hệ thống, chú thích "shotme".
2. Bấm chuột trái hoặc phải vào biểu tượng để mở menu: **Chụp màn hình**, **Cài đặt**, **Thoát**.
3. Lần chạy đầu, khi chưa có cấu hình, cửa sổ **Cài đặt** tự mở. Điền thông tin R2 (xem mục bên dưới) rồi bấm **Lưu**.
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

- Định dạng PNG, thư mục mặc định `Pictures\shotme` (đổi được trong Cài đặt, thư mục chưa có thì được tạo).
- Tên file: `shotme_YYYY-MM-DD_HH-MM-SS.png` theo giờ máy. Nếu trùng tên thì thêm `_2`, `_3`.

### Tải lên

- Ảnh được đưa vào thư mục hạn trên kho. Link có dạng `{địa chỉ công khai}/{thư mục hạn}/{tên}.png`, ví dụ `https://img.example.com/d30/k3X9f2abQ7mZp1Lw.png`.
- Tên ảnh gồm 16 ký tự chữ và số, sinh từ bộ sinh số ngẫu nhiên của Windows.
- Tải lên thành công: link được chép vào clipboard, thông báo "Đã chép link".
- Tải lên thất bại (mất mạng, khóa sai, kho không tồn tại, địa chỉ công khai không mở được): ảnh vẫn được lưu vào thư mục lưu. Thông báo "Không tải lên được" nêu lý do và đường dẫn ảnh đã lưu.
- Chưa cấu hình R2 mà bấm tải lên: ảnh được lưu vào thư mục lưu, thông báo nói rõ chưa cấu hình, và cửa sổ Cài đặt tự mở.

### Hạn của link

Mục **Hạn của link** trong Cài đặt có bốn lựa chọn:

| Lựa chọn | Thư mục trên kho | Ghi chú |
|---|---|---|
| 1 ngày | `d1/` | |
| 7 ngày | `d7/` | |
| 30 ngày | `d30/` | Mặc định. |
| Không hết hạn | `keep/` | Không có quy tắc xóa cho thư mục này. |

Hạn áp dụng cho mọi lần tải lên sau khi lưu cài đặt. Không chọn hạn riêng cho từng ảnh.

### Cài đặt

Cửa sổ Cài đặt có ba nhóm:

- **Cloudflare R2:** Account ID, Access Key ID, Secret Access Key, Tên kho (bucket), Địa chỉ công khai, nút **Kiểm tra kết nối**.
- **Chia sẻ và lưu:** Hạn của link, Thư mục lưu (nút **Chọn…**).
- **Chung:** Phím tắt chụp (nút **Đổi…**), **Khởi động cùng Windows**.

Nút **Lưu** ghi cài đặt. Nút **Hủy** bỏ thay đổi.

- **Địa chỉ công khai** phải bắt đầu bằng `https://`, không có khoảng trắng. Dấu `/` cuối được bỏ tự động khi lưu.
- **Secret Access Key** không hiện lại giá trị đã lưu. Ô hiện dòng "Đã lưu khóa; để trống nếu không đổi." Để trống ô này khi lưu thì khóa cũ vẫn được dùng.
- **Kiểm tra kết nối** tải lên một file thử nhỏ, tải lại qua địa chỉ công khai để chắc link mở được, rồi xóa file thử. Kết quả hiện ngay bên cạnh nút. Thông báo lỗi phân biệt lỗi tải lên với lỗi địa chỉ công khai. File thử nằm ở `d1/shotme-test-{16 ký tự}.txt`. Nếu bước xóa thất bại, kết quả báo lỗi quyền dù tải lên đã được, và file thử ở lại trên kho đến khi quy tắc `d1/` xóa nó.

### Phím tắt chụp

- Mặc định là `PrintScreen`. Bấm **Đổi…** rồi nhấn tổ hợp mới. Bấm **Esc** để bỏ qua việc đổi.
- Phím thường phải kèm **Ctrl**, **Alt** hoặc **Win**. Ví dụ `Ctrl+Shift+S` được nhận, còn chữ `S` một mình thì không.
- Các phím được đứng riêng: `PrintScreen`, `Pause`, `ScrollLock`, `F1` đến `F24`.
- Shift một mình không đủ. `Alt+F4` vẫn là lệnh đóng cửa sổ của Windows.
- Nếu phím đang bị ứng dụng khác giữ (ví dụ Lightshot giữ `PrintScreen`), Cài đặt báo "đang bị ứng dụng khác chiếm" và không lưu. Ứng dụng vẫn chạy, và vẫn chụp được qua menu khay.
- Nếu phím bị chiếm lúc shotme khởi động, biểu tượng khay hiện thông báo "Phím … đang bị ứng dụng khác chiếm" và gợi ý đổi phím trong Cài đặt.

### Menu khay

| Mục | Việc làm |
|---|---|
| Chụp màn hình | Mở lớp phủ chụp. |
| Cài đặt | Mở cửa sổ Cài đặt. Nếu đã mở thì đưa cửa sổ đó ra trước. |
| Thoát | Thoát ứng dụng và xóa biểu tượng khỏi khay. |

Chỉ một bản shotme chạy cùng lúc. Mở bản thứ hai thì bản mới thoát ngay.

## Chuẩn bị Cloudflare R2

Các bước dưới đây theo tài liệu R2 của Cloudflare, đối chiếu ngày 2026-11-16. Giao diện Cloudflare có thể đổi tên nút. Nếu tên không khớp, hãy tìm theo mô tả trong tài liệu.

Tài liệu tham khảo:

- Tạo bucket: https://developers.cloudflare.com/r2/buckets/create-buckets/
- Truy cập công khai: https://developers.cloudflare.com/r2/buckets/public-buckets/
- API token: https://developers.cloudflare.com/r2/api/tokens/
- Quy tắc vòng đời: https://developers.cloudflare.com/r2/buckets/object-lifecycles/

### Bước 1: Tạo tài khoản và bucket

1. Tạo tài khoản Cloudflare, đăng nhập trang quản lý, rồi mở mục **R2 object storage**.
2. Tạo một bucket (kho) mới. Tên bucket chỉ gồm chữ thường `a-z`, số `0-9` và dấu gạch ngang `-`, dài từ 3 đến 63 ký tự, không bắt đầu hoặc kết thúc bằng `-`. Đây là quy tắc của Cloudflare; shotme không tự kiểm độ dài hay chữ hoa.
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

### Bước 5: Nhập vào shotme

1. Mở Cài đặt, nhập Account ID, Access Key ID, Secret Access Key, Tên kho và Địa chỉ công khai (ví dụ `https://img.example.com`).
2. Bấm **Kiểm tra kết nối**. Chờ thông báo "Thành công: tải lên được và link mở được."
3. Bấm **Lưu**.

## Kiểm tra quy tắc xóa

Ba quy tắc chỉ hoạt động nếu đã tạo đúng. Kiểm tra như sau:

1. Trong Cài đặt, chọn **Hạn của link** là **1 ngày**, bấm **Lưu**.
2. Chụp một vùng bất kỳ, bấm `Ctrl+D`. Chép link, ví dụ `https://img.example.com/d1/{16 ký tự}.png`. Link phải có `/d1/`.
3. Mở link trong trình duyệt và thấy ảnh.
4. Đợi qua hạn 1 ngày, cộng thêm độ trễ xóa tối đa khoảng 24 giờ. Mở lại link: phải thấy lỗi không tìm thấy.
5. Nếu ảnh vẫn còn sau thời gian này, kiểm tra lại quy tắc `d1/` trong trang Cloudflare.

Sau khi kiểm tra xong, đặt lại **Hạn của link** theo ý bạn.

## Phím PrintScreen

Nếu `PrintScreen` không mở được lớp phủ, một ứng dụng khác đang giữ phím này.

- **Lightshot:** thoát hẳn Lightshot trước khi dùng shotme. Cách tắt việc Lightshot giữ `PrintScreen` tùy phiên bản; chưa đối chiếu với tài liệu của Lightshot.
- **Snipping Tool của Windows 11:** tắt việc gán `PrintScreen` cho công cụ cắt. Vào **Settings** > **Accessibility** > **Keyboard** rồi tắt mục dùng nút Print Screen để mở công cụ cắt. Tên mục có thể khác theo bản Windows. Tham khảo: https://support.microsoft.com/en-gb/accessibility/windows/make-your-mouse-keyboard-and-other-input-devices-easier-to-use

Nếu không muốn tắt, đổi phím tắt chụp trong Cài đặt của shotme sang tổ hợp khác, ví dụ `Ctrl+Shift+S`.

## Bảo mật

- Cấu hình nằm ở `%APPDATA%\shotme\config.json`.
- **Secret Access Key** được mã hóa bằng DPAPI của Windows, gắn với tài khoản Windows hiện tại, trước khi ghi vào file. Các mục khác ghi dạng văn bản thường. Nếu chép file sang tài khoản Windows khác thì khóa không đọc được, và Cài đặt sẽ yêu cầu nhập lại.
- Ứng dụng tải ảnh thẳng lên địa chỉ S3 của R2 qua HTTPS, yêu cầu được ký theo AWS Signature V4. Không có máy chủ trung gian.
- Ứng dụng chỉ mở kết nối đến hai địa chỉ: `https://{Account ID}.r2.cloudflarestorage.com` và địa chỉ công khai của kho do bạn nhập.
- HTTPS dùng thư viện `rustls` với danh sách chứng chỉ gốc đóng kèm trong file, không dùng bộ chứng chỉ của Windows.
- Ứng dụng không ghi log ra file. Không gửi thống kê, không kiểm tra cập nhật.
- Ảnh chụp chỉ nằm trong bộ nhớ đến khi bạn chọn Lưu hoặc Tải lên.
- **Ai có link đều xem được ảnh.** Sự riêng tư dựa vào tên ngẫu nhiên khó đoán và hạn của link. Không bật liệt kê nội dung công khai cho kho.
- **Ứng dụng không xóa được ảnh chụp đã tải lên.** (Nó chỉ xóa file thử của nút Kiểm tra kết nối.) Muốn xóa, vào trang Cloudflare R2, mở bucket và xóa đối tượng đó.
- Link "Không hết hạn" nằm trong thư mục `keep/`. Ảnh ở đó vẫn nằm trên kho cho đến khi bạn tự xóa trong trang Cloudflare.

## Phạm vi

Ứng dụng làm một việc: chụp vùng màn hình, lưu hoặc tải lên và chép link. Không có chức năng vẽ lên ảnh, lịch sử ảnh, hay xóa ảnh từ ứng dụng.
