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
    /// Nội dung dịch xuống dy hàng và khung đã được nối (khung đầu: H). Số hàng thêm vào ảnh
    /// có thể lớn hơn dy khi dải tĩnh dưới thu nhỏ; xem `height()`.
    Appended(u32),
    /// Khung bị bỏ.
    Uncertain(Reason),
    /// Đã đạt giới hạn chiều cao hoặc bộ nhớ; không nhận thêm khung.
    Limit,
}

/// Độ lệch điểm cho phép giữa dy tốt nhất và dy khác trước khi coi là mơ hồ.
const AMBIGUITY_MARGIN: f64 = 0.005;

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// FNV-1a 64 bit trên các byte B, G, R của một hàng BGRA (bỏ alpha).
fn row_hash(row: &[u8]) -> u64 {
    let mut h = FNV_OFFSET;
    for px in row.as_chunks::<4>().0 {
        for &b in &px[..3] {
            h ^= u64::from(b);
            h = h.wrapping_mul(FNV_PRIME);
        }
    }
    h
}

/// Đúng khi mọi điểm ảnh của hàng có cùng B, G, R.
fn row_uniform(row: &[u8]) -> bool {
    match row.get(..3) {
        Some(first) => row.as_chunks::<4>().0.iter().all(|px| &px[..3] == first),
        None => true,
    }
}

/// Băm FNV-1a (như `row_hash`) bốn hàng cùng độ dài một lúc: bốn chuỗi nhân độc lập chạy song
/// song trong bộ xử lý, nhanh hơn băm lần lượt từng hàng.
fn row_hash4(rows: [&[u8]; 4]) -> [u64; 4] {
    #[inline(always)]
    fn step(h: u64, b: u8) -> u64 {
        (h ^ u64::from(b)).wrapping_mul(FNV_PRIME)
    }
    let (mut h0, mut h1, mut h2, mut h3) = (FNV_OFFSET, FNV_OFFSET, FNV_OFFSET, FNV_OFFSET);
    let [a, b, c, d] = rows;
    for (((pa, pb), pc), pd) in a
        .as_chunks::<4>()
        .0
        .iter()
        .zip(b.as_chunks::<4>().0)
        .zip(c.as_chunks::<4>().0)
        .zip(d.as_chunks::<4>().0)
    {
        for k in 0..3 {
            h0 = step(h0, pa[k]);
            h1 = step(h1, pb[k]);
            h2 = step(h2, pc[k]);
            h3 = step(h3, pd[k]);
        }
    }
    [h0, h1, h2, h3]
}

/// Hai hàng BGRA cùng B, G, R ở mọi điểm ảnh.
fn rows_equal_bgr(a: &[u8], b: &[u8]) -> bool {
    a == b
        || (a.len() == b.len()
            && a.as_chunks::<4>()
                .0
                .iter()
                .zip(b.as_chunks::<4>().0)
                .all(|(p, q)| p[..3] == q[..3]))
}

/// Đúng khi ảnh W×H nằm trong `MAX_HEIGHT` và `MAX_BYTES`.
fn within_limits(w: u32, h: u32) -> bool {
    h <= MAX_HEIGHT
        && u64::from(w)
            .checked_mul(u64::from(h))
            .and_then(|n| n.checked_mul(4))
            .is_some_and(|n| n <= MAX_BYTES)
}

/// Số hàng tối đa của ảnh ghép rộng `w`.
fn max_rows(w: u32) -> u32 {
    let by_bytes = MAX_BYTES / (u64::from(w.max(1)) * 4);
    u32::try_from(by_bytes).unwrap_or(u32::MAX).min(MAX_HEIGHT)
}

/// Chồng lấp tối thiểu (hàng) cho vùng chạy cao `run_h`.
fn min_overlap(run_h: u32) -> u32 {
    let frac = (f64::from(run_h) * MIN_OVERLAP_FRAC).ceil() as u32;
    MIN_OVERLAP_PX.max(frac)
}

/// Bề rộng dải phải (cột) không dùng để so khớp: chỗ của thanh cuộn dọc, con trượt di chuyển
/// theo nội dung nên làm hàng khác nhau dù nội dung khớp. Ảnh hẹp hơn 128 px: không bỏ cột nào.
fn right_strip(w: u32) -> u32 {
    if w >= 128 {
        (w / 8).min(32)
    } else {
        0
    }
}

/// Một khung đã băm: điểm ảnh, băm và cờ một màu của phần chính mỗi hàng (bỏ dải phải).
struct Frame {
    bgra: Vec<u8>,
    hash: Vec<u64>,
    uniform: Vec<bool>,
    /// Số byte đầu mỗi hàng thuộc phần chính.
    main: usize,
}

impl Frame {
    /// `img` phải hợp lệ (đã kiểm `is_valid`).
    fn new(img: &Image) -> Frame {
        let stride = img.width as usize * 4;
        let main = (img.width - right_strip(img.width)) as usize * 4;
        let rows: Vec<&[u8]> = img.bgra.chunks_exact(stride).map(|r| &r[..main]).collect();
        let mut hash = Vec::with_capacity(rows.len());
        let (groups, rest) = rows.as_chunks::<4>();
        for g in groups {
            hash.extend(row_hash4(*g));
        }
        hash.extend(rest.iter().map(|r| row_hash(r)));
        let uniform = rows.iter().map(|r| row_uniform(r)).collect();
        Frame {
            bgra: img.bgra.clone(),
            hash,
            uniform,
            main,
        }
    }

    /// Phần chính của hàng `y`.
    fn main_row(&self, y: u32, stride: usize) -> &[u8] {
        let s = y as usize * stride;
        &self.bgra[s..s + self.main]
    }

    /// Đúng khi các hàng `ys` có ít nhất `MIN_DISTINCT_ROWS` giá trị băm khác nhau, không kể
    /// hàng một màu.
    fn enough_distinct(&self, ys: impl Iterator<Item = usize>) -> bool {
        let mut seen = [0u64; MIN_DISTINCT_ROWS as usize];
        let mut k = 0;
        for y in ys {
            let h = self.hash[y];
            if self.uniform[y] || seen[..k].contains(&h) {
                continue;
            }
            seen[k] = h;
            k += 1;
            if k == seen.len() {
                return true;
            }
        }
        false
    }

    /// Khối [top, top + n) của khung này gặp lại trong vùng chạy [top, top + run_h) ở một độ dịch
    /// s ≥ 1 (nội dung lặp): so [0, n − s) với [s, n) trong khối, và khối với phần sau nó
    /// [s, s + min(n, run_h − s)). Đoạn so phải dài ≥ `MIN_OVERLAP_PX`, khớp ≥ MATCH_RATIO − 0,005
    /// và có ≥ `MIN_DISTINCT_ROWS` băm khác nhau.
    fn self_similar(&self, top: usize, n: usize, run_h: usize) -> bool {
        let floor = MATCH_RATIO - AMBIGUITY_MARGIN;
        let min = MIN_OVERLAP_PX as usize;
        for s in 1..=run_h.saturating_sub(min) {
            let inside = n.saturating_sub(s);
            let beyond = n.min(run_h - s);
            for len in [inside, beyond] {
                if len < min {
                    continue;
                }
                let win = &self.hash[top..top + s + len];
                if score(win, win, s as i64, floor).is_some()
                    && self.enough_distinct(
                        (top..top + len).filter(|&i| self.hash[i] == self.hash[i + s]),
                    )
                {
                    return true;
                }
                if inside == beyond {
                    break;
                }
            }
        }
        false
    }
}

/// Dải tĩnh ứng viên giữa hai khung cùng cỡ: số hàng đầu và số hàng cuối có băm giống nhau,
/// mỗi dải tối đa H/3.
fn static_bands(prev: &Frame, cur: &Frame) -> (u32, u32) {
    let h = cur.hash.len();
    let cap = h / 3;
    let pairs = || prev.hash.iter().zip(cur.hash.iter());
    let top = pairs().take_while(|(a, b)| a == b).count().min(cap);
    let bottom = pairs().rev().take_while(|(a, b)| a == b).count().min(cap);
    (top as u32, bottom as u32)
}

/// Hàng y ∈ [`y0`, `y1`) của khung mới là nội dung cuộn: khớp hàng y + d của khung trước, với tỉ
/// lệ ≥ MATCH_RATIO − 0,005. Hàng nằm ngoài khung (không so được) tính là không khớp.
fn moved_with(cur: &Frame, prev: &Frame, y0: i64, y1: i64, d: u32) -> bool {
    if y1 <= y0 {
        return true;
    }
    let ok = (y0..y1)
        .filter(|&y| {
            let (c, p) = (usize::try_from(y), usize::try_from(y + i64::from(d)));
            match (c, p) {
                (Ok(c), Ok(p)) => cur
                    .hash
                    .get(c)
                    .zip(prev.hash.get(p))
                    .is_some_and(|(a, b)| a == b),
                _ => false,
            }
        })
        .count();
    ok as f64 >= (MATCH_RATIO - AMBIGUITY_MARGIN) * (y1 - y0) as f64
}

/// Tỉ lệ hàng khớp khi hàng chạy `i` của khung mới ứng với hàng chạy `i + d` của khung trước
/// (`d` âm: cuộn lên). `None` khi tỉ lệ chắc chắn dưới `floor` (dừng sớm).
fn score(prev: &[u64], cur: &[u64], d: i64, floor: f64) -> Option<f64> {
    let run = cur.len() as i64;
    let n = run - d.abs();
    if n <= 0 {
        return None;
    }
    let need = (floor * n as f64).ceil() as i64;
    let max_miss = n - need;
    let (c0, p0) = if d >= 0 { (0, d) } else { (-d, 0) };
    let (c0, p0, n) = (c0 as usize, p0 as usize, n as usize);
    let mut miss = 0i64;
    for (a, b) in cur[c0..c0 + n].iter().zip(&prev[p0..p0 + n]) {
        if a != b {
            miss += 1;
            if miss > max_miss {
                return None;
            }
        }
    }
    Some((n as i64 - miss) as f64 / n as f64)
}

/// Ghép các khung cùng cỡ thành một ảnh dọc.
///
/// Ảnh ghép = `body` + hàng [`tail`, H) của khung đã nhận cuối (`prev`). `body` kết thúc đúng
/// trước hàng `tail` của `prev`, nên phần đuôi (chân trang dính) chỉ có một lần, lấy từ khung cuối.
/// Khung không ghép được bị bỏ; `prev` luôn là khung đã nhận cuối cùng, nên mọi chỗ nối đều đã
/// được xác nhận bằng vùng chồng lấp.
pub struct Stitcher {
    w: u32,
    h: u32,
    prev: Option<Frame>,
    body: Vec<u8>,
    tail: u32,
    /// Dải tĩnh (trên, dưới) nhỏ nhất đã dùng cho một lần nối.
    bands: Option<(u32, u32)>,
    appended: u32,
    uncertain: u32,
    in_gap: bool,
    limited: bool,
}

/// Kết quả so khớp một khung với khung trước.
enum Match {
    Same,
    Shift { d: u32, top: u32, bottom: u32 },
    Fail(Reason),
}

impl Stitcher {
    /// Bộ ghép cho các khung W×H.
    pub fn new(w: u32, h: u32) -> Stitcher {
        Stitcher {
            w,
            h,
            prev: None,
            body: Vec::new(),
            tail: 0,
            bands: None,
            appended: 0,
            uncertain: 0,
            in_gap: false,
            limited: false,
        }
    }

    fn stride(&self) -> usize {
        self.w as usize * 4
    }

    /// Nhận một khung.
    pub fn push(&mut self, frame: &Image) -> Push {
        if self.limited {
            return Push::Limit;
        }
        if !frame.is_valid() || frame.width != self.w || frame.height != self.h {
            return self.fail(Reason::SizeMismatch);
        }
        let Some(prev) = &self.prev else {
            return self.first(frame);
        };
        let cur = Frame::new(frame);
        match self.find_shift(prev, &cur) {
            Match::Same => Push::NoChange,
            Match::Fail(r) => self.fail(r),
            Match::Shift { d, top, bottom } => self.append(cur, d, top, bottom),
        }
    }

    fn first(&mut self, frame: &Image) -> Push {
        if !within_limits(self.w, self.h) {
            self.limited = true;
            return Push::Limit;
        }
        let f = Frame::new(frame);
        self.body = f.bgra.clone();
        self.tail = self.h;
        self.prev = Some(f);
        self.appended = 1;
        Push::Appended(self.h)
    }

    fn fail(&mut self, r: Reason) -> Push {
        if !self.in_gap && self.appended >= 1 {
            self.uncertain += 1;
        }
        self.in_gap = true;
        Push::Uncertain(r)
    }

    /// Tìm độ dịch của khung mới so với khung trước.
    fn find_shift(&self, prev: &Frame, cur: &Frame) -> Match {
        if prev.hash == cur.hash {
            return Match::Same;
        }
        let h = self.h;
        let (ct, cb) = static_bands(prev, cur);
        let (mut top, mut bottom) = match self.bands {
            Some((t, b)) => (t.min(ct), b.min(cb)),
            None => (ct, cb),
        };
        let mut run_h = h - top - bottom;
        if run_h < 2 * min_overlap(run_h) {
            (top, bottom, run_h) = (0, 0, h);
        }
        let min_ov = min_overlap(run_h);
        if run_h <= min_ov {
            return Match::Fail(Reason::NoOverlap);
        }
        let max_d = i64::from(run_h - min_ov);
        let run = top as usize..(top + run_h) as usize;
        let (p, c) = (&prev.hash[run.clone()], &cur.hash[run]);
        let floor = MATCH_RATIO - AMBIGUITY_MARGIN;
        let scored: Vec<(i64, f64)> = (-max_d..=max_d)
            .filter_map(|d| score(p, c, d, floor).map(|s| (d, s)))
            .collect();
        let Some(&(best_d, best)) = scored.iter().max_by(|a, b| a.1.total_cmp(&b.1)) else {
            return Match::Fail(Reason::NoOverlap);
        };
        if best < MATCH_RATIO {
            return Match::Fail(Reason::NoOverlap);
        }
        if scored
            .iter()
            .any(|&(d, s)| d != best_d && s >= best - AMBIGUITY_MARGIN)
        {
            return Match::Fail(Reason::Ambiguous);
        }
        if best_d < 0 {
            return Match::Fail(Reason::Backward);
        }
        if best_d == 0 {
            // Đổi cục bộ (con trỏ nhấp nháy, hiệu ứng rê chuột), nội dung không cuộn.
            return Match::Same;
        }
        let d = best_d as u32;
        if !self.confirm(prev, cur, top, run_h, d) {
            return Match::Fail(Reason::Ambiguous);
        }
        // Nội dung lặp: khối chồng lấp gặp lại ở độ dịch khác thì dy thật có thể là d + s, nằm
        // ngoài khoảng tìm (cuộn quá xa). Giới hạn còn lại, không sửa được ở mức hai khung: khối
        // lặp cao hơn vùng chồng lấp ở hai nơi cách xa nhau mà khung mới chỉ thấy một lần.
        if cur.self_similar(top as usize, (run_h - d) as usize, run_h as usize) {
            return Match::Fail(Reason::Ambiguous);
        }
        // Hàng mà cặp khung này thấy đứng yên nhưng được tính là hàng chạy (dải tĩnh đã lưu nhỏ
        // hơn): phải khớp như nội dung cuộn với độ dịch d; không khớp là thanh dính mới xuất
        // hiện (ví dụ thanh đáy mỏng), nối vào sẽ lặp nó giữa ảnh.
        // Dải trên: hàng [top, ct) của khung mới đến từ hàng y + d của khung trước. Dải dưới:
        // hàng [H − cb, H − bottom) của khung trước phải hiện ở hàng y − d của khung mới.
        let (h64, d64) = (i64::from(h), i64::from(d));
        let (ct64, cb64, top64, bottom64) = (
            i64::from(ct),
            i64::from(cb),
            i64::from(top),
            i64::from(bottom),
        );
        if !moved_with(cur, prev, top64, ct64, d)
            || !moved_with(cur, prev, h64 - cb64 - d64, h64 - bottom64 - d64, d)
        {
            return Match::Fail(Reason::NoOverlap);
        }
        Match::Shift { d, top, bottom }
    }

    /// Kiểm hàng khớp theo băm bằng so điểm ảnh (phần chính), và đủ băm khác nhau.
    fn confirm(&self, prev: &Frame, cur: &Frame, top: u32, run_h: u32, d: u32) -> bool {
        let stride = self.stride();
        let matched: Vec<usize> = (top..top + run_h - d)
            .filter(|&c| cur.hash[c as usize] == prev.hash[(c + d) as usize])
            .map(|c| c as usize)
            .collect();
        matched.iter().all(|&c| {
            let c = c as u32;
            rows_equal_bgr(cur.main_row(c, stride), prev.main_row(c + d, stride))
        }) && cur.enough_distinct(matched.into_iter())
    }

    /// Nối khung đã khớp với độ dịch `d` và dải tĩnh `top`, `bottom`.
    fn append(&mut self, cur: Frame, d: u32, top: u32, bottom: u32) -> Push {
        let stride = self.stride();
        let run_end = self.h - bottom;
        // Lần nối đầu: bỏ phần dưới vùng chạy của khung đầu (chân trang lấy từ khung cuối).
        let (body_rows, tail) = {
            let rows = (self.body.len() / stride.max(1)) as u32;
            if self.tail > run_end {
                if self.appended != 1 {
                    return self.fail(Reason::NoOverlap);
                }
                (rows - (self.tail - run_end), run_end)
            } else {
                (rows, self.tail)
            }
        };
        // Hàng cuối của body (hàng tail - 1 của khung trước) phải nằm trong vùng chồng lấp.
        if tail < top + d + 1 {
            return self.fail(Reason::NoOverlap);
        }
        let from = tail - d;
        let add = run_end - from;
        let total = body_rows + add + (self.h - run_end);
        if !within_limits(self.w, total) {
            self.limited = true;
            return Push::Limit;
        }
        let keep = body_rows as usize * stride;
        let need = keep + add as usize * stride;
        if need > self.body.capacity() {
            let cap_max = max_rows(self.w) as usize * stride;
            let target = (self.body.capacity() * 2).clamp(need, cap_max.max(need));
            if self
                .body
                .try_reserve_exact(target - self.body.len())
                .is_err()
            {
                self.limited = true;
                return Push::Limit;
            }
        }
        self.body.truncate(keep);
        self.body
            .extend_from_slice(&cur.bgra[from as usize * stride..run_end as usize * stride]);
        self.tail = run_end;
        self.prev = Some(cur);
        self.bands = Some((top, bottom));
        self.appended += 1;
        self.in_gap = false;
        Push::Appended(d)
    }

    /// Chiều cao ảnh ghép hiện tại (0 khi chưa có khung).
    pub fn height(&self) -> u32 {
        match self.prev {
            Some(_) => (self.body.len() / self.stride()) as u32 + (self.h - self.tail),
            None => 0,
        }
    }

    /// Số chỗ ghép không chắc chắn: mỗi chuỗi khung liên tiếp không ghép được (sau khung đầu)
    /// tính là một, kể cả khi chưa nối được lần nào.
    pub fn uncertain_count(&self) -> u32 {
        self.uncertain
    }

    /// Số khung đã góp hàng vào ảnh, kể cả khung đầu.
    pub fn appended_frames(&self) -> u32 {
        self.appended
    }

    /// Ảnh ghép và số chỗ ghép không chắc chắn. Chưa có khung: ảnh 0×0.
    pub fn finish(self) -> (Image, u32) {
        let Some(prev) = &self.prev else {
            let empty = Image {
                width: 0,
                height: 0,
                bgra: Vec::new(),
            };
            return (empty, self.uncertain);
        };
        let stride = self.stride();
        let mut bgra = self.body;
        let tail_bytes = &prev.bgra[self.tail as usize * stride..];
        // Không cấp được bộ nhớ cho phần đuôi: trả phần thân, vẫn là ảnh đúng.
        if bgra.try_reserve_exact(tail_bytes.len()).is_ok() {
            bgra.extend_from_slice(tail_bytes);
        }
        let height = (bgra.len() / stride) as u32;
        (
            Image {
                width: self.w,
                height,
                bgra,
            },
            self.uncertain,
        )
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
                        let c = if (yy / p).is_multiple_of(2) { a } else { b };
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
                            let c = if (x / p).is_multiple_of(2) { a } else { b };
                            set_px(&mut img, x, yy, c);
                        }
                    }
                }
                3 => {
                    let (a, b) = (rng.color(), rng.color());
                    let p = 4 + rng.below(12) as u32;
                    for yy in y..y + len {
                        for x in 0..w {
                            let c = if ((x / p) + (yy / p)).is_multiple_of(2) {
                                a
                            } else {
                                b
                            };
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
    fn four_row_hash_equals_single_row_hash() {
        let pg = page(37, 8, 13);
        let r: Vec<&[u8]> = pg.bgra.chunks_exact(pg.width as usize * 4).collect();
        let four = row_hash4([r[0], r[1], r[2], r[3]]);
        for k in 0..4 {
            assert_eq!(four[k], row_hash(r[k]));
        }
        let f = Frame::new(&pg);
        let single: Vec<u64> = r.iter().map(|x| row_hash(x)).collect();
        assert_eq!(f.hash, single);
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
        // Đã cuộn mà chưa nối được lần nào: vẫn tính một chỗ, để kết quả có cảnh báo.
        assert_eq!(st.uncertain_count(), 1);
        assert_eq!(st.finish().1, 1);
    }

    /// Trang W×h: `noise` hàng nhiễu rồi các "dòng" giống hệt nhau, chu kỳ `period` hàng, mỗi dòng
    /// `ink` hàng nhiễu (cùng một mẫu cho mọi dòng) rồi hàng trắng.
    fn periodic_page(w: u32, h: u32, noise: u32, period: u32, ink: u32, seed: u64) -> Image {
        let mut img = noise_page(w, h, seed);
        let line = noise_page(w, ink, seed + 1000);
        let s = (w * 4) as usize;
        for y in noise..h {
            let phase = (y - noise) % period;
            let dst = &mut img.bgra[y as usize * s..(y as usize + 1) * s];
            if phase < ink {
                dst.copy_from_slice(&line.bgra[phase as usize * s..(phase as usize + 1) * s]);
            } else {
                dst.fill(255);
            }
        }
        img
    }

    /// Nhảy vào nội dung lặp, quá xa so với khoảng tìm dy: không bao giờ được nối (mất hàng).
    #[test]
    fn jump_into_periodic_content_is_never_appended() {
        // (nhiễu, chu kỳ, hàng nhiễu mỗi dòng, offset khung thứ hai)
        let cases = [
            (580, 20, 12, 650),
            (580, 20, 12, 1250),
            (500, 100, 60, 650),
            (560, 40, 12, 630),
            (500, 100, 9, 650),
        ];
        for (noise, period, ink, off) in cases {
            let pg = periodic_page(200, 2500, noise, period, ink, 21);
            let mut st = Stitcher::new(200, H);
            assert_eq!(st.push(&frame(&pg, 0, H)), Push::Appended(H));
            let r = st.push(&frame(&pg, off, H));
            assert!(
                matches!(r, Push::Uncertain(_)),
                "{noise} {period} {ink} {off}: {r:?}"
            );
            let (img, n) = st.finish();
            assert_eq!(img, frame(&pg, 0, H));
            assert_eq!(n, 1);
        }
    }

    /// Thanh dính đáy mỏng xuất hiện giữa chừng: không bao giờ bị nối lặp vào giữa ảnh.
    #[test]
    fn late_thin_bottom_bar_is_never_repeated_inside() {
        for b in [6u32, 10, 12] {
            let pg = page(W, 6000, 30 + u64::from(b));
            let bar = noise_page(W, b, 40);
            let s = (W * 4) as usize;
            let mut st = Stitcher::new(W, H);
            for (k, o) in [0u32, 100, 200, 300, 400, 500, 600].into_iter().enumerate() {
                let mut f = frame(&pg, o, H);
                if k >= 3 {
                    f.bgra[(H - b) as usize * s..].copy_from_slice(&bar.bgra);
                }
                let _ = st.push(&f);
            }
            let (img, _) = st.finish();
            assert!(img.height > b);
            let body = img.height - b;
            assert!(
                img.bgra[..body as usize * s] == rows(&pg, 0, body),
                "bar {b}: stitched rows differ from the page"
            );
            let last = &img.bgra[body as usize * s..];
            assert!(
                last == &bar.bgra[..] || last == &rows(&pg, body, img.height)[..],
                "bar {b}: last rows"
            );
        }
    }

    /// Vùng chọn có thanh cuộn dọc 12 px ở mép phải, con trượt 100 hàng di chuyển theo nội dung.
    #[test]
    fn moving_scrollbar_thumb_does_not_block_stitching() {
        let page_h = 4000;
        let pg = page(W, page_h, 15);
        let bar_x = W - 12;
        let scrolled = |o: u32| {
            let mut f = frame(&pg, o, H);
            let ty = o * (H - 100) / (page_h - H);
            for y in 0..H {
                let c = if (ty..ty + 100).contains(&y) {
                    [90, 90, 90, 255]
                } else {
                    [30, 30, 30, 255]
                };
                for x in bar_x..W {
                    set_px(&mut f, x, y, c);
                }
            }
            f
        };
        let mut st = Stitcher::new(W, H);
        assert_eq!(st.push(&scrolled(0)), Push::Appended(H));
        let mut o = 0;
        for step in [3u32, 6, 20, 100, 3, 6, 20, 100] {
            o += step;
            assert_eq!(st.push(&scrolled(o)), Push::Appended(step), "step {step}");
        }
        let (img, n) = st.finish();
        assert_eq!(n, 0);
        assert_eq!(img.height, o + H);
        let s = (W * 4) as usize;
        let content = (bar_x * 4) as usize;
        for y in 0..img.height as usize {
            assert!(
                img.bgra[y * s..y * s + content] == pg.bgra[y * s..y * s + content],
                "row {y}"
            );
        }
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
        // Toàn nhiễu: trang hỗn hợp rộng 16 px có đoạn 200 hàng chỉ gồm sọc và ô vuông (quá ít
        // băm khác nhau), sẽ bị bỏ đúng là Ambiguous; kiểm thử này chỉ nhắm giới hạn.
        let pg = noise_page(w, MAX_HEIGHT + 2000, 9);
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
        // Trường hợp xấu nhất cho tìm dy: khung gần như một màu, mọi dy khớp đến cuối.
        let mut pg = noise_page(1920, 3000, 14);
        let s = 1920 * 4;
        for p in pg.bgra[200 * s..].chunks_mut(4) {
            p.copy_from_slice(&[1, 2, 3, 255]);
        }
        let mut st = Stitcher::new(1920, 1080);
        st.push(&frame(&pg, 0, 1080));
        let f = frame(&pg, 400, 1080);
        let t = std::time::Instant::now();
        assert_eq!(st.push(&f), Push::Uncertain(Reason::Ambiguous));
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        println!("push 1920x1080 one colour: {ms:.2} ms");
        assert!(ms < 15.0, "{ms} ms");
        // Nội dung thưa: một hàng nhiễu mỗi 20 hàng, còn lại trắng (tìm tự tương đồng lâu nhất).
        let mut pg = noise_page(1920, 3000, 16);
        for (y, row) in pg.bgra.chunks_mut(s).enumerate() {
            if y % 20 != 0 {
                row.fill(255);
            }
        }
        let mut st = Stitcher::new(1920, 1080);
        st.push(&frame(&pg, 0, 1080));
        let f = frame(&pg, 90, 1080);
        let t = std::time::Instant::now();
        assert_eq!(st.push(&f), Push::Appended(90));
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        println!("push 1920x1080 sparse: {ms:.2} ms");
        assert!(ms < 15.0, "{ms} ms");
    }
}
