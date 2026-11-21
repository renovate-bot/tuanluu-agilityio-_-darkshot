# Task 100: OCR nâng cao — tài liệu, website, demo

**Agent:** `scaffolder` · **Model:** sonnet · **Nhánh:** `task/100-ocr-pro-docs` (từ `feature/ocr-pro-15` sau Task 99) · **Rà soát:** phiên điều phối · **Phụ thuộc:** 99

Thiết kế: `docs/superpowers/specs/2026-11-21-ocr-pro-design.md`. Global Constraints: `docs/superpowers/plans/2026-11-21-ocr-pro.md`. Luật website: `CLAUDE.md` mục Website và `website/README.md`.

## Files
`README.md` (mục OCR, phím tắt nếu có, "Đã biết"), `CHANGELOG.md` ("Chưa phát hành" → Mới), `docs/testing/manual-checklist.md` (mục "Đợt 15"), `website/index.html` + `website/src/vi.json` (cùng khóa `data-i18n`), `website/src/scene.ts` nếu hình OCR có trong đó, `scripts/demo-ocr.ps1` (ảnh mẫu có ít nhất hai đoạn chữ Anh để thử chọn dòng/tách đoạn).

## Yêu cầu
1. Mô tả đúng mã: bấm khung chép dòng, Ctrl+bấm gom, ↑/↓/Enter/Space/Ctrl+A, ba công tắc (mặc định tắt, nhớ lại), "Chép tất cả" không đóng cửa sổ.
2. Nói thẳng: Windows chưa có bộ nhận dạng tiếng Việt nên chữ có dấu có thể mất dấu; không hứa sẽ có. Vẫn hoàn toàn trên máy, không mạng.
3. Checklist "Đợt 15": các bước thử tay theo thiết kế mục 2 và Review Focus của kế hoạch, ô `[ ]`.
4. `website/`: `npm ci` rồi `npm run build` phải qua. Không ảnh có dữ liệu cá nhân.
5. Không sửa số phiên bản, không tag.

`powershell -File scripts/check.ps1` → `ALL CHECKS PASSED`. Commit `docs:`.
