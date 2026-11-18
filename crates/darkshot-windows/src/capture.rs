//! Chụp toàn bộ màn hình ảo vào bộ nhớ.

use darkshot_core::geometry::{Point, Rect};
use darkshot_core::image::Image;
use std::ffi::c_void;
use windows::core::{Error, BOOL};
use windows::Win32::Foundation::{E_FAIL, LPARAM, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, EnumDisplayMonitors,
    GdiFlush, GetDC, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CAPTUREBLT,
    DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ, HMONITOR, ROP_CODE, SRCCOPY,
};
use windows::Win32::UI::HiDpi::{
    SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DrawIconEx, GetCursorInfo, GetCursorPos, GetIconInfo, GetSystemMetrics, CURSORINFO,
    CURSOR_SHOWING, DI_NORMAL, HICON, ICONINFO, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
    SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

pub struct Screenshot {
    /// Toàn bộ màn hình ảo, pixel vật lý.
    pub image: Image,
    /// Góc trên trái của màn hình ảo trong tọa độ màn hình (có thể âm).
    pub origin: Point,
    /// Từng màn hình, đã đổi sang tọa độ ảnh (trừ origin).
    pub monitors: Vec<Rect>,
    /// Vị trí con trỏ chuột lúc chụp, tọa độ màn hình; không lấy được thì tâm màn hình ảo.
    pub cursor: Point,
    /// Khung cửa sổ ở TỌA ĐỘ ẢNH (đã trừ origin); None = tắt gợi ý cửa sổ.
    pub windows: Option<Vec<Rect>>,
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
    Rect {
        x: image.x + origin.x,
        y: image.y + origin.y,
        ..image
    }
}

/// Góc trên trái để vẽ con trỏ vào ảnh: vị trí con trỏ (tọa độ màn hình) trừ điểm nóng,
/// rồi đổi sang tọa độ ảnh của màn hình ảo có gốc `origin`.
pub fn cursor_draw_origin(pos: Point, hotspot: Point, origin: Point) -> Point {
    Point {
        x: pos.x - hotspot.x - origin.x,
        y: pos.y - hotspot.y - origin.y,
    }
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
pub(crate) struct DpiScope(DPI_AWARENESS_CONTEXT);

impl DpiScope {
    pub(crate) fn per_monitor_v2() -> DpiScope {
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

/// Hai bitmap mà `GetIconInfo` tạo ra cho bên gọi; xóa cả hai khi drop (`hbmColor` rỗng với
/// con trỏ đơn sắc).
struct IconBitmaps(ICONINFO);
impl Drop for IconBitmaps {
    fn drop(&mut self) {
        for bmp in [self.0.hbmMask, self.0.hbmColor] {
            if !bmp.is_invalid() {
                // SAFETY: bitmap do GetIconInfo tạo cho ta, không được chọn vào DC nào; xóa một lần.
                unsafe {
                    let _ = DeleteObject(bmp.into());
                }
            }
        }
    }
}

/// Vị trí con trỏ (tọa độ màn hình) bằng `GetCursorPos`.
fn cursor_pos() -> Option<Point> {
    let mut pt = POINT::default();
    // SAFETY: chỉ ghi vào `pt`.
    unsafe { GetCursorPos(&mut pt) }.ok()?;
    Some(Point { x: pt.x, y: pt.y })
}

/// Vẽ con trỏ đang hiện vào `dc` (DC bộ nhớ đang chọn DIB của ảnh có gốc `origin`).
/// Trả về vị trí con trỏ nếu đọc được. Con trỏ ẩn hoặc lời gọi nào thất bại thì không vẽ.
fn draw_cursor(dc: HDC, origin: Point) -> Option<Point> {
    let mut ci = CURSORINFO {
        cbSize: std::mem::size_of::<CURSORINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: `ci` có cbSize đúng; hàm chỉ ghi vào `ci`.
    unsafe { GetCursorInfo(&mut ci) }.ok()?;
    let pos = Point {
        x: ci.ptScreenPos.x,
        y: ci.ptScreenPos.y,
    };
    // hCursor là handle dùng chung của hệ thống: không hủy.
    if ci.flags.0 & CURSOR_SHOWING.0 == 0 || ci.hCursor.is_invalid() {
        return Some(pos);
    }
    let icon: HICON = ci.hCursor.into();
    let mut info = ICONINFO::default();
    // SAFETY: icon hợp lệ vừa đọc; hàm chỉ ghi vào `info`. Guard xóa mọi bitmap nó tạo ra,
    // kể cả khi lời gọi thất bại (khi đó các trường vẫn rỗng).
    let ok = unsafe { GetIconInfo(icon, &mut info) }.is_ok();
    let bitmaps = IconBitmaps(info);
    if !ok {
        return Some(pos);
    }
    let hotspot = Point {
        x: i32::try_from(bitmaps.0.xHotspot).unwrap_or(0),
        y: i32::try_from(bitmaps.0.yHotspot).unwrap_or(0),
    };
    let at = cursor_draw_origin(pos, hotspot, origin);
    // SAFETY: `dc` còn sống và đang chọn DIB; cỡ 0 không kèm DI_DEFAULTSIZE = cỡ thật của con trỏ.
    unsafe {
        let _ = DrawIconEx(dc, at.x, at.y, icon, 0, 0, 0, None, DI_NORMAL);
    }
    drop(bitmaps);
    Some(pos)
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

/// Chép vùng `r` (tọa độ màn hình, pixel vật lý) của màn hình bằng BitBlt với `rop` vào bộ nhớ BGRA
/// từ trên xuống, alpha 255. `draw_cursor_in`: vẽ con trỏ đang hiện vào ảnh và trả vị trí của nó.
fn grab(
    r: Rect,
    rop: ROP_CODE,
    draw_cursor_in: bool,
) -> windows::core::Result<(Vec<u8>, Option<Point>)> {
    let Rect { x, y, w, h } = r;
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
    let pos;
    {
        // SAFETY: chọn DIB vào DC bộ nhớ; guard chọn lại đối tượng cũ trước khi DIB bị xóa.
        let _sel = Selected {
            dc: mem.0,
            old: unsafe { SelectObject(mem.0, bmp.0.into()) },
        };
        // SAFETY: cả hai DC còn sống; vùng nguồn là `r`, đích vừa khít DIB.
        // Lỗi (màn hình khóa, UAC) trả về Err cho bên gọi.
        unsafe {
            BitBlt(mem.0, 0, 0, w, h, Some(screen.0), x, y, rop)?;
        }
        // Con trỏ vẽ khi DIB còn được chọn trong DC bộ nhớ.
        pos = if draw_cursor_in {
            draw_cursor(mem.0, Point { x, y })
        } else {
            None
        };
        // SAFETY: đẩy mọi lệnh vẽ GDI còn chờ vào DIB trước khi đọc pixel.
        unsafe {
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
    Ok((bgra, pos))
}

/// Chụp đúng vùng `screen` (tọa độ màn hình, pixel vật lý; luồng gọi được đặt PerMonitorV2 trong
/// lúc chụp), không có con trỏ, alpha 255. Dùng cho chụp cuộn (mỗi lượt hẹn giờ).
///
/// Chỉ `SRCCOPY`, không `CAPTUREBLT`: khi DWM bật, màn hình đã gồm cửa sổ lớp; `CAPTUREBLT` có thể
/// làm con trỏ nháy ở mỗi lượt chụp (10 lần mỗi giây).
pub fn capture_rect(screen: Rect) -> windows::core::Result<Image> {
    let _dpi = DpiScope::per_monitor_v2();
    let (bgra, _) = grab(screen, SRCCOPY, false)?;
    Ok(Image {
        width: screen.w as u32,
        height: screen.h as u32,
        bgra,
    })
}

/// Chụp toàn bộ màn hình ảo (mọi màn hình). `include_cursor`: vẽ con trỏ đang hiện vào ảnh.
pub fn capture_virtual_screen(include_cursor: bool) -> windows::core::Result<Screenshot> {
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
    let origin = Point { x, y };
    let (bgra, pos) = grab(
        Rect { x, y, w, h },
        ROP_CODE(SRCCOPY.0 | CAPTUREBLT.0),
        include_cursor,
    )?;
    let cursor = pos.or_else(cursor_pos).unwrap_or(Point {
        x: x + w / 2,
        y: y + h / 2,
    });

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
        cursor,
        windows: None,
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
        let s = capture_virtual_screen(false).unwrap();
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

    /// Chụp nhiều lần có vẽ con trỏ: số đối tượng GDI của tiến trình không tăng (hai bitmap của
    /// `GetIconInfo` được xóa), ảnh hợp lệ và cùng cỡ với lượt không có con trỏ.
    #[test]
    #[ignore = "chạm màn hình thật; chạy tay: cargo test -p darkshot-windows -- --ignored capture_with_cursor --test-threads=1"]
    fn capture_with_cursor_does_not_leak_gdi() {
        use windows::Win32::System::Threading::{
            GetCurrentProcess, GetGuiResources, GR_GDIOBJECTS,
        };
        let _gdi = GDI_TEST_LOCK.lock();
        // SAFETY: chỉ đọc số đối tượng GDI của tiến trình này.
        let gdi = || unsafe { GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS) };
        let plain = capture_virtual_screen(false).unwrap();
        let mut after_second = 0;
        for i in 1..=50 {
            let s = capture_virtual_screen(true).unwrap();
            assert!(s.image.is_valid());
            assert_eq!(
                (s.image.width, s.image.height),
                (plain.image.width, plain.image.height)
            );
            if i == 2 {
                after_second = gdi();
            }
        }
        let last = gdi();
        assert!(
            last <= after_second,
            "GDI tăng từ {after_second} lên {last}"
        );
    }

    /// Chụp một vùng: đúng cỡ, alpha đục, không rò GDI; vùng rỗng bị từ chối.
    #[test]
    fn capture_rect_returns_region_only() {
        use windows::Win32::System::Threading::{
            GetCurrentProcess, GetGuiResources, GR_GDIOBJECTS,
        };
        let _gdi = GDI_TEST_LOCK.lock();
        // SAFETY: chỉ đọc số đối tượng GDI của tiến trình này.
        let gdi = || unsafe { GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS) };
        // SAFETY: chỉ đọc giá trị hệ thống.
        let (vx, vy) = unsafe {
            (
                GetSystemMetrics(SM_XVIRTUALSCREEN),
                GetSystemMetrics(SM_YVIRTUALSCREEN),
            )
        };
        let region = Rect {
            x: vx + 10,
            y: vy + 20,
            w: 50,
            h: 40,
        };
        let img = capture_rect(region).unwrap();
        assert_eq!((img.width, img.height), (50, 40));
        assert_eq!(img.bgra.len(), 50 * 40 * 4);
        assert!(img.bgra.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
        let before = gdi();
        for _ in 0..30 {
            assert!(capture_rect(region).unwrap().is_valid());
        }
        assert!(gdi() <= before, "GDI tăng");
        for bad in [
            Rect { w: 0, ..region },
            Rect { h: 0, ..region },
            Rect { w: -5, ..region },
        ] {
            assert!(capture_rect(bad).is_err(), "{bad:?}");
        }
    }

    /// Ảnh 64×48 gồm bốn ô màu phẳng 32×24 (đỏ, lục, lam, xám); kèm tâm và màu [R, G, B] của mỗi ô.
    type Tiles = [(i32, i32, [u8; 3]); 4];
    fn four_tiles() -> (darkshot_core::image::Image, Tiles) {
        let tiles: Tiles = [
            (16, 12, [220, 30, 30]),
            (48, 12, [30, 200, 40]),
            (16, 36, [30, 50, 210]),
            (48, 36, [128, 128, 128]),
        ];
        let (w, h) = (64u32, 48u32);
        let mut bgra = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                let idx = usize::from(x >= 32) + 2 * usize::from(y >= 24);
                let [r, g, b] = tiles[idx].2;
                bgra.extend_from_slice(&[b, g, r, 255]);
            }
        }
        let img = darkshot_core::image::Image {
            width: w,
            height: h,
            bgra,
        };
        (img, tiles)
    }

    /// Giải nén JPEG do core tạo bằng GDI+ (bộ giải nén độc lập), so kích thước và màu tâm mỗi ô.
    #[test]
    fn jpeg_decodes_with_gdiplus() {
        // Tạo đối tượng GDI: không chen vào kiểm thử đếm GDI của cả tiến trình.
        let _gdi = crate::capture::GDI_TEST_LOCK.lock();
        use windows::Win32::Graphics::GdiPlus::{
            GdipBitmapGetPixel, GdipCreateBitmapFromStream, GdipDisposeImage, GdipGetImageHeight,
            GdipGetImageWidth, GdiplusShutdown, GdiplusStartup, GdiplusStartupInput, GpBitmap,
            GpImage,
        };
        use windows::Win32::UI::Shell::SHCreateMemStream;

        /// Tắt GDI+ khi ra khỏi phạm vi, kể cả khi một khẳng định thất bại.
        struct Gdip(usize);
        impl Drop for Gdip {
            fn drop(&mut self) {
                // SAFETY: token từ GdiplusStartup thành công; mọi bitmap đã được giải phóng trước đó.
                unsafe { GdiplusShutdown(self.0) };
            }
        }
        /// Giải phóng bitmap khi ra khỏi phạm vi.
        struct Bmp(*mut GpBitmap);
        impl Drop for Bmp {
            fn drop(&mut self) {
                if !self.0.is_null() {
                    // SAFETY: bitmap do GdipCreateBitmapFromStream tạo, giải phóng đúng một lần.
                    unsafe { GdipDisposeImage(self.0 as *mut GpImage) };
                }
            }
        }

        let input = GdiplusStartupInput {
            GdiplusVersion: 1,
            ..Default::default()
        };
        let mut token = 0usize;
        // SAFETY: tham số là biến cục bộ; không xin luồng nền của GDI+.
        let st = unsafe { GdiplusStartup(&mut token, &input, std::ptr::null_mut()) };
        assert_eq!(st.0, 0, "GdiplusStartup");
        let _gdip = Gdip(token);

        let (img, tiles) = four_tiles();
        for (quality, tol) in [(90u8, 6i32), (50, 12), (100, 6)] {
            let jpeg = darkshot_core::image::encode_jpeg(&img, quality).unwrap();
            // SAFETY: SHCreateMemStream chép dữ liệu vào stream của nó; IStream tự Release khi drop.
            let stream = unsafe { SHCreateMemStream(Some(&jpeg)) }.expect("SHCreateMemStream");
            let mut raw: *mut GpBitmap = std::ptr::null_mut();
            // SAFETY: stream hợp lệ suốt lời gọi; bitmap mới (nếu có) do `Bmp` giải phóng.
            let st = unsafe { GdipCreateBitmapFromStream(&stream, &mut raw) };
            let bmp = Bmp(raw);
            assert_eq!(st.0, 0, "GdipCreateBitmapFromStream q={quality}");
            assert!(!bmp.0.is_null());
            let (mut w, mut h) = (0u32, 0u32);
            // SAFETY: bitmap hợp lệ; w, h là biến cục bộ.
            unsafe {
                GdipGetImageWidth(bmp.0 as *mut GpImage, &mut w);
                GdipGetImageHeight(bmp.0 as *mut GpImage, &mut h);
            }
            assert_eq!((w, h), (64, 48), "q={quality}");
            for (x, y, rgb) in tiles {
                let mut argb = 0u32;
                // SAFETY: bitmap hợp lệ, (x, y) nằm trong ảnh; argb là biến cục bộ.
                let st = unsafe { GdipBitmapGetPixel(bmp.0, x, y, &mut argb) };
                assert_eq!(st.0, 0, "GdipBitmapGetPixel");
                let got = [(argb >> 16) & 0xFF, (argb >> 8) & 0xFF, argb & 0xFF];
                for k in 0..3 {
                    let d = (got[k] as i32 - i32::from(rgb[k])).abs();
                    assert!(
                        d <= tol,
                        "q={quality} ({x},{y}) kênh {k}: {} so với {} lệch quá {tol}",
                        got[k],
                        rgb[k]
                    );
                }
            }
            // Bitmap giải phóng trước stream (GDI+ còn đọc stream tới lúc đó).
            drop(bmp);
            drop(stream);
        }
    }
}
