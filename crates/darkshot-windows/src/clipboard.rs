//! Clipboard của Windows: chép link (văn bản) và ảnh.

use crate::lang;
use darkshot_core::actions::Clipboard;
use darkshot_core::i18n::Msg;
use darkshot_core::image::Image;
use std::time::Duration;
use windows::core::{w, HSTRING};
use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL, HWND};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, RegisterClipboardFormatW,
    SetClipboardData,
};
use windows::Win32::System::Memory::{
    GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, HWND_MESSAGE, WINDOW_EX_STYLE, WINDOW_STYLE,
};

/// Định dạng chuẩn (winuser.h): CF_DIB = 8, CF_UNICODETEXT = 13.
const CF_DIB: u32 = 8;
const CF_UNICODETEXT: u32 = 13;
/// Định dạng đăng ký mà Windows dùng để loại nội dung khỏi lịch sử clipboard và đồng bộ đám mây,
/// kèm giá trị DWORD đặt vào.
/// Nguồn: learn.microsoft.com/windows/win32/dataxchg/clipboard-formats, mục
/// "Cloud Clipboard and Clipboard History Formats" (lấy mã bằng `RegisterClipboardFormatW`).
pub(crate) const EXCLUSION_FORMATS: &[(&str, u32)] = &[
    // "Place any data ... to prevent all clipboard formats being included in the clipboard
    // history or synchronized": dữ liệu tùy ý, ta đặt DWORD 0.
    ("ExcludeClipboardContentFromMonitorProcessing", 0),
    // DWORD 0: không đưa vào lịch sử clipboard (Win+V).
    ("CanIncludeInClipboardHistory", 0),
    // DWORD 0: không đồng bộ sang thiết bị khác.
    ("CanUploadToCloudClipboard", 0),
];
/// `OpenClipboard` thất bại khi ứng dụng khác đang giữ clipboard: thử tối đa 10 lần, cách 30 ms.
const OPEN_ATTEMPTS: u32 = 10;
const OPEN_RETRY_DELAY: Duration = Duration::from_millis(30);
const BITMAPINFOHEADER_SIZE: u32 = 40;

/// Clipboard thật của hệ thống. Dùng được từ bất kỳ luồng nào: mỗi lần chép tạo một cửa sổ
/// chỉ-thông-điệp riêng ở luồng gọi để làm chủ clipboard, rồi hủy nó.
#[derive(Debug, Clone, Copy, Default)]
pub struct WinClipboard;

impl Clipboard for WinClipboard {
    fn set_text(&self, s: &str) -> Result<(), String> {
        let bytes: Vec<u8> = s
            .encode_utf16()
            .chain(std::iter::once(0))
            .flat_map(u16::to_le_bytes)
            .collect();
        put(CF_UNICODETEXT, &bytes)
    }

    fn set_image(&self, img: &Image) -> Result<(), String> {
        put(CF_DIB, &dib_from_image(img)?)
    }
}

/// Dựng khối CF_DIB 24 bit (BITMAPINFOHEADER + pixel từ dưới lên, mỗi hàng đệm tới bội của 4).
pub(crate) fn dib_from_image(img: &Image) -> Result<Vec<u8>, String> {
    if !img.is_valid() {
        return Err(lang::tr(Msg::ClipInvalidImage).into());
    }
    let too_big = || lang::tr(Msg::ClipImageTooBig).to_string();
    let w = img.width as usize;
    let width = i32::try_from(img.width).map_err(|_| too_big())?;
    let height = i32::try_from(img.height).map_err(|_| too_big())?;
    let stride = w
        .checked_mul(3)
        .and_then(|n| n.checked_add(3))
        .map(|n| n & !3)
        .ok_or_else(too_big)?;
    let image_size = stride
        .checked_mul(img.height as usize)
        .ok_or_else(too_big)?;
    let image_size_u32 = u32::try_from(image_size).map_err(|_| too_big())?;
    let total = image_size
        .checked_add(BITMAPINFOHEADER_SIZE as usize)
        .ok_or_else(too_big)?;

    let mut out = Vec::new();
    out.try_reserve_exact(total).map_err(|_| too_big())?;
    out.extend_from_slice(&BITMAPINFOHEADER_SIZE.to_le_bytes()); // biSize
    out.extend_from_slice(&width.to_le_bytes()); // biWidth
    out.extend_from_slice(&height.to_le_bytes()); // biHeight > 0: từ dưới lên
    out.extend_from_slice(&1u16.to_le_bytes()); // biPlanes
    out.extend_from_slice(&24u16.to_le_bytes()); // biBitCount
    out.extend_from_slice(&0u32.to_le_bytes()); // biCompression = BI_RGB
    out.extend_from_slice(&image_size_u32.to_le_bytes()); // biSizeImage
    out.extend_from_slice(&0i32.to_le_bytes()); // biXPelsPerMeter
    out.extend_from_slice(&0i32.to_le_bytes()); // biYPelsPerMeter
    out.extend_from_slice(&0u32.to_le_bytes()); // biClrUsed
    out.extend_from_slice(&0u32.to_le_bytes()); // biClrImportant

    let pad = stride - w * 3;
    // `is_valid` bảo đảm bgra dài đúng w * h * 4, nên có đúng h hàng.
    for row in img.bgra.chunks_exact(w * 4).rev() {
        for [b, g, r, _a] in row.as_chunks::<4>().0 {
            out.extend([*b, *g, *r]);
        }
        out.extend(std::iter::repeat_n(0u8, pad));
    }
    Ok(out)
}

/// Cửa sổ chỉ-thông-điệp làm chủ clipboard; hủy khi drop (cùng luồng đã tạo).
struct OwnerWindow(HWND);

impl OwnerWindow {
    fn new() -> Result<Self, String> {
        // SAFETY: lớp "STATIC" có sẵn của hệ thống; cửa sổ con của HWND_MESSAGE không bao giờ
        // hiện, và được hủy trong Drop ở chính luồng này.
        unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                w!("darkshot-clipboard"),
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
        }
        .map(OwnerWindow)
        .map_err(|e| lang::tr_code(Msg::ClipWindowFailed, &format!("{:#010x}", e.code().0)))
    }
}

impl Drop for OwnerWindow {
    fn drop(&mut self) {
        // SAFETY: cửa sổ do luồng này tạo và chỉ hủy một lần.
        unsafe {
            let _ = DestroyWindow(self.0);
        }
    }
}

/// Clipboard đang mở; đóng khi drop.
struct OpenGuard;

impl OpenGuard {
    fn open(owner: &OwnerWindow) -> Result<Self, String> {
        for attempt in 0..OPEN_ATTEMPTS {
            if attempt > 0 {
                std::thread::sleep(OPEN_RETRY_DELAY);
            }
            // SAFETY: owner là cửa sổ hợp lệ của luồng này; mở thành công thì Drop sẽ đóng.
            if unsafe { OpenClipboard(Some(owner.0)) }.is_ok() {
                return Ok(OpenGuard);
            }
        }
        Err(lang::tr(Msg::ClipBusy).into())
    }
}

impl Drop for OpenGuard {
    fn drop(&mut self) {
        // SAFETY: chỉ tạo OpenGuard sau khi OpenClipboard thành công.
        unsafe {
            let _ = CloseClipboard();
        }
    }
}

/// Vùng nhớ GMEM_MOVEABLE; được GlobalFree khi drop nếu chưa giao cho hệ thống.
struct GlobalMem(HGLOBAL);

impl GlobalMem {
    fn from_bytes(data: &[u8]) -> Result<Self, String> {
        let err = || lang::tr(Msg::ClipAllocFailed).to_string();
        // SAFETY: cấp phát vùng nhớ mới; quyền sở hữu nằm trong GlobalMem.
        let mem =
            GlobalMem(unsafe { GlobalAlloc(GMEM_MOVEABLE, data.len().max(1)) }.map_err(|_| err())?);
        // SAFETY: khóa vùng nhớ vừa cấp (ít nhất data.len() byte), chép vào rồi mở khóa.
        unsafe {
            let p = GlobalLock(mem.0) as *mut u8;
            if p.is_null() {
                return Err(err());
            }
            std::ptr::copy_nonoverlapping(data.as_ptr(), p, data.len());
            // GlobalUnlock trả "lỗi" NO_ERROR khi số lần khóa về 0; không phải lỗi thật.
            let _ = GlobalUnlock(mem.0);
        }
        Ok(mem)
    }

    /// Giao vùng nhớ cho clipboard. Thành công: hệ thống sở hữu; thất bại: drop sẽ GlobalFree.
    fn into_clipboard(self, format: u32) -> Result<(), String> {
        // SAFETY: clipboard đang mở bởi luồng này; hMem cấp bằng GMEM_MOVEABLE và đã mở khóa.
        match unsafe { SetClipboardData(format, Some(HANDLE(self.0 .0))) } {
            Ok(_) => {
                std::mem::forget(self);
                Ok(())
            }
            Err(e) => Err(lang::tr_code(
                Msg::ClipSetFailed,
                &format!("{:#010x}", e.code().0),
            )),
        }
    }
}

impl Drop for GlobalMem {
    fn drop(&mut self) {
        // SAFETY: vùng nhớ vẫn thuộc ta (chưa giao cho clipboard) và chỉ giải phóng một lần.
        unsafe {
            let _ = GlobalFree(Some(self.0));
        }
    }
}

/// Mã và dữ liệu của các định dạng loại trừ; định dạng nào không đăng ký hoặc cấp phát được thì bỏ.
fn exclusion_data() -> Vec<(u32, GlobalMem)> {
    EXCLUSION_FORMATS
        .iter()
        .filter_map(|(name, value)| {
            let h = HSTRING::from(*name);
            // SAFETY: chuỗi kết thúc bằng 0, sống suốt lời gọi.
            let id = unsafe { RegisterClipboardFormatW(&h) };
            if id == 0 {
                return None;
            }
            GlobalMem::from_bytes(&value.to_le_bytes())
                .ok()
                .map(|m| (id, m))
        })
        .collect()
}

/// Thay nội dung clipboard bằng một định dạng chính, kèm các định dạng báo Windows không đưa
/// nội dung vào lịch sử clipboard hay đồng bộ đám mây.
fn put(format: u32, data: &[u8]) -> Result<(), String> {
    // Cấp phát trước khi mở để giữ clipboard ngắn nhất có thể.
    let mem = GlobalMem::from_bytes(data)?;
    let extras = exclusion_data();
    let owner = OwnerWindow::new()?;
    let result = {
        let _open = OpenGuard::open(&owner)?;
        // SAFETY: clipboard đang mở với chủ là `owner`.
        let r = unsafe { EmptyClipboard() }
            .map_err(|e| lang::tr_code(Msg::ClipEmptyFailed, &format!("{:#010x}", e.code().0)))
            .and_then(|()| mem.into_clipboard(format));
        if r.is_ok() {
            for (id, m) in extras {
                // Đặt định dạng phụ thất bại không làm hỏng việc chép chính; vùng nhớ tự giải phóng.
                let _ = m.into_clipboard(id);
            }
        }
        r
        // `_open` đóng clipboard ở đây, trước khi hủy cửa sổ chủ.
    };
    drop(owner);
    result
}

/// Số lần thử mở clipboard khi dán (ngắn hơn khi chép: người dùng đang chờ trong lớp phủ).
const PASTE_ATTEMPTS: u32 = 3;

/// Chữ thuần (CF_UNICODETEXT) đang có trên clipboard, đọc tới ký tự 0 đầu tiên. `owner` là cửa sổ
/// của luồng gọi. `None` khi clipboard bận, không có chữ hoặc dữ liệu hỏng.
pub fn paste_text(owner: HWND) -> Option<String> {
    let _open = (0..PASTE_ATTEMPTS).find_map(|attempt| {
        if attempt > 0 {
            std::thread::sleep(OPEN_RETRY_DELAY);
        }
        // SAFETY: owner là cửa sổ của luồng này; mở thành công thì OpenGuard đóng khi drop.
        if unsafe { OpenClipboard(Some(owner)) }.is_ok() {
            Some(OpenGuard)
        } else {
            None
        }
    })?;
    // SAFETY: clipboard đang mở; handle thuộc hệ thống, chỉ khóa để đọc trong giới hạn GlobalSize
    // rồi mở khóa trước khi clipboard được đóng (`_open` thả sau khối này).
    unsafe {
        let h = GetClipboardData(CF_UNICODETEXT).ok()?;
        let g = HGLOBAL(h.0);
        let p = GlobalLock(g) as *const u16;
        if p.is_null() {
            return None;
        }
        let units = std::slice::from_raw_parts(p, GlobalSize(g) / 2);
        let end = units.iter().position(|u| *u == 0).unwrap_or(units.len());
        let text = String::from_utf16_lossy(&units[..end]);
        let _ = GlobalUnlock(g);
        Some(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use windows::Win32::Foundation::HGLOBAL;
    use windows::Win32::System::DataExchange::{CloseClipboard, GetClipboardData, OpenClipboard};
    use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};

    /// Hai kiểm thử clipboard không được chạy song song.
    static CLIPBOARD_LOCK: Mutex<()> = Mutex::new(());

    const CF_DIB_ID: u32 = 8;
    const CF_UNICODETEXT_ID: u32 = 13;

    /// Đọc nguyên khối dữ liệu của một định dạng trên clipboard (chỉ dùng trong kiểm thử).
    fn read_clipboard(format: u32) -> Option<Vec<u8>> {
        for _ in 0..20 {
            // SAFETY: mở clipboard cho luồng này; luôn đóng lại ngay bên dưới.
            if unsafe { OpenClipboard(None) }.is_ok() {
                // SAFETY: clipboard đang mở; handle thuộc hệ thống, chỉ khóa để đọc rồi mở khóa
                // trước khi đóng clipboard.
                let out = unsafe {
                    GetClipboardData(format).ok().and_then(|h| {
                        let g = HGLOBAL(h.0);
                        let p = GlobalLock(g) as *const u8;
                        if p.is_null() {
                            return None;
                        }
                        let v = std::slice::from_raw_parts(p, GlobalSize(g)).to_vec();
                        let _ = GlobalUnlock(g);
                        Some(v)
                    })
                };
                // SAFETY: đóng clipboard vừa mở.
                unsafe {
                    let _ = CloseClipboard();
                }
                return out;
            }
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        None
    }

    fn read_text() -> Option<String> {
        let bytes = read_clipboard(CF_UNICODETEXT_ID)?;
        let units: Vec<u16> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&c| u16::from_le_bytes(c))
            .take_while(|&u| u != 0)
            .collect();
        String::from_utf16(&units).ok()
    }

    fn u32_at(b: &[u8], off: usize) -> u32 {
        u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
    }

    fn i32_at(b: &[u8], off: usize) -> i32 {
        u32_at(b, off) as i32
    }

    /// (rộng, cao, B, G, R của pixel trên cùng bên trái) đọc từ khối CF_DIB.
    fn parse_dib(b: &[u8]) -> (i32, i32, [u8; 3]) {
        let header = u32_at(b, 0) as usize;
        let width = i32_at(b, 4);
        let height = i32_at(b, 8);
        let bits = u16::from_le_bytes([b[14], b[15]]) as usize;
        let compression = u32_at(b, 16);
        let bpp = bits / 8;
        // BI_BITFIELDS (3) có thêm 3 mặt nạ màu ngay sau header 40 byte.
        let masks = if compression == 3 && header == 40 {
            12
        } else {
            0
        };
        let stride = (width as usize * bpp + 3) & !3;
        let rows = height.unsigned_abs() as usize;
        let top_row = if height > 0 { rows - 1 } else { 0 };
        let p = header + masks + top_row * stride;
        (width, height.abs(), [b[p], b[p + 1], b[p + 2]])
    }

    /// Mọi định dạng loại trừ có mặt trên clipboard với DWORD 0.
    fn assert_exclusion_formats_present() {
        assert_eq!(EXCLUSION_FORMATS.len(), 3);
        for (name, _) in EXCLUSION_FORMATS {
            let h = HSTRING::from(*name);
            // SAFETY: chuỗi kết thúc bằng 0, sống suốt lời gọi.
            let id = unsafe { RegisterClipboardFormatW(&h) };
            assert_ne!(id, 0, "{name}");
            let data = read_clipboard(id).unwrap_or_else(|| panic!("thiếu định dạng {name}"));
            assert!(data.len() >= 4, "{name}");
            assert_eq!(&data[..4], &0u32.to_le_bytes(), "{name}");
        }
    }

    #[test]
    fn exclusion_formats_match_microsoft_docs() {
        // learn.microsoft.com/windows/win32/dataxchg/clipboard-formats,
        // mục "Cloud Clipboard and Clipboard History Formats".
        assert_eq!(
            EXCLUSION_FORMATS,
            &[
                ("ExcludeClipboardContentFromMonitorProcessing", 0),
                ("CanIncludeInClipboardHistory", 0),
                ("CanUploadToCloudClipboard", 0),
            ]
        );
    }

    #[test]
    #[ignore = "ghi đè clipboard thật của máy; chạy tay: cargo test -p darkshot-windows clipboard -- --ignored"]
    fn clipboard_text_round_trip() {
        let _g = CLIPBOARD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let link = "https://img.example.com/d30/Ab12.png";
        WinClipboard.set_text(link).unwrap();
        assert_eq!(read_text().as_deref(), Some(link));
        assert_exclusion_formats_present();

        let vi = "Đã chép link: ảnh tiếng Việt ở đây";
        WinClipboard.set_text(vi).unwrap();
        assert_eq!(read_text().as_deref(), Some(vi));
        assert_exclusion_formats_present();
    }

    #[test]
    #[ignore = "ghi đè clipboard thật của máy; chạy tay: cargo test -p darkshot-windows clipboard -- --ignored"]
    fn clipboard_paste_text_reads_unicode() {
        let _g = CLIPBOARD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let owner = OwnerWindow::new().unwrap();
        let s = "Vi\u{1ec7}t\r\nd\u{f2}ng 2 \u{1f600}";
        WinClipboard.set_text(s).unwrap();
        assert_eq!(paste_text(owner.0).as_deref(), Some(s));
    }

    #[test]
    #[ignore = "ghi đè clipboard thật của máy; chạy tay: cargo test -p darkshot-windows clipboard -- --ignored"]
    fn clipboard_image_round_trip() {
        let _g = CLIPBOARD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // Hàng trên: đỏ, xanh lá, xanh dương; hàng dưới: trắng, đen, xám. BGRA.
        let img = Image {
            width: 3,
            height: 2,
            bgra: vec![
                0, 0, 255, 255, 0, 255, 0, 255, 255, 0, 0, 255, //
                255, 255, 255, 255, 0, 0, 0, 255, 128, 128, 128, 255,
            ],
        };
        WinClipboard.set_image(&img).unwrap();
        let dib = read_clipboard(CF_DIB_ID).expect("CF_DIB có trên clipboard");
        let (w, h, top_left) = parse_dib(&dib);
        assert_eq!((w, h), (3, 2));
        assert_eq!(top_left, [0, 0, 255]);
        assert_exclusion_formats_present();
    }

    #[test]
    fn dib_layout_is_24_bit_bottom_up_with_padding() {
        let img = Image {
            width: 3,
            height: 2,
            bgra: vec![
                1, 2, 3, 255, 4, 5, 6, 255, 7, 8, 9, 255, //
                10, 11, 12, 255, 13, 14, 15, 255, 16, 17, 18, 255,
            ],
        };
        let d = dib_from_image(&img).unwrap();
        // 40 byte header + 2 hàng × (9 byte đệm lên 12).
        assert_eq!(d.len(), 40 + 2 * 12);
        assert_eq!(u32_at(&d, 0), 40);
        assert_eq!((i32_at(&d, 4), i32_at(&d, 8)), (3, 2));
        assert_eq!(u16::from_le_bytes([d[12], d[13]]), 1);
        assert_eq!(u16::from_le_bytes([d[14], d[15]]), 24);
        assert_eq!(u32_at(&d, 16), 0);
        assert_eq!(u32_at(&d, 20), 24);
        // Hàng dưới của ảnh nằm trước.
        assert_eq!(&d[40..52], &[10, 11, 12, 13, 14, 15, 16, 17, 18, 0, 0, 0]);
        assert_eq!(&d[52..64], &[1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 0, 0]);
        assert_eq!(parse_dib(&d), (3, 2, [1, 2, 3]));
    }

    #[test]
    fn dib_rejects_invalid_image() {
        let bad = Image {
            width: 2,
            height: 2,
            bgra: vec![0; 3],
        };
        assert!(dib_from_image(&bad).is_err());
    }
}
