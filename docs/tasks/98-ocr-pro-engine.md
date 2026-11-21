# Task 98: OCR nâng cao — khung dòng từ Windows OCR

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/98-ocr-pro-engine` (từ `feature/ocr-pro-15` sau Task 97) · **Rà soát:** `reviewer` (fable) · **Phụ thuộc:** 97

Thiết kế: `docs/superpowers/specs/2026-11-21-ocr-pro-design.md` mục 6.1, 7b.1–7b.3. Global Constraints: `docs/superpowers/plans/2026-11-21-ocr-pro.md`.

## Cần đọc
`crates/darkshot-windows/src/ocr.rs` (`recognize_in_mta`, `scale_bgra`, kiểm thử `manual`), `crates/darkshot-core/src/ocr_text.rs` (Interfaces của Task 97: `OcrLine`, `compose_all`, `map_rect`, `TextOptions`).

## Files
`crates/darkshot-windows/src/ocr.rs` (và chỗ dùng `OcrResult` nếu cần sửa để biên dịch).

## Interfaces
- Produces: `OcrResult { text: String, lang_tag: String, lang_name: String, lines: usize, lines_detail: Vec<OcrLine> }`. `text = compose_all(&lines_detail, TextOptions::default())`; `lines = lines_detail.len()`.

## Yêu cầu
1. Mỗi `OcrLine` của WinRT: chữ = `OcrLine.Text()`; khung = hợp `Words()[k].BoundingRect()` (f32, tọa độ ảnh đã đưa vào OCR `(w2,h2)`; làm tròn floor/ceil) rồi `map_rect(.., w2, h2, image.width, image.height)`. Dòng không có từ, hoặc chữ rỗng sau trim → bỏ.
2. Không đổi luồng, apartment, giao kết quả hay quyền sở hữu `Box`.
3. Lỗi đọc từ/khung một dòng → `OcrError::Failed` như các lỗi WinRT khác (không panic).

## Kiểm thử (viết trước)
- Chạy tay `#[ignore]` (nhóm `ocr::tests::manual`): ảnh tổng hợp ba dòng chữ Anh cách đều, vẽ bằng GDI (giữ `GDI_TEST_LOCK`) → 3 dòng, khung nằm trong ảnh, `top` tăng dần, mỗi khung chứa tâm dòng đã vẽ (sai số ≤ 4 px); ảnh nhỏ (cạnh ngắn < 40, bị phóng ×2) → khung vẫn ở tọa độ ảnh gốc; `text` bằng chuỗi cũ của cùng ảnh.
- Kiểm thử cũ của `ocr` vẫn qua.

TDD `test:` rồi `feat:`. `cargo fmt --all`; `powershell -File scripts/check.ps1` → `ALL CHECKS PASSED`; chạy `cargo test -p darkshot-windows ocr::tests::manual -- --ignored --test-threads=1`. Đo exe release (`--target-dir target/task98`) và ghi số vào commit.
