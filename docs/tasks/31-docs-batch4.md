# Task 31: Tài liệu đợt 4

**Agent:** `scaffolder` · **Model:** haiku · **Nhánh:** `task/31-docs-batch4` · **Rà soát:** `reviewer`

Gói task tự chứa. Thuộc kế hoạch `docs/superpowers/plans/2026-11-16-darkshot-dark-theme.md`. Chạy sau Task 30. Chỉ sửa tài liệu và `scripts/check.ps1`; không sửa mã Rust.

## Cần đọc

- Thiết kế đợt 4 `docs/superpowers/specs/2026-11-16-darkshot-dark-theme-design.md` (toàn bộ, ngắn)
- Kế hoạch đợt 4 (toàn bộ)
- `README.md`, `docs/testing/manual-checklist.md`, `CLAUDE.md`, `memory-bank/{projectbrief,productContext,systemPatterns,techContext,decisions}.md`, `.claude/agents/*.md`, `.claude/commands/*.md`
- `git log --oneline main` từ commit `docs: batch 4 design` tới nay, và báo cáo cuối của Task 27–30 trong `memory-bank/activeContext.md`

## Global Constraints

- Không sửa mã Rust, spec, kế hoạch đã duyệt, gói task cũ (`docs/tasks/01` đến `30`), `prototype/`, các dòng lịch sử trong `memory-bank/progress.md` và các quyết định 1–42 trong `decisions.md` (chỉ thêm dòng mới).
- Tài liệu không được ghi là đã thử tay hay đã nghiệm thu đợt 4: chủ dự án chưa thử.
- Viết tiếng Việt như các tài liệu hiện có (README có thể giữ các đoạn English sẵn có).
- Kiểm `git branch --show-current` trước khi commit; không chuyển nhánh. Không đẩy lên remote. Không đọc hay ghi khóa R2. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.

## Các bước

- [ ] **1. Đổi tên trong tài liệu đang dùng.** `README.md`, `docs/testing/manual-checklist.md` (các mục cũ cũng đổi tên exe, thư mục, tên file: đây là checklist đang dùng), `CLAUDE.md` (dòng đầu thành `# darkshot`, câu mô tả, thêm một dòng "Đợt 4 (đổi tên darkshot, giao diện luôn tối): thiết kế `docs/superpowers/specs/2026-11-16-darkshot-dark-theme-design.md`, kế hoạch `docs/superpowers/plans/2026-11-16-darkshot-dark-theme.md`. Thiết kế đã duyệt cũng không sửa." và mục "Ràng buộc kỹ thuật" thêm "từ Task 27 dùng mục cùng tên của `docs/superpowers/plans/2026-11-16-darkshot-dark-theme.md`"), `memory-bank/{projectbrief,productContext,systemPatterns,techContext}.md`, `.claude/agents/*.md`, `.claude/commands/*.md` (thêm đoạn "Task 27–31 (đợt 4) theo kế hoạch ... và thiết kế ...; rà soát đối chiếu thêm Review Focus. Task 28 và 30 do `specialist` rà soát. Không dừng sau Task 30."). Tên repo GitHub `tuanluu-agilityio/shotme` và thư mục `D:\2026 - Tuan\shotme` chưa đổi: giữ nguyên những chỗ nhắc tới chúng như đường dẫn thật, và miễn chúng ở bước 3.
- [ ] **2. README.** Thêm mục "Chuyển từ shotme": thoát shotme trước khi chạy darkshot; lần đầu darkshot tự chép cấu hình (kể cả khóa R2) và chuyển khởi động cùng Windows; thư mục ảnh cũ vẫn được dùng; `%APPDATA%\shotme` không bị xóa, có thể xóa tay sau khi thấy darkshot chạy đúng. Thêm mục "Giao diện": luôn tối; thông báo ở khay và hộp chọn thư mục theo chế độ sáng/tối của Windows; thanh tiêu đề tối cần Windows 10 bản 20H1 trở lên.
- [ ] **3. `scripts/check.ps1`.** Mở rộng bước `no legacy name` sang `README.md` và `docs\testing\*`, miễn: dòng chứa `tuanluu-agilityio/` hoặc `2026 - Tuan`, và mục "Chuyển từ shotme" của README cùng mục "Đợt 4: chuyển từ shotme" của checklist (miễn theo khoảng dòng giữa tiêu đề mục đó và tiêu đề `##` kế tiếp). Giữ nguyên cách ghép mẫu tìm từ hai nửa.
- [ ] **4. `memory-bank/decisions.md`.** Thêm quyết định 43 trở đi, mỗi dòng theo mẫu bảng hiện có: (43) tên ứng dụng là darkshot ở mọi nơi trừ spec, kế hoạch, gói task cũ; tên cũ chỉ trong `migrate.rs` và `legacy.rs`; (44) chuyển dữ liệu: chép cấu hình khi chưa có, chèn `save_dir` cũ khi thiếu, không xóa thư mục cũ, cấu hình darkshot là nguồn đúng duy nhất của khởi động cùng Windows (khóa Run theo `autostart`, kể cả xóa giá trị `darkshot` khi tắt), xóa giá trị Run và WER cũ; (45) giao diện luôn tối, giữ điều khiển gốc và tự vẽ phần nhìn ở mức giống hệt prototype (chủ dự án chọn); (46) bảng màu theo kế hoạch đợt 4, màu nút chính khi trỏ `#2563EB` thay `#3D7BF0` của thiết kế vì `#3D7BF0` chỉ đạt 3,97:1; tỉ lệ tương phản ở mục 7.1 của thiết kế cao hơn thực tế, số đúng nằm ở kế hoạch; (47) biểu tượng giữ nguyên (chủ dự án chọn A), màu nhấn xanh lam; (48) đợt 4 là Task 27–31 với agent và model theo kế hoạch, chạy hết không dừng; cộng mọi quyết định mới mà báo cáo Task 27–30 ghi lại.
- [ ] **5. `memory-bank/systemPatterns.md`.** Thêm mục "Giao diện tối": hằng màu ở `darkshot_core::theme`, mọi cặp chữ/nền mới phải thêm vào `TEXT_PAIRS`; `theme.rs` của lớp Windows (thanh tiêu đề, chổi dùng chung, cách vẽ từng loại điều khiển theo báo cáo Task 30); ô mới trong Tùy chọn phải tự vẽ theo cùng cách. Sửa mục "Thêm một ô hoặc một tab" thêm bước tô màu.
- [ ] **6.** `powershell -File scripts/check.ps1` in `ALL CHECKS PASSED`. Commit `docs: batch 4 (darkshot, dark theme)`.

**Xong khi:** check qua; `git grep -il shotme -- README.md docs/testing CLAUDE.md .claude memory-bank/projectbrief.md memory-bank/productContext.md memory-bank/systemPatterns.md memory-bank/techContext.md` chỉ còn các chỗ được miễn (liệt kê trong báo cáo).
