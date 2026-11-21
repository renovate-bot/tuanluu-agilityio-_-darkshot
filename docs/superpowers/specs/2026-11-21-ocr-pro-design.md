# Thiết kế: Nhận dạng chữ nâng cao: chọn dòng trên ảnh (đợt 15 nội bộ)

Ngày: 2026-11-21. Trạng thái: chủ dự án duyệt 2026-11-21.

## 1. Vì sao

Hôm nay cửa sổ "Chữ nhận dạng" chỉ đưa ra toàn bộ chữ trong vùng chụp. Người dùng thường chỉ cần vài dòng (một câu, một con số, một tiêu đề) nên phải chép hết rồi xóa bớt. Chủ dự án muốn chọn đúng những chỗ cần lấy ngay trên ảnh, như cửa sổ OCR của ShareX đang làm (chỉ lấy ý tưởng bố cục, không chép giao diện hay thương hiệu).

**Thành công:** người dùng nhìn ảnh, thấy khung quanh từng dòng chữ, bấm vào là dòng đó vào clipboard. Ctrl+bấm thì gom nhiều dòng, dán một lần là có đủ theo thứ tự đọc. Exe gần như không nặng thêm.

## 2. Mục tiêu

1. Cửa sổ hai khung: trái là ảnh đã nhận dạng, mỗi dòng chữ có khung. Phải là ô chữ kết quả (vẫn sửa được), ba công tắc và các nút.
2. Bấm một khung: chọn đúng dòng đó và chép chữ của nó. Hiện "Đã chép" ngay cạnh khung trong khoảng 1,2 s.
3. Ctrl+bấm một khung: thêm hoặc bỏ dòng đó khỏi vùng chọn, rồi chép mọi dòng đang chọn theo thứ tự đọc (thứ tự Windows trả về). Vùng chọn rỗng thì không đụng clipboard.
4. Bấm chỗ trống trên ảnh: bỏ chọn, không đụng clipboard.
5. Ba công tắc, mặc định đều tắt, nhớ giữa các lần mở:
   - **Gộp thành một dòng:** nối mọi dòng bằng một dấu cách.
   - **Tách đoạn:** chèn một dòng trống giữa các đoạn (cách chia đoạn ở mục 6.2). Bị vô hiệu khi "Gộp thành một dòng" bật.
   - **Tự chép khi xong:** nhận dạng xong (có chữ) thì chép toàn bộ chữ và báo "Đã chép chữ" như khi bấm nút.
   Hai công tắc đầu áp dụng cho cả ô chữ lẫn chữ chép từ các khung.
6. Nút **"Chép tất cả"** (thay "Chép") và **"Đóng"**.
7. Dùng được bằng bàn phím (mục 6.4).

## 3. Không làm

- Dịch chữ, gửi chữ hay ảnh đi đâu. Mọi thứ vẫn trên máy.
- Chọn vùng mới hay mở tệp ảnh từ cửa sổ OCR.
- Chọn từng từ, kéo chuột để quét chọn, phóng to/thu nhỏ ảnh, sửa khung.
- Bộ nhận dạng khác Windows. Tesseract đã được đo: exe sẽ thành khoảng 6–8 MB với mô hình tiếng Việt nhẹ nhất, hoặc 14–18 MB với mô hình tốt. Chủ dự án chọn giữ darkshot nhẹ (quyết định 134).
- Đổi luồng nhận dạng (vẫn luồng nền `darkshot-ocr`, quyết định 119).

**Coi là thất bại dù vẫn chạy:**
- clipboard bị ghi khi người dùng chưa bấm gì (trừ khi họ bật "Tự chép khi xong");
- exe nặng thêm đáng kể (quá khoảng 60 KB);
- chữ nhận dạng xuất hiện trong nhật ký hay tệp;
- giao diện không giữ được 60 fps khi rê chuột trên ảnh lớn.

## 4. Ràng buộc

- Chỉ dùng `Windows.Media.Ocr` và các feature `windows` đã bật ở đợt 11. Không thêm crate.
- Tiếng Việt: Windows không có gói nhận dạng tiếng Việt (máy chủ dự án chỉ có en-US, ja, ko dù danh sách ngôn ngữ có `vi`), nên chữ có dấu có thể mất dấu. README, website và Trợ giúp phải nói thẳng điều này. Không hứa sẽ có.
- Mọi chữ hiện lên lấy từ `darkshot_core::i18n` (vi, en).
- Giao diện tối theo `theme::*`. Khung dòng dùng `theme::ACCENT`.
- DPI theo màn hình. `WM_DPICHANGED` vẫn đúng.
- Các luật sở hữu `WM_OCR_RESULT` hiện có (lấy `Box` đúng một lần, vét hàng đợi khi hủy cửa sổ) giữ nguyên.
- Cửa sổ OCR mở từ ảnh ghim vẫn theo quyết định 132 (topmost khi phải chồng lên ảnh ghim).
- QA không giả lập chuột/phím thật (quyết định 111): kiểm bằng PrintWindow và thông điệp gửi thẳng tới HWND của chính cửa sổ.

## 5. Các hướng đã cân nhắc

| Hướng | Được | Mất | Chọn? |
|---|---|---|---|
| A. Bấm một khung chép đúng một dòng (như ShareX) | Đơn giản nhất | Lấy ba dòng phải dán ba lần | Không |
| **B. Như A, thêm Ctrl+bấm để gom nhiều dòng** | Đúng nhu cầu "lấy vài chỗ", một lần dán | Thêm trạng thái vùng chọn | **Có (chủ dự án chọn)** |
| Bộ nhận dạng tiếng Việt: Tesseract đóng gói | Giữ dấu tiếng Việt | Exe gấp 3–8 lần, build cần C++/CMake | Không (quá nặng) |
| Tesseract tải về khi cần | Exe vẫn nhẹ | Tải và chạy mã từ mạng, cần kiểm chữ ký | Không (chủ dự án chọn hướng chỉ giao diện) |
| PaddleOCR / dịch vụ mạng | Chính xác hơn | Vài chục MB / gửi ảnh ra ngoài | Loại |

## 6. Thiết kế

### 6.1 Dữ liệu nhận dạng

`ocr.rs` lấy thêm khung của từng dòng. Khung dòng là hợp các `OcrWord.BoundingRect` của dòng đó. Tọa độ đổi ngược về điểm ảnh của ảnh gốc: chia theo tỉ lệ `ocr_scale` đã dùng, làm tròn ra ngoài và kẹp trong ảnh.

```
// darkshot-core::ocr_text (thuần, kiểm thử được)
pub struct OcrLine { pub text: String, pub rect: Rect }   // rect: px ảnh gốc
pub struct TextOptions { pub single_line: bool, pub paragraphs: bool }
pub fn paragraphs(lines: &[OcrLine]) -> Vec<usize>        // chỉ số dòng bắt đầu mỗi đoạn
pub fn compose(lines: &[OcrLine], idx: &[usize], o: TextOptions) -> String
pub fn line_at(lines: &[OcrLine], x: i32, y: i32) -> Option<usize>
```

- `OcrResult` thêm `lines_detail: Vec<OcrLine>`. `text` = `compose` của mọi dòng với tùy chọn mặc định, nên hành vi cũ không đổi. `lines` (số dòng) giữ nguyên.
- `compose` dùng các dòng `idx` theo thứ tự đọc (sắp theo chỉ số), bỏ khoảng trắng đầu và cuối dòng, rồi:
  - `single_line`: nối bằng `" "`;
  - còn lại: nối bằng `"\r\n"`, và với `paragraphs` thì thêm một `"\r\n"` trước dòng mở đoạn mới (trừ dòng đầu tiên trong `idx`).
- `line_at`: dòng đầu tiên chứa điểm. Khung được nới 2 px mỗi phía để dễ trúng.

### 6.2 Chia đoạn

Dòng i (i > 0) mở đoạn mới khi một trong hai điều sau đúng:
- **khe dọc lớn:** `top(i) − bottom(i−1)` lớn hơn 0,8 × trung vị chiều cao dòng;
- **dòng không nằm dưới dòng trước:** `top(i) < top(i−1)`, tức một cột mới hoặc khối ở chỗ khác.

Đây là phép đoán đơn giản, chỉ cần đúng với đoạn văn thông thường. Người làm viết kiểm thử cho: đoạn liền, hai đoạn có khe, hai cột, một dòng, rỗng.

### 6.3 Cửa sổ

```
┌ Chữ nhận dạng ───────────────────────────────────────────── ─ □ × ┐
│ ┌─────────────────────────────────┐  Ngôn ngữ: English · 12 dòng  │
│ │ [Hiện trạng thành cổ Vinh…]     │  ┌──────────────────────────┐  │
│ │ ┌ảnh┐ [NGHỆ AN – 222 năm…]      │  │ chữ kết quả (EDIT, sửa   │  │
│ │ └───┘ [đô thị, thành cổ…] Đã chép│  │ được, cuộn dọc)         │  │
│ │       ...                       │  └──────────────────────────┘  │
│ └─────────────────────────────────┘  ☐ Gộp thành một dòng          │
│  Bấm khung để chép dòng ·             ☐ Tách đoạn                   │
│  Ctrl+bấm để chọn nhiều dòng          ☐ Tự chép khi xong            │
│                                      [   Chép tất cả   ] [ Đóng ]  │
└──────────────────────────────────────────────────────────────────┘
```

- **Cỡ:** mặc định 960×600 DIP, tối thiểu 640×400 DIP. Khung ảnh chiếm khoảng 58% bề rộng, khung phải tối thiểu 280 DIP. Đặt cửa sổ như hiện nay (giữa màn hình chứa con trỏ, tránh ảnh ghim).
- **Khung ảnh** là cửa sổ con tự vẽ, có `WS_TABSTOP`, nền `INPUT`, đệm 8 DIP.
  - Ảnh vừa khít, giữ tỉ lệ, không phóng quá 1:1 điểm ảnh vật lý.
  - Ảnh cao hơn khung khi đã khớp bề rộng thì khớp bề rộng và cuộn dọc bằng bánh xe hoặc thanh cuộn.
  - Vẽ bằng bộ đệm ngoài màn hình. Ảnh đã co được lưu sẵn thành DIB mỗi lần đổi cỡ, không co lại ở mỗi `WM_PAINT`.
- **Khung dòng:**
  - thường: viền 1 px `ACCENT`;
  - rê chuột qua: viền 2 px;
  - đang chọn: phủ `ACCENT` độ đục khoảng 30% (`AlphaBlend`, như `history_paint`) và viền 2 px;
  - con trỏ chuột hình bàn tay khi ở trên khung.
- **"Đã chép":** nhãn nhỏ nền `SURFACE`, chữ `TEXT`, đặt trên khung vừa bấm (dưới khung nếu sát mép trên), tự ẩn sau 1,2 s.
- **Trước khi có kết quả và khi lỗi:**
  - khung ảnh vẫn hiện ảnh, không có khung dòng;
  - ô chữ và dòng trạng thái giữ hành vi hiện nay (Đang nhận dạng…, Không tìm thấy chữ, chưa có gói ngôn ngữ, lỗi);
  - công tắc dùng được, nút "Chép tất cả" bị vô hiệu như hiện nay.
- **Dòng gợi ý** dưới khung ảnh, màu `TEXT_MUTED`.
- **Công tắc:** ô tích tự vẽ cùng kiểu Tùy chọn (`BS_AUTOCHECKBOX` + vẽ tối hiện có). Đổi công tắc thì:
  - dựng lại ô chữ từ các dòng; sửa tay trong ô chữ sẽ mất, như ShareX;
  - lưu ngay vào cấu hình;
  - không chép lại vùng chọn đang có: tùy chọn mới chỉ áp dụng từ lần bấm sau, để clipboard không bị ghi khi người dùng chưa bấm khung.
- **"Chép tất cả"** chép nội dung ô chữ như nút "Chép" hiện nay (kể cả phần đã sửa tay).
- **Ảnh hiển thị:** cửa sổ giữ một bản `Image` riêng (bản gửi `ocr::start` là bản sao), giải phóng khi đóng.
  - Người làm có thể giữ bản đã co thay vì bản gốc nếu ảnh rất lớn, miễn khung dòng vẫn đúng.
  - Tối đa vẫn `MAX_WINDOWS` = 8 cửa sổ.

### 6.4 Bàn phím

- Tab đi vòng: khung ảnh → ô chữ → ba công tắc → Chép tất cả → Đóng.
- Khi khung ảnh có tiêu điểm:
  - **↑/↓:** chuyển dòng đang trỏ (viền nét đứt quanh dòng); cuộn ảnh cho dòng đó hiện ra;
  - **Enter:** như bấm dòng đó;
  - **Space:** như Ctrl+bấm dòng đó;
  - **Ctrl+A:** chọn mọi dòng và chép;
  - **Esc:** bỏ chọn, nếu đang có vùng chọn; nếu không thì đóng cửa sổ như hiện nay.
- Ctrl+C trong ô chữ vẫn là chép phần bôi đen của EDIT.

### 6.5 Cấu hình

`config.json` thêm `ocr_single_line`, `ocr_paragraphs`, `ocr_auto_copy` (bool). Thiếu hoặc sai kiểu thì `false`, theo mẫu `history_enabled`. Ba công tắc chỉ nằm trong cửa sổ OCR, không thêm vào Tùy chọn.

"Tự chép khi xong" đảo lại một phần quyết định 118 (trước kia không tự chép), nhưng chỉ khi người dùng tự bật (quyết định 133).

### 6.6 Chữ mới (vi / en)

| Khóa | vi | en |
|---|---|---|
| `OcrCopyAll` | Chép tất cả | Copy all |
| `OcrSingleLine` | Gộp thành một dòng | Single line |
| `OcrParagraphs` | Tách đoạn | Separate paragraphs |
| `OcrAutoCopy` | Tự chép khi xong | Copy automatically when done |
| `OcrPickHint` | Bấm khung để chép dòng · Ctrl+bấm để chọn nhiều dòng | Click a box to copy the line · Ctrl+click to pick several lines |
| `OcrCopiedTip` | Đã chép | Copied |

`OcrCopy` ("Chép") bỏ nếu không còn dùng.

Trợ giúp thêm một câu về bấm khung và Ctrl+bấm, và một câu: "Windows chưa có bộ nhận dạng tiếng Việt, chữ có dấu có thể bị mất dấu."

### 6.7 Quyền riêng tư

Không đổi: ảnh và chữ chỉ ở bộ nhớ, không mạng, không nhật ký chữ. Clipboard chỉ bị ghi khi người dùng bấm hoặc gõ phím (khung, Chép tất cả, Ctrl+A, Enter, Space), hoặc khi tự bật "Tự chép khi xong".

## 7. Kiểm thử và QA

- **Lõi:** `paragraphs`, `compose` (mọi tổ hợp tùy chọn, `idx` lộn xộn hoặc trùng, rỗng), `line_at` (nới 2 px, chồng lấn thì lấy dòng đầu), đổi tọa độ khi ảnh bị co hoặc phóng (`ocr_scale`), cấu hình thiếu hoặc sai kiểu.
- **`ocr` thủ công** (như `ocr::tests::manual`): ảnh tổng hợp chữ Anh nhiều dòng, khung trả về nằm trong ảnh và theo thứ tự trên xuống.
- **Cửa sổ:** kiểm thử `live` bằng thông điệp gửi thẳng tới khung ảnh (`WM_LBUTTONDOWN` có và không có `MK_CONTROL`, phím ↑/↓/Enter/Space/Ctrl+A/Esc). Đọc clipboard bằng `WinClipboard`, chụp bằng PrintWindow. Không gửi đầu vào thật.
- **Kịch bản demo** `scripts/demo-ocr.ps1` vẫn chạy được: ảnh mẫu có vài đoạn chữ Anh.
- **Kích thước exe:** đo trước và sau, ghi vào kế hoạch; tăng quá 60 KB thì dừng lại hỏi.

## 7b. Chi tiết đã chốt sau khi rà (người viết thiết kế quyết)

1. **Dòng rỗng:** dòng có chữ rỗng sau khi trim, hoặc không có từ nào, bị bỏ khỏi `lines_detail`. Không có khung, không tính dòng. `text` = `compose` mọi dòng còn lại. `join_lines` chỉ còn giữ nếu nơi khác dùng; các kiểm thử cũ của nó vẫn phải qua hoặc chuyển sang `compose`.
2. **Chữ và khung:** chữ lấy từ `OcrLine.Text`. Khung lấy từ hợp `BoundingRect` các từ.
3. **Đổi tọa độ:** dùng tỉ lệ riêng từng trục: `x·w/w2`, `y·h/h2`, với `(w2,h2)` là cỡ đã đưa vào OCR.
4. **Chép:**
   - "Đã chép" chỉ hiện khi ghi clipboard thành công.
   - Ghi lỗi thì báo bằng thông báo "Không chép được chữ" (`OcrCopyFailedTitle`) như nút hiện nay.
   - Ctrl+bấm bỏ dòng cuối cùng đang chọn: không ghi, không nhãn.
   - Ctrl+A khi không có dòng nào: không làm gì.
5. **Ô chữ luôn hiện toàn bộ chữ** (theo công tắc), không bao giờ chỉ hiện vùng chọn. Clipboard và ô chữ có thể khác nhau; điều này chấp nhận được.
6. **Tự chép:** xét công tắc lúc kết quả về, chép `compose` theo công tắc lúc đó. Bật công tắc sau khi kết quả đã về thì không chép bù.
7. **Bộ nhớ:**
   - Ảnh có cạnh dài trên 4096 px thì cửa sổ giữ bản đã co về cạnh dài 4096 (`scale_bgra`).
   - Khung dòng đổi theo hai lớp: px gốc → px bản giữ → px hiển thị.
   - Bản giữ là bản duy nhất ngoài DIB đã co cho khung hiện tại.
8. **Thanh cuộn:** `WS_VSCROLL` thật với giao diện `DarkMode_Explorer`, như danh sách Lịch sử (`history_window.rs`).
9. **Chia đoạn:** "dòng không nằm dưới dòng trước" là `top(i) < top(i−1) − 0,5 × trung vị chiều cao`, có dung sai để chữ khác cỡ trên cùng một hàng không bị tách nhầm.
10. **Cấu hình:** ba khóa độc lập, mỗi khóa thiếu hoặc sai kiểu thì `false`. Lưu bằng đường lưu cấu hình hiện có mỗi lần đổi công tắc (người dùng bấm tay nên tần suất thấp). `ocr_window` nhận giá trị đầu và hàm lưu từ `main.rs`, như các cửa sổ khác lấy cấu hình.

## 8. Để người làm quyết

Cách co ảnh (`StretchBlt` HALFTONE hay co sẵn bằng `scale_bgra`), cấu trúc mã trong `ocr_window.rs` (có thể tách `ocr_view.rs` cho khung ảnh), cách vẽ viền nét đứt, độ dài thanh cuộn.

## 9. Quyết định mới

- **133:** OCR chọn dòng trên ảnh; Ctrl+bấm gom; ba công tắc lưu trong cấu hình, mặc định tắt; "Tự chép khi xong" do người dùng tự bật (đảo một phần 118).
- **134:** Không thêm bộ nhận dạng ngoài Windows (Tesseract đo được +4–6 MB tối thiểu). Giữ darkshot nhẹ; ghi rõ giới hạn tiếng Việt.
