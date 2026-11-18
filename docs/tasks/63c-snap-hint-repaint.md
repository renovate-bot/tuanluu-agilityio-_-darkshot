# Task 63c: Viên nhắc còn đọng sau khi bấm chọn ứng viên

**Agent:** `builder` · **Model:** sonnet (sửa nhỏ, phạm vi rõ) · **Nhánh:** `task/63c-snap-hint-repaint` (tách từ `feature/window-snap`) · **Rà soát:** `reviewer` (sonnet)

Nguồn: QA giao diện trên bản demo (phiên điều phối, 2026-11-18). Global Constraints: mục cùng tên của `docs/superpowers/plans/2026-11-18-window-snap.md`.

## Lỗi
Bấm (không kéo) chọn một ứng viên: vùng chọn, tay nắm, thanh công cụ hiện đúng nhưng viên nhắc "Bấm: chọn cửa sổ · …" vẫn còn trên màn hình cho tới lần di chuột kế tiếp. Thiết kế 3.5: viên nhắc ẩn khi có vùng chọn. Nguyên nhân dự đoán: `select_candidate` (và có thể nhánh `on_up` gọi nó) không làm mất hiệu lực vùng của viên nhắc; chỉ `on_move` làm việc đó.

## Files
`darkshot-windows/src/overlay.rs`.

## Yêu cầu
- Mọi đường đổi trạng thái hiện/ẩn của viên nhắc (chọn ứng viên bằng bấm hoặc Enter, bỏ vùng chọn bằng bấm ra ngoài, bắt đầu/kết thúc kéo) phải vẽ lại ô viên nhắc cũ. Cách gọn: một hàm nhỏ so `hint()` trước/sau thay đổi trạng thái và invalidate cả hai ô; gọi ở `select_candidate` và các chỗ tương tự.
- Kiểm thử trạng thái (mô-đun `snap_state`, dựng `State::new` không cần cửa sổ): ghi lại các vùng được invalidate (nếu đã có cơ chế ghi trong kiểm thử; nếu chưa, thêm bộ ghi chỉ khi `cfg(test)`) và khẳng định sau khi bấm chọn ứng viên, vùng viên nhắc cũ nằm trong vùng được vẽ lại; tương tự cho Enter.

TDD: commit `test:` rồi `fix:`. `cargo fmt --all`; `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.
