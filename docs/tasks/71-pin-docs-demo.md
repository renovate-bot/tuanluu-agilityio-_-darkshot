# Task 71: Ghim ảnh — tài liệu, kịch bản demo

**Agent:** `scaffolder` · **Model:** haiku (tài liệu cơ học, không mã sản phẩm) · **Nhánh:** `task/71-pin-docs-demo` (tách từ `feature/pin-screenshot`) · **Rà soát:** phiên điều phối đọc diff · **Phụ thuộc:** 70

Thiết kế: `docs/superpowers/specs/2026-11-18-pin-screenshot-design.md`. Global Constraints: mục cùng tên của `docs/superpowers/plans/2026-11-18-pin-screenshot.md`. Phần memory bank, CLAUDE.md, ảnh QA và quyết định do phiên điều phối làm, KHÔNG thuộc task này.

## Cần đọc
`README.md` (mục tính năng, cách dùng, chạy demo cách ly), `CHANGELOG.md` (định dạng mục phiên bản), `docs/testing/manual-checklist.md` (mục "Đợt 8" làm mẫu), `scripts/demo-snap.ps1` (mẫu kịch bản demo), `crates/darkshot-core/src/i18n.rs` (chữ thật của các mục `Pin*`, `OverlayPin`, `HistoryPin` để tài liệu dùng đúng chữ).

## Files
`README.md`, `CHANGELOG.md`, `docs/testing/manual-checklist.md`, tạo `scripts/demo-pin.ps1`.

## Yêu cầu
1. README: mục ngắn "Ghim ảnh lên màn hình" (cách ghim từ lớp phủ — nút Ghim/Ctrl+P — và từ Lịch sử; kéo, cuộn, Ctrl+cuộn, bấm đúp, chuột phải, Esc, menu khay "Đóng mọi ảnh ghim"; tối đa 32). Thêm `scripts/demo-pin.ps1` vào phần chạy demo cách ly. KHÔNG đổi mục "Tải về" hay số phiên bản (chưa phát hành).
2. CHANGELOG: thêm mục `## [Chưa phát hành]` ở trên `0.3.0` với nhóm "Mới" (Ghim ảnh lên màn hình), "Sửa lỗi" (để trống hoặc ghi theo báo cáo Task 66 mà phiên điều phối dán vào lời giao), "Đã biết". Theo định dạng các mục cũ.
3. Checklist: mục mới "Đợt 9: Ghim ảnh lên màn hình" ở cuối, mỗi dòng `[ ]`, phủ: ghim từ lớp phủ (nút, Ctrl+P, khít vị trí, có hình đánh dấu), ghim từ Lịch sử (giữa màn hình, ≤ 80%), kéo, cuộn quanh chuột, Ctrl+cuộn, bấm đúp, phím (+ − 0 mũi tên Esc Ctrl+W Ctrl+C Ctrl+S), menu ngữ cảnh và dấu tích, nút × khi rê, nhãn mức, menu khay có/không ảnh ghim, giới hạn 32, DPI 100/125/150/200%, hai màn hình DPI khác nhau, màn hình tọa độ âm, kéo qua màn hình khác, rút màn hình, thoát khi còn ảnh ghim, English, Windows 10, các kiểm thử `#[ignore]` mới (lấy tên từ mã: `grep -n "fn pin_" crates/darkshot-windows/src/*.rs`).
4. `scripts/demo-pin.ps1`: chép mẫu `demo-snap.ps1` (hồ sơ cách ly `DARKSHOT_PROFILE_DIR`, cấu hình tổng hợp không có R2, `history_enabled: true`, `check_updates: false`, `autostart: false`, phím Ctrl+Alt+Shift+F9, dữ liệu Lịch sử tổng hợp qua `make_demo_store`), tham số `-Exe`, `-Lang vi|en`, thư mục mặc định `%TEMP%\darkshot-demo-pin`; chú thích đầu tệp ASCII không dấu như mẫu.

## Tiêu chí nghiệm thu
Tài liệu khớp chữ thật trong `i18n.rs`; không chứa khóa, đường dẫn cá nhân hay email; `scripts/check.ps1` in `ALL CHECKS PASSED`.
