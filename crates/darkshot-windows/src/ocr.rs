//! Nhận dạng chữ cục bộ bằng Windows.Media.Ocr (WinRT), chạy trên luồng nền `darkshot-ocr`.
//! Ảnh chỉ ở bộ nhớ; không ghi tệp, không nhật ký chữ, không mạng.

use std::ffi::c_void;
use std::sync::{Arc, Once};
use std::time::{Duration, Instant};

use darkshot_core::geometry::Rect;
use darkshot_core::image::Image;
use darkshot_core::ocr_text::{compose_all, map_rect, ocr_scale, OcrLine, OcrWord, TextOptions};
use darkshot_core::privacy::{self, text::wipe, Finding, ScanError, ScanReport};
use windows::core::HSTRING;
use windows::Globalization::Language;
use windows::Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::{OcrEngine, OcrResult as WinOcrResult};
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
    /// Các dòng có chữ, theo thứ tự WinRT trả; khung ở px ảnh gốc. `lines == lines_detail.len()`.
    pub lines_detail: Vec<OcrLine>,
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
    let bitmap = bitmap_from(&pixels, w, h).map_err(fail("SoftwareBitmap"))?;
    drop(pixels);

    let result = engine
        .RecognizeAsync(&bitmap)
        .map_err(fail("RecognizeAsync"))?
        .join()
        .map_err(fail("RecognizeAsync.join"))?;
    drop(bitmap);
    let mut detail = Vec::new();
    read_lines(&result, w, h, image.width, image.height, &mut detail).map_err(fail("Lines"))?;
    let text = compose_all(&detail, TextOptions::default());

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
        lines: detail.len(),
        text,
        lang_tag,
        lang_name,
        lines_detail: detail,
    })
}

/// `SoftwareBitmap` Bgra8 `w×h` từ điểm ảnh BGRA (alpha 255; Premultiplied đúng nghĩa).
fn bitmap_from(pixels: &[u8], w: u32, h: u32) -> windows::core::Result<SoftwareBitmap> {
    let (Ok(wi), Ok(hi)) = (i32::try_from(w), i32::try_from(h)) else {
        return Err(windows::core::Error::empty());
    };
    let writer = DataWriter::new()?;
    writer.WriteBytes(pixels)?;
    let buffer = writer.DetachBuffer()?;
    drop(writer);
    // OCR nhận Bgra8 (hoặc Gray8). Bitmap là bản sao: thả bộ đệm ngay sau khi tạo.
    let bitmap = SoftwareBitmap::CreateCopyWithAlphaFromBuffer(
        &buffer,
        BitmapPixelFormat::Bgra8,
        wi,
        hi,
        BitmapAlphaMode::Premultiplied,
    );
    drop(buffer);
    bitmap
}

/// Thêm vào `out` các dòng có chữ của kết quả WinRT, kèm chữ và khung từng từ; khung đổi từ px ảnh
/// đã đưa vào OCR (`fw×fh`) về px `tw×th`. `OcrLine.rect` = hợp khung từ; dòng không có khung nào
/// bị bỏ (thiết kế 7b.1). Từ khung hỏng: giữ chữ, khung rỗng. Chữ của dòng bị bỏ được ghi đè.
fn read_lines(
    result: &WinOcrResult,
    fw: u32,
    fh: u32,
    tw: u32,
    th: u32,
    out: &mut Vec<OcrLine>,
) -> windows::core::Result<()> {
    let lines = result.Lines()?;
    let n = lines.Size()?;
    out.reserve(n as usize);
    let mut boxes = Vec::new();
    for i in 0..n {
        let line = lines.GetAt(i)?;
        let mut g = LineGuard(OcrLine {
            text: hstring_text(&line.Text()?),
            rect: Rect {
                x: 0,
                y: 0,
                w: 0,
                h: 0,
            },
            words: Vec::new(),
        });
        if g.0.text.trim().is_empty() {
            continue;
        }
        let words = line.Words()?;
        let nw = words.Size()?;
        g.0.words.reserve_exact(nw as usize);
        boxes.clear();
        for k in 0..nw {
            let word = words.GetAt(k)?;
            // Khung đọc trước chữ: lỗi `BoundingRect()` không để lại chữ từ chưa ghi đè.
            let r = word.BoundingRect()?;
            let text = hstring_text(&word.Text()?);
            let b = (r.X, r.Y, r.Width, r.Height);
            g.0.words.push(OcrWord {
                text,
                rect: word_rect(b, fw, fh, tw, th),
            });
            boxes.push(b);
        }
        let Some(r) = union_rect(&boxes) else {
            continue;
        };
        g.0.rect = map_rect(r, fw, fh, tw, th);
        out.push(g.take());
    }
    Ok(())
}

/// Hợp các khung `(x, y, rộng, cao)` (f32, px ảnh đã đưa vào OCR): góc trên-trái làm tròn xuống,
/// góc dưới-phải làm tròn lên. Khung có số không hữu hạn bị bỏ; cỡ âm coi như 0.
/// Không còn khung nào → None.
fn union_rect(boxes: &[(f32, f32, f32, f32)]) -> Option<Rect> {
    let mut acc: Option<(f32, f32, f32, f32)> = None;
    for &(x, y, w, h) in boxes {
        if ![x, y, w, h].iter().all(|v| v.is_finite()) {
            continue;
        }
        let (r, b) = (x + w.max(0.0), y + h.max(0.0));
        if !(r.is_finite() && b.is_finite()) {
            continue;
        }
        acc = Some(match acc {
            None => (x, y, r, b),
            Some((l, t, rr, bb)) => (l.min(x), t.min(y), rr.max(r), bb.max(b)),
        });
    }
    // `as i32` bão hòa với số quá lớn; khung hợp lệ của OCR luôn nằm trong ảnh.
    let (l, t, r, b) = acc?;
    let (l, t) = (l.floor() as i32, t.floor() as i32);
    let (r, b) = (r.ceil() as i32, b.ceil() as i32);
    Some(Rect {
        x: l,
        y: t,
        w: r.saturating_sub(l),
        h: b.saturating_sub(t),
    })
}

/// Co giãn ảnh BGRA `w×h` thành `nw×nh`: mỗi điểm đích lấy trung bình khối nguồn tương ứng
/// (thu nhỏ = hộp trung bình; phóng to = láng giềng gần nhất). Alpha luôn 255.
/// Đầu vào sai cỡ → ảnh đích toàn 0 (không panic; `recognize` đã kiểm tra trước).
pub(crate) fn scale_bgra(src: &[u8], w: u32, h: u32, nw: u32, nh: u32) -> Vec<u8> {
    try_scale_bgra(src, w, h, nw, nh).unwrap_or_else(|| vec![0u8; nw as usize * nh as usize * 4])
}

/// Như `scale_bgra`, nhưng không cấp được bộ nhớ đích → None (không hủy tiến trình).
fn try_scale_bgra(src: &[u8], w: u32, h: u32, nw: u32, nh: u32) -> Option<Vec<u8>> {
    let (w, h, nw, nh) = (w as usize, h as usize, nw as usize, nh as usize);
    let len = nw.checked_mul(nh)?.checked_mul(4)?;
    let mut out = Vec::new();
    out.try_reserve_exact(len).ok()?;
    out.resize(len, 0u8);
    if w == 0 || h == 0 || nw == 0 || nh == 0 || src.len() != w * h * 4 {
        return Some(out);
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
    Some(out)
}

// ---- Privacy Shield: quét OCR theo dải trên luồng nền (Task 128) ----

/// Hai dải liền nhau chồng ít nhất chừng này px (lớn hơn chiều cao dòng thường gặp).
pub const SCAN_TILE_OVERLAP: u32 = 128;
/// Thời hạn mỗi lượt nhận dạng một dải; quá hạn = cả lượt quét lỗi `Timeout`. Một dải có tối đa
/// `SCAN_PASSES_PER_TILE` lượt (bản gốc, rồi bản sao đã bỏ đường kẻ bảng).
pub const SCAN_TILE_TIMEOUT: Duration = Duration::from_secs(20);
pub const SCAN_PASSES_PER_TILE: u32 = 2;

/// Giá trị `Windows.Foundation.AsyncStatus` (crate `windows` không tái xuất kiểu này).
const ASYNC_STARTED: i32 = 0;
const ASYNC_COMPLETED: i32 = 1;

/// Một dải quét: hàng đầu `y` và chiều cao `h` (px ảnh gốc, cả chiều rộng ảnh).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Tile {
    pub y: u32,
    pub h: u32,
}

/// Cắt chiều cao `height` thành dải ngang cao ≤ `max`: `height ≤ max` → một dải; không thì các dải
/// cao `max` cách nhau `max - overlap`, dải cuối áp sát đáy (chồng ≥ `overlap` với dải trước).
/// `max ≤ overlap` (không xảy ra với WinRT) thì bước = nửa `max`. `height == 0` hay `max == 0` → rỗng.
pub(crate) fn scan_tiles(height: u32, max: u32, overlap: u32) -> Vec<Tile> {
    if height == 0 || max == 0 {
        return Vec::new();
    }
    if height <= max {
        return vec![Tile { y: 0, h: height }];
    }
    let step = if max > overlap {
        max - overlap
    } else {
        max.div_ceil(2)
    };
    let last = height - max;
    let mut out = Vec::with_capacity(((last / step) + 2) as usize);
    let mut y = 0u32;
    while y < last {
        out.push(Tile { y, h: max });
        y = y.saturating_add(step);
    }
    out.push(Tile { y: last, h: max });
    out
}

/// Các hàng của dải `t` trong ảnh BGRA rộng `w` (hàng liền nhau nên là một lát cắt, không sao chép).
fn tile_rows(src: &[u8], w: u32, t: Tile) -> &[u8] {
    let row = w as usize * 4;
    &src[t.y as usize * row..(t.y as usize + t.h as usize) * row]
}

/// Đọc trạng thái thao tác bất đồng bộ tới khi rời `Started` (→ `Some(trạng thái)`) hoặc hết
/// `timeout` (→ `None`). Ngủ ngắn giữa các lượt đọc (2 ms, tăng dần tới 20 ms).
fn wait_status(
    timeout: Duration,
    mut status: impl FnMut() -> windows::core::Result<i32>,
) -> Result<Option<i32>, ScanError> {
    let deadline = Instant::now() + timeout;
    let mut nap = Duration::from_millis(2);
    loop {
        let s = status().map_err(|_| ScanError::Failed)?;
        if s != ASYNC_STARTED {
            return Ok(Some(s));
        }
        let now = Instant::now();
        if now >= deadline {
            return Ok(None);
        }
        std::thread::sleep(nap.min(deadline - now));
        nap = (nap * 2).min(Duration::from_millis(20));
    }
}

/// Khung một từ `(x, y, rộng, cao)` ở px ảnh đưa vào OCR (`fw×fh`) → px ảnh `tw×th`.
/// Khung hỏng (không hữu hạn, cỡ ≤ 0, rơi ngoài ảnh) → khung rỗng (bộ dò rơi về khung dòng).
fn word_rect(b: (f32, f32, f32, f32), fw: u32, fh: u32, tw: u32, th: u32) -> Rect {
    const EMPTY: Rect = Rect {
        x: 0,
        y: 0,
        w: 0,
        h: 0,
    };
    let (_, _, bw, bh) = b;
    if !(bw > 0.0 && bh > 0.0) {
        return EMPTY;
    }
    match union_rect(&[b]) {
        Some(r) if r.w > 0 && r.h > 0 => {
            let m = map_rect(r, fw, fh, tw, th);
            if m.w > 0 && m.h > 0 {
                m
            } else {
                EMPTY
            }
        }
        _ => EMPTY,
    }
}

/// LPARAM của thông báo tiến độ: `(k << 16) | n`, mỗi số kẹp ở 0xFFFF.
fn progress_lparam(k: u32, n: u32) -> LPARAM {
    LPARAM(((k.min(0xFFFF) << 16) | n.min(0xFFFF)) as isize)
}

/// Chép chữ WinRT sang `String` cấp đủ chỗ một lần (mỗi đơn vị UTF-16 ≤ 3 byte UTF-8), để không
/// bản sao nào bị bỏ lại khi chuỗi nới ra. Bản HSTRING gốc thuộc WinRT, không ghi đè được.
fn hstring_text(h: &HSTRING) -> String {
    let wide: &[u16] = h;
    let mut s = String::with_capacity(wide.len() * 3);
    s.extend(char::decode_utf16(wide.iter().copied()).map(|c| c.unwrap_or('\u{FFFD}')));
    s
}

/// Ghi đè chữ của dòng và từng từ bằng 0 rồi làm rỗng.
fn wipe_line(l: &mut OcrLine) {
    wipe(&mut l.text);
    for w in &mut l.words {
        wipe(&mut w.text);
    }
}

/// Một dòng đang dựng: chữ bị ghi đè khi thả (bỏ dòng hay lỗi giữa chừng), trừ khi đã `take`.
struct LineGuard(OcrLine);

impl LineGuard {
    fn take(mut self) -> OcrLine {
        std::mem::replace(
            &mut self.0,
            OcrLine {
                text: String::new(),
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 0,
                    h: 0,
                },
                words: Vec::new(),
            },
        )
    }
}

impl Drop for LineGuard {
    fn drop(&mut self) {
        wipe_line(&mut self.0);
    }
}

/// Các dòng OCR của một dải; mọi chữ bị ghi đè khi thả, trên mọi đường đi.
struct ScanLines(Vec<OcrLine>);

impl Drop for ScanLines {
    fn drop(&mut self) {
        for l in &mut self.0 {
            wipe_line(l);
        }
    }
}

/// Bộ nhận dạng của lượt quét: `en-US` nếu Windows có, không thì ngôn ngữ hồ sơ; không có → NoLanguage.
fn scan_engine() -> Result<OcrEngine, ScanError> {
    if let Some(e) = english_engine() {
        return Ok(e);
    }
    OcrEngine::TryCreateFromUserProfileLanguages().map_err(|_| ScanError::NoLanguage)
}

fn english_engine() -> Option<OcrEngine> {
    let lang = Language::CreateLanguage(&HSTRING::from("en-US")).ok()?;
    if !OcrEngine::IsLanguageSupported(&lang).ok()? {
        return None;
    }
    OcrEngine::TryCreateFromLanguage(&lang).ok()
}

/// Mọi điểm có alpha 255 (khi đó gửi thẳng điểm ảnh gốc cho OCR, không cần bản sao).
fn opaque(bgra: &[u8]) -> bool {
    bgra.as_chunks::<4>().0.iter().all(|p| p[3] == 255)
}

/// Nhận dạng một dải `src` (`w×h` px gốc), trả các dòng có khung ở px của dải. Một dải trong bộ
/// nhớ một lúc.
fn recognize_tile(
    engine: &OcrEngine,
    src: &[u8],
    w: u32,
    h: u32,
    max: u32,
) -> Result<ScanLines, ScanError> {
    let (ow, oh) = ocr_scale(w, h, max).unwrap_or((w, h));
    let bitmap = if (ow, oh) == (w, h) && opaque(src) {
        bitmap_from(src, ow, oh)
    } else {
        let px = try_scale_bgra(src, w, h, ow, oh).ok_or(ScanError::TooLarge)?;
        bitmap_from(&px, ow, oh)
    }
    .map_err(|_| ScanError::Failed)?;

    let op = engine
        .RecognizeAsync(&bitmap)
        .map_err(|_| ScanError::Failed)?;
    match wait_status(SCAN_TILE_TIMEOUT, || op.Status().map(|s| s.0)) {
        Ok(Some(ASYNC_COMPLETED)) => {}
        Ok(Some(_)) => return Err(ScanError::Failed),
        Ok(None) => {
            // Hết hạn: xin hủy (tốt nhất có thể); kết quả muộn không ai đọc.
            let _ = op.Cancel();
            return Err(ScanError::Timeout);
        }
        Err(e) => {
            let _ = op.Cancel();
            return Err(e);
        }
    }
    let result = op.GetResults().map_err(|_| ScanError::Failed)?;
    drop(op);
    drop(bitmap);
    let mut lines = ScanLines(Vec::new());
    read_lines(&result, ow, oh, w, h, &mut lines.0).map_err(|_| ScanError::Failed)?;
    Ok(lines)
}

/// Phát hiện của một dải `t`, tọa độ px của dải. Lượt một: OCR bản gốc. Lượt hai (Task 137), chỉ
/// khi dải có đường kẻ bảng: OCR bản sao đã bỏ đường kẻ (`privacy::rules::strip_rules`), cùng
/// engine, cùng tỉ lệ, cùng khung tọa độ. Lỗi hay hết hạn ở lượt nào cũng là lỗi quét. Chữ OCR mỗi
/// lượt bị ghi đè ngay sau khi dò; bản sao ảnh được nhả trước khi về.
fn scan_tile(
    engine: &OcrEngine,
    image: &Image,
    t: Tile,
    max: u32,
    two_pass: bool,
) -> Result<Vec<Finding>, ScanError> {
    let w = image.width;
    let src = tile_rows(&image.bgra, w, t);
    let lines = recognize_tile(engine, src, w, t.h, max)?;
    let mut found = privacy::detect(&lines.0);
    drop(lines);
    if !two_pass {
        return Ok(found);
    }
    if let Some(clean) = privacy::rules::strip_rules(src, w, t.h) {
        let lines = recognize_tile(engine, &clean.bgra, w, t.h, max);
        drop(clean);
        let lines = lines?;
        found.extend(privacy::detect(&lines.0));
        drop(lines);
    }
    Ok(found)
}

/// Quét trên luồng hiện tại (đã ở MTA): OCR từng dải (hai lượt nếu dải có đường kẻ bảng), dò,
/// dịch tọa độ, gộp. Chữ OCR chỉ sống trong `ScanLines` của từng lượt và bị ghi đè trước lượt sau.
fn scan_in_mta(
    image: &Image,
    t0: Instant,
    progress: impl Fn(u32, u32),
) -> Result<ScanReport, ScanError> {
    let engine = scan_engine()?;
    let max = OcrEngine::MaxImageDimension().map_err(|_| ScanError::Failed)?;
    let tiles = scan_tiles(image.height, max, SCAN_TILE_OVERLAP);
    if tiles.is_empty() {
        return Err(ScanError::Failed);
    }
    let n = u32::try_from(tiles.len()).map_err(|_| ScanError::TooLarge)?;
    if i32::try_from(image.height).is_err() {
        return Err(ScanError::TooLarge);
    }
    let mut findings: Vec<Finding> = Vec::new();
    for (k, &t) in tiles.iter().enumerate() {
        progress(k as u32 + 1, n);
        let found = scan_tile(&engine, image, t, max, true)?;
        privacy::merge_tile(&mut findings, found, t.y as i32);
    }
    Ok(ScanReport {
        findings,
        tiles: n,
        elapsed_ms: u32::try_from(t0.elapsed().as_millis()).unwrap_or(u32::MAX),
    })
}

/// Đồng bộ, chỉ gọi trên luồng nền (tự vào/ra MTA). `progress(k, n)` trước dải thứ k (từ 1).
fn scan(image: &Image, progress: impl Fn(u32, u32)) -> Result<ScanReport, ScanError> {
    let t0 = Instant::now();
    if !image.is_valid() {
        return Err(ScanError::Failed);
    }
    let _apartment = Mta::enter().map_err(|_| ScanError::Failed)?;
    // Mọi đối tượng WinRT sống trong hàm dưới và đã được nhả trước khi `_apartment` hủy apartment.
    scan_in_mta(image, t0, progress)
}

/// Tạo luồng `darkshot-ocr`, quét ảnh: OCR (en-US nếu có, không thì hồ sơ) theo dải, `privacy::detect`
/// trên luồng nền, gửi `Box<Result<ScanReport, ScanError>>` tới `hwnd` bằng `msg` (WPARAM = token).
/// Chữ OCR không bao giờ rời luồng nền. Không tạo được luồng → Err. Nếu có `progress_msg`, trước
/// mỗi dải gửi `PostMessage(hwnd, progress_msg, token, (k << 16) | n)` (k từ 1; không có Box,
/// không cần giải phóng).
pub fn start_scan_with_progress(
    image: Arc<Image>,
    hwnd: HWND,
    msg: u32,
    progress_msg: Option<u32>,
    token: usize,
) -> Result<(), String> {
    // HWND không Send; chuyển sang luồng nền dưới dạng số.
    let target = hwnd.0 as isize;
    std::thread::Builder::new()
        .name("darkshot-ocr".into())
        .spawn(move || {
            let res = scan(&image, |k, n| {
                if let Some(pm) = progress_msg {
                    post_progress(target, pm, token, k, n);
                }
            });
            drop(image);
            post_box(target, msg, token, res);
        })
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Bên nhận lấy lại quyền sở hữu: gọi đúng một lần cho mỗi thông điệp kết quả nhận được.
///
/// # Safety
/// `lp` phải là LPARAM của một thông điệp kết quả do `start_scan_with_progress` gửi và chưa được lấy lần nào.
pub unsafe fn take_scan_result(lp: LPARAM) -> Box<Result<ScanReport, ScanError>> {
    // SAFETY: theo hợp đồng của hàm, `lp` là con trỏ Box do `start_scan_with_progress` tạo, chưa ai lấy lại.
    unsafe { Box::from_raw(lp.0 as *mut Result<ScanReport, ScanError>) }
}

fn post_progress(hwnd: isize, msg: u32, token: usize, k: u32, n: u32) {
    // SAFETY: PostMessageW an toàn từ luồng bất kỳ; LPARAM chỉ là số, không có tài nguyên nào.
    let _ = unsafe {
        PostMessageW(
            Some(HWND(hwnd as *mut c_void)),
            msg,
            WPARAM(token),
            progress_lparam(k, n),
        )
    };
}

/// Đăng `Box<T>` bằng `PostMessageW`; gửi không được thì tự giải phóng Box. Trả về đã gửi hay chưa.
fn post_box<T>(hwnd: isize, msg: u32, token: usize, res: T) -> bool {
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

/// Đăng kết quả nhận dạng (cửa sổ OCR); gửi không được thì tự giải phóng Box.
fn post_result(hwnd: isize, msg: u32, token: usize, res: Result<OcrResult, OcrError>) -> bool {
    post_box(hwnd, msg, token, res)
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
    use windows::Win32::Graphics::Gdi::{GetTextMetricsW, TEXTMETRICW};
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
                words: Vec::new(),
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
    fn manual_tall_image_shrunk_boxes_in_source_pixels() {
        // Cạnh dài vượt MaxImageDimension (như ảnh chụp cuộn): ảnh bị co trước OCR; khung vẫn ở px gốc.
        let (img, drawn) = {
            let _gdi = crate::capture::GDI_TEST_LOCK.lock();
            lines_image(800, 12_000, 48, &[("Hello darkshot 123", 6_000)])
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

    // ---- Privacy Shield: quét theo dải (Task 128) ----

    use darkshot_core::privacy::{Category, ScanError, ScanReport};
    use std::sync::Arc;

    /// Bất biến của bộ cắt dải: phủ kín [0, h), mỗi dải ≤ max, dải đầu ở 0, dải cuối chạm đáy,
    /// y tăng ngặt, hai dải liền nhau chồng ít nhất `overlap` (khi max > overlap).
    fn check_tiles(h: u32, max: u32, overlap: u32) -> Vec<Tile> {
        let t = scan_tiles(h, max, overlap);
        assert!(!t.is_empty(), "h={h} max={max}");
        assert_eq!(t[0].y, 0);
        let last = t[t.len() - 1];
        assert_eq!(last.y + last.h, h, "last tile must end at the bottom");
        for x in &t {
            assert!(x.h >= 1 && x.h <= max, "{x:?} max={max}");
            assert!(x.y + x.h <= h);
        }
        for p in t.windows(2) {
            assert!(p[1].y > p[0].y, "{p:?}");
            // phủ kín: dải sau bắt đầu trước khi dải trước kết thúc
            assert!(p[1].y < p[0].y + p[0].h, "gap {p:?}");
            if max > overlap {
                assert!(p[0].y + p[0].h - p[1].y >= overlap, "overlap {p:?}");
            }
        }
        t
    }

    #[test]
    fn scan_tiles_single_when_fits() {
        assert_eq!(scan_tiles(1080, 10_000, 128), vec![Tile { y: 0, h: 1080 }]);
        // đúng max: vẫn một dải
        assert_eq!(
            scan_tiles(10_000, 10_000, 128),
            vec![Tile { y: 0, h: 10_000 }]
        );
        assert_eq!(scan_tiles(1, 10_000, 128), vec![Tile { y: 0, h: 1 }]);
        assert!(scan_tiles(0, 10_000, 128).is_empty());
        assert!(scan_tiles(100, 0, 128).is_empty());
    }

    #[test]
    fn scan_tiles_max_plus_one() {
        let t = check_tiles(10_001, 10_000, 128);
        assert_eq!(t, vec![Tile { y: 0, h: 10_000 }, Tile { y: 1, h: 10_000 }]);
    }

    #[test]
    fn scan_tiles_three_max_plus_one() {
        let t = check_tiles(30_001, 10_000, 128);
        // bước 9872; dải cuối áp sát đáy (30001 - 10000 = 20001).
        assert_eq!(
            t,
            vec![
                Tile { y: 0, h: 10_000 },
                Tile {
                    y: 9_872,
                    h: 10_000
                },
                Tile {
                    y: 19_744,
                    h: 10_000
                },
                Tile {
                    y: 20_001,
                    h: 10_000
                },
            ]
        );
    }

    #[test]
    fn scan_tiles_exact_steps() {
        // 10000 + 9872 = 19872: hai dải, không có dải thứ ba thừa.
        let t = check_tiles(19_872, 10_000, 128);
        assert_eq!(
            t,
            vec![
                Tile { y: 0, h: 10_000 },
                Tile {
                    y: 9_872,
                    h: 10_000
                }
            ]
        );
        let t = check_tiles(19_873, 10_000, 128);
        assert_eq!(t.len(), 3);
    }

    #[test]
    fn scan_tiles_invariants_many_heights() {
        for h in 1..2_000 {
            check_tiles(h, 300, 128);
        }
        for h in [20_000, 30_000, 99_999, 1_000_000] {
            check_tiles(h, 10_000, 128);
        }
        // max ≤ overlap (không xảy ra với WinRT): vẫn dừng và phủ kín.
        for h in [1, 99, 100, 101, 1_000] {
            check_tiles(h, 100, 128);
            check_tiles(h, 128, 128);
        }
    }

    #[test]
    fn tile_rows_are_the_tile_slice() {
        // ảnh 2×5: hàng y có mọi byte = y
        let w = 2u32;
        let mut src = Vec::new();
        for y in 0..5u8 {
            src.extend(std::iter::repeat_n(y, (w * 4) as usize));
        }
        let r = tile_rows(&src, w, Tile { y: 1, h: 3 });
        assert_eq!(r.len(), 3 * 8);
        assert!(r[..8].iter().all(|&b| b == 1));
        assert!(r[16..].iter().all(|&b| b == 3));
        assert_eq!(tile_rows(&src, w, Tile { y: 0, h: 5 }).len(), src.len());
    }

    #[test]
    fn wide_tile_is_shrunk_and_maps_back() {
        // Chiều rộng > max: dải co theo tỉ lệ như `ocr_scale`; khung về lại px gốc trong ảnh.
        let (ow, oh) = ocr_scale(12_000, 10_000, 10_000).unwrap();
        assert_eq!((ow, oh), (10_000, 8_333));
        let r = word_rect((9_990.0, 8_300.0, 10.0, 33.0), ow, oh, 12_000, 10_000);
        assert!(r.w > 0 && r.h > 0, "{r:?}");
        assert!(r.x + r.w <= 12_000 && r.y + r.h <= 10_000, "{r:?}");
        assert!(r.x >= 11_980, "{r:?}");
    }

    #[test]
    fn word_rect_maps_and_rejects_broken_boxes() {
        assert_eq!(
            word_rect((10.0, 20.0, 30.0, 8.0), 100, 100, 100, 100),
            Rect {
                x: 10,
                y: 20,
                w: 30,
                h: 8
            }
        );
        // ảnh đã phóng 2×: về px gốc, làm tròn ra ngoài
        assert_eq!(
            word_rect((11.0, 20.0, 30.0, 9.0), 200, 200, 100, 100),
            Rect {
                x: 5,
                y: 10,
                w: 16,
                h: 5
            }
        );
        let zero = Rect {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
        };
        assert_eq!(
            word_rect((f32::NAN, 0.0, 5.0, 5.0), 100, 100, 100, 100),
            zero
        );
        assert_eq!(
            word_rect((1.0, 1.0, f32::INFINITY, 5.0), 100, 100, 100, 100),
            zero
        );
        assert_eq!(word_rect((1.0, 1.0, -4.0, 5.0), 100, 100, 100, 100), zero);
        assert_eq!(word_rect((1.0, 1.0, 5.0, 0.0), 100, 100, 100, 100), zero);
        // nằm ngoài ảnh hoàn toàn: kẹp thành rỗng
        assert_eq!(word_rect((500.0, 1.0, 5.0, 5.0), 100, 100, 100, 100), zero);
    }

    #[test]
    fn word_rects_inside_line_rect_after_mapping() {
        let boxes = [
            (10.3, 20.7, 50.2, 30.1),
            (70.9, 18.2, 40.0, 33.3),
            (120.0, 21.0, 0.0, 30.0),
        ];
        for (fw, fh, tw, th) in [
            (800, 72, 400, 36),
            (1000, 500, 1000, 500),
            (833, 1000, 1000, 1200),
        ] {
            let line = map_rect(union_rect(&boxes).unwrap(), fw, fh, tw, th);
            for b in boxes {
                let r = word_rect(b, fw, fh, tw, th);
                if r.w == 0 {
                    continue;
                }
                assert!(
                    r.x >= line.x
                        && r.y >= line.y
                        && r.x + r.w <= line.x + line.w
                        && r.y + r.h <= line.y + line.h,
                    "{r:?} not in {line:?}"
                );
            }
        }
    }

    #[test]
    fn wait_status_finishes_and_times_out() {
        // Xong ngay
        assert_eq!(wait_status(Duration::from_secs(1), || Ok(1)), Ok(Some(1)));
        // Xong sau vài lượt đọc
        let mut n = 0;
        let got = wait_status(Duration::from_secs(5), || {
            n += 1;
            Ok(if n < 4 { 0 } else { 3 })
        });
        assert_eq!(got, Ok(Some(3)));
        assert_eq!(n, 4);
        // Không bao giờ xong: hết hạn, không treo
        let t0 = Instant::now();
        assert_eq!(wait_status(Duration::from_millis(60), || Ok(0)), Ok(None));
        let el = t0.elapsed();
        assert!(el >= Duration::from_millis(60), "{el:?}");
        assert!(el < Duration::from_secs(2), "{el:?}");
        // Lỗi đọc trạng thái → Failed (không mang mã lỗi)
        assert_eq!(
            wait_status(
                Duration::from_secs(1),
                || Err(windows::core::Error::empty())
            ),
            Err(ScanError::Failed)
        );
    }

    #[test]
    fn progress_lparam_packs_k_and_n() {
        assert_eq!(progress_lparam(1, 3), LPARAM((1 << 16) | 3));
        assert_eq!(progress_lparam(3, 3), LPARAM((3 << 16) | 3));
        assert_eq!(
            progress_lparam(70_000, 80_000),
            LPARAM((0xFFFF << 16) | 0xFFFF)
        );
    }

    #[test]
    fn hstring_text_decodes_without_regrowth() {
        let h = windows::core::HSTRING::from("ab\u{00e9}\u{1F600}");
        let s = hstring_text(&h);
        assert_eq!(s, "ab\u{00e9}\u{1F600}");
        // đủ chỗ ngay từ đầu (mỗi đơn vị UTF-16 ≤ 3 byte UTF-8): không bản sao cũ nào bị bỏ lại
        assert!(s.capacity() >= h.len() * 3);
        assert_eq!(hstring_text(&windows::core::HSTRING::new()), "");
    }

    #[test]
    fn wipe_line_clears_line_and_word_text() {
        let rect = Rect {
            x: 1,
            y: 1,
            w: 1,
            h: 1,
        };
        let mut l = OcrLine {
            text: "secret words".into(),
            rect,
            words: vec![
                darkshot_core::ocr_text::OcrWord {
                    text: "secret".into(),
                    rect,
                },
                darkshot_core::ocr_text::OcrWord {
                    text: "words".into(),
                    rect,
                },
            ],
        };
        wipe_line(&mut l);
        assert!(l.text.is_empty());
        assert!(l.words.iter().all(|w| w.text.is_empty()));
    }

    #[test]
    fn scan_rejects_invalid_image() {
        let img = Image {
            width: 2,
            height: 2,
            bgra: vec![0; 3],
        };
        assert_eq!(scan(&img, |_, _| {}), Err(ScanError::Failed));
    }

    #[test]
    fn scan_result_box_delivered_once_and_freed_on_failure() {
        let w = MsgWindow::new();
        let raw = w.0 .0 as isize;
        let rep: Result<ScanReport, ScanError> = Ok(ScanReport {
            findings: Vec::new(),
            tiles: 2,
            elapsed_ms: 5,
        });
        assert!(post_box(raw, WM_APP + 21, 9, rep.clone()));
        let mut m = MSG::default();
        // SAFETY: lấy thông điệp của cửa sổ do luồng này tạo.
        let got = unsafe { PeekMessageW(&mut m, Some(w.0), WM_APP + 21, WM_APP + 21, PM_REMOVE) };
        assert!(got.as_bool());
        assert_eq!(m.wParam, WPARAM(9));
        // SAFETY: thông điệp vừa nhận do post_box gửi kiểu này, lấy đúng một lần.
        let b = unsafe { take_scan_result(m.lParam) };
        assert_eq!(*b, rep);
        drop(w);
        let e: Result<ScanReport, ScanError> = Err(ScanError::Timeout);
        assert!(!post_box(raw, WM_APP + 21, 9, e));
    }

    #[test]
    fn start_scan_invalid_image_posts_failed() {
        let w = MsgWindow::new();
        let msg = WM_APP + 22;
        let img = Arc::new(Image {
            width: 3,
            height: 3,
            bgra: vec![0; 5],
        });
        start_scan_with_progress(img, w.0, msg, None, 5).expect("start_scan");
        let b = wait_scan(&w, msg, None, Duration::from_secs(10)).0;
        assert_eq!(*b.expect("no result"), Err(ScanError::Failed));
    }

    type ScanBox = Box<Result<ScanReport, ScanError>>;

    /// Chờ kết quả quét (và gom thông báo tiến độ nếu có) trên cửa sổ thông điệp `w`.
    fn wait_scan(
        w: &MsgWindow,
        msg: u32,
        progress: Option<u32>,
        limit: Duration,
    ) -> (Option<ScanBox>, Vec<(u32, u32)>) {
        let mut m = MSG::default();
        let deadline = Instant::now() + limit;
        let mut steps = Vec::new();
        while Instant::now() < deadline {
            if let Some(pm) = progress {
                // SAFETY: thông điệp tiến độ của cửa sổ do luồng này tạo (không có Box).
                while unsafe { PeekMessageW(&mut m, Some(w.0), pm, pm, PM_REMOVE) }.as_bool() {
                    let v = m.lParam.0 as u32;
                    steps.push((v >> 16, v & 0xFFFF));
                }
            }
            // SAFETY: lấy thông điệp của cửa sổ do luồng này tạo.
            if unsafe { PeekMessageW(&mut m, Some(w.0), msg, msg, PM_REMOVE) }.as_bool() {
                // SAFETY: thông điệp do `start_scan_with_progress` gửi, lấy đúng một lần.
                let b = unsafe { take_scan_result(m.lParam) };
                if let Some(pm) = progress {
                    // SAFETY: như trên.
                    while unsafe { PeekMessageW(&mut m, Some(w.0), pm, pm, PM_REMOVE) }.as_bool() {
                        let v = m.lParam.0 as u32;
                        steps.push((v >> 16, v & 0xFFFF));
                    }
                }
                return (Some(b), steps);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        (None, steps)
    }

    /// `MaxImageDimension` và thẻ ngôn ngữ engine quét chọn, đọc trên luồng MTA riêng.
    fn engine_info() -> Option<(u32, String)> {
        std::thread::spawn(|| {
            let _m = Mta::enter().ok()?;
            let max = OcrEngine::MaxImageDimension().ok()?;
            let e = scan_engine().ok()?;
            let tag = e
                .RecognizerLanguage()
                .ok()?
                .LanguageTag()
                .ok()?
                .to_string_lossy();
            Some((max, tag))
        })
        .join()
        .unwrap()
    }

    fn in_image(r: Rect, w: u32, h: u32) -> bool {
        r.w > 0 && r.h > 0 && r.x >= 0 && r.y >= 0 && r.x + r.w <= w as i32 && r.y + r.h <= h as i32
    }

    /// Tâm dọc của khung đã vẽ `d` nằm trong khung `r` (nới `tol`).
    fn covers_row(r: Rect, d: &RECT, tol: i32) -> bool {
        let cy = (d.top + d.bottom) / 2;
        cy >= r.y - tol && cy <= r.y + r.h + tol
    }

    fn run_scan(img: Image, limit: Duration) -> (Result<ScanReport, ScanError>, Vec<(u32, u32)>) {
        let w = MsgWindow::new();
        let (msg, pm) = (WM_APP + 23, WM_APP + 24);
        start_scan_with_progress(Arc::new(img), w.0, msg, Some(pm), 3).expect("start_scan");
        let (b, steps) = wait_scan(&w, msg, Some(pm), limit);
        (*b.expect("no scan result in time"), steps)
    }

    // Chạy tay (máy có OCR), bản release:
    // cargo test --release -p darkshot-windows ocr::tests::manual_scan -- --ignored --nocapture --test-threads=1
    #[test]
    #[ignore]
    fn manual_scan_word_boxes_inside_line_boxes() {
        let (img, _) = {
            let _gdi = crate::capture::GDI_TEST_LOCK.lock();
            lines_image(
                900,
                200,
                32,
                &[
                    ("Alpha bravo charlie delta", 60),
                    ("echo foxtrot 12345", 140),
                ],
            )
        };
        match recognize(&img) {
            Ok(r) => {
                assert_eq!(r.lines_detail.len(), 2, "{:?}", r.lines_detail);
                for l in &r.lines_detail {
                    assert!(l.words.len() >= 3, "{l:?}");
                    for wd in &l.words {
                        assert!(!wd.text.is_empty());
                        assert!(in_image(wd.rect, img.width, img.height), "{wd:?}");
                        let (lr, wr) = (l.rect, wd.rect);
                        assert!(
                            wr.x >= lr.x
                                && wr.y >= lr.y
                                && wr.x + wr.w <= lr.x + lr.w
                                && wr.y + wr.h <= lr.y + lr.h,
                            "{wd:?} not in {l:?}"
                        );
                    }
                }
            }
            Err(OcrError::NoLanguage) => println!("NoLanguage: may khong co goi OCR"),
            Err(e) => panic!("{e:?}"),
        }
    }

    #[test]
    #[ignore]
    fn manual_scan_finds_email_and_key() {
        let Some((max, tag)) = engine_info() else {
            println!("NoLanguage: may khong co goi OCR");
            return;
        };
        println!("MaxImageDimension={max} scan language={tag}");
        // Không có chữ "I": phông Segoe UI làm OCR đọc "I" hoa thành "l" thường (ghi trong báo cáo).
        let key = concat!("AKIA", "TESTKEYZ", "7EXAMPLE");
        let key_line = format!("aws id {key}");
        let wanted = [("Contact: user@example.com", 80), (key_line.as_str(), 200)];
        let (img, drawn) = {
            let _gdi = crate::capture::GDI_TEST_LOCK.lock();
            lines_image(1200, 300, 32, &wanted)
        };
        let (w, h) = (img.width, img.height);
        let (res, steps) = run_scan(img, Duration::from_secs(60));
        let rep = res.expect("scan");
        println!(
            "tiles={} elapsed_ms={} findings={}",
            rep.tiles,
            rep.elapsed_ms,
            rep.findings.len()
        );
        assert_eq!(rep.tiles, 1);
        assert_eq!(steps, vec![(1, 1)]);
        let dbg = format!("{rep:?}");
        assert!(
            !dbg.contains("example") && !dbg.contains("AKIA"),
            "text leaked"
        );
        let email: Vec<_> = rep
            .findings
            .iter()
            .filter(|f| f.category == Category::Email)
            .collect();
        let api: Vec<_> = rep
            .findings
            .iter()
            .filter(|f| f.category == Category::ApiKey)
            .collect();
        assert_eq!(email.len(), 1, "{:?}", rep.findings);
        assert_eq!(api.len(), 1, "{:?}", rep.findings);
        for f in &rep.findings {
            for &r in &f.rects {
                assert!(in_image(r, w, h), "{r:?}");
            }
        }
        assert!(email[0].rects.iter().all(|&r| covers_row(r, &drawn[0], 4)));
        assert!(api[0].rects.iter().all(|&r| covers_row(r, &drawn[1], 4)));
        // Khung email không phủ chữ "Contact:" bên trái (khung từ, không khung dòng).
        assert!(
            email[0].rects.iter().all(|r| r.x > drawn[0].left + 40),
            "{:?}",
            email[0]
        );
    }

    #[test]
    #[ignore]
    fn manual_scan_tall_image_every_tile() {
        let Some((max, tag)) = engine_info() else {
            println!("NoLanguage: may khong co goi OCR");
            return;
        };
        let h = 3 * max;
        let tiles = scan_tiles(h, max, SCAN_TILE_OVERLAP);
        println!("MaxImageDimension={max} lang={tag} h={h} tiles={tiles:?}");
        assert!(tiles.len() >= 3);
        // Một email gần đầu mỗi dải, cộng một email nằm trọn trong vùng chồng dải 1/2.
        let texts = [
            "alice@example.com",
            "bobby@example.org",
            "carol@example.net",
            "david@example.com",
            "erika@example.org",
            "frank@example.net",
        ];
        let mut rows: Vec<i32> = tiles.iter().map(|t| (t.y + 300) as i32).collect();
        rows.push((tiles[1].y + SCAN_TILE_OVERLAP / 2) as i32);
        rows.sort_unstable();
        rows.dedup_by(|a, b| (*a - *b).abs() < 150);
        let lines: Vec<(&str, i32)> = texts.iter().copied().zip(rows.iter().copied()).collect();
        assert_eq!(lines.len(), rows.len());
        let (img, drawn) = {
            let _gdi = crate::capture::GDI_TEST_LOCK.lock();
            lines_image(1000, h as i32, 32, &lines)
        };
        let (iw, ih) = (img.width, img.height);
        let (res, steps) = run_scan(img, Duration::from_secs(300));
        let rep = res.expect("scan");
        println!(
            "tiles={} elapsed_ms={} findings={}",
            rep.tiles,
            rep.elapsed_ms,
            rep.findings.len()
        );
        assert_eq!(rep.tiles as usize, tiles.len());
        let n = tiles.len() as u32;
        assert_eq!(steps, (1..=n).map(|k| (k, n)).collect::<Vec<_>>());
        let email: Vec<_> = rep
            .findings
            .iter()
            .filter(|f| f.category == Category::Email)
            .collect();
        assert_eq!(email.len(), lines.len(), "{:?}", rep.findings);
        for d in &drawn {
            let hits = email
                .iter()
                .filter(|f| f.rects.iter().any(|&r| covers_row(r, d, 4)))
                .count();
            assert_eq!(hits, 1, "line at {d:?}: {:?}", rep.findings);
        }
        for f in &rep.findings {
            for &r in &f.rects {
                assert!(in_image(r, iw, ih), "{r:?}");
            }
        }
    }

    #[test]
    #[ignore]
    fn manual_scan_timing() {
        let Some((max, tag)) = engine_info() else {
            println!("NoLanguage: may khong co goi OCR");
            return;
        };
        println!("MaxImageDimension={max} scan language={tag}");
        let fhd: Vec<(String, i32)> = (0..12)
            .map(|i| {
                (
                    format!("Row {i} contact user{i}@example.com phone 0912 345 67{i}"),
                    60 + i * 85,
                )
            })
            .collect();
        let refs: Vec<(&str, i32)> = fhd.iter().map(|(s, y)| (s.as_str(), *y)).collect();
        let (img, _) = {
            let _gdi = crate::capture::GDI_TEST_LOCK.lock();
            lines_image(1920, 1080, 24, &refs)
        };
        let (res, _) = run_scan(img, Duration::from_secs(120));
        let rep = res.expect("scan 1920x1080");
        println!(
            "1920x1080: tiles={} elapsed_ms={} findings={}",
            rep.tiles,
            rep.elapsed_ms,
            rep.findings.len()
        );
        // Cùng ảnh 1920×1080 kẻ bảng (Task 137): đường ngang 1 px sát chân chữ mỗi dòng, đường dọc
        // mỗi 480 px — có lượt thứ hai.
        let (mut img, drawn) = {
            let _gdi = crate::capture::GDI_TEST_LOCK.lock();
            lines_image(1920, 1080, 24, &refs)
        };
        let row = 1920usize * 4;
        for d in &drawn {
            let y = (d.bottom - 4) as usize;
            img.bgra[y * row..(y + 1) * row]
                .as_chunks_mut::<4>()
                .0
                .iter_mut()
                .for_each(|p| p[..3].fill(0));
        }
        for x in (0..1920usize).step_by(480) {
            for y in 0..1080usize {
                img.bgra[y * row + x * 4..y * row + x * 4 + 3].fill(0);
            }
        }
        assert!(privacy::rules::strip_rules(&img.bgra, 1920, 1080).is_some());
        let (res, _) = run_scan(img, Duration::from_secs(120));
        let rep = res.expect("scan 1920x1080 table");
        println!(
            "1920x1080 table: tiles={} elapsed_ms={} findings={}",
            rep.tiles,
            rep.elapsed_ms,
            rep.findings.len()
        );
        let uhd: Vec<(String, i32)> = (0..24)
            .map(|i| {
                (
                    format!(
                        "Row {i} contact user{i}@example.com phone 0912 345 67{}",
                        i % 10
                    ),
                    60 + i * 85,
                )
            })
            .collect();
        let refs: Vec<(&str, i32)> = uhd.iter().map(|(s, y)| (s.as_str(), *y)).collect();
        let (img, _) = {
            let _gdi = crate::capture::GDI_TEST_LOCK.lock();
            lines_image(3840, 2160, 24, &refs)
        };
        let (res, _) = run_scan(img, Duration::from_secs(120));
        let rep = res.expect("scan 3840x2160");
        println!(
            "3840x2160: tiles={} elapsed_ms={} findings={}",
            rep.tiles,
            rep.elapsed_ms,
            rep.findings.len()
        );
        let tall: Vec<(String, i32)> = (0..10)
            .map(|i| {
                (
                    format!("Item {i} mail user{i}@example.com"),
                    1_000 + i * 2_000,
                )
            })
            .collect();
        let refs: Vec<(&str, i32)> = tall.iter().map(|(s, y)| (s.as_str(), *y)).collect();
        let (img, _) = {
            let _gdi = crate::capture::GDI_TEST_LOCK.lock();
            lines_image(1000, 20_000, 24, &refs)
        };
        let (res, _) = run_scan(img, Duration::from_secs(300));
        let rep = res.expect("scan 1000x20000");
        println!(
            "1000x20000: tiles={} elapsed_ms={} findings={}",
            rep.tiles,
            rep.elapsed_ms,
            rep.findings.len()
        );
    }

    // ---- Task 133: đo đạc (chạy tay, bản release, chỉ in số) ----
    // cargo test --release -p darkshot-windows ocr::tests::manual_perf -- --ignored --nocapture --test-threads=1

    /// OCR (Segoe UI) đọc `I` hoa thành `l`: khóa AWS có `I` vẫn được nhận (tinh chỉnh Task 133).
    #[test]
    #[ignore]
    fn manual_scan_aws_key_with_capital_i() {
        if engine_info().is_none() {
            println!("NoLanguage: may khong co goi OCR");
            return;
        }
        let key = concat!("AKIA", "IOSFODNN7", "EXAMPLE");
        let line = format!("aws id {key}");
        let (img, _) = {
            let _gdi = crate::capture::GDI_TEST_LOCK.lock();
            lines_image(1200, 160, 32, &[(line.as_str(), 80)])
        };
        let (res, _) = run_scan(img, Duration::from_secs(60));
        let rep = res.expect("scan");
        let api = rep
            .findings
            .iter()
            .filter(|f| f.category == Category::ApiKey)
            .count();
        println!("api_key findings={api} total={}", rep.findings.len());
        assert_eq!(api, 1, "{:?}", rep.findings);
    }

    /// Dòng OCR tổng hợp (khung từ 10 px/ký tự), cho đo `detect` không cần OCR.
    fn synthetic_line(text: &str, row: i32) -> OcrLine {
        let mut x = 0;
        let mut words = Vec::new();
        for w in text.split(' ').filter(|w| !w.is_empty()) {
            let width = 10 * w.chars().count() as i32;
            words.push(darkshot_core::ocr_text::OcrWord {
                text: w.to_string(),
                rect: Rect {
                    x,
                    y: row * 24,
                    w: width,
                    h: 20,
                },
            });
            x += width + 10;
        }
        OcrLine {
            text: text.to_string(),
            rect: Rect {
                x: 0,
                y: row * 24,
                w: x.max(1),
                h: 20,
            },
            words,
        }
    }

    fn median_ms(mut v: Vec<f64>) -> f64 {
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        v[v.len() / 2]
    }

    #[test]
    #[ignore]
    fn manual_perf_detect_and_flatten() {
        use darkshot_core::privacy::redact::flatten;
        // 1000 dòng: một nửa có dữ liệu giả nhạy cảm (4 loại), còn lại chữ thường.
        let key = format!("{}{}", "AK", "IAEXAMPLEKEY012345");
        let lines: Vec<OcrLine> = (0..1000)
            .map(|i| {
                let t = match i % 8 {
                    0 => format!("Row {i} contact user{i}@example.com now"),
                    1 => format!("Row {i} phone 0912 345 {:03} call", i % 1000),
                    2 => format!("Row {i} aws id {key} done"),
                    3 => format!("Row {i} password: hunter{i}"),
                    _ => format!("Row {i} the quick brown fox jumps over the lazy dog 2026-11-22"),
                };
                synthetic_line(&t, i)
            })
            .collect();
        let mut runs = Vec::new();
        let mut n = 0;
        for _ in 0..7 {
            let t = Instant::now();
            n = darkshot_core::privacy::detect(&lines).len();
            runs.push(t.elapsed().as_secs_f64() * 1000.0);
        }
        println!(
            "detect 1000 lines: median {:.2} ms (runs {:?}), findings={n}",
            median_ms(runs.clone()),
            runs.iter()
                .map(|x| (x * 100.0).round() / 100.0)
                .collect::<Vec<_>>()
        );

        let rects = |w: i32, h: i32, k: i32| -> Vec<Rect> {
            (0..k)
                .map(|i| Rect {
                    x: (i * 97) % (w - 300).max(1),
                    y: (i * 53) % (h - 40).max(1),
                    w: 280,
                    h: 24,
                })
                .collect()
        };
        for (w, h, k) in [(3840u32, 2160u32, 40), (8000, 20000, 400)] {
            let img = Image {
                width: w,
                height: h,
                bgra: vec![200u8; w as usize * h as usize * 4],
            };
            let rs = rects(w as i32, h as i32, k);
            let mut runs = Vec::new();
            for _ in 0..3 {
                let t = Instant::now();
                let out = flatten(&img, &rs).unwrap();
                runs.push(t.elapsed().as_secs_f64() * 1000.0);
                drop(out);
            }
            println!(
                "flatten {w}x{h} rects={k}: median {:.1} ms (runs {:?})",
                median_ms(runs.clone()),
                runs.iter().map(|x| x.round()).collect::<Vec<_>>()
            );
        }
    }

    /// Bộ nhớ đỉnh: chạy riêng một kiểm thử mỗi tiến trình, đo PeakWorkingSet từ bên ngoài.
    #[test]
    #[ignore]
    fn manual_mem_baseline() {
        std::thread::sleep(Duration::from_millis(300));
    }

    #[test]
    #[ignore]
    fn manual_mem_scan_1000x20000() {
        if engine_info().is_none() {
            println!("NoLanguage: may khong co goi OCR");
            return;
        }
        let tall: Vec<(String, i32)> = (0..10)
            .map(|i| {
                (
                    format!("Item {i} mail user{i}@example.com"),
                    1_000 + i * 2_000,
                )
            })
            .collect();
        let refs: Vec<(&str, i32)> = tall.iter().map(|(s, y)| (s.as_str(), *y)).collect();
        let (img, _) = {
            let _gdi = crate::capture::GDI_TEST_LOCK.lock();
            lines_image(1000, 20_000, 24, &refs)
        };
        // Nghỉ trước/sau lượt quét để bộ đo bên ngoài thấy mức nền (ảnh 80 MB đã dựng xong).
        std::thread::sleep(Duration::from_millis(600));
        let t = Instant::now();
        let (res, _) = run_scan(img, Duration::from_secs(300));
        let rep = res.expect("scan");
        std::thread::sleep(Duration::from_millis(600));
        println!(
            "mem scan 1000x20000: tiles={} findings={} wall_ms={}",
            rep.tiles,
            rep.findings.len(),
            t.elapsed().as_millis()
        );
    }

    /// Xuất 10 bước 4K như `steps_window` (Task 135): mỗi ảnh đã qua cổng được mã hóa PNG ngay và
    /// chỉ byte PNG được giữ trong `ExportJob.replacements`. `noise_rows`: cứ mỗi `noise_rows` hàng
    /// có một hàng nhiễu (0 = ảnh phẳng như số đo cũ), để ảnh nén còn vài MB như ảnh chụp thật.
    fn mem_steps_export_10x4k(noise_rows: u32) {
        use darkshot_core::steps::Step;
        let dir = std::env::temp_dir().join(format!("darkshot-mem-export-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (w, h) = (3840u32, 2160u32);
        let mut seed = 0x1234_5678u32;
        let t = Instant::now();
        let replacements: Vec<(usize, Vec<u8>)> = (0..10)
            .map(|i| {
                let mut bgra = vec![(i * 20) as u8; w as usize * h as usize * 4];
                if noise_rows > 0 {
                    for y in (0..h).step_by(noise_rows as usize) {
                        for x in 0..w {
                            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                            let k = ((y * w + x) * 4) as usize;
                            bgra[k..k + 3].copy_from_slice(&seed.to_le_bytes()[..3]);
                        }
                    }
                }
                for px in bgra.chunks_mut(4) {
                    px[3] = 255;
                }
                let img = Image {
                    width: w,
                    height: h,
                    bgra,
                };
                (i, darkshot_core::image::encode_png(&img).expect("png"))
            })
            .collect();
        let png: usize = replacements.iter().map(|(_, b)| b.len()).sum();
        println!(
            "mem export 10x4K noise_rows={noise_rows}: png_total={:.1} MB encode_ms={}",
            png as f64 / 1_048_576.0,
            t.elapsed().as_millis()
        );
        let job = crate::steps_export::ExportJob {
            parent: dir.clone(),
            when: darkshot_core::naming::LocalTime {
                year: 2026,
                month: 11,
                day: 22,
                hour: 10,
                minute: 0,
                second: 0,
            },
            lang: darkshot_core::i18n::Lang::En,
            steps: (0..10)
                .map(|i| Step {
                    file: format!("{:03}.png", i + 1),
                    caption: String::new(),
                    scanned: false,
                })
                .collect(),
            sources: (0..10)
                .map(|i| dir.join(format!("missing-{i}.png")))
                .collect(),
            replacements,
        };
        let t = Instant::now();
        let out = crate::steps_export::export(&job).expect("export");
        println!("mem export 10x4K: write_ms={}", t.elapsed().as_millis());
        drop(job);
        let _ = std::fs::remove_dir_all(&out);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    #[ignore]
    fn manual_mem_steps_export_10x4k() {
        mem_steps_export_10x4k(0);
    }

    #[test]
    #[ignore]
    fn manual_mem_steps_export_10x4k_noisy() {
        mem_steps_export_10x4k(10);
    }

    // ---- Task 137: bảng có đường kẻ sát chân chữ (chạy tay, bản release, chỉ in số) ----
    // cargo test --release -p darkshot-windows --bin darkshot ocr::tests::manual_table -- --ignored --nocapture --test-threads=1

    /// Bảng tổng hợp kiểu Excel (dữ liệu GIẢ) và vị trí để chấm điểm.
    struct Table {
        img: Image,
        /// Tâm y của từng hàng dữ liệu.
        centers: Vec<i32>,
        /// Cột email `[x0, x1)`.
        email_x: (i32, i32),
    }

    /// `rows` × 3 cột, chữ `font_px` phông `face` vẽ bằng GDI, đường kẻ ô 1 px độ sáng `rule` nằm
    /// `drop` px dưới đường chân chữ (chân g/y/p/q cắt qua đường kẻ). `dark`: nền tối chữ sáng.
    fn table_image(
        rows: &[[String; 3]],
        font_px: i32,
        face: &str,
        rule: u8,
        drop: i32,
        dark: bool,
    ) -> Table {
        let cols = [0i32, 360, 900, 1300];
        let w = cols[3] + 1;
        let (bg, fg) = if dark { (32u8, 230u32) } else { (255u8, 0u32) };
        let mut lf = LOGFONTW {
            lfHeight: -font_px,
            lfWeight: 400,
            ..Default::default()
        };
        for (i, c) in face.encode_utf16().enumerate() {
            lf.lfFaceName[i] = c;
        }
        let mut tm = TEXTMETRICW::default();
        // SAFETY: DC và phông tạm (đo đường chân chữ), giải phóng ngay trong khối.
        unsafe {
            let dc = CreateCompatibleDC(None);
            let font = CreateFontIndirectW(&lf);
            let old = SelectObject(dc, font.into());
            let _ = GetTextMetricsW(dc, &mut tm);
            SelectObject(dc, old);
            let _ = DeleteObject(font.into());
            let _ = DeleteDC(dc);
        }
        let pad = 4;
        let row_h = pad + tm.tmAscent + drop;
        let h = row_h * (rows.len() as i32 + 1) + 1;
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
        let mut out = vec![bg; len];
        let mut bits: *mut c_void = std::ptr::null_mut();
        let mut centers = Vec::new();
        // SAFETY: DC, DIB và phông chỉ dùng trong khối này và đều được giải phóng ở cuối.
        unsafe {
            let dc = CreateCompatibleDC(None);
            assert!(!dc.is_invalid());
            let bmp =
                CreateDIBSection(Some(dc), &bmi, DIB_RGB_COLORS, &mut bits, None, 0).expect("dib");
            assert!(!bits.is_null());
            std::ptr::write_bytes(bits.cast::<u8>(), bg, len);
            let old = SelectObject(dc, bmp.into());
            let font = CreateFontIndirectW(&lf);
            let old_font = SelectObject(dc, font.into());
            SetBkMode(dc, TRANSPARENT);
            SetTextColor(
                dc,
                windows::Win32::Foundation::COLORREF(fg | (fg << 8) | (fg << 16)),
            );
            let header = ["Name", "Email", "Phone"].map(String::from);
            for (r, cells) in std::iter::once(&header).chain(rows.iter()).enumerate() {
                // Hàng r: đường kẻ trên ở y = row_h·r, chữ bắt đầu ở y + 1 + pad.
                let top = row_h * r as i32 + 1;
                if r > 0 {
                    centers.push(top + pad + tm.tmAscent / 2);
                }
                for (c, t) in cells.iter().enumerate() {
                    let mut s: Vec<u16> = t.encode_utf16().collect();
                    let mut rc = RECT {
                        left: cols[c] + 6,
                        top: top + pad - 1,
                        right: cols[c + 1] - 2,
                        bottom: top + row_h + tm.tmDescent,
                    };
                    DrawTextW(
                        dc,
                        &mut s,
                        &mut rc,
                        DT_LEFT | DT_TOP | DT_SINGLELINE | DT_NOPREFIX,
                    );
                }
            }
            let _ = GdiFlush();
            std::ptr::copy_nonoverlapping(bits.cast::<u8>(), out.as_mut_ptr(), len);
            SelectObject(dc, old_font);
            let _ = DeleteObject(font.into());
            SelectObject(dc, old);
            let _ = DeleteObject(bmp.into());
            let _ = DeleteDC(dc);
        }
        let (wu, hu) = (w as usize, h as usize);
        // Đường kẻ vẽ sau chữ, đè lên chân chữ như lưới ô của bảng tính.
        let mut put = |x: usize, y: usize| {
            let i = (y * wu + x) * 4;
            out[i..i + 3].fill(rule);
        };
        for r in 0..=rows.len() + 1 {
            let y = (row_h as usize * r).min(hu - 1);
            for x in 0..wu {
                put(x, y);
            }
        }
        for &x in &cols {
            for y in 0..hu {
                put(x as usize, y);
            }
        }
        for p in out.chunks_mut(4) {
            p[3] = 255;
        }
        Table {
            img: Image {
                width: w as u32,
                height: h as u32,
                bgra: out,
            },
            centers,
            email_x: (cols[1], cols[2]),
        }
    }

    /// 14 hàng GIẢ: tên có dấu, email `@example.com` nhiều chữ g/y/p/q, số điện thoại giả.
    fn table_rows() -> Vec<[String; 3]> {
        let names = [
            "gia.huy",
            "quynh.yen",
            "phuong.ngoc",
            "yogi.pham",
            "giang.quy",
            "thuy.duong",
            "hung.nguyen",
            "ngoc.yen",
            "quang.huy",
            "phuc.giang",
            "my.pham",
            "yen.nhi",
            "duy.quyen",
            "gia.phong",
        ];
        names
            .iter()
            .enumerate()
            .map(|(i, n)| {
                [
                    format!("Nguy\u{1EC5}n V\u{0103}n {}", (b'A' + i as u8) as char),
                    format!("{}{}@example.com", n.replace('.', ""), 10 + i),
                    format!("0912 345 {:03}", 100 + i),
                ]
            })
            .collect()
    }

    /// Quét bằng đường thật (`scan_tile`, như `scan_in_mta`), một hay hai lượt, trên luồng MTA riêng.
    fn scan_passes(img: &Image, two_pass: bool) -> Result<(Vec<Finding>, u32), ScanError> {
        std::thread::scope(|sc| {
            sc.spawn(|| {
                let t0 = Instant::now();
                let _m = Mta::enter().map_err(|_| ScanError::Failed)?;
                let engine = scan_engine()?;
                let max = OcrEngine::MaxImageDimension().map_err(|_| ScanError::Failed)?;
                let mut findings = Vec::new();
                for t in scan_tiles(img.height, max, SCAN_TILE_OVERLAP) {
                    let found = scan_tile(&engine, img, t, max, two_pass)?;
                    privacy::merge_tile(&mut findings, found, t.y as i32);
                }
                Ok((findings, t0.elapsed().as_millis() as u32))
            })
            .join()
            .unwrap()
        })
    }

    /// (số hàng có một phát hiện Email phủ tâm ô email của hàng, số phát hiện Email).
    fn email_rows(t: &Table, f: &[Finding]) -> (usize, usize) {
        let email: Vec<&Finding> = f.iter().filter(|f| f.category == Category::Email).collect();
        let rows = t
            .centers
            .iter()
            .filter(|&&cy| {
                email.iter().any(|f| {
                    f.rects.iter().any(|r| {
                        cy >= r.y && cy < r.y + r.h && r.x >= t.email_x.0 && r.x < t.email_x.1
                    })
                })
            })
            .count();
        (rows, email.len())
    }

    #[test]
    #[ignore]
    fn manual_table_rules_two_pass() {
        let Some((max, tag)) = engine_info() else {
            println!("NoLanguage: may khong co goi OCR");
            return;
        };
        println!("MaxImageDimension={max} scan language={tag}");
        let rows = table_rows();
        let cases: [(&str, i32, u8, i32, bool); 7] = [
            ("Calibri", 20, 0, 1, false),
            ("Calibri", 20, 0, 2, false),
            ("Calibri", 20, 0, 3, false),
            ("Calibri", 20, 160, 2, false),
            ("Calibri", 20, 212, 2, false),
            ("Segoe UI", 20, 0, 2, false),
            ("Calibri", 20, 200, 2, true),
        ];
        for (face, px, rule, drop, dark) in cases {
            let t = {
                let _gdi = crate::capture::GDI_TEST_LOCK.lock();
                table_image(&rows, px, face, rule, drop, dark)
            };
            let (f1, ms1) = scan_passes(&t.img, false).expect("scan one pass");
            let (f2, ms2) = scan_passes(&t.img, true).expect("scan two passes");
            let (r1, n1) = email_rows(&t, &f1);
            let (r2, n2) = email_rows(&t, &f2);
            let phones1 = f1.iter().filter(|f| f.category == Category::Phone).count();
            let phones2 = f2.iter().filter(|f| f.category == Category::Phone).count();
            let strip = Instant::now();
            let changed =
                privacy::rules::strip_rules(&t.img.bgra, t.img.width, t.img.height).is_some();
            let strip_ms = strip.elapsed().as_secs_f64() * 1000.0;
            println!(
                "{face} {px}px rule={rule} drop={drop} dark={dark} {}x{}: one pass email rows \
                 {r1}/14 (email findings {n1}, phone {phones1}, {ms1} ms); two passes {r2}/14 \
                 (email findings {n2}, phone {phones2}, {ms2} ms); strip={changed} {strip_ms:.1} ms",
                t.img.width, t.img.height
            );
            assert!(r2 >= r1, "two passes must not lose rows");
            assert!(phones2 >= phones1, "two passes must not lose phones");
        }
    }

    #[test]
    #[ignore]
    fn manual_perf_strip_rules_4k() {
        // 4K: lưới 1 px (mỗi 30 hàng, mỗi 200 cột) cộng các khối "chữ" 6×12.
        let (w, h) = (3840usize, 2160usize);
        let mut px = vec![255u8; w * h * 4];
        for y in 0..h {
            for x in 0..w {
                let line = y % 30 == 29 || x % 200 == 0;
                let glyph = (12..24).contains(&(y % 30)) && x % 12 < 6 && x % 200 > 10;
                if line || glyph {
                    let i = (y * w + x) * 4;
                    px[i..i + 3].fill(if line { 90 } else { 0 });
                }
            }
        }
        let blank = vec![255u8; w * h * 4];
        for (name, data) in [("table", &px), ("blank", &blank)] {
            let mut runs = Vec::new();
            for _ in 0..5 {
                let t = Instant::now();
                let out = privacy::rules::strip_rules(data, w as u32, h as u32);
                runs.push(t.elapsed().as_secs_f64() * 1000.0);
                assert_eq!(out.is_some(), name == "table");
            }
            println!(
                "strip_rules 3840x2160 {name}: median {:.1} ms (runs {:?})",
                median_ms(runs.clone()),
                runs.iter().map(|x| x.round()).collect::<Vec<_>>()
            );
        }
    }
}
