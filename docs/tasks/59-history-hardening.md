# Task 59: Gia cố Lịch sử – hàng đợi ghi, flush khi thoát, trao lượt tải lên đúng một lần, thanh cuộn khi thẻ mở

**Agent:** `specialist` · **Model:** opus (đồng thời, vòng đời tiến trình, toàn vẹn dữ liệu) · **Nhánh:** `task/59-history-hardening` (tách từ `feature/window-snap`) · **Rà soát:** `specialist` (opus)

Thiết kế: `docs/superpowers/specs/2026-11-18-window-snap-design.md` mục 1 (đọc kỹ bảng 1.1 và mục 1.2). Global Constraints: mục cùng tên của `docs/superpowers/plans/2026-11-18-window-snap.md`.

## Cần đọc
`crates/darkshot-windows/src/history_service.rs` (toàn bộ), `app.rs` (`PendingUploads`, `save_pending`, `UploadJob`, `spawn_upload` và kiểm thử của chúng), `main.rs` (`save_pending_uploads`, `run_on_image`, nhánh `WM_ENDSESSION`, cuối `main()`), `history_window.rs` (`update_scrollbar`, `relayout`, `open_confirm`, `close_confirm`), `crates/darkshot-core/src/history_store.rs` (`write_index`, `write_atomic`, khóa `LOCK`).

## Files
Sửa: `darkshot-windows/src/{history_service.rs, app.rs, main.rs, history_window.rs}`, `darkshot-core/src/history_store.rs`.

## Yêu cầu

### A. Hàng đợi ghi một luồng (`history_service.rs`)
- Thay "mỗi mục một luồng" bằng **một** luồng `darkshot-history` sống suốt tiến trình, sinh lười ở lần đẩy việc đầu tiên, nhận việc qua `std::sync::mpsc` (hoặc `Mutex<VecDeque>` + `Condvar`). Ghi tuần tự theo thứ tự đẩy vào.
- Bộ đếm việc chưa xong (đẩy vào +1, ghi xong hoặc lỗi −1) để `flush` biết khi nào rỗng. Lỗi/panic khi ghi: nuốt, `fail()`, bộ đếm vẫn giảm (dùng guard để giảm cả khi panic).
- Không tạo được luồng: ghi đồng bộ ngay trên luồng gọi? KHÔNG (không I/O trên luồng giao diện) → `fail()` và bỏ việc; lần sau thử sinh lại.
- Ảnh: nhận `Arc<Image>` để khỏi sao chép lần hai khi người gọi đã có `Arc` (lượt tải lên). Giữ hàm công khai cũ dùng `&Image` nếu tiện (sao chép một lần như cũ), thêm biến thể `Arc`.
- API (giữ tên cũ, thêm mới):
```rust
pub fn record_outcome(outcome: &Outcome, img: &Image, expiry: Expiry, now: LocalTime); // đẩy vào hàng, không chặn
pub fn record_outcome_arc(outcome: &Outcome, img: Arc<Image>, expiry: Expiry, now: LocalTime);
pub fn record_outcome_blocking(...);                    // giữ nguyên: ghi đồng bộ (lúc thoát)
pub fn flush(timeout: Duration) -> bool;               // true: mọi việc đã đẩy trước lời gọi đã xong
```
- Để kiểm thử được: lõi của hàng đợi là một kiểu nội bộ (ví dụ `Writer { root: PathBuf, … }`) tạo được với thư mục tạm; hàm công khai dùng một thể hiện toàn cục với `paths::history_dir()`. Không kiểm thử trên `%LOCALAPPDATA%` thật.

### B. Trao lượt tải lên đúng một lần (`app.rs`)
- `PendingUploads::finish(&self, id: u64, f: impl FnOnce()) -> bool`: giữ khóa sổ; nếu lượt còn trong sổ thì rút nó, chạy `f` **khi vẫn giữ khóa**, trả `true`; nếu không còn (đã bị `drain` lúc thoát) thì không chạy `f`, trả `false`.
- `spawn_upload`: thay `reg.remove(id)` + gọi móc bằng `reg.finish(id, || hook(&outcome, &job.image))` khi có sổ; không có sổ (`pending: None`) thì gọi móc như cũ. Móc `on_outcome` đổi kiểu để nhận được `&Arc<Image>` nếu cần cho `record_outcome_arc` (tùy, nêu trong báo cáo).
- `on_needs_settings` vẫn gọi như cũ (không phụ thuộc kết quả `finish`).
- `main::save_pending_uploads`: thứ tự `drain()` → `save_pending(..., record_outcome_blocking)` → `history_service::flush(Duration::from_secs(3))`. Gọi ở cả cuối `main()` và `WM_ENDSESSION` (như hiện nay). Gọi hai lần liên tiếp phải vô hại.

### C. Thanh cuộn khi thẻ xác nhận mở (`history_window.rs`)
- Trong `update_scrollbar`: sau `SetScrollInfo`, nếu thẻ đang mở thì `EnableScrollBar(list, SB_VERT, ESB_DISABLE_BOTH)`; nếu không thì `ESB_ENABLE_BOTH` chỉ khi nội dung dài hơn khung nhìn (để `SIF_DISABLENOSCROLL` vẫn đúng khi không cần cuộn). Bảo đảm `relayout` gọi lại khi thẻ đóng (đã có). Nếu cần, thêm import `EnableScrollBar`, `ESB_*` (feature `Win32_UI_Controls`/`WindowsAndMessaging` đã bật; kiểm tra module đúng của crate `windows` 0.62).
- Tách quyết định thành hàm thuần `scrollbar_arrows(card_open: bool, content: i32, view: i32) -> ScrollArrows { Disabled, Enabled, Auto }` (hoặc tương đương) và kiểm thử.

### D. Bền vững trên đĩa (`history_store.rs`)
- `write_index` và `write_atomic`: ghi tệp tạm bằng `File::create` + `write_all` + `sync_all` rồi mới `rename`. Lỗi `sync_all` coi như lỗi ghi (xóa tệp tạm, trả lỗi) – giữ nguyên hành vi lỗi hiện có. Kiểm thử hiện có phải qua nguyên.

### E. Hạn của mục tải lên
- KHÔNG đổi (thiết kế 1.1 #4). Thêm chú thích một dòng ở chỗ móc tính `now` giải thích vì sao lấy lúc tải xong.

## Kiểm thử (viết trước, phải thấy thất bại)
- `writer_flush_waits_for_queued_entries`: đẩy 5 mục vào `Writer` trên thư mục tạm, `flush(5s)` trả `true`, `Store::load` thấy đủ 5 mục, đúng thứ tự thời gian.
- `writer_flush_times_out`: một việc chặn (ví dụ móc kiểm thử giữ một `Barrier`/`Mutex`) → `flush(50ms)` trả `false`, rồi thả ra → `flush(5s)` `true`.
- `writer_uses_one_thread`: đẩy nhiều mục, chỉ một luồng tên `darkshot-history` được tạo (đếm qua móc kiểm thử hoặc id luồng ghi lại).
- `writer_panic_does_not_wedge_flush`: một ảnh hỏng (bgra ngắn) → `failure_count` tăng, `flush` vẫn trả `true`.
- `disabled_spawns_no_thread_and_no_dir` (giữ ý của `disabled_does_nothing`).
- `finish_after_drain_skips_hook`: `add`, `drain`, rồi `finish(id, f)` → `false`, `f` không chạy.
- `drain_after_finish_sees_nothing`: `add`, `finish` → `true`, `f` chạy đúng một lần, `drain` rỗng.
- `finish_runs_hook_under_lock`: một luồng `finish` với `f` chậm (ngủ 100 ms), luồng khác `drain` ngay → `drain` chỉ trả sau khi `f` xong và không thấy lượt (dùng mốc thời gian hoặc cờ).
- `spawn_upload_records_once_when_drained_first`: sổ thật, `drain` trước khi luồng tải lên kết thúc (uploader giả chờ một tín hiệu) → móc `on_outcome` không được gọi.
- Kiểm thử cũ `on_outcome_called_once_*` vẫn qua.
- `scrollbar_arrows` cho ba trường hợp.
- `history_store`: kiểm thử cũ qua; thêm `index_written_via_sync_path` nếu có cách kiểm (không bắt buộc).

## Việc cần chủ dự án thử tay
Mục "Đợt 8 – Lịch sử" của checklist (phiên điều phối thêm): thoát ngay sau Lưu thấy mục; thẻ xác nhận mở thì thanh cuộn mờ.

## Báo cáo
Theo mẫu `.claude/agents/specialist.md`. Nêu rõ: thứ tự thoát, giới hạn `flush`, mọi chỗ làm khác gói.
