# Task 89: Chụp cuộn — thuật toán ghép (lõi)

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/89-scroll-core` (từ `feature/scroll-capture`) · **Rà soát:** `reviewer` (fable)

Thiết kế: `docs/superpowers/specs/2026-11-18-scroll-capture-design.md` mục 1, 3, 5. Global Constraints: kế hoạch `docs/superpowers/plans/2026-11-18-scroll-capture.md`.

## Cần đọc
`crates/darkshot-core/src/image.rs` (`Image`), `actions.rs` (`Action`, cách `Pin`/`Ocr`/`AddStep` được xử lý), `last_capture.rs` (`remember`), `i18n.rs`. `grep -rn "Action::AddStep" crates/` để thêm nhánh `Scroll` mọi nơi.

## Files
Tạo `crates/darkshot-core/src/scroll_stitch.rs`; sửa `lib.rs`, `actions.rs`, `last_capture.rs`, `i18n.rs`. Lớp Windows chỉ sửa tối thiểu để biên dịch (nhánh `Scroll` no-op, tooltip map `Msg::OverlayScroll`; KHÔNG thêm ô thanh ngang — Task 90).

## Interfaces
Như thiết kế mục 3: `Stitcher::new(w, h)`, `push(&Image) -> Push`, `height()`, `uncertain_count()`, `appended_frames()`, `finish(self) -> (Image, u32)`; `Push::{NoChange, Appended(u32), Uncertain(Reason), Limit}`; `Reason::{Ambiguous, NoOverlap, Backward, SizeMismatch}`; `MAX_HEIGHT`, `MAX_BYTES`, `MIN_OVERLAP_PX = 24`, `MIN_OVERLAP_FRAC = 0.15`, `MATCH_RATIO = 0.98`, `MIN_DISTINCT_ROWS = 8`. Tham số có thể chỉnh nếu kiểm thử cho thấy cần, nhưng ghi lý do.

## Yêu cầu
1. Băm hàng FNV-1a 64 bit trên byte B, G, R (bỏ alpha).
2. Dải tĩnh trên/dưới theo thiết kế; không để dải tĩnh nuốt cả khung (tối đa H/3 mỗi dải; còn vùng chạy < 2·MIN_OVERLAP thì coi như không có dải tĩnh).
3. Tìm dy: duyệt dy tăng dần, điểm = số hàng khớp / số hàng chồng lấp; chấp nhận dy tốt nhất nếu ≥ MATCH_RATIO, đủ hàng phân biệt và duy nhất (dy khác đạt ≥ điểm tốt nhất − 0,005 → Ambiguous). Không tìm được: kiểm cuộn ngược (dy âm khớp) → Backward; còn lại → NoOverlap.
4. Hiệu năng: khung 1920×1080 tìm dy < 15 ms ở release (đo trong kiểm thử `#[ignore]` hoặc ghi chú); dùng so băm hàng, không so điểm ảnh từng điểm trừ khi xác nhận ứng viên.
5. Không panic với khung cỡ khác (`SizeMismatch`), khung rỗng, W hoặc H = 0.
6. `Action::Scroll` sau `AddStep`; `run_action` no-op; `remember` giữ prev; JSON không bao giờ ghi "scroll".
7. Chuỗi i18n thiết kế mục 5 + đoạn Trợ giúp.

## Kiểm thử (viết trước) — ảnh tổng hợp có tính xác định
Sinh "trang" cao (ví dụ 900×6000) với các khối hàng có mẫu khác nhau (sọc, ô vuông, nhiễu giả ngẫu nhiên có seed); cắt các khung 900×700 theo chuỗi offset:
- cuộn đều 120 px → ảnh ghép bằng đúng phần trang tương ứng (so từng byte);
- cuộn không đều (37, 200, 5, 0, 350);
- tiêu đề dính 60 px + chân trang dính 40 px chồng lên mỗi khung → ảnh ghép có tiêu đề một lần ở trên, chân trang một lần ở dưới, phần giữa đúng;
- vùng một màu dài hơn khung → Ambiguous, không nối;
- nhảy 690 px (không đủ chồng lấp) → NoOverlap;
- cuộn ngược → Backward;
- khung giống hệt → NoChange;
- vượt MAX_HEIGHT → Limit;
- khung cỡ khác → SizeMismatch.
Cùng kiểm thử `run_action(Scroll)`, `remember`, i18n.

TDD `test:` rồi `feat:`. `cargo fmt --all`; `powershell -File scripts/check.ps1` → `ALL CHECKS PASSED`.

## Review focus
Đúng từng byte trên các ca; không có ca nào ghép sai mà trả `Appended`; mơ hồ được bắt; giới hạn; hiệu năng.
