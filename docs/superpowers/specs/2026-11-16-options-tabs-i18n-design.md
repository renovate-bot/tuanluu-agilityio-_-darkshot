# shotme đợt 1: menu mới, cửa sổ Tùy chọn dạng tab, đa ngôn ngữ

Ngày: 2026-11-16. Trạng thái: chủ dự án đã duyệt (2026-11-16).

Tài liệu này bổ sung cho spec gốc `2026-11-16-shotme-design.md` (không sửa spec gốc). Chỗ nào hai tài liệu khác nhau thì tài liệu này thắng, và điểm khác được ghi vào `memory-bank/decisions.md`.

## 1. Ý định

Chủ dự án muốn giao diện gọn hơn và muốn thêm tính năng mới cho ứng dụng, lấy bố cục hộp thoại Tùy chọn trong các ảnh họ gửi làm mẫu. Việc được chia ba đợt, mỗi đợt xong là dùng được:

| Đợt | Nội dung | Tài liệu |
|---|---|---|
| 1. Bố cục | Menu khay mới, cửa sổ Tùy chọn dạng tab, Thông tin, Trợ giúp, hai ngôn ngữ | tài liệu này |
| 2. Chụp | Phím tắt lưu và tải lên toàn màn hình, chụp cả con trỏ, giữ vị trí vùng chọn, bật/tắt thông báo | thiết kế riêng, sau đợt 1 |
| 3. Định dạng | Tải lên bằng PNG hoặc JPEG, chất lượng JPEG | thiết kế riêng, sau đợt 2 |

Đợt 1 không thêm tính năng chụp nào. Nó dựng cái khung để đợt 2 và 3 chỉ việc thêm ô vào tab.

## 2. Mục tiêu, ngoài phạm vi, điều phải tránh

**Mục tiêu**

- Menu khay có năm mục: Chụp màn hình, Tùy chọn..., Thông tin..., Trợ giúp, Thoát.
- Cửa sổ "Cài đặt" thành "Tùy chọn" với ba tab: Thông thường, Phím tắt, Lưu trữ R2. Mọi cài đặt đang có vẫn còn, chỉ đổi chỗ.
- Toàn bộ chữ người dùng nhìn thấy có bản tiếng Việt và bản English, chọn trong tab Thông thường, đổi là có hiệu lực ngay khi bấm OK.
- Cửa sổ Thông tin và cửa sổ Trợ giúp nằm sẵn trong ứng dụng.

**Ngoài phạm vi** (chủ dự án đã chốt)

- Proxy: không làm, ở mọi đợt.
- Mục "Đăng nhập...", ô "Tự động đóng cửa sổ tải lên", ô "Tự động sao chép đường dẫn sau khi tải lên": không làm. shotme không có tài khoản, không có cửa sổ tải lên, và luôn chép link.
- Tab Định dạng và mọi ô của đợt 2, 3: chưa xuất hiện ở đợt 1.
- Ngôn ngữ thứ ba, ngôn ngữ viết từ phải sang trái.
- Xử lý `WM_DPICHANGED` cho cửa sổ Tùy chọn (vẫn là việc để sau như hiện nay).
- Dịch câu lỗi do Windows trả về (ví dụ nội dung `io::Error`): giữ nguyên như hệ điều hành đưa.

**Điều phải tránh** (làm được mà vẫn hỏng)

- Một chuỗi hiện ra sai ngôn ngữ, hoặc thiếu bản dịch.
- Tab rỗng, hoặc ô bấm được mà không có tác dụng.
- Làm hỏng hành vi đã qua rà soát của cửa sổ hiện tại: bắt phím tắt, đăng ký lại phím khi Hủy, Kiểm tra kết nối chạy nền, khóa bí mật không lọt ra lỗi hay `Debug`.
- Ứng dụng gọi thêm bất kỳ địa chỉ mạng nào. Trợ giúp và Thông tin không mở trang web, không có link.
- File cấu hình cũ không đọc được.

## 3. Ràng buộc

- Mọi mục "Global Constraints" của `docs/superpowers/plans/2026-11-16-shotme.md` vẫn áp dụng.
- `shotme-core` không phụ thuộc hệ điều hành; bảng chuỗi nằm ở core nên phải là Rust thuần, không thêm crate.
- Bản release dùng `panic = "abort"`: mã chạy thật không `unwrap`/`expect` trên dữ liệu đầu vào.
- Cửa sổ dựng bằng Win32 thuần, Common Controls v6 (đã có manifest), DPI `PerMonitorV2`.
- Quy trình dự án: mỗi task một nhánh `task/NN-...`, gói task trong `docs/tasks/`, `scripts/check.ps1` phải in `ALL CHECKS PASSED` trước khi gộp, commit kiểm thử trước commit mã.

## 4. Các cách đã cân nhắc

**Đa ngôn ngữ**

| Cách | Được | Mất |
|---|---|---|
| A. Bảng chuỗi viết trong mã Rust, đặt ở `shotme-core` (**đã chọn**) | Trình biên dịch bắt buộc mọi chuỗi có đủ hai ngôn ngữ; không thêm thư viện; exe vẫn một file | Thêm ngôn ngữ phải build lại |
| B. File JSON rời cạnh exe | Thêm ngôn ngữ không cần build | Chuỗi thiếu chỉ lộ lúc chạy; phải xử lý file hỏng hoặc mất; không còn một file duy nhất |
| C. Bảng chuỗi tài nguyên của Windows | Đúng kiểu Windows | Thông báo lỗi nằm ở core, core không được phụ thuộc hệ điều hành: phá kiến trúc hai lớp |

Chủ dự án chọn A: chỉ cần hai ngôn ngữ, và "không thể thiếu bản dịch" quan trọng hơn "thêm ngôn ngữ không cần build".

**Cửa sổ dạng tab**

| Cách | Được | Mất |
|---|---|---|
| A. Điều khiển tab chuẩn (`SysTabControl32`), mỗi tab là một khung con ẩn/hiện (**đã chọn**) | Giống ảnh mẫu; giữ cách dựng bằng mã và giữ nguyên phần bắt phím, Kiểm tra kết nối | Tự lo việc ẩn/hiện và thứ tự Tab |
| B. Property sheet của Windows | Windows lo tab và nút | Viết lại cả cửa sổ sang khuôn tài nguyên, kể cả phần bắt phím đã tốn nhiều công rà soát; nhìn không khác |

Chủ dự án chọn A.

## 5. Thiết kế

Phần ghi **(chủ dự án)** là điều họ đã quyết trong trao đổi. Phần ghi **(đề xuất)** là lựa chọn của người viết thiết kế, chủ dự án đã xem bản tóm tắt nhưng có thể sửa khi đọc tài liệu này.

### 5.1 Menu khay (chủ dự án)

| Tiếng Việt | English | Việc làm |
|---|---|---|
| Chụp màn hình | Take screenshot | như hiện nay |
| Tùy chọn... | Options... | mở cửa sổ Tùy chọn |
| Thông tin... | About... | mở cửa sổ Thông tin |
| Trợ giúp | Help | mở cửa sổ Trợ giúp |
| Thoát | Exit | như hiện nay |

Không có đường kẻ ngăn. Menu được dựng lại mỗi lần mở (đã vậy), nên đổi ngôn ngữ là thấy ngay.

### 5.2 Cửa sổ Tùy chọn

Tiêu đề: "Tùy chọn" / "Options". Bề rộng vùng client giữ 480 DIP như hiện nay. Chiều cao cố định, bằng tab cao nhất (Lưu trữ R2), không đổi khi chuyển tab.

```
┌ Tùy chọn ──────────────────────────────── X ┐
│ (dòng thông báo, chỉ có khi cấu hình lỗi)   │
│ [Thông thường] [Phím tắt] [Lưu trữ R2]      │
│ ┌─────────────────────────────────────────┐ │
│ │ ☐ Khởi động cùng Windows                │ │
│ │ Thư mục lưu  [C:\...\shotme  ] [Chọn…]  │ │
│ │ Ngôn ngữ     [Tiếng Việt          ▾]    │ │
│ └─────────────────────────────────────────┘ │
│ (dòng lỗi màu đỏ)                           │
│                          [  OK  ] [  Hủy  ] │
└─────────────────────────────────────────────┘
```

**Tab Thông thường / General** (chủ dự án): ô tích Khởi động cùng Windows; Thư mục lưu kèm nút Chọn…; hộp chọn Ngôn ngữ với hai mục "Tiếng Việt" và "English" (mỗi mục luôn viết bằng chính ngôn ngữ đó).

**Tab Phím tắt / Hotkeys** (chủ dự án): một dòng "Chụp vùng màn hình" với ô chỉ đọc hiện tổ hợp phím và nút Đổi…. Hành vi bắt phím giữ nguyên từng chi tiết (luật ở quyết định 22, Esc bỏ qua, mất kích hoạt thì thoát chế độ bắt). Đợt 2 thêm hai dòng toàn màn hình kèm ô tích; đợt 1 không vẽ chúng.

**Tab Lưu trữ R2 / R2 storage** (đề xuất; ảnh mẫu không có tab này nhưng các ô phải có chỗ, chủ dự án đã đồng ý): Account ID, Access Key ID, Secret Access Key kèm dòng gợi ý, Tên kho, Địa chỉ công khai, nút Kiểm tra kết nối với dòng kết quả, Hạn của link.

```
│ Account ID          [                      ] │
│ Access Key ID       [                      ] │
│ Secret Access Key   [••••••••              ] │
│                     Đã lưu khóa; để trống nếu không đổi.
│ Tên kho (bucket)    [                      ] │
│ Địa chỉ công khai   [                      ] │
│ Hạn của link        [30 ngày        ▾]       │
│ [Kiểm tra kết nối]  (kết quả)                │
```

**Nút** (đề xuất): "OK" và "Hủy" / "OK" và "Cancel", thay cho "Lưu" và "Hủy". OK là nút mặc định. Enter, Esc, nút X giữ nghĩa hiện nay (Enter = OK trừ khi đang đứng ở một nút bấm; Esc và X = Hủy).

**Hành vi**

- Các ô của cả ba tab tồn tại suốt đời cửa sổ; chuyển tab chỉ ẩn/hiện. Giá trị đã gõ ở tab này không mất khi sang tab khác. Bấm OK lưu cả ba tab một lượt; Hủy bỏ cả ba.
- Bấm OK mà một ô không hợp lệ: cửa sổ tự chuyển sang tab chứa ô đó, đặt con trỏ vào ô, và hiện lỗi ở dòng lỗi (đề xuất, chủ dự án đã đồng ý). Dòng lỗi và dòng thông báo nằm ngoài vùng tab nên tab nào cũng thấy.
- Kiểm tra kết nối đang chạy mà người dùng sang tab khác: vẫn chạy, kết quả hiện khi quay lại tab R2.
- Đang ở chế độ bắt phím mà bấm sang tab khác: thoát chế độ bắt, giữ phím đang chọn (như bấm ra ngoài nút Đổi… hiện nay).
- Bàn phím: Tab/Shift+Tab đi qua dải tab, các ô của tab đang hiện, rồi OK, Hủy. Mũi tên trái/phải đổi tab khi dải tab giữ tiêu điểm. Ctrl+Tab và Ctrl+Shift+Tab đổi tab từ bất cứ đâu. Ô của tab đang ẩn không nhận tiêu điểm.
- Tab mở đầu (đề xuất): Thông thường. Ngoại lệ: khi ứng dụng tự mở cửa sổ vì R2 chưa cấu hình hoặc mất khóa (lần chạy đầu, tải lên báo `needs_settings`, `secret_lost`) thì mở ở tab Lưu trữ R2.
- Mọi câu nhắc người dùng "mở Cài đặt" trong thông báo đổi thành "mở Tùy chọn" / "open Options".
- Cổng chụp (`gate_step`) không đổi: cửa sổ Tùy chọn vẫn là trạng thái `SettingsOpen`, và trong lúc nó mở thì bấm biểu tượng khay chỉ đưa nó ra trước (menu không hiện).
- Vì vậy cửa sổ Tùy chọn có thêm nút "Trợ giúp" / "Help" ở góc dưới bên trái, cùng hàng với OK và Hủy (đề xuất). Nút này mở cửa sổ Trợ giúp (mục 5.5) mà không đóng Tùy chọn, để người đang nhập thông tin R2 lần đầu đọc được các bước chuẩn bị. Bấm nút khi Trợ giúp đã mở thì đưa nó ra trước. Trợ giúp mở theo đường này vẫn dùng được song song: gõ tiếp vào Tùy chọn được, đóng cửa sổ nào trước cũng được.

### 5.3 Ngôn ngữ

- Hai ngôn ngữ: Tiếng Việt, English (chủ dự án).
- Phạm vi dịch (chủ dự án): mọi chữ người dùng nhìn thấy, gồm menu, ba cửa sổ, hộp chọn thư mục (tiêu đề), chữ rê chuột trên thanh công cụ lớp phủ, tiêu đề và nội dung thông báo khay, mọi câu lỗi do shotme tự viết ở cả hai crate (kể cả lý do lỗi lồng bên trong như lỗi clipboard, DPAPI, autostart, lớp phủ). Không dịch: tên phím ("Ctrl", "PrintScreen"), tên riêng ("Account ID", "Secret Access Key", "Cloudflare R2", "shotme"), chú thích mã, tên và thông điệp của kiểm thử.
- Tên hạn link: "1 ngày / 7 ngày / 30 ngày / Không hết hạn" và "1 day / 7 days / 30 days / Never expires".
- Lần chạy đầu, khi chưa có file cấu hình: lấy theo ngôn ngữ giao diện của Windows; là tiếng Việt thì Tiếng Việt, còn lại English (chủ dự án).
- Đã có file cấu hình đọc được nhưng không có trường ngôn ngữ (người đang dùng bản hiện tại), hoặc trường mang giá trị lạ: Tiếng Việt, để nâng cấp không tự đổi ngôn ngữ của họ (đề xuất).
- File cấu hình hỏng hoặc không đọc được: xử lý như chưa có file, tức là theo ngôn ngữ Windows; dòng thông báo lỗi cấu hình trong cửa sổ Tùy chọn hiện bằng ngôn ngữ đó (đề xuất).
- Đổi ngôn ngữ có hiệu lực ngay khi bấm OK, không khởi động lại (chủ dự án). Cửa sổ Tùy chọn đóng khi bấm OK nên không phải vẽ lại nó. Cửa sổ Thông tin hoặc Trợ giúp đang mở thì bị đóng lại khi bấm OK mà ngôn ngữ hoặc phím tắt chụp đã đổi, vì chữ trong đó không còn đúng (đề xuất). Thông báo của lượt tải lên đang chạy dở hiện bằng ngôn ngữ nào cũng được.
- Bấm Hủy: ngôn ngữ không đổi.

**Cấu trúc** (chủ dự án chọn cách; chi tiết là đề xuất)

- `shotme-core` có module mới `i18n`: kiểu `Lang { Vi, En }`, và bảng chuỗi mà mỗi mục bắt buộc có đủ hai bản (ví dụ một `enum` mục chuỗi với `match` vét cạn cho từng ngôn ngữ; chuỗi có tham số là hàm nhận tham số). Không có chuỗi dự phòng lúc chạy: thiếu bản dịch là lỗi biên dịch.
- Core không giữ ngôn ngữ hiện hành. Hàm nào của core sinh chữ thì nhận `Lang` từ bên gọi: `actions::Ctx` thêm trường `lang`; `UploadError::message_vi()` thành `message(lang)`; `normalize_public_base_url` trả lỗi có kiểu để lớp trên dịch (hoặc nhận `Lang`).
- `Config` thêm trường `language: Lang`. File cấu hình thêm `"language": "vi" | "en"`; thiếu hoặc giá trị lạ thì xử lý như hai quy tắc ở trên. `config::load` nhận ngôn ngữ mặc định từ bên gọi giống cách nó nhận `default_save_dir`.
- `shotme-windows` giữ ngôn ngữ hiện hành ở một chỗ đọc được từ mọi luồng (ví dụ biến nguyên tử), đặt lúc khởi động và sau mỗi lần lưu Tùy chọn. Lớp Windows cũng là nơi hỏi ngôn ngữ giao diện của hệ điều hành.
- Chuỗi của lớp Windows cũng nằm trong bảng ở core, để chỉ có một bảng và một cơ chế kiểm tra.

### 5.4 Cửa sổ Thông tin / About (đề xuất, chủ dự án đã đồng ý nội dung)

Cửa sổ nhỏ, không đổi cỡ được: biểu tượng ứng dụng cỡ 48, tên "shotme", "Phiên bản x.y.z" (lấy từ `CARGO_PKG_VERSION`), một dòng mô tả ("Chụp vùng màn hình, lưu vào máy hoặc tải lên kho Cloudflare R2 của chính bạn và chép link." và bản English tương ứng), nút Đóng / Close. Không có link, không gọi mạng.

### 5.5 Cửa sổ Trợ giúp / Help (đề xuất, chủ dự án đã đồng ý dạng cửa sổ)

Cửa sổ khoảng 520×420 DIP, không đổi cỡ được: một vùng chữ chỉ đọc, cuộn được, và nút Đóng / Close. Nội dung là văn bản thuần, gồm các phần:

1. Chụp: bấm phím tắt (hiện đúng tổ hợp đang cấu hình) hoặc chọn "Chụp màn hình" ở menu khay, kéo chọn vùng.
2. Thanh công cụ và phím: tải lên và chép link (Ctrl+D), lưu vào máy (Ctrl+S), chép ảnh (Ctrl+C), hủy (Esc).
3. Tùy chọn: đổi phím tắt, thư mục lưu, ngôn ngữ, hạn link ở đâu.
4. Chuẩn bị Cloudflare R2: bản rút gọn các bước của mục "Chuẩn bị Cloudflare R2" trong `README.md` (tạo kho, bật địa chỉ công khai, tạo khóa quyền Object Read & Write, tạo quy tắc vòng đời cho `d1/`, `d7/`, `d30/`, nhập vào tab Lưu trữ R2, bấm Kiểm tra kết nối).
5. Khi tải lên không được: ảnh luôn được lưu vào thư mục lưu.

Lời văn cụ thể do người làm viết, hai ngôn ngữ, chủ dự án đọc lại khi thử tay.

### 5.6 Thông tin và Trợ giúp với phần còn lại của ứng dụng (đề xuất)

- Hai cửa sổ này không chặn ứng dụng: trong lúc chúng mở, phím tắt chụp, menu khay và cửa sổ Tùy chọn vẫn dùng được bình thường (người đọc Trợ giúp phải thử chụp ngay được). Chúng không tham gia cổng chụp.
- Mỗi loại chỉ có một cửa sổ. Chọn lại mục menu khi cửa sổ đang mở thì đưa nó ra trước.
- Enter và Esc đóng cửa sổ. Thoát ứng dụng thì cả hai bị hủy, không rò handle hay phông.
- Cả hai có biểu tượng shotme trên thanh tiêu đề và hiện giữa màn hình đang có con trỏ, như cửa sổ Tùy chọn.

### 5.7 Kiểm thử

- Core: kiểm thử cho `i18n` (không mục nào rỗng ở cả hai ngôn ngữ; chuỗi có tham số chèn đúng tham số ở cả hai), cho `config` (đọc/ghi trường ngôn ngữ; file cũ không có trường; giá trị lạ), cho `actions` và `upload` ở cả hai ngôn ngữ. Kiểm thử hiện có so chuỗi tiếng Việt thì truyền `Lang::Vi`.
- Lớp Windows: phần logic mới tách thành hàm thuần có kiểm thử, tối thiểu: ô lỗi nào thuộc tab nào; tab mở đầu theo lý do mở; mã ngôn ngữ của Windows ra `Lang` nào. Kiểm thử chạy tay mở/đóng cửa sổ Tùy chọn được cập nhật, thêm kiểm thử chạy tay tương tự cho Thông tin và Trợ giúp.
- `docs/testing/manual-checklist.md` thêm mục cho: menu năm mục, ba tab và điều hướng bàn phím, lỗi nhảy tab, đổi ngôn ngữ qua lại và mọi chỗ có chữ, Thông tin, Trợ giúp, nâng cấp từ file cấu hình cũ.

## 6. Chia task, agent và model (đề xuất, chốt ở bước lập kế hoạch)

Chủ dự án yêu cầu chia model cho hợp với từng task. Số task nối tiếp sổ hiện có.

| Task | Nội dung | Agent làm | Model | Rà soát |
|---|---|---|---|---|
| 15 | `i18n` ở core: `Lang`, bảng chuỗi của core, trường ngôn ngữ trong `Config`, `actions` và `upload` nhận `Lang` | builder | sonnet | reviewer (sonnet) |
| 16 | Lớp Windows dùng bảng chuỗi: chuyển mọi chuỗi viết cứng, ngôn ngữ hiện hành, dò ngôn ngữ Windows. Chưa đổi bố cục | builder | sonnet | reviewer (sonnet) |
| 17 | Cửa sổ Tùy chọn dạng tab: ba tab, điều hướng bàn phím, lỗi nhảy tab, tab mở đầu, hộp chọn ngôn ngữ và áp dụng ngay | specialist | opus | specialist (opus) |
| 18 | Menu năm mục, cửa sổ Thông tin, cửa sổ Trợ giúp | builder | sonnet | reviewer (sonnet) |
| 19 | README, checklist thử tay, tài liệu AI (xem mục 7) | scaffolder | haiku | reviewer (sonnet) |

Lý do: Task 17 đụng vòng lặp thông điệp modal, tiêu điểm, chế độ bắt phím và luồng nền của Kiểm tra kết nối, là loại việc mà ở Task 8 và 10 lượt rà soát tìm ra nhiều điểm nhất, nên giao model mạnh nhất cả làm lẫn rà soát. Task 15, 16, 18 có đặc tả rõ và phần lớn là chuyển đổi có kiểm thử che. Task 19 là việc máy móc.

Thứ tự bắt buộc: 15 → 16 → 17 → 18 → 19.

Các điểm sau cũng là đề xuất:

- Đợt 1 bắt đầu ngay, không chờ nghiệm thu R2 của bản đầu. Lỗi chủ dự án báo từ lượt thử tay bản đầu được sửa chồng lên sau.
- Các task chạy nối tiếp không hỏi lại (quyết định 18). Có một điểm dừng duy nhất: sau Task 18, chủ dự án thử tay theo các mục mới của checklist và đọc lời văn Trợ giúp. Task 19 chạy sau khi họ báo kết quả, để tài liệu ghi đúng trạng thái cuối. Vì vậy các mục checklist của đợt 1 được viết trong Task 17 và 18, không để tới Task 19.
- Gói task 15 đến 19, các dòng mới trong bảng `progress.md` và chỉ dẫn trong `next-task.md` (kế hoạch nào, điểm dừng nào, Task 17 do `specialist` rà soát) được tạo ở bước lập kế hoạch, vì `/next-task` cần chúng trước khi Task 15 chạy.
- Sau mỗi task, cả workspace phải build và qua `scripts/check.ps1`. Task 15 đổi chữ ký hàm của core nên phải sửa tối thiểu các chỗ gọi ở lớp Windows (truyền `Lang::Vi`), Task 16 mới thay bằng ngôn ngữ hiện hành.

## 7. Cập nhật tài liệu AI (chủ dự án yêu cầu)

- `memory-bank/decisions.md`: thêm quyết định cho ba đợt (thay giới hạn "bản đầu chỉ ba tính năng" của quyết định 1 cho các bản sau), không làm Proxy, cách làm đa ngôn ngữ, cách làm tab, hai quy tắc chọn ngôn ngữ mặc định, Thông tin và Trợ giúp không gọi mạng.
- `memory-bank/activeContext.md`, `progress.md`: sau mỗi task như quy trình hiện có; bảng token thêm Task 15 đến 19.
- `memory-bank/productContext.md`, `systemPatterns.md`, `techContext.md`: thêm phần đa ngôn ngữ, cửa sổ dạng tab, hai cửa sổ mới.
- `CLAUDE.md` gốc: thêm đường dẫn tài liệu này và kế hoạch đợt 1. `crates/shotme-windows/CLAUDE.md`: thay dòng "Chuỗi hiển thị bằng tiếng Việt" bằng quy tắc "mọi chuỗi hiển thị lấy từ bảng `i18n`, không viết cứng". `crates/shotme-core/CLAUDE.md`: thêm quy tắc hàm sinh chữ nhận `Lang`.
- `.claude/commands/next-task.md` và `status.md`: trỏ tới kế hoạch đợt 1 nếu chúng đang ghi cứng đường dẫn kế hoạch cũ.
- Gói task `docs/tasks/15-...md` đến `19-...md`, mỗi gói chép sẵn "Global Constraints" (tạo ở bước lập kế hoạch, xem mục 6).
- `scripts/check.ps1`: thêm một kiểm tra chặn chuỗi tiếng Việt viết cứng ngoài bảng `i18n` trong mã chạy thật (không tính chú thích và kiểm thử), nếu làm được bằng một phép tìm đơn giản; nếu không thì ghi thành mục rà soát.

## 8. Để người làm quyết

- Tên và hình dạng cụ thể của bảng chuỗi (một `enum` lớn hay chia nhóm), miễn là thiếu bản dịch thì không biên dịch được.
- Có đổi tên module `settings` và các tên nội bộ (`OpenSettings`, `SettingsOpen`) hay không. Không bắt buộc.
- Kích thước, khoảng cách, thứ tự ô trong mỗi tab, miễn khớp các sơ đồ ở trên.
- Cách dựng khung con của tab và cách bảo đảm ô của tab ẩn không nhận tiêu điểm.
- Lời văn bản English, và lời văn Trợ giúp ở cả hai ngôn ngữ.
- Điều khiển dùng cho vùng chữ của Trợ giúp.
