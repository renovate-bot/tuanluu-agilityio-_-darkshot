//! Tiện ích kiểm thử: chụp cửa sổ thật bằng `PrintWindow` cho ảnh website, vẽ ảnh mẫu bịa bằng GDI,
//! cách ly hồ sơ (`DARKSHOT_PROFILE_DIR`) và đổi ngôn ngữ cho từng ảnh.
//!
//! Chạy: `$env:DARKSHOT_WEBSHOTS = "<repo>\target\webshots";
//! cargo test -p darkshot-windows webshot_ -- --ignored --test-threads=1`.

use darkshot_core::i18n::Lang;
use darkshot_core::image::Image;
use std::ffi::{c_void, OsString};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{COLORREF, HWND, POINT, RECT};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, CreateFontIndirectW, CreatePen, CreateSolidBrush,
    DeleteDC, DeleteObject, FillRect, GdiFlush, LineTo, MoveToEx, Polygon, SelectObject, SetBkMode,
    SetTextColor, TextOutW, ANTIALIASED_QUALITY, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    DIB_RGB_COLORS, FONT_QUALITY, HBITMAP, HDC, HGDIOBJ, LOGFONTW, PS_SOLID, TRANSPARENT,
};
use windows::Win32::UI::HiDpi::{
    GetDpiForWindow, SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetWindowRect, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
};

/// Biến môi trường chứa thư mục xuất PNG.
pub const OUT_ENV: &str = "DARKSHOT_WEBSHOTS";

/// Thư mục xuất (đã tạo) hoặc `None` khi thiếu biến: kiểm thử bỏ qua, không ghi gì.
pub fn out_dir() -> Option<PathBuf> {
    let dir = std::env::var_os(OUT_ENV).filter(|s| !s.is_empty())?;
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).expect("create webshot dir");
    Some(dir)
}

/// Như manifest của ứng dụng: luồng kiểm thử theo DPI từng màn hình.
pub fn per_monitor() {
    // SAFETY: chỉ đặt chế độ DPI cho luồng hiện tại.
    unsafe {
        SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

/// DPI của cửa sổ `h` (để ghi vào báo cáo).
pub fn dpi_of(h: HWND) -> u32 {
    // SAFETY: chỉ hỏi DPI của cửa sổ thuộc tiến trình này.
    unsafe { GetDpiForWindow(h) }
}

/// Hồ sơ cách ly: đặt `DARKSHOT_PROFILE_DIR` tới thư mục tạm riêng; khi bỏ thì trả biến cũ và xóa
/// thư mục.
pub struct Profile {
    dir: PathBuf,
    prev: Option<OsString>,
}

impl Profile {
    pub fn new(tag: &str) -> Profile {
        let dir =
            std::env::temp_dir().join(format!("darkshot_webshot_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp profile");
        let prev = std::env::var_os(crate::paths::PROFILE_ENV);
        std::env::set_var(crate::paths::PROFILE_ENV, &dir);
        Profile { dir, prev }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }
}

impl Drop for Profile {
    fn drop(&mut self) {
        match self.prev.take() {
            Some(v) => std::env::set_var(crate::paths::PROFILE_ENV, v),
            None => std::env::remove_var(crate::paths::PROFILE_ENV),
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Đặt lại tiếng Việt khi bỏ (kể cả khi kiểm thử hỏng giữa chừng).
struct ResetLang;

impl Drop for ResetLang {
    fn drop(&mut self) {
        crate::lang::set(Lang::Vi);
    }
}

/// Gọi `f` cho English rồi Tiếng Việt với hậu tố tên tệp `en`/`vi`; xong đặt lại `Vi`.
pub fn each_lang(mut f: impl FnMut(Lang, &str)) {
    let _reset = ResetLang;
    for (lang, code) in [(Lang::En, "en"), (Lang::Vi, "vi")] {
        crate::lang::set(lang);
        f(lang, code);
    }
}

/// Bơm thông điệp của luồng (qua `pre` như ứng dụng) trong tối đa `ms`, dừng sớm khi `done`.
pub fn pump(ms: u64, pre: fn(&MSG) -> bool, done: impl Fn() -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_millis(ms);
    let mut m = MSG::default();
    loop {
        // SAFETY: vòng lặp thông điệp của luồng kiểm thử.
        unsafe {
            while PeekMessageW(&mut m, None, 0, 0, PM_REMOVE).as_bool() {
                if !pre(&m) {
                    let _ = TranslateMessage(&m);
                    DispatchMessageW(&m);
                }
            }
        }
        if done() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(15));
    }
}

/// Không lọc thông điệp nào.
pub fn no_pre(_: &MSG) -> bool {
    false
}

#[link(name = "user32")]
unsafe extern "system" {
    fn PrintWindow(hwnd: HWND, hdc: HDC, flags: u32) -> i32;
}

/// `PW_RENDERFULLCONTENT`.
const PW_RENDERFULLCONTENT: u32 = 2;

/// Ảnh của cửa sổ `h` bằng PrintWindow (PW_RENDERFULLCONTENT), không đọc màn hình.
pub fn print_window(h: HWND) -> Image {
    let mut rc = RECT::default();
    // SAFETY: rc là biến cục bộ; hỏi khung của chính cửa sổ này.
    unsafe {
        let _ = GetWindowRect(h, &mut rc);
    }
    let (w, hh) = (rc.right - rc.left, rc.bottom - rc.top);
    let c = Canvas::new(w, hh, (0, 0, 0));
    // SAFETY: DC của khung vẽ còn sống tới khi `c` bị bỏ.
    let ok = unsafe { PrintWindow(h, c.dc, PW_RENDERFULLCONTENT) };
    assert_ne!(ok, 0, "PrintWindow");
    let full = c.image();
    // Bỏ viền co giãn vô hình (Windows 10/11): cắt theo khung DWM thấy được.
    let mut vis = RECT::default();
    // SAFETY: vis là biến cục bộ đúng cỡ RECT; hỏi thuộc tính của chính cửa sổ này.
    let got = unsafe {
        DwmGetWindowAttribute(
            h,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            (&mut vis as *mut RECT).cast(),
            std::mem::size_of::<RECT>() as u32,
        )
    };
    if got.is_err() {
        return full;
    }
    let x0 = (vis.left - rc.left).clamp(0, w);
    let y0 = (vis.top - rc.top).clamp(0, hh);
    let x1 = (vis.right - rc.left).clamp(x0, w);
    let y1 = (vis.bottom - rc.top).clamp(y0, hh);
    if (x0, y0, x1, y1) == (0, 0, w, hh) || x1 == x0 || y1 == y0 {
        return full;
    }
    crop(&full, x0, y0, x1 - x0, y1 - y0)
}

/// Phần `(x, y, w, h)` của `img` (đã nằm trong ảnh).
fn crop(img: &Image, x: i32, y: i32, w: i32, h: i32) -> Image {
    let stride = img.width as usize * 4;
    let mut bgra = Vec::with_capacity(w as usize * h as usize * 4);
    for row in y as usize..(y + h) as usize {
        let start = row * stride + x as usize * 4;
        bgra.extend_from_slice(&img.bgra[start..start + w as usize * 4]);
    }
    Image {
        width: w as u32,
        height: h as u32,
        bgra,
    }
}

/// Ghi `img` thành `<dir>\<name>.png`.
pub fn save(dir: &Path, name: &str, img: &Image) {
    let path = dir.join(format!("{name}.png"));
    let png = darkshot_core::image::encode_png(img).expect("png");
    std::fs::write(&path, png).expect("write webshot");
    println!("saved {name}.png {}x{}", img.width, img.height);
}

/// Ảnh có ít nhất hai màu khác nhau (không đen trơn).
pub fn not_blank(img: &Image) -> bool {
    let first = &img.bgra[..3];
    img.bgra.chunks(4).any(|p| &p[..3] != first)
}

pub type Rgb = (u8, u8, u8);

fn color(c: Rgb) -> COLORREF {
    COLORREF(u32::from(c.0) | u32::from(c.1) << 8 | u32::from(c.2) << 16)
}

/// Khung vẽ GDI trong bộ nhớ (DIB 32 bit) để dựng ảnh mẫu bịa; mọi đối tượng GDI giải phóng khi bỏ.
pub struct Canvas {
    dc: HDC,
    bmp: HBITMAP,
    old: HGDIOBJ,
    bits: *mut c_void,
    w: i32,
    h: i32,
}

impl Canvas {
    pub fn new(w: i32, h: i32, bg: Rgb) -> Canvas {
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut c_void = std::ptr::null_mut();
        // SAFETY: DC và DIB thuộc khung vẽ này, giải phóng trong `Drop`.
        let mut c = unsafe {
            let dc = CreateCompatibleDC(None);
            assert!(!dc.is_invalid());
            let bmp =
                CreateDIBSection(Some(dc), &bmi, DIB_RGB_COLORS, &mut bits, None, 0).expect("dib");
            assert!(!bits.is_null());
            let old = SelectObject(dc, bmp.into());
            SetBkMode(dc, TRANSPARENT);
            Canvas {
                dc,
                bmp,
                old,
                bits,
                w,
                h,
            }
        };
        c.fill(0, 0, w, h, bg);
        c
    }

    /// Tô hình chữ nhật.
    pub fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, c: Rgb) {
        let rc = RECT {
            left: x,
            top: y,
            right: x + w,
            bottom: y + h,
        };
        // SAFETY: chổi tạm, xóa ngay sau khi tô vào DC của khung vẽ.
        unsafe {
            let b = CreateSolidBrush(color(c));
            FillRect(self.dc, &rc, b);
            let _ = DeleteObject(b.into());
        }
    }

    /// Viền chữ nhật dày `t` px.
    pub fn frame(&mut self, x: i32, y: i32, w: i32, h: i32, t: i32, c: Rgb) {
        self.fill(x, y, w, t, c);
        self.fill(x, y + h - t, w, t, c);
        self.fill(x, y, t, h, c);
        self.fill(x + w - t, y, t, h, c);
    }

    /// Chữ một dòng, góc trên trái `(x, y)`, phông Segoe UI cao `px`.
    pub fn text(&mut self, x: i32, y: i32, px: i32, bold: bool, c: Rgb, s: &str) {
        let mut lf = LOGFONTW {
            lfHeight: -px,
            lfWeight: if bold { 700 } else { 400 },
            lfQuality: FONT_QUALITY(ANTIALIASED_QUALITY.0),
            ..Default::default()
        };
        for (i, ch) in "Segoe UI".encode_utf16().enumerate() {
            lf.lfFaceName[i] = ch;
        }
        let s: Vec<u16> = s.encode_utf16().collect();
        // SAFETY: phông tạm, chọn ra và xóa ngay sau khi vẽ.
        unsafe {
            let font = CreateFontIndirectW(&lf);
            let old = SelectObject(self.dc, font.into());
            SetTextColor(self.dc, color(c));
            let _ = TextOutW(self.dc, x, y, &s);
            SelectObject(self.dc, old);
            let _ = DeleteObject(font.into());
        }
    }

    /// Đoạn thẳng dày `width` px.
    pub fn line(&mut self, a: (i32, i32), b: (i32, i32), width: i32, c: Rgb) {
        // SAFETY: bút tạm, chọn ra và xóa ngay sau khi vẽ.
        unsafe {
            let pen = CreatePen(PS_SOLID, width, color(c));
            let old = SelectObject(self.dc, pen.into());
            let _ = MoveToEx(self.dc, a.0, a.1, None);
            let _ = LineTo(self.dc, b.0, b.1);
            SelectObject(self.dc, old);
            let _ = DeleteObject(pen.into());
        }
    }

    /// Đa giác tô kín.
    pub fn poly(&mut self, pts: &[(i32, i32)], c: Rgb) {
        let pts: Vec<POINT> = pts.iter().map(|&(x, y)| POINT { x, y }).collect();
        // SAFETY: bút và chổi tạm, chọn ra và xóa ngay sau khi vẽ.
        unsafe {
            let pen = CreatePen(PS_SOLID, 1, color(c));
            let brush = CreateSolidBrush(color(c));
            let op = SelectObject(self.dc, pen.into());
            let ob = SelectObject(self.dc, brush.into());
            let _ = Polygon(self.dc, &pts);
            SelectObject(self.dc, ob);
            SelectObject(self.dc, op);
            let _ = DeleteObject(brush.into());
            let _ = DeleteObject(pen.into());
        }
    }

    /// Chép điểm ảnh ra (alpha 255).
    pub fn image(self) -> Image {
        let len = (self.w as usize) * (self.h as usize) * 4;
        let mut out = vec![0u8; len];
        // SAFETY: DIB `w×h×4` byte còn sống tới khi `self` bị bỏ ở cuối hàm.
        unsafe {
            let _ = GdiFlush();
            std::ptr::copy_nonoverlapping(self.bits.cast::<u8>(), out.as_mut_ptr(), len);
        }
        for p in out.chunks_mut(4) {
            p[3] = 255;
        }
        Image {
            width: self.w as u32,
            height: self.h as u32,
            bgra: out,
        }
    }
}

impl Drop for Canvas {
    fn drop(&mut self) {
        // SAFETY: trả bitmap cũ rồi xóa đối tượng do `new` tạo, đúng một lần.
        unsafe {
            SelectObject(self.dc, self.old);
            let _ = DeleteObject(self.bmp.into());
            let _ = DeleteDC(self.dc);
        }
    }
}
