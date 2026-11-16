//! Ảnh chụp trong bộ nhớ và mã hóa PNG.

use crate::config::{clamp_jpeg_quality, UploadFormat};
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
    /// Một cạnh vượt 65.535 pixel, không nén JPEG được.
    TooLarge,
}

impl EncodeError {
    pub fn message(&self, lang: Lang) -> String {
        match self {
            EncodeError::InvalidImage => i18n::t(lang, Msg::EncodeInvalidImage).to_string(),
            EncodeError::TooLarge => i18n::t(lang, Msg::EncodeTooLargeForJpeg).to_string(),
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

/// JPEG baseline 8 bit, 4:4:4, không EXIF, không bình luận.
pub fn encode_jpeg(img: &Image, quality: u8) -> Result<Vec<u8>, EncodeError> {
    if !img.is_valid() {
        return Err(EncodeError::InvalidImage);
    }
    let (w, h) = match (u16::try_from(img.width), u16::try_from(img.height)) {
        (Ok(w), Ok(h)) => (w, h),
        _ => return Err(EncodeError::TooLarge),
    };
    let mut out = Vec::new();
    let mut enc = jpeg_encoder::Encoder::new(&mut out, clamp_jpeg_quality(i64::from(quality)));
    enc.set_sampling_factor(jpeg_encoder::SamplingFactor::R_4_4_4);
    enc.set_progressive(false);
    enc.encode(&img.bgra, w, h, jpeg_encoder::ColorType::Bgra)
        .map_err(|e| EncodeError::Encoder(e.to_string()))?;
    Ok(out)
}

pub fn encode_for_upload(
    img: &Image,
    format: UploadFormat,
    quality: u8,
) -> Result<Vec<u8>, EncodeError> {
    match format {
        UploadFormat::Png => encode_png(img),
        UploadFormat::Jpeg => encode_jpeg(img, quality),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Duyệt các đoạn marker từ SOI đến hết SOS; trả (marker, nội dung) từng đoạn.
    fn segments(data: &[u8]) -> Vec<(u8, Vec<u8>)> {
        assert_eq!(&data[..2], &[0xFF, 0xD8]);
        let mut out = Vec::new();
        let mut i = 2;
        while i + 4 <= data.len() {
            assert_eq!(data[i], 0xFF, "marker tại {i}");
            let m = data[i + 1];
            let len = usize::from(u16::from_be_bytes([data[i + 2], data[i + 3]]));
            out.push((m, data[i + 4..i + 2 + len].to_vec()));
            i += 2 + len;
            if m == 0xDA {
                break;
            }
        }
        out
    }

    fn colorful(w: u32, h: u32) -> Image {
        let mut bgra = Vec::new();
        for y in 0..h {
            for x in 0..w {
                bgra.extend_from_slice(&[
                    (x * 37 + y * 11) as u8,
                    (x * 5 + y * 71) as u8,
                    (x * y * 13) as u8,
                    255,
                ]);
            }
        }
        Image {
            width: w,
            height: h,
            bgra,
        }
    }

    #[test]
    fn jpeg_structure() {
        let img = colorful(37, 21);
        for q in [50u8, 89, 90, 100] {
            let data = encode_jpeg(&img, q).unwrap();
            assert_eq!(&data[..2], &[0xFF, 0xD8], "q {q}");
            assert_eq!(&data[data.len() - 2..], &[0xFF, 0xD9], "q {q}");
            let segs = segments(&data);
            let sof0: Vec<_> = segs.iter().filter(|(m, _)| *m == 0xC0).collect();
            assert_eq!(sof0.len(), 1, "q {q}");
            let p = &sof0[0].1;
            assert_eq!(p[0], 8);
            assert_eq!(u16::from_be_bytes([p[1], p[2]]), 21);
            assert_eq!(u16::from_be_bytes([p[3], p[4]]), 37);
            assert_eq!(p[5], 3);
            for c in 0..3 {
                assert_eq!(p[6 + c * 3 + 1], 0x11, "q {q} thành phần {c}");
            }
            assert!(!segs.iter().any(|(m, _)| *m == 0xC2), "progressive q {q}");
            assert!(!segs.iter().any(|(m, _)| *m == 0xE1), "APP1 q {q}");
            assert!(!segs.iter().any(|(m, _)| *m == 0xFE), "COM q {q}");
        }
    }

    #[test]
    fn jpeg_quality_changes_size() {
        let mut state: u32 = 12345;
        let mut bgra = Vec::new();
        for _ in 0..64 * 64 {
            for _ in 0..3 {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                bgra.push((state >> 24) as u8);
            }
            bgra.push(255);
        }
        let img = Image {
            width: 64,
            height: 64,
            bgra,
        };
        let lo = encode_jpeg(&img, 50).unwrap().len();
        let hi = encode_jpeg(&img, 100).unwrap().len();
        assert!(hi > lo, "{hi} <= {lo}");
    }

    #[test]
    fn jpeg_rejects_oversized_and_invalid() {
        let empty = Image {
            width: 0,
            height: 0,
            bgra: vec![],
        };
        assert_eq!(encode_jpeg(&empty, 90), Err(EncodeError::InvalidImage));
        let short = Image {
            width: 2,
            height: 2,
            bgra: vec![1, 2, 3],
        };
        assert_eq!(encode_jpeg(&short, 90), Err(EncodeError::InvalidImage));
        let wide = Image {
            width: 65_536,
            height: 1,
            bgra: vec![0; 65_536 * 4],
        };
        assert!(wide.is_valid());
        assert_eq!(encode_jpeg(&wide, 90), Err(EncodeError::TooLarge));
        let ok = Image {
            width: 65_535,
            height: 1,
            bgra: vec![0; 65_535 * 4],
        };
        assert!(encode_jpeg(&ok, 90).is_ok());
    }

    #[test]
    fn encode_for_upload_picks_encoder() {
        let img = colorful(9, 7);
        assert_eq!(
            encode_for_upload(&img, UploadFormat::Png, 80).unwrap(),
            encode_png(&img).unwrap()
        );
        assert_eq!(
            encode_for_upload(&img, UploadFormat::Jpeg, 80).unwrap(),
            encode_jpeg(&img, 80).unwrap()
        );
    }

    #[test]
    fn too_large_message_in_both_languages() {
        let (vi, en) = (
            EncodeError::TooLarge.message(Lang::Vi),
            EncodeError::TooLarge.message(Lang::En),
        );
        assert!(!vi.is_empty() && !en.is_empty() && vi != en);
    }

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
