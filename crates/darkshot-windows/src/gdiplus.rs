//! Khởi động GDI+ và đối tượng vẽ GDI+ trên một DC (dùng chung cho lớp phủ và nét đánh dấu).

use windows::Win32::Graphics::Gdi::HDC;
use windows::Win32::Graphics::GdiPlus::{
    GdipCreateFromHDC, GdipDeleteGraphics, GdipSetPixelOffsetMode, GdipSetSmoothingMode,
    GdiplusShutdown, GdiplusStartup, GdiplusStartupInput, GpGraphics, PixelOffsetModeHalf,
    SmoothingModeAntiAlias,
};

/// Đối tượng GDI+ vẽ lên một DC (khử răng cưa, lệch nửa pixel); giải phóng khi drop.
pub struct Gp(*mut GpGraphics);

impl Gp {
    /// `dc` phải còn sống suốt đời của `Gp`.
    pub fn new(dc: HDC) -> Option<Gp> {
        let mut g: *mut GpGraphics = std::ptr::null_mut();
        // SAFETY: dc do người gọi giữ sống suốt đời của Gp; g là con trỏ ra hợp lệ.
        let st = unsafe { GdipCreateFromHDC(dc, &mut g) };
        if st.0 != 0 || g.is_null() {
            return None;
        }
        // SAFETY: g hợp lệ vừa tạo.
        unsafe {
            GdipSetSmoothingMode(g, SmoothingModeAntiAlias);
            GdipSetPixelOffsetMode(g, PixelOffsetModeHalf);
        }
        Some(Gp(g))
    }

    pub fn raw(&self) -> *mut GpGraphics {
        self.0
    }
}

impl Drop for Gp {
    fn drop(&mut self) {
        // SAFETY: đối tượng do GdipCreateFromHDC tạo, xóa đúng một lần.
        unsafe {
            GdipDeleteGraphics(self.0);
        }
    }
}

/// Giữ GDI+ chạy trong thời gian đối tượng còn sống; `GdiplusShutdown` khi drop.
pub struct GdiPlus(Option<usize>);

impl GdiPlus {
    pub fn start() -> GdiPlus {
        let input = GdiplusStartupInput {
            GdiplusVersion: 1,
            ..Default::default()
        };
        let mut token = 0usize;
        // SAFETY: input hợp lệ; output được phép null khi không tắt luồng nền.
        let st = unsafe { GdiplusStartup(&mut token, &input, std::ptr::null_mut()) };
        GdiPlus((st.0 == 0).then_some(token))
    }

    /// Đúng khi GDI+ khởi động được.
    pub fn ok(&self) -> bool {
        self.0.is_some()
    }
}

impl Drop for GdiPlus {
    fn drop(&mut self) {
        if let Some(t) = self.0 {
            // SAFETY: token từ GdiplusStartup thành công; mọi đối tượng GDI+ đã xóa.
            unsafe { GdiplusShutdown(t) };
        }
    }
}
