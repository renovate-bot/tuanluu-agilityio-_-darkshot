//! Ảnh chụp trong bộ nhớ và mã hóa PNG.

use crate::geometry::Rect;
use crate::i18n::{self, Lang, Msg};

/// Ảnh từ trên xuống, 4 byte mỗi pixel: B, G, R, A.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
}

impl Image {
    /// Đúng khi kích thước dương và `bgra` dài đúng `width * height * 4`.
    pub fn is_valid(&self) -> bool {
        self.width > 0
            && self.height > 0
            && (self.width as usize)
                .checked_mul(self.height as usize)
                .and_then(|n| n.checked_mul(4))
                == Some(self.bgra.len())
    }

    /// Cắt theo `r`; `r` được kẹp vào ảnh.
    pub fn crop(&self, r: Rect) -> Image {
        if !self.is_valid() {
            return Image {
                width: 0,
                height: 0,
                bgra: Vec::new(),
            };
        }
        let x0 = i64::from(r.x).max(0);
        let y0 = i64::from(r.y).max(0);
        let x1 = (i64::from(r.x) + i64::from(r.w)).min(i64::from(self.width));
        let y1 = (i64::from(r.y) + i64::from(r.h)).min(i64::from(self.height));
        if x1 <= x0 || y1 <= y0 {
            return Image {
                width: 0,
                height: 0,
                bgra: Vec::new(),
            };
        }
        let (w, h) = ((x1 - x0) as usize, (y1 - y0) as usize);
        let stride = self.width as usize * 4;
        let mut bgra = Vec::with_capacity(w * h * 4);
        for y in y0 as usize..y1 as usize {
            let start = y * stride + x0 as usize * 4;
            bgra.extend_from_slice(&self.bgra[start..start + w * 4]);
        }
        Image {
            width: w as u32,
            height: h as u32,
            bgra,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum EncodeError {
    InvalidImage,
    Encoder(String),
}

impl EncodeError {
    pub fn message(&self, lang: Lang) -> String {
        match self {
            EncodeError::InvalidImage => i18n::t(lang, Msg::EncodeInvalidImage).to_string(),
            EncodeError::Encoder(e) => i18n::encoder_error(lang, e),
        }
    }
}

/// PNG RGB 8 bit, bỏ kênh alpha.
pub fn encode_png(img: &Image) -> Result<Vec<u8>, EncodeError> {
    if !img.is_valid() {
        return Err(EncodeError::InvalidImage);
    }
    let mut rgb = Vec::with_capacity(img.width as usize * img.height as usize * 3);
    for p in img.bgra.as_chunks::<4>().0 {
        rgb.extend_from_slice(&[p[2], p[1], p[0]]);
    }
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, img.width, img.height);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc
            .write_header()
            .map_err(|e| EncodeError::Encoder(e.to_string()))?;
        w.write_image_data(&rgb)
            .map_err(|e| EncodeError::Encoder(e.to_string()))?;
    }
    Ok(out)
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
    fn crop_of_short_buffer_is_empty() {
        let img = Image {
            width: 4,
            height: 4,
            bgra: vec![0; 8],
        };
        let c = img.crop(Rect {
            x: 0,
            y: 0,
            w: 2,
            h: 2,
        });
        assert_eq!((c.width, c.height), (0, 0));
        assert!(c.bgra.is_empty());
    }

    #[test]
    fn encode_rejects_invalid_images() {
        let empty = Image {
            width: 0,
            height: 0,
            bgra: vec![],
        };
        assert_eq!(encode_png(&empty), Err(EncodeError::InvalidImage));
        let short = Image {
            width: 2,
            height: 2,
            bgra: vec![1, 2, 3],
        };
        assert_eq!(encode_png(&short), Err(EncodeError::InvalidImage));
        assert!(!empty.is_valid() && !short.is_valid());
        assert!(gradient(2, 2).is_valid());
    }

    #[test]
    fn encode_error_messages() {
        for e in [
            EncodeError::InvalidImage,
            EncodeError::Encoder("boom".into()),
        ] {
            let (vi, en) = (e.message(Lang::Vi), e.message(Lang::En));
            assert!(!vi.is_empty() && !en.is_empty() && vi != en, "{e:?}");
        }
        assert!(EncodeError::Encoder("boom".into())
            .message(Lang::En)
            .contains("boom"));
    }

    #[test]
    fn png_round_trip() {
        let mut img = gradient(3, 2);
        img.bgra[0..4].copy_from_slice(&[10, 20, 30, 255]); // B=10 G=20 R=30
        let data = encode_png(&img).unwrap();
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
