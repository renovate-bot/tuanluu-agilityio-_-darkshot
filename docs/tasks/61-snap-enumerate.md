# Task 61: Bắt cửa sổ – liệt kê cửa sổ (Win32 + DWM) và nối vào luồng chụp

**Agent:** `specialist` · **Model:** opus (Win32, DWM, DPI, lọc cửa sổ hệ thống) · **Nhánh:** `task/61-snap-enumerate` (tách từ `feature/window-snap`) · **Rà soát:** `specialist` (opus)

Thiết kế: `docs/superpowers/specs/2026-11-18-window-snap-design.md` mục 3.2, 4, 5, 6, 7. Global Constraints: mục cùng tên của `docs/superpowers/plans/2026-11-18-window-snap.md`. Task 60 (`darkshot_core::snap`, `Config.snap_windows`) đã ở nhánh phát triển.

## Cần đọc
`crates/darkshot-core/src/snap.rs`, `crates/darkshot-windows/src/capture.rs` (`Screenshot`, `capture_virtual_screen`, `DpiScope`, `to_screen_coords`), `main.rs` (`capture_flow`), `theme.rs` (cách gọi DWM sẵn có), `Cargo.toml` của `darkshot-windows`. Tài liệu chính thức: `EnumWindows`, `DwmGetWindowAttribute` (`DWMWA_EXTENDED_FRAME_BOUNDS`, `DWMWA_CLOAKED`), `IsWindowVisible`, `IsIconic`, `GetClassNameW`, `GetWindowLongPtrW(GWL_EXSTYLE)`.

## Files
Tạo `darkshot-windows/src/snap.rs`. Sửa `darkshot-windows/src/{main.rs, capture.rs}` và mọi chỗ dựng `Screenshot` trong kiểm thử (`overlay.rs` chỉ phần kiểm thử, nếu có). Feature `Win32_Graphics_Dwm` đã bật; không cần feature mới (nếu cần, nêu trong báo cáo).

## Interfaces
```rust
// darkshot-windows/src/snap.rs
/// Khung các cửa sổ cấp cao hiện được, thứ tự Z từ trên xuống, tọa độ MÀN HÌNH, pixel vật lý.
/// Tối đa darkshot_core::snap::MAX_WINDOWS. Lỗi liệt kê → danh sách (có thể rỗng) đã có tới lúc lỗi.
pub fn enumerate_windows() -> Vec<Rect>;
/// Thuần: lớp cửa sổ là nền màn hình (bỏ qua). So khớp không phân biệt hoa thường: "Progman", "WorkerW".
pub fn is_shell_background(class: &str) -> bool;
/// Thuần: quyết định một cửa sổ có là ứng viên không từ các thuộc tính đã đọc.
pub fn accept(visible: bool, iconic: bool, cloaked: bool, ex_style: u32, class: &str) -> bool;

// capture.rs
pub struct Screenshot { …, /// Khung cửa sổ ở TỌA ĐỘ ẢNH (đã trừ origin); None = tắt gợi ý cửa sổ.
    pub windows: Option<Vec<Rect>> }
```
- `accept` = `visible && !iconic && !cloaked && ex_style & WS_EX_TRANSPARENT == 0 && !is_shell_background(class)`.
- `enumerate_windows`: chạy trong `DpiScope::per_monitor_v2()` (hoặc tương đương) để khung là pixel vật lý; callback `EnumWindows` đẩy vào `Vec` qua `LPARAM` trỏ tới biến cục bộ (an toàn vì `EnumWindows` đồng bộ); khung ưu tiên `DWMWA_EXTENDED_FRAME_BOUNDS`, lỗi thì `GetWindowRect`; khung rỗng/âm bỏ. Dừng liệt kê khi đủ `MAX_WINDOWS`. Không đọc tiêu đề cửa sổ.
- `capture_flow`: chỉ khi `kind == Region` và `config.snap_windows`: gọi `enumerate_windows()` NGAY TRƯỚC `capture_virtual_screen`, rồi đổi sang tọa độ ảnh (trừ `shot.origin`, phép trừ bão hòa) và gán `shot.windows = Some(..)`. Ngược lại `None`. `capture_virtual_screen` đặt `windows: None`.
- Không đổi chữ ký `pick_region` (Task 62 đọc `shot.windows`).

## Kiểm thử (viết trước)
- `is_shell_background`: "Progman", "progman", "WorkerW" → true; "Chrome_WidgetWin_1", "" → false.
- `accept`: bảng các tổ hợp (ẩn, thu nhỏ, cloaked, WS_EX_TRANSPARENT, lớp nền → false; bình thường → true; WS_EX_TOOLWINDOW đơn thuần → true).
- `enumerate_finds_own_visible_window` (kiểm thử thường, không `ignore` nếu chạy ổn trên CI cục bộ; nếu phụ thuộc màn hình thì `#[ignore]` kèm lệnh chạy tay): tạo một cửa sổ popup hiện ở (100,100) cỡ 300×200 (lớp riêng, `WS_POPUP | WS_VISIBLE`), `enumerate_windows()` chứa một khung ≈ khung cửa sổ đó (sai số 0 với popup không viền); ẩn nó → không còn. Hủy cửa sổ, hủy lớp trên mọi đường đi.
- `screenshot_windows_none_by_default`: kiểm thử thuần trên hàm đổi tọa độ `windows_to_image(&[Rect], origin) -> Vec<Rect>` (đặt trong `snap.rs`) với origin âm.
- Đo thời gian `enumerate_windows` trong kiểm thử chạy tay và ghi vào báo cáo (mục tiêu < 10 ms).

## Việc cần chủ dự án thử tay
Không riêng (thử cùng Task 62).

## Báo cáo
Theo mẫu `.claude/agents/specialist.md`; nêu số cửa sổ và thời gian đo trên máy.
