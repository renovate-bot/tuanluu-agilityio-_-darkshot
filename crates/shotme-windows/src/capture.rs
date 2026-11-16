//! Chụp toàn bộ màn hình ảo vào bộ nhớ.

use shotme_core::geometry::{Point, Rect};
use shotme_core::image::Image;
use std::ffi::c_void;
use windows::core::{Error, BOOL};
use windows::Win32::Foundation::{E_FAIL, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, EnumDisplayMonitors,
    GdiFlush, GetDC, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CAPTUREBLT,
    DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ, HMONITOR, ROP_CODE, SRCCOPY,
};
use windows::Win32::UI::HiDpi::{
    SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

pub struct Screenshot {
    /// Toàn bộ màn hình ảo, pixel vật lý.
    pub image: Image,
    /// Góc trên trái của màn hình ảo trong tọa độ màn hình (có thể âm).
    pub origin: Point,
    /// Từng màn hình, đã đổi sang tọa độ ảnh (trừ origin).
    pub monitors: Vec<Rect>,
}

/// Đổi một hình chữ nhật từ tọa độ màn hình sang tọa độ ảnh của màn hình ảo.
pub fn to_image_coords(screen: Rect, origin: Point) -> Rect {
    Rect {
        x: screen.x - origin.x,
        y: screen.y - origin.y,
        ..screen
    }
}

/// Đổi một hình chữ nhật từ tọa độ ảnh của màn hình ảo sang tọa độ màn hình (ngược của
/// `to_image_coords`).
pub fn to_screen_coords(image: Rect, origin: Point) -> Rect {
    todo!()
}

/// Góc trên trái để vẽ con trỏ vào ảnh: vị trí con trỏ (tọa độ màn hình) trừ điểm nóng,
/// rồi đổi sang tọa độ ảnh của màn hình ảo có gốc `origin`.
pub fn cursor_draw_origin(pos: Point, hotspot: Point, origin: Point) -> Point {
    todo!()
}

/// Lỗi Win32 của luồng hiện tại; nếu luồng không ghi mã lỗi thì dùng `E_FAIL`.
fn last_error() -> Error {
    let e = Error::from_thread();
    if e.code().is_ok() {
        Error::from_hresult(E_FAIL)
    } else {
        e
    }
}

/// Đặt luồng sang PerMonitorV2 trong lúc chụp (tiến trình đã khai báo qua manifest;
/// với bản kiểm thử không có manifest thì nhờ đó vẫn lấy pixel vật lý). Trả lại khi drop.
struct DpiScope(DPI_AWARENESS_CONTEXT);

impl DpiScope {
    fn per_monitor_v2() -> DpiScope {
        // SAFETY: chỉ đổi ngữ cảnh DPI của luồng hiện tại; giá trị cũ được trả lại trong Drop.
        DpiScope(unsafe {
            SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)
        })
    }
}

impl Drop for DpiScope {
    fn drop(&mut self) {
        // Null nghĩa là lần đặt trước thất bại, không có gì để trả lại.
        if !self.0 .0.is_null() {
            // SAFETY: trả lại đúng ngữ cảnh mà SetThreadDpiAwarenessContext đã trả về.
            unsafe {
                SetThreadDpiAwarenessContext(self.0);
            }
        }
    }
}

struct ScreenDc(HDC);
impl Drop for ScreenDc {
    fn drop(&mut self) {
        // SAFETY: DC lấy bằng GetDC(None) ở luồng này, giải phóng đúng một lần.
        unsafe {
            ReleaseDC(None, self.0);
        }
    }
}

struct MemDc(HDC);
impl Drop for MemDc {
    fn drop(&mut self) {
        // SAFETY: DC tạo bằng CreateCompatibleDC, xóa đúng một lần.
        unsafe {
            let _ = DeleteDC(self.0);
        }
    }
}

struct Bitmap(HBITMAP);
impl Drop for Bitmap {
    fn drop(&mut self) {
        // SAFETY: bitmap do ta tạo và đã được bỏ chọn khỏi DC trước khi drop (thứ tự khai báo).
        unsafe {
            let _ = DeleteObject(self.0.into());
        }
    }
}

struct Selected {
    dc: HDC,
    old: HGDIOBJ,
}
impl Drop for Selected {
    fn drop(&mut self) {
        // SAFETY: chọn lại đối tượng cũ vào DC còn sống.
        unsafe {
            SelectObject(self.dc, self.old);
        }
    }
}

unsafe extern "system" fn collect_monitor(
    _m: HMONITOR,
    _dc: HDC,
    rc: *mut RECT,
    data: LPARAM,
) -> BOOL {
    // SAFETY: `data` là con trỏ tới Vec<Rect> còn sống trong capture_virtual_screen;
    // `rc` do hệ thống cấp và hợp lệ trong lời gọi này.
    unsafe {
        let out = &mut *(data.0 as *mut Vec<Rect>);
        if let Some(r) = rc.as_ref() {
            out.push(Rect {
                x: r.left,
                y: r.top,
                w: r.right - r.left,
                h: r.bottom - r.top,
            });
        }
    }
    BOOL(1)
}

/// Phần giao của `r` với ảnh `w`×`h`; rỗng thì `None`.
fn clip_to_image(r: Rect, w: i32, h: i32) -> Option<Rect> {
    let x0 = r.x.max(0);
    let y0 = r.y.max(0);
    let x1 = r.right().min(w);
    let y1 = r.bottom().min(h);
    (x1 > x0 && y1 > y0).then_some(Rect {
        x: x0,
        y: y0,
        w: x1 - x0,
        h: y1 - y0,
    })
}

/// Chụp toàn bộ màn hình ảo (mọi màn hình), không có con trỏ chuột.
pub fn capture_virtual_screen() -> windows::core::Result<Screenshot> {
    let _dpi = DpiScope::per_monitor_v2();
    // SAFETY: GetSystemMetrics chỉ đọc giá trị hệ thống.
    let (x, y, w, h) = unsafe {
        (
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
        )
    };
    if w <= 0 || h <= 0 {
        return Err(Error::from_hresult(E_FAIL));
    }
    let len = (w as usize)
        .checked_mul(h as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(|| Error::from_hresult(E_FAIL))?;

    // SAFETY: lấy DC của toàn màn hình; guard giải phóng trên mọi đường đi.
    let screen = ScreenDc(unsafe { GetDC(None) });
    if screen.0.is_invalid() {
        return Err(last_error());
    }
    // SAFETY: DC tương thích với DC màn hình còn sống; guard xóa nó.
    let mem = MemDc(unsafe { CreateCompatibleDC(Some(screen.0)) });
    if mem.0.is_invalid() {
        return Err(last_error());
    }
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h, // âm: từ trên xuống
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bits: *mut c_void = std::ptr::null_mut();
    // SAFETY: bmi hợp lệ; bits nhận con trỏ tới vùng nhớ của DIB, sống cùng `bmp`.
    let bmp = Bitmap(unsafe {
        CreateDIBSection(Some(screen.0), &bmi, DIB_RGB_COLORS, &mut bits, None, 0)?
    });
    if bits.is_null() {
        return Err(Error::from_hresult(E_FAIL));
    }
    {
        // SAFETY: chọn DIB vào DC bộ nhớ; guard chọn lại đối tượng cũ trước khi DIB bị xóa.
        let _sel = Selected {
            dc: mem.0,
            old: unsafe { SelectObject(mem.0, bmp.0.into()) },
        };
        // SAFETY: cả hai DC còn sống; vùng nguồn là màn hình ảo, đích vừa khít DIB.
        // Lỗi (màn hình khóa, UAC) trả về Err cho bên gọi hiện thông báo.
        unsafe {
            BitBlt(
                mem.0,
                0,
                0,
                w,
                h,
                Some(screen.0),
                x,
                y,
                ROP_CODE(SRCCOPY.0 | CAPTUREBLT.0),
            )?;
            let _ = GdiFlush();
        }
    }
    // SAFETY: DIB 32 bit w×h từ trên xuống có đúng `len` byte, còn sống đến hết hàm.
    let src = unsafe { std::slice::from_raw_parts(bits as *const u8, len) };
    let mut bgra = src.to_vec();
    drop(bmp);
    drop(mem);
    drop(screen);
    // BitBlt không ghi kênh alpha; đặt đục để ảnh dùng được ở mọi nơi.
    for px in bgra.as_chunks_mut::<4>().0 {
        px[3] = 255;
    }

    let origin = Point { x, y };
    let mut screens: Vec<Rect> = Vec::new();
    // SAFETY: callback chỉ ghi vào `screens`, sống đến hết lời gọi đồng bộ này.
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(collect_monitor),
            LPARAM(&mut screens as *mut Vec<Rect> as isize),
        );
    }
    let mut monitors: Vec<Rect> = screens
        .into_iter()
        .filter_map(|m| clip_to_image(to_image_coords(m, origin), w, h))
        .collect();
    if monitors.is_empty() {
        // Không liệt kê được màn hình: coi cả màn hình ảo là một màn hình.
        monitors.push(Rect { x: 0, y: 0, w, h });
    }

    Ok(Screenshot {
        image: Image {
            width: w as u32,
            height: h as u32,
            bgra,
        },
        origin,
        monitors,
    })
}

/// Kiểm thử dùng GDI chạy lần lượt để đếm được đối tượng GDI của tiến trình.
#[cfg(test)]
pub static GDI_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_coords_with_negative_origin() {
        let r = to_image_coords(
            Rect {
                x: -1920,
                y: -200,
                w: 1920,
                h: 1080,
            },
            Point { x: -1920, y: -200 },
        );
        assert_eq!(
            r,
            Rect {
                x: 0,
                y: 0,
                w: 1920,
                h: 1080
            }
        );
        let r = to_image_coords(
            Rect {
                x: 0,
                y: 0,
                w: 2560,
                h: 1440,
            },
            Point { x: -1920, y: -200 },
        );
        assert_eq!(
            r,
            Rect {
                x: 1920,
                y: 200,
                w: 2560,
                h: 1440
            }
        );
    }

    #[test]
    fn screen_and_image_coords_are_inverse() {
        let o = Point { x: -1280, y: 40 };
        let r = Rect {
            x: 10,
            y: 20,
            w: 30,
            h: 40,
        };
        assert_eq!(
            to_screen_coords(r, o),
            Rect {
                x: -1270,
                y: 60,
                w: 30,
                h: 40
            }
        );
        assert_eq!(to_image_coords(to_screen_coords(r, o), o), r);
    }

    #[test]
    fn cursor_origin_subtracts_hotspot_and_screen_origin() {
        assert_eq!(
            cursor_draw_origin(
                Point { x: 100, y: 50 },
                Point { x: 0, y: 0 },
                Point { x: 0, y: 0 }
            ),
            Point { x: 100, y: 50 }
        );
        // màn hình phụ bên trái: gốc âm; con trỏ chữ I có điểm nóng ở giữa
        assert_eq!(
            cursor_draw_origin(
                Point { x: -200, y: 10 },
                Point { x: 8, y: 12 },
                Point { x: -1280, y: 0 }
            ),
            Point { x: 1072, y: -2 }
        );
    }

    #[test]
    fn capture_matches_virtual_screen() {
        let _gdi = GDI_TEST_LOCK.lock();
        let s = capture_virtual_screen().unwrap();
        assert!(s.image.width > 0 && s.image.height > 0);
        assert_eq!(
            s.image.bgra.len(),
            (s.image.width * s.image.height * 4) as usize
        );
        assert!(!s.monitors.is_empty());
        // mọi màn hình nằm trọn trong ảnh
        assert!(s.monitors.iter().all(|m| m.x >= 0
            && m.y >= 0
            && m.right() <= s.image.width as i32
            && m.bottom() <= s.image.height as i32));
    }
}
