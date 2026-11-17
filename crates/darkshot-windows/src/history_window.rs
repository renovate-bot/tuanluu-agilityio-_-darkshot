//! Cửa sổ Lịch sử ảnh chụp. Task 55 dựng cửa sổ thật; tới lúc đó `show` không làm gì.

use windows::Win32::Foundation::HWND;

/// Mở (hoặc đưa ra trước) cửa sổ Lịch sử. Hiện là chỗ giữ: Task 55 thay bằng cửa sổ thật.
pub fn show(_owner: HWND) {}
