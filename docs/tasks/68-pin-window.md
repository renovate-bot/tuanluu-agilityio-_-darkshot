# Task 68: Ghim ảnh — cửa sổ ảnh ghim (Win32)

**Agent:** `specialist` · **Model:** opus (cửa sổ Win32 mới: vòng đời, GDI, DPI theo màn hình, layered window, chuột/phím, hẹn giờ) · **Nhánh:** `task/68-pin-window` (tách từ `feature/pin-screenshot`) · **Rà soát:** `reviewer` (fable) · **Phụ thuộc:** 67

Thiết kế: `docs/superpowers/specs/2026-11-18-pin-screenshot-design.md` mục 4.2–4.7, 5 (toàn bộ). Global Constraints: mục cùng tên của `docs/superpowers/plans/2026-11-18-pin-screenshot.md`. Task 67 (`darkshot_core::pin`, `Action::Pin`, chuỗi i18n) đã ở nhánh phát triển.

## Cần đọc
`crates/darkshot-core/src/pin.rs`, `i18n.rs` (mục `Pin*`), `theme.rs`; `crates/darkshot-windows/src/`: `winutil.rs` (`create_popup`, phông, DPI, `dialog_pre_translate`), `theme.rs` (`enable_dark_menus`, `brush`, vẽ chữ), `help.rs`/`about.rs` (mẫu cửa sổ không chặn: `show`/`close`/dọn ở `WM_NCDESTROY`), `history_window.rs` (cách tạo DIB và `StretchDIBits`/vẽ ảnh, hẹn giờ, `WM_DPICHANGED` — đọc theo đoạn bằng `grep -n`), `clipboard.rs` (`WinClipboard::set_image`), `capture.rs` (danh sách màn hình, `Image`), `lang.rs`, `main.rs` (`with_app`, cách hành động Lưu/Chép chạy: `run_on_image`). Kiểm `Cargo.toml` của `darkshot-windows` để biết feature `windows` đã bật.

## Files
Tạo `crates/darkshot-windows/src/pin.rs`; thêm `mod pin;` ở `main.rs` (chỉ dòng khai báo module, KHÔNG nối luồng chụp — Task 69). Feature `windows` mới chỉ nếu thiếu (ghi trong báo cáo).

## Interfaces (hợp đồng với Task 69–70)
```rust
pub enum PinPlace {
    /// Ảnh hiện 100% với góc trên trái của ẢNH tại điểm màn hình này (cửa sổ = ảnh nới một viền ra ngoài).
    At(darkshot_core::geometry::Point),
    /// Căn giữa vùng làm việc của màn hình chứa con trỏ, mức `pin::fit_zoom`.
    CenterOnCursorMonitor,
}
pub enum PinError { Limit, Create(String) }
/// Tạo một ảnh ghim (sở hữu `image`). Gọi trên luồng giao diện chính.
pub fn open(image: darkshot_core::image::Image, place: PinPlace) -> Result<(), PinError>;
pub fn count() -> usize;
/// Hủy mọi ảnh ghim (đồng bộ). Gọi được nhiều lần.
pub fn close_all();
/// Hàm `main` đăng ký để Lưu/Chép từ ảnh ghim đi đúng đường `run_on_image` (thông báo, Lịch sử).
pub fn set_action_sink(f: fn(darkshot_core::actions::Action, darkshot_core::image::Image));
```
Menu khay đọc `count()` lúc mở, nên không cần thông báo khi số ảnh ghim đổi.
`open` khi `!pin::can_add(count())` → `Err(PinError::Limit)` (không tạo cửa sổ). Người gọi (Task 69/70) ra thông báo.

## Yêu cầu
1. Lớp cửa sổ `darkshot.pin` đăng ký một lần (`OnceCell`/cờ `thread_local`). Kiểu: `WS_POPUP`, `WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED`; `SetLayeredWindowAttributes(LWA_ALPHA, 255)` ngay khi tạo; hiện bằng `SW_SHOWNA`. Tiêu đề: `PinWindowTitle` + " W × H".
2. Trạng thái `Box<Pin>` trong `GWLP_USERDATA`: ảnh gốc, DIB/bitmap nguồn tạo một lần, mức phóng, độ mờ, DPI hiện tại, cờ rê, trạng thái kéo (điểm bấm, góc cửa sổ lúc bấm), cờ nút × đang rê, hẹn giờ nhãn. Giải phóng mọi thứ ở `WM_NCDESTROY` (bitmap, DC, phông, `KillTimer`), xóa khỏi sổ. Không con trỏ thô treo.
3. Sổ `thread_local! { static PINS: RefCell<Vec<HWND>> }`. `close_all` chép danh sách rồi `DestroyWindow` từng cái (không giữ `borrow` khi gọi Win32 — tránh tái nhập).
4. Vẽ: `WM_PAINT` vẽ vào đệm (DC tương thích cỡ client) rồi `BitBlt` một lần. Viền 1 DIP (≥ 1 px) màu `ACCENT` khi kích hoạt (`WM_ACTIVATE`/`WM_NCACTIVATE` → vẽ lại), `OVERLAY_BORDER` khi không. Ảnh trong viền: mức 100% chép thẳng; < 100% `SetStretchBltMode(HALFTONE)` (+ `SetBrushOrgEx`); > 100% `COLORONCOLOR`. `WM_ERASEBKGND` trả 1.
5. Nút × (rê chuột, ảnh hiển thị ≥ 48 DIP mỗi chiều): ô 20 DIP, cách mép trên/phải 4 DIP; nền `OVERLAY_LABEL` (rê lên nút: `DANGER_BUTTON`), dấu × `TEXT` nét 1,5 DIP (GDI pen là được). Theo dõi rời chuột bằng `TrackMouseEvent(TME_LEAVE)`. Bấm × = đóng.
6. Nhãn mức/độ mờ (thiết kế 4.3): giữa đáy, cách đáy 8 DIP, đệm 8×4 DIP, bo 4 DIP, hẹn giờ 1200 ms rồi ẩn. Không vẽ khi ảnh nhỏ hơn nhãn + 8 DIP.
7. Chuột: kéo trái để dời (`SetCapture`, `WM_MOUSEMOVE` → `SetWindowPos` không đổi cỡ, `SWP_NOACTIVATE|SWP_NOZORDER`; nhả hoặc `WM_CAPTURECHANGED` kết thúc rồi `keep_visible`); bấm đúp (lớp có `CS_DBLCLKS`) về 100% giữ điểm dưới chuột; `WM_MOUSEWHEEL` (tích lũy delta theo `WHEEL_DELTA`, hỗ trợ bánh xe mịn) phóng/thu quanh chuột, kèm Ctrl thì độ mờ; `WM_RBUTTONUP`/`WM_CONTEXTMENU` (cả Shift+F10, phím Menu: đặt menu ở góc trên trái ảnh) mở menu ngữ cảnh. `WM_SETCURSOR`: `IDC_SIZEALL` trên ảnh, `IDC_ARROW` trên nút ×.
8. Phím (`WM_KEYDOWN`): Esc, Ctrl+W đóng; Ctrl+C → sink(`Action::Copy`, bản sao ảnh gốc); Ctrl+S → sink(`Action::Save`, bản sao); `VK_OEM_PLUS`/`VK_ADD` phóng, `VK_OEM_MINUS`/`VK_SUBTRACT` thu (quanh tâm cửa sổ); `0`/`VK_NUMPAD0` về 100%; mũi tên dời 1 px, Shift 10 px. Sink chưa đăng ký thì Ctrl+C chép trực tiếp bằng `WinClipboard`, Ctrl+S không làm gì.
9. Đổi mức: `pin::zoom_about` → khung ảnh mới → khung cửa sổ = nới viền → `keep_visible` theo danh sách màn hình hiện tại (`EnumDisplayMonitors`) → `SetWindowPos` một lần (đổi cả vị trí và cỡ). Đổi độ mờ: `SetLayeredWindowAttributes`. Mức không đổi (đã ở đầu/cuối) thì vẫn hiện nhãn mức hiện tại.
10. Menu ngữ cảnh (thiết kế 4.4): `CreatePopupMenu`, hai menu con Thu phóng (50/100/150/200 — mức nào vượt `MAX_SIDE` thì vô hiệu) và Độ mờ (100/80/60/40), dấu tích mức hiện tại (`MF_CHECKED` khi khớp đúng), `TrackPopupMenu(TPM_RETURNCMD | TPM_RIGHTBUTTON)`; `SetForegroundWindow` trước khi mở; hủy menu trên mọi đường. "Đóng mọi ảnh ghim" gọi `close_all` SAU khi `TrackPopupMenu` trả về (không hủy cửa sổ đang giữ menu giữa chừng — hoãn bằng `PostMessage` nếu cần).
11. DPI: `WM_DPICHANGED` KHÔNG dùng khung gợi ý; giữ góc trên trái và cỡ ẢNH (điểm ảnh), tính lại viền theo DPI mới (cỡ cửa sổ có thể đổi 1–2 px), phông nhãn, rồi `keep_visible`. `WM_DISPLAYCHANGE` → `keep_visible`.
12. `open(image, At(p))`: khung ảnh = (p, w×h ở 100%), cửa sổ = nới viền; `keep_visible`. `CenterOnCursorMonitor`: `MonitorFromPoint(GetCursorPos, MONITORINFO.rcWork)`, `fit_zoom`, `centered`. Kích thước 0 hoặc ảnh không hợp lệ → `Err(Create)`.
13. Không panic trong thủ tục cửa sổ (không `unwrap`/`expect`/chỉ số trực tiếp); mọi phép nhân cỡ dùng hàm của lõi hoặc i64 kẹp.
14. Chữ: tiêu đề, menu, nhãn độ mờ qua i18n theo `lang::current()` lúc dùng.

## Kiểm thử (viết trước)
- Thuần trong `pin.rs` (tách ra hàm không cần cửa sổ): `window_rect(image_rect, border)`, `close_button_rect(client, dpi)`, `close_button_visible(display_w, display_h, dpi)`, `badge_rect(client, text_size, dpi)` (+ ẩn khi ảnh nhỏ), `stretch_mode(zoom)` (HALFTONE < 100, COLORONCOLOR > 100, chép thẳng = 100), `wheel_steps(accum, delta) -> (steps, rest)`, `hit(client_pt) -> Hit::{Close, Image}`, `menu_zoom_enabled(w,h,zoom)`. Kiểm DPI 96/120/144/192.
- Chạy tay `#[ignore]` (ghi lệnh trong báo cáo, chạy từng cái `--test-threads=1`): `pin_round_trip_without_gdi_leaks` (mở/đóng 50 lần, đếm `GetGuiResources` GDI và USER trước/sau, chênh ≤ 2); `pin_close_all_destroys_every_window` (mở 5, `close_all`, `count()==0`, `IsWindow` sai với mọi HWND cũ); `pin_limit` (mở 32, lần 33 `Err(Limit)`, dọn); `pin_zoom_keeps_anchor` (gửi `WM_MOUSEWHEEL` tại một điểm, đọc `GetWindowRect`, điểm ảnh dưới chuột không đổi ±1 px); `pin_at_places_image_exactly` (`At(p)` → `GetClientRect` + `ClientToScreen` cho góc ảnh = p).

## Tiêu chí nghiệm thu
Mọi kiểm thử thuần qua trong `cargo test`; các kiểm thử chạy tay qua trên máy này (ghi kết quả thật); `scripts/check.ps1` in `ALL CHECKS PASSED`. Không đổi hành vi nào ngoài `pin.rs`.

## Review focus
Sở hữu và giải phóng tài nguyên trên mọi đường (kể cả `open` thất bại giữa chừng); tái nhập (`close_all` khi menu đang mở, `DestroyWindow` trong thủ tục cửa sổ); `RefCell` không bị mượn khi gọi Win32 có thể gửi thông điệp; DPI và tọa độ âm; tràn số; nhân đôi `WM_NCDESTROY`/dùng sau giải phóng.

TDD: commit `test:` (hàm thuần `todo!()`) rồi `feat:`. Báo cáo theo mẫu `.claude/agents/specialist.md`, nêu rõ chỗ làm khác gói và feature `windows` đã thêm.
