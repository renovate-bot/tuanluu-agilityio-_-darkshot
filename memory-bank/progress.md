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
| 17 Cửa sổ Tùy chọn dạng tab | task/17-options-tabs | specialist | opus | xong, chưa thử tay | 233.114 | 168.494 | 11.798 | 413.406 |
| 18 Menu, Thông tin, Trợ giúp | task/18-menu-about-help | builder | sonnet | đang làm | | | | |
| 19 Tài liệu đợt 1 | task/19-docs-batch1 | scaffolder | haiku | chờ (sau khi chủ dự án thử tay Task 18) | | | | |

Task 15–19 là đợt 1 (kế hoạch `docs/superpowers/plans/2026-11-16-options-tabs-i18n.md`).

Rà soát Task 4, 8, 9, 17 dùng `specialist` (opus); các task khác dùng `reviewer` (sonnet).

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
