# Task 81: OCR cục bộ — tài liệu, website, kịch bản demo

**Agent:** `scaffolder` · **Model:** haiku · **Nhánh:** `task/81-ocr-docs` · **Rà soát:** phiên điều phối · **Phụ thuộc:** 80

Thiết kế: `docs/superpowers/specs/2026-11-18-local-ocr-design.md`. Memory bank, CLAUDE.md, quyết định do phiên điều phối làm.

## Files
`README.md`, `CHANGELOG.md` (mục "Chưa phát hành"), `docs/testing/manual-checklist.md` (mục "Đợt 11: Nhận dạng chữ cục bộ", mọi dòng `[ ]`), `scripts/demo-ocr.ps1` (mẫu `scripts/demo-repeat.ps1`), `website/index.html` + `website/src/vi.json` (một ý/mục ngắn, bảng phím có Ctrl+T, câu quyền riêng tư: OCR chạy trên máy bằng Windows). Chữ phải khớp `crates/darkshot-core/src/i18n.rs`. README nêu: cần gói ngôn ngữ OCR của Windows; tiếng Việt có hay không tùy máy; không gửi ảnh đi đâu. `npm ci` + `npm run build` trong `website/` phải qua; `powershell -File scripts/check.ps1` → `ALL CHECKS PASSED`. Commit `docs:`; không email, đường dẫn cá nhân, khóa.
