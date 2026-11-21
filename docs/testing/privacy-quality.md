# Privacy Shield — chất lượng bộ dò, hiệu năng, kích thước (Task 133)

Mọi số dưới đây là số đo thật, chạy ngày 2026-11-22 trên máy của chủ dự án: Intel Core Ultra 7 155H, RAM 32 GB, Windows 11 Home bản dựng 26300, OCR `en-US`, `MaxImageDimension` = 10000. Dữ liệu mẫu đều GIẢ, tự sinh; chuỗi trông như khóa thật được ghép lúc chạy.

## 1. Chất lượng bộ dò (bộ mẫu tổng hợp)

Nguồn: `crates/darkshot-core/src/privacy/quality_tests.rs` (`cargo test -p darkshot-core --lib privacy::quality`). Kiểm thử `quality_does_not_regress` khẳng định ngưỡng = số đo dưới đây (số đúng tối thiểu, số nhận nhầm tối đa mỗi loại; tối đa 6 dòng âm tính bị nhận nhầm). Sửa bộ dò làm tốt hơn thì nâng ngưỡng.

Cách đo:
- Mẫu dương tính: 177 mẫu (mỗi mẫu một hoặc vài dòng OCR tổng hợp, khung từ 10 px/ký tự), mỗi mẫu mong đúng một loại. Gồm biến thể định dạng, nhóm số, nhãn tiếng Việt/Anh, lỗi OCR hay gặp (`I/l/1`, `O/0`, `S/5`, `rn`/`m`, dấu `,` thay `.`), từ bị OCR tách sát nhau.
- Mẫu âm tính: 196 dòng (ngày, giờ, IP, MAC, phiên bản, tiền, mã băm, UUID, mã đơn, đường dẫn, URL, biến môi trường thường gặp như `PWD=/home/x`, `SECRETARY=…`, `HOTKEY=…`, `SECRETSANTA=…`, chữ giao diện "Password"/"Forgot password?", chữ thường vi/en, đoạn mã, 20 dòng "khó" gần giống dữ liệu nhạy cảm).
- Một mẫu "đúng" khi có ít nhất một phát hiện cùng loại (không chấm vị trí khung ở đây; khung được kiểm ở `privacy/tests.rs`). "Nhận nhầm" loại X: có phát hiện loại X trên mẫu không mong X.
- Độ chính xác = đúng / (đúng + nhận nhầm). Độ phủ = đúng / số mẫu. Tỉ lệ nhận nhầm = số dòng âm tính có phát hiện loại đó / 196.

| Loại | Mẫu | Đúng | Bỏ sót | Nhận nhầm | Độ chính xác | Độ phủ | Tỉ lệ nhận nhầm |
|---|---|---|---|---|---|---|---|
| Email | 24 | 20 | 4 | 0 | 100,0 % | 83,3 % | 0,00 % |
| Điện thoại | 26 | 23 | 3 | 1 | 95,8 % | 88,5 % | 0,51 % |
| Thẻ | 25 | 25 | 0 | 1 | 96,2 % | 100,0 % | 0,51 % |
| Khóa API | 31 | 30 | 1 | 0 | 100,0 % | 96,8 % | 0,00 % |
| JWT / token | 23 | 22 | 1 | 0 | 100,0 % | 95,7 % | 0,00 % |
| Mật khẩu / bí mật | 28 | 26 | 2 | 4 | 86,7 % | 92,9 % | 2,04 % |
| Khóa riêng PEM | 20 | 17 | 3 | 0 | 100,0 % | 85,0 % | 0,00 % |
| **Tổng** | **177** | **163** | **14** | **6** | **96,4 %** | **92,1 %** | **3,06 %** (6/196 dòng) |

Trước các tinh chỉnh của Task 133 (bộ mẫu bản đầu 173/195, trước khi thêm 4 mẫu dương tính `SECRETS`/`+84(0) …` và 1 dòng âm tính; mã bộ dò ở `990e17c`): tổng độ chính xác 93,9 %, độ phủ 88,4 %, 10/195 dòng âm tính bị nhận nhầm (5,13 %). Trên cùng bộ mẫu bản đầu, sau tinh chỉnh: 93,9 % → 96,4 % độ chính xác, 88,4 % → 91,9 % độ phủ, 10 → 6 dòng nhận nhầm (Điện thoại đúng 21 → 22; Khóa API 27 → 30; Mật khẩu đúng 21 → 23, nhận nhầm 8 → 4).

Các tinh chỉnh (mỗi điều có kiểm thử trong `privacy/tests.rs`, mục "Task 133"):
- Khóa AWS (`AKIA`/`ASIA`) và GitHub `gh?_`: chấp nhận `l`/`|` ở vị trí chữ hoa `I` (tiền tố và thân), trên một bản sao cục bộ bị ghi đè sau khi dò. Đã xác nhận trên OCR thật: dòng `aws id AKIA…` vẽ bằng Segoe UI 32 px, OCR đọc `I` thành `l`; trước tinh chỉnh 0 phát hiện, sau 1 (`ocr::tests::manual_scan_aws_key_with_capital_i`).
- `PWD=`/`OLDPWD=` (và nhãn `pwd:`) có giá trị là đường dẫn tuyệt đối (`/…`, `~/…`, `X:\…`, `X:/…`, `\\máy\…`): không phải mật khẩu.
- `SECRET` trong tên biến chỉ tính khi là cả đoạn hoặc theo sau là `_`, `KEY` hay hết tên; `SECRETS` khi hết tên hoặc trước `_` (`SECRETS=`, `APP_SECRETS=`, `CLIENT_SECRETS_FILE=` khớp; `SECRETARY`, `SECRETARIAT_NAME`, `SECRETSANTA` không).
- Đoạn cuối `…KEY` có ≥ 3 chữ trước `KEY` là bí mật (`MASTERKEY`, `SSHKEY`, `APP_SIGNINGKEY`), trừ `MONKEY`, `HOTKEY`, `TURKEY`, `DONKEY` (và `KEYBOARD` vốn không kết thúc bằng `KEY`).
- Điện thoại `+84(0)912345678`, `+84(0) 912 345 678`, `+84 (0) 912 345 678`: `(0)` ngay sau mã nước là số trung kế, bỏ khi đếm chữ số (thành VN di động, Cao). `+44(0)2079460000` thành quốc tế.

### Còn bỏ sót (14)

| Loại | Mẫu (giả) | Vì sao |
|---|---|---|
| Email | `user @example.com` | khoảng trắng chỉ một bên `@` (luật đòi không có hoặc cả hai bên) |
| Email | `user@example,com` | OCR đọc `.` thành `,` |
| Email | `user@example .com` | OCR chèn khoảng trắng trước TLD |
| Email | `us3r@examp1e.c0m` | TLD có chữ số (`c0m`) |
| Điện thoại | `O912345678` | OCR đọc `0` thành `O` (bộ dò điện thoại không chuẩn hóa nhầm OCR) |
| Điện thoại | `+84 9l2 345 678` | `1` thành `l` |
| Điện thoại | `Call 09I2 345 678` | `1` thành `I` |
| Khóa API | `AK1A…` | `I` thành số `1` (chỉ chấp nhận `l`/`|`) |
| JWT | `eyj…` | OCR đọc `eyJ` thành `eyj` |
| Mật khẩu | `passw0rd: abc` | nhãn bị OCR đổi `o` → `0` |
| Mật khẩu | `Pass word: hunter2` | nhãn bị tách hai từ |
| PEM | `-----BEGlN RSA …` | `I` thành `l` trong `BEGIN` |
| PEM | `… PRlVATE KEY-----` | `I` thành `l` trong `PRIVATE` |
| PEM | `-----BEGIN 0PENSSH …` | `O` thành `0` (luật chỉ nhận chữ hoa và space giữa `BEGIN` và `PRIVATE KEY`) |

### Còn nhận nhầm (6/196 dòng âm tính)

| Dòng (giả) | Loại nhận | Ghi chú |
|---|---|---|
| `Delta +12 345 678 units` | Điện thoại | `+` và 8 chữ số có nhóm: đúng dạng E.164 |
| `Hash 4111111111111111 (test vector)` | Thẻ | 16 chữ số, Luhn đúng, tiền tố Visa |
| `Tip: the secret: practice daily` | Mật khẩu | nhãn `secret:` + chữ |
| `Test summary pass: 12 fail: 0` | Mật khẩu | nhãn `pass:` + giá trị |
| `PIN: required` | Mật khẩu | nhãn `pin:` + chữ |
| `MY_SECRET_NAME_LABEL=Shown on screen` | Mật khẩu | tên `.env` có đoạn `SECRET` |

Không có mẫu dương tính nào bị gán thêm loại khác.

## 2. Hiệu năng (bản release)

Lệnh: `cargo test --release -p darkshot-windows --bin darkshot -- --ignored --nocapture --test-threads=1 ocr::tests::manual_scan_timing ocr::tests::manual_perf_detect_and_flatten`. Ảnh tổng hợp nền trắng, chữ đen Segoe UI 24 px vẽ bằng GDI (không ảnh thật). `elapsed_ms` là thời gian của luồng `darkshot-ocr` từ lúc bắt đầu tới khi có `ScanReport` (OCR + dò + gộp dải).

| Việc | Ảnh / dữ liệu | Kết quả |
|---|---|---|
| OCR + dò | 1920×1080, 12 dòng chữ | 57 ms, 1 dải, 24 phát hiện |
| OCR + dò | 3840×2160, 24 dòng chữ | 135 ms, 1 dải, 48 phát hiện |
| OCR + dò | 1000×20000 (chụp cuộn), 10 dòng | 376 ms, 3 dải, 10 phát hiện |
| `privacy::detect` riêng | 1000 dòng tổng hợp (1/2 có dữ liệu giả) | trung vị 8,89 ms (7 lần: 9,99–10,56 lần đầu, sau đó 8,83–8,90), 500 phát hiện |
| `redact::flatten` | 3840×2160, 40 khung | trung vị 8,0 ms (3 lần) |
| `redact::flatten` | 8000×20000, 400 khung | trung vị 173,2 ms (169–178) |

Cổng làm phẳng trên luồng nền khi ảnh > 4 MP và có vùng che (`FLATTEN_INLINE_MAX_PIXELS`), nên 4K (8,3 MP) và ảnh chụp cuộn không làm phẳng trên luồng giao diện.

## 3. Bộ nhớ đỉnh (Working Set)

Không thêm feature `Win32_System_ProcessStatus` (`GetProcessMemoryInfo` chưa bật): đo từ bên ngoài bằng `System.Diagnostics.Process.PeakWorkingSet64` (đúng số `PeakWorkingSetSize` của `GetProcessMemoryInfo`), mỗi kiểm thử chạy một tiến trình riêng, lấy mẫu 20–25 ms, 3 lần mỗi kiểm thử (ba lần trùng nhau tới 0,1 MB).

| Kiểm thử (`ocr::tests::…`) | Đỉnh WS | Ghi chú |
|---|---|---|
| `manual_mem_baseline` (không làm gì) | 9,0 MB | nền của tiến trình kiểm thử |
| `manual_mem_scan_1000x20000` | 200,7 MB | dựng ảnh bằng GDI lên 165 MB (DIB + bản sao); nền trước quét 89 MB (ảnh 80 MB); trong quét lên 201 MB, tức lượt quét thêm ≈ 112 MB (bản dải ≤ 10000 hàng × 1000 × 4 = 40 MB cho `SoftwareBitmap` cộng bộ đệm của Windows OCR); sau quét về 16 MB |
| `manual_mem_steps_export_10x4k` (Task 133, trước Task 135) | 349,9 MB | 10 ảnh thay thế 4K BGRA trong `ExportJob.replacements` (10 × 33,2 MB = 331,8 MB) + mã hóa PNG; ghi xong 1,37–1,52 s |
| `manual_mem_steps_export_10x4k` (Task 135, ảnh phẳng như trên) | 65,2–65,3 MB | mỗi ảnh mã hóa PNG ngay rồi thả, chỉ giữ byte PNG (tổng 0,3 MB); mã hóa 10 ảnh 1,75–1,84 s, ghi 24–37 ms |
| `manual_mem_steps_export_10x4k_noisy` (Task 135) | 94,2–94,4 MB | như trên, cứ 10 hàng có 1 hàng nhiễu để PNG còn cỡ ảnh chụp thật (tổng 24,5 MB, ≈ 2,5 MB/bước); mã hóa 4,8–5,0 s, ghi 62–75 ms |

Task 135: xuất phiên các bước mã hóa PNG mỗi ảnh đã che ngay khi cổng trả ảnh (ảnh > 4 MP trên luồng nền `darkshot-steps-encode`) và chỉ giữ byte PNG trong `ExportJob.replacements` tới khi ghi; mỗi lúc chỉ một ảnh BGRA (33,2 MB cho 4K) cộng bộ đệm RGB của bộ mã hóa. Đo lại (bản release, cùng máy, cùng cách đo; số trước đo lại hôm nay: 349,9–354,5 MB): 10 bước 4K từ 349,9 MB xuống 65,2 MB (ảnh phẳng) và 94,4 MB (PNG 24,5 MB). 99 bước 4K cỡ ảnh nhiễu trên ≈ 60 MB + 99 × 2,5 MB ≈ 310 MB (suy ra, chưa đo), thay vì ≈ 3,3 GB. Mã hóa lỗi thì hủy lượt xuất và báo lỗi xuất (không tạo thư mục).

## 4. Khởi động khi Shield tắt

Kiểm mã (không đo thời gian): lúc khởi động `main` chỉ gọi `privacy_gate::set_window`, `set_sink`, `set_notifier`, `set_enabled_source`, `set_steps_session_source` — mỗi hàm chỉ ghi một con trỏ hàm hay `HWND` vào `thread_local`. Không tạo `OcrEngine`, không tạo luồng, không cấp bộ đệm, không đặt hẹn giờ (hẹn giờ canh chỉ chạy khi có lượt chờ). Bộ dò, OCR quét, cửa sổ Duyệt chỉ chạy khi `submit` với Shield bật. Shield tắt: `submit` đọc cờ một lần rồi chạy đầu ra với `PreparedOutput::unscanned` (ảnh chuyển qua `Arc` rồi lấy lại không sao chép).

## 5. Kích thước exe release

Build `cargo build --release -p darkshot-windows` ở từng mốc gộp của đợt (cùng thư mục target, máy trên):

| Mốc | Byte | Tăng so với mốc trước |
|---|---|---|
| `b2ee232` (gốc) | 2.335.744 | — |
| Task 127 `d18fab0` | 2.336.256 | +512 |
| Task 128 `8787a8a` | 2.336.768 | +512 |
| Task 129 `f42adb3` | 2.336.768 | 0 |
| Task 130 `2a1f30e` | 2.351.104 | +14.336 |
| Task 131 `daa18c8` | 2.442.752 | +91.648 |
| Task 132 `6f016ae` | 2.455.552 | +12.800 |
| Task 133 | 2.456.576 | +1.024 |
| Task 135 | 2.459.648 | +3.072 |

Cả đợt: **+123.904 byte (121,0 KiB)** — vượt mục tiêu 80 KB, dưới ngưỡng dừng 150 KB. Task 127–129 gần như không tăng vì bộ dò, quét và làm phẳng chưa được gọi từ exe (bị lược khi liên kết); chúng vào exe ở Task 131 khi cổng nối mọi thứ.

Phân bổ theo module (build `6f016ae` với một module bị cắt — hàm vào trả về ngay — so với build đầy đủ 2.455.552):

| Cắt | Byte | Phần của module |
|---|---|---|
| `privacy::detect` trả rỗng (bỏ 7 bộ dò + dựng chữ dòng) | 2.424.320 | ≈ 31.232 |
| `ocr::scan_in_mta` trả lỗi ngay (bỏ OCR quét theo dải + bộ dò + gộp dải) | 2.408.448 | ≈ 47.104 (trong đó bộ dò ≈ 31.232, phần quét/gộp ≈ 15.872) |
| `review_window::open` trả lỗi ngay (bỏ cửa sổ Duyệt và mô hình duyệt) | 2.434.560 | ≈ 20.992 |
| `redact::flatten` trả lỗi ngay (bỏ bộ tô che) | 2.449.920 | ≈ 5.632 |
| Phần còn lại (cổng `privacy_gate`, nối các đường xuất, chuỗi i18n, ô Tùy chọn, bước/Lịch sử, dòng GIF) | — | ≈ 46.080 (= 119.808 − 47.104 − 20.992 − 5.632) |

Số theo module là xấp xỉ (cắt một hàm vào làm trình liên kết bỏ mọi mã chỉ nó gọi; phần dùng chung được tính cho module bị cắt).

Không có `cargo bloat` (không cài mới).

## 6. JPEG sau che (giải mã bằng Windows)

`privacy_gate::tests::jpeg_redaction_stays_black_after_windows_decode`: ảnh 320×200 giống chữ (nền trắng, nét đen), 3 vùng che, `encode_jpeg` chất lượng 90 và 50, giải mã bằng `Windows.Graphics.Imaging.BitmapDecoder` (WinRT, dựa trên WIC; dùng feature `Graphics_Imaging` sẵn có, không thêm `Win32_Graphics_Imaging`). Kênh lớn nhất trong vùng che (sau khi co vào):

| Chất lượng | Co vào 4 px | Co vào 8 px | Co vào 12 px |
|---|---|---|---|
| 90 | 7 | 0 | 0 |
| 50 | 26 | 0 | 0 |

Chất lượng 50, mép trong 4 px vượt ngưỡng 8 của gói task: đó là nhiễu nén từ nền trắng bên ngoài cùng khối 8×8, không phải chữ đã che. Kiểm thử lõi `redact::tests::jpeg_bytes_do_not_depend_on_covered_pixels` khẳng định hai ảnh chỉ khác nhau trong vùng che cho cùng byte PNG và JPEG (90, 50) sau khi làm phẳng, tức tệp xuất không mang thông tin nào của phần bị che.
