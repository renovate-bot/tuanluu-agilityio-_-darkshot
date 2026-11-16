# Tiến độ và sổ token

Cập nhật: 2026-11-16

Trạng thái: `chờ`, `đang làm`, `rà soát`, `xong`. Token là số do agent con báo khi kết thúc. Token của phiên điều phối không đo được theo task.

## Bảng task

| Task | Nhánh | Agent | Model | Trạng thái | Token làm | Token rà soát | Token sửa | Tổng |
|---|---|---|---|---|---|---|---|---|
| 0 Môi trường, git, khung AI-first | main | phiên chính | fable | xong | không đo | | | |
| 1 Khung workspace | task/01-workspace | scaffolder | haiku | xong | 55.218 | 16.463 | 0 | 71.681 |
| 2 Hình học vùng chọn | task/02-geometry | builder | sonnet | xong | 58.497 | 28.360 | 0 | 86.857 |
| 3 Đặt tên và cấu hình | task/03-naming-config | builder | sonnet | chờ | | | | |
| 4 Chữ ký SigV4 | task/04-sigv4 | specialist | opus | chờ | | | | |
| 5 Tải lên R2 | task/05-upload | builder | sonnet | chờ | | | | |
| 6 PNG và luồng hành động | task/06-image-actions | builder | sonnet | chờ | | | | |
| 7 Vỏ ứng dụng Windows | task/07-win-shell | builder | sonnet | chờ | | | | |
| 8 Chụp màn hình và lớp phủ | task/08-capture-overlay | specialist | opus | chờ | | | | |
| 9 Dịch vụ Windows | task/09-win-services | specialist | opus | chờ | | | | |
| 10 Cửa sổ Cài đặt | task/10-settings | builder | sonnet | chờ | | | | |
| 11 Nối toàn bộ | task/11-integration | builder | sonnet | chờ | | | | |
| 12 Tài liệu và phát hành | task/12-docs-release | scaffolder | haiku | chờ | | | | |
| Rà soát toàn bộ sau Task 12 | main | specialist | opus | chờ | | | | |

Rà soát Task 4, 8, 9 dùng `specialist` (opus); các task khác dùng `reviewer` (sonnet).

## Token trước giai đoạn xây dựng (giai đoạn 1 đến 4)

| Việc | Agent | Token |
|---|---|---|
| Rà soát bản thiết kế | general-purpose | 49.194 |
| Tra tài liệu Claude Code (Remote Control, phiên đám mây) | claude-code-guide | 56.113 |
| **Cộng** | | **105.307** |

## Tổng kết (lập khi dự án xong)

Theo task, theo model, theo giai đoạn.
