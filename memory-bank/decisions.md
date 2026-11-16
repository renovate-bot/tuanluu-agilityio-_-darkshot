# Sổ quyết định

Quyết định đã chốt. Không bàn lại nếu chủ dự án không yêu cầu. Mục mới thêm ở cuối.

| # | Ngày | Quyết định | Lý do | Ai chốt |
|---|---|---|---|---|
| 1 | 2026-11-16 | Bản đầu chỉ có ba tính năng: chụp vùng, lưu local, chia sẻ link | Tập trung | Chủ dự án |
| 2 | 2026-11-16 | Ảnh chia sẻ lưu trên Cloudflare R2 đứng tên chủ dự án; ứng dụng tải thẳng lên, không máy chủ trung gian | Không muốn ảnh nằm trên máy chủ của Lightshot; trải nghiệm link trực tiếp | Chủ dự án |
| 3 | 2026-11-16 | Viết bằng Rust thuần, gọi API Windows | Gọn nhẹ mà đủ tính năng | Chủ dự án |
| 4 | 2026-11-16 | Thanh công cụ chỉ có biểu tượng; chữ hiện khi rê chuột | Phản hồi sau khi xem prototype | Chủ dự án |
| 5 | 2026-11-16 | Hạn link mặc định 30 ngày, chọn được hạn khác | Bảo mật | Chủ dự án |
| 6 | 2026-11-16 | Các hạn: 1 ngày, 7 ngày, 30 ngày, không hết hạn; đặt trong Cài đặt, áp dụng cho mọi lần tải lên sau đó | Đơn giản | Đề xuất, chủ dự án duyệt trong spec |
| 7 | 2026-11-16 | Hết hạn bằng quy tắc vòng đời của R2 theo thư mục `d1/`, `d7/`, `d30/`; chủ dự án tạo quy tắc bằng tay | Ảnh bị xóa thật; khóa của ứng dụng chỉ cần quyền đọc và ghi đối tượng | Đề xuất, chủ dự án duyệt trong spec |
| 8 | 2026-11-16 | HTTPS dùng `rustls` với chứng chỉ gốc đóng kèm, không dùng TLS của Windows | Windows không tự gọi máy chủ bên thứ ba khi kiểm tra chứng chỉ | Đề xuất, chủ dự án duyệt trong spec |
| 9 | 2026-11-16 | Agent không nhận khóa R2; chủ dự án tự nhập và tự nghiệm thu với kho thật | Bảo mật | Đề xuất, chủ dự án duyệt trong spec |
| 10 | 2026-11-16 | Được cài Rust và Visual Studio Build Tools lên máy phát triển | Cần để build | Chủ dự án |
| 11 | 2026-11-16 | Quy trình: SDLC tám giai đoạn, có prototype, mỗi task một nhánh, ghi token từng task | Yêu cầu quy trình | Chủ dự án |
| 12 | 2026-11-16 | Kiến trúc hai lớp: core không phụ thuộc hệ điều hành, lớp nền tảng riêng | Dễ mở rộng, có thể thêm macOS và Linux | Chủ dự án yêu cầu; cách chia là đề xuất |
| 13 | 2026-11-16 | Tổ chức AI-first: `memory-bank/`, vai trò agent trong `.claude/agents/`, gói task trong `docs/tasks/`, lệnh `/next-task` | Yêu cầu của chủ dự án | Chủ dự án |
| 14 | 2026-11-16 | `memory-bank/` thay cho `docs/progress/STATE.md` và `tasks.md` ghi ở spec mục 11.3; sổ token nằm trong `memory-bank/progress.md` | Theo mẫu memory bank chủ dự án nêu | Chủ dự án |
| 15 | 2026-11-16 | Phiên mới chỉ đọc `activeContext.md` và `progress.md`, không đọc cả memory bank | Mở lại không phải đọc lại toàn bộ | Đề xuất, chủ dự án chưa phản đối |
| 16 | 2026-11-16 | Làm tiếp từ claude.ai bằng Remote Control; không đẩy mã lên GitHub hay phiên đám mây | Mã và công cụ build ở lại máy Windows | Chủ dự án |
| 17 | 2026-11-16 | Mã được đẩy lên repo GitHub riêng tư `tuanluu-agilityio/shotme` (remote `origin`): `main` và mọi nhánh `task/...`. Thay phần "không đẩy lên GitHub" của mục 16; Remote Control vẫn là cách làm tiếp từ claude.ai | Chủ dự án cung cấp repo; chuyển sang riêng tư vì lý do bảo mật | Chủ dự án |
| 18 | 2026-11-16 | Phiên điều phối tự chạy nối tiếp các task, không hỏi lại sau mỗi task; chỉ dừng ở điểm cần chủ dự án thử tay hoặc quyết định | Chủ dự án không muốn bị hỏi lại | Chủ dự án |
| 19 | 2026-11-16 | Quy tắc quyền `Bash(git push origin:*)` trong `.claude/settings.local.json` (không đưa lên git) cho phép phiên điều phối tự đẩy lên `origin` | Bỏ bước chủ dự án phải tự chạy lệnh đẩy | Chủ dự án |
| 20 | 2026-11-16 | `overlay::pick_region` trả `Result<Option<(Action, Rect)>, String>` thay cho `Option` trong kế hoạch | Lỗi mở lớp phủ phải có thông báo, không được trông như người dùng hủy | Phiên điều phối, sau rà soát Task 8 |
| 21 | 2026-11-16 | Chụp từ menu khay hoãn 250 ms; Esc là phím tắt toàn cục tạm trong lúc lớp phủ mở; mọi lối chụp đi qua một cổng trạng thái duy nhất | Menu không lọt vào ảnh; lớp phủ không bao giờ kẹt hay mở hai lần | Phiên điều phối, sau rà soát Task 8 |
