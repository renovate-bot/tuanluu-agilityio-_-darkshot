# shotme — thiết kế bản đầu

Ngày: 2026-11-16

Quy ước: mục ghi **(đã chốt)** là quyết định của chủ dự án trong buổi trao đổi. Mục ghi **(đề xuất)** là lựa chọn của người viết thiết kế, chủ dự án chưa xác nhận riêng từng mục và có thể đổi.

## 1. Mục đích

Chủ dự án đang dùng Lightshot để chụp màn hình và chia sẻ link ảnh. Lightshot tải ảnh lên máy chủ của hãng (prnt.sc). Vì lý do an toàn và bảo mật, chủ dự án không muốn ảnh của mình nằm trên máy chủ của bên đó.

shotme là ứng dụng thay thế cho riêng chủ dự án: trải nghiệm chụp giống Lightshot, nhưng ảnh chia sẻ nằm trong kho lưu trữ đứng tên chủ dự án.

Người dùng: một người (chủ dự án), trên Windows 11.

## 2. Mục tiêu, ngoài phạm vi, điều phải tránh

### Mục tiêu (đã chốt)

1. Chụp một vùng màn hình bằng phím tắt.
2. Lưu ảnh ra file trên máy.
3. Tải ảnh lên kho của chủ dự án và nhận link chia sẻ.
4. Link chia sẻ tự hết hạn.
5. Ứng dụng gọn nhẹ: file nhỏ, tốn ít RAM khi chạy ngầm, phản hồi ngay khi bấm phím tắt.

### Ngoài phạm vi bản đầu (đã chốt)

- Vẽ lên ảnh: mũi tên, chữ, khung, làm mờ.
- Lịch sử ảnh đã tải lên.
- Xóa ảnh trên kho từ trong ứng dụng.
- Chụp theo cửa sổ, chụp cuộn trang, quay video.
- Bản chạy trên macOS và Linux. Kiến trúc phải sẵn sàng cho việc thêm sau (xem mục 10), nhưng bản đầu chỉ xây và kiểm thử trên Windows.

### Điều phải tránh

Ứng dụng coi như thất bại, dù chạy được, nếu vi phạm một trong các điều sau:

- Gửi ảnh hoặc bất kỳ dữ liệu nào đến máy chủ khác ngoài địa chỉ R2 mà chủ dự án cấu hình. Không thống kê sử dụng, không kiểm tra cập nhật, không báo lỗi tự động.
- Ghi khóa bí mật R2 ra đĩa hoặc ra log ở dạng đọc được.
- Tạo link có thể đoán được (tên theo thứ tự, theo thời gian, hoặc quá ngắn).
- Làm mất ảnh vừa chụp khi tải lên thất bại.

## 3. Ràng buộc

- Hệ điều hành: Windows 10 và 11, 64-bit. Máy phát triển là Windows 11.
- Máy phát triển chưa có Rust và bộ build tools C++ của Microsoft; phải cài trước khi làm.
- Thư mục dự án `D:\2026 - Tuan\shotme` đang trống và chưa là kho git.
- Lightshot đang cài trên máy và đang giữ phím `PrintScreen` khi nó chạy.

## 4. Các cách đã cân nhắc

### Nơi chứa ảnh chia sẻ

| Cách | Được | Mất |
|---|---|---|
| **Kho đám mây đứng tên chủ dự án (Cloudflare R2)** | Link ảnh trực tiếp, không cần nuôi máy chủ, miễn phí 10 GB | Cấu hình một lần; ảnh vẫn nằm trên hạ tầng Cloudflare |
| Máy chủ riêng (VPS, NAS) | Kiểm soát tuyệt đối | Phải viết và vận hành thêm phần mềm phía máy chủ |
| Drive cá nhân (Google Drive, OneDrive) | Không tạo gì mới, giới hạn được người xem | Link mở trang xem của Drive, đăng nhập phức tạp |

**Đã chốt: Cloudflare R2.** Lý do: gần trải nghiệm Lightshot nhất và ảnh thuộc tài khoản của chủ dự án.

### Công nghệ xây ứng dụng

| Cách | Được | Mất |
|---|---|---|
| **Rust thuần, gọi API Windows** | Một file `.exe` vài MB, RAM thấp nhất, không phụ thuộc khi chạy | Mã dài nhất, cài công cụ build nặng, cửa sổ cài đặt đơn giản |
| Go | Một file `.exe`, dễ đọc | Nặng hơn Rust, thư viện giao diện Windows ít được bảo trì |
| C# / .NET | Thư viện Windows đầy đủ nhất, viết nhanh nhất | Cần .NET runtime hoặc file 60 MB trở lên, RAM cao hơn |
| Electron | Dùng Node.js sẵn có, đa nền tảng | File 150 MB trở lên, RAM 150–300 MB, lớp phủ hiện chậm |
| Tauri | Nhẹ, đa nền tảng | Lớp phủ nhiều màn hình phải tự xử lý nhiều |

**Đã chốt: Rust thuần.** Lý do: chủ dự án ưu tiên gọn nhẹ mà vẫn đủ tính năng.

### Cách làm link tự hết hạn (đề xuất)

| Cách | Được | Mất |
|---|---|---|
| **Quy tắc vòng đời của R2 theo thư mục** | Ảnh bị xóa thật khỏi kho; link ngắn | Độ chính xác theo ngày, không theo phút |
| Link có chữ ký hết hạn (presigned URL) | Hết hạn chính xác đến giây | Link rất dài, tối đa 7 ngày, ảnh vẫn nằm lại trên kho |
| Cloudflare Worker kiểm tra hạn | Linh hoạt nhất | Thêm một thành phần phải viết và vận hành |

**Đề xuất: quy tắc vòng đời theo thư mục.** Lý do: với mục đích bảo mật, ảnh bị xóa thật tốt hơn là chỉ link hết hiệu lực.

## 5. Hành vi

### 5.1 Chạy ngầm

- Ứng dụng không có cửa sổ chính. Nó hiện một biểu tượng ở khay hệ thống.
- Chuột phải vào biểu tượng mở menu: **Chụp màn hình**, **Cài đặt**, **Thoát**.
- Chỉ một bản chạy cùng lúc. Mở lần hai thì bản mới thoát ngay.
- Có tùy chọn tự khởi động cùng Windows, mặc định tắt (đề xuất).
- Lần chạy đầu, khi chưa có cấu hình, ứng dụng mở cửa sổ Cài đặt.

### 5.2 Chụp

1. Người dùng bấm phím tắt (mặc định `PrintScreen`, đổi được) hoặc chọn **Chụp màn hình** trong menu.
2. Ứng dụng chụp toàn bộ các màn hình ngay lúc đó, rồi hiện lớp phủ toàn màn hình với ảnh vừa chụp bị làm tối. Màn hình trông như đứng yên.
3. Người dùng kéo chuột chọn vùng. Vùng chọn hiện sáng, có nhãn kích thước tính bằng pixel.
4. Sau khi thả chuột, người dùng có thể kéo cạnh, góc để đổi kích thước, hoặc kéo bên trong để di chuyển khung. Kéo ở ngoài khung thì chọn lại từ đầu.
5. Thanh công cụ gồm bốn nút chỉ có biểu tượng, không có chữ; rê chuột vào nút thì hiện tên hành động và phím tắt (đã chốt). Thanh hiện sát cạnh dưới bên phải của khung. Nếu không đủ chỗ thì hiện ở phía trên hoặc bên trong khung.

```
┌──────────────────────────────────────────┐
│░░░░░░░░░░ (màn hình tối đi) ░░░░░░░░░░░░░│
│░░░░░░ 640 × 360 ░░░░░░░░░░░░░░░░░░░░░░░░░│
│░░░░░░┌─────────────────────┐░░░░░░░░░░░░░│
│░░░░░░│                     │░░░░░░░░░░░░░│
│░░░░░░│   vùng chọn (sáng)  │░░░░░░░░░░░░░│
│░░░░░░│                     │░░░░░░░░░░░░░│
│░░░░░░└─────────────────────┘░░░░░░░░░░░░░│
│░░░░░░░░░░░░░░░░░░░░[ ↥ │ ▣ │ ⧉ │ ✕ ]░░│
└──────────────────────────────────────────┘
```

Chi tiết:

- Vùng chọn kéo được qua nhiều màn hình.
- Ảnh kết quả dùng pixel thật của màn hình, không bị co giãn theo tỉ lệ phóng to của Windows.
- Con trỏ chuột không xuất hiện trong ảnh (đề xuất).
- Bấm phím tắt khi lớp phủ đang mở thì không có gì xảy ra.

### 5.3 Hành động trên vùng chọn

| Hành động | Phím | Kết quả |
|---|---|---|
| Tải lên | `Ctrl+D` | Đóng lớp phủ, tải ảnh lên R2 ở nền, chép link vào clipboard, hiện thông báo |
| Lưu | `Ctrl+S` | Đóng lớp phủ, lưu PNG vào thư mục mặc định, hiện thông báo |
| Chép | `Ctrl+C` | Đóng lớp phủ, chép ảnh vào clipboard |
| Hủy | `Esc` | Đóng lớp phủ, không làm gì |

Các phím trên chỉ có tác dụng khi đã có vùng chọn, trừ `Esc`.

**Lưu:**

- Định dạng PNG.
- Thư mục mặc định `Pictures\shotme` của người dùng, đổi được. Tự tạo nếu chưa có.
- Tên file `shotme_YYYY-MM-DD_HH-MM-SS.png` theo giờ máy. Nếu trùng tên thì thêm `_2`, `_3`.
- Không hỏi lại, không mở hộp thoại chọn nơi lưu (đề xuất).

**Tải lên:**

- Tên ảnh trên kho là 16 ký tự ngẫu nhiên gồm chữ và số (khoảng 95 bit), sinh từ bộ sinh số ngẫu nhiên của hệ điều hành (đề xuất).
- Link có dạng `{địa chỉ công khai}/{thư mục hạn}/{tên}.png`, ví dụ `https://img.example.com/d30/k3X9f2abQ7mZp1Lw.png`.
- Tải lên thành công: link được chép vào clipboard, thông báo "Đã chép link".
- Tải lên thất bại (mất mạng, sai khóa, kho không tồn tại): ảnh được lưu vào thư mục mặc định như hành động Lưu, thông báo nêu lý do thất bại và nơi đã lưu ảnh.
- Chưa cấu hình R2 mà bấm Tải lên: ảnh được lưu local như trên, và ứng dụng mở cửa sổ Cài đặt.

**Thông báo:** dùng thông báo của biểu tượng khay hệ thống.

### 5.4 Link tự hết hạn (hạn và mặc định đã chốt; cách làm là đề xuất)

- Cài đặt có mục **Hạn của link**: 1 ngày, 7 ngày, 30 ngày, hoặc không hết hạn. Mặc định 30 ngày (đã chốt: mặc định 30 ngày và chọn được hạn khác; danh sách các hạn cụ thể là đề xuất).
- Hạn áp dụng cho mọi lần tải lên sau đó. Không chọn hạn riêng cho từng ảnh ở bản đầu (đề xuất).
- Ứng dụng đặt ảnh vào thư mục tương ứng trên kho: `d1/`, `d7/`, `d30/`, `keep/`.
- Việc xóa do R2 thực hiện theo quy tắc vòng đời gắn với từng thư mục. Ảnh bị xóa thật, link trả về lỗi không tìm thấy.
- R2 xóa theo đợt, nên ảnh có thể còn tồn tại thêm đến khoảng một ngày sau hạn. Người làm cần kiểm tra lại con số này với tài liệu của Cloudflare và ghi vào hướng dẫn.
- Chủ dự án tự tạo ba quy tắc vòng đời trong trang quản lý Cloudflare một lần, theo hướng dẫn kèm ứng dụng. Ứng dụng không tự tạo quy tắc, để khóa cấp cho ứng dụng chỉ cần quyền đọc và ghi ảnh trong đúng một kho.
- Hệ quả: ứng dụng không kiểm tra được quy tắc có tồn tại hay không. Nếu quy tắc thiếu hoặc gõ sai tên thư mục, link sẽ không hết hạn mà không có cảnh báo. Hướng dẫn phải có bước tự kiểm tra: xem lại quy tắc trong trang quản lý Cloudflare ngay sau khi tạo, rồi tải một ảnh với hạn 1 ngày và xem nó biến mất sau hạn (đề xuất).

### 5.5 Cài đặt

Một cửa sổ đơn giản với các mục:

- **R2:** Account ID, Access Key ID, Secret Access Key, tên kho (bucket), địa chỉ công khai (tên miền riêng hoặc địa chỉ `r2.dev` của kho).
- Nút **Kiểm tra kết nối** (đề xuất): tải lên một file thử nhỏ, tải lại nó qua địa chỉ công khai để chắc link mở được, rồi xóa. Báo thành công hoặc lý do thất bại, phân biệt lỗi tải lên với lỗi địa chỉ công khai.
- **Hạn của link:** 1 ngày, 7 ngày, 30 ngày (mặc định), không hết hạn.
- **Phím tắt chụp.**
- **Thư mục lưu.**
- **Khởi động cùng Windows.**

Ô Secret Access Key không hiện lại giá trị đã lưu.

Ngôn ngữ giao diện: tiếng Việt (đề xuất).

### 5.6 Phím tắt bị chiếm

Nếu Windows từ chối đăng ký phím tắt (ví dụ Lightshot đang chạy và giữ `PrintScreen`), ứng dụng vẫn chạy, hiện thông báo nói rõ phím nào bị chiếm, và người dùng vẫn chụp được qua menu khay hệ thống. Người dùng đổi phím trong Cài đặt hoặc tắt ứng dụng đang giữ phím.

Trên Windows 11, `PrintScreen` có thể được gán cho Snipping Tool trong cài đặt của Windows. Hướng dẫn kèm ứng dụng cần chỉ cách tắt.

## 6. Bảo mật và dữ liệu

- Cấu hình nằm ở `%APPDATA%\shotme\config.json`.
- Secret Access Key được mã hóa bằng DPAPI của Windows (gắn với tài khoản Windows hiện tại) trước khi ghi vào file. Các mục khác ghi dạng thường.
- Ứng dụng tải thẳng lên địa chỉ S3 của R2 (`https://{account id}.r2.cloudflarestorage.com`) qua HTTPS, yêu cầu được ký theo chuẩn AWS Signature V4. Không có máy chủ trung gian.
- "Địa chỉ R2 mà chủ dự án cấu hình" gồm hai địa chỉ: địa chỉ S3 ở trên và địa chỉ công khai của kho. Ứng dụng chỉ tự mở kết nối đến hai địa chỉ này (đề xuất).
- Kết nối HTTPS dùng thư viện TLS viết bằng Rust với danh sách chứng chỉ gốc đóng kèm trong file, không dùng bộ TLS của Windows, để Windows không tự gọi đến máy chủ bên thứ ba khi kiểm tra chứng chỉ (đề xuất). Đổi lại file lớn hơn khoảng 1–2 MB.
- Ứng dụng không ghi log ra file ở bản đầu (đề xuất).
- Ảnh chụp chỉ nằm trong bộ nhớ cho đến khi người dùng chọn Lưu hoặc Tải lên.
- Link công khai với bất kỳ ai có link. Sự riêng tư dựa vào tên ngẫu nhiên không đoán được và hạn của link. Kho không được bật tính năng liệt kê nội dung công khai.

## 7. Chủ dự án cần chuẩn bị

Hướng dẫn từng bước sẽ nằm trong `README.md`:

1. Tạo tài khoản Cloudflare và một kho R2.
2. Bật truy cập công khai cho kho (tên miền riêng hoặc `r2.dev`).
3. Tạo khóa API chỉ có quyền đọc và ghi ảnh trong kho đó.
4. Tạo ba quy tắc vòng đời: `d1/` xóa sau 1 ngày, `d7/` sau 7 ngày, `d30/` sau 30 ngày.
5. Nhập thông tin vào Cài đặt của shotme và bấm Kiểm tra kết nối.

## 8. Để người làm quyết định

- Cấu trúc mã và việc chia module.
- Chọn thư viện Rust. Gợi ý: bộ `windows` của Microsoft cho API Windows; một thư viện HTTP nhỏ, đồng bộ, cho việc tải lên; một thư viện mã hóa PNG. Ưu tiên ít phụ thuộc để file nhỏ.
- Cách vẽ lớp phủ và cách dựng cửa sổ Cài đặt.
- Hình dạng biểu tượng ứng dụng.
- Lời văn cụ thể của các thông báo.

## 9. Kiểm thử

- Kiểm thử tự động cho phần không có giao diện: ký yêu cầu S3 (so với ví dụ mẫu đã biết kết quả), sinh tên ngẫu nhiên, đặt tên file và xử lý trùng tên, đọc ghi cấu hình kèm mã hóa khóa, phép tính hình học của vùng chọn, ghép link theo hạn.
- Danh sách kiểm tra bằng tay cho phần giao diện: chụp trên một và nhiều màn hình, màn hình có tỉ lệ phóng to khác nhau, bốn hành động, tải lên khi mất mạng, phím tắt bị chiếm, khởi động cùng Windows.
- Người làm không được cấp khóa R2. Trong lúc phát triển, phần tải lên được kiểm bằng ví dụ ký mẫu và máy chủ giả chạy trên máy (đề xuất).
- Chủ dự án tự nhập khóa vào ứng dụng và tự chạy bước nghiệm thu với kho R2 thật, theo danh sách kiểm tra. Việc ảnh trong `d1/` biến mất cần chờ một đến hai ngày, được chủ dự án kiểm sau khi bàn giao.

## 10. Kiến trúc để mở rộng (yêu cầu đã chốt; cách chia là đề xuất)

Chủ dự án yêu cầu dự án dễ mở rộng và có thể hỗ trợ macOS, Linux sau này.

- Mã chia thành hai lớp trong một Cargo workspace:
  - **Lõi, không phụ thuộc hệ điều hành:** hình học vùng chọn, luồng bốn hành động, đặt tên file, sinh tên ngẫu nhiên, mô hình cấu hình, hạn của link, ký và tải lên S3, mã hóa PNG.
  - **Lớp nền tảng:** chụp màn hình, lớp phủ, phím tắt toàn cục, khay hệ thống, clipboard, thông báo, cất giữ khóa bí mật, khởi động cùng hệ thống, cửa sổ cài đặt. Lõi chỉ gọi các phần này qua giao diện (trait) chung.
- Bản đầu chỉ có một hiện thực của lớp nền tảng, cho Windows. Thêm macOS hoặc Linux là thêm một hiện thực mới, không sửa lõi.
- Nơi tải lên cũng nằm sau một giao diện chung, để sau này thêm được nơi chứa khác. Kho tương thích S3 khác R2 dùng được ngay bằng cách đổi địa chỉ.
- Các tính năng ngoài phạm vi (vẽ lên ảnh, lịch sử, xóa từ ứng dụng) không được xây trước, nhưng luồng hành động phải cho phép thêm hành động mới mà không viết lại lớp phủ.

## 11. Quy trình làm việc (yêu cầu đã chốt; chi tiết là đề xuất)

### 11.1 Các giai đoạn

| Giai đoạn | Kết quả | Ai duyệt |
|---|---|---|
| 1. Yêu cầu | Tài liệu này | Chủ dự án |
| 2. Prototype | `prototype/index.html`: bản giao diện bấm thử được trên trình duyệt, không chụp thật | Chủ dự án |
| 3. Thiết kế | Mục 10 và phần kiến trúc chi tiết trong kế hoạch | Chủ dự án |
| 4. Kế hoạch | Danh sách task, mỗi task có model, nhánh, tiêu chí xong | Chủ dự án |
| 5. Xây dựng | Mã và kiểm thử tự động, từng task một | Rà soát mã tự động, chủ dự án xem khi muốn |
| 6. Kiểm thử | Danh sách kiểm tra bằng tay, nghiệm thu với R2 thật | Chủ dự án |
| 7. Phát hành | `shotme.exe`, `README.md` | Chủ dự án |
| 8. Bảo trì | Danh sách việc để sau | Chủ dự án |

Prototype là bản dùng thử rồi bỏ; mã của nó không được dùng lại trong ứng dụng.

### 11.2 AI làm là chính, task chia theo model

- Mỗi task do một agent con thực hiện, phiên chính chỉ điều phối và rà soát.
- Task được viết đủ nhỏ và tự chứa: agent chỉ cần đọc mô tả task và các file được nêu tên, không cần đọc lại cả dự án.
- Mỗi task ghi rõ model theo độ khó:

| Loại task | Model |
|---|---|
| Việc máy móc: khung dự án, cấu hình, tài liệu, đổi tên | Haiku |
| Việc có đặc tả rõ: logic lõi, kiểm thử, cửa sổ cài đặt | Sonnet |
| Việc khó: lớp phủ nhiều màn hình và DPI, ký S3, cất giữ khóa | Opus |
| Điều phối, rà soát thiết kế, quyết định kiến trúc | Fable (phiên chính) |

### 11.3 Lưu tiến trình

- `docs/progress/STATE.md`: giai đoạn hiện tại, task đang làm, bước kế tiếp, các quyết định còn mở. Dài tối đa khoảng một trang.
- `docs/progress/tasks.md`: bảng mọi task với nhánh, model, trạng thái, số token.
- `CLAUDE.md` ở gốc dự án chỉ dẫn phiên mới đọc `STATE.md` trước tiên. Mở lại dự án thì đọc file đó là đủ để làm tiếp.
- `STATE.md` được cập nhật khi bắt đầu và khi kết thúc mỗi task.

### 11.4 Mỗi task một nhánh

- Dự án là kho git, nhánh chính `main`.
- Mỗi task làm trên nhánh `task/NN-ten-ngan`, gộp vào `main` sau khi kiểm thử tự động qua và mã được rà soát.
- Yêu cầu này được hiểu là chủ dự án cho phép tạo commit và gộp nhánh trong kho local của dự án. Không đẩy lên máy chủ git nào nếu chủ dự án không yêu cầu.

### 11.5 Ghi số token

- Mỗi agent con trả về số token đã dùng khi xong việc. Con số đó được ghi vào `tasks.md` cho task tương ứng, cộng dồn nếu task cần nhiều lượt (làm, sửa sau rà soát).
- Token của phiên chính (điều phối, trao đổi với chủ dự án) không đo được theo từng task. Phần này được ghi riêng thành một dòng "điều phối" nếu chủ dự án cung cấp con số từ công cụ.
- Khi dự án xong, `tasks.md` có bảng tổng theo task, theo model và theo giai đoạn.
- Các giai đoạn 1 đến 3 diễn ra trước khi có bảng theo dõi. Số token đã biết của chúng (một lượt rà soát thiết kế: 49.194 token) được ghi vào khi tạo bảng.

## 12. Xong nghĩa là gì

- Một file `shotme.exe` chạy được trên máy chưa cài gì thêm.
- Chủ dự án chụp một vùng, lưu được file, và tải lên được kho R2 của mình, nhận link mở được trên trình duyệt khác.
- Ba quy tắc xóa có trong trang quản lý Cloudflare. Ảnh tải lên với hạn 1 ngày biến mất sau hạn (kiểm sau bàn giao).
- Khi ứng dụng chạy, không có kết nối mạng nào ngoài kết nối đến R2 lúc tải lên hoặc kiểm tra kết nối.
