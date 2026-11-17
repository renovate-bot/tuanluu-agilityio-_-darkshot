//! Giải mã PNG về BGRA 8 bit và thu nhỏ ảnh (lọc trung bình diện tích).
//! Chỉ dùng `std::fs` và crate `png`; không phụ thuộc hệ điều hành.

use crate::image::Image;
use std::io::{BufRead, BufReader, Cursor, Seek};
use std::path::Path;

pub const THUMB_MAX_W: u32 = 320;
pub const THUMB_MAX_H: u32 = 200;
pub const MAX_DECODE_SIDE: u32 = 16384;
pub const MAX_DECODE_PIXELS: u64 = 100_000_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// Header khai kích thước vượt `MAX_DECODE_SIDE` hoặc `MAX_DECODE_PIXELS`.
    TooLarge,
    /// Không phải PNG hoặc dữ liệu hỏng.
    Corrupt,
    /// Lỗi đọc tệp (không kèm đường dẫn).
    Io(String),
}

/// Giải mã PNG mọi kiểu màu/độ sâu về BGRA 8 bit; alpha giữ nguyên (không có thì 255).
pub fn decode_png(bytes: &[u8]) -> Result<Image, DecodeError> {
    decode_reader(Cursor::new(bytes))
}

/// Như `decode_png` nhưng đọc từ tệp. Kích thước được kiểm từ header trước khi cấp phát bộ đệm ảnh.
pub fn decode_png_file(path: &Path) -> Result<Image, DecodeError> {
    let f = std::fs::File::open(path).map_err(|e| DecodeError::Io(e.to_string()))?;
    decode_reader(BufReader::new(f))
}

fn decode_reader<R: BufRead + Seek>(r: R) -> Result<Image, DecodeError> {
    let mut dec = png::Decoder::new(r);
    let (w, h) = {
        let info = dec.read_header_info().map_err(map_png_err)?;
        (info.width, info.height)
    };
    if w == 0
        || h == 0
        || w > MAX_DECODE_SIDE
        || h > MAX_DECODE_SIDE
        || u64::from(w) * u64::from(h) > MAX_DECODE_PIXELS
    {
        return Err(if w == 0 || h == 0 {
            DecodeError::Corrupt
        } else {
            DecodeError::TooLarge
        });
    }
    dec.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = dec.read_info().map_err(map_png_err)?;
    let size = reader.output_buffer_size().ok_or(DecodeError::TooLarge)?;
    let mut buf = vec![0u8; size];
    let out = reader.next_frame(&mut buf).map_err(map_png_err)?;
    if out.width != w || out.height != h || out.bit_depth != png::BitDepth::Eight {
        return Err(DecodeError::Corrupt);
    }
    let samples = out.color_type.samples();
    let line = out.line_size;
    let (wu, hu) = (w as usize, h as usize);
    if line < wu * samples || buf.len() < line * hu {
        return Err(DecodeError::Corrupt);
    }
    let mut bgra = Vec::with_capacity(wu * hu * 4);
    for row in buf.chunks_exact(line).take(hu) {
        let px = &row[..wu * samples];
        match out.color_type {
            png::ColorType::Grayscale => {
                for &v in px {
                    bgra.extend_from_slice(&[v, v, v, 255]);
                }
            }
            png::ColorType::GrayscaleAlpha => {
                for p in px.as_chunks::<2>().0 {
                    bgra.extend_from_slice(&[p[0], p[0], p[0], p[1]]);
                }
            }
            png::ColorType::Rgb => {
                for p in px.as_chunks::<3>().0 {
                    bgra.extend_from_slice(&[p[2], p[1], p[0], 255]);
                }
            }
            png::ColorType::Rgba => {
                for p in px.as_chunks::<4>().0 {
                    bgra.extend_from_slice(&[p[2], p[1], p[0], p[3]]);
                }
            }
            png::ColorType::Indexed => return Err(DecodeError::Corrupt),
        }
    }
    let img = Image {
        width: w,
        height: h,
        bgra,
    };
    if img.is_valid() {
        Ok(img)
    } else {
        Err(DecodeError::Corrupt)
    }
}

fn map_png_err(e: png::DecodingError) -> DecodeError {
    match e {
        png::DecodingError::IoError(e) if e.kind() != std::io::ErrorKind::UnexpectedEof => {
            DecodeError::Io(e.to_string())
        }
        png::DecodingError::LimitsExceeded => DecodeError::TooLarge,
        _ => DecodeError::Corrupt,
    }
}

/// Kích thước vừa khung `max_w × max_h`, giữ tỉ lệ, không phóng to, tối thiểu 1×1.
pub fn fit_within(w: u32, h: u32, max_w: u32, max_h: u32) -> (u32, u32) {
    let (w, h) = (w.max(1), h.max(1));
    let (max_w, max_h) = (max_w.max(1), max_h.max(1));
    if w <= max_w && h <= max_h {
        return (w, h);
    }
    let (w64, h64, mw, mh) = (
        u64::from(w),
        u64::from(h),
        u64::from(max_w),
        u64::from(max_h),
    );
    // So sánh w/max_w với h/max_h bằng nhân chéo để tránh số thực.
    let (nw, nh) = if w64 * mh >= h64 * mw {
        (mw, (h64 * mw + w64 / 2) / w64)
    } else {
        ((w64 * mh + h64 / 2) / h64, mh)
    };
    (
        nw.clamp(1, mw) as u32, // ≤ max_w nên vừa u32
        nh.clamp(1, mh) as u32,
    )
}

/// Thu nhỏ bằng trung bình diện tích (màu trọng số theo alpha). Không phóng to: ảnh đã vừa thì trả bản sao.
pub fn downscale(img: &Image, max_w: u32, max_h: u32) -> Image {
    if !img.is_valid() {
        return img.clone();
    }
    let (dw, dh) = fit_within(img.width, img.height, max_w, max_h);
    if dw == img.width && dh == img.height {
        return img.clone();
    }
    let (sw, sh) = (img.width as usize, img.height as usize);
    let (dw, dh) = (dw as usize, dh as usize);
    // Biên cột nguồn cho từng cột đích (mỗi ô ít nhất một pixel).
    let span = |i: usize, s: usize, d: usize| -> (usize, usize) {
        let a = i * s / d;
        let b = ((i + 1) * s / d).clamp(a + 1, s);
        (a, b)
    };
    let cols: Vec<(usize, usize)> = (0..dw).map(|x| span(x, sw, dw)).collect();
    let mut out = Vec::with_capacity(dw * dh * 4);
    // Tổng mỗi cột đích cho một hàng đích: [B*A, G*A, R*A, A, B, G, R, n].
    let mut acc = vec![[0u64; 8]; dw];
    for y in 0..dh {
        let (y0, y1) = span(y, sh, dh);
        for a in acc.iter_mut() {
            *a = [0; 8];
        }
        for sy in y0..y1 {
            let row = &img.bgra[sy * sw * 4..(sy + 1) * sw * 4];
            for (a, &(x0, x1)) in acc.iter_mut().zip(cols.iter()) {
                for p in row[x0 * 4..x1 * 4].as_chunks::<4>().0 {
                    let al = u64::from(p[3]);
                    a[0] += u64::from(p[0]) * al;
                    a[1] += u64::from(p[1]) * al;
                    a[2] += u64::from(p[2]) * al;
                    a[3] += al;
                    a[4] += u64::from(p[0]);
                    a[5] += u64::from(p[1]);
                    a[6] += u64::from(p[2]);
                    a[7] += 1;
                }
            }
        }
        for a in &acc {
            let n = a[7].max(1);
            let avg = |s: u64, d: u64| ((s + d / 2) / d).min(255) as u8;
            if a[3] > 0 {
                out.extend_from_slice(&[
                    avg(a[0], a[3]),
                    avg(a[1], a[3]),
                    avg(a[2], a[3]),
                    avg(a[3], n),
                ]);
            } else {
                out.extend_from_slice(&[avg(a[4], n), avg(a[5], n), avg(a[6], n), 0]);
            }
        }
    }
    Image {
        width: dw as u32,
        height: dh as u32,
        bgra: out,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enc(
        w: u32,
        h: u32,
        color: png::ColorType,
        depth: png::BitDepth,
        data: &[u8],
        palette: Option<Vec<u8>>,
        trns: Option<Vec<u8>>,
    ) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut e = png::Encoder::new(&mut out, w, h);
            e.set_color(color);
            e.set_depth(depth);
            if let Some(p) = palette {
                e.set_palette(p);
            }
            if let Some(t) = trns {
                e.set_trns(t);
            }
            let mut wr = e.write_header().unwrap();
            wr.write_image_data(data).unwrap();
        }
        out
    }

    fn px(img: &Image, i: usize) -> [u8; 4] {
        [
            img.bgra[i * 4],
            img.bgra[i * 4 + 1],
            img.bgra[i * 4 + 2],
            img.bgra[i * 4 + 3],
        ]
    }

    #[test]
    fn decodes_every_color_type() {
        use png::BitDepth::*;
        use png::ColorType::*;
        let g = decode_png(&enc(2, 1, Grayscale, Eight, &[10, 200], None, None)).unwrap();
        assert_eq!((g.width, g.height), (2, 1));
        assert_eq!(px(&g, 0), [10, 10, 10, 255]);
        assert_eq!(px(&g, 1), [200, 200, 200, 255]);

        let ga = decode_png(&enc(1, 1, GrayscaleAlpha, Eight, &[50, 7], None, None)).unwrap();
        assert_eq!(px(&ga, 0), [50, 50, 50, 7]);

        let rgb = decode_png(&enc(1, 2, Rgb, Eight, &[1, 2, 3, 4, 5, 6], None, None)).unwrap();
        assert_eq!((rgb.width, rgb.height), (1, 2));
        assert_eq!(px(&rgb, 0), [3, 2, 1, 255]);
        assert_eq!(px(&rgb, 1), [6, 5, 4, 255]);

        let rgba = decode_png(&enc(1, 1, Rgba, Eight, &[10, 20, 30, 40], None, None)).unwrap();
        assert_eq!(px(&rgba, 0), [30, 20, 10, 40]);

        let idx = decode_png(&enc(
            3,
            1,
            Indexed,
            Eight,
            &[0, 1, 2],
            Some(vec![255, 0, 0, 0, 255, 0, 0, 0, 255]),
            Some(vec![255, 128]),
        ))
        .unwrap();
        assert_eq!(px(&idx, 0), [0, 0, 255, 255]);
        assert_eq!(px(&idx, 1), [0, 255, 0, 128]);
        assert_eq!(px(&idx, 2), [255, 0, 0, 255]);

        let rgba16 = decode_png(&enc(
            1,
            1,
            Rgba,
            Sixteen,
            &[0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0],
            None,
            None,
        ))
        .unwrap();
        assert_eq!(px(&rgba16, 0), [0x9a, 0x56, 0x12, 0xde]);

        let g16 = decode_png(&enc(1, 1, Grayscale, Sixteen, &[0xab, 0xcd], None, None)).unwrap();
        assert_eq!(px(&g16, 0), [0xab, 0xab, 0xab, 255]);

        let rgb16 = decode_png(&enc(1, 1, Rgb, Sixteen, &[1, 0, 2, 0, 3, 0], None, None)).unwrap();
        assert_eq!(px(&rgb16, 0), [3, 2, 1, 255]);

        let g1 = decode_png(&enc(2, 1, Grayscale, One, &[0b1000_0000], None, None)).unwrap();
        assert_eq!(px(&g1, 0), [255, 255, 255, 255]);
        assert_eq!(px(&g1, 1), [0, 0, 0, 255]);
    }

    #[test]
    fn decode_round_trips_encode_png() {
        let img = Image {
            width: 3,
            height: 2,
            bgra: (0..24)
                .map(|i| if i % 4 == 3 { 255 } else { i * 9 })
                .collect(),
        };
        let back = decode_png(&crate::image::encode_png(&img).unwrap()).unwrap();
        assert_eq!(back, img);
    }

    fn crc32(data: &[u8]) -> u32 {
        let mut c = 0xFFFF_FFFFu32;
        for &b in data {
            c ^= u32::from(b);
            for _ in 0..8 {
                c = if c & 1 != 0 {
                    0xEDB8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
            }
        }
        !c
    }

    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend_from_slice(data);
        out.extend_from_slice(&body);
        out.extend_from_slice(&crc32(&body).to_be_bytes());
    }

    /// PNG chỉ có IHDR khai `w × h` và IEND, không có dữ liệu ảnh.
    fn header_only(w: u32, h: u32) -> Vec<u8> {
        let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&w.to_be_bytes());
        ihdr.extend_from_slice(&h.to_be_bytes());
        ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
        chunk(&mut out, b"IHDR", &ihdr);
        chunk(&mut out, b"IEND", &[]);
        out
    }

    #[test]
    fn decode_bomb_is_rejected_from_header() {
        assert_eq!(
            decode_png(&header_only(20000, 20000)),
            Err(DecodeError::TooLarge)
        );
        assert_eq!(
            decode_png(&header_only(MAX_DECODE_SIDE + 1, 1)),
            Err(DecodeError::TooLarge)
        );
        assert_eq!(
            decode_png(&header_only(12000, 10000)),
            Err(DecodeError::TooLarge)
        );
        // Kích thước hợp lệ nhưng không có IDAT: hỏng chứ không phải quá lớn.
        assert_eq!(decode_png(&header_only(4, 4)), Err(DecodeError::Corrupt));

        let dir = std::env::temp_dir().join(format!("darkshot_hist_thumb_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("bomb.png");
        std::fs::write(&f, header_only(20000, 20000)).unwrap();
        assert_eq!(decode_png_file(&f), Err(DecodeError::TooLarge));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn garbage_is_corrupt_and_missing_is_io() {
        assert_eq!(decode_png(b"not a png at all"), Err(DecodeError::Corrupt));
        assert_eq!(decode_png(&[]), Err(DecodeError::Corrupt));
        let good = enc(
            4,
            4,
            png::ColorType::Rgb,
            png::BitDepth::Eight,
            &[7; 48],
            None,
            None,
        );
        assert_eq!(
            decode_png(&good[..good.len() - 20]),
            Err(DecodeError::Corrupt)
        );
        let missing = std::env::temp_dir().join("darkshot_hist_missing_zz9.png");
        assert!(matches!(decode_png_file(&missing), Err(DecodeError::Io(_))));
    }

    #[test]
    fn fit_within_rules() {
        assert_eq!(fit_within(100, 50, 320, 200), (100, 50));
        assert_eq!(fit_within(1920, 1080, 320, 200), (320, 180));
        assert_eq!(fit_within(10000, 1, 320, 200), (320, 1));
        assert_eq!(fit_within(1, 10000, 320, 200), (1, 200));
        assert_eq!(fit_within(1000, 1000, 320, 200), (200, 200));
        assert_eq!(fit_within(320, 200, 320, 200), (320, 200));
        assert_eq!(fit_within(0, 0, 320, 200), (1, 1));
        assert_eq!(fit_within(500, 500, 0, 0), (1, 1));
        for (w, h) in [(65535, 1), (1, 65535), (3, 70000), (u32::MAX, u32::MAX)] {
            let (a, b) = fit_within(w, h, 320, 200);
            assert!(a >= 1 && b >= 1 && a <= 320 && b <= 200, "{w}x{h}");
        }
    }

    fn solid(w: u32, h: u32, p: [u8; 4]) -> Image {
        Image {
            width: w,
            height: h,
            bgra: p.repeat((w * h) as usize),
        }
    }

    #[test]
    fn downscale_sizes_colors_alpha() {
        let img = solid(1920, 1080, [12, 34, 56, 255]);
        let d = downscale(&img, 320, 200);
        assert_eq!((d.width, d.height), (320, 180));
        assert!(d.is_valid());
        assert!(d.bgra.chunks(4).all(|p| p == [12, 34, 56, 255]));

        let semi = solid(1000, 10, [200, 100, 50, 77]);
        let d = downscale(&semi, 320, 200);
        assert_eq!((d.width, d.height), (320, 3));
        assert!(d.bgra.chunks(4).all(|p| p == [200, 100, 50, 77]));

        let thin = solid(10000, 1, [1, 2, 3, 4]);
        let d = downscale(&thin, 320, 200);
        assert_eq!((d.width, d.height), (320, 1));

        // Không phóng to: trả bản sao.
        let small = solid(10, 5, [9, 9, 9, 9]);
        assert_eq!(downscale(&small, 320, 200), small);

        // Trung bình diện tích: 2×1 đen/trắng → 1×1 xám.
        let mut bw = solid(2, 1, [0, 0, 0, 255]);
        bw.bgra[4..8].copy_from_slice(&[255, 255, 255, 255]);
        let d = downscale(&bw, 1, 1);
        assert_eq!(d.bgra, vec![128, 128, 128, 255]);

        // Pixel trong suốt hoàn toàn không làm bẩn màu.
        let mut tr = solid(2, 1, [0, 0, 0, 0]);
        tr.bgra[4..8].copy_from_slice(&[0, 0, 255, 255]);
        let d = downscale(&tr, 1, 1);
        assert_eq!(d.bgra, vec![0, 0, 255, 128]);
    }
}
