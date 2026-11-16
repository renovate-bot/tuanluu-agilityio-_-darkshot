# shotme đợt 3: định dạng ảnh tải lên (PNG hoặc JPEG)

Ngày: 2026-11-16. Trạng thái: chủ dự án đã duyệt (2026-11-16).

Tài liệu này bổ sung cho spec gốc `2026-11-16-shotme-design.md`, thiết kế đợt 1 `2026-11-16-options-tabs-i18n-design.md` và thiết kế đợt 2 `2026-11-16-capture-batch2-design.md` (không sửa ba tài liệu đó). Chỗ nào khác nhau thì tài liệu này thắng, và điểm khác được ghi vào `memory-bank/decisions.md`.

Trong tài liệu, "chủ dự án quyết" là điều họ đã chọn trong buổi trao đổi; "đề xuất" là điều phiên điều phối tự chọn và đã nói với họ.

## 1. Ý định

Hôm nay mọi ảnh shotme tạo ra đều là PNG. PNG giữ nguyên từng điểm ảnh nhưng nặng, nhất là ảnh cả màn hình có hình nền hay ảnh chụp; từ đợt 2 đã có phím tải lên nhanh toàn màn hình nên ảnh nặng xuất hiện thường hơn. Đợt 3 cho chủ dự án chọn tải lên bằng JPEG để link nhẹ, mở nhanh, tốn ít dung lượng kho. Đây là đợt cuối trong ba đợt của quyết định 27, theo hộp thoại mẫu chủ dự án gửi lúc đầu ("Định dạng ảnh tải lên").

## 2. Mục tiêu, ngoài phạm vi, điều phải tránh

**Mục tiêu**

- Cửa sổ Tùy chọn có tab thứ tư "Định dạng": chọn PNG hoặc JPEG cho ảnh tải lên, và mức chất lượng JPEG.
- Khi chọn JPEG, ảnh tải lên là file JPEG, tên trên kho và link kết thúc bằng `.jpg`.
- Người đang dùng bản hiện tại nâng cấp lên không thấy gì đổi cho tới khi tự chọn JPEG.

**Ngoài phạm vi**

- JPEG cho ảnh lưu vào máy (chủ dự án quyết: không). Mọi ảnh ghi vào thư mục lưu luôn là PNG.
- Định dạng khác (WebP, AVIF, GIF), JPEG kiểu progressive, tự chọn định dạng theo nội dung ảnh, giới hạn dung lượng, đổi cỡ ảnh.
- Xem trước chất lượng, ước lượng dung lượng trong cửa sổ Tùy chọn.
- Chuyển đổi các ảnh đã tải lên trước đó.
- Proxy và các mục ở quyết định 28.

**Điều phải tránh** (làm được mà vẫn hỏng)

- Ảnh bị mất: nén JPEG lỗi hay tải lên lỗi thì ảnh vẫn phải được lưu vào máy như hôm nay.
- Một file JPEG nằm trong thư mục lưu, hoặc một file trên kho có đuôi không khớp nội dung (`.png` chứa JPEG hay ngược lại) hay kiểu nội dung gửi kèm không khớp.
- Ảnh JPEG mở được ở trình duyệt này mà hỏng ở trình xem khác.
- Chữ màu trong ảnh JPEG bị nhòe vì giảm độ phân giải màu.
- Người chọn PNG nhận ảnh khác trước dù chỉ một byte.
- File cấu hình cũ không đọc được, hoặc một trường cũ đổi nghĩa.
- Tab mới làm hỏng hành vi đã rà soát của cửa sổ Tùy chọn: chuyển tab bằng chuột và bàn phím, chế độ bắt phím, vị trí lỗi nhảy đúng tab, OK và Hủy.
- Chuỗi mới viết cứng, hoặc thiếu một trong hai ngôn ngữ.
- Ứng dụng gọi thêm địa chỉ mạng nào, hoặc thư viện mới kéo theo thư viện khác.

## 3. Ràng buộc

- Mọi mục "Global Constraints" của `docs/superpowers/plans/2026-11-16-capture-batch2.md` vẫn áp dụng, trừ câu "Không thêm crate mới", nay là: **chỉ thêm đúng một crate `jpeg-encoder`** vào `shotme-core` (mục 4). Không thêm crate nào khác, kể cả `dev-dependencies`.
- `shotme-core` không phụ thuộc hệ điều hành; bước kiểm tra phụ thuộc trong `scripts/check.ps1` giữ nguyên.
- Bản release dùng `panic = "abort"`: mã chạy thật không `unwrap`/`expect` trên dữ liệu đầu vào.
- Quy trình dự án: mỗi task một nhánh `task/NN-...`, gói task trong `docs/tasks/`, `scripts/check.ps1` in `ALL CHECKS PASSED` trước khi gộp, commit kiểm thử trước commit mã.

## 4. Các cách đã cân nhắc

**JPEG áp dụng cho ảnh nào**

- A. Chỉ ảnh tải lên. Bản trong máy luôn nét nguyên gốc; link thì nhẹ. Đúng với hộp thoại mẫu.
- B. Cả ảnh tải lên lẫn ảnh lưu vào máy. Thư mục lưu cũng nhẹ, nhưng không còn bản gốc nào.
- C. Hai cài đặt riêng. Linh hoạt nhất, thêm ô và trường hợp phải thử.

Chủ dự án chọn **A**.

**Lấy bộ nén JPEG ở đâu**

- A. Thêm crate `jpeg-encoder` (0.7.1): Rust thuần, không phụ thuộc crate nào khác, giấy phép `(MIT OR Apache-2.0) AND IJG`, chỉ tính toán trong bộ nhớ; cho chọn chất lượng và kiểu lấy mẫu màu. Phá luật "không thêm crate" một lần.
- B. Tự viết bộ nén trong `shotme-core`. Giữ luật, nhưng vài trăm dòng thuật toán phải tự viết, tự kiểm; lỗi kiểu "hỏng trên một số trình xem" khó bắt.
- C. Bộ nén của Windows (GDI+ hoặc WIC). Không thêm crate, nhưng việc nén phải rời core sang lớp Windows, đổi cấu trúc luồng tải lên đã rà soát; đường đơn giản không cho chọn kiểu lấy mẫu màu.

Chủ dự án chọn **A**. Lý do của phiên điều phối: luật "không thêm crate" nhằm giữ ứng dụng nhỏ và không có đường đưa dữ liệu ra ngoài; một crate chỉ làm phép tính, không phụ thuộc, không đi ngược mục đích đó.

**Ô chọn chất lượng** (đề xuất, chủ dự án đã đồng ý): thanh trượt 50 đến 100 có số bên cạnh, thay vì hộp chọn vài mức. Dưới 50 chữ trong ảnh chụp màn hình vỡ rõ nên không cho chọn.

## 5. Thiết kế

### 5.1 Tab "Định dạng" (chủ dự án đã đồng ý bản phác)

Thứ tự tab: Thông thường, Phím tắt, **Định dạng**, Lưu trữ R2.

```
[Thông thường] [Phím tắt] [Định dạng] [Lưu trữ R2]

Định dạng ảnh tải lên
  (•) PNG    nét nguyên gốc, file nặng hơn
  ( ) JPEG   file nhẹ hơn, giảm chút chất lượng

Chất lượng JPEG   [=========|===]  90

Ảnh lưu vào máy luôn là PNG.
```

- Hai nút chọn một (radio), mặc định PNG. Lời giải thích ngắn nằm ngay trong nhãn hoặc cạnh nút.
- Thanh trượt chất lượng: số nguyên từ 50 đến 100, mặc định 90. Số hiện bên cạnh và đổi theo ngay khi kéo. Phím mũi tên đổi 1, Page Up/Page Down đổi 10, Home/End về hai đầu.
- Khi đang chọn PNG: thanh trượt, nhãn và số bị mờ, không nhận phím; giá trị vẫn được giữ và vẫn được lưu khi OK. Chọn lại JPEG thì thanh sáng lên với giá trị cũ.
- Dòng ghi chú "Ảnh lưu vào máy luôn là PNG." (đề xuất) để người dùng không tưởng Ctrl+S cũng đổi.
- Như mọi ô khác: chỉ có tác dụng sau khi bấm OK; Hủy hay đóng cửa sổ thì bỏ. Tab này không có ô nào có thể sai, nên không sinh lỗi kiểm tra.
- Chuyển tab bằng chuột, Ctrl+Tab và Ctrl+Shift+Tab (các cách đang có; không thêm phím mới) đi qua đủ bốn tab theo thứ tự trên. Ctrl+Tab vẫn chuyển tab khi tiêu điểm đang ở thanh trượt. Thứ tự Tab trong tab: nút chọn (một điểm dừng, mũi tên đổi giữa hai nút như radio chuẩn của Windows), rồi thanh trượt.
- Dải tab bốn mục phải hiện đủ chữ ở cả hai ngôn ngữ và mọi DPI; nếu chưa đủ thì bề rộng cửa sổ phải nới theo (bề rộng hiện là 530 DIP; `Layout` hôm nay chưa đo dải tab nên phép đo này là mã mới). Không tab nào được xuống dòng thứ hai.
- Trình đọc màn hình đọc được tên thanh trượt (nhãn đứng ngay trước nó trong thứ tự ô).
- Chế độ bắt phím của tab Phím tắt không đổi. Chuyển sang tab Định dạng lúc đang bắt phím thì cư xử như chuyển sang tab khác hôm nay.

### 5.2 Tải lên

Khi một lượt tải lên bắt đầu (Ctrl+D ở lớp phủ, nút tải lên của lớp phủ, hoặc phím tải lên nhanh toàn màn hình), ứng dụng đọc định dạng và chất lượng từ cấu hình đang có hiệu lực:

| | PNG (mặc định) | JPEG |
|---|---|---|
| Dữ liệu gửi đi | như hôm nay, không đổi một byte | JPEG nén ở mức chất lượng đã chọn |
| Tên trên kho | `{d1\|d7\|d30\|keep}/{tên ngẫu nhiên}.png` | `{d1\|d7\|d30\|keep}/{tên ngẫu nhiên}.jpg` |
| Kiểu nội dung gửi kèm | `image/png` | `image/jpeg` |
| Link chép vào clipboard | kết thúc bằng `.png` | kết thúc bằng `.jpg` |

- Tên ngẫu nhiên, thư mục theo thời hạn, chữ ký, địa chỉ gọi, thông báo thành công và thất bại: không đổi.
- Lượt tải lên đang chạy dở khi người dùng đổi cài đặt giữ định dạng lúc nó bắt đầu (đề xuất).
- Quy tắc tự xóa trên R2 dựa vào thư mục nên không phải đổi gì trên Cloudflare.
- "Kiểm tra kết nối" ở tab Lưu trữ R2 không đổi (vẫn dùng file văn bản thử).

### 5.3 File JPEG (đề xuất)

- JPEG baseline (không progressive), 8 bit, ba kênh màu, lấy mẫu màu 4:4:4 (không giảm độ phân giải màu) ở mọi mức chất lượng. Kênh alpha bị bỏ, như PNG hôm nay.
- Không ghi siêu dữ liệu (EXIF, bình luận, tên phần mềm). Mật độ điểm ảnh để mặc định của bộ nén.
- Mức 100 vẫn là nén có mất mát; lời văn không được gọi nó là "nguyên gốc".
- Nén xảy ra trong `shotme-core`, từ ảnh gốc trong bộ nhớ (không nén lại từ PNG).

### 5.4 Khi có lỗi

- **Nén JPEG thất bại** (ảnh không hợp lệ, hoặc một cạnh dài hơn 65.535 pixel là giới hạn của JPEG): xử lý y như "mã hóa PNG thất bại" hôm nay ở luồng tải lên, tức là lưu ảnh vào máy bằng PNG và hiện thông báo lỗi tải lên kèm lý do. Không tự chuyển sang tải lên PNG (đề xuất: một lượt tải lên chỉ có một kết quả dễ đoán).
- **Tải lên thất bại** (mạng, quyền, đồng hồ lệch, chưa cấu hình): không đổi; bản lưu dự phòng là PNG nén từ ảnh gốc.
- **Thoát lúc đang tải dở, hoặc không khởi động được luồng tải lên:** không đổi; bản lưu là PNG.
- Ctrl+S, Ctrl+C và phím lưu toàn màn hình không đọc cài đặt này.

### 5.5 File cấu hình (đề xuất)

- Thêm hai trường: định dạng tải lên (hai giá trị, PNG hoặc JPEG) và chất lượng JPEG (số nguyên). File không có hai trường này thì là PNG và 90. Mọi trường cũ giữ tên và nghĩa.
- Chất lượng trong file nằm ngoài 50 đến 100 (do sửa tay): kéo về giới hạn gần nhất khi đọc, không coi là file hỏng. Điều này đúng với mọi số nguyên, kể cả số âm hay lớn hơn 255 (ví dụ -1 thành 50, 300 thành 100): đọc bằng kiểu số nguyên rộng rồi mới kẹp. Số có phần thập phân là sai kiểu.
- Sai kiểu dữ liệu (ví dụ chất lượng là chuỗi), hoặc tên định dạng không nhận ra: theo đúng cách các trường cùng loại hiện có đang xử lý, không đặt luật mới. Hôm nay sai kiểu là file hỏng (`Corrupt`), còn chuỗi không nhận ra của `expiry` và `language` thì về mặc định; vậy tên định dạng lạ là PNG.
- Ghi file: luôn ghi cả hai trường.

### 5.6 Thư viện mới

- `jpeg-encoder`, ghim đúng phiên bản 0.7.1 trong `crates/shotme-core/Cargo.toml`; không bật tính năng `simd` (đề xuất: tránh mã `unsafe` riêng cho từng CPU; ảnh chụp màn hình nén đủ nhanh mà không cần). `Cargo.lock` chỉ được thêm đúng một gói.
- Giấy phép có phần IJG, yêu cầu tài liệu đi kèm ghi rằng phần mềm dựa một phần trên công trình của Independent JPEG Group. README hôm nay chưa có mục nào về giấy phép hay thư viện: thêm một mục ngắn "Thư viện bên thứ ba" chỉ gồm dòng ghi nhận này (đề xuất). Cửa sổ Thông tin không liệt kê thư viện và không đổi.
- Ghi quyết định mới vào `memory-bank/decisions.md`: ngoại lệ một crate và lý do.

### 5.7 Trợ giúp và tài liệu

- Lời văn Trợ giúp (trong bảng chuỗi, hai ngôn ngữ) thêm một đoạn ngắn về tab Định dạng: chỉ áp dụng cho ảnh tải lên, ảnh lưu vào máy luôn là PNG, chất lượng cao hơn thì file nặng hơn.
- README: mô tả tab, hai giá trị mặc định, đuôi `.jpg`, dòng ghi nhận IJG.
- `docs/testing/manual-checklist.md`: mục mới "Đợt 3: định dạng" (viết trong task làm tab, vì chủ dự án thử tay ngay sau task đó).
- `memory-bank/`: `productContext.md`, `systemPatterns.md` (tab thứ tư, ô radio và thanh trượt là loại ô mới), `techContext.md` (crate mới).

### 5.8 Kiểm thử

Tự động, ở core:

- Bộ nén: kết quả bắt đầu bằng `FF D8`, kết thúc bằng `FF D9`; đoạn SOF0 (baseline) ghi đúng rộng, cao, ba thành phần và hệ số lấy mẫu 1×1 cho cả ba (tức 4:4:4); không có đoạn SOF2 (progressive) và không có đoạn EXIF. Ảnh không hợp lệ trả lỗi, không panic. Với cùng một ảnh nhiều chi tiết, chất lượng 100 cho file lớn hơn chất lượng 50.
- Tải lên: chọn JPEG thì khóa gửi đi kết thúc `.jpg`, kiểu nội dung là `image/jpeg`, thân bắt đầu bằng `FF D8`; chọn PNG thì yêu cầu gửi đi y như kiểm thử hiện có (các kiểm thử tải lên cũ không được sửa giá trị mong đợi; kiểm thử về thứ tự tab ở `settings.rs` thì phải đổi vì tab Lưu trữ R2 chuyển từ vị trí thứ ba sang thứ tư).
- Luồng hành động: chọn JPEG mà tải lên lỗi thì file lưu dự phòng có đuôi `.png` và nội dung đúng bằng `encode_png` của ảnh gốc; hành động Lưu không phụ thuộc cài đặt; nén JPEG lỗi thì đi đường lưu dự phòng.
- Cấu hình: file cũ đọc ra PNG và 90; ghi rồi đọc lại giữ nguyên; chất lượng -1, 0, 49, 101, 255, 300 được kéo về 50 hoặc 100; 90.5 và "90" là file hỏng.
- Bảng chuỗi: các kiểm thử "đủ hai ngôn ngữ" hiện có bao luôn chuỗi mới.

Tự động, ở lớp Windows (đề xuất): một kiểm thử giải nén JPEG do core tạo ra bằng GDI+ (tính năng `Win32_Graphics_GdiPlus` đã bật sẵn) rồi so kích thước và so màu vài vùng phẳng với ảnh gốc trong sai số nhỏ. Đây là chỗ duy nhất một bộ giải nén độc lập xác nhận file hợp lệ, vì dự án không thêm crate giải nén.

Thử tay (mục "Đợt 3: định dạng" của checklist): tab hiện đúng ở hai ngôn ngữ và ở DPI 100% và 150%; thanh trượt bằng chuột và bàn phím; mờ khi chọn PNG; OK, Hủy; chuyển đủ bốn tab bằng bàn phím; tải lên JPEG rồi mở link trong trình duyệt và tải file về mở bằng ứng dụng Ảnh của Windows; so chữ nhỏ có màu ở mức 50, 90, 100; so dung lượng với PNG của cùng vùng; tải lên lỗi (rút mạng) thì file lưu là PNG; nâng cấp từ file cấu hình cũ vẫn tải lên PNG.

## 6. Chia task, agent và model (đề xuất, chốt ở bước lập kế hoạch)

Số task nối tiếp sổ hiện có.

| Task | Nội dung | Agent làm | Model | Rà soát |
|---|---|---|---|---|
| 24 | Core: crate `jpeg-encoder`, bộ nén JPEG, hai trường cấu hình, khóa và kiểu nội dung theo định dạng, luồng tải lên chọn định dạng, chuỗi mới; lớp Windows chỉ sửa đủ để truyền cài đặt vào luồng tải lên và để build được | builder | sonnet | reviewer (sonnet) |
| 25 | Tùy chọn: tab thứ tư với hai nút chọn và thanh trượt, đọc/ghi form, lời văn Trợ giúp, kiểm thử giải nén bằng GDI+, mục checklist "Đợt 3: định dạng" | specialist | opus | specialist (opus) |
| 26 | README (kể cả dòng ghi nhận IJG), tài liệu trong `memory-bank/` | scaffolder | haiku | reviewer (sonnet) |

Lý do: Task 25 thêm tab và hai loại ô mới (radio, thanh trượt) vào cửa sổ Tùy chọn, nơi các lượt rà soát trước tìm ra nhiều điểm nhất. Task 24 là logic thuần có kiểm thử. Task 26 là việc máy móc.

Thứ tự bắt buộc: 24 → 25 → 26. Sau Task 24 cả workspace build được và qua `scripts/check.ps1`; cài đặt mới đã có tác dụng nếu sửa file cấu hình bằng tay nhưng chưa có ô để chọn, điều này chấp nhận được vì không phát hành giữa chừng.

Các task chạy nối tiếp không hỏi lại (quyết định 18). Một điểm dừng: sau Task 25, chủ dự án thử tay theo mục "Đợt 3: định dạng"; Task 26 chạy sau khi họ báo kết quả.

## 7. Để người làm quyết

- Tên hai trường trong file cấu hình và tên, hình dạng kiểu trong `Config` (ví dụ một enum `UploadFormat` và một `u8`), miễn đúng mục 5.5.
- Cách luồng tải lên nhận định dạng (trường mới trong `Ctx`, tham số của `object_key`, tham số kiểu nội dung của `Uploader::upload`…), miễn đúng bảng ở mục 5.2 và không sửa giá trị mong đợi của kiểm thử cũ.
- Lỗi nén JPEG dùng lại `EncodeError` hay kiểu mới; lời văn lý do.
- Loại ô Windows cho thanh trượt (ví dụ `msctls_trackbar32` với `ICC_BAR_CLASSES`), mã ID của các ô mới, cách hiện số bên cạnh.
- Kích thước, khoảng cách các ô; lời văn English; lời văn chính xác của nhãn, ghi chú và đoạn Trợ giúp ở cả hai ngôn ngữ, miễn giữ đúng ý ở mục 5.1 và 5.7.
- Kiểm thử GDI+ chạy thường hay đánh dấu `#[ignore]` (nếu nó không ổn định trên máy không có màn hình thì để `#[ignore]` và ghi lệnh chạy vào `techContext.md`).
