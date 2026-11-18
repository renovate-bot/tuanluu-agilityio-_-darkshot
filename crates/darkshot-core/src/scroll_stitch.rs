//! Ghép các khung của một lần chụp cuộn thành một ảnh dọc.
//!
//! Mỗi khung là cùng một vùng màn hình (W×H, BGRA) chụp lại sau mỗi lượt; người dùng tự cuộn
//! nội dung xuống. Thuật toán so băm từng hàng (FNV-1a 64 bit trên B, G, R), bỏ dải tĩnh trên và
//! dưới (tiêu đề, chân trang dính), tìm độ dịch dy của vùng chạy rồi nối dy hàng mới vào đáy.
//! Không bao giờ đoán: khớp không chắc chắn thì bỏ khung và trả `Push::Uncertain`.

use crate::image::Image;

/// Chiều cao tối đa của ảnh ghép.
pub const MAX_HEIGHT: u32 = 30_000;
/// Số byte tối đa của ảnh ghép (BGRA).
pub const MAX_BYTES: u64 = 256 * 1024 * 1024;
/// Chồng lấp tối thiểu giữa hai khung, tính bằng hàng.
pub const MIN_OVERLAP_PX: u32 = 24;
/// Chồng lấp tối thiểu theo tỉ lệ chiều cao vùng chạy.
pub const MIN_OVERLAP_FRAC: f64 = 0.15;
/// Tỉ lệ hàng khớp tối thiểu trên vùng chồng lấp.
pub const MATCH_RATIO: f64 = 0.98;
/// Số hàng phân biệt tối thiểu trong vùng chồng lấp đã khớp.
pub const MIN_DISTINCT_ROWS: u32 = 8;

/// Lý do một khung không ghép được.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    /// Nhiều độ dịch khớp gần như nhau, hoặc quá ít hàng phân biệt (nền một màu, nền lặp).
    Ambiguous,
    /// Không còn chồng lấp đủ (cuộn quá nhanh) hoặc nội dung đổi không phải do cuộn.
    NoOverlap,
    /// Nội dung bị cuộn lên.
    Backward,
    /// Khung khác cỡ với `Stitcher::new`, rỗng hoặc hỏng.
    SizeMismatch,
}

/// Kết quả của một lần `push`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Push {
    /// Khung giống khung đã nhận trước đó (hoặc chỉ đổi cục bộ, không cuộn).
    NoChange,
    /// Đã nối số hàng này (khung đầu: cả chiều cao khung).
    Appended(u32),
    /// Khung bị bỏ.
    Uncertain(Reason),
    /// Đã đạt giới hạn chiều cao hoặc bộ nhớ; không nhận thêm khung.
    Limit,
}

/// FNV-1a 64 bit trên các byte B, G, R của một hàng BGRA (bỏ alpha).
fn row_hash(row: &[u8]) -> u64 {
    let _ = row;
    todo!()
}

/// Đúng khi ảnh W×H nằm trong `MAX_HEIGHT` và `MAX_BYTES`.
fn within_limits(w: u32, h: u32) -> bool {
    let _ = (w, h);
    todo!()
}

/// Ghép các khung cùng cỡ thành một ảnh dọc.
pub struct Stitcher {
    w: u32,
    h: u32,
}

impl Stitcher {
    /// Bộ ghép cho các khung W×H.
    pub fn new(w: u32, h: u32) -> Stitcher {
        Stitcher { w, h }
    }

    /// Nhận một khung.
    pub fn push(&mut self, frame: &Image) -> Push {
        let _ = (frame, self.w, self.h);
        todo!()
    }

    /// Chiều cao ảnh ghép hiện tại (0 khi chưa có khung).
    pub fn height(&self) -> u32 {
        todo!()
    }

    /// Số chỗ ghép không chắc chắn: mỗi chuỗi khung liên tiếp không ghép được, sau khi đã có ít
    /// nhất một lần nối, tính là một.
    pub fn uncertain_count(&self) -> u32 {
        todo!()
    }

    /// Số khung đã góp hàng vào ảnh, kể cả khung đầu.
    pub fn appended_frames(&self) -> u32 {
        todo!()
    }

    /// Ảnh ghép và số chỗ ghép không chắc chắn. Chưa có khung: ảnh 0×0.
    pub fn finish(self) -> (Image, u32) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;

    /// Bộ sinh số giả ngẫu nhiên có seed (xorshift64*), để ảnh tổng hợp xác định.
    struct Rng(u64);
    impl Rng {
        fn new(seed: u64) -> Rng {
            Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
        }
        fn next(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.0 = x;
            x.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }
        fn below(&mut self, n: u64) -> u64 {
            self.next() % n
        }
        fn color(&mut self) -> [u8; 4] {
            let v = self.next();
            [v as u8, (v >> 8) as u8, (v >> 16) as u8, 255]
        }
    }

    fn set_px(img: &mut Image, x: u32, y: u32, c: [u8; 4]) {
        let i = ((y * img.width + x) * 4) as usize;
        img.bgra[i..i + 4].copy_from_slice(&c);
    }

    fn blank(w: u32, h: u32) -> Image {
        Image {
            width: w,
            height: h,
            bgra: vec![0; (w * h * 4) as usize],
        }
    }

    /// Trang tổng hợp cao: các khối nhiễu, sọc ngang, sọc dọc, ô vuông và "chữ" thưa trên nền
    /// trắng; mỗi khối ngắn hơn chồng lấp tối thiểu nên mỗi dy chỉ có một chỗ khớp.
    fn page(w: u32, h: u32, seed: u64) -> Image {
        let mut img = blank(w, h);
        let mut rng = Rng::new(seed);
        let mut y = 0;
        let mut kind = 0u32;
        while y < h {
            let len = (20 + rng.below(70) as u32).min(h - y);
            match kind % 5 {
                0 => {
                    for yy in y..y + len {
                        for x in 0..w {
                            let c = rng.color();
                            set_px(&mut img, x, yy, c);
                        }
                    }
                }
                1 => {
                    let (a, b) = (rng.color(), rng.color());
                    let p = 2 + rng.below(6) as u32;
                    for yy in y..y + len {
                        let c = if (yy / p) % 2 == 0 { a } else { b };
                        for x in 0..w {
                            set_px(&mut img, x, yy, c);
                        }
                    }
                }
                2 => {
                    let (a, b) = (rng.color(), rng.color());
                    let p = 3 + rng.below(9) as u32;
                    for yy in y..y + len {
                        for x in 0..w {
                            let c = if (x / p) % 2 == 0 { a } else { b };
                            set_px(&mut img, x, yy, c);
                        }
                    }
                }
                3 => {
                    let (a, b) = (rng.color(), rng.color());
                    let p = 4 + rng.below(12) as u32;
                    for yy in y..y + len {
                        for x in 0..w {
                            let c = if ((x / p) + (yy / p)) % 2 == 0 { a } else { b };
                            set_px(&mut img, x, yy, c);
                        }
                    }
                }
                _ => {
                    let ink = rng.color();
                    for yy in y..y + len {
                        for x in 0..w {
                            set_px(&mut img, x, yy, [255, 255, 255, 255]);
                        }
                        // Dòng chữ cao 9 hàng, cách nhau 5 hàng trắng.
                        if (yy - y) % 14 < 9 {
                            for _ in 0..(w / 12).max(1) {
                                let x = rng.below(u64::from(w)) as u32;
                                set_px(&mut img, x, yy, ink);
                            }
                        }
                    }
                }
            }
            y += len;
            kind += 1;
        }
        img
    }

    /// Trang toàn nhiễu: mọi hàng khác nhau.
    fn noise_page(w: u32, h: u32, seed: u64) -> Image {
        let mut img = blank(w, h);
        let mut rng = Rng::new(seed);
        for p in img.bgra.chunks_mut(4) {
            p.copy_from_slice(&rng.color());
        }
        img
    }

    fn rows(img: &Image, y0: u32, y1: u32) -> Vec<u8> {
        let s = (img.width * 4) as usize;
        img.bgra[y0 as usize * s..y1 as usize * s].to_vec()
    }

    fn frame(pg: &Image, offset: u32, h: u32) -> Image {
        pg.crop(Rect {
            x: 0,
            y: offset as i32,
            w: pg.width as i32,
            h: h as i32,
        })
    }

    /// Phần trang [y0, y1) dưới dạng ảnh.
    fn slice(pg: &Image, y0: u32, y1: u32) -> Image {
        Image {
            width: pg.width,
            height: y1 - y0,
            bgra: rows(pg, y0, y1),
        }
    }

    const W: u32 = 900;
    const H: u32 = 700;

    /// Đẩy các khung tại các offset; trả kết quả của từng lần đẩy.
    fn push_offsets(st: &mut Stitcher, pg: &Image, offsets: &[u32]) -> Vec<Push> {
        offsets.iter().map(|&o| st.push(&frame(pg, o, H))).collect()
    }

    #[test]
    fn row_hash_is_fnv1a_over_bgr_ignoring_alpha() {
        // FNV-1a 64 của chuỗi rỗng, "a" và "foobar" (vector thử chuẩn).
        assert_eq!(row_hash(&[]), 0xcbf2_9ce4_8422_2325);
        // "foobar" = hai điểm ảnh (f, o, o) và (b, a, r); alpha bị bỏ qua.
        let px = [b'f', b'o', b'o', 0, b'b', b'a', b'r', 77];
        assert_eq!(row_hash(&px), 0x8594_4171_f739_67e8);
        let px2 = [b'f', b'o', b'o', 255, b'b', b'a', b'r', 1];
        assert_eq!(row_hash(&px2), row_hash(&px));
        // Một điểm ảnh khác alpha nhưng cùng BGR: cùng băm; khác BGR: khác băm.
        assert_ne!(row_hash(&[b'a', 0, 0, 0]), row_hash(&[b'b', 0, 0, 0]));
    }

    #[test]
    fn limits() {
        assert!(within_limits(900, MAX_HEIGHT));
        assert!(!within_limits(900, MAX_HEIGHT + 1));
        // 256 MiB / (4 byte × 4096) = 16 384 hàng.
        assert!(within_limits(4096, 16_384));
        assert!(!within_limits(4096, 16_385));
        assert!(!within_limits(u32::MAX, u32::MAX));
    }

    #[test]
    fn first_frame_is_the_result_and_no_warning() {
        let pg = page(W, 2000, 1);
        let mut st = Stitcher::new(W, H);
        assert_eq!(st.height(), 0);
        assert_eq!(st.appended_frames(), 0);
        assert_eq!(st.push(&frame(&pg, 0, H)), Push::Appended(H));
        assert_eq!(st.height(), H);
        assert_eq!(st.appended_frames(), 1);
        let (img, n) = st.finish();
        assert_eq!(n, 0);
        assert_eq!(img, frame(&pg, 0, H));
    }

    #[test]
    fn identical_frame_is_no_change() {
        let pg = page(W, 2000, 2);
        let mut st = Stitcher::new(W, H);
        st.push(&frame(&pg, 0, H));
        assert_eq!(st.push(&frame(&pg, 0, H)), Push::NoChange);
        // Alpha khác nhưng BGR giống: vẫn không đổi.
        let mut f = frame(&pg, 0, H);
        for p in f.bgra.chunks_mut(4) {
            p[3] = 0;
        }
        assert_eq!(st.push(&f), Push::NoChange);
        assert_eq!(st.height(), H);
        assert_eq!(st.uncertain_count(), 0);
        assert_eq!(st.finish().0, frame(&pg, 0, H));
    }

    #[test]
    fn even_scroll_matches_page_byte_for_byte() {
        let pg = page(W, 6000, 3);
        let mut st = Stitcher::new(W, H);
        let offsets: Vec<u32> = (0..20).map(|k| k * 120).collect();
        let res = push_offsets(&mut st, &pg, &offsets);
        assert_eq!(res[0], Push::Appended(H));
        for r in &res[1..] {
            assert_eq!(*r, Push::Appended(120));
        }
        let last = *offsets.last().unwrap();
        assert_eq!(st.height(), last + H);
        assert_eq!(st.appended_frames(), 20);
        let (img, n) = st.finish();
        assert_eq!(n, 0);
        assert_eq!(img, slice(&pg, 0, last + H));
    }

    #[test]
    fn uneven_scroll_matches_page_byte_for_byte() {
        let pg = page(W, 6000, 4);
        let mut st = Stitcher::new(W, H);
        // Bước 37, 200, 5, 0, 350.
        let offsets = [0, 37, 237, 242, 242, 592];
        let res = push_offsets(&mut st, &pg, &offsets);
        assert_eq!(
            res,
            vec![
                Push::Appended(H),
                Push::Appended(37),
                Push::Appended(200),
                Push::Appended(5),
                Push::NoChange,
                Push::Appended(350),
            ]
        );
        assert_eq!(st.appended_frames(), 5);
        let (img, n) = st.finish();
        assert_eq!(n, 0);
        assert_eq!(img, slice(&pg, 0, 592 + H));
    }

    /// Khung tại `offset` có tiêu đề dính 60 hàng ở trên và chân trang dính 40 hàng ở dưới.
    fn sticky_frame(pg: &Image, header: &Image, footer: &Image, offset: u32) -> Image {
        let mut f = frame(pg, offset, H);
        let s = (W * 4) as usize;
        f.bgra[..60 * s].copy_from_slice(&header.bgra);
        f.bgra[(H as usize - 40) * s..].copy_from_slice(&footer.bgra);
        f
    }

    #[test]
    fn sticky_header_and_footer_appear_once() {
        let pg = page(W, 6000, 5);
        let header = slice(&page(W, 60, 50), 0, 60);
        let footer = slice(&page(W, 40, 51), 0, 40);
        let mut st = Stitcher::new(W, H);
        let offsets = [0, 120, 240, 300, 500, 620, 900];
        for (k, &o) in offsets.iter().enumerate() {
            let r = st.push(&sticky_frame(&pg, &header, &footer, o));
            if k == 0 {
                assert_eq!(r, Push::Appended(H));
            } else {
                assert_eq!(r, Push::Appended(o - offsets[k - 1]), "frame {k}");
            }
        }
        let last = 900;
        // Tiêu đề, phần trang [60, last + 660), chân trang.
        let mut want = header.bgra.clone();
        want.extend_from_slice(&rows(&pg, 60, last + H - 40));
        want.extend_from_slice(&footer.bgra);
        let want_h = 60 + (last + H - 40 - 60) + 40;
        assert_eq!(st.height(), want_h);
        let (img, n) = st.finish();
        assert_eq!(n, 0);
        assert_eq!((img.width, img.height), (W, want_h));
        assert!(img.bgra == want, "stitched image differs from the page");
    }

    #[test]
    fn uniform_area_longer_than_frame_is_ambiguous() {
        // Nhiễu 300 hàng rồi một màu 1 500 hàng.
        let mut pg = noise_page(W, 1800, 6);
        let s = (W * 4) as usize;
        for p in pg.bgra[300 * s..].chunks_mut(4) {
            p.copy_from_slice(&[40, 50, 60, 255]);
        }
        let mut st = Stitcher::new(W, H);
        assert_eq!(st.push(&frame(&pg, 0, H)), Push::Appended(H));
        assert_eq!(st.push(&frame(&pg, 200, H)), Push::Appended(200));
        // Khung toàn một màu: mọi dy đều khớp.
        assert_eq!(
            st.push(&frame(&pg, 400, H)),
            Push::Uncertain(Reason::Ambiguous)
        );
        assert_eq!(st.height(), 200 + H);
        assert_eq!(st.uncertain_count(), 1);
        let (img, n) = st.finish();
        assert_eq!(n, 1);
        assert_eq!(img, slice(&pg, 0, 200 + H));
    }

    #[test]
    fn repeating_background_is_ambiguous() {
        // Sọc ngang chu kỳ 16 hàng, mỗi hàng có hoa văn (không một màu) để có hàng phân biệt.
        let mut pg = blank(W, 3000);
        for y in 0..3000 {
            for x in 0..W {
                let v = ((y % 16) * 13 + (x % 7)) as u8;
                set_px(&mut pg, x, y, [v, v.wrapping_mul(3), 9, 255]);
            }
        }
        let mut st = Stitcher::new(W, H);
        st.push(&frame(&pg, 0, H));
        assert_eq!(
            st.push(&frame(&pg, 40, H)),
            Push::Uncertain(Reason::Ambiguous)
        );
        assert_eq!(st.height(), H);
        // Chưa có lần nối nào: không tính là chỗ ghép không chắc chắn.
        assert_eq!(st.uncertain_count(), 0);
    }

    #[test]
    fn jump_without_enough_overlap_is_no_overlap() {
        let pg = page(W, 6000, 7);
        let mut st = Stitcher::new(W, H);
        assert_eq!(st.push(&frame(&pg, 0, H)), Push::Appended(H));
        assert_eq!(st.push(&frame(&pg, 120, H)), Push::Appended(120));
        assert_eq!(
            st.push(&frame(&pg, 810, H)),
            Push::Uncertain(Reason::NoOverlap)
        );
        assert_eq!(
            st.push(&frame(&pg, 820, H)),
            Push::Uncertain(Reason::NoOverlap)
        );
        // Hai khung liên tiếp không ghép được: một chỗ.
        assert_eq!(st.uncertain_count(), 1);
        // Cuộn lại về gần khung đã nhận thì ghép tiếp được, đúng nội dung.
        assert_eq!(st.push(&frame(&pg, 400, H)), Push::Appended(280));
        assert_eq!(
            st.push(&frame(&pg, 1100, H)),
            Push::Uncertain(Reason::NoOverlap)
        );
        assert_eq!(st.uncertain_count(), 2);
        let (img, n) = st.finish();
        assert_eq!(n, 2);
        assert_eq!(img, slice(&pg, 0, 400 + H));
    }

    #[test]
    fn scrolling_up_is_backward() {
        let pg = page(W, 6000, 8);
        let mut st = Stitcher::new(W, H);
        st.push(&frame(&pg, 0, H));
        assert_eq!(st.push(&frame(&pg, 300, H)), Push::Appended(300));
        assert_eq!(
            st.push(&frame(&pg, 150, H)),
            Push::Uncertain(Reason::Backward)
        );
        assert_eq!(st.height(), 300 + H);
        let (img, n) = st.finish();
        assert_eq!(n, 1);
        assert_eq!(img, slice(&pg, 0, 300 + H));
    }

    #[test]
    fn height_limit_stops_and_keeps_a_correct_prefix() {
        let w = 16;
        let pg = page(w, MAX_HEIGHT + 2000, 9);
        let mut st = Stitcher::new(w, H);
        let mut offset = 0;
        let mut limited = false;
        while offset + H <= pg.height {
            match st.push(&frame(&pg, offset, H)) {
                Push::Appended(_) => {}
                Push::Limit => {
                    limited = true;
                    break;
                }
                other => panic!("unexpected {other:?} at {offset}"),
            }
            assert!(st.height() <= MAX_HEIGHT);
            offset += 500;
        }
        assert!(limited);
        let h = st.height();
        assert!(h <= MAX_HEIGHT && h > MAX_HEIGHT - 500, "{h}");
        // Sau giới hạn: mọi khung đều Limit.
        assert_eq!(st.push(&frame(&pg, offset, H)), Push::Limit);
        let (img, _) = st.finish();
        assert_eq!(img.height, h);
        assert_eq!(img, slice(&pg, 0, h));
    }

    #[test]
    fn first_frame_over_limits_is_limit() {
        let mut st = Stitcher::new(1, MAX_HEIGHT + 1);
        let f = blank(1, MAX_HEIGHT + 1);
        assert_eq!(st.push(&f), Push::Limit);
        assert_eq!(st.height(), 0);
        let (img, n) = st.finish();
        assert_eq!((img.width, img.height, img.bgra.len(), n), (0, 0, 0, 0));
    }

    #[test]
    fn size_mismatch_and_bad_frames_never_panic() {
        let pg = page(W, 2000, 10);
        let mut st = Stitcher::new(W, H);
        // Chưa có khung nào mà khung sai cỡ.
        assert_eq!(
            st.push(&frame(&pg, 0, 600)),
            Push::Uncertain(Reason::SizeMismatch)
        );
        assert_eq!(st.push(&frame(&pg, 0, H)), Push::Appended(H));
        assert_eq!(
            st.push(&frame(&pg, 0, 600)),
            Push::Uncertain(Reason::SizeMismatch)
        );
        let empty = Image {
            width: 0,
            height: 0,
            bgra: Vec::new(),
        };
        assert_eq!(st.push(&empty), Push::Uncertain(Reason::SizeMismatch));
        // Đúng cỡ khai báo nhưng thiếu byte.
        let broken = Image {
            width: W,
            height: H,
            bgra: vec![0; 10],
        };
        assert_eq!(st.push(&broken), Push::Uncertain(Reason::SizeMismatch));
        assert_eq!(st.height(), H);
        assert_eq!(st.finish().0, frame(&pg, 0, H));
    }

    #[test]
    fn zero_sized_stitcher_never_panics() {
        for (w, h) in [(0, 0), (0, 10), (10, 0)] {
            let mut st = Stitcher::new(w, h);
            let empty = Image {
                width: w,
                height: h,
                bgra: Vec::new(),
            };
            assert_eq!(st.push(&empty), Push::Uncertain(Reason::SizeMismatch));
            assert_eq!(st.height(), 0);
            let (img, n) = st.finish();
            assert_eq!((img.width, img.height, img.bgra.len(), n), (0, 0, 0, 0));
        }
        let (img, n) = Stitcher::new(W, H).finish();
        assert_eq!((img.width, img.height, n), (0, 0, 0));
    }

    #[test]
    fn small_frames_never_panic() {
        // Khung rất thấp: vùng chạy nhỏ hơn chồng lấp tối thiểu.
        for h in 1..40 {
            let pg = page(8, 400, 11);
            let mut st = Stitcher::new(8, h);
            for o in [0, 1, 3, 2, 30, 31] {
                let _ = st.push(&frame(&pg, o, h));
            }
            let (img, _) = st.finish();
            assert!(img.is_valid());
            assert_eq!(img.bgra, rows(&pg, 0, img.height));
        }
    }

    /// Đo thời gian một lần `push` với khung 1920×1080 (chạy bằng `--release -- --ignored`).
    #[test]
    #[ignore]
    fn perf_1920x1080() {
        let pg = page(1920, 3000, 12);
        let mut st = Stitcher::new(1920, 1080);
        st.push(&frame(&pg, 0, 1080));
        let f = frame(&pg, 90, 1080);
        let t = std::time::Instant::now();
        assert_eq!(st.push(&f), Push::Appended(90));
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        println!("push 1920x1080: {ms:.2} ms");
        assert!(ms < 15.0, "{ms} ms");
    }
}
