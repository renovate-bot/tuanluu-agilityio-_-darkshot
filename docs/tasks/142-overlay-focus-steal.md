# Task 142: Lớp phủ không tự hủy khi chương trình khác giành tiêu điểm thoáng qua

**Agent:** `builder` · **Model:** sonnet (sửa nhỏ, khoanh vùng trong `overlay.rs`; không bảo mật, không dữ liệu) · **Nhánh:** `task/142-overlay-focus-steal` (từ `main`), worktree `D:\2026 - Tuan\shotme-142` · **Rà soát:** `reviewer` (opus; không dùng fable, quyết định 154)

## Lỗi (chủ dự án báo, 2026-11-23)

"Khi select phủ một vùng và chờ. Ứng dụng tự động tắt sau vài giây." Tiến trình không chết (không có sự kiện lỗi, `darkshot.exe` chạy liên tục): lớp phủ đóng. Phiên điều phối theo dõi `EVENT_SYSTEM_FOREGROUND` (thụ động) trong hai lần chủ dự án thử:

| Lớp phủ `darkshot-overlay` lên trước | Mất tiêu điểm | Cửa sổ giành |
|---|---|---|
| 13:49:40.065 | 13:49:44.983 | `MousePositioningWnd1.exe` lớp `MaskWnd` |
| 13:50:22.634 | 13:50:28.268 | `MousePositioningWnd1.exe` lớp `MaskWnd` |

`C:\Program Files\Lenovo\Lenovo Accessories and Display Manager\MousePositioningWnd1.exe` được chạy lại ~21 s một lần, mỗi lần một cửa sổ `MaskWnd` lên trước rồi tắt ngay (vài chục ms, tiêu điểm trả về cửa sổ trước đó). `FloatingMenu1.exe` cùng phần mềm cũng làm vậy. Nguyên nhân phía darkshot: `overlay.rs` `WM_ACTIVATE` (~5047) đặt `result = Some(None)` (hủy) với MỌI `WA_INACTIVE`, nên một cửa sổ thoáng qua hủy luôn vùng chọn và nét vẽ.

## Files

`crates/darkshot-windows/src/overlay.rs` (xử lý `WM_ACTIVATE`, `WM_KEYDOWN`/`WM_SYSKEYDOWN`, hẹn giờ lấy lại tiêu điểm, kiểm thử); `CHANGELOG.md`; `README.md` (chỉ nếu README có câu nói lớp phủ đóng khi mất tiêu điểm / Alt+Tab — tìm và sửa cho đúng); `website/index.html` + `website/src/vi.json` (chỉ nếu website có câu như vậy); `memory-bank/decisions.md` (quyết định mới, số kế tiếp sau mục cuối); `docs/testing/manual-checklist.md` (một mục thử tay).

## Yêu cầu

1. **Quy tắc mới** khi lớp phủ nhận `WM_ACTIVATE` với `WA_INACTIVE`:
   - Người dùng chủ động chuyển cửa sổ → hủy như cũ. "Chủ động" = lúc đó phím Alt hoặc Win đang giữ (`GetAsyncKeyState` `VK_MENU`, `VK_LWIN`, `VK_RWIN`, bit cao), HOẶC lớp phủ đã nhận lần nhấn Alt/Win (`WM_SYSKEYDOWN`/`WM_KEYDOWN` với `VK_MENU`/`VK_LMENU`/`VK_RMENU`/`VK_LWIN`/`VK_RWIN`) trong vòng `SWITCH_KEY_WINDOW_MS` (đề xuất 1500 ms; Start mở khi NHẢ phím Win nên phải nhớ lần nhấn). Ghi thời điểm bằng `GetTickCount64` hoặc `Instant` trong `State`.
   - Còn lại (chương trình khác tự giành) → KHÔNG hủy; giữ nguyên vùng chọn, nét vẽ, ô chữ, bảng màu. Đặt lại `focus_retries = 0` và bật lại `FOCUS_TIMER_ID` (cơ chế sẵn có: `take_focus` mỗi `FOCUS_RETRY_MS`, tối đa `MAX_FOCUS_RETRIES`). Không gọi `SetForegroundWindow` ngay trong `WM_ACTIVATE` (đang trong lúc chuyển kích hoạt) — để hẹn giờ làm. Hết lượt thử mà chưa lấy lại được thì lớp phủ vẫn mở (luôn trên cùng; Esc toàn cục `ESC_HOTKEY_ID` và bấm chuột vào lớp phủ vẫn dùng được) — không vòng lặp giành tiêu điểm vô hạn.
   - Tách quyết định thành hàm thuần (ví dụ `fn deactivation_cancels(switch_key_down: bool, since_switch_key_ms: Option<u64>) -> bool`) có kiểm thử đơn vị: Alt/Win đang giữ → hủy; nhấn trong cửa sổ thời gian → hủy; ngoài cửa sổ hoặc chưa từng nhấn → không hủy; biên `SWITCH_KEY_WINDOW_MS`.
   - Esc, nút Hủy, `WM_CLOSE`/`WM_DESTROY`, `WM_QUIT` giữ nguyên. Phím tắt Esc toàn cục phải còn đăng ký khi lớp phủ mất tiêu điểm (trừ lúc ô chữ mở như hiện nay) — kiểm và ghi trong báo cáo.
   - Đang kéo chọn/vẽ (có `SetCapture`) mà bị giành: hành vi `WM_CAPTURECHANGED` hiện có giữ nguyên; ghi trong báo cáo điều xảy ra.
   - Ô chữ đang gõ / IME: sau khi lấy lại tiêu điểm vẫn gõ tiếp được (con trỏ nhấp nháy, `SetFocus`); ghi trong báo cáo.
2. **Kiểm thử `#[ignore]`** theo mẫu kiểm thử lớp phủ sẵn có (luồng phụ tìm `CLASS` và `PostMessageW`, `pick_region` trên luồng kiểm thử, ~5760–5830): tạo một cửa sổ cấp cao nhỏ của CHÍNH tiến trình kiểm thử trên một luồng khác (luồng khác để khác hàng đợi nhập), chọn vùng bằng thông điệp, rồi cho cửa sổ đó `SetForegroundWindow` (mô phỏng `MaskWnd`) và tự hủy sau ~50 ms:
   - không có phím chuyển → lớp phủ còn mở sau ~1 s, lấy lại tiêu điểm (`GetForegroundWindow` == lớp phủ, nếu Windows cho phép; nếu khóa tiêu điểm chặn thì ghi rõ), rồi bấm Lưu (thông điệp) → `Ok(Some(Picked{ rect: vùng đã chọn, … }))`;
   - đăng `WM_SYSKEYDOWN` `VK_MENU` vào lớp phủ trước khi giành → `Ok(None)`.
   Không giả lập chuột/phím thật (không `SendInput`, `keybd_event`, `mouse_event`) — chủ dự án đang dùng máy. Chạy tay các `#[ignore]` sẵn có của lớp phủ liên quan (vòng Lưu/Esc/phím Esc toàn cục, GDI) và ghi số thật.
3. CHANGELOG "Chưa phát hành" → "Sửa lỗi": lớp phủ chọn vùng không còn tự đóng khi một chương trình khác giành tiêu điểm thoáng qua (ví dụ phần mềm màn hình/chuột chạy nền); Alt+Tab, phím Win vẫn đóng như trước. Quyết định mới (ngày 2026-11-23, nguyên nhân + số đo trên, quy tắc, hằng số). Một mục thử tay trong checklist (Lenovo Accessories and Display Manager hoặc chương trình tự lên trước; Alt+Tab; Win; Win+L; UAC; gõ chữ rồi bị giành).
4. `cargo fmt --all`; `powershell -File scripts/check.ps1` in `ALL CHECKS PASSED`. Commit `fix:` / `test:` / `docs:` trên nhánh; không gộp, không đẩy. Báo cáo: thay đổi, kết quả kiểm thử (số thật), điều chưa kiểm được, token nếu biết.
