# Task 49: Tài liệu đợt 6 (Annotation Pro)

**Agent:** `scaffolder` · **Model:** haiku · **Nhánh:** `task/49-docs-batch6` · **Rà soát:** `reviewer`

Gói task tự chứa. Thuộc kế hoạch `docs/superpowers/plans/2026-11-17-annotation-pro.md`. Chạy sau Task 48. Chỉ sửa tài liệu; không sửa mã Rust hay `scripts/`.

## Cần đọc

- Thiết kế đợt 6 `docs/superpowers/specs/2026-11-17-annotation-pro-design.md` (toàn bộ).
- Kế hoạch đợt 6 (toàn bộ).
- `README.md` (mục "Đánh dấu ảnh", "Phạm vi"), `docs/testing/manual-checklist.md` (mục "Đợt 5"), `CLAUDE.md`, `.claude/commands/next-task.md`, `memory-bank/{systemPatterns,techContext,decisions}.md`.
- Báo cáo Task 44–48 trong `memory-bank/activeContext.md`; `git log --oneline main` từ commit `docs: batch 6 design and plan (annotation pro)`.

## Global Constraints

- Không sửa mã Rust, `scripts/`, spec, kế hoạch, gói task cũ (`docs/tasks/01` đến `48`), `prototype/`, các dòng lịch sử trong `memory-bank/progress.md`, các quyết định đã có trong `decisions.md` (chỉ thêm dòng mới).
- Tài liệu không được ghi là đã thử tay hay đã nghiệm thu đợt 6.
- Mọi phím tắt và tên nút viết đúng như bảng `darkshot_core::i18n` (đọc `crates/darkshot-core/src/i18n.rs`, không sửa).
- Viết tiếng Việt như các tài liệu hiện có.
- Kiểm `git branch --show-current` trước khi commit; không chuyển nhánh. Không đẩy lên remote. Không đọc hay ghi khóa R2. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.

## Các bước

- [ ] **1. README, mục "Đánh dấu ảnh".** Cập nhật cho thanh 14 nút: bảng công cụ thêm Chọn (V), Hình elip (E, Shift: hình tròn), Đánh số (N, số tự đánh lại 1..n khi xóa/hoàn tác), Cỡ (`[`/`]`, bốn mức và giá trị ở thiết kế 5.6), Làm lại (Ctrl+Y, Ctrl+Shift+Z). Thêm đoạn "Sửa hình đã vẽ": chọn bằng công cụ Chọn, kéo để di chuyển, kéo tay nắm để đổi cỡ (Shift giữ tỉ lệ), Delete/Backspace xóa, đổi màu/cỡ áp cho hình đang chọn, Esc hủy lần kéo rồi bỏ chọn, mọi thao tác hoàn tác/làm lại được, khung chọn không in vào ảnh. Ghi vùng che (Làm mờ) khi di chuyển/đổi cỡ được tính lại màu từ ảnh gốc. Sửa mọi câu cũ nói "chỉ hoàn tác hình cuối", "không có làm lại" hay chưa chọn/sửa được hình. Mục "Phạm vi" nếu nhắc các điều này cũng sửa; ghi phần ngoài phạm vi của thiết kế mục 1.
- [ ] **2. Checklist.** Thêm mục "## Đợt 6: Annotation Pro" vào `docs/testing/manual-checklist.md` với các dòng `[ ]` theo phần 8 của yêu cầu (A–H của thiết kế mục 10): hoàn tác/làm lại nhiều bước; kéo = một bước; Delete hoàn tác được; Ctrl+Z khi gõ chữ như cũ; chọn hình trên cùng; bấm ngoài bỏ chọn; di chuyển/đổi cỡ từng loại hình; elip mọi hướng, Shift ra tròn; vòng số 1..n, hoàn tác/làm lại/xóa giữ số liên tục, đọc được trên nền sáng và tối; bốn mức cỡ cho hình mới và hình chọn; chữ tiếng Việt ở các cỡ; vùng che kín sau dời và đổi cỡ, màu từ ảnh gốc, không lộ; khung chọn không có trong ảnh Lưu/Chép/Tải lên (PNG, JPEG); ảnh nhỏ, 4K, hai màn hình 4K, 100 hình, nét tự do dài, DPI 100/125/150/200% (thanh hai cột ở 1080p 200%); không công cụ thì mọi luồng cũ (PrintScreen, Ctrl+D/S/C, Esc, Giữ vị trí vùng, phím toàn màn hình) như trước.
- [ ] **3. CLAUDE.md.** Thêm dòng "Đợt 6 (Annotation Pro: hoàn tác/làm lại nhiều bước, chọn và sửa hình, Hình elip, Đánh số, mức cỡ): thiết kế `docs/superpowers/specs/2026-11-17-annotation-pro-design.md`, kế hoạch `docs/superpowers/plans/2026-11-17-annotation-pro.md`. Thiết kế đã duyệt cũng không sửa." sau dòng đợt 5; mục "Ràng buộc kỹ thuật" thêm "; từ Task 44 dùng mục cùng tên của `docs/superpowers/plans/2026-11-17-annotation-pro.md`".
- [ ] **4. `.claude/commands/next-task.md`.** Thêm đoạn cuối: "Task 44–49 (đợt 6) theo kế hoạch `docs/superpowers/plans/2026-11-17-annotation-pro.md` và thiết kế `docs/superpowers/specs/2026-11-17-annotation-pro-design.md`; Task 45, 46, 47, 48 do `specialist` rà soát. Chạy đến hết không dừng (quyết định 75)."
- [ ] **5. `memory-bank/decisions.md`.** Thêm dòng sau quyết định cuối hiện có (không trùng số đã có): sổ thay đổi theo hình (Add/Remove/Replace) thay vì chụp cả danh sách; số vòng Đánh số đánh lại theo thứ tự; một nút Cỡ bốn mức dùng chung, nhớ trong tiến trình, không ghi cấu hình; công cụ Chọn chỉ bật khi chọn nút V, không công cụ thì như cũ; Esc năm bậc; thanh hai cột khi thấp; ngoài phạm vi (chọn nhiều, z-order, sửa nội dung chữ, nút Xóa); cộng mọi quyết định mới mà báo cáo Task 44–48 trong `activeContext.md` ghi lại.
- [ ] **6. `memory-bank/systemPatterns.md`, `techContext.md`.** Mục "Đánh dấu ảnh" thêm: `darkshot_core::edit` (trúng hình, tay nắm, biến đổi, đổi kiểu), sổ `Annotations` và hàm áp `Applied` ở lớp phủ (giữ `mark_bounds` song song), `geometry::side_layout`; thêm một loại hình mới giờ cần sửa những chỗ nào (lõi: `Shape`, `bounds`, `edit::{hit_shape, handles, transform, restyle}`; vẽ: `draw_one`; lớp phủ: `begin_stroke`, biểu tượng, `SIDE_ITEMS`).
- [ ] **7.** `powershell -File scripts/check.ps1` in `ALL CHECKS PASSED`. Commit `docs: batch 6 (annotation pro)`.

**Xong khi:** check qua; báo cáo liệt kê file đã sửa và quyết định mới.
