# Memory bank

Bộ nhớ sống của dự án. Phiên mới chỉ đọc `activeContext.md` và `progress.md`; các file khác đọc khi cần.

| File | Nội dung | Khi nào đọc | Khi nào cập nhật |
|---|---|---|---|
| `activeContext.md` | Task đang làm, nhánh, bước kế tiếp, việc đang chờ chủ dự án | Mọi phiên, đọc đầu tiên | Đầu và cuối mỗi task |
| `progress.md` | Bảng mọi task, trạng thái, sổ token | Mọi phiên | Cuối mỗi task |
| `projectbrief.md` | Mục đích, mục tiêu, ngoài phạm vi, điều phải tránh | Khi xét phạm vi | Khi phạm vi đổi |
| `productContext.md` | Vì sao có dự án, người dùng, hành vi mong muốn | Task có giao diện hoặc hành vi | Khi hành vi đổi |
| `systemPatterns.md` | Kiến trúc, các trait, quy ước mã | Task viết mã | Khi thêm mẫu hoặc trait mới |
| `techContext.md` | Công cụ, crate, lệnh build và kiểm tra | Task viết mã | Khi thêm phụ thuộc hoặc đổi công cụ |
| `decisions.md` | Sổ quyết định đã chốt | Trước khi đề xuất thay đổi | Mỗi khi có quyết định mới |

Mỗi file giữ trong khoảng một trang. Chi tiết nằm ở spec và các gói task; ở đây chỉ ghi điều cần để làm tiếp.
