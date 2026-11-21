# Thiết kế: Privacy Shield — che thông tin nhạy cảm trước khi xuất ảnh (đợt 21)

Ngày: 2026-11-22. Yêu cầu của chủ dự án (lời giao "Darkshot Privacy Shield", Prompt 0–5). Đợt 16–19 / task 115–121 giữ cho kế hoạch MCP, đợt 20 dùng task 122–126, nhánh macOS trên `origin` dùng tới `task/115-…`: đợt này là **đợt 21**, task **127–134**, quyết định bắt đầu từ **144**.

Chủ dự án đã chọn trong trao đổi (2026-11-22):
- Cách 1: **một cổng chung** (`privacy_gate`) sau bước làm phẳng ảnh; mọi đầu ra ảnh nhận kiểu `PreparedOutput`.
- Ghi GIF **ngoài phạm vi**: khi Shield bật, thanh ghi hiện dòng "GIF không được quét"; README/website ghi rõ.

Chữ "(phiên điều phối chọn)" đánh dấu những điều chủ dự án chưa nói, phiên điều phối tự quyết; chủ dự án sửa được khi duyệt.

## 1. Mục tiêu, không làm, phản mục tiêu

**Mục tiêu.** Người dùng bật một công tắc; từ đó mọi ảnh rời phiên chụp (Lưu, Chép, Tải R2, Ghim, ghi Lịch sử, ghi ảnh bước, xuất lại từ Lịch sử) đều được nhận dạng chữ cục bộ, dò 7 loại thông tin, cho người dùng duyệt và che bằng ô đen đặc đã làm phẳng vào điểm ảnh. Không có phát hiện thì không hỏi gì.

**Không làm (MVP).**
- Không nhận diện khuôn mặt, mã QR, NER, AI, OCR đám mây, bộ nhận dạng mới (quyết định 134), mô hình học máy.
- Không quét Ghi GIF (đã chọn), không viết lại chữ của "Nhận dạng chữ" (Ctrl+T, menu ảnh ghim) — đường xuất CHỮ, để giai đoạn sau, ghi rõ trong tài liệu.
- Không quét ghi chú của Chụp từng bước (chữ người dùng gõ).
- Không viết lại mục Lịch sử cũ; không chặn "Mở" / "Hiện trong thư mục" của Lịch sử (tệp đã nằm trên máy, darkshot không chuyển đi đâu).
- Không đổi mô hình đóng gói (exe portable), không thêm crate (kể cả `regex`).

**Phản mục tiêu** (chạy được mà vẫn là thất bại):
- Bất kỳ đường nào ghi/chép/tải ảnh CHƯA qua cổng khi Shield bật.
- Báo "sạch" khi OCR lỗi; tự đi tiếp khi OCR lỗi.
- Chữ OCR hay giá trị trùng khớp lọt vào nhật ký, JSON, Lịch sử, metadata ảnh, thông điệp lỗi, `Debug`.
- Che bằng lớp mờ/trong suốt, hay che chỉ trên lớp phủ mà điểm ảnh gốc vẫn ở tệp xuất.
- Hành động gốc chạy hai lần, hoặc chạy sau khi người dùng Hủy.
- Ảnh bước thô nằm trong `steps\current` khi Shield bật.
- Hỏi người dùng khi không có gì để hỏi (quét xong, không phát hiện).

## 2. Hiện trạng liên quan (đã đọc mã)

- Ảnh: `darkshot_core::image::Image { width, height, bgra }` (BGRA, top-down). `encode_png` ghi RGB8 (bỏ alpha), `encode_jpeg`, `encode_for_upload`.
- OCR: `darkshot-windows/src/ocr.rs` — `start(image, hwnd, msg, token)` tạo luồng `darkshot-ocr`, `recognize_in_mta` đọc `OcrLine.Words()` và `BoundingRect()` từng từ (ocr.rs:154–160) rồi **gộp** thành khung dòng (`union_rect`) và bỏ khung từ. Kết quả về bằng `PostMessage` với `Box` trong LPARAM (quyết định 120). Ảnh lớn hơn `OcrEngine::MaxImageDimension()` bị co trước khi nhận dạng (`ocr_scale`). `OcrLine { text, rect }` có `derive(Debug)` in cả chữ.
- Đã chạy được trên exe portable không danh tính gói (v0.5.0, QA quyết định 121, en-US). Tài liệu Microsoft nói `Windows.Media.Ocr` hỗ trợ ứng dụng desktop có package identity: ghi vào mục thử tay (máy Windows 11 sạch), KHÔNG đổi đóng gói (mục 13).
- Ranh giới tự nhiên: `main::picked_image` (main.rs:1465) — mọi ảnh tĩnh của lớp phủ, phím toàn màn hình (SaveFull/UploadFull), Lặp lại đi qua đây (cắt + `annotate_draw::burn`), rồi rẽ: Ghim (`pin::open`), OCR, Thêm bước (`steps_store::add`), Lưu/Chép/Tải (`run_on_image` → `actions::run_action`, `app::spawn_upload`, `history_service::record_outcome`).
- Chụp cuộn đi đường riêng: `on_scroll_result` → `scroll_result::open` (cửa sổ xem trước) → `scroll_action` (Lưu/Chép/Tải/Ghim); khi không tạo được cửa sổ thì tự Lưu (scroll_result.rs:518).
- Lịch sử: chỉ có Mở, Chép ảnh, Chép link, Hiện trong thư mục, Ghim (không có Lưu/Tải từ Lịch sử). Chép ảnh / Ghim giải mã PNG trên đĩa rồi `set_image` / `pin::open`, **không** qua `run_on_image`.
- Ảnh ghim: menu Chép/Lưu/OCR qua `main::pin_action` → `run_on_image`; nếu không có sink thì `WinClipboard.set_image` trực tiếp (pin.rs:1001).
- Chụp từng bước: `steps_store::add` ghi `NNN.png` vào `%LOCALAPPDATA%\darkshot\steps\current` trên luồng `darkshot-steps`; xuất (`steps_export`) **chép tệp sang tệp**, không có ảnh ghép. Thư mục hỏng được giữ thành `current.corrupt-*`.
- Tải lên dở lúc thoát được lưu thành PNG (`app::save_pending`); lưu lỗi thì chép ảnh vào clipboard (`actions::save_failed`); tải lỗi thì lưu (`fallback`).
- Không có tệp tạm chứa điểm ảnh nào khác lúc chạy (webshot chỉ ở kiểm thử; WER loại heap). `last_capture.json` chỉ có vùng + hành động.
- WM_APP đã dùng tới +18. Exe release hiện 2.335.744 byte.

## 3. Trải nghiệm người dùng

### 3.1 Cài đặt

Tab **Chung** của Tùy chọn, ngay dưới ô Lịch sử (phiên điều phối chọn):

```
[ ] Quét thông tin nhạy cảm trước khi xuất ảnh
    Nhận dạng chữ trên máy rồi che email, số điện thoại, thẻ, khóa API, token, mật khẩu, khóa riêng.
    Không phát hiện được mọi thứ; nên tự xem lại ảnh.
```
English: "Scan for sensitive information before exporting" / "Reads text on this PC, then covers emails, phone numbers, cards, API keys, tokens, passwords and private keys. It can miss things; review your image." Khóa cấu hình `privacy_shield: bool`, mặc định `false`, đọc theo mẫu khóa mới của `config.rs` (thiếu khóa → `false`, file cũ không vỡ, ghi không đè khóa khác). Đọc giá trị **lúc cổng nhận ảnh**: đổi công tắc không ảnh hưởng lượt đang chờ.

### 3.2 Luồng chính (Shield bật)

1. Người dùng chọn hành động (Lưu/Chép/Tải/Ghim/Thêm bước) như thường. Lớp phủ đóng như cũ.
2. Cổng nhận ảnh đã làm phẳng (có hình đánh dấu), bắt đầu quét nền.
3. Quét xong, **không phát hiện** → hành động gốc chạy ngay, không cửa sổ, không thông báo thêm.
4. Quét lâu hơn **800 ms** (phiên điều phối chọn) → mở cửa sổ Duyệt ở trạng thái "Đang quét…" (có Hủy); xong mà không phát hiện thì cửa sổ tự đóng và hành động chạy.
5. **Có phát hiện** → cửa sổ Duyệt (mục 3.3). Hành động gốc chờ.
6. "Che và tiếp tục" → làm phẳng vùng đã chọn → hành động gốc chạy **đúng một lần** với ảnh đã che.
7. "Hủy" / Esc / đóng cửa sổ → không chạy gì, ảnh bị bỏ (không lưu, không Lịch sử). Hành động Thêm bước: bước không được thêm.

Lặp lại lần chụp trước và phím toàn màn hình cũng vào cổng (không có lớp phủ, nhưng có cửa sổ Duyệt nếu có phát hiện). `last_capture` vẫn nhớ như cũ (chỉ vùng + hành động).

Nhiều lượt cùng lúc: cổng giữ hàng đợi tối đa **3** lượt chờ; mỗi lúc một cửa sổ Duyệt, theo thứ tự (phiên điều phối chọn). Lượt thứ 4 bị từ chối với thông báo "Đang có ảnh chờ duyệt" (ảnh bị bỏ, không xuất).

### 3.3 Cửa sổ Duyệt (Privacy Review)

Cửa sổ riêng, giao diện tối (`theme::*`), topmost và lên trước (nó chặn một hành động người dùng vừa yêu cầu), mặc định 960×640 DIP kẹp vào vùng làm việc, đổi cỡ được, tối thiểu 640×420 DIP, đặt giữa màn hình có con trỏ (phiên điều phối chọn).

```
┌ Kiểm tra thông tin nhạy cảm ─────────────────────────────────────── ✕ ┐
│ ┌──────────────────────────────────────┐  Tìm thấy 3 mục · Lưu ảnh    │
│ │                                      │  [Chọn tất cả] [Bỏ chọn hết] │
│ │   ███████████      (ô đen = sẽ che)  │  ───────────────────────────  │
│ │                                      │  [x] Email          Cao      │
│ │   ┌ ─ ─ ─ ─ ┐  (viền nhấn = đang trỏ)│  [x] Khóa API       Cao      │
│ │   ███████████                        │  [ ] Số điện thoại  Trung bình│
│ │   └ ─ ─ ─ ─ ┘  (bỏ chọn: chỉ viền)   │  [x] Vùng tự thêm       [Xóa]│
│ │                                      │                              │
│ └──────────────────────────────────────┘  Tự động có thể bỏ sót.      │
│                                            Hãy xem lại ảnh.            │
├───────────────────────────────────────────────────────────────────────┤
│ [Hủy]                      [+ Thêm vùng che]      [Che và tiếp tục]   │
└───────────────────────────────────────────────────────────────────────┘
```

- Khung trái: ảnh co vừa khung (không phóng to quá 100%). Vùng **đã chọn** vẽ ô đen đặc; vùng **bỏ chọn** chỉ vẽ viền đứt nét mảnh (để người dùng thấy chỗ đó có gì). Mục đang trỏ/chọn trong danh sách → viền màu nhấn quanh vùng. Bấm vào một vùng trên ảnh = bật/tắt mục đó.
- Khung phải: tóm tắt (số mục, tên hành động đang chờ), Chọn tất cả / Bỏ chọn hết, danh sách: ô chọn, tên loại, độ tin cậy (Cao / Trung bình). **Không bao giờ hiện giá trị** (kể cả một phần). Một phát hiện có nhiều khung (khóa PEM nhiều dòng) là một mục.
- **Thêm vùng che**: bật chế độ kéo; kéo trên ảnh vẽ khung xem trước; thả → mục "Vùng tự thêm" (đã chọn) cuối danh sách. Khung kẹp vào ảnh; khung < 4×4 px ảnh bị bỏ. Mục tự thêm có nút Xóa. Esc trong chế độ kéo = thoát chế độ kéo (không hủy cửa sổ).
- Mọi mục mặc định **đã chọn**.
- "Che và tiếp tục" khi **không mục nào được chọn** → không đi tiếp ngay: thanh xác nhận trong cửa sổ (không MessageBox, phiên điều phối chọn): "Không có vùng nào được che. Tiếp tục mà không che?" [Quay lại] [Tiếp tục không che].
- Phím: Tab/Shift+Tab giữa các điều khiển, ↑/↓ trong danh sách, Space bật/tắt, Ctrl+A chọn tất cả, Enter = Che và tiếp tục, Esc = Hủy (như cửa sổ OCR, quyết định 133).
- Cửa sổ không làm việc tệp hay mạng: trả `ReviewOutcome` cho cổng.

### 3.4 Trạng thái lỗi OCR

Cùng cửa sổ, nội dung khác (luôn hiện, không tự đi tiếp):

```
│  ⚠ Không quét được ảnh này                                         │
│  Nhận dạng chữ của Windows không chạy được (chưa có ngôn ngữ OCR /  │
│  lỗi). Ảnh CHƯA được kiểm tra.                                     │
│  [Hủy]   [Thử lại]   [+ Thêm vùng che]   [Tiếp tục không quét]      │
```

- Thử lại: quét lại cùng ảnh (cửa sổ về "Đang quét…").
- Thêm vùng che vẫn dùng được (người dùng tự che).
- "Tiếp tục không quét" cần bấm lần hai trên thanh xác nhận: "Ảnh chưa được kiểm tra. Vẫn tiếp tục?" [Quay lại] [Vẫn tiếp tục].
- Lý do hiển thị chỉ là loại lỗi (không ngôn ngữ OCR / lỗi nhận dạng / quá thời gian) — không mã HRESULT, không chữ.
- Quá thời gian quét: **20 s** (phiên điều phối chọn) → lỗi "quá thời gian"; kết quả muộn của luồng cũ bị bỏ theo token.

## 4. Loại phát hiện và luật

Mỗi bộ dò là một `impl Detector` độc lập (thêm loại mới = thêm một tệp + một dòng đăng ký). Bộ so khớp viết tay trên `&str`/`char`, **không crate `regex`** (phiên điều phối chọn: crate `regex` thêm vài trăm KB, trái tinh thần quyết định 134; các mẫu ở đây đều ngắn, quét tuyến tính).

Văn bản đầu vào của bộ dò là chữ một dòng OCR dựng lại từ các từ (mục 5). Tiền xử lý chung: chuẩn hóa Unicode tối thiểu (dấu gạch dài → `-`, nháy cong → nháy thẳng, khoảng trắng lạ → space); so nhãn không phân biệt hoa thường và bỏ dấu tiếng Việt khi so nhãn.

Độ tin cậy hai mức: **Cao** (cấu trúc đã kiểm chứng) và **Trung bình** (dựa vào ngữ cảnh/nhãn). Mọi mức đều được chọn sẵn.

| Loại | Luật | Cao khi | Trung bình khi | Tránh nhận nhầm |
|---|---|---|---|---|
| Email | `local@domain.tld`; local `[A-Za-z0-9._%+-]{1,64}`, domain các nhãn `[A-Za-z0-9-]` cách bằng `.`, TLD ≥ 2 chữ cái | không có khoảng trắng quanh `@` | OCR tách `a @ b.com` (cho phép đúng một khoảng trắng mỗi bên `@`) | không nhận `@user` (không có domain), `x@2x.png`(TLD toàn số / đuôi ảnh phổ biến) |
| Điện thoại | VN: `(+84\|84\|0)` + đầu `3/5/7/8/9` + 8 số, cho phép nhóm cách bằng space `.` `-`; VN cố định `(+84\|0)2` + 9 số; quốc tế: `+` mã nước 1–3 số + 6–12 số có nhóm, tổng 8–15 số (E.164) | định dạng VN di động, hoặc có `+` | không `+`, không khớp VN, nhưng có nhãn trong cùng dòng hoặc dòng trên (`tel`, `phone`, `mobile`, `hotline`, `sđt`, `điện thoại`, `số điện thoại`, `liên hệ`, `zalo`) và 9–12 số | không nhận chuỗi chỉ số không nhãn; bỏ ngày (`2026-11-22`, `22/11/2026`), giờ, IP (`10.0.0.1`), số phiên bản (`1.2.3`), số tiền có `,`/`đ`/`$`, số liền chữ (`abc123…`) |
| Thẻ tín dụng | 13–19 chữ số, nhóm cách bằng space/`-` (nhóm 4 hoặc 4-6-5 của Amex); chuẩn hóa nhầm OCR **trong chuỗi số** `O/o→0`, `l/I→1`, `S→5`, `B→8` | Luhn đúng VÀ tiền tố hãng (Visa 4, MC 51–55/2221–2720, Amex 34/37, Discover 6011/65, JCB 3528–3589, UnionPay 62) | Luhn đúng, có nhãn (`card`, `thẻ`, `visa`, `mastercard`, `cvv`, `exp`) nhưng tiền tố lạ | không nhận số Luhn sai; không nhận 16 số liền không nhóm nếu không có tiền tố hãng hoặc nhãn; bỏ chuỗi toàn một chữ số |
| Khóa API | tiền tố đã biết + bộ ký tự + độ dài: `AKIA/ASIA` + 16 `[A-Z0-9]`; `AIza` + 35; `ghp_/gho_/ghu_/ghs_/ghr_` + 36; `github_pat_` + ≥ 22; `glpat-` + 20; `xox[abpr]-` …; `sk-`/`sk-proj-`/`sk-ant-` + ≥ 20; `sk_live_/sk_test_/rk_live_/pk_live_` + ≥ 16; `SG.` + 22 `.` 43; nhãn `api_key`/`api-key`/`apikey`/`x-api-key`/`client_secret`/`secret_key`/`access_key` + `:`/`=` + giá trị ≥ 16 ký tự `[A-Za-z0-9_\-./+=]` | tiền tố đã biết đúng độ dài | nhãn + giá trị dài đủ, có ≥ 2 nhóm ký tự (chữ hoa/thường/số) | không nhận chuỗi ngẫu nhiên không nhãn không tiền tố (mã băm git, UUID không nhãn) |
| JWT / token truy cập | ba đoạn base64url cách bằng `.`, đoạn 1 bắt đầu `eyJ`, mỗi đoạn ≥ 8 ký tự (đoạn 3 có thể rỗng với `alg:none`); giải mã base64url đoạn 1 ra JSON bắt đầu `{` và có `"alg"` hoặc `"typ"`; `Authorization: Bearer <token ≥ 16>`; nhãn `access_token`/`refresh_token`/`id_token`/`token`/`bearer` + giá trị ≥ 16 | JWT giải mã được; `Bearer` | nhãn + giá trị | `eyJ` không giải mã được → không nhận (trừ có nhãn); bản giải mã chỉ nằm trong biến cục bộ, xóa ngay |
| Mật khẩu / bí mật có nhãn | nhãn (`password`, `passwd`, `pwd`, `pass`, `mật khẩu`, `mat khau`, `mk`, `secret`, `passphrase`, `pin`) + `:`/`=`/`：` + giá trị không rỗng (tới hết từ, hoặc hết dòng nếu giá trị có khoảng trắng và ≤ 64 ký tự); cũng `KEY=value` trong dạng `.env` với tên chứa `PASSWORD`/`SECRET`/`TOKEN`/`KEY` | — | luôn Trung bình | chỉ che **giá trị**, không che nhãn; bỏ giá trị là `*`/`•`/`●` lặp (đã ẩn sẵn), bỏ `password:` không giá trị, bỏ chữ "Password" một mình (nút, tiêu đề) |
| Khóa riêng PEM | dòng `-----BEGIN [A-Z ]*PRIVATE KEY-----` (cả `OPENSSH`, `RSA`, `EC`, `ENCRYPTED`) tới dòng `-----END … PRIVATE KEY-----`; không thấy END → tới dòng cuối liên tiếp trông như base64 (≥ 16 ký tự `[A-Za-z0-9+/=]`) | có BEGIN | — | một phát hiện, nhiều khung (mọi dòng từ BEGIN tới END) |

Thứ tự ưu tiên khi chồng nhau (cùng khoảng ký tự): PEM > JWT > Khóa API > Thẻ > Mật khẩu > Email > Điện thoại; khoảng bị phát hiện ưu tiên cao hơn chứa trọn thì bỏ phát hiện thấp. Nhiều phát hiện trong một dòng, nhiều dòng: đều có.

Giới hạn phải ghi rõ (README/website): OCR có thể đọc sai (nhất là chữ nhỏ, font đặc biệt, nền phức tạp, tiếng Việt có dấu — quyết định 134), nên có thể bỏ sót hoặc nhận nhầm; không thấy chữ trong hình, chữ viết tay, ảnh trong ảnh; không biết ngữ cảnh ngoài ảnh. Không công bố tỉ lệ chính xác chưa đo (đo ở Task 133 trên bộ mẫu tổng hợp và ghi đúng con số).

## 5. Hình học OCR

- `ocr.rs` giữ thêm khung **từng từ** (đã đọc sẵn) cùng chữ từng từ: `OcrWord { text, rect }`, `OcrLine { text, rect, words: Vec<OcrWord> }`. `OcrLine.rect` giữ nguyên nghĩa (hợp khung từ), API cũ của cửa sổ OCR không đổi hành vi.
- Dựng chữ dòng cho bộ dò từ các từ, nối bằng một space, và giữ bảng `(char_start, char_end, word_index)` → biết chính xác từ nào chứa khoảng trùng khớp. Không dùng `OcrLine.Text()` để ánh xạ (khoảng trắng của nó không chắc khớp từ, nhất là CJK).
- Khung phát hiện = hợp khung các từ có giao với khoảng ký tự trùng khớp. Trùng một phần từ (ví dụ `password:abc123` là một từ) → che cả từ (an toàn hơn là cắt hẹp đoán chừng). Riêng luật mật khẩu: nếu nhãn và giá trị cùng một từ thì che cả từ đó.
- Dự phòng: từ không có khung hợp lệ (NaN, âm, rỗng) hoặc không ánh xạ được ký tự → dùng **khung cả dòng**; ghi `mapping: Line` vào phát hiện.
- Tọa độ: khung WinRT ở px ảnh đã đưa vào OCR → `map_rect` về px ảnh gốc, làm tròn ra ngoài (trên-trái xuống, dưới-phải lên).
- **Ảnh cao/rộng hơn `MaxImageDimension`** (ảnh chụp cuộn): KHÔNG co cả ảnh như cửa sổ OCR (chữ nhỏ sẽ mất). Cắt thành dải ngang cao ≤ `max`, chồng nhau **128 px** (phiên điều phối chọn; lớn hơn chiều cao dòng thường gặp), quét từng dải ở độ phân giải gốc, dịch tọa độ về ảnh, gộp phát hiện cùng loại có khung giao nhau giữa hai dải. Chiều rộng > `max` (hiếm) thì co theo chiều rộng như hiện nay và ghi giới hạn. Kích thước dải đặt sao cho mỗi lượt tối đa một dải trong bộ nhớ ngoài ảnh gốc.
- Luồng: chạy **cả OCR và bộ dò trên luồng `darkshot-ocr`**; luồng giao diện chỉ nhận `ScanReport` (không có chữ). Chữ OCR không bao giờ rời luồng nền; mọi `String` chứa chữ OCR bị hủy trước khi gửi kết quả (`zeroize` thủ công không cần crate: ghi đè byte trước khi `drop` cho bộ đệm chữ dòng; phiên điều phối chọn — tốt nhất có thể, không hứa trước bộ cấp phát).
- Ảnh đưa cho luồng nền bằng `Arc<Image>` (cổng giữ một bản, luồng quét đọc chung, không nhân đôi bộ nhớ).

## 6. Mô hình dữ liệu (lõi, `darkshot-core/src/privacy/`)

```rust
pub enum Category { Email, Phone, CreditCard, ApiKey, Token, Password, PrivateKey }
pub enum Confidence { High, Medium }
pub enum Mapping { Word, Line }
pub enum RuleId { EmailPlain, EmailSpaced, PhoneVnMobile, ..., PemBlock }   // &'static tên luật, không chữ

pub struct Finding {            // KHÔNG có trường chữ nào
    pub id: u32,                // tạm, duy nhất trong một ScanReport
    pub category: Category,
    pub confidence: Confidence,
    pub rects: Vec<Rect>,       // px ảnh gốc, ≥ 1
    pub rule: RuleId,
    pub mapping: Mapping,
}
pub struct ScanReport { pub findings: Vec<Finding>, pub tiles: u32, pub elapsed_ms: u32 }
pub enum ScanError { NoLanguage, Failed, Timeout, TooLarge }   // không mang chuỗi

pub struct Redaction { pub rect: Rect, pub source: RedactionSource, pub selected: bool }
pub enum RedactionSource { Detected { finding: u32, category: Category }, Manual }
pub struct RedactionPlan { pub rects: Vec<Rect> }  // chỉ vùng đã chọn, px ảnh gốc

pub enum Protection {
    Disabled,                         // Shield tắt lúc xuất
    NoFindings,                       // quét xong, không phát hiện
    Redacted { regions: u32 },        // đã che ≥ 1 vùng (tự động hoặc tự thêm)
    ReviewedNoneApplied,              // có phát hiện, người dùng xác nhận không che
    SkippedAfterError { regions: u32 },// OCR lỗi, người dùng xác nhận đi tiếp (có thể tự che)
}
impl Protection { pub fn scanned(&self) -> bool /* NoFindings | Redacted | ReviewedNoneApplied */ }

pub struct PreparedOutput { image: Image, protection: Protection }   // trường riêng tư
```

- `Debug` của `Finding`/`ScanReport` tự sinh được vì không có chữ. `OcrLine`/`OcrWord` đổi `Debug` thành bản tay chỉ in độ dài và khung (`OcrLine { text: <redacted 12 chars>, rect, words: 3 }`); `PartialEq` giữ cho kiểm thử.
- `PreparedOutput` chỉ tạo qua các hàm trong `darkshot-core/src/privacy/prepared.rs`: `PreparedOutput::unscanned(image)` (→ `Disabled`), `no_findings(image)`, `apply(image, &RedactionPlan, kind)` (làm phẳng, mục 7). Truy cập: `image(&self) -> &Image`, `protection()`, `into_image(self)` (chỉ cho đầu ra cuối). `check.ps1` thêm luật grep: các hàm tạo `PreparedOutput` chỉ được gọi trong `privacy_gate.rs` và kiểm thử (một ranh giới người rà soát tìm được bằng một lệnh).
- `Category::label(lang)`, `Confidence::label(lang)` lấy chữ từ `i18n`.

## 7. Bộ tô che (renderer)

`privacy::redact::flatten(src: &Image, rects: &[Rect]) -> Image` (lõi, thuần Rust, không GDI):
1. Bỏ khung rỗng/âm/không hữu hạn; nới mỗi khung bằng `pad = max(3, ceil(0,15 × chiều cao khung))` px (phủ viền khử răng cưa); kẹp vào ảnh.
2. Gộp khung giao hoặc chạm nhau thành khung bao (lặp tới khi không còn giao) — giảm số khung, không đổi tập điểm ảnh bị che ngoài phần bao thêm (chấp nhận che rộng hơn chút).
3. Sao chép `src` thành ảnh mới; ghi mọi điểm ảnh trong khung thành `B=0,G=0,R=0,A=255`. Không blur, không mosaic, không trong suốt. Ảnh gốc không đổi.
4. Mã hóa bằng `encode_png` / `encode_jpeg` / `encode_for_upload` có sẵn — không thêm chunk chữ (`tEXt/iTXt/zTXt`), không EXIF; kiểm thử khẳng định điều này. Không đổi hành vi PNG/JPEG hiện tại.

Không dùng lại `Shape::Blur`. Hình đánh dấu của người dùng vẫn sửa được trong lớp phủ như cũ; vùng che là lớp cuối cùng, áp lên ảnh đã `burn`.

## 8. Cổng xuất (`darkshot-windows/src/privacy_gate.rs`)

```
ảnh đã phẳng ─► privacy_gate::submit(Request { action: OutputAction, image, ctx })
                 │ Shield tắt → PreparedOutput::unscanned → dispatch(action, prepared)
                 │ Shield bật → Pending{token} + ocr::start_scan(Arc<Image>, gate_hwnd, WM_PRIVACY_SCAN, token)
                 ▼
         WM_PRIVACY_SCAN (WM_APP+19) ─► không phát hiện → no_findings → dispatch
                                      ─► có phát hiện / lỗi → review_window::open(token, …)
         WM_PRIVACY_REVIEW (WM_APP+20) ─► ReviewOutcome::{Apply(plan), ContinueNone, ContinueUnscanned(plan), Cancel}
                                         → PreparedOutput::apply/… → dispatch | bỏ
```

- `OutputAction`: `Save`, `Copy`, `Upload`, `Pin { at }`, `AddStep`, `HistoryCopy`, `HistoryPin`, `StepExport { … }` (mục 9), cùng bộ thông báo cần thiết. `dispatch` là **nơi duy nhất** gọi đầu ra: `run_on_image` (nhận `PreparedOutput`), `pin::open(PreparedOutput, …)`, `steps_store::add(PreparedOutput)`, clipboard của Lịch sử.
- Mỗi `Pending` có `token` tăng dần và trạng thái `Scanning → Reviewing → Done`; `dispatch` lấy `Pending` ra khỏi bảng **trước** khi chạy (chạy đúng một lần; thông điệp trùng/muộn không tìm thấy token thì bỏ). Hủy, đóng cửa sổ, quá thời gian đều xóa `Pending` và thả ảnh.
- Cửa sổ nhận thông điệp của cổng: cửa sổ ẩn của khay (đã có vòng thông điệp chính); `Box` trong LPARAM lấy đúng một lần, vét hàng đợi khi hủy cửa sổ (theo quyết định 120).
- Đọc `privacy_shield` một lần lúc `submit`.
- Thoát / `WM_ENDSESSION`: lượt đang chờ quét/duyệt bị **bỏ** (không lưu ảnh chưa duyệt; khác tải lên dở). Tải lên dở đã là `PreparedOutput` nên `save_pending` vẫn lưu như cũ. Không cộng thời gian vào `EXIT_BUDGET`.
- Lớp phủ đang mở khi cửa sổ Duyệt cần mở (lượt trước trong hàng đợi): chờ lớp phủ đóng (`overlay_open`), như `open_pending_pin`.

### 8.1 Các đường xuất và cách nối

| Đường | Hiện tại | Sau |
|---|---|---|
| Lớp phủ → Lưu/Chép/Tải | `picked_image` → `run_on_image(Image)` | `picked_image` → `submit(Save/Copy/Upload)` → `run_on_image(PreparedOutput)` |
| Lớp phủ → Ghim | `pin::open(Image)` | `submit(Pin)`; `pin::open(PreparedOutput)` |
| Lớp phủ → Thêm bước | `steps_store::add(Image)` | `submit(AddStep)`; `steps_store::add(PreparedOutput)` — chỉ ảnh đã qua cổng được ghi xuống `steps\current` |
| Lớp phủ → OCR (Ctrl+T) | `open_ocr` | **không đổi** (đường chữ, ngoài phạm vi) |
| Phím Lưu/Tải toàn màn hình | `run_on_image` | `submit` |
| Lặp lại lần chụp trước | `picked_image` → nhánh | `submit` |
| Chụp cuộn → Lưu/Chép/Tải/Ghim | `scroll_action` → `run_on_image`/`pin::open` | `scroll_action` → `submit`; tự Lưu khi không mở được cửa sổ → `submit(Save)`. Cửa sổ xem trước vẫn hiện ảnh gốc (chỉ trên màn hình, như lớp phủ) |
| Ảnh ghim → Chép/Lưu | `pin_action` → `run_on_image` | ảnh ghim giữ `PreparedOutput`; nếu Shield đang bật mà `protection.scanned() == false` (ghim lúc Shield tắt hoặc bỏ qua lỗi) → `submit` lại; còn lại → `run_on_image`. Bỏ nhánh `set_image` trực tiếp ở pin.rs:1001 (đi qua sink hoặc không làm gì) |
| Lịch sử → Chép ảnh / Ghim | giải mã PNG → `set_image` / `pin::open` | Shield bật → `submit(HistoryCopy/HistoryPin)` luôn quét lại (mục cũ không được coi là đã che); tắt → `unscanned` |
| Lịch sử → Mở / Hiện trong thư mục / Chép link | Shell / chữ | không đổi (ghi trong giới hạn) |
| Ghi Lịch sử mục mới | `record_outcome(Image)` trong `run_on_image` | nhận ảnh từ `PreparedOutput` (cùng ảnh đã xuất); ảnh thumbnail tạo từ ảnh đó |
| Tải lỗi → Lưu dự phòng; Lưu lỗi → clipboard; thoát → lưu tải dở | ảnh của lượt | không đổi: đã là ảnh của `PreparedOutput` |
| Ghi GIF | stream xuống `.part` | không đổi; khi Shield bật, thanh ghi hiện "GIF không được quét" |
| Xuất phiên các bước | chép tệp | mục 9 |

`run_on_image`, `pin::open`, `steps_store::add` đổi chữ ký sang `PreparedOutput` → mọi chỗ gọi cũ không biên dịch được nếu chưa đi qua cổng (trình biên dịch giữ ranh giới).

## 9. Chụp từng bước và Lịch sử

**Bước mới.** Thêm bước đi qua cổng; chỉ `PreparedOutput` tới `steps_store::add`, nên khi Shield bật không có ảnh bước thô nào được ghi (cả `.tmp` của `write_atomic`). Hủy duyệt → bước không được thêm, `session.json` không đổi. OCR lỗi → cửa sổ lỗi; không bao giờ tự ghi. Thư mục `current.corrupt-*` chỉ chứa ảnh đã từng được ghi (tức đã qua cổng khi Shield bật).

**Đánh dấu bước.** `session.json` thêm trường tùy chọn mỗi bước `"scanned": true` (thiếu = `false`; `StepSession::from_json` v1 cũ đọc được, bản cũ của darkshot bỏ qua trường lạ — kiểm lại khi làm Task 132). Ghi `true` khi `protection.scanned()`.

**Xuất phiên.** Không có ảnh ghép; xuất là chép từng tệp. Khi Shield bật lúc xuất: bước có `scanned: true` chép như cũ; bước `scanned: false` (chụp lúc Shield tắt, hay bỏ qua lỗi) được giải mã và đi qua cổng lần lượt (`OutputAction::StepExport`), ảnh đã che được ghi vào thư mục xuất tạm thay cho bản chép; Hủy ở bất kỳ bước nào → hủy cả lượt xuất (xóa thư mục tạm, như lỗi xuất hiện nay). Tệp trong `steps\current` không bị viết lại. Đây là cách hiểu đúng của "quét lại ảnh cuối trước khi xuất" cho mô hình xuất thư mục (phiên điều phối chọn).

**Lịch sử.** Mục mới chỉ chứa ảnh đã xuất (cùng `PreparedOutput`). Mục cũ không bị viết lại; Chép ảnh / Ghim từ Lịch sử khi Shield bật luôn quét lại. Không thêm trường "đã che" vào Lịch sử (đơn giản, và mục cũ vẫn phải quét); hệ quả: người dùng có thể thấy lại phát hiện mình đã bỏ chọn — ghi "Đã biết".

## 10. Chế độ lỗi

| Tình huống | Hành vi |
|---|---|
| Không có ngôn ngữ OCR | trạng thái lỗi "Windows chưa cài ngôn ngữ nhận dạng chữ" |
| WinRT lỗi / ảnh không hợp lệ | trạng thái lỗi "Không nhận dạng được chữ" |
| Quá 20 s | trạng thái lỗi "Quét quá lâu"; kết quả muộn bị bỏ |
| Không tạo được luồng quét | trạng thái lỗi như WinRT lỗi |
| Không tạo được cửa sổ Duyệt | KHÔNG xuất; thông báo lỗi "Không mở được cửa sổ kiểm tra; ảnh chưa được xuất" |
| Hàng đợi đầy (3) | thông báo, bỏ lượt mới |
| Ảnh quá lớn cho bộ nhớ dải | `TooLarge` → trạng thái lỗi |
| Làm phẳng lỗi (không cấp được bộ nhớ) | không xuất; thông báo lỗi |
| Thoát khi đang chờ | bỏ lượt, không ghi gì |

Không đường lỗi nào dẫn tới xuất ảnh chưa quét mà không có hai lần bấm xác nhận của người dùng.

## 11. Mô hình đe dọa

Tài sản: điểm ảnh chứa thông tin nhạy cảm; chữ OCR. Kẻ có thể thấy: người nhận ảnh/link R2, ai đọc clipboard (ứng dụng khác, lịch sử clipboard), ai đọc ổ đĩa (thư mục lưu, `history\`, `steps\`), ai đọc nhật ký/báo lỗi.

| Mối đe dọa | Biện pháp |
|---|---|
| Đường xuất vòng qua cổng | chữ ký nhận `PreparedOutput`; luật grep trong `check.ps1`; rà soát liệt kê mọi lệnh `set_image`, `encode_png`, `write`, `upload` |
| Ảnh thô trên đĩa (bước, tạm) | quét trước `steps_store::add`; không tạo tệp tạm cho quét; không ghi ảnh chờ duyệt lúc thoát |
| Che có thể đảo ngược | ô đặc alpha 255 làm phẳng trước khi mã hóa; không lớp, không blur; kiểm thử điểm ảnh |
| Lộ viền chữ | nới khung; gộp |
| Lộ qua metadata | encoder hiện có không ghi chunk chữ/EXIF; kiểm thử |
| Lộ chữ OCR | bộ dò chạy trên luồng nền; `Finding` không có chữ; `Debug` của dòng OCR che chữ; `ScanError` không chuỗi; không `eprintln!`/log ở module privacy (luật grep) |
| Báo sạch giả khi OCR lỗi | lỗi là trạng thái riêng, cần xác nhận hai bước |
| Chạy hành động hai lần / sau Hủy | bảng `Pending` theo token, lấy ra trước khi dispatch |
| Lộ trong cửa sổ Duyệt | không hiển thị giá trị; cửa sổ chỉ trên màn hình người dùng |
| Lộ qua clipboard history/cloud | giữ định dạng loại trừ có sẵn của `clipboard.rs` |

Ngoài phạm vi (ghi rõ): kẻ đọc bộ nhớ tiến trình; ảnh gốc hiện trên màn hình lúc duyệt; Ghi GIF; chữ OCR của tính năng Nhận dạng chữ; tệp người dùng tự Mở từ Lịch sử; mục Lịch sử/phiên bước tạo khi Shield tắt.

## 12. Cam kết và giới hạn quyền riêng tư (chữ cho README, website, Trợ giúp)

- Nhận dạng và dò chạy hoàn toàn trên máy (Windows OCR); ảnh và chữ không gửi tới dịch vụ AI hay máy chủ nào để phân tích.
- Tự động là cố gắng tốt nhất: OCR có thể bỏ sót; hãy tự xem lại ảnh; có thể tự thêm vùng che.
- Không phát hiện thông tin không phải chữ (khuôn mặt, mã QR, hình ảnh) trong bản này.
- Không áp cho Ghi GIF và cho chữ chép từ "Nhận dạng chữ".
- Không bao giờ viết "bảo đảm" hay "100%".

## 13. Kiểm chứng OCR với exe portable

Task 128 ghi vào báo cáo: phiên bản Windows, `MaxImageDimension`, ngôn ngữ nhận dạng, thời gian trên ảnh tổng hợp — chạy trên exe release không có package identity trên máy chủ dự án (đã biết là chạy, quyết định 121). Máy Windows 11 sạch: mục thử tay (không có máy đó trong phiên). Có khác biệt với tài liệu Microsoft thì ghi vào `decisions.md`, không đổi đóng gói.

## 14. Kiểm thử

Mọi dữ liệu mẫu là **giả, tự sinh** (ví dụ `user@example.com`, số thẻ thử Luhn đúng công khai `4111 1111 1111 1111`, khóa giả có tiền tố nhưng thân `EXAMPLE…`); không ảnh chụp thật, không khóa thật. Chuỗi mẫu dạng khóa thật được ghép lúc chạy (`concat!`) để công cụ quét bí mật của GitHub không báo nhầm.

- Lõi bộ dò (Task 127): đúng/sai từng loại; VN và quốc tế; Luhn đúng/sai; JWT giải mã được/không; nhãn mật khẩu; PEM có/không END; nhiều phát hiện một dòng; nhiều dòng; ưu tiên chồng nhau; bộ mẫu dương tính giả (ngày, giờ, IP, phiên bản, số tiền, mã băm, UUID, `@mention`, "Password" một mình, `****`); ánh xạ ký tự → từ, hợp khung, dự phòng khung dòng, co tọa độ, ghép dải chồng.
- OCR (Task 128): kiểm thử `#[ignore]` trên ảnh vẽ chữ tổng hợp bằng GDI (không ảnh thật): khung từ nằm trong khung dòng; ảnh cao 3 × max ra đủ phát hiện ở mọi dải; không chữ nào trong `ScanReport`.
- Renderer (Task 129): mọi điểm trong khung = (0,0,0,255); điểm ngoài khung không đổi; nới, kẹp, gộp, khung rỗng/âm/ngoài ảnh; ảnh 1×1, ảnh lớn (tổng hợp, 8000×20000); ảnh gốc không đổi; PNG/JPEG giải mã lại vẫn đen ở vùng che (JPEG: sai số ≤ ngưỡng nhỏ, không còn cấu trúc chữ — đo độ lệch chuẩn ≈ 0); không chunk chữ; ảnh nguồn có alpha < 255.
- Cửa sổ Duyệt (Task 130): logic thuần (chọn mặc định, chọn/bỏ tất cả, thêm/xóa vùng tay, khung quá nhỏ, xác nhận rỗng, Hủy, lỗi → Thử lại/Tiếp tục) tách khỏi Win32 để kiểm thử; bố cục ở 96/120/144/192 DPI; chuỗi đủ hai ngôn ngữ; đếm GDI/USER trước–sau mở/đóng; ảnh PrintWindow (quyết định 121, không giả lập chuột/phím thật).
- Cổng (Task 131–132): mỗi đường của bảng 8.1 với Shield tắt / bật không phát hiện / có phát hiện che hết / che một phần / bỏ hết (xác nhận) / Hủy / OCR lỗi / Thử lại / Tiếp tục không quét; token trùng/muộn; hàng đợi đầy; thoát khi chờ; bước Hủy không vào phiên; xuất phiên có bước chưa quét; Lịch sử cũ. Tải lên dùng `Uploader` giả (không R2 thật).
- Task 133: rà soát bảo mật toàn đợt, đo chất lượng bộ dò (độ chính xác, độ phủ, tỉ lệ nhận nhầm trên bộ mẫu tổng hợp — ghi đúng số đo, không suy diễn), hiệu năng, hồi quy (OCR, hoàn tác, Ghim, Lặp lại, Lịch sử, Chụp cuộn, Chụp từng bước, clipboard, tải lên, DPI, vòng đời cửa sổ, GDI).

## 15. Hiệu năng và kích thước

- Shield tắt: không thêm việc nào trên đường xuất (một lần đọc cờ).
- Luồng giao diện không bao giờ chạy OCR, dò hay làm phẳng ảnh lớn. Làm phẳng chạy trên luồng nền khi ảnh > 4 MP (phiên điều phối chọn); nhỏ hơn thì chạy tại chỗ (vài ms).
- Bộ nhớ: ảnh chờ (một bản, `Arc`) + một dải OCR + ảnh đã che (bản mới, ảnh gốc thả ngay sau khi làm phẳng). Tối đa 3 lượt chờ.
- Mục tiêu exe: tăng ≤ **80 KB** (không crate mới); quá 150 KB thì dừng và báo phiên điều phối. Đo trước/sau mỗi task (gốc 2.335.744 byte).
- Đo và ghi (Task 133): thời gian OCR, dò, làm phẳng cho ảnh 1080p, 4K, chụp cuộn 20 000 px; bộ nhớ đỉnh; ảnh hưởng khởi động (mục tiêu: 0, không khởi tạo gì khi Shield tắt).

## 16. Tiêu chí chấp nhận

1. Shield tắt: mọi đầu ra giống hệt trước (kiểm thử hồi quy qua), `Protection::Disabled`.
2. Shield bật, ảnh không có chữ nhạy cảm: hành động chạy không có cửa sổ (trừ quét > 800 ms thì có cửa sổ "Đang quét" tự đóng).
3. Shield bật, có phát hiện: cửa sổ Duyệt, mọi mục chọn sẵn, không hiện giá trị; che → tệp/clipboard/R2/ghim/Lịch sử/bước đều chỉ chứa ảnh đã che.
4. Hủy: không có gì được xuất, ghi, tải, ghim hay thêm bước.
5. OCR lỗi: không bao giờ đi tiếp nếu người dùng chưa xác nhận hai bước.
6. Không có ảnh bước thô trong `steps\current` khi Shield bật.
7. Chép/Ghim từ Lịch sử khi Shield bật luôn quét lại.
8. Không chữ OCR/giá trị trong log, JSON, Lịch sử, metadata, thông điệp lỗi, `Debug`.
9. Vùng che là điểm ảnh (0,0,0,255) trong tệp xuất.
10. Một ranh giới: `privacy_gate.rs`; luật grep trong `check.ps1` qua.
11. Chuỗi hai ngôn ngữ từ `i18n`; DPI 96–192; không rò GDI.
12. `check.ps1` in `ALL CHECKS PASSED`; exe tăng trong ngân sách; website `npm run build` qua.
13. Tài liệu (README, CHANGELOG "Chưa phát hành", Trợ giúp, website EN/VI) nói đúng cam kết và giới hạn mục 12. Không phát hành.

## 17. Cách tiếp cận đã cân nhắc

1. **Một cổng chung sau bước làm phẳng** (đã chọn): một nơi kiểm, trình biên dịch giữ ranh giới qua kiểu `PreparedOutput`; phải tách `capture_flow` thành bước chờ quét (rủi ro tái cấu trúc, đặt ở task model mạnh).
2. Quét trong lớp phủ trước khi đóng: không phủ chụp cuộn, Lịch sử, Lặp lại; vẫn cần cách 1 → làm hai lần.
3. Gắn kiểm tra vào từng đầu ra: ít đụng `capture_flow`, nhưng năm–sáu điểm rải rác, khó chứng minh không còn đường vòng; dễ sót khi thêm tính năng.

## 18. Điểm đã chốt sau đọc thử (phiên điều phối chọn)

1. **Xuất phiên các bước = một lượt của cổng** (chiếm một chỗ trong hàng đợi 3). Trước khi luồng `darkshot-steps-export` chạy, cổng quét lần lượt các bước `scanned: false` (mỗi bước một lần quét, cửa sổ Duyệt chỉ mở cho bước có phát hiện hoặc lỗi; tiêu đề ghi "Bước 3/12"). OCR lỗi ở một bước cần xác nhận hai lần cho đúng bước đó (không có "áp cho tất cả"). Hủy ở bất kỳ bước nào = hủy cả lượt xuất. Xong thì luồng xuất nhận danh sách `(chỉ số bước, ảnh đã che)` và ghi PNG mã hóa từ ảnh đó thay cho bản chép; bước khác vẫn chép tệp. Lượt chụp mới vẫn xếp hàng sau lượt xuất. Luồng xuất không thành máy trạng thái; phần quét/duyệt chạy trước nó trên luồng giao diện qua cổng.
2. **Thời hạn quét 20 s cho mỗi dải**, không cho cả ảnh. Một dải quá hạn = cả lượt quét lỗi "Quét quá lâu" (không có "quét một phần"). Không có thời hạn tổng riêng; cửa sổ "Đang quét…" (sau 800 ms) có Hủy và hiện "dải k/n".
3. **Ngôn ngữ OCR của lượt quét:** dùng `en-US` nếu Windows có (`OcrEngine::IsLanguageSupported` + `TryCreateFromLanguage`), vì bảy loại đều là chữ Latin/ASCII; không có thì dùng ngôn ngữ hồ sơ như cửa sổ OCR; không có cả hai → `NoLanguage`. Chỉ một bộ nhận dạng mỗi dải. Cửa sổ "Nhận dạng chữ" không đổi.
4. **Ghép từ bị OCR tách** (cho Khóa API, JWT/token, Thẻ, PEM): khi dựng chữ dòng, hai từ liền nhau được nối KHÔNG khoảng trắng nếu khe ngang giữa hai khung < 0,35 × chiều cao dòng VÀ ký tự cuối/đầu đều thuộc bộ ký tự token (`[A-Za-z0-9_\-.+/=]`). Bộ dò chạy trên cả hai bản dựng (có và không ghép), phát hiện trùng khung thì gộp. Email, điện thoại, mật khẩu dùng bản có khoảng trắng. Một token xuống dòng (hai dòng OCR) không được ghép: ghi "Đã biết".
5. **Luồng:** mọi cửa sổ (lớp phủ, ảnh ghim, Lịch sử, kết quả chụp cuộn, các bước, Duyệt) chạy trên luồng giao diện chính; luồng phụ (giải mã PNG của Lịch sử…) gửi kết quả về luồng chính trước khi gọi `submit`. `submit` và bảng `Pending` chỉ dùng trên luồng chính (`thread_local!`, không `Mutex`); gọi từ luồng khác là lỗi lập trình (`debug_assert` theo luồng).
6. **Lịch sử và tải lên:** duyệt xảy ra TRƯỚC `run_on_image`. Không có bản ghi Lịch sử, ảnh thu nhỏ, lưu dự phòng hay chép link nào trước khi cổng có `PreparedOutput`. `last_capture.json` vẫn được ghi trước như hiện nay (chỉ vùng + hành động, không điểm ảnh); Hủy không xóa nó (Lặp lại sau đó vẫn qua cổng).

## 19. Để lại cho người làm

Tên hàm/biến cụ thể ngoài các kiểu ở mục 6; bố cục điểm ảnh chính xác của cửa sổ Duyệt trong khung mục 3.3; cách vẽ viền đứt nét; cách tách logic cửa sổ để kiểm thử; danh sách tiền tố khóa API bổ sung (giữ nguyên nguyên tắc "tiền tố + độ dài", báo trong báo cáo task).
