---
description: Chạy trọn task kế tiếp của darkshot - nhánh, agent, rà soát, gộp, ghi token, cập nhật memory bank
---

Chạy task kế tiếp của dự án darkshot. Bạn là phiên điều phối: không tự viết mã của task.

1. Đọc `memory-bank/activeContext.md` và `memory-bank/progress.md`. Xác định task kế tiếp (dòng `chờ` đầu tiên, hoặc task đang dở). Nếu mục "Đang chờ chủ dự án" có việc chặn task này, dừng và hỏi chủ dự án.
2. Kiểm tra `git status` sạch và đang ở `main`. Nếu task đang dở trên một nhánh, chuyển sang nhánh đó và làm tiếp.
3. Ghi vào `activeContext.md`: task đang làm, nhánh, trạng thái `đang làm`. Đổi trạng thái trong `progress.md`. Commit hai file trên `main`.
4. `git switch -c task/NN-ten-ngan main`.
5. Giao việc cho agent ghi ở cột Agent của `progress.md`, lời giao: "Thực hiện gói task `docs/tasks/NN-ten-ngan.md`. Bạn đang ở nhánh `task/NN-ten-ngan`." Nếu loại agent đó chưa có trong phiên, dùng agent `general-purpose` với model ở cột Model và dán nội dung file `.claude/agents/<tên>.md` vào đầu lời giao.
6. Khi agent báo xong, ghi lại số token nó báo (dòng `subagent_tokens`). Nếu agent báo vướng, xử lý: trả lời nếu gói task hoặc spec có đáp án, hỏi chủ dự án nếu không.
7. Giao rà soát cho `reviewer` (Task 4, 8, 9, 17, 21, 22 và rà soát toàn bộ: `specialist`), lời giao: "Rà soát nhánh `task/NN-ten-ngan` theo gói task `docs/tasks/NN-ten-ngan.md`." Ghi số token.
8. Nếu kết quả là `CẦN SỬA` với lỗi `chặn`: giao lại cho agent làm kèm danh sách lỗi, ghi token vào cột "Token sửa", rồi rà soát lại. Tối đa ba vòng; quá thì dừng và báo chủ dự án.
9. Chạy `powershell -File scripts/check.ps1`. Phải thấy `ALL CHECKS PASSED`.
10. `git switch main` rồi `git merge --no-ff task/NN-ten-ngan`.
11. Cập nhật `memory-bank/progress.md` (trạng thái `xong`, các cột token, cột Tổng) và `memory-bank/activeContext.md` (task kế tiếp, việc chờ chủ dự án). Nếu task sinh quyết định, mẫu hoặc phụ thuộc mới: cập nhật `decisions.md`, `systemPatterns.md`, `techContext.md`. Commit trên `main`.
12. Đẩy lên GitHub: `git push origin main task/NN-ten-ngan`. Trước khi đẩy, xác nhận repo vẫn riêng tư (`gh repo view --json visibility`); nếu không còn riêng tư thì dừng và hỏi chủ dự án. Chạy lệnh đẩy một mình, đúng dạng `git push origin ...`, không ghép với `cd` hay lệnh khác bằng `&&`: quy tắc quyền `Bash(git push origin:*)` chỉ khớp khi lệnh bắt đầu bằng `git push origin`. Nếu vẫn bị chặn quyền, nhờ chủ dự án tự chạy.
13. Báo chủ dự án bằng tiếng Việt, ngắn gọn: task nào xong, kết quả kiểm tra, token đã dùng, việc cần họ thử bằng tay nếu có, task kế tiếp là gì.

Sau Task 8, Task 12, Task 18 và Task 22, dừng lại để chủ dự án thử theo `docs/testing/manual-checklist.md` trước khi chạy task tiếp theo.

Task 15–19 (đợt 1) theo kế hoạch `docs/superpowers/plans/2026-11-16-options-tabs-i18n.md` và thiết kế `docs/superpowers/specs/2026-11-16-options-tabs-i18n-design.md`. Rà soát các task này đối chiếu thêm mục "Review Focus" của kế hoạch đó.

Task 24–26 (đợt 3) theo kế hoạch `docs/superpowers/plans/2026-11-16-upload-format.md` và thiết kế `docs/superpowers/specs/2026-11-16-upload-format-design.md`; rà soát đối chiếu thêm mục "Review Focus" của kế hoạch đó. Task 25 do `specialist` rà soát. Không dừng sau Task 25 (quyết định 42).

Task 20–23 (đợt 2) theo kế hoạch `docs/superpowers/plans/2026-11-16-capture-batch2.md` và thiết kế `docs/superpowers/specs/2026-11-16-capture-batch2-design.md`. Rà soát các task này đối chiếu thêm mục "Review Focus" của kế hoạch đó.

Task 27–31 (đợt 4) theo kế hoạch `docs/superpowers/plans/2026-11-16-darkshot-dark-theme.md` và thiết kế `docs/superpowers/specs/2026-11-16-darkshot-dark-theme-design.md`; rà soát đối chiếu thêm mục "Review Focus" của kế hoạch đó. Task 28 và 30 do `specialist` rà soát. Không dừng sau Task 30 (quyết định 48).

Task 34–38 (đợt 5) theo kế hoạch `docs/superpowers/plans/2026-11-17-annotate.md` và thiết kế `docs/superpowers/specs/2026-11-17-annotate-design.md`; Task 35, 36, 37 do `specialist` rà soát. Chạy đến hết không dừng (quyết định 58).
