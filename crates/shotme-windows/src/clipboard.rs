//! Clipboard của Windows: chép link (văn bản) và ảnh.

use shotme_core::actions::Clipboard;
use shotme_core::image::Image;

/// Clipboard thật của hệ thống. Dùng được từ bất kỳ luồng nào.
#[derive(Debug, Clone, Copy, Default)]
pub struct WinClipboard;

impl Clipboard for WinClipboard {
    fn set_text(&self, _s: &str) -> Result<(), String> {
        todo!()
    }

    fn set_image(&self, _img: &Image) -> Result<(), String> {
        todo!()
    }
}

/// Dựng khối CF_DIB 24 bit (BITMAPINFOHEADER + pixel từ dưới lên, mỗi hàng đệm tới bội của 4).
pub(crate) fn dib_from_image(_img: &Image) -> Result<Vec<u8>, String> {
    todo!()
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
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
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

    #[test]
    #[ignore = "ghi đè clipboard thật của máy; chạy tay: cargo test -p shotme-windows clipboard -- --ignored"]
    fn clipboard_text_round_trip() {
        let _g = CLIPBOARD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let link = "https://img.example.com/d30/Ab12.png";
        WinClipboard.set_text(link).unwrap();
        assert_eq!(read_text().as_deref(), Some(link));

        let vi = "Đã chép link: ảnh tiếng Việt ở đây";
        WinClipboard.set_text(vi).unwrap();
        assert_eq!(read_text().as_deref(), Some(vi));
    }

    #[test]
    #[ignore = "ghi đè clipboard thật của máy; chạy tay: cargo test -p shotme-windows clipboard -- --ignored"]
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
