---
description: Chạy trọn task kế tiếp của darkshot - nhánh, agent, rà soát, gộp, ghi token, cập nhật memory bank
---

Chạy task kế tiếp của dự án darkshot. Bạn là phiên điều phối: không tự viết mã của task.

1. Đọc `memory-bank/activeContext.md` và `memory-bank/progress.md`. Xác định task kế tiếp (dòng `chờ` đầu tiên, hoặc task đang dở). Nếu mục "Đang chờ chủ dự án" có việc chặn task này, dừng và hỏi chủ dự án.
2. Kiểm tra `git status` sạch. Xác định nhánh gốc của task: nhánh phát triển của đợt nếu đợt có (ví dụ `feature/window-snap`, xem `activeContext.md`), nếu không thì `main`. Đứng ở nhánh gốc đó. Nếu task đang dở trên một nhánh, chuyển sang nhánh đó và làm tiếp.
3. Ghi vào `activeContext.md`: task đang làm, nhánh, trạng thái `đang làm`. Đổi trạng thái trong `progress.md`. Commit hai file trên nhánh gốc.
4. `git switch -c task/NN-ten-ngan <nhánh gốc>`.
5. Chọn agent và model. Agent theo cột Agent của kế hoạch đợt. **Model là quyết định của phiên điều phối cho từng task** (CLAUDE.md, "Agent và model"): mặc định theo cột Model của kế hoạch; đánh giá lại nếu độ khó khác dự kiến (nâng model khi agent vướng về kiến trúc hoặc ba vòng sửa chưa qua). Giao bằng công cụ Agent với `subagent_type` = agent và tham số `model` = model đã chọn (`fable`, `opus`, `sonnet`, `haiku`); model trong `.claude/agents/<tên>.md` chỉ là mặc định khi không truyền. Lời giao: "Thực hiện gói task `docs/tasks/NN-ten-ngan.md`. Bạn đang ở nhánh `task/NN-ten-ngan`." Nếu loại agent đó chưa có trong phiên, dùng `general-purpose` với model đã chọn và dán nội dung `.claude/agents/<tên>.md` vào đầu lời giao. Ghi model thực dùng vào cột Model của `progress.md`.
6. Khi agent báo xong, ghi lại số token nó báo (dòng `subagent_tokens`). Nếu agent báo vướng, xử lý: trả lời nếu gói task hoặc spec có đáp án, hỏi chủ dự án nếu không.
7. Giao rà soát cho agent rà soát ghi trong kế hoạch đợt (luôn khác agent đã làm; task rủi ro cao dùng model mạnh bằng hoặc hơn model đã làm), truyền `model` như bước 5, lời giao: "Rà soát nhánh `task/NN-ten-ngan` theo gói task `docs/tasks/NN-ten-ngan.md`." Ghi số token.
8. Nếu kết quả là `CẦN SỬA` với lỗi `chặn`: giao lại cho agent làm kèm danh sách lỗi, ghi token vào cột "Token sửa", rồi rà soát lại. Tối đa ba vòng; quá thì dừng và báo chủ dự án.
9. Chạy `powershell -File scripts/check.ps1`. Phải thấy `ALL CHECKS PASSED`.
10. `git switch <nhánh gốc>` rồi `git merge --no-ff task/NN-ten-ngan`.
11. Cập nhật `memory-bank/progress.md` (trạng thái `xong`, các cột token, cột Tổng) và `memory-bank/activeContext.md` (task kế tiếp, việc chờ chủ dự án). Nếu task sinh quyết định, mẫu hoặc phụ thuộc mới: cập nhật `decisions.md`, `systemPatterns.md`, `techContext.md`. Commit trên nhánh gốc. Nếu task đổi điều người dùng thấy được: cập nhật `website/` theo luật "Website" trong CLAUDE.md và chạy `npm run build` ở đó.
12. Đẩy lên GitHub: `git push origin <nhánh gốc> task/NN-ten-ngan`. Khi nhánh gốc là nhánh phát triển của đợt, KHÔNG đẩy hay gộp vào `main`: chỉ gộp sau khi chủ dự án duyệt demo. Trước khi đẩy, nhớ repo đang CÔNG KHAI (chủ dự án xác nhận 2026-11-18): kiểm tra diff không chứa khóa R2, cấu hình thật hay dữ liệu cá nhân; nếu `gh repo view --json visibility` báo khác `PUBLIC` thì dừng và hỏi chủ dự án. Chạy lệnh đẩy một mình, đúng dạng `git push origin ...`, không ghép với `cd` hay lệnh khác bằng `&&`: quy tắc quyền `Bash(git push origin:*)` chỉ khớp khi lệnh bắt đầu bằng `git push origin`. Nếu vẫn bị chặn quyền, nhờ chủ dự án tự chạy.
13. Báo chủ dự án bằng tiếng Việt, ngắn gọn: task nào xong, kết quả kiểm tra, token đã dùng, việc cần họ thử bằng tay nếu có, task kế tiếp là gì.

Lịch sử (đã qua, không còn áp dụng): sau Task 8, Task 12, Task 18 và Task 22 từng dừng lại để chủ dự án thử theo `docs/testing/manual-checklist.md` trước khi chạy task tiếp theo.

Task 15–19 (đợt 1) theo kế hoạch `docs/superpowers/plans/2026-11-16-options-tabs-i18n.md` và thiết kế `docs/superpowers/specs/2026-11-16-options-tabs-i18n-design.md`. Rà soát các task này đối chiếu thêm mục "Review Focus" của kế hoạch đó.

Task 24–26 (đợt 3) theo kế hoạch `docs/superpowers/plans/2026-11-16-upload-format.md` và thiết kế `docs/superpowers/specs/2026-11-16-upload-format-design.md`; rà soát đối chiếu thêm mục "Review Focus" của kế hoạch đó. Task 25 do `specialist` rà soát. Không dừng sau Task 25 (quyết định 42).

Task 20–23 (đợt 2) theo kế hoạch `docs/superpowers/plans/2026-11-16-capture-batch2.md` và thiết kế `docs/superpowers/specs/2026-11-16-capture-batch2-design.md`. Rà soát các task này đối chiếu thêm mục "Review Focus" của kế hoạch đó.

Task 27–31 (đợt 4) theo kế hoạch `docs/superpowers/plans/2026-11-16-darkshot-dark-theme.md` và thiết kế `docs/superpowers/specs/2026-11-16-darkshot-dark-theme-design.md`; rà soát đối chiếu thêm mục "Review Focus" của kế hoạch đó. Task 28 và 30 do `specialist` rà soát. Không dừng sau Task 30 (quyết định 48).

Task 34–38 (đợt 5) theo kế hoạch `docs/superpowers/plans/2026-11-17-annotate.md` và thiết kế `docs/superpowers/specs/2026-11-17-annotate-design.md`; Task 35, 36, 37 do `specialist` rà soát. Chạy đến hết không dừng (quyết định 58).

Task 44–49 (đợt 6) theo kế hoạch `docs/superpowers/plans/2026-11-17-annotation-pro.md` và thiết kế `docs/superpowers/specs/2026-11-17-annotation-pro-design.md`; Task 45, 46, 47, 48 do `specialist` rà soát. Chạy đến hết không dừng (quyết định 75).

Task 50 (thanh công cụ gọn, thêm sau đợt 6) theo thiết kế `docs/superpowers/specs/2026-11-17-compact-toolbar-design.md`, `specialist` làm và rà soát. Task 51: cập nhật tài liệu cuối đợt 6. Đợt 6 đã xong và được chủ dự án duyệt; task mới cần thiết kế mới.

Task 52–58 (đợt 7, Lịch sử ảnh chụp) theo kế hoạch `docs/superpowers/plans/2026-11-18-screenshot-history.md` và thiết kế `docs/superpowers/specs/2026-11-18-screenshot-history-design.md`; Task 53, 54, 55, 56, 57 do `specialist` làm và rà soát. Đợt 7 làm trên nhánh phát triển `feature/screenshot-history` (các nhánh task gộp vào đó) và chỉ gộp vào `main` sau khi chủ dự án duyệt demo (quyết định 86); đã duyệt và gộp 2026-11-18. Task mới sau đợt 7 cần thiết kế mới.

Task 59–63 (đợt 8: gia cố Lịch sử, bắt cửa sổ) theo kế hoạch `docs/superpowers/plans/2026-11-18-window-snap.md` (bảng có cột Model và lý do) và thiết kế `docs/superpowers/specs/2026-11-18-window-snap-design.md`; rà soát đối chiếu thêm "Review Focus" của kế hoạch đó. Nhánh gốc: `feature/window-snap`. Đã xong, chủ dự án duyệt và gộp vào `main` (2026-11-18). Task mới sau đợt 8 cần thiết kế mới; nhánh gốc của đợt mới do kế hoạch của đợt đó quy định.

Task 66–72 (đợt 9 nội bộ = Giai đoạn 3 sản phẩm: Ghim ảnh lên màn hình) theo kế hoạch `docs/superpowers/plans/2026-11-18-pin-screenshot.md` và thiết kế `docs/superpowers/specs/2026-11-18-pin-screenshot-design.md`. Nhánh gốc: `feature/pin-screenshot`; đã duyệt và gộp vào `main` (2026-11-18). Task mới sau đợt 9 cần thiết kế mới. Thay đổi người dùng thấy được phải cập nhật `website/` trong cùng đợt (CLAUDE.md).

Task 73–77 (đợt 10 = Giai đoạn 3: Lặp lại lần chụp trước) theo kế hoạch `docs/superpowers/plans/2026-11-18-repeat-capture.md`; đã gộp `main`. Từ đợt 10, xong mỗi đợt thì phiên điều phối tự gộp `main` và làm tiếp đợt kế (quyết định 114).

Task 78–83 (đợt 11 = Giai đoạn 3: Nhận dạng chữ cục bộ) theo kế hoạch `docs/superpowers/plans/2026-11-18-local-ocr.md`; đã gộp `main`.

Task 84–88 (đợt 12 = Giai đoạn 3: Chụp từng bước) theo kế hoạch `docs/superpowers/plans/2026-11-18-step-capture.md`; đã gộp `main`.
