# Tiến độ và sổ token

Cập nhật: 2026-11-16

Trạng thái: `chờ`, `đang làm`, `rà soát`, `xong`. Token là số do agent con báo khi kết thúc. Token của phiên điều phối không đo được theo task.

## Bảng task

| Task | Nhánh | Agent | Model | Trạng thái | Token làm | Token rà soát | Token sửa | Tổng |
|---|---|---|---|---|---|---|---|---|
| 0 Môi trường, git, khung AI-first | main | phiên chính | fable | xong | không đo | | | |
| 1 Khung workspace | task/01-workspace | scaffolder | haiku | xong | 55.218 | 16.463 | 0 | 71.681 |
| 2 Hình học vùng chọn | task/02-geometry | builder | sonnet | xong | 58.497 | 28.360 | 0 | 86.857 |
| 3 Đặt tên và cấu hình | task/03-naming-config | builder | sonnet | xong | 66.710 | 30.618 | 2.507 (xem ghi chú) | 99.835 |
| 4 Chữ ký SigV4 | task/04-sigv4 | specialist | opus | xong | 73.515 | 56.402 | 5.542 | 135.459 |
| 5 Tải lên R2 | task/05-upload | builder | sonnet | xong | 72.426 | 33.228 | 6.043 | 111.697 |
| 6 PNG và luồng hành động | task/06-image-actions | builder | sonnet | xong | 63.192 | 35.413 | 8.253 | 106.858 |
| 7 Vỏ ứng dụng Windows | task/07-win-shell | builder | sonnet | xong | 74.617 | 36.916 | 15.158 | 126.691 |
| 8 Chụp màn hình và lớp phủ | task/08-capture-overlay | specialist | opus | xong, chưa thử tay | 185.819 | 193.926 | 55.331 | 435.076 |
| 9 Dịch vụ Windows | task/09-win-services | specialist | opus | xong | 134.713 | 73.546 | 0 | 208.259 |
| 10 Cửa sổ Cài đặt | task/10-settings | builder | sonnet | xong | 118.217 | 67.217 | 37.831 | 223.265 |
| 11 Nối toàn bộ | task/11-integration | builder | sonnet | xong | 125.499 | 58.921 | 15.661 | 200.081 |
| 14 Biểu tượng ứng dụng mới | task/14-icon | builder | sonnet | xong | 71.041 | 0 (phiên chính xem ảnh, không rà soát mã) | 8.117 | 79.158 |
| 12 Tài liệu và phát hành | task/12-docs-release | scaffolder | haiku | xong, chờ nghiệm thu | 97.632 | 53.365 | 0 (phiên chính sửa README) | 150.997 |
| Rà soát toàn bộ sau Task 12 | main | specialist | opus | xong (DUYỆT) | | 189.767 | | 189.767 |
| 13 Gia cố sau rà soát toàn bộ | task/13-hardening | specialist | opus | xong | 199.988 | 93.033 | 0 | 293.021 |
| 15 Bảng chuỗi và ngôn ngữ ở core | task/15-i18n-core | builder | sonnet | xong | 109.936 | 53.801 | 0 | 163.737 |
| 16 Lớp Windows dùng bảng chuỗi | task/16-i18n-windows | builder | sonnet | xong | 148.870 | 78.526 | 7.629 | 235.025 |
| 17 Cửa sổ Tùy chọn dạng tab | task/17-options-tabs | specialist | opus | xong | 233.114 | 168.494 | 11.798 | 413.406 |
| 18 Menu, Thông tin, Trợ giúp | task/18-menu-about-help | builder | sonnet | xong | 178.194 | 72.826 | 0 | 251.020 |
| 19 Tài liệu đợt 1 | task/19-docs-batch1 | scaffolder | haiku | xong | 177.000 | 74.920 | 11.776 | 263.696 |
| 19b Kiểm thử chập chờn, chữ "Đổi…" trong Trợ giúp | task/19b-flaky-test-help-text | builder | sonnet | xong | 53.642 | 0 (phiên chính đọc diff 18 dòng, không giao rà soát) | 0 | 53.642 |
| 20 Lõi đợt 2 | task/20-batch2-core | builder | sonnet | xong | 121.832 | 41.358 | 0 | 163.190 |
| 21 Tùy chọn: ba phím tắt, ba ô tích, lọc thông báo, Trợ giúp | task/21-hotkeys-options | specialist | opus | xong | 223.262 | 127.269 | 21.840 | 372.371 |
| 22 Chụp: con trỏ, toàn màn hình, khung nhớ | task/22-fullscreen-cursor-selection | specialist | opus | xong | 165.719 | 108.270 | 6.482 | 280.471 |
| 23 Tài liệu đợt 2 | task/23-docs-batch2 | scaffolder | haiku | xong | 199.156 | 65.960 | 8.601 | 273.717 |
| 24 Lõi đợt 3: định dạng tải lên, bộ nén JPEG | task/24-upload-format-core | builder | sonnet | xong | 143.728 | 46.466 | 3.060 | 193.254 |
| 25 Tab Định dạng | task/25-format-tab | specialist | opus | xong | 184.196 | 118.542 | 4.661 | 307.399 |
| 26 Tài liệu đợt 3 | task/26-docs-batch3 | scaffolder | haiku | xong | 140.579 | 44.680 | 0 | 185.259 |
| 27 Đổi tên darkshot | task/27-rename-darkshot | builder | sonnet | xong | 72.367 | 36.526 | 0 | 108.893 |
| 28 Chuyển dữ liệu từ shotme | task/28-legacy-migration | specialist | opus | xong | 133.605 | 89.121 | 6.665 | 229.391 |
| 29 Lõi giao diện tối, Thông tin, Trợ giúp, menu khay, lớp phủ | task/29-dark-theme-core | builder | sonnet | xong | 95.280 | 44.739 | 2.387 | 142.406 |
| 30 Tùy chọn tự vẽ | task/30-dark-options | specialist | opus | xong | 308.654 | 146.553 | 3.810 | 459.017 |
| 31 Tài liệu đợt 4 | task/31-docs-batch4 | scaffolder | haiku | xong | 171.829 | 50.900 | 0 (phiên điều phối sửa một câu README) | 222.729 |
| 30b Ô tích, nút chọn mịn | task/30b-check-radio-aa | specialist | opus | xong | 146.263 | 67.580 | 0 (phiên điều phối dời một chú thích) | 213.843 |
| 30c Ô nhập một viền bo mịn | task/30c-edit-frame | specialist | opus | xong | 171.111 | 78.174 | 0 | 249.285 |
| 32 Biểu tượng tối (phương án B) | task/32-dark-icon | builder | sonnet | xong | 58.938 | 27.339 | 0 | 86.277 |
| 33 README (cỡ build, biểu tượng) | task/33-readme-refresh | phiên điều phối | opus | xong | 0 | 0 | 0 | 0 |
| 34 Mô hình đánh dấu ở core | task/34-annotate-core | builder | sonnet | xong | 99.922 | 47.328 | 0 | 147.250 |
| 35 Vẽ hình đánh dấu, in vào ảnh | task/35-annotate-render | specialist | opus | xong | 133.795 | 95.127 | 0 | 228.922 |
| 36 Thanh công cụ vẽ trong lớp phủ | task/36-annotate-overlay | specialist | opus | xong | 247.332 | 142.586 | 0 | 389.918 |
| 37 Công cụ Chữ | task/37-annotate-text | specialist | opus | xong | 250.948 | 107.379 | 9.966 | 368.293 |
| 38 Tài liệu đợt 5 | task/38-docs-batch5 | scaffolder | haiku | xong | 168.623 | 61.934 | 0 | 230.557 |
| 39 Bảng màu 48 ô | task/39-palette-48 | builder | sonnet | xong | 67.722 | 69.531 | 0 | 137.253 |
| 40 Mô hình màu tự chọn | task/40-color-model | builder | sonnet | đang làm | | | | |
| 41 Bảng chọn màu tự do | task/41-color-picker | specialist | opus | chờ | | | | |

Task 15–19 là đợt 1 (kế hoạch `docs/superpowers/plans/2026-11-16-options-tabs-i18n.md`). Cộng Task 15–19: 1.326.884 (làm 847.114; rà soát 448.567; sửa 31.203). Theo model: opus (Task 17) 413.406; sonnet (15, 16, 18 và rà soát 19) 724.702; haiku (làm và sửa 19) 188.776. Chủ dự án đã nghiệm thu đợt 1 qua thử tay (2026-11-16).

Task 20–23 là đợt 2 (kế hoạch `docs/superpowers/plans/2026-11-16-capture-batch2.md`). Trước khi xây: đọc soát thiết kế đợt 2 (general-purpose) 146.071. Cộng Task 20–23: 1.089.749 (làm 709.969; rà soát 342.857; sửa 36.923); kể cả đọc soát thiết kế: 1.235.820. Theo model: opus (Task 21, 22) 652.842; sonnet (Task 20 và rà soát 23) 229.150; haiku (làm và sửa 23) 207.757. Chủ dự án đã nghiệm thu đợt 2 qua thử tay (2026-11-16).

Task 24–26 là đợt 3 (kế hoạch `docs/superpowers/plans/2026-11-16-upload-format.md`). Trước khi xây: đọc soát thiết kế đợt 3 (general-purpose) 129.579. Cộng Task 24–26: 685.912 (làm 468.503; rà soát 209.688; sửa 7.721); kể cả đọc soát thiết kế: 815.491. Theo model: opus (Task 25) 307.399; sonnet (Task 24 và rà soát 26) 237.934; haiku (Task 26) 140.579. Chủ dự án báo "đã duyệt. Đúng yêu cầu" (2026-11-16).

Task 27–31 là đợt 4 (kế hoạch `docs/superpowers/plans/2026-11-16-darkshot-dark-theme.md`). Trước khi xây: đọc soát thiết kế đợt 4 (general-purpose) 65.075. Cộng Task 27–31: 1.162.436 (làm 781.735; rà soát 367.839; sửa 12.862); kể cả đọc soát thiết kế: 1.227.511. Theo model: opus (Task 28, 30) 688.408; sonnet (Task 27, 29 và rà soát 31) 302.199; haiku (Task 31) 171.829. Sửa sau đợt 4: Task 30b (chủ dự án báo ô tích và nút chọn không như mong đợi: răng cưa) 213.843; Task 30c (chủ dự án báo ô nhập trộn góc vuông và bo tròn) 249.285. Chưa thử tay, chưa nghiệm thu.

Rà soát Task 4, 8, 9, 17, 21, 22, 25, 28, 30 dùng `specialist` (opus); các task khác dùng `reviewer` (sonnet).

Ghi chú về "Token sửa": khi một agent được gọi tiếp để sửa, công cụ báo một con số duy nhất cho agent đó (Task 3: 69.217 sau lượt sửa, so với 66.710 sau lượt làm). Sổ này coi con số đó là cộng dồn và ghi phần chênh lệch vào cột "Token sửa". Đây là giả định, chưa kiểm chứng; nếu con số là riêng của lượt sửa thì Token sửa của Task 3 là 69.217. Task 4: agent làm báo 58.879 (lượt bị dừng vì không truy cập được tài liệu AWS), 73.515 (sau khi làm xong), 79.057 (sau lượt sửa); mức tăng nhỏ giữa các lượt cho thấy con số có thể là kích thước ngữ cảnh cuối của agent, thấp hơn lượng token thực tính tiền. Coi bảng này là số tương đối giữa các task.

## Token trước giai đoạn xây dựng (giai đoạn 1 đến 4)

| Việc | Agent | Token |
|---|---|---|
| Rà soát bản thiết kế | general-purpose | 49.194 |
| Tra tài liệu Claude Code (Remote Control, phiên đám mây) | claude-code-guide | 56.113 |
| **Cộng** | | **105.307** |

## Tổng kết (lập khi dự án xong)

Lập ngày 2026-11-16, sau Task 13. Số là `subagent_tokens` do công cụ báo cho từng agent con: số tương đối, không gồm phiên điều phối, có thể thấp hơn số tính tiền.

### Theo task

| Task | Tổng |
|---|---|
| 1 Khung workspace | 71.681 |
| 2 Vùng chọn | 86.857 |
| 3 Đặt tên, cấu hình | 99.835 |
| 4 Ký yêu cầu | 135.459 |
| 5 Tải lên R2 | 111.697 |
| 6 PNG, hành động | 106.858 |
| 7 Vỏ ứng dụng Windows | 126.691 |
| 8 Chụp màn hình và lớp phủ | 435.076 |
| 9 Dịch vụ Windows | 208.259 |
| 10 Cửa sổ Cài đặt | 223.265 |
| 11 Nối toàn bộ | 200.081 |
| 12 Tài liệu và phát hành | 150.997 |
| Rà soát toàn bộ | 189.767 |
| 13 Gia cố | 293.021 |
| Trước giai đoạn xây dựng | 105.307 |
| **Cộng đến Task 13** | **2.544.851** |
| 14 Biểu tượng (sau tổng kết; các bảng model và giai đoạn bên dưới chưa gồm) | 79.158 |
| **Cộng** | **2.624.009** |

### Theo model

| Model | Token | Tỉ lệ |
|---|---|---|
| Opus (Task 4, 8, 9, 13 và rà soát toàn bộ) | 1.261.582 | 50% |
| Sonnet (Task 2, 3, 5, 6, 7, 10, 11 và các lượt rà soát thường) | 1.025.112 | 40% |
| Haiku (làm Task 1 và 12) | 152.850 | 6% |
| Trước giai đoạn xây dựng (general-purpose, claude-code-guide) | 105.307 | 4% |

### Theo giai đoạn

| Giai đoạn | Token | Tỉ lệ |
|---|---|---|
| Làm | 1.326.043 | 52% |
| Rà soát (gồm rà soát toàn bộ) | 967.175 | 38% |
| Sửa sau rà soát | 146.326 | 6% |
| Trước giai đoạn xây dựng | 105.307 | 4% |

Nhận xét: Task 8 (lớp phủ) một mình chiếm 17%; rà soát chiếm gần hai phần năm và lượt nào cũng tìm ra điểm đáng sửa, không lượt nào tìm ra lỗi chặn.

Task 34–38 là đợt 5 (kế hoạch `docs/superpowers/plans/2026-11-17-annotate.md`). Trước khi xây: đọc soát thiết kế đợt 5 (general-purpose) 68.315. Cộng Task 34–38: 1.364.940 (làm 900.620; rà soát 454.354; sửa 9.966); kể cả đọc soát thiết kế: 1.433.255. Theo model: opus (Task 35, 36, 37) 987.133; sonnet (Task 34 và rà soát 34, 38) 209.184; haiku (Task 38) 168.623. Chưa thử tay, chưa nghiệm thu.
