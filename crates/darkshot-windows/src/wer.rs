//! Báo cáo lỗi của Windows (WER): không gửi bản kết xuất bộ nhớ của darkshot đi đâu khi sự cố.
//!
//! Tài liệu: learn.microsoft.com/windows/win32/api/werapi/nf-werapi-weraddexcludedapplication
//! và .../nf-werapi-wersetflags.

use std::ffi::OsStr;
use windows::core::HSTRING;
use windows::Win32::System::ErrorReporting::{
    WerAddExcludedApplication, WerSetFlags, WER_FAULT_REPORTING_FLAG_NOHEAP,
};

/// Tên file exe đang chạy (mặc định `darkshot.exe`).
fn exe_name() -> HSTRING {
    let path = std::env::current_exe().ok();
    let name = path
        .as_deref()
        .and_then(|p| p.file_name())
        .unwrap_or(OsStr::new("darkshot.exe"));
    HSTRING::from(name)
}

/// Loại exe này khỏi WER cho người dùng hiện tại (`bAllUsers = FALSE`: ghi vào HKCU, không cần
/// quyền quản trị) và bỏ dữ liệu heap khỏi mọi báo cáo của tiến trình này. Lỗi bị bỏ qua.
pub fn exclude_from_error_reporting() {
    let name = exe_name();
    // SAFETY: HSTRING kết thúc bằng 0 và sống suốt lời gọi; hai hàm không giữ con trỏ lại.
    unsafe {
        let _ = WerAddExcludedApplication(&name, false);
        let _ = WerSetFlags(WER_FAULT_REPORTING_FLAG_NOHEAP);
    }
}
