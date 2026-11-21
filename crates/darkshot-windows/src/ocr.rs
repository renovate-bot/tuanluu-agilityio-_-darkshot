//! Nhận dạng chữ cục bộ bằng Windows.Media.Ocr (WinRT), chạy trên luồng nền `darkshot-ocr`.
//! Ảnh chỉ ở bộ nhớ; không ghi tệp, không nhật ký chữ, không mạng.

use std::ffi::c_void;
use std::sync::Once;

use darkshot_core::image::Image;
use darkshot_core::ocr_text::{join_lines, line_count, ocr_scale};
use windows::Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine;
use windows::Storage::Streams::DataWriter;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::Com::{
    CoIncrementMTAUsage, CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED,
};
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

/// Kết quả nhận dạng.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcrResult {
    pub text: String,
    pub lang_tag: String,
    pub lang_name: String,
    pub lines: usize,
}

/// Lỗi nhận dạng. `Failed` chỉ mang thông điệp ngắn cho nhật ký nội bộ/kiểm thử.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OcrError {
    NoLanguage,
    Failed(String),
}

/// Đồng bộ, gọi trên luồng nền. Tự khởi tạo/hủy WinRT apartment MTA cho luồng gọi.
pub fn recognize(image: &Image) -> Result<OcrResult, OcrError> {
    if !image.is_valid() {
        return Err(OcrError::Failed("invalid image".into()));
    }
    let _apartment = Mta::enter()?;
    // Mọi đối tượng WinRT sống trong hàm dưới và đã được nhả trước khi `_apartment` hủy apartment.
    recognize_in_mta(image)
}

/// Tạo luồng `darkshot-ocr`, chạy `recognize`, gửi `Box<Result<OcrResult, OcrError>>` tới `hwnd`
/// bằng `msg` (LPARAM = con trỏ Box; WPARAM = `token`). Gửi lỗi → tự giải phóng.
/// Không tạo được luồng → Err. Luồng không được join: thoát ứng dụng giữa chừng thì
/// `PostMessageW` tới cửa sổ đã hủy thất bại và Box được giải phóng.
pub fn start(image: Image, hwnd: HWND, msg: u32, token: usize) -> Result<(), String> {
    // HWND không Send; chuyển sang luồng nền dưới dạng số.
    let target = hwnd.0 as isize;
    std::thread::Builder::new()
        .name("darkshot-ocr".into())
        .spawn(move || {
            let res = recognize(&image);
            drop(image);
            post_result(target, msg, token, res);
        })
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Bên nhận lấy lại quyền sở hữu: gọi đúng một lần cho mỗi thông điệp nhận được.
///
/// # Safety
/// `lp` phải là LPARAM của một thông điệp do `start` gửi và chưa được lấy lần nào.
pub unsafe fn take_result(lp: LPARAM) -> Box<Result<OcrResult, OcrError>> {
    // SAFETY: theo hợp đồng của hàm, `lp` là con trỏ Box do `start` tạo, chưa ai lấy lại.
    unsafe { Box::from_raw(lp.0 as *mut Result<OcrResult, OcrError>) }
}

/// Apartment MTA của luồng gọi; `CoUninitialize` khi drop (chỉ tạo khi khởi tạo thành công).
struct Mta;

impl Mta {
    fn enter() -> Result<Mta, OcrError> {
        keep_mta_alive();
        // SAFETY: khởi tạo COM cho chính luồng này; cặp CoUninitialize nằm trong Drop của Mta.
        let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        if hr.is_err() {
            // Luồng đã ở STA (RPC_E_CHANGED_MODE) hoặc lỗi khác: không có gì để hủy.
            return Err(OcrError::Failed(format!("CoInitializeEx: {:#010x}", hr.0)));
        }
        Ok(Mta)
    }
}

impl Drop for Mta {
    fn drop(&mut self) {
        // SAFETY: cân bằng đúng một CoInitializeEx thành công (S_OK hoặc S_FALSE) trên luồng này.
        unsafe { CoUninitialize() };
    }
}

/// Crate `windows` lưu nhà máy WinRT agile vào biến tĩnh dùng chung mọi luồng. Giữ MTA của tiến
/// trình sống suốt đời tiến trình để COM không dỡ thư viện OCR khi luồng MTA cuối cùng rời đi;
/// nếu không, con trỏ nhà máy đã lưu có thể treo ở lần nhận dạng sau.
fn keep_mta_alive() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        // SAFETY: chỉ tăng bộ đếm MTA của tiến trình; cookie cố ý không bao giờ trả lại.
        let _ = unsafe { CoIncrementMTAUsage() };
    });
}

fn fail(what: &'static str) -> impl Fn(windows::core::Error) -> OcrError {
    move |e| OcrError::Failed(format!("{what}: {:#010x}", e.code().0))
}

fn recognize_in_mta(image: &Image) -> Result<OcrResult, OcrError> {
    // Lỗi hay null (hồ sơ người dùng không có ngôn ngữ OCR nào) đều là NoLanguage.
    let engine =
        OcrEngine::TryCreateFromUserProfileLanguages().map_err(|_| OcrError::NoLanguage)?;
    let max = OcrEngine::MaxImageDimension().map_err(fail("MaxImageDimension"))?;
    let (w, h) = ocr_scale(image.width, image.height, max).unwrap_or((image.width, image.height));
    let pixels = scale_bgra(&image.bgra, image.width, image.height, w, h);
    let (Ok(wi), Ok(hi)) = (i32::try_from(w), i32::try_from(h)) else {
        return Err(OcrError::Failed("image too large".into()));
    };

    let writer = DataWriter::new().map_err(fail("DataWriter"))?;
    writer.WriteBytes(&pixels).map_err(fail("WriteBytes"))?;
    drop(pixels);
    let buffer = writer.DetachBuffer().map_err(fail("DetachBuffer"))?;
    drop(writer);
    // OCR nhận Bgra8 (hoặc Gray8); alpha đã là 255 nên Premultiplied đúng nghĩa.
    let bitmap = SoftwareBitmap::CreateCopyWithAlphaFromBuffer(
        &buffer,
        BitmapPixelFormat::Bgra8,
        wi,
        hi,
        BitmapAlphaMode::Premultiplied,
    )
    .map_err(fail("SoftwareBitmap"))?;
    drop(buffer);

    let result = engine
        .RecognizeAsync(&bitmap)
        .map_err(fail("RecognizeAsync"))?
        .join()
        .map_err(fail("RecognizeAsync.join"))?;
    let lines = result.Lines().map_err(fail("Lines"))?;
    let n = lines.Size().map_err(fail("Lines.Size"))?;
    let mut texts = Vec::with_capacity(n as usize);
    for i in 0..n {
        let line = lines.GetAt(i).map_err(fail("Lines.GetAt"))?;
        texts.push(line.Text().map_err(fail("OcrLine.Text"))?.to_string_lossy());
    }
    let text = join_lines(&texts);

    let lang = engine
        .RecognizerLanguage()
        .map_err(fail("RecognizerLanguage"))?;
    let lang_tag = lang
        .LanguageTag()
        .map_err(fail("LanguageTag"))?
        .to_string_lossy();
    let lang_name = lang
        .DisplayName()
        .map_err(fail("DisplayName"))?
        .to_string_lossy();

    Ok(OcrResult {
        lines: line_count(&text),
        text,
        lang_tag,
        lang_name,
    })
}

/// Co giãn ảnh BGRA `w×h` thành `nw×nh`: mỗi điểm đích lấy trung bình khối nguồn tương ứng
/// (thu nhỏ = hộp trung bình; phóng to = láng giềng gần nhất). Alpha luôn 255.
/// Đầu vào sai cỡ → ảnh đích toàn 0 (không panic; `recognize` đã kiểm tra trước).
fn scale_bgra(src: &[u8], w: u32, h: u32, nw: u32, nh: u32) -> Vec<u8> {
    let (w, h, nw, nh) = (w as usize, h as usize, nw as usize, nh as usize);
    let mut out = vec![0u8; nw * nh * 4];
    if w == 0 || h == 0 || nw == 0 || nh == 0 || src.len() != w * h * 4 {
        return out;
    }
    if (w, h) == (nw, nh) {
        out.copy_from_slice(src);
    } else {
        // Khoảng nguồn [a, b) của chỉ số đích i khi đổi trục dài `len` thành `n` (không rỗng).
        let span = |i: usize, n: usize, len: usize| {
            let a = i * len / n;
            let b = ((i + 1) * len / n).max(a + 1).min(len);
            (a, b)
        };
        let xs: Vec<(usize, usize)> = (0..nw).map(|x| span(x, nw, w)).collect();
        for y in 0..nh {
            let (y0, y1) = span(y, nh, h);
            for (x, &(x0, x1)) in xs.iter().enumerate() {
                let mut sum = [0u64; 3];
                for sy in y0..y1 {
                    let row = &src[(sy * w + x0) * 4..(sy * w + x1) * 4];
                    for p in row.as_chunks::<4>().0 {
                        sum[0] += u64::from(p[0]);
                        sum[1] += u64::from(p[1]);
                        sum[2] += u64::from(p[2]);
                    }
                }
                let cnt = ((y1 - y0) * (x1 - x0)) as u64;
                let o = (y * nw + x) * 4;
                for c in 0..3 {
                    out[o + c] = ((sum[c] + cnt / 2) / cnt) as u8;
                }
            }
        }
    }
    for p in out.as_chunks_mut::<4>().0 {
        p[3] = 255;
    }
    out
}

/// Đăng kết quả bằng `PostMessageW`; gửi không được thì tự giải phóng Box. Trả về đã gửi hay chưa.
fn post_result(hwnd: isize, msg: u32, token: usize, res: Result<OcrResult, OcrError>) -> bool {
    let p = Box::into_raw(Box::new(res));
    // SAFETY: PostMessageW an toàn từ luồng bất kỳ; khi thất bại không ai nhận con trỏ nên lấy lại.
    unsafe {
        let ok = PostMessageW(
            Some(HWND(hwnd as *mut c_void)),
            msg,
            WPARAM(token),
            LPARAM(p as isize),
        )
        .is_ok();
        if !ok {
            drop(Box::from_raw(p));
        }
        ok
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use darkshot_core::geometry::Rect;
    use darkshot_core::ocr_text::{compose_all, join_lines, OcrLine, TextOptions};
    use std::ffi::c_void;
    use std::time::{Duration, Instant};
    use windows::core::w;
    use windows::Win32::Foundation::{RECT, WPARAM};
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, CreateDIBSection, CreateFontIndirectW, DeleteDC, DeleteObject,
        DrawTextW, GdiFlush, SelectObject, SetBkMode, SetTextColor, BITMAPINFO, BITMAPINFOHEADER,
        BI_RGB, DIB_RGB_COLORS, DT_CALCRECT, DT_LEFT, DT_NOPREFIX, DT_SINGLELINE, DT_TOP, LOGFONTW,
        TRANSPARENT,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, PeekMessageW, HWND_MESSAGE, MSG, PM_REMOVE,
        WINDOW_EX_STYLE, WINDOW_STYLE, WM_APP,
    };

    fn px(img: &[u8], w: u32, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * w + x) * 4) as usize;
        [img[i], img[i + 1], img[i + 2], img[i + 3]]
    }

    #[test]
    fn scale_bgra_box_average_when_shrinking() {
        // 2×2 → 1×1: trung bình bốn điểm; alpha thành 255.
        let src = [
            0, 10, 20, 0, //
            100, 110, 120, 0, //
            0, 10, 20, 0, //
            100, 110, 120, 0,
        ];
        let out = scale_bgra(&src, 2, 2, 1, 1);
        assert_eq!(out, vec![50, 60, 70, 255]);

        // 4×2 → 2×1: mỗi điểm đích là trung bình khối 2×2 của nó.
        let mut src = vec![0u8; 4 * 2 * 4];
        for y in 0..2u32 {
            for x in 0..4u32 {
                let i = ((y * 4 + x) * 4) as usize;
                let v = if x < 2 { 40 } else { 200 };
                src[i..i + 4].copy_from_slice(&[v, v, v, 255]);
            }
        }
        let out = scale_bgra(&src, 4, 2, 2, 1);
        assert_eq!(out.len(), 2 * 4);
        assert_eq!(px(&out, 2, 0, 0), [40, 40, 40, 255]);
        assert_eq!(px(&out, 2, 1, 0), [200, 200, 200, 255]);
    }

    #[test]
    fn scale_bgra_nearest_when_doubling() {
        let src = [1, 2, 3, 255, 4, 5, 6, 255];
        let out = scale_bgra(&src, 2, 1, 4, 2);
        assert_eq!(out.len(), 4 * 2 * 4);
        for y in 0..2 {
            assert_eq!(px(&out, 4, 0, y), [1, 2, 3, 255]);
            assert_eq!(px(&out, 4, 1, y), [1, 2, 3, 255]);
            assert_eq!(px(&out, 4, 2, y), [4, 5, 6, 255]);
            assert_eq!(px(&out, 4, 3, y), [4, 5, 6, 255]);
        }
    }

    #[test]
    fn scale_bgra_sizes() {
        let src = vec![128u8; 300 * 7 * 4];
        let out = scale_bgra(&src, 300, 7, 150, 4);
        assert_eq!(out.len(), 150 * 4 * 4);
        assert!(out.chunks(4).all(|p| p == [128, 128, 128, 255]));
        let out = scale_bgra(&src, 300, 7, 600, 14);
        assert_eq!(out.len(), 600 * 14 * 4);
        // Cùng cỡ: chép nguyên, alpha 255.
        let out = scale_bgra(&src, 300, 7, 300, 7);
        assert_eq!(out.len(), src.len());
        assert!(out.chunks(4).all(|p| p == [128, 128, 128, 255]));
    }

    #[test]
    fn recognize_rejects_invalid_image() {
        let img = Image {
            width: 2,
            height: 2,
            bgra: vec![0; 3],
        };
        assert!(matches!(recognize(&img), Err(OcrError::Failed(_))));
    }

    /// Cửa sổ chỉ-thông-điệp tạm (không bao giờ hiện); hủy khi drop.
    struct MsgWindow(HWND);

    impl MsgWindow {
        fn new() -> MsgWindow {
            // SAFETY: lớp "STATIC" có sẵn; con của HWND_MESSAGE không hiện; hủy trong Drop.
            let h = unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    w!("STATIC"),
                    w!("darkshot-ocr-test"),
                    WINDOW_STYLE(0),
                    0,
                    0,
                    0,
                    0,
                    Some(HWND_MESSAGE),
                    None,
                    None,
                    None,
                )
            };
            MsgWindow(h.expect("message-only window"))
        }
    }

    impl Drop for MsgWindow {
        fn drop(&mut self) {
            // SAFETY: cửa sổ do luồng này tạo, hủy một lần.
            unsafe {
                let _ = DestroyWindow(self.0);
            }
        }
    }

    #[test]
    fn post_result_to_destroyed_window_fails_and_frees() {
        let w = MsgWindow::new();
        let raw = w.0 .0 as isize;
        drop(w);
        assert!(!post_result(raw, WM_APP + 7, 1, Err(OcrError::NoLanguage)));
    }

    #[test]
    fn post_result_delivers_box_and_token() {
        let w = MsgWindow::new();
        let raw = w.0 .0 as isize;
        let res = Ok(OcrResult {
            text: "a".into(),
            lang_tag: "en-US".into(),
            lang_name: "English".into(),
            lines: 1,
            lines_detail: vec![OcrLine {
                text: "a".into(),
                rect: Rect {
                    x: 1,
                    y: 2,
                    w: 3,
                    h: 4,
                },
            }],
        });
        assert!(post_result(raw, WM_APP + 8, 42, res.clone()));
        let mut m = MSG::default();
        // SAFETY: lấy thông điệp của cửa sổ do luồng này tạo.
        let got = unsafe { PeekMessageW(&mut m, Some(w.0), WM_APP + 8, WM_APP + 8, PM_REMOVE) };
        assert!(got.as_bool());
        assert_eq!(m.wParam, WPARAM(42));
        // SAFETY: thông điệp vừa nhận do post_result gửi, lấy đúng một lần.
        let b = unsafe { take_result(m.lParam) };
        assert_eq!(*b, res);
    }

    /// Ảnh trắng `w×h`, có `text` đen (phông 32 px) vẽ bằng GDI vào DIB trong bộ nhớ.
    pub(crate) fn text_image(w: i32, h: i32, text: Option<&str>) -> Image {
        let lines: Vec<(&str, i32)> = text.map(|t| (t, h / 2)).into_iter().collect();
        lines_image(w, h, 32, &lines).0
    }

    /// Ảnh trắng `w×h`; mỗi `(chữ, tâm y)` vẽ đen một dòng, phông `font_px`, lề trái 20 px.
    /// Trả kèm khung thật của từng dòng đã vẽ (DT_CALCRECT).
    pub(crate) fn lines_image(
        w: i32,
        h: i32,
        font_px: i32,
        lines: &[(&str, i32)],
    ) -> (Image, Vec<RECT>) {
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
        let len = (w as usize) * (h as usize) * 4;
        let mut out = vec![255u8; len];
        let mut drawn = Vec::with_capacity(lines.len());
        let mut bits: *mut c_void = std::ptr::null_mut();
        // SAFETY: DC, DIB và phông chỉ dùng trong khối này và đều được giải phóng ở cuối.
        unsafe {
            let dc = CreateCompatibleDC(None);
            assert!(!dc.is_invalid());
            let bmp =
                CreateDIBSection(Some(dc), &bmi, DIB_RGB_COLORS, &mut bits, None, 0).expect("dib");
            assert!(!bits.is_null());
            std::ptr::write_bytes(bits.cast::<u8>(), 255, len);
            let old = SelectObject(dc, bmp.into());
            let mut lf = LOGFONTW {
                lfHeight: -font_px,
                lfWeight: 400,
                ..Default::default()
            };
            for (i, c) in "Segoe UI".encode_utf16().enumerate() {
                lf.lfFaceName[i] = c;
            }
            let font = CreateFontIndirectW(&lf);
            let old_font = SelectObject(dc, font.into());
            SetBkMode(dc, TRANSPARENT);
            SetTextColor(dc, windows::Win32::Foundation::COLORREF(0));
            for &(t, cy) in lines {
                let mut s: Vec<u16> = t.encode_utf16().collect();
                let mut rc = RECT {
                    left: 20,
                    top: 0,
                    right: w - 20,
                    bottom: h,
                };
                DrawTextW(
                    dc,
                    &mut s,
                    &mut rc,
                    DT_CALCRECT | DT_SINGLELINE | DT_NOPREFIX,
                );
                let th = rc.bottom - rc.top;
                rc.top = cy - th / 2;
                rc.bottom = rc.top + th;
                DrawTextW(
                    dc,
                    &mut s,
                    &mut rc,
                    DT_LEFT | DT_TOP | DT_SINGLELINE | DT_NOPREFIX,
                );
                drawn.push(rc);
            }
            let _ = GdiFlush();
            std::ptr::copy_nonoverlapping(bits.cast::<u8>(), out.as_mut_ptr(), len);
            SelectObject(dc, old_font);
            let _ = DeleteObject(font.into());
            SelectObject(dc, old);
            let _ = DeleteObject(bmp.into());
            let _ = DeleteDC(dc);
        }
        for p in out.chunks_mut(4) {
            p[3] = 255;
        }
        let img = Image {
            width: w as u32,
            height: h as u32,
            bgra: out,
        };
        (img, drawn)
    }

    /// Khung `r` nằm trong ảnh `w×h` và (nới `tol` px) chứa tâm khung đã vẽ `d`.
    fn assert_line_box(r: Rect, w: u32, h: u32, d: &RECT, tol: i32) {
        assert!(r.w > 0 && r.h > 0, "empty box {r:?}");
        assert!(r.x >= 0 && r.y >= 0, "box {r:?}");
        assert!(
            r.x + r.w <= w as i32 && r.y + r.h <= h as i32,
            "box {r:?} in {w}x{h}"
        );
        let (cx, cy) = ((d.left + d.right) / 2, (d.top + d.bottom) / 2);
        assert!(
            cx >= r.x - tol && cx <= r.x + r.w + tol && cy >= r.y - tol && cy <= r.y + r.h + tol,
            "box {r:?} misses drawn centre ({cx},{cy}) of {d:?}"
        );
    }

    fn texts_of(r: &OcrResult) -> Vec<String> {
        r.lines_detail.iter().map(|l| l.text.clone()).collect()
    }

    #[test]
    fn union_rect_floors_and_ceils() {
        assert_eq!(union_rect(&[]), None);
        let r = union_rect(&[(10.4, 20.6, 5.0, 4.0), (30.0, 18.2, 10.3, 8.0)]).unwrap();
        // x: floor(10.4)=10 .. ceil(40.3)=41; y: floor(18.2)=18 .. ceil(26.2)=27.
        assert_eq!(
            r,
            Rect {
                x: 10,
                y: 18,
                w: 31,
                h: 9
            }
        );
        let r = union_rect(&[(1.0, 2.0, 3.0, 4.0)]).unwrap();
        assert_eq!(
            r,
            Rect {
                x: 1,
                y: 2,
                w: 3,
                h: 4
            }
        );
    }

    #[test]
    fn union_rect_ignores_non_finite_and_negative_size() {
        // NaN/vô cực bị bỏ; cỡ âm coi như 0.
        let r = union_rect(&[(f32::NAN, 0.0, 1.0, 1.0), (5.0, 6.0, -3.0, 2.0)]).unwrap();
        assert_eq!(
            r,
            Rect {
                x: 5,
                y: 6,
                w: 0,
                h: 2
            }
        );
        assert_eq!(union_rect(&[(f32::INFINITY, 0.0, 1.0, 1.0)]), None);
    }

    // Chạy tay: cargo test -p darkshot-windows ocr::tests::manual -- --ignored --nocapture --test-threads=1
    #[test]
    #[ignore]
    fn manual_recognize_text_image() {
        let img = text_image(600, 120, Some("Hello darkshot 123"));
        match recognize(&img) {
            Ok(r) => {
                println!(
                    "lang={} ({}) lines={} text={:?}",
                    r.lang_tag, r.lang_name, r.lines, r.text
                );
                let low = r.text.to_lowercase();
                assert!(low.contains("hello"), "text: {:?}", r.text);
                assert!(low.contains("123"), "text: {:?}", r.text);
                assert!(r.lines >= 1);
                assert!(!r.lang_tag.is_empty());
            }
            Err(OcrError::NoLanguage) => println!("NoLanguage: may khong co goi OCR"),
            Err(e) => panic!("{e:?}"),
        }
    }

    #[test]
    #[ignore]
    fn manual_recognize_blank_image() {
        let img = text_image(400, 100, None);
        match recognize(&img) {
            Ok(r) => {
                assert_eq!(r.text, "");
                assert_eq!(r.lines, 0);
            }
            Err(OcrError::NoLanguage) => println!("NoLanguage: may khong co goi OCR"),
            Err(e) => panic!("{e:?}"),
        }
    }

    #[test]
    #[ignore]
    fn manual_start_posts_exactly_one_result() {
        let w = MsgWindow::new();
        let msg = WM_APP + 9;
        let img = text_image(600, 120, Some("Hello darkshot 123"));
        start(img, w.0, msg, 77).expect("start");
        let mut m = MSG::default();
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut got = None;
        while Instant::now() < deadline {
            // SAFETY: lấy thông điệp của cửa sổ do luồng này tạo.
            if unsafe { PeekMessageW(&mut m, Some(w.0), msg, msg, PM_REMOVE) }.as_bool() {
                assert_eq!(m.wParam, WPARAM(77));
                // SAFETY: thông điệp do `start` gửi, lấy đúng một lần.
                got = Some(unsafe { take_result(m.lParam) });
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let b = got.expect("no result within 30 s");
        match *b {
            Ok(r) => assert!(r.text.to_lowercase().contains("hello"), "{:?}", r.text),
            Err(OcrError::NoLanguage) => println!("NoLanguage: may khong co goi OCR"),
            Err(e) => panic!("{e:?}"),
        }
        std::thread::sleep(Duration::from_millis(300));
        // SAFETY: như trên; chỉ kiểm tra không có thông điệp thứ hai.
        let again = unsafe { PeekMessageW(&mut m, Some(w.0), msg, msg, PM_REMOVE) }.as_bool();
        assert!(!again, "a second result was posted");
    }

    #[test]
    #[ignore]
    fn manual_three_lines_have_boxes() {
        let wanted = [
            ("Alpha line one", 50),
            ("Bravo line two", 150),
            ("Charlie line three", 250),
        ];
        let (img, drawn) = {
            let _gdi = crate::capture::GDI_TEST_LOCK.lock();
            lines_image(600, 300, 32, &wanted)
        };
        match recognize(&img) {
            Ok(r) => {
                println!("lines={} detail={:?}", r.lines, r.lines_detail);
                assert_eq!(r.lines_detail.len(), 3, "{:?}", r.lines_detail);
                assert_eq!(r.lines, 3);
                for (i, (l, d)) in r.lines_detail.iter().zip(&drawn).enumerate() {
                    let word = ["alpha", "bravo", "charlie"][i];
                    assert!(l.text.to_lowercase().contains(word), "{:?}", l.text);
                    assert_line_box(l.rect, img.width, img.height, d, 4);
                }
                for p in r.lines_detail.windows(2) {
                    assert!(p[0].rect.y < p[1].rect.y, "{:?}", r.lines_detail);
                }
                // Chuỗi cũ của cùng ảnh: các dòng OCR trim rồi nối "\r\n" (ảnh không có dòng rỗng).
                assert_eq!(r.text, join_lines(&texts_of(&r)));
                assert_eq!(r.text, compose_all(&r.lines_detail, TextOptions::default()));
            }
            Err(OcrError::NoLanguage) => println!("NoLanguage: may khong co goi OCR"),
            Err(e) => panic!("{e:?}"),
        }
    }

    #[test]
    #[ignore]
    fn manual_small_image_boxes_in_source_pixels() {
        // Cạnh ngắn 36 < 40: ảnh được phóng ×2 trước OCR; khung phải về lại px ảnh gốc.
        assert_eq!(ocr_scale(400, 36, 10_000), Some((800, 72)));
        let (img, drawn) = {
            let _gdi = crate::capture::GDI_TEST_LOCK.lock();
            lines_image(400, 36, 22, &[("Hello darkshot 123", 18)])
        };
        match recognize(&img) {
            Ok(r) => {
                println!("lines={} detail={:?}", r.lines, r.lines_detail);
                assert_eq!(r.lines_detail.len(), 1, "{:?}", r.lines_detail);
                assert!(r.text.to_lowercase().contains("hello"), "{:?}", r.text);
                let b = r.lines_detail[0].rect;
                assert_line_box(b, img.width, img.height, &drawn[0], 4);
                // Khung đúng px gốc: không rộng hơn dòng đã vẽ quá 4 px mỗi phía.
                assert!(b.x >= drawn[0].left - 4, "{b:?} vs {:?}", drawn[0]);
                assert!(b.x + b.w <= drawn[0].right + 4, "{b:?} vs {:?}", drawn[0]);
                assert_eq!(r.text, join_lines(&texts_of(&r)));
            }
            Err(OcrError::NoLanguage) => println!("NoLanguage: may khong co goi OCR"),
            Err(e) => panic!("{e:?}"),
        }
    }

    #[test]
    #[ignore]
    fn manual_wide_image_shrunk_boxes_in_source_pixels() {
        // Cạnh dài vượt MaxImageDimension: ảnh bị co trước OCR; khung vẫn ở px ảnh gốc.
        let (img, drawn) = {
            let _gdi = crate::capture::GDI_TEST_LOCK.lock();
            lines_image(12_000, 200, 48, &[("Hello darkshot 123", 100)])
        };
        match recognize(&img) {
            Ok(r) => {
                println!("lines={} detail={:?}", r.lines, r.lines_detail);
                assert_eq!(r.lines_detail.len(), 1, "{:?}", r.lines_detail);
                assert_line_box(r.lines_detail[0].rect, img.width, img.height, &drawn[0], 6);
            }
            Err(OcrError::NoLanguage) => println!("NoLanguage: may khong co goi OCR"),
            Err(e) => panic!("{e:?}"),
        }
    }
}
