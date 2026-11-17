# darkshot

Ứng dụng Windows chạy ở khay hệ thống: chụp vùng màn hình, lưu PNG, tải lên Cloudflare R2 của chủ dự án và chép link có hạn. Viết bằng Rust thuần. Giao diện luôn tối.

## Bắt đầu phiên làm việc

1. Đọc `memory-bank/activeContext.md` rồi `memory-bank/progress.md`. Hai file này đủ để biết đang ở đâu.
2. Chỉ đọc các file memory bank khác khi việc đang làm cần đến (bảng trong `memory-bank/README.md`).
3. Chạy task kế tiếp bằng `/next-task`. Xem tiến độ và token bằng `/status`.

Không đọc lại toàn bộ kế hoạch hay spec khi không cần.

## Luật chung

- Mỗi task làm trên nhánh `task/NN-ten-ngan`, do một agent con thực hiện theo gói `docs/tasks/NN-ten-ngan.md`. Phiên chính chỉ điều phối và rà soát.
- Vai trò agent và model nằm trong `.claude/agents/`. Không tự đổi model của một task.
- Trước khi gộp nhánh: `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.
- Sau mỗi task: ghi token vào `memory-bank/progress.md`, cập nhật `memory-bank/activeContext.md`.
- Spec `docs/superpowers/specs/2026-11-16-shotme-design.md` đã duyệt, không sửa. Thay đổi về sau ghi vào `memory-bank/decisions.md`.
- Đợt 1 (menu mới, Tùy chọn dạng tab, hai ngôn ngữ): thiết kế `docs/superpowers/specs/2026-11-16-options-tabs-i18n-design.md`, kế hoạch `docs/superpowers/plans/2026-11-16-options-tabs-i18n.md`. Thiết kế đã duyệt cũng không sửa.
- Đợt 2 (phím tắt toàn màn hình, con trỏ trong ảnh, giữ vị trí vùng, bật/tắt thông báo): thiết kế `docs/superpowers/specs/2026-11-16-capture-batch2-design.md`, kế hoạch `docs/superpowers/plans/2026-11-16-capture-batch2.md`. Thiết kế đã duyệt cũng không sửa.
- Đợt 3 (định dạng ảnh tải lên: PNG hoặc JPEG): thiết kế `docs/superpowers/specs/2026-11-16-upload-format-design.md`, kế hoạch `docs/superpowers/plans/2026-11-16-upload-format.md`. Thiết kế đã duyệt cũng không sửa.
- Đợt 4 (đổi tên darkshot, giao diện luôn tối): thiết kế `docs/superpowers/specs/2026-11-16-darkshot-dark-theme-design.md`, kế hoạch `docs/superpowers/plans/2026-11-16-darkshot-dark-theme.md`. Thiết kế đã duyệt cũng không sửa.
- Đợt 5 (công cụ đánh dấu trên ảnh chụp): thiết kế `docs/superpowers/specs/2026-11-17-annotate-design.md`, kế hoạch `docs/superpowers/plans/2026-11-17-annotate.md`. Thiết kế đã duyệt cũng không sửa.
- Đợt 6 (Annotation Pro: hoàn tác/làm lại nhiều bước, chọn và sửa hình, Hình elip, Đánh số, mức cỡ): thiết kế `docs/superpowers/specs/2026-11-17-annotation-pro-design.md`, kế hoạch `docs/superpowers/plans/2026-11-17-annotation-pro.md`. Thiết kế đã duyệt cũng không sửa.
- Mọi chữ người dùng nhìn thấy lấy từ bảng `darkshot_core::i18n` (Tiếng Việt và English), không viết cứng trong mã.
- Không bàn lại quyết định đã có trong `memory-bank/decisions.md` nếu chủ dự án không yêu cầu.
- Chỉ đẩy mã lên remote `origin` (repo GitHub riêng tư của chủ dự án). Không gửi dữ liệu dự án ra dịch vụ nào khác.
- Không bao giờ yêu cầu, đọc hay ghi lại khóa R2 của chủ dự án.
- `prototype/` là bản giao diện dùng thử; không dùng lại mã của nó trong ứng dụng.
- Trao đổi với chủ dự án bằng tiếng Việt.

## Ràng buộc kỹ thuật

Danh sách đầy đủ ở mục "Global Constraints" của `docs/superpowers/plans/2026-11-16-shotme.md`; từ Task 15 dùng mục cùng tên của `docs/superpowers/plans/2026-11-16-options-tabs-i18n.md`; từ Task 20 dùng mục cùng tên của `docs/superpowers/plans/2026-11-16-capture-batch2.md`; từ Task 24 dùng mục cùng tên của `docs/superpowers/plans/2026-11-16-upload-format.md`; từ Task 27 dùng mục cùng tên của `docs/superpowers/plans/2026-11-16-darkshot-dark-theme.md`; từ Task 34 dùng mục cùng tên của `docs/superpowers/plans/2026-11-17-annotate.md`; từ Task 44 dùng mục cùng tên của `docs/superpowers/plans/2026-11-17-annotation-pro.md`. Mỗi gói task đã chép sẵn mục này.
