# Task 84: Chụp từng bước — lõi

**Agent:** `builder` · **Model:** sonnet · **Nhánh:** `task/84-steps-core` (từ `feature/step-capture`) · **Rà soát:** `reviewer` (sonnet)

Thiết kế: `docs/superpowers/specs/2026-11-18-step-capture-design.md` mục 2, 3 (phần core). Global Constraints: kế hoạch `docs/superpowers/plans/2026-11-18-step-capture.md`.

## Cần đọc
`crates/darkshot-core/src/actions.rs` (`Action`, xử lý `Pin`/`Ocr`), `last_capture.rs` (`remember`), `naming.rs` (`LocalTime`, định dạng tên tệp), `history.rs`/`history_store.rs` (mẫu JSON có phiên bản, kiểm đầu vào), `i18n.rs`. `grep -rn "Action::Ocr" crates/` để thêm nhánh `AddStep` mọi nơi.

## Files
Tạo `crates/darkshot-core/src/steps.rs`; sửa `lib.rs`, `actions.rs`, `last_capture.rs`, `i18n.rs`. Sửa tối thiểu lớp Windows chỉ để biên dịch (nhánh `AddStep` no-op, tooltip map sang `Msg::OverlayAddStep`; KHÔNG thêm ô thanh ngang — Task 85).

## Interfaces
Như thiết kế mục 3: `StepSession`, `Step`, `MAX_STEPS = 99`, `MAX_CAPTION = 2000`, `add`, `remove`, `move_up`, `move_down`, `set_caption`, `next_file_name`, `to_json`, `from_json`, `export_names`, `markdown(lang, when, steps)`, `export_dir_name(when)`. Lỗi `add` khi đủ → `Err(Full)`. Chỉ số ngoài phạm vi → không làm gì (trả `false`). `from_json` chỉ nhận `file` khớp `^\d{3}\.png$` (chống đường dẫn lạ), bỏ trùng.

## Chuỗi (Việt / English)
`TrayStepsStart` "Chụp từng bước" / "Step capture"; hàm `tray_steps_review(lang, n)` "Xem lại các bước ({n})" / "Review steps ({n})"; `OverlayAddStep` "Thêm bước (Ctrl+N)" / "Add step (Ctrl+N)"; `StepsBarTitle` "Chụp từng bước" / "Step capture"; hàm `steps_count(lang, n)` "{n} bước" / "{n} step(s)" (English: "1 step", "2 steps"); `StepsCapture` "Chụp bước" / "Capture step"; `StepsReview` "Xem lại" / "Review"; `StepsFinish` "Kết thúc" / "Finish"; `StepsWindowTitle` "Các bước" / "Steps"; hàm `step_label(lang, k)` "Bước {k}" / "Step {k}"; `StepsCaption` "Ghi chú" / "Caption"; `StepsUp` "Lên" / "Up"; `StepsDown` "Xuống" / "Down"; `StepsDelete` "Xóa bước" / "Delete step"; `StepsExport` "Xuất…" / "Export…"; `StepsEndSession` "Kết thúc phiên" / "End session"; `StepsClose` "Đóng" / "Close"; `StepsEmpty` "Chưa có bước nào. Bấm Chụp bước trên thanh phiên." / "No steps yet. Click Capture step on the session bar."; hàm `steps_end_confirm(lang, n)` "Kết thúc mà không xuất? {n} bước sẽ bị xóa." / "End without exporting? {n} step(s) will be deleted."; `StepsEndConfirmYes` "Kết thúc" / "End"; `StepsCancel` "Hủy" / "Cancel"; `StepsFullTitle` "Đã đủ 99 bước" / "99 steps reached"; `StepsFullBody` "Xuất hoặc kết thúc phiên để chụp thêm." / "Export or end the session to capture more."; hàm `steps_exported(lang, n)` "Đã xuất {n} bước" / "Exported {n} step(s)"; `StepsExportFailedTitle` "Không xuất được các bước" / "Couldn't export the steps"; Markdown: "Các bước"/"Steps", "Bước {k}"/"Step {k}". Trợ giúp: một đoạn ngắn về Chụp từng bước.

## Kiểm thử (viết trước)
Thao tác danh sách (thêm đến 99, Full; xóa; lên/xuống ở biên; caption cắt 2000 ký tự có dấu); `next_file_name` không trùng sau xóa; JSON khứ hồi, từ chối version lạ/`file` lạ (`../x.png`, `1.png`, `001.PNG.exe`); `markdown` hai ngôn ngữ (bước có/không ghi chú, ghi chú nhiều dòng, ký tự `#` đầu dòng trong ghi chú không biến thành tiêu đề — thoát bằng `\`); `export_dir_name` định dạng; `run_action(AddStep)` no-op; `remember(prev, AddStep, r) == prev`; i18n hai bản khác nhau.

TDD `test:` rồi `feat:`. `cargo fmt --all`; `powershell -File scripts/check.ps1` → `ALL CHECKS PASSED`.
