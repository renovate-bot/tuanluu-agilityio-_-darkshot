# Task 132: Privacy Shield — Chụp từng bước, Lịch sử, Ghi GIF

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/132-privacy-steps-history` (từ `feature/privacy-shield`), worktree `D:\2026 - Tuan\shotme-privacy` · **Rà soát:** `reviewer` (fable)

Thiết kế: `docs/superpowers/specs/2026-11-22-sensitive-info-redaction-design.md` mục 8.1 (các hàng Lịch sử, Ghi GIF, Xuất phiên), 9, 10, 14 (gạch "Cổng"), 18.1, 18.5. Global Constraints: `docs/superpowers/plans/2026-11-22-sensitive-info-redaction.md`. Quyết định 122, 123, 124 trong `memory-bank/decisions.md`.

## Cần đọc
`crates/darkshot-windows/src/privacy_gate.rs` (Task 131: `submit`, `submit_with`, `OutputAction`, `Done`), `steps_store.rs` (`add`, `persist`, `restore_at`, `write_png`), `darkshot-core/src/steps.rs` (`Step`, `StepSession::from_json`/`to_json`), `steps_window.rs` (`start_export` ~1799, `EXPORT_DONE`), `steps_export.rs` (`ExportJob`, `fill`, `copy_new`), `history_window.rs` (`on_action` ~1940: nhánh `1` Chép ảnh và `BTN_PIN`, `post_pin_image`, `open_pin`, `on_status`), `record_session.rs` (thanh ghi, chuỗi trạng thái), `i18n.rs`.

## Files
Sửa `darkshot-core/src/steps.rs`, `crates/darkshot-windows/src/steps_store.rs`, `steps_window.rs`, `steps_export.rs`, `history_window.rs`, `record_session.rs`, `privacy_gate.rs` (chỉ thêm dispatch cho HistoryCopy/HistoryPin/StepExport nếu Task 131 chưa có), `main.rs` (nếu cần nối), `i18n.rs`.

## Yêu cầu
1. **`scanned` của bước:** `Step { file, caption, scanned: bool }`; JSON thêm `"scanned": true` chỉ khi true; thiếu = false; `session.json` vẫn v1; bản cũ đọc phiên mới vẫn được (kiểm thử `from_json` với khóa lạ — xác nhận serde của bản cũ không `deny_unknown_fields`; nếu có thì báo, không đổi định dạng). `steps_store::add(prepared)` ghi `scanned = prepared.protection().scanned()`. Xác nhận lại (kiểm thử) rằng khi Shield bật không có `.png`/`.tmp` nào được ghi vào thư mục phiên trước khi cổng có `PreparedOutput` (Hủy → thư mục không đổi, `session.json` không đổi).
2. **Xuất phiên (mục 18.1):** Shield tắt → như cũ. Shield bật và có bước `scanned == false`: trên luồng giao diện, lần lượt cho từng bước đó: giải mã PNG trên luồng nền → `privacy_gate::submit_with(StepExport { step, steps }, image, done)` → `done(Some(prepared))` lưu `(chỉ số, Image)`; `done(None)` (Hủy) → hủy cả lượt xuất (không tạo thư mục, báo "Đã hủy xuất"), không quét tiếp. Xong hết → `ExportJob` thêm `replacements: Vec<(usize, Image)>`; `fill` ghi `encode_png` của ảnh thay thế (tạo mới, không ghi đè) thay cho `copy_new` ở đúng các bước đó. Nút Xuất bị khóa (`EXPORTING`) suốt quá trình quét. Không viết lại tệp trong `steps\current`. Phiên kết thúc/thoát giữa chừng → bỏ lượt xuất, thả ảnh. Lỗi giải mã → báo lỗi xuất như hiện nay.
3. **Lịch sử:** Chép ảnh (nút 1) và Ghim: sau khi giải mã ở luồng nền, gửi ảnh về luồng giao diện (mẫu `post_pin_image`) rồi `privacy_gate::submit(HistoryCopy | HistoryPin, image)` (Shield tắt → cổng dispatch ngay `unscanned`). Dispatch HistoryCopy: `WinClipboard.set_image(prepared.image())` + trạng thái "Đã chép ảnh"/lỗi trên cửa sổ Lịch sử (nếu còn mở). HistoryPin: `pin::open(prepared, CenterOnCursorMonitor hoặc chỗ cũ)`. Mở / Hiện trong thư mục / Chép link không đổi. Mục mới ghi Lịch sử vẫn đến từ `run_on_image` (Task 131): kiểm thử xác nhận ảnh ghi vào `history\images` là ảnh đã che.
4. **Ghi GIF:** khi Shield bật lúc bắt đầu ghi, thanh ghi hiện thêm dòng/nhãn "GIF không được quét" (vi) / "GIF is not scanned" (en), màu `TEXT_MUTED`, không đổi kích thước tối thiểu của thanh quá mức (kiểm thử bố cục 4 DPI). Không đổi gì khác của ghi GIF.
5. Kiểm thử: `scanned` đọc/ghi/thiếu; Hủy Thêm bước không ghi gì; xuất với 0/1/nhiều bước chưa quét, Hủy giữa chừng, `fill` dùng ảnh thay thế (đọc lại PNG ra đúng điểm đen); Lịch sử Chép/Ghim qua cổng (sink giả) đúng một lần, Shield tắt như cũ; GIF nhãn hiện/ẩn. Không giả lập chuột/phím thật.
6. Exe trước/sau.

## Ghi chú từ Task 131 (đã gộp)
- Lịch sử → Ghim ĐÃ qua cổng (`history_window.rs` `open_pin` → `privacy_gate::submit(HistoryPin, img)`). Task này chỉ còn Lịch sử → Chép ảnh (`history_window.rs` ~1984 `set_image` thẳng): thêm dispatch `HistoryCopy` trong `main::gate_output` (hiện bỏ qua) và báo trạng thái về cửa sổ Lịch sử nếu còn mở.
- `HistoryCopy`/`StepExport`/`submit_with` đang có `allow(dead_code)` ngoài kiểm thử: gỡ khi dùng. `Done` KHÔNG được gọi khi `submit_with` trả `Err` hoặc khi `shutdown`: bên xuất phiên phải tự xử lý cả hai (Err → hủy xuất, báo lỗi; shutdown → bỏ).
- `submit*` chỉ trên luồng chính (`debug_assert`). Chưa có nguồn cấu hình/cổng đóng → `Err(WindowFailed)`.
- `steps_store::session_id()` có sẵn; lượt Thêm bước của phiên khác bị cổng bỏ.
- Cổng chặn mở cửa sổ khi `output_windows_blocked()` (lớp phủ mở hoặc chụp cuộn/ghi GIF).
