# shotme đợt 2: phím tắt toàn màn hình, con trỏ chuột, giữ vị trí vùng, bật/tắt thông báo

Ngày: 2026-11-16. Trạng thái: chủ dự án đã duyệt (2026-11-16).

Tài liệu này bổ sung cho spec gốc `2026-11-16-shotme-design.md` và thiết kế đợt 1 `2026-11-16-options-tabs-i18n-design.md` (không sửa hai tài liệu đó). Chỗ nào khác nhau thì tài liệu này thắng, và điểm khác được ghi vào `memory-bank/decisions.md`.

## 1. Ý định

Đợt 1 đã dựng khung: cửa sổ Tùy chọn dạng tab, bảng chuỗi hai ngôn ngữ. Đợt 2 thêm năm tính năng chụp vào khung đó, theo hộp thoại mẫu chủ dự án gửi lúc đầu (quyết định 27):

1. Phím tắt lưu ảnh toàn màn hình.
2. Phím tắt tải lên nhanh toàn màn hình.
3. Bao gồm con trỏ chuột vào ảnh.
4. Giữ vị trí vùng đã chọn.
5. Bật/tắt các thông báo lưu và sao chép.

Đợt 3 (PNG hoặc JPEG) có thiết kế riêng sau đợt này.

## 2. Mục tiêu, ngoài phạm vi, điều phải tránh

**Mục tiêu**

- Tab Phím tắt có ba dòng, mỗi dòng một ô tích bật/tắt, một ô hiện tổ hợp phím và nút Đổi….
- Hai phím mới chụp ngay màn hình đang có con trỏ chuột, không mở lớp phủ: một phím lưu vào máy, một phím tải lên và chép link.
- Tab Thông thường có thêm ba ô tích: thông báo, giữ vị trí vùng, con trỏ chuột.
- Người đang dùng bản hiện tại nâng cấp lên không thấy gì đổi cho tới khi họ tự bật tính năng mới.

**Ngoài phạm vi**

- Mục menu khay cho hai kiểu chụp toàn màn hình: menu giữ năm mục.
- Chụp một cửa sổ, chụp cuộn trang, hẹn giờ chụp.
- Nhớ vị trí vùng qua lần khởi động lại ứng dụng.
- Chọn màn hình khác màn hình có con trỏ; chụp ghép mọi màn hình bằng phím tắt.
- Tắt thông báo lỗi.
- Proxy và các mục ở quyết định 28.
- Tab Định dạng (đợt 3).

**Điều phải tránh** (làm được mà vẫn hỏng)

- Ảnh bị mất: mọi đường lưu dự phòng hiện có (tải lên lỗi thì lưu vào máy; thoát lúc đang tải thì lưu) phải áp dụng cho hai phím mới.
- Một lỗi bị ẩn vì người dùng đã tắt thông báo.
- Hai lớp phủ, hoặc một lượt chụp toàn màn hình chen vào lúc lớp phủ hay cửa sổ Tùy chọn đang mở.
- Phím tắt cũ của người dùng mất hoặc đổi sau khi nâng cấp; file cấu hình cũ không đọc được.
- Một dòng phím tắt đã bỏ tích mà vẫn chiếm phím của hệ thống.
- Làm hỏng hành vi đã rà soát của lớp phủ (Task 8) và của chế độ bắt phím (Task 10, 17).
- Chuỗi mới viết cứng, hoặc thiếu một trong hai ngôn ngữ.
- Ứng dụng gọi thêm địa chỉ mạng nào.

## 3. Ràng buộc

- Mọi mục "Global Constraints" của `docs/superpowers/plans/2026-11-16-options-tabs-i18n.md` vẫn áp dụng.
- `shotme-core` không phụ thuộc hệ điều hành. Không thêm crate.
- Bản release dùng `panic = "abort"`: mã chạy thật không `unwrap`/`expect` trên dữ liệu đầu vào.
- Mọi tọa độ là pixel vật lý; DPI `PerMonitorV2`.
- Luật tổ hợp phím của quyết định 22 áp dụng cho cả ba dòng.
- Quy trình dự án: mỗi task một nhánh `task/NN-...`, gói task trong `docs/tasks/`, `scripts/check.ps1` in `ALL CHECKS PASSED` trước khi gộp, commit kiểm thử trước commit mã.

## 4. Các cách đã cân nhắc

**"Toàn màn hình" trên máy nhiều màn hình**

| Cách | Được | Mất |
|---|---|---|
| A. Màn hình đang có con trỏ chuột (**đã chọn**) | Ảnh gọn, đúng thứ đang nhìn | Muốn chụp màn hình khác phải đưa chuột sang |
| B. Mọi màn hình ghép lại | Dễ làm nhất | Ảnh rất rộng, có vùng đen khi màn hình khác cỡ, tải lên nặng |
| C. Màn hình chứa cửa sổ đang dùng | Không phụ thuộc chuột | Khó đoán khi cửa sổ vắt qua hai màn hình hoặc không có cửa sổ nào |

Chủ dự án chọn A.

**Con trỏ chuột khi chụp vùng**

| Cách | Được | Mất |
|---|---|---|
| A. Vẽ con trỏ vào ảnh ngay lúc chụp (**đã chọn**) | Thấy trước con trỏ có lọt vào vùng chọn không; ít mã; lớp phủ gần như không đổi | Trên lớp phủ có một con trỏ "đóng băng" cạnh con trỏ thật |
| B. Giữ con trỏ thành lớp riêng, ghép khi xuất ảnh | Lớp phủ sạch | Không thấy trước; sửa sâu vào lớp phủ, phần đắt và dễ lỗi nhất dự án |

Chủ dự án chọn A.

**Lấy ảnh một màn hình** (người viết thiết kế quyết)

| Cách | Được | Mất |
|---|---|---|
| A. Chụp cả màn hình ảo bằng `capture_virtual_screen` rồi cắt lấy một màn hình (**đã chọn**) | Một đường chụp duy nhất, phần DPI và nhiều màn hình đã rà soát; con trỏ vẽ ở một chỗ | Chụp thừa rồi bỏ khi có nhiều màn hình |
| B. Viết hàm chụp riêng một màn hình | Ít bộ nhớ hơn | Hai đường chụp phải giữ cho giống nhau |

**Tắt thông báo** (người viết thiết kế quyết)

| Cách | Được | Mất |
|---|---|---|
| A. Thêm loại thông báo "thành công", lọc ở một chỗ trước khi hiện (**đã chọn**) | Không thể lỡ ẩn một lỗi; core không cần biết cài đặt | Thêm một biến thể `NoticeKind` |
| B. Truyền cờ vào từng chỗ gọi `notify` | Không đổi kiểu | 23 chỗ gọi; dễ sót hoặc ẩn nhầm lỗi |

## 5. Thiết kế

**(chủ dự án)** là điều họ đã quyết. **(đề xuất)** là lựa chọn của người viết thiết kế; chủ dự án đã xem bản tóm tắt trong trao đổi và không sửa, nhưng có thể sửa khi đọc tài liệu này.

### 5.1 Tab Phím tắt

```
☑ Chụp vùng màn hình             [PrintScreen      ] [Đổi…]
☐ Lưu ảnh (toàn màn hình)        [Shift+PrintScreen] [Đổi…]
☐ Tải lên nhanh (toàn màn hình)  [Ctrl+PrintScreen ] [Đổi…]
```

| Dòng | Tiếng Việt | English | Mặc định |
|---|---|---|---|
| 1 | Chụp vùng màn hình | Capture region | bật, `PrintScreen` (hoặc phím người dùng đang có) |
| 2 | Lưu ảnh (toàn màn hình) | Save full screen | tắt, `Shift+PrintScreen` |
| 3 | Tải lên nhanh (toàn màn hình) | Quick upload (full screen) | tắt, `Ctrl+PrintScreen` |

Bản English là gợi ý, người làm được chỉnh lời.

- (đề xuất) Bỏ tích một dòng: phím của dòng đó không được đăng ký với Windows; ô phím và nút Đổi… của dòng bị mờ, tổ hợp vẫn được nhớ để tích lại là có. Cả ba dòng đều bỏ tích được; khi dòng 1 tắt thì chỉ chụp vùng được qua menu khay.
- Bấm Đổi… ở dòng nào thì vào chế độ bắt phím cho dòng đó, đúng hành vi hiện có (luật quyết định 22, Esc bỏ qua, mất kích hoạt hoặc đổi tab thì thoát). Mỗi lúc chỉ một dòng ở chế độ bắt; bấm Đổi… của dòng khác thì dòng trước thoát, giữ phím cũ của nó.
- (đề xuất) Bấm OK:
  - Hai dòng **đang bật** có cùng tổ hợp: không lưu, chuyển sang tab Phím tắt, đặt tiêu điểm vào nút Đổi… của dòng đứng sau, dòng lỗi nêu tên hai dòng. Dòng đang tắt không tính là trùng.
  - Một dòng đang bật không đăng ký được vì ứng dụng khác đang giữ phím: không lưu, cửa sổ vẫn mở, dòng lỗi nêu tên dòng và tổ hợp, tiêu điểm vào nút Đổi… của dòng đó. Giữ cách làm hiện nay của `settings::save` (cờ `hk_unregistered`): sau lần OK thất bại, cả ba phím ở trạng thái chưa đăng ký cho tới khi OK thành công hoặc Hủy. Điều này vô hại vì trong lúc cửa sổ Tùy chọn mở, phím tắt chỉ đưa cửa sổ ra trước.
  - Bấm Hủy, hoặc OK mà ghi file thất bại: ba phím được đăng ký lại theo cấu hình trước khi mở cửa sổ (như hiện nay với một phím).
- Lúc khởi động: các dòng đang bật mà không đăng ký được được gộp vào **một** thông báo lỗi liệt kê các tổ hợp bị chiếm (chuỗi mới, có dạng một tổ hợp và nhiều tổ hợp), vì hàng đợi của khay chỉ hiện thông báo lỗi mới nhất khi biểu tượng chưa sẵn sàng (`tray::take_notice_to_show`) và hành vi đó không đổi. Các dòng khác vẫn hoạt động. Cùng cách gộp này dùng khi đăng ký lại sau Hủy hoặc sau ghi file thất bại.
- Cửa sổ Thông tin và Trợ giúp tự đóng khi bất kỳ dòng nào đổi (bật/tắt hoặc tổ hợp), mở rộng quy tắc của quyết định 31.

### 5.2 Hai phím toàn màn hình

Khi bấm một trong hai phím (đề xuất, trừ chỗ ghi khác):

1. Chụp ngay, không lớp phủ, không hỏi gì.
2. (chủ dự án) Ảnh là màn hình đang chứa con trỏ chuột lúc chụp. Nếu vị trí con trỏ không nằm trong màn hình nào (hiếm, lúc cấu hình màn hình đang đổi) thì lấy màn hình gần con trỏ nhất.
3. **Lưu ảnh:** lưu vào thư mục lưu với cách đặt tên hiện có, thông báo "Đã lưu ảnh" như bấm Ctrl+S ở lớp phủ.
4. **Tải lên nhanh:** tải lên và chép link, đúng đường đi của Ctrl+D ở lớp phủ: chạy nền, R2 chưa cấu hình hoặc tải lỗi thì lưu vào máy và báo lỗi, thiếu cấu hình thì mở Tùy chọn ở tab Lưu trữ R2, thoát ứng dụng lúc đang tải thì ảnh được lưu (quyết định 25).

Với cổng chụp (`gate_step`):

- Hai phím đi qua cùng cổng với chụp vùng, là cùng sự kiện "phím tắt" với một loại chụp đi kèm; trong lúc lượt chụp đang chạy cổng ở trạng thái `Capturing` như chụp vùng.
- Lớp phủ đang mở, hoặc một lượt chụp khác đang chạy: phím bị bỏ qua.
- Cửa sổ Tùy chọn đang mở: không chụp, đưa cửa sổ Tùy chọn ra trước (như phím chụp vùng hiện nay).
- Menu khay đang mở: đóng menu, chờ 250 ms rồi chụp, để menu không lọt vào ảnh (như phím chụp vùng hiện nay, quyết định 21). Loại chụp được nhớ qua lượt chờ.
- Thông tin và Trợ giúp đang mở: chụp bình thường, hai cửa sổ đó có trong ảnh nếu chúng nằm trên màn hình được chụp.
- Tải lên chạy nền nên bấm phím lần nữa ngay sau đó là một lượt chụp mới; nhiều lượt tải lên cùng lúc đã được hỗ trợ.

Không chụp được màn hình (màn hình khóa, UAC): thông báo lỗi hiện có.

### 5.3 Tab Thông thường

```
☐ Khởi động cùng Windows
☑ Hiện các thông báo lưu và sao chép
☐ Giữ vị trí vùng đã chọn
☐ Bao gồm con trỏ chuột vào ảnh chụp màn hình
Thư mục lưu  [C:\...\shotme  ] [Chọn…]
Ngôn ngữ     [Tiếng Việt          ▾]
```

| Ô | English | Mặc định |
|---|---|---|
| Hiện các thông báo lưu và sao chép | Show save and copy notifications | bật |
| Giữ vị trí vùng đã chọn | Keep the selected area position | tắt |
| Bao gồm con trỏ chuột vào ảnh chụp màn hình | Include the mouse cursor in screenshots | tắt |

Chữ tiếng Việt của hai ô đầu và ô thứ ba lấy theo hộp thoại mẫu; thứ tự ô do người làm xếp, miễn ba ô tích mới nằm liền nhau. Chiều cao cửa sổ vẫn là chiều cao của tab cao nhất; nếu tab Thông thường vượt tab Lưu trữ R2 thì cửa sổ cao lên cho vừa.

### 5.4 Bật/tắt thông báo (đề xuất)

- Thông báo có ba loại: thành công, thông tin, lỗi. "Thành công" là đúng ba thông báo hiện có: Đã lưu ảnh, Đã chép ảnh, Đã chép link. Mọi thông báo khác giữ loại hiện tại.
- Ô bỏ tích: thông báo loại thành công không hiện. Loại lỗi và thông tin luôn hiện.
- Hệ quả đã nói với chủ dự án: khi tắt, "Tải lên nhanh" thành công không báo gì; link nằm trong clipboard.
- Trường hợp tải lên được nhưng không chép được link là thông báo lỗi (đã vậy), nên vẫn hiện kèm link.
- Việc lọc nằm ở một chỗ của lớp Windows, ngay trước khi thông báo vào hàng đợi của khay. Core vẫn gọi `notify` như cũ và không biết tới cài đặt này.
- Cài đặt có hiệu lực ngay khi bấm OK. Lượt tải lên đang chạy dở dùng cài đặt mới hay cũ đều được.
- Biểu tượng của thông báo thành công giữ như hiện nay (biểu tượng thông tin).

### 5.5 Giữ vị trí vùng đã chọn (đề xuất)

- Ô tích bật: lớp phủ của lượt chụp vùng mở ra với khung của lần trước đã chọn sẵn, đúng vị trí và kích thước trên màn hình, kèm tay nắm, nhãn kích thước và thanh công cụ. Ctrl+D, Ctrl+S, Ctrl+C dùng được ngay. Người dùng vẫn chỉnh khung, kéo di chuyển, hoặc kéo bên ngoài để chọn lại như thường.
- Khung được nhớ khi lượt chụp vùng kết thúc bằng tải lên, lưu hoặc chép. Hủy (Esc) không đổi khung đang nhớ.
- Khung được nhớ theo tọa độ màn hình (không phải tọa độ ảnh), vì gốc của màn hình ảo có thể đổi khi người dùng xếp lại màn hình.
- Chỉ nhớ trong bộ nhớ của ứng dụng: thoát ứng dụng là quên. Khung vẫn được ghi nhớ cả khi ô đang tắt, để bật ô lên là có ngay khung của lượt chụp gần nhất trong phiên.
- Lúc mở lớp phủ, khung đang nhớ chỉ được dùng khi nó nằm trọn trong ảnh màn hình ảo vừa chụp **và** chạm ít nhất một màn hình. Không thỏa (đã rút màn hình, đổi độ phân giải) thì lớp phủ mở trống như thường; khung đang nhớ không bị xóa.
- Hai phím toàn màn hình không đọc và không ghi khung này.

### 5.6 Con trỏ chuột trong ảnh (chủ dự án chọn cách; chi tiết là đề xuất)

- Ô tích bật: ngay sau khi chép màn hình, con trỏ đang hiện được vẽ vào ảnh ở đúng vị trí của nó (đã trừ điểm nóng của con trỏ), đúng hình dạng lúc đó (mũi tên, bàn tay, chữ I...), ở kích thước mà Windows trả về cho con trỏ đó.
- Giới hạn chấp nhận: nếu người dùng phóng to con trỏ trong Cài đặt Windows (Trợ năng) mà cách vẽ chuẩn chỉ cho cỡ gốc, con trỏ trong ảnh được phép nhỏ hơn con trỏ trên màn hình. Không tự co giãn con trỏ; ghi giới hạn vào checklist và Trợ giúp không cần nhắc.
- Áp dụng cho cả ba kiểu chụp. Với chụp vùng, lớp phủ hiện ảnh đã có con trỏ đóng băng; con trỏ vào ảnh xuất ra nếu nó nằm trong vùng chọn.
- Windows báo con trỏ đang ẩn (ví dụ đang gõ trong một số ứng dụng, đang xem phim toàn màn hình), hoặc không lấy được con trỏ: chụp bình thường, không có con trỏ, không báo lỗi.
- Chụp từ menu khay: con trỏ ở vị trí lúc chụp (sau 250 ms chờ), thường là gần khay.
- Ô tích tắt: ảnh giống hệt hiện nay.

### 5.7 File cấu hình (đề xuất)

- Thêm vào file: trạng thái bật của dòng 1; trạng thái bật và tổ hợp của dòng 2, dòng 3; ba ô tích mới. Tên trường do người làm đặt, viết bằng tiếng Anh kiểu `snake_case` như các trường đang có.
- Trường `hotkey` hiện có giữ nguyên tên và nghĩa (tổ hợp của dòng 1), để file của bản đang dùng đọc được và bản cũ vẫn đọc được file mới.
- File không có các trường mới (mọi người đang dùng): dòng 1 bật với phím đang có; dòng 2, 3 tắt với tổ hợp mặc định; thông báo bật; giữ vị trí tắt; con trỏ tắt. Tức là không gì đổi.
- Trường mới có kiểu sai làm cả file thành "file hỏng", như mọi trường hiện nay (`#[serde(default)]` chỉ bù trường thiếu). Không thêm cách đọc khoan dung riêng.
- Tổ hợp phím đọc từ file không được kiểm theo quyết định 22, cho cả ba dòng, như trường `hotkey` hiện nay: luật đó là luật của chế độ bắt phím ở lớp Windows và ở lại đó. Tổ hợp Windows không nhận thì thành "không đăng ký được" lúc khởi động.
- File mà hai dòng đang bật trùng tổ hợp (sửa tay): lúc khởi động dòng đứng sau không đăng ký được và được báo như một phím bị chiếm.

### 5.8 Trợ giúp và tài liệu

- Lời văn Trợ giúp (hai ngôn ngữ) thêm: hai phím toàn màn hình và cách bật chúng, ba ô tích mới. Trợ giúp nêu tổ hợp thật của từng dòng đang bật; dòng đang tắt được nêu là "chưa bật" kèm chỗ bật.
- README, `docs/testing/manual-checklist.md` (mục "Đợt 2"), `memory-bank/productContext.md`, `systemPatterns.md`, `techContext.md` cập nhật theo.
- `memory-bank/decisions.md` thêm quyết định của đợt này: toàn màn hình là màn hình có con trỏ; con trỏ vẽ vào ảnh lúc chụp; thông báo lỗi luôn hiện; vị trí vùng chỉ nhớ trong phiên; ba dòng phím tắt và cách nâng cấp file; chia task.

### 5.9 Kiểm thử

- Core, tự động:
  - cấu hình: đọc/ghi các trường mới; file cũ không có trường mới cho đúng mặc định ở 5.7; trường `hotkey` cũ vẫn là dòng 1;
  - kiểm tra trùng tổ hợp giữa các dòng đang bật (dòng tắt không tính);
  - chọn màn hình theo điểm, kể cả điểm ngoài mọi màn hình;
  - khung nhớ có dùng được không (trọn trong ảnh, chạm một màn hình), và đổi tọa độ màn hình sang tọa độ ảnh;
  - vùng chọn khởi tạo sẵn một khung cho đúng trạng thái "đã chọn";
  - thông báo thành công mang đúng loại mới; lỗi vẫn là lỗi; hàm lọc: tắt thì chỉ loại thành công bị bỏ;
  - chuỗi mới có đủ hai ngôn ngữ (cơ chế hiện có).
- Lớp Windows, tự động: các hàm thuần mới (ô lỗi nào thuộc tab nào cho hai dòng mới; cổng chụp với loại chụp mới; điều kiện đóng Thông tin/Trợ giúp).
- Lớp Windows, chạy tay có sẵn khung `#[ignore]`: chụp có con trỏ không rò handle GDI.
- `docs/testing/manual-checklist.md`, mục "Đợt 2": ba dòng phím tắt (bật/tắt, đổi, trùng, bị chiếm, Hủy); hai phím toàn màn hình trên một và hai màn hình, DPI khác nhau; tải lên nhanh khi chưa có R2 và khi mất mạng; phím bấm lúc lớp phủ, Tùy chọn, menu đang mở; con trỏ ở ba kiểu chụp và với con trỏ dạng chữ I, bàn tay, con trỏ phóng to trong Cài đặt Windows; giữ vị trí vùng qua nhiều lượt, sau khi rút màn hình; tắt thông báo rồi thử ba hành động thành công và một lỗi; nâng cấp từ file cấu hình của đợt 1.

## 6. Chia task, agent và model (đề xuất, chốt ở bước lập kế hoạch)

Số task nối tiếp sổ hiện có.

| Task | Nội dung | Agent làm | Model | Rà soát |
|---|---|---|---|---|
| 20 | Core: trường cấu hình mới và nâng cấp file, kiểm tra trùng tổ hợp, loại thông báo "thành công", chuỗi mới, các hàm hình học (chọn màn hình, khung nhớ, vùng chọn khởi tạo sẵn) | builder | sonnet | reviewer (sonnet) |
| 21 | Tùy chọn: ba dòng phím tắt với ô tích và bắt phím từng dòng, đăng ký ba phím (OK, Hủy, lỗi, khởi động), ba ô tích ở tab Thông thường, lọc thông báo, lời văn Trợ giúp (chốt ở kế hoạch) | specialist | opus | specialist (opus) |
| 22 | Chụp: con trỏ vào ảnh, hai luồng toàn màn hình qua cổng chụp, lớp phủ mở với khung nhớ | specialist | opus | specialist (opus) |
| 23 | Lời văn Trợ giúp, README, checklist thử tay, tài liệu AI | scaffolder | haiku | reviewer (sonnet) |

Lý do: Task 21 đụng chế độ bắt phím và vòng lặp của cửa sổ Tùy chọn, Task 22 đụng GDI, DPI, cổng chụp và lớp phủ; đây là hai loại việc mà các lượt rà soát trước tìm ra nhiều điểm nhất. Task 20 là logic thuần có kiểm thử. Task 23 là việc máy móc; riêng lời văn Trợ giúp nằm trong bảng chuỗi ở core nên là thay đổi mã có kiểm thử, người lập kế hoạch có thể chuyển phần đó sang Task 21 nếu thấy hợp hơn.

Thứ tự bắt buộc: 20 → 21 → 22 → 23. Sau mỗi task cả workspace build được và qua `scripts/check.ps1`; sau Task 21 các ô và phím mới đã lưu được nhưng chưa có tác dụng chụp, điều này chấp nhận được vì không phát hành giữa chừng.

Các task chạy nối tiếp không hỏi lại (quyết định 18). Một điểm dừng: sau Task 22, chủ dự án thử tay theo mục "Đợt 2" của checklist; Task 23 chạy sau khi họ báo kết quả. Vì vậy mục checklist "Đợt 2" được viết trong Task 21 và 22.

## 7. Để người làm quyết

- Tên trường trong file cấu hình và hình dạng kiểu trong `Config` (ví dụ ba trường riêng hay một mảng ba phần tử), miễn đúng mục 5.7.
- Mã định danh của ba phím với Windows, và cách cổng chụp mang theo loại chụp.
- Cách vẽ con trỏ (ví dụ `GetCursorInfo` + `DrawIconEx` vào DC bộ nhớ trước khi đọc pixel), miễn đúng vị trí, đúng hình, không rò handle.
- Cách lớp phủ nhận khung ban đầu và cách vùng chọn ở core được khởi tạo sẵn.
- Chỗ đặt bộ lọc thông báo ở lớp Windows.
- Kích thước, khoảng cách các ô; lời văn English; lời văn Trợ giúp.
