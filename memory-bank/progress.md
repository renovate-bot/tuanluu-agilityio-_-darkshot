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
| 8 Chụp màn hình và lớp phủ | task/08-capture-overlay | specialist | opus | đang làm | | | | |
| 9 Dịch vụ Windows | task/09-win-services | specialist | opus | chờ | | | | |
| 10 Cửa sổ Cài đặt | task/10-settings | builder | sonnet | chờ | | | | |
| 11 Nối toàn bộ | task/11-integration | builder | sonnet | chờ | | | | |
| 12 Tài liệu và phát hành | task/12-docs-release | scaffolder | haiku | chờ | | | | |
| Rà soát toàn bộ sau Task 12 | main | specialist | opus | chờ | | | | |

Rà soát Task 4, 8, 9 dùng `specialist` (opus); các task khác dùng `reviewer` (sonnet).

Ghi chú về "Token sửa": khi một agent được gọi tiếp để sửa, công cụ báo một con số duy nhất cho agent đó (Task 3: 69.217 sau lượt sửa, so với 66.710 sau lượt làm). Sổ này coi con số đó là cộng dồn và ghi phần chênh lệch vào cột "Token sửa". Đây là giả định, chưa kiểm chứng; nếu con số là riêng của lượt sửa thì Token sửa của Task 3 là 69.217. Task 4: agent làm báo 58.879 (lượt bị dừng vì không truy cập được tài liệu AWS), 73.515 (sau khi làm xong), 79.057 (sau lượt sửa); mức tăng nhỏ giữa các lượt cho thấy con số có thể là kích thước ngữ cảnh cuối của agent, thấp hơn lượng token thực tính tiền. Coi bảng này là số tương đối giữa các task.

## Token trước giai đoạn xây dựng (giai đoạn 1 đến 4)

| Việc | Agent | Token |
|---|---|---|
| Rà soát bản thiết kế | general-purpose | 49.194 |
| Tra tài liệu Claude Code (Remote Control, phiên đám mây) | claude-code-guide | 56.113 |
| **Cộng** | | **105.307** |

## Tổng kết (lập khi dự án xong)

Theo task, theo model, theo giai đoạn.
