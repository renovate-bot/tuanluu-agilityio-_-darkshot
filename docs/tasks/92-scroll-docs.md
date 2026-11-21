# Task 92: Chụp cuộn — tài liệu, website, demo

**Agent:** `scaffolder` · **Model:** haiku · **Nhánh:** `task/92-scroll-docs` · **Rà soát:** phiên điều phối · **Phụ thuộc:** 91

Thiết kế: `docs/superpowers/specs/2026-11-18-scroll-capture-design.md`. Memory bank, CLAUDE.md, quyết định do phiên điều phối làm.

## Files
`README.md` (mục "Chụp cuộn": phạm vi hỗ trợ — chỉ cuộn dọc, người dùng tự cuộn, vùng cố định; không đảm bảo mọi ứng dụng; cảnh báo không chắc chắn; giới hạn 30 000 px; Ctrl+L; thanh ngang thêm ô), `CHANGELOG.md` ("Chưa phát hành"), `docs/testing/manual-checklist.md` ("Đợt 13: Chụp cuộn", mọi dòng `[ ]`, gồm trình duyệt, tài liệu Word/PDF, danh sách Explorer, trang có tiêu đề dính, cuộn nhanh, DPI, nhiều màn hình, Windows 10 trước 2004), `scripts/demo-scroll.ps1` (mẫu `scripts/demo-ocr.ps1`), `website/index.html` + `website/src/vi.json` (một ý về Scrolling capture, Ctrl+L, nêu rõ phạm vi). Chữ khớp `crates/darkshot-core/src/i18n.rs`. `npm ci` + `npm run build`; `powershell -File scripts/check.ps1` → `ALL CHECKS PASSED`. Commit `docs:`; không email, đường dẫn cá nhân, khóa.
