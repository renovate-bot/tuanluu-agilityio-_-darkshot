//! Ảnh chụp trong bộ nhớ và mã hóa PNG.

use crate::geometry::Rect;

/// Ảnh từ trên xuống, 4 byte mỗi pixel: B, G, R, A.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
}

impl Image {
    /// Cắt theo `r`; `r` được kẹp vào ảnh.
    pub fn crop(&self, r: Rect) -> Image {
        let _ = r;
        todo!()
    }
}

/// PNG RGB 8 bit, bỏ kênh alpha.
pub fn encode_png(img: &Image) -> Vec<u8> {
    let _ = img;
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gradient(w: u32, h: u32) -> Image {
        let mut bgra = Vec::new();
        for y in 0..h {
            for x in 0..w {
                bgra.extend_from_slice(&[x as u8, y as u8, 7, 255]);
            }
        }
        Image {
            width: w,
            height: h,
            bgra,
        }
    }

    #[test]
    fn crop_takes_the_right_pixels() {
        let img = gradient(4, 3);
        let c = img.crop(Rect {
            x: 1,
            y: 1,
            w: 2,
            h: 2,
        });
        assert_eq!((c.width, c.height), (2, 2));
        assert_eq!(c.bgra.len(), 16);
        let bg: Vec<u8> = c.bgra.chunks(4).flat_map(|p| [p[0], p[1]]).collect();
        assert_eq!(bg, vec![1, 1, 2, 1, 1, 2, 2, 2]);
    }

    #[test]
    fn crop_is_clamped() {
        let img = gradient(4, 3);
        let c = img.crop(Rect {
            x: 3,
            y: 2,
            w: 10,
            h: 10,
        });
        assert_eq!((c.width, c.height), (1, 1));
        assert_eq!(c.bgra.len(), 4);
        let c = img.crop(Rect {
            x: -2,
            y: -2,
            w: 4,
            h: 4,
        });
        assert_eq!((c.width, c.height), (2, 2));
        let c = img.crop(Rect {
            x: 50,
            y: 50,
            w: 4,
            h: 4,
        });
        assert_eq!((c.width, c.height), (0, 0));
        assert!(c.bgra.is_empty());
    }

    #[test]
    fn png_round_trip() {
        let mut img = gradient(3, 2);
        img.bgra[0..4].copy_from_slice(&[10, 20, 30, 255]); // B=10 G=20 R=30
        let data = encode_png(&img);
        assert_eq!(data[..8], [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
        let dec = png::Decoder::new(std::io::Cursor::new(&data));
        let mut reader = dec.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut buf).unwrap();
        assert_eq!((info.width, info.height), (3, 2));
        assert_eq!(info.color_type, png::ColorType::Rgb);
        assert_eq!(info.bit_depth, png::BitDepth::Eight);
        assert_eq!(&buf[0..3], &[30, 20, 10]);
    }
}
