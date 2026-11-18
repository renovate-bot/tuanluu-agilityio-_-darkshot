# Task 87: Chụp từng bước — tài liệu, website, demo

**Agent:** `scaffolder` · **Model:** haiku · **Nhánh:** `task/87-steps-docs` · **Rà soát:** phiên điều phối · **Phụ thuộc:** 86

Thiết kế: `docs/superpowers/specs/2026-11-18-step-capture-design.md`. Memory bank, CLAUDE.md, quyết định do phiên điều phối làm.

## Files
`README.md` (mục "Chụp từng bước", menu khay, thanh ngang có ô Thêm bước khi có phiên, Ctrl+N, nơi lưu phiên, xuất), `CHANGELOG.md` ("Chưa phát hành"), `docs/testing/manual-checklist.md` ("Đợt 12: Chụp từng bước", mọi dòng `[ ]`), `scripts/demo-steps.ps1` (mẫu `scripts/demo-ocr.ps1`, tạo sẵn phiên tổng hợp 3 bước trong hồ sơ cách ly: ảnh màu đơn giản + `session.json`), `website/index.html` + `website/src/vi.json` (một mục/ý về Step capture, Ctrl+N, cập nhật câu đếm nhóm tính năng nếu thêm nhóm). Chữ khớp `crates/darkshot-core/src/i18n.rs`. `npm ci` + `npm run build` trong `website/`; `powershell -File scripts/check.ps1` → `ALL CHECKS PASSED`. Commit `docs:`; không email, đường dẫn cá nhân, khóa.
