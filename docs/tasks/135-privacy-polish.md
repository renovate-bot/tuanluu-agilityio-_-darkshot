# Task 135: Privacy Shield — sửa sau rà soát toàn đợt 21

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/135-privacy-polish` (từ `feature/privacy-shield`), worktree `D:\2026 - Tuan\shotme-privacy` · **Rà soát:** `reviewer` (fable)

Thiết kế: `docs/superpowers/specs/2026-11-22-sensitive-info-redaction-design.md` mục 3.2 (thứ tự), 8, 9, 18.1, 18.5. Global Constraints: `docs/superpowers/plans/2026-11-22-sensitive-info-redaction.md`. Số đo: `docs/testing/privacy-quality.md`.

## Files
`crates/darkshot-windows/src/privacy_gate.rs`, `steps_window.rs`, `steps_export.rs`, `main.rs`, `ocr.rs`, `scripts/check.ps1`, `docs/testing/privacy-quality.md` (cập nhật số đo bộ nhớ xuất).

## Yêu cầu
1. **Thứ tự theo lúc nhận** (privacy_gate.rs `on_scan`→`ready` ~518–558, `try_open` ~628–694): lượt được duyệt VÀ xuất theo thứ tự `submit`, không theo lúc quét xong. Lượt sau quét xong sạch trước thì chờ tới khi mọi lượt trước nó đã xuất hoặc bị bỏ. Không có lượt nào bị kẹt vì lượt trước (lượt trước bị Drop/Hủy/hết hạn thì lượt sau chạy tiếp). Kiểm thử lõi thuần (A chậm, B nhanh sạch → xuất A rồi B; A bị Hủy → B xuất ngay) và sửa `manual_two_requests_are_reviewed_one_at_a_time` (~3002) cho hết chập chờn (chạy 10 lần liên tiếp, ghi số).
2. **Xuất phiên giữ byte PNG thay ảnh BGRA** (steps_window.rs ~2033, `ExportJob.replacements`): mỗi ảnh đã che mã hóa PNG ngay khi cổng trả `done(Some)` (trên luồng nền nếu ảnh > 4 MP, theo mẫu sẵn có), giữ `Vec<u8>`; `fill` ghi byte đó (tạo mới, không ghi đè). Đo lại bộ nhớ đỉnh xuất 10 bước 4K, cập nhật `privacy-quality.md`. Mã hóa lỗi → hủy xuất, báo lỗi.
3. `main.rs` ~1543 `privacy_shield_now`: khi chưa có trạng thái ứng dụng trả `true` (nhất quán `privacy_gate::enabled()`).
4. Cửa sổ Duyệt của lượt xuất đã bị bỏ (steps_window.rs ~2081 `check_scan_session`→`drop_scan`): đóng luôn lượt `StepExport` trong cổng (API cổng hủy theo mã lượt, ví dụ `privacy_gate::cancel(id)` trả về id từ `submit_with`) để cửa sổ của nó đóng.
5. `check.ps1` luật "no logging": phủ thêm `crates/darkshot-windows/src/ocr.rs` ngoài `mod tests` (mã chạy thật); thử cài lỗi tạm.
6. Gỡ `ocr::start_scan` nếu chỉ kiểm thử dùng (cho kiểm thử gọi `start_scan_with_progress(.., None, ..)`), bỏ `allow(dead_code)`.
7. `cargo fmt --all`, `check.ps1` ALL CHECKS PASSED, `#[ignore]` của privacy_gate/review_window/steps_window chạy tay; exe trước/sau. Không giả lập chuột/phím thật.
