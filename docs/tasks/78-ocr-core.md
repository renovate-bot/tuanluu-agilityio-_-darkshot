# Task 78: OCR cục bộ — lõi

**Agent:** `builder` · **Model:** sonnet · **Nhánh:** `task/78-ocr-core` (từ `feature/local-ocr`) · **Rà soát:** `reviewer` (sonnet)

Thiết kế: `docs/superpowers/specs/2026-11-18-local-ocr-design.md` mục 2.6–2.7, 3 (phần core), 5. Global Constraints: kế hoạch `docs/superpowers/plans/2026-11-18-local-ocr.md`.

## Cần đọc
`crates/darkshot-core/src/actions.rs` (`Action`, `run_action`, cách `Pin` được xử lý), `i18n.rs` (bảng, hàm có tham số, `help_text`, kiểm thử bảng), `lib.rs`. `grep -rn "Action::Pin" crates/` để thấy mọi `match` phải thêm nhánh.

## Files
Tạo `crates/darkshot-core/src/ocr_text.rs`; sửa `lib.rs`, `actions.rs`, `i18n.rs`. Sửa tối thiểu `crates/darkshot-windows/src/*.rs` CHỈ để biên dịch (nhánh `Action::Ocr` như `Pin`/`Cancel`: không làm gì; tooltip thanh ngang map `Ocr` → `Msg::OverlayOcr`; KHÔNG thêm ô thanh ngang — Task 80).

## Interfaces
```rust
// darkshot_core::ocr_text
pub const SMALL_EDGE: u32 = 40;
/// Trim từng dòng, bỏ dòng rỗng ở đầu/cuối (giữ dòng rỗng giữa), nối "\r\n".
pub fn join_lines(lines: &[String]) -> String;
/// Cỡ mới trước khi OCR: cạnh dài > max_dim → thu giữ tỉ lệ (làm tròn, mỗi cạnh ≥ 1);
/// cạnh ngắn < SMALL_EDGE và 2× vẫn ≤ max_dim → (2w, 2h); còn lại None.
pub fn ocr_scale(w: u32, h: u32, max_dim: u32) -> Option<(u32, u32)>;
/// Số dòng không rỗng.
pub fn line_count(text: &str) -> usize;
```
`Action::Ocr` sau `Pin`; `run_action(Ocr)` → `Outcome::Cancelled`, không tác dụng phụ. `remember_after`/`last_capture::remember` ở lớp Windows: Ocr KHÔNG được nhớ là lần chụp trước (lặp lại không chạy OCR) — nếu `last_capture::remember` nhận `Ocr` thì giữ `prev` (sửa core + kiểm thử); JSON không bao giờ ghi "ocr".

## Chuỗi
Như thiết kế mục 5, cộng `ocr_status(lang, name, lines) -> String`. `help_text`: một câu về Ctrl+T trong lớp phủ và "Nhận dạng chữ" ở menu ảnh ghim.

## Kiểm thử (viết trước)
`join_lines` (trim, dòng rỗng đầu/cuối/giữa, rỗng hết → ""), `ocr_scale` (giữ nguyên, thu 20000×100 về max 10000, phóng 300×30 → 600×60, phóng bị chặn bởi max), `line_count`; `run_action(Ocr)`; `remember(prev, Ocr, r) == prev`; i18n hai bản khác nhau, `ocr_status` chứa tên và số; `help_text` chứa "Ctrl+T".

TDD `test:` rồi `feat:`. `cargo fmt --all`; `powershell -File scripts/check.ps1` → `ALL CHECKS PASSED`.
