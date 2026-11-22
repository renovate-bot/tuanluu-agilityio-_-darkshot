# Task 137: Privacy Shield — đọc được chữ trong bảng có đường kẻ

**Agent:** `specialist` · **Model:** opus (đổi luồng quét OCR, ảnh hưởng độ phủ của tính năng bảo mật) · **Nhánh:** `task/137-privacy-table-lines` (từ `main`), worktree `D:\2026 - Tuan\shotme-136` · **Rà soát:** `reviewer` (opus, quyết định 154)

Thiết kế: `docs/superpowers/specs/2026-11-22-sensitive-info-redaction-design.md` (mục 4, 5, 11, 12, 18). Global Constraints và Review Focus: `docs/superpowers/plans/2026-11-22-sensitive-info-redaction.md`. Số đo: `docs/testing/privacy-quality.md`. Quyết định 144–153.

## Lỗi (chủ dự án báo khi thử tay, 2026-11-22)

Ảnh chụp bảng Excel (chữ đen trên nền trắng, đường kẻ ô xám/đen mảnh, cỡ chữ ~20 px, rộng ~1950 px, cao ~545 px): số điện thoại được che, **14/14 email không được che**.

Phiên điều phối đã chạy OCR (en-US) trên ảnh đó: OCR bỏ hết `@` và các chữ có phần xuống dưới (g, y, p, q) vì đường kẻ ô nằm sát chân chữ. Ví dụ `cathuynhtinh@gmail.com` → dòng "cathu nhtinh" và một dòng riêng "ail.com"; `sailethuy@yahoo.com` → "sailethu ahoo.com"; cột tên "Nguyễn" → "N en". Phóng to ×2/×3 hay bộ nhận dạng ngôn ngữ hồ sơ: vẫn 0/14. **Xóa các đoạn tối dài (ngang ≥ 60 px, dọc ≥ 40 px) trên bản sao rồi OCR: đọc đúng 14/14, bộ dò thấy 14/14 email.** Lỗi phụ: một dòng OCR đọc `xuanthuy dng@gmail.com` (chèn space vào phần trước `@`) → chỉ `dng@gmail.com` được che, "xuanthuy" lộ.

Không đưa ảnh thật hay dữ liệu của chủ dự án vào kiểm thử, tài liệu, commit.

## Files

`crates/darkshot-core/src/privacy/` (mô-đun mới, ví dụ `rules.rs`, + `mod.rs`, `detect/email.rs`, `quality_tests.rs`/`tests.rs`), `crates/darkshot-windows/src/ocr.rs` (quét), `crates/darkshot-windows/src/privacy_gate.rs` (hạn quét nếu đổi), `docs/testing/privacy-quality.md`, `CHANGELOG.md` (Chưa phát hành), `README.md` (mục Privacy Shield nếu cần một câu), `memory-bank/decisions.md` (quyết định 155), `memory-bank/activeContext.md`, `memory-bank/progress.md` (phiên điều phối ghi token).

## Yêu cầu

1. **Lõi thuần `strip_rules`** (darkshot-core, không phụ thuộc Windows): nhận một dải ảnh BGRA, trả `Option<Image>` (bản sao đã xóa đường kẻ) hoặc `None` khi không thấy đường kẻ nào (không cần lượt hai).
   - Đường kẻ = đoạn ngang dài ≥ ~40 px (hoặc dọc ≥ ~40 px), dày ≤ 3 px, mà mỗi điểm khác hẳn (độ sáng) cả điểm phía trên VÀ phía dưới cách nó (độ dày + 1) px (với đường dọc: trái và phải), còn hai điểm hai bên đó gần nhau. Nghĩa là nét mảnh tương phản với nền ở hai bên — chạy đúng cả nền sáng (Excel) lẫn nền tối (bảng giao diện tối, kẻ sáng). Không dùng ngưỡng "tối tuyệt đối" (nền tối sẽ bị xóa hết).
   - Thay điểm đường kẻ bằng màu nền lấy từ hai phía (trung bình), không tô trắng cứng.
   - Không xóa nét chữ: nét ngang của chữ lớn/đậm (dày > 3 px), chữ thường, gạch chân ngắn. Gạch chân dài bị xóa thì chấp nhận (bản sao chỉ dùng để quét).
   - Chạy O(w·h), không cấp phát lớn hơn một bản sao dải. Không log, không lưu.
   - Kiểm thử: bảng tổng hợp nền sáng kẻ tối; nền tối kẻ sáng; kẻ dày 1/2/3 px; nét dày 5 px không bị xóa; ảnh không có kẻ → `None`; ảnh 1×1, ảnh rỗng; kẻ ở mép ảnh.
2. **Quét hai lượt** (`ocr.rs` `scan_in_mta`/`recognize_tile`): mỗi dải OCR bản gốc như hiện nay; nếu `strip_rules` trả `Some`, OCR thêm bản sao đó (cùng engine, cùng tỉ lệ, cùng khung tọa độ), `privacy::detect` cả hai, gộp bằng đường chuẩn hóa sẵn có (không trùng phát hiện, vùng che là hợp hai lượt). Chữ OCR của lượt hai cũng chỉ sống trên luồng `darkshot-ocr` và bị ghi đè như lượt một; bản sao ảnh nhả trước dải sau. Lỗi/hết hạn ở lượt hai = lỗi quét (không bao giờ coi như sạch). Hạn mỗi dải và hạn của cổng (`privacy_gate`, hiện 20 s × dải + 10 s) phải đủ cho hai lượt — sửa và cập nhật kiểm thử hạn nếu cần.
3. **Email bị OCR chèn space trước `@`**: khi phần trước `@` bắt đầu ngay sau một space và trước space đó là một từ chỉ gồm ký tự hợp lệ của phần tên email (chữ thường/số/`._%+-`, ≥ 2 ký tự, không chữ hoa), và khoảng cách giữa hai khung từ nhỏ (ví dụ < 0,6 × chiều cao dòng, dùng khung từ sẵn có), mở rộng vùng che sang từ đó (độ tin cậy Trung bình). Không mở rộng khi từ trước là chữ thường gặp tách xa (`email: a@b.com`, `contact at a@b.com` theo khoảng cách bình thường). Kiểm thử cả hai hướng.
4. **Chất lượng** (`quality_tests.rs`): thêm bộ mẫu dòng mô phỏng lỗi OCR bảng nếu hợp lý; ngưỡng = số đo mới. Thêm kiểm thử `#[ignore]` trong `darkshot-windows` vẽ một bảng tổng hợp (dữ liệu GIẢ, `@example.com`, chữ đen nền trắng, kẻ ô sát chân chữ có g/y/p/q, ~20 px) bằng GDI, quét bằng đường thật, ghi số email phát hiện trước (lượt một) và sau (hai lượt). Ghi số thật vào `privacy-quality.md`; nếu ảnh tổng hợp không tái hiện được lỗi thì ghi rõ như vậy, không bịa.
5. **Hiệu năng**: đo lại OCR+dò 1920×1080 (có bảng và không bảng), 3840×2160, 1000×20000; thời gian `strip_rules` 4K. Cập nhật `privacy-quality.md`. Exe trước/sau (trước: 2.459.648).
6. **Tài liệu**: CHANGELOG "Chưa phát hành" — thêm vào mục Privacy Shield một câu: bảng có đường kẻ được quét thêm một lượt trên bản sao đã bỏ đường kẻ (ảnh xuất không đổi). `memory-bank/decisions.md` quyết định 155 (ngày 2026-11-22, chủ dự án duyệt hướng sửa). Website không đổi nếu không có hành vi người dùng thấy mới ngoài độ phủ (ghi lý do trong báo cáo).
7. `cargo fmt --all`; `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`; chạy tay các `#[ignore]` của `ocr`, `privacy_gate` liên quan và ghi số. Không giả lập chuột/phím thật (chủ dự án đang dùng máy). Commit `fix:` / `test:` / `docs:` trên nhánh, KHÔNG gộp, không đẩy. Báo cáo: thay đổi, số đo, rủi ro, token.
