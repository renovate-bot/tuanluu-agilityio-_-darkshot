# Task 76: Lặp lại lần chụp trước — tài liệu, website, kịch bản demo

**Agent:** `scaffolder` · **Model:** haiku (tài liệu và chữ website cơ học) · **Nhánh:** `task/76-repeat-docs` · **Rà soát:** phiên điều phối đọc diff · **Phụ thuộc:** 75

Thiết kế: `docs/superpowers/specs/2026-11-18-repeat-capture-design.md`. Phần memory bank, CLAUDE.md, quyết định do phiên điều phối làm.

## Cần đọc
`README.md` (mục phím tắt, menu khay, Ghim làm mẫu), `CHANGELOG.md` (mục "Chưa phát hành"), `docs/testing/manual-checklist.md` (mục "Đợt 9" làm mẫu), `scripts/demo-pin.ps1`, `crates/darkshot-core/src/i18n.rs` (chữ thật), `CLAUDE.md` mục Website, `website/index.html`, `website/src/vi.json`, `website/README.md`.

## Files
`README.md`, `CHANGELOG.md`, `docs/testing/manual-checklist.md`, `scripts/demo-repeat.ps1` (mới), `website/index.html`, `website/src/vi.json`.

## Yêu cầu
1. README: phím "Chụp lại vùng trước" (mặc định tắt, Ctrl+Shift+PrintScreen, bật ở tab Phím tắt), mục menu khay, hành vi (cùng vùng, cùng hành động, không lớp phủ, không lặp hình đánh dấu, nhớ qua khởi động, vùng mất → mở lớp phủ kèm thông báo), tệp `last_capture.json` chỉ chứa tọa độ và hành động; `keep_selection` nay nhớ qua khởi động.
2. CHANGELOG: thêm vào mục "Chưa phát hành" → "Mới".
3. Checklist: mục "Đợt 10: Lặp lại lần chụp trước", mọi dòng `[ ]` (bốn dòng phím, DPI, tọa độ âm, rút màn hình, khởi động lại, Ghim lặp, Tải lên khi thiếu R2, tệp hỏng, English, kiểm thử `#[ignore]` mới lấy tên bằng `grep`).
4. `scripts/demo-repeat.ps1`: theo mẫu `demo-pin.ps1`, phím vùng Ctrl+Alt+Shift+F10 (vk 121) và phím lặp Ctrl+Alt+Shift+F11 (vk 122) bật sẵn; tạo sẵn `last_capture.json` tổng hợp (vùng 200,200,640,360, action "save") trong hồ sơ cách ly.
5. Website: một ý ngắn trong mục phím tắt/chụp (EN trong `index.html`, VI cùng khóa trong `vi.json`) về "Repeat last capture"; chỉ điều có thật. `npm ci` + `npm run build` trong `website/` phải qua.
6. `powershell -File scripts/check.ps1` → `ALL CHECKS PASSED`. Commit `docs:`; không email, đường dẫn cá nhân, khóa.
