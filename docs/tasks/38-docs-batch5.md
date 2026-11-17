# Task 38: Tài liệu đợt 5

**Agent:** `scaffolder` · **Model:** haiku · **Nhánh:** `task/38-docs-batch5` · **Rà soát:** `reviewer`

Gói task tự chứa. Thuộc kế hoạch `docs/superpowers/plans/2026-11-17-annotate.md`. Chạy sau Task 37. Chỉ sửa tài liệu; không sửa mã Rust hay `scripts/`.

## Cần đọc

- Thiết kế đợt 5 `docs/superpowers/specs/2026-11-17-annotate-design.md` (toàn bộ)
- Kế hoạch đợt 5 (toàn bộ)
- `README.md`, `docs/testing/manual-checklist.md`, `CLAUDE.md`, `memory-bank/{systemPatterns,techContext,decisions}.md`, `.claude/commands/*.md`, `.claude/agents/*.md`
- Báo cáo Task 34–37 trong `memory-bank/activeContext.md`; `git log --oneline main` từ commit `docs: batch 5 design draft (annotate)`

## Global Constraints

- Không sửa mã Rust, `scripts/`, spec, kế hoạch, gói task cũ (`docs/tasks/01` đến `37`), `prototype/`, các dòng lịch sử trong `memory-bank/progress.md` và quyết định 1–54 trong `decisions.md` (chỉ thêm dòng mới).
- Tài liệu không được ghi là đã thử tay hay đã nghiệm thu đợt 5: chủ dự án chưa thử.
- Viết tiếng Việt như các tài liệu hiện có.
- Kiểm `git branch --show-current` trước khi commit; không chuyển nhánh. Không đẩy lên remote. Không đọc hay ghi khóa R2. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.

## Các bước

- [ ] **1. README.** Trong "Cách dùng", thêm mục "### Đánh dấu ảnh" sau "### Thanh công cụ": thanh dọc, tám nút và phím tắt (P, L, A, R, H, T, Ctrl+Z), cách chọn/bỏ chọn công cụ, Shift khóa góc và hình vuông, bảng tám màu (nhớ trong lúc ứng dụng chạy), công cụ Chữ (Enter xuống dòng, bấm chỗ khác để chốt, Esc hủy ô đang gõ, gõ được tiếng Việt), kéo ngoài vùng khi đã có nét không tạo vùng mới, Esc ngoài lúc gõ đóng lớp phủ và bỏ mọi nét, nét được in vào ảnh tải lên/lưu/chép, hai phím tắt toàn màn hình không có công cụ vẽ. Cập nhật mục "Phạm vi" nếu nó liệt kê tính năng không có.
- [ ] **2. CLAUDE.md.** Thêm dòng "Đợt 5 (công cụ đánh dấu trên ảnh chụp): thiết kế `docs/superpowers/specs/2026-11-17-annotate-design.md`, kế hoạch `docs/superpowers/plans/2026-11-17-annotate.md`. Thiết kế đã duyệt cũng không sửa." sau dòng đợt 4; mục "Ràng buộc kỹ thuật" thêm "từ Task 34 dùng mục cùng tên của `docs/superpowers/plans/2026-11-17-annotate.md`".
- [ ] **3. `.claude/commands/*.md`, `.claude/agents/*.md`.** Chỗ nào liệt kê các đợt hoặc kế hoạch theo task thì thêm "Task 34–38 (đợt 5) theo kế hoạch `docs/superpowers/plans/2026-11-17-annotate.md` và thiết kế `docs/superpowers/specs/2026-11-17-annotate-design.md`; Task 35, 36, 37 do `specialist` rà soát."
- [ ] **4. `memory-bank/decisions.md`.** Thêm quyết định 55 trở đi theo mẫu bảng: (55) đợt 5 thêm công cụ đánh dấu trong lớp phủ, chủ dự án đồng ý làm đủ tám mục và giao các lựa chọn còn lại cho điều phối viên (vị trí thanh, độ dày, bảng màu, Esc, kéo ngoài vùng; liệt kê giá trị); (56) hình lưu theo tọa độ ảnh chụp, giữ yên khi di chuyển hay đổi cỡ vùng, phần ngoài vùng không in; (57) một hàm vẽ GDI+ dùng chung cho lớp phủ và ảnh ra; in thất bại thì dừng hành động và báo lỗi, không gửi ảnh thiếu nét; (58) đợt 5 là Task 34–38 với agent và model theo kế hoạch, chạy hết không dừng; cộng mọi quyết định mới mà báo cáo Task 34–37 ghi lại.
- [ ] **5. `memory-bank/systemPatterns.md`, `techContext.md`.** Thêm mục "Đánh dấu ảnh": mô hình ở `darkshot_core::annotate`, vẽ ở `annotate_draw.rs` (`draw_shapes` dùng chung, `burn`), `gdiplus.rs`; thêm một công cụ mới cần sửa những chỗ nào (theo báo cáo Task 36–37). Feature mới của crate `windows` nếu có.
- [ ] **6.** Kiểm mục "Đợt 5: công cụ đánh dấu" của `docs/testing/manual-checklist.md` đủ dòng của Task 36 và 37; không sửa nội dung, chỉ sửa lỗi định dạng nếu có.
- [ ] **7.** `powershell -File scripts/check.ps1` in `ALL CHECKS PASSED`. Commit `docs: batch 5 (annotate)`.

**Xong khi:** check qua; báo cáo liệt kê file đã sửa và quyết định mới.
