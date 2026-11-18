# Task 66: Viên nhắc Bắt cửa sổ còn đọng sau khi BẤM chọn — tìm nguyên nhân gốc

**Agent:** `specialist` · **Model:** opus (điều tra vẽ lại trong máy trạng thái lớp phủ lớn; cần suy luận sâu, không sửa mò) · **Nhánh:** `task/66-snap-hint-click` (tách từ `feature/pin-screenshot`) · **Rà soát:** `reviewer` (fable) · **Phụ thuộc:** –

Global Constraints: mục cùng tên của `docs/superpowers/plans/2026-11-18-pin-screenshot.md`. Bối cảnh: quyết định 100, gói `docs/tasks/63c-snap-hint-repaint.md`, mục "Đợt 8" của `docs/testing/manual-checklist.md` (dòng "Viên nhắc sau khi BẤM chọn").

## Hiện tượng (đã thấy trên demo đợt 8, DPI 200%)
Chưa có vùng chọn, rê lên một cửa sổ, BẤM (không kéo): vùng chọn, tay nắm, hai thanh công cụ hiện đúng nhưng viên nhắc "Bấm: chọn cửa sổ · …" ở mép trên màn hình vẫn còn tới lần di chuột kế tiếp. Chọn bằng Enter thì không bị. Task 63c đã thêm invalidate ô viên nhắc ở `select_candidate` và kiểm thử trạng thái cho thấy vùng đó đã được yêu cầu vẽ lại, nhưng hiện tượng vẫn còn trên demo (demo điều khiển bằng thông điệp gửi tới cửa sổ, không phải chuột thật).

## Cần đọc
`crates/darkshot-windows/src/overlay.rs`: `select_candidate`, `hint`, `on_down`, `on_up`, `on_move`, `on_key` (nhánh Enter), `dragging`, các hàm invalidate, `paint` (cách tính vùng cần vẽ, đệm `back`, các lớp vẽ theo thứ tự), nhánh `WM_LBUTTONDOWN/UP/MOUSEMOVE/SETCURSOR/CAPTURECHANGED` của thủ tục cửa sổ, mô-đun kiểm thử `snap_state`. Dùng `grep -n` rồi đọc theo đoạn.

## Hướng điều tra (gợi ý, không phải kết luận)
- So hai đường Enter và Bấm: thứ tự "đổi trạng thái → invalidate → vẽ" có khác không; có chỗ nào sau `on_up` vẽ lại ô viên nhắc bằng trạng thái CŨ (ví dụ `self.mouse`, `dragging()` còn đúng khi tính, `SetCapture`/`ReleaseCapture` gửi `WM_MOUSEMOVE`/`WM_CAPTURECHANGED` làm tính lại `hint()` trước khi `sel` đổi)?
- `hint()` trả `None` khi có vùng chọn, nhưng ô cần invalidate được tính ở đâu: trước hay sau khi `sel` đổi? Nếu tính sau thì `hint()` đã là `None` và ô cũ không được biết.
- Đệm `back`: vùng viên nhắc có được vẽ lại vào đệm rồi mới chép ra màn hình không, hay chỉ phần giao với khung chọn?
- Bộ ghi invalidate của kiểm thử có phản ánh đúng đường thật (cửa sổ thật) không.

## Yêu cầu
1. Tái hiện trên cửa sổ thật nếu được: kiểm thử chạy tay `#[ignore]` mở lớp phủ với `dpis` tường minh và ứng viên giả, gửi `WM_MOUSEMOVE` → `WM_LBUTTONDOWN` → `WM_LBUTTONUP` tại cùng điểm, rồi đọc điểm ảnh của đệm/màn hình ở ô viên nhắc (hoặc dùng bộ ghi vùng vẽ). Ghi lệnh chạy.
2. Nêu nguyên nhân gốc kèm bằng chứng (dòng mã, trình tự thông điệp). Chỉ sửa khi đã có nguyên nhân. Sửa nhỏ nhất có thể, không đổi hành vi khác.
3. Thêm kiểm thử hồi quy (thuần/trạng thái chạy được trong `cargo test`, và kiểm thử chạy tay nếu cần cửa sổ thật). Kiểm thử phải thất bại trước khi sửa.
4. Nếu sau khi điều tra kỹ vẫn không tái hiện được: không sửa; báo cáo các giả thuyết đã loại trừ và bằng chứng; phiên điều phối giữ mục thử tay.
5. Không đụng gì ngoài lớp phủ; không đổi kiểm thử cũ cho khớp mã.

## Files
`crates/darkshot-windows/src/overlay.rs` (và `crates/darkshot-core/src/snap.rs` chỉ nếu nguyên nhân nằm ở hàm thuần ở đó, kèm kiểm thử).

## Tiêu chí nghiệm thu
- Có nguyên nhân gốc có bằng chứng và kiểm thử hồi quy qua, HOẶC báo cáo "không tái hiện được" có bằng chứng.
- Hành vi chọn bằng Enter, kéo, vùng nhớ, ô gợi ý tắt giữ nguyên (kiểm thử `snap_state` cũ vẫn qua).
- `scripts/check.ps1` in `ALL CHECKS PASSED`.

## Review focus
Nguyên nhân có thật sự giải thích hiện tượng (bấm bị, Enter không)? Sửa có làm thừa vùng vẽ lại (chớp) hay đổi thứ tự sự kiện chuột không?

TDD: commit `test:` rồi `fix:`. Báo cáo theo mẫu `.claude/agents/specialist.md`.
