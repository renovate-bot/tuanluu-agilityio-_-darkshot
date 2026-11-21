//! Ghi vùng màn hình thành GIF động: thu nhỏ, mã hóa, tệp tạm và luồng ghi nền.
//!
//! Thuần Rust, không phụ thuộc `windows`; lớp Windows chỉ cấp nguồn chụp và hàm đánh thức.

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU8;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::i18n::Lang;
use crate::image::Image;

pub const FPS: u32 = 10;
pub const FRAME_INTERVAL_MS: u64 = 100;
pub const MAX_DURATION_MS: u64 = 15_000;
pub const MAX_FRAME_PIXELS: u64 = 1_000_000;
pub const MAX_SOURCE_PIXELS: u64 = 64_000_000;
pub const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;
pub const QUANT_SPEED: i32 = 10;
pub const MIN_DELAY_CS: u16 = 2;
/// Cạnh nhỏ nhất của vùng ghi (pixel).
pub const MIN_SIDE: u32 = 2;
/// Cạnh lớn nhất của một khung GIF.
pub const MAX_SIDE: u32 = 65_535;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GifError {
    InvalidRegion,
    TooLarge,
    NoFrames,
    Capture(String),
    Encode(String),
    Io(String),
    Thread(String),
}

impl GifError {
    pub fn message(&self, _lang: Lang) -> String {
        todo!()
    }
}

pub fn bgra_to_rgba(_src: &[u8], _w: u32, _h: u32) -> Result<Vec<u8>, GifError> {
    todo!()
}

pub fn frame_size(_w: u32, _h: u32) -> Result<(u16, u16), GifError> {
    todo!()
}

pub fn scale_bgra(_img: &Image, _w: u16, _h: u16) -> Result<Image, GifError> {
    todo!()
}

/// Mốc chụp kế tiếp (ms) sau thời điểm `elapsed_ms`.
fn next_tick(_elapsed_ms: u64, _interval_ms: u64) -> u64 {
    todo!()
}

pub struct GifWriter<W: Write> {
    _w: Option<W>,
}

impl<W: Write> GifWriter<W> {
    pub fn new(_w: u16, _h: u16, _out: W) -> Result<Self, GifError> {
        todo!()
    }
    pub fn push(&mut self, _rgba: Vec<u8>, _at_ms: u64) -> Result<(), GifError> {
        todo!()
    }
    pub fn finish(self, _end_ms: u64) -> Result<W, GifError> {
        todo!()
    }
    pub fn frames(&self) -> u32 {
        todo!()
    }
    pub fn bytes_written(&self) -> u64 {
        todo!()
    }
}

pub struct TempGif {
    _file: Option<BufWriter<File>>,
    _path: PathBuf,
}

impl TempGif {
    pub fn create(_dir: &Path) -> Result<TempGif, GifError> {
        todo!()
    }
    pub fn path(&self) -> &Path {
        todo!()
    }
    pub fn commit(self, _dir: &Path, _name: &str) -> Result<PathBuf, GifError> {
        todo!()
    }
}

impl Write for TempGif {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        todo!()
    }
    fn flush(&mut self) -> io::Result<()> {
        todo!()
    }
}

fn place_final<L>(_tmp: &Path, _dir: &Path, _name: &str, _link: L) -> Result<PathBuf, GifError>
where
    L: FnMut(&Path, &Path) -> io::Result<()>,
{
    todo!()
}

pub fn sweep_stale(_dir: &Path) -> usize {
    todo!()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordConfig {
    pub dir: PathBuf,
    pub file_name: String,
    pub interval_ms: u64,
    pub max_duration_ms: u64,
    pub max_file_bytes: u64,
}

impl RecordConfig {
    pub fn new(_dir: PathBuf, _file_name: String) -> Self {
        todo!()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason {
    Stopped,
    MaxDuration,
    MaxSize,
    SourceLost,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Saved {
        path: PathBuf,
        reason: StopReason,
        frames: u32,
    },
    Cancelled,
    Failed(GifError),
}

#[allow(dead_code)]
struct Shared {
    state: AtomicU8,
    lock: Mutex<()>,
    cv: Condvar,
}

#[allow(dead_code)]
pub struct Recorder {
    shared: Arc<Shared>,
    rx: Receiver<Outcome>,
    done: bool,
    started: Instant,
    max_duration_ms: u64,
    _thread: Option<JoinHandle<()>>,
}

impl Recorder {
    pub fn start<S, W>(_cfg: RecordConfig, _source: S, _wake: W) -> Result<Recorder, GifError>
    where
        S: FnMut() -> Result<Image, String> + Send + 'static,
        W: Fn() + Send + 'static,
    {
        todo!()
    }
    pub fn request_stop(&self) -> bool {
        todo!()
    }
    pub fn request_cancel(&self) -> bool {
        todo!()
    }
    pub fn try_outcome(&mut self) -> Option<Outcome> {
        todo!()
    }
    pub fn join(&mut self, _timeout: Duration) -> Option<Outcome> {
        todo!()
    }
    pub fn elapsed_ms(&self) -> u64 {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{t, Msg};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;

    fn temp_dir() -> PathBuf {
        let mut b = [0u8; 8];
        getrandom::fill(&mut b).unwrap();
        let name: String = b.iter().map(|x| format!("{x:02x}")).collect();
        let d = std::env::temp_dir().join(format!("darkshot_gif_test_{name}"));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    }

    fn no_part(dir: &Path) {
        let n = names(dir);
        assert!(n.iter().all(|x| !x.ends_with(".part")), "{n:?}");
    }

    /// Ảnh BGRA một màu.
    fn solid(w: u32, h: u32, b: u8, g: u8, r: u8) -> Image {
        let mut bgra = Vec::with_capacity((w * h * 4) as usize);
        for _ in 0..w * h {
            bgra.extend_from_slice(&[b, g, r, 0]);
        }
        Image {
            width: w,
            height: h,
            bgra,
        }
    }

    /// RGBA một màu, alpha 255.
    fn rgba_solid(w: u16, h: u16, r: u8, g: u8, b: u8) -> Vec<u8> {
        let n = usize::from(w) * usize::from(h);
        let mut v = Vec::with_capacity(n * 4);
        for _ in 0..n {
            v.extend_from_slice(&[r, g, b, 255]);
        }
        v
    }

    /// RGBA dạng gradient hai chiều cộng "chữ" (khối tối), nhiều hơn 256 màu.
    fn rgba_busy(w: u16, h: u16, seed: u32) -> Vec<u8> {
        let (w, h) = (u32::from(w), u32::from(h));
        let mut v = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                let glyph = (x / 6 + y / 10 + seed) % 7 == 0 && (y % 10) < 7 && (x % 6) < 4;
                if glyph {
                    v.extend_from_slice(&[20, 20, 24, 255]);
                } else {
                    let r = (x * 255 / w.max(1)) as u8;
                    let g = (y * 255 / h.max(1)) as u8;
                    let b = ((x + y + seed * 13) % 256) as u8;
                    v.extend_from_slice(&[r, g, b, 255]);
                }
            }
        }
        v
    }

    struct Decoded {
        width: u16,
        height: u16,
        repeat: gif::Repeat,
        /// (độ trễ cs, RGBA của khung)
        frames: Vec<(u16, Vec<u8>)>,
    }

    fn decode(bytes: &[u8]) -> Decoded {
        let mut opts = gif::DecodeOptions::new();
        opts.set_color_output(gif::ColorOutput::RGBA);
        let mut d = opts.read_info(bytes).unwrap();
        let (width, height) = (d.width(), d.height());
        let mut frames = Vec::new();
        while let Some(f) = d.read_next_frame().unwrap() {
            assert_eq!((f.left, f.top), (0, 0));
            assert_eq!((f.width, f.height), (width, height));
            frames.push((f.delay, f.buffer.to_vec()));
        }
        Decoded {
            width,
            height,
            repeat: d.repeat(),
            frames,
        }
    }

    fn close(a: &[u8], b: &[u8], tol: u8) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.abs_diff(*y) <= tol)
    }

    // --- đổi màu, kích thước ---

    #[test]
    fn bgra_to_rgba_swaps_and_makes_opaque() {
        let src = [1u8, 2, 3, 0, 10, 20, 30, 77];
        assert_eq!(
            bgra_to_rgba(&src, 2, 1).unwrap(),
            vec![3, 2, 1, 255, 30, 20, 10, 255]
        );
    }

    #[test]
    fn bgra_to_rgba_rejects_bad_length_and_overflow() {
        assert_eq!(bgra_to_rgba(&[0u8; 7], 2, 1), Err(GifError::InvalidRegion));
        assert_eq!(bgra_to_rgba(&[0u8; 8], 1, 1), Err(GifError::InvalidRegion));
        assert_eq!(bgra_to_rgba(&[], 0, 0), Err(GifError::InvalidRegion));
        assert_eq!(
            bgra_to_rgba(&[0u8; 4], u32::MAX, u32::MAX),
            Err(GifError::TooLarge)
        );
    }

    #[test]
    fn frame_size_limits() {
        assert_eq!(frame_size(1, 100), Err(GifError::InvalidRegion));
        assert_eq!(frame_size(100, 1), Err(GifError::InvalidRegion));
        assert_eq!(frame_size(0, 0), Err(GifError::InvalidRegion));
        assert_eq!(frame_size(2, 2), Ok((2, 2)));
        assert_eq!(frame_size(1000, 1000), Ok((1000, 1000)));
        assert_eq!(frame_size(800, 600), Ok((800, 600)));
        // 4K: s = sqrt(1e6 / 8 294 400).
        assert_eq!(frame_size(3840, 2160), Ok((1333, 750)));
        assert_eq!(frame_size(8000, 8000), Ok((1000, 1000)));
        assert_eq!(frame_size(8001, 8000), Err(GifError::TooLarge));
        assert_eq!(frame_size(u32::MAX, u32::MAX), Err(GifError::TooLarge));
        // Vùng hẹp dài: cạnh GIF tối đa 65 535.
        assert_eq!(frame_size(100_000, 2), Ok((65_535, 1)));
        assert_eq!(frame_size(2, 100_000), Ok((1, 65_535)));
    }

    #[test]
    fn frame_size_never_exceeds_pixel_budget() {
        for (w, h) in [
            (1001, 1000),
            (1920, 1080),
            (2560, 1440),
            (5120, 2880),
            (7680, 4320),
            (12345, 678),
            (3, 7_000_000),
        ] {
            let (tw, th) = frame_size(w, h).unwrap();
            let px = u64::from(tw) * u64::from(th);
            assert!(px <= MAX_FRAME_PIXELS, "{w}x{h} -> {tw}x{th}");
            assert!(
                px >= MAX_FRAME_PIXELS * 9 / 10
                    || tw == 65_535
                    || th == 65_535
                    || tw == 1
                    || th == 1,
                "{w}x{h} -> {tw}x{th}"
            );
            // Giữ tỉ lệ (sai số làm tròn một pixel).
            let lhs = f64::from(tw) / f64::from(th);
            let rhs = f64::from(w) / f64::from(h);
            if tw > 1 && th > 1 && tw < 65_535 && th < 65_535 {
                assert!((lhs / rhs - 1.0).abs() < 0.01, "{w}x{h} -> {tw}x{th}");
            }
        }
    }

    #[test]
    fn scale_bgra_box_average() {
        // Nửa trái đỏ, nửa phải xanh dương, 4x2 -> 2x1.
        let mut img = solid(4, 2, 0, 0, 255);
        for y in 0..2usize {
            for x in 2..4usize {
                let i = (y * 4 + x) * 4;
                img.bgra[i..i + 4].copy_from_slice(&[255, 0, 0, 0]);
            }
        }
        let s = scale_bgra(&img, 2, 1).unwrap();
        assert_eq!((s.width, s.height), (2, 1));
        assert!(close(&s.bgra[0..3], &[0, 0, 255], 0));
        assert!(close(&s.bgra[4..7], &[255, 0, 0], 0));
        // Bàn cờ đen trắng 2x2 -> 1x1 xám 128.
        let mut c = solid(2, 2, 0, 0, 0);
        for i in [0usize, 3] {
            c.bgra[i * 4..i * 4 + 3].copy_from_slice(&[255, 255, 255]);
        }
        let g = scale_bgra(&c, 1, 1).unwrap();
        assert!(close(&g.bgra[0..3], &[128, 128, 128], 0), "{:?}", g.bgra);
    }

    #[test]
    fn scale_bgra_same_size_and_errors() {
        let img = solid(5, 3, 1, 2, 3);
        assert_eq!(scale_bgra(&img, 5, 3).unwrap(), img);
        assert_eq!(scale_bgra(&img, 0, 3), Err(GifError::InvalidRegion));
        let bad = Image {
            width: 5,
            height: 3,
            bgra: vec![0; 7],
        };
        assert_eq!(scale_bgra(&bad, 2, 2), Err(GifError::InvalidRegion));
        // Thu nhỏ 4K giữ đúng kích thước đích.
        let big = solid(3840, 2160, 9, 8, 7);
        let (tw, th) = frame_size(3840, 2160).unwrap();
        let s = scale_bgra(&big, tw, th).unwrap();
        assert_eq!((s.width, s.height), (1333, 750));
        assert!(s.is_valid());
        assert!(close(&s.bgra[0..3], &[9, 8, 7], 0));
    }

    #[test]
    fn pacer_skips_missed_ticks() {
        assert_eq!(next_tick(0, 100), 100);
        assert_eq!(next_tick(30, 100), 100);
        assert_eq!(next_tick(99, 100), 100);
        assert_eq!(next_tick(100, 100), 200);
        // Lượt chậm tới 250 ms: bỏ 100, 200; chờ 300.
        assert_eq!(next_tick(250, 100), 300);
        assert_eq!(next_tick(5, 0), 6);
    }

    // --- GifWriter ---

    fn write_frames(frames: &[(Vec<u8>, u64)], w: u16, h: u16, end: u64) -> Vec<u8> {
        let mut g = GifWriter::new(w, h, Vec::new()).unwrap();
        for (f, at) in frames {
            g.push(f.clone(), *at).unwrap();
        }
        g.finish(end).unwrap()
    }

    fn delays(d: &Decoded) -> Vec<u16> {
        d.frames.iter().map(|f| f.0).collect()
    }

    #[test]
    fn writer_basic_delays_loop_and_size() {
        let fr = [
            (rgba_solid(8, 6, 255, 0, 0), 0),
            (rgba_solid(8, 6, 0, 255, 0), 100),
            (rgba_solid(8, 6, 0, 0, 255), 200),
        ];
        let bytes = write_frames(&fr, 8, 6, 300);
        let d = decode(&bytes);
        assert_eq!((d.width, d.height), (8, 6));
        assert_eq!(d.repeat, gif::Repeat::Infinite);
        assert_eq!(delays(&d), vec![10, 10, 10]);
        for (i, (_, px)) in d.frames.iter().enumerate() {
            assert!(close(px, &fr[i].0, 0), "frame {i}");
        }
    }

    #[test]
    fn writer_delays_follow_cumulative_marks() {
        let at = [0u64, 95, 200, 305, 400];
        let fr: Vec<(Vec<u8>, u64)> = at
            .iter()
            .enumerate()
            .map(|(i, &t)| (rgba_solid(4, 4, (i * 40) as u8, 0, 0), t))
            .collect();
        let d = decode(&write_frames(&fr, 4, 4, 500));
        assert_eq!(delays(&d), vec![10, 10, 11, 9, 10]);
        assert_eq!(delays(&d).iter().map(|&x| u32::from(x)).sum::<u32>(), 50);
        // 250 ms lệch: 0, 250, 500 -> 25, 25, không trôi.
        let fr2: Vec<(Vec<u8>, u64)> = [0u64, 250, 500]
            .iter()
            .enumerate()
            .map(|(i, &t)| (rgba_solid(4, 4, 0, (i * 50) as u8, 0), t))
            .collect();
        let d2 = decode(&write_frames(&fr2, 4, 4, 750));
        assert_eq!(delays(&d2), vec![25, 25, 25]);
    }

    #[test]
    fn writer_minimum_delay_without_drift() {
        let fr: Vec<(Vec<u8>, u64)> = [0u64, 5, 10]
            .iter()
            .enumerate()
            .map(|(i, &t)| (rgba_solid(4, 4, 0, 0, (i * 60) as u8), t))
            .collect();
        let d = decode(&write_frames(&fr, 4, 4, 100));
        assert_eq!(delays(&d), vec![2, 2, 6]);
    }

    #[test]
    fn writer_merges_identical_frames() {
        let a = rgba_solid(4, 4, 10, 20, 30);
        let b = rgba_solid(4, 4, 200, 20, 30);
        let mut g = GifWriter::new(4, 4, Vec::new()).unwrap();
        g.push(a.clone(), 0).unwrap();
        g.push(a.clone(), 100).unwrap();
        assert_eq!(g.frames(), 1);
        g.push(b, 200).unwrap();
        assert_eq!(g.frames(), 2);
        g.push(a, 300).unwrap();
        assert_eq!(g.frames(), 3);
        let d = decode(&g.finish(400).unwrap());
        assert_eq!(delays(&d), vec![20, 10, 10]);
    }

    #[test]
    fn writer_no_frames_is_error() {
        let g = GifWriter::new(4, 4, Vec::new()).unwrap();
        assert_eq!(g.frames(), 0);
        assert!(matches!(g.finish(100), Err(GifError::NoFrames)));
    }

    #[test]
    fn writer_rejects_wrong_length_and_zero_size() {
        assert!(matches!(
            GifWriter::new(0, 4, Vec::new()),
            Err(GifError::InvalidRegion)
        ));
        let mut g = GifWriter::new(4, 4, Vec::new()).unwrap();
        assert_eq!(g.push(vec![0; 10], 0), Err(GifError::InvalidRegion));
    }

    #[test]
    fn writer_counts_bytes_and_quantizes_busy_frames() {
        let (w, h) = (64u16, 48u16);
        let f0 = rgba_busy(w, h, 0);
        let f1 = rgba_busy(w, h, 3);
        let mut g = GifWriter::new(w, h, Vec::new()).unwrap();
        let header = g.bytes_written();
        assert!(header > 0);
        g.push(f0.clone(), 0).unwrap();
        assert_eq!(g.bytes_written(), header, "khung chờ chưa ghi");
        g.push(f1.clone(), 100).unwrap();
        assert!(g.bytes_written() > header);
        let out = g.finish(200).unwrap();
        let d = decode(&out);
        assert_eq!(d.frames.len(), 2);
        // Lượng tử hóa 256 màu: sai số trung bình nhỏ.
        for (i, src) in [&f0, &f1].iter().enumerate() {
            let px = &d.frames[i].1;
            let err: u64 = px
                .iter()
                .zip(src.iter())
                .map(|(a, b)| u64::from(a.abs_diff(*b)))
                .sum();
            let mean = err as f64 / px.len() as f64;
            assert!(mean < 8.0, "frame {i} mean err {mean}");
        }
    }

    // --- TempGif ---

    #[test]
    fn temp_gif_name_and_drop_removes() {
        let d = temp_dir();
        let t = TempGif::create(&d).unwrap();
        let n = t.path().file_name().unwrap().to_string_lossy().into_owned();
        assert!(
            n.starts_with(".darkshot-rec-") && n.ends_with(".gif.part"),
            "{n}"
        );
        let mid = &n[".darkshot-rec-".len()..n.len() - ".gif.part".len()];
        assert_eq!(mid.len(), 16);
        assert!(mid.bytes().all(|b| b.is_ascii_alphanumeric()));
        assert_eq!(names(&d), vec![n]);
        drop(t);
        assert!(names(&d).is_empty());
    }

    #[test]
    fn temp_gif_creates_missing_dir() {
        let d = temp_dir().join("sub").join("deeper");
        let t = TempGif::create(&d).unwrap();
        assert!(t.path().exists());
    }

    #[test]
    fn temp_gif_create_fails_when_dir_is_a_file() {
        let d = temp_dir();
        let f = d.join("file");
        std::fs::write(&f, b"x").unwrap();
        assert!(matches!(TempGif::create(&f), Err(GifError::Io(_))));
    }

    #[test]
    fn temp_gif_commit_never_overwrites() {
        let d = temp_dir();
        let name = "darkshot_2026-11-21_10-00-00.gif";
        std::fs::write(d.join(name), b"old").unwrap();
        let mut t = TempGif::create(&d).unwrap();
        t.write_all(b"new").unwrap();
        let p = t.commit(&d, name).unwrap();
        assert_eq!(p, d.join("darkshot_2026-11-21_10-00-00_2.gif"));
        assert_eq!(std::fs::read(d.join(name)).unwrap(), b"old");
        assert_eq!(std::fs::read(&p).unwrap(), b"new");
        assert_eq!(names(&d).len(), 2);
        no_part(&d);
    }

    #[test]
    fn place_final_falls_back_to_rename_without_overwrite() {
        let d = temp_dir();
        let name = "a.gif";
        std::fs::write(d.join(name), b"old").unwrap();
        let tmp = d.join(".darkshot-rec-AAAAAAAAAAAAAAAA.gif.part");
        std::fs::write(&tmp, b"new").unwrap();
        let unsupported = |_: &Path, _: &Path| -> io::Result<()> {
            Err(io::Error::from(io::ErrorKind::Unsupported))
        };
        let p = place_final(&tmp, &d, name, unsupported).unwrap();
        assert_eq!(p, d.join("a_2.gif"));
        assert_eq!(std::fs::read(d.join(name)).unwrap(), b"old");
        assert_eq!(std::fs::read(&p).unwrap(), b"new");
        assert!(!tmp.exists());
    }

    #[test]
    fn place_final_retries_on_already_exists() {
        let d = temp_dir();
        let tmp = d.join(".darkshot-rec-BBBBBBBBBBBBBBBB.gif.part");
        std::fs::write(&tmp, b"new").unwrap();
        // Lần đầu giả lập một tiến trình khác vừa giành tên.
        let calls = AtomicUsize::new(0);
        let racy = |from: &Path, to: &Path| -> io::Result<()> {
            if calls.fetch_add(1, Ordering::SeqCst) == 0 {
                std::fs::write(to, b"other")?;
                return Err(io::Error::from(io::ErrorKind::AlreadyExists));
            }
            std::fs::hard_link(from, to)
        };
        let p = place_final(&tmp, &d, "x.gif", racy).unwrap();
        assert_eq!(p, d.join("x_2.gif"));
        assert_eq!(std::fs::read(d.join("x.gif")).unwrap(), b"other");
        assert_eq!(std::fs::read(&p).unwrap(), b"new");
    }

    #[test]
    fn sweep_stale_removes_only_matching_files() {
        let d = temp_dir();
        let keep = [
            "darkshot_2026-11-21_10-00-00.gif",
            ".darkshot-rec-short.gif.part",
            ".darkshot-rec-AAAAAAAAAAAAAAAA.gif",
            "x.darkshot-rec-AAAAAAAAAAAAAAAA.gif.part",
            ".darkshot-rec-AAAAAAAA-AAAAAAA.gif.part",
            "notes.txt",
        ];
        for k in keep {
            std::fs::write(d.join(k), b"k").unwrap();
        }
        std::fs::create_dir(d.join(".darkshot-rec-DDDDDDDDDDDDDDDD.gif.part")).unwrap();
        std::fs::write(d.join(".darkshot-rec-AAAAAAAAAAAAAAAA.gif.part"), b"s").unwrap();
        std::fs::write(d.join(".darkshot-rec-0123456789abcdef.gif.part"), b"s").unwrap();
        assert_eq!(sweep_stale(&d), 2);
        let mut want: Vec<String> = keep.iter().map(|s| s.to_string()).collect();
        want.push(".darkshot-rec-DDDDDDDDDDDDDDDD.gif.part".into());
        want.sort();
        assert_eq!(names(&d), want);
        assert_eq!(sweep_stale(&d.join("missing")), 0);
    }

    // --- Recorder ---

    const WAIT: Duration = Duration::from_secs(10);

    fn cfg(dir: &Path) -> RecordConfig {
        RecordConfig {
            dir: dir.to_path_buf(),
            file_name: "darkshot_2026-11-21_10-00-00.gif".into(),
            interval_ms: 10,
            max_duration_ms: 200,
            max_file_bytes: MAX_FILE_BYTES,
        }
    }

    /// Nguồn đổi màu mỗi lần chụp, 16x12.
    fn counting_source() -> impl FnMut() -> Result<Image, String> + Send + 'static {
        let mut k = 0u32;
        move || {
            k += 1;
            Ok(solid(16, 12, (k * 7) as u8, (k * 13) as u8, (k * 29) as u8))
        }
    }

    fn wake_counter() -> (Arc<AtomicUsize>, impl Fn() + Send + 'static) {
        let n = Arc::new(AtomicUsize::new(0));
        let n2 = n.clone();
        (n, move || {
            n2.fetch_add(1, Ordering::SeqCst);
        })
    }

    fn wait_wakes(n: &AtomicUsize, want: usize) {
        let until = Instant::now() + WAIT;
        while n.load(Ordering::SeqCst) < want && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(n.load(Ordering::SeqCst), want);
    }

    #[test]
    fn record_config_defaults() {
        let c = RecordConfig::new(PathBuf::from("d"), "n.gif".into());
        assert_eq!(c.interval_ms, FRAME_INTERVAL_MS);
        assert_eq!(c.max_duration_ms, MAX_DURATION_MS);
        assert_eq!(c.max_file_bytes, MAX_FILE_BYTES);
        assert_eq!(FRAME_INTERVAL_MS * u64::from(FPS), 1000);
    }

    #[test]
    fn recorder_runs_to_max_duration() {
        let d = temp_dir();
        let (wakes, wake) = wake_counter();
        let mut r = Recorder::start(cfg(&d), counting_source(), wake).unwrap();
        let out = r.join(WAIT).expect("outcome");
        let Outcome::Saved {
            path,
            reason,
            frames,
        } = out
        else {
            panic!("{out:?}");
        };
        assert_eq!(reason, StopReason::MaxDuration);
        assert_eq!(path, d.join("darkshot_2026-11-21_10-00-00.gif"));
        assert!(frames >= 2, "{frames}");
        let dec = decode(&std::fs::read(&path).unwrap());
        assert_eq!(dec.frames.len() as u32, frames);
        assert_eq!((dec.width, dec.height), (16, 12));
        assert_eq!(dec.repeat, gif::Repeat::Infinite);
        // Khung cuối kéo tới đúng mốc hết giờ: tổng độ trễ = 200 ms.
        let sum: u32 = delays(&dec).iter().map(|&x| u32::from(x)).sum();
        assert_eq!(sum, 20);
        assert_eq!(
            names(&d),
            vec!["darkshot_2026-11-21_10-00-00.gif".to_string()]
        );
        wait_wakes(&wakes, 1);
        assert_eq!(r.try_outcome(), None, "chỉ trả một lần");
        assert_eq!(r.join(Duration::from_millis(10)), None);
        assert!(!r.request_cancel(), "hủy sau khi tự dừng không có tác dụng");
        assert!(!r.request_stop());
        assert_eq!(r.elapsed_ms(), 200);
        std::thread::sleep(Duration::from_millis(30));
        assert_eq!(wakes.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn recorder_creates_temp_before_returning_then_stop_saves() {
        let d = temp_dir();
        let (gate_tx, gate_rx) = mpsc::channel::<()>();
        let mut first = true;
        let source = move || {
            if first {
                first = false;
                gate_rx.recv_timeout(WAIT).map_err(|e| e.to_string())?;
            }
            Ok(solid(10, 10, 1, 2, 3))
        };
        let mut c = cfg(&d);
        c.max_duration_ms = 60_000;
        let (wakes, wake) = wake_counter();
        let mut r = Recorder::start(c, source, wake).unwrap();
        let n = names(&d);
        assert_eq!(n.len(), 1, "{n:?}");
        assert!(n[0].ends_with(".gif.part"));
        gate_tx.send(()).unwrap();
        std::thread::sleep(Duration::from_millis(50));
        assert!(r.request_stop());
        assert!(!r.request_stop(), "dừng lần hai không đổi gì");
        assert!(!r.request_cancel(), "hủy sau dừng không có tác dụng");
        let out = r.join(WAIT).expect("outcome");
        let Outcome::Saved {
            path,
            reason,
            frames,
        } = out
        else {
            panic!("{out:?}");
        };
        assert_eq!(reason, StopReason::Stopped);
        assert_eq!(frames, 1, "khung giống hệt được gộp");
        let dec = decode(&std::fs::read(&path).unwrap());
        assert_eq!(dec.frames.len(), 1);
        assert!(dec.frames[0].0 >= MIN_DELAY_CS);
        assert_eq!(
            names(&d),
            vec!["darkshot_2026-11-21_10-00-00.gif".to_string()]
        );
        wait_wakes(&wakes, 1);
    }

    #[test]
    fn recorder_cancel_leaves_nothing() {
        let d = temp_dir();
        let mut c = cfg(&d);
        c.max_duration_ms = 60_000;
        let (wakes, wake) = wake_counter();
        let mut r = Recorder::start(c, counting_source(), wake).unwrap();
        std::thread::sleep(Duration::from_millis(50));
        assert!(r.request_cancel());
        assert!(!r.request_cancel());
        assert!(!r.request_stop(), "dừng sau hủy không có tác dụng");
        assert_eq!(r.join(WAIT), Some(Outcome::Cancelled));
        assert!(names(&d).is_empty(), "{:?}", names(&d));
        wait_wakes(&wakes, 1);
    }

    #[test]
    fn recorder_stop_wakes_waiting_thread_promptly() {
        let d = temp_dir();
        let mut c = cfg(&d);
        c.interval_ms = 5_000;
        c.max_duration_ms = 60_000;
        let (_w, wake) = wake_counter();
        let mut r = Recorder::start(c, counting_source(), wake).unwrap();
        std::thread::sleep(Duration::from_millis(50));
        let t0 = Instant::now();
        assert!(r.request_stop());
        let out = r.join(WAIT).expect("outcome");
        assert!(t0.elapsed() < Duration::from_secs(2), "{:?}", t0.elapsed());
        assert!(
            matches!(
                out,
                Outcome::Saved {
                    reason: StopReason::Stopped,
                    frames: 1,
                    ..
                }
            ),
            "{out:?}"
        );
    }

    #[test]
    fn recorder_source_fails_immediately() {
        let d = temp_dir();
        let (wakes, wake) = wake_counter();
        let mut r = Recorder::start(cfg(&d), || Err("boom".to_string()), wake).unwrap();
        assert_eq!(
            r.join(WAIT),
            Some(Outcome::Failed(GifError::Capture("boom".into())))
        );
        assert!(names(&d).is_empty());
        wait_wakes(&wakes, 1);
    }

    #[test]
    fn recorder_source_fails_after_frames_saves_partial() {
        let d = temp_dir();
        let mut k = 0u32;
        let source = move || {
            k += 1;
            if k > 3 {
                return Err("gone".into());
            }
            Ok(solid(8, 8, (k * 50) as u8, 0, 0))
        };
        let mut r = Recorder::start(cfg(&d), source, || {}).unwrap();
        let out = r.join(WAIT).expect("outcome");
        let Outcome::Saved {
            path,
            reason,
            frames,
        } = out
        else {
            panic!("{out:?}");
        };
        assert_eq!((reason, frames), (StopReason::SourceLost, 3));
        assert_eq!(decode(&std::fs::read(&path).unwrap()).frames.len(), 3);
        no_part(&d);
    }

    #[test]
    fn recorder_size_change_is_source_lost() {
        let d = temp_dir();
        let mut k = 0u32;
        let source = move || {
            k += 1;
            if k >= 3 {
                return Ok(solid(9, 8, 0, 0, 0));
            }
            Ok(solid(8, 8, (k * 50) as u8, 0, 0))
        };
        let mut r = Recorder::start(cfg(&d), source, || {}).unwrap();
        let out = r.join(WAIT).expect("outcome");
        assert!(
            matches!(
                out,
                Outcome::Saved {
                    reason: StopReason::SourceLost,
                    frames: 2,
                    ..
                }
            ),
            "{out:?}"
        );
        no_part(&d);
    }

    #[test]
    fn recorder_invalid_first_frame_fails() {
        let d = temp_dir();
        let mut r = Recorder::start(cfg(&d), || Ok(solid(1, 8, 0, 0, 0)), || {}).unwrap();
        assert_eq!(r.join(WAIT), Some(Outcome::Failed(GifError::InvalidRegion)));
        assert!(names(&d).is_empty());
    }

    #[test]
    fn recorder_scales_large_frames() {
        let d = temp_dir();
        let mut c = cfg(&d);
        c.max_duration_ms = 30;
        let mut r = Recorder::start(c, || Ok(solid(2000, 1000, 5, 6, 7)), || {}).unwrap();
        let Some(Outcome::Saved { path, .. }) = r.join(WAIT) else {
            panic!();
        };
        let dec = decode(&std::fs::read(&path).unwrap());
        assert_eq!((dec.width, dec.height), frame_size(2000, 1000).unwrap());
    }

    #[test]
    fn recorder_max_file_bytes_stops_with_max_size() {
        let d = temp_dir();
        let mut c = cfg(&d);
        c.max_duration_ms = 60_000;
        c.max_file_bytes = 6_000;
        let mut k = 0u32;
        let source = move || {
            k += 1;
            let rgba = rgba_busy(64, 48, k);
            // RGBA -> BGRA
            let mut bgra = rgba;
            for p in bgra.chunks_exact_mut(4) {
                p.swap(0, 2);
            }
            Ok(Image {
                width: 64,
                height: 48,
                bgra,
            })
        };
        let mut r = Recorder::start(c, source, || {}).unwrap();
        let out = r.join(WAIT).expect("outcome");
        let Outcome::Saved {
            path,
            reason,
            frames,
        } = out
        else {
            panic!("{out:?}");
        };
        assert_eq!(reason, StopReason::MaxSize);
        assert!(frames >= 2);
        assert_eq!(
            decode(&std::fs::read(&path).unwrap()).frames.len() as u32,
            frames
        );
        no_part(&d);
    }

    #[test]
    fn recorder_start_fails_on_bad_dir() {
        let d = temp_dir();
        let f = d.join("file");
        std::fs::write(&f, b"x").unwrap();
        let mut c = cfg(&d);
        c.dir = f;
        let (wakes, wake) = wake_counter();
        assert!(matches!(
            Recorder::start(c, counting_source(), wake),
            Err(GifError::Io(_))
        ));
        assert_eq!(names(&d), vec!["file".to_string()]);
        assert_eq!(wakes.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn recorder_drop_cancels_and_cleans_up() {
        let d = temp_dir();
        let mut c = cfg(&d);
        c.max_duration_ms = 60_000;
        let (wakes, wake) = wake_counter();
        let r = Recorder::start(c, counting_source(), wake).unwrap();
        std::thread::sleep(Duration::from_millis(30));
        drop(r);
        wait_wakes(&wakes, 1);
        // Kết quả gửi trước khi đánh thức; tệp tạm đã xóa trước khi gửi.
        assert!(names(&d).is_empty(), "{:?}", names(&d));
    }

    #[test]
    fn recorder_source_panic_is_failed() {
        let d = temp_dir();
        let mut r = Recorder::start(
            cfg(&d),
            || -> Result<Image, String> { panic!("source panicked") },
            || {},
        )
        .unwrap();
        assert!(matches!(
            r.join(WAIT),
            Some(Outcome::Failed(GifError::Thread(_)))
        ));
        assert!(names(&d).is_empty());
    }

    #[test]
    fn recorder_existing_name_gets_suffix() {
        let d = temp_dir();
        std::fs::write(d.join("darkshot_2026-11-21_10-00-00.gif"), b"old").unwrap();
        let mut c = cfg(&d);
        c.max_duration_ms = 30;
        let mut r = Recorder::start(c, counting_source(), || {}).unwrap();
        let Some(Outcome::Saved { path, .. }) = r.join(WAIT) else {
            panic!();
        };
        assert_eq!(path, d.join("darkshot_2026-11-21_10-00-00_2.gif"));
        assert_eq!(
            std::fs::read(d.join("darkshot_2026-11-21_10-00-00.gif")).unwrap(),
            b"old"
        );
        no_part(&d);
    }

    // --- chữ ---

    #[test]
    fn error_messages_both_languages() {
        let all = [
            GifError::InvalidRegion,
            GifError::TooLarge,
            GifError::NoFrames,
            GifError::Capture("c-detail".into()),
            GifError::Encode("e-detail".into()),
            GifError::Io("i-detail".into()),
            GifError::Thread("t-detail".into()),
        ];
        for l in Lang::ALL {
            for e in &all {
                let m = e.message(l);
                assert!(!m.trim().is_empty(), "{e:?} {l:?}");
                if let GifError::Capture(s)
                | GifError::Encode(s)
                | GifError::Io(s)
                | GifError::Thread(s) = e
                {
                    assert!(m.contains(s.as_str()), "{m}");
                }
            }
            assert_eq!(
                GifError::InvalidRegion.message(l),
                t(l, Msg::RecordInvalidRegion)
            );
            assert!(GifError::Encode("x".into())
                .message(l)
                .starts_with(t(l, Msg::RecordEncodeFailed)));
        }
        assert_ne!(
            GifError::TooLarge.message(Lang::Vi),
            GifError::TooLarge.message(Lang::En)
        );
    }

    /// Đo thời gian mã hóa một khung 1000x1000. Chạy:
    /// `cargo test --release -p darkshot-core gif_record::tests::perf_encode_1mp -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn perf_encode_1mp() {
        let (w, h) = (1000u16, 1000u16);
        let frames: Vec<Vec<u8>> = (0..6).map(|k| rgba_busy(w, h, k)).collect();
        let mut g = GifWriter::new(w, h, Vec::new()).unwrap();
        g.push(frames[0].clone(), 0).unwrap();
        let mut times = Vec::new();
        for (i, f) in frames.iter().enumerate().skip(1) {
            let t0 = Instant::now();
            g.push(f.clone(), i as u64 * 100).unwrap();
            times.push(t0.elapsed());
        }
        let t0 = Instant::now();
        let out = g.finish(600).unwrap();
        times.push(t0.elapsed());
        let ms: Vec<f64> = times.iter().map(|d| d.as_secs_f64() * 1000.0).collect();
        let avg = ms.iter().sum::<f64>() / ms.len() as f64;
        let max = ms.iter().cloned().fold(0.0, f64::max);
        println!("encode 1000x1000 speed {QUANT_SPEED}: per frame ms {ms:?}, avg {avg:.1}, max {max:.1}, file {} bytes", out.len());
        // Thu nhỏ 4K -> 1333x750, rồi đổi RGBA.
        let big = solid(3840, 2160, 1, 2, 3);
        let t0 = Instant::now();
        let (tw, th) = frame_size(3840, 2160).unwrap();
        let s = scale_bgra(&big, tw, th).unwrap();
        let _ = bgra_to_rgba(&s.bgra, s.width, s.height).unwrap();
        println!(
            "scale 3840x2160 -> {tw}x{th} + rgba: {:.1} ms",
            t0.elapsed().as_secs_f64() * 1000.0
        );
    }
}
