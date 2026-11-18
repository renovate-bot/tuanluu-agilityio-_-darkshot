# Task 75: Lặp lại lần chụp trước — dòng phím tắt thứ tư trong Tùy chọn

**Agent:** `builder` · **Model:** sonnet (mẫu "Dòng phím tắt mới" đã có; bố cục có kiểm thử) · **Nhánh:** `task/75-repeat-options` · **Rà soát:** `reviewer` (opus) · **Phụ thuộc:** 74

Thiết kế: `docs/superpowers/specs/2026-11-18-repeat-capture-design.md` mục 2.1, 6.4. Global Constraints: kế hoạch `docs/superpowers/plans/2026-11-18-repeat-capture.md`.

## Cần đọc
`memory-bank/systemPatterns.md` mục "Dòng phím tắt mới" và "Thêm một ô hoặc một tab"; `crates/darkshot-windows/src/settings.rs`: `Layout`, `HK_BOX`, các mảng `hk_*` theo `HotkeyRole::ALL`, tab Phím tắt (~2460), bắt phím (`St.capturing`), `Form.hotkeys`, kiểm thử bố cục và xung đột; `i18n.rs` (`LabelHotkeyRepeatLast`, `hotkeys_conflict`).

## Files
`crates/darkshot-windows/src/settings.rs` (và `i18n.rs` chỉ nếu một câu ghi chú tab cần chữ mới).

## Yêu cầu
1. Tab Phím tắt có bốn dòng theo `HotkeyRole::ALL` (dòng mới cuối): ô tích (nhãn "Chụp lại vùng trước"), ô chỉ đọc, nút "Đổi…"; bắt phím, xung đột, lưu, Hủy (khôi phục và đăng ký lại) giống ba dòng cũ.
2. Bố cục: không cắt chữ, không chồng, cửa sổ đủ cao cho bốn dòng ở Việt/English, DPI 96/120/144/192 (kiểm thử thuần trên hàm bố cục theo mẫu sẵn có). Thứ tự Tab theo dòng.
3. Mọi mảng/chỉ số cỡ 3 viết cứng đổi theo `HotkeyRole::ALL.len()`.

## Kiểm thử (viết trước)
Bố cục bốn dòng không chồng/cắt; `Form` khứ hồi bốn dòng; xung đột dòng 4 với dòng 1 được báo; kiểm thử cũ chỉ cập nhật ở chỗ số dòng đổi theo thiết kế.

TDD: `test:` rồi `feat:`. `cargo fmt --all`; `powershell -File scripts/check.ps1` → `ALL CHECKS PASSED`.

## Review focus
Chỉ số cứng 3 còn sót; tab khác không đổi; bố cục DPI; bắt phím dòng 4 rồi Hủy.
