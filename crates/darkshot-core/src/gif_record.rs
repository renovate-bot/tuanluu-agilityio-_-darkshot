//! Ghi vùng màn hình thành GIF động: thu nhỏ, mã hóa, tệp tạm và luồng ghi nền.
//!
//! Thuần Rust, không phụ thuộc `windows`; lớp Windows chỉ cấp nguồn chụp và hàm đánh thức.
//! Bộ nhớ: mỗi lượt tối đa một khung thô, một bản thu nhỏ/RGBA và một khung chờ; không giữ mọi
//! khung.

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, TryRecvError};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use gif::{Encoder, EncodingError, Frame, Repeat};

use crate::i18n::{self, t, Lang, Msg};
use crate::image::Image;
use crate::naming;

pub const FPS: u32 = 10;
pub const FRAME_INTERVAL_MS: u64 = 100;
pub const MAX_DURATION_MS: u64 = 15_000;
/// Đo ở Task 122: khung 1 MP nhiều màu mất ~270 ms (NeuQuant tốc độ 10); 0,5 MP ở tốc độ 20 ~95 ms.
pub const MAX_FRAME_PIXELS: u64 = 500_000;
pub const MAX_SOURCE_PIXELS: u64 = 64_000_000;
pub const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;
pub const QUANT_SPEED: i32 = 20;
pub const MIN_DELAY_CS: u16 = 2;
/// Cạnh nhỏ nhất của vùng ghi (pixel).
pub const MIN_SIDE: u32 = 2;
/// Cạnh lớn nhất của một khung GIF.
pub const MAX_SIDE: u32 = 65_535;

const TEMP_PREFIX: &str = ".darkshot-rec-";
const TEMP_SUFFIX: &str = ".gif.part";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GifError {
    /// Cạnh < `MIN_SIDE`, ảnh hỏng hoặc độ dài điểm ảnh không khớp kích thước.
    InvalidRegion,
    /// Vùng > `MAX_SOURCE_PIXELS` hoặc phép tính kích thước tràn số.
    TooLarge,
    NoFrames,
    Capture(String),
    Encode(String),
    Io(String),
    Thread(String),
    /// Lỗi do darkshot tự phát hiện (không có chữ từ hệ điều hành); chữ lấy từ `i18n`.
    Fault(GifFault),
}

/// Các lỗi nội bộ của bộ ghi GIF, mỗi lỗi một chuỗi `i18n`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GifFault {
    RandomUnavailable,
    NoTempName,
    TempClosed,
    NoFileName,
    ThreadPanicked,
    ThreadEnded,
    FrameSizeChanged,
    Internal,
}

impl GifFault {
    pub const ALL: [GifFault; 8] = [
        GifFault::RandomUnavailable,
        GifFault::NoTempName,
        GifFault::TempClosed,
        GifFault::NoFileName,
        GifFault::ThreadPanicked,
        GifFault::ThreadEnded,
        GifFault::FrameSizeChanged,
        GifFault::Internal,
    ];

    pub fn msg(self) -> Msg {
        match self {
            GifFault::RandomUnavailable => Msg::RecordFaultRandom,
            GifFault::NoTempName => Msg::RecordFaultNoTempName,
            GifFault::TempClosed => Msg::RecordFaultTempClosed,
            GifFault::NoFileName => Msg::RecordFaultNoFileName,
            GifFault::ThreadPanicked => Msg::RecordFaultPanicked,
            GifFault::ThreadEnded => Msg::RecordFaultThreadEnded,
            GifFault::FrameSizeChanged => Msg::RecordFaultSizeChanged,
            GifFault::Internal => Msg::RecordFaultInternal,
        }
    }
}

impl GifError {
    pub fn message(&self, lang: Lang) -> String {
        match self {
            GifError::InvalidRegion => t(lang, Msg::RecordInvalidRegion).to_string(),
            GifError::TooLarge => t(lang, Msg::RecordTooLarge).to_string(),
            GifError::NoFrames => t(lang, Msg::RecordNoFrames).to_string(),
            GifError::Capture(e) => i18n::record_capture_failed(lang, e),
            GifError::Encode(e) => i18n::record_encode_failed(lang, e),
            GifError::Io(e) => i18n::record_io_failed(lang, e),
            GifError::Thread(e) => i18n::record_thread_failed(lang, e),
            GifError::Fault(f) => t(lang, f.msg()).to_string(),
        }
    }
}

fn io_err(e: io::Error) -> GifError {
    GifError::Io(e.to_string())
}

fn enc_err(e: EncodingError) -> GifError {
    match e {
        EncodingError::Io(e) => io_err(e),
        other => GifError::Encode(other.to_string()),
    }
}

/// Số byte của ảnh 4 byte mỗi pixel, có kiểm tra tràn.
fn pixel_bytes(w: u32, h: u32) -> Result<usize, GifError> {
    let w = usize::try_from(w).map_err(|_| GifError::TooLarge)?;
    let h = usize::try_from(h).map_err(|_| GifError::TooLarge)?;
    w.checked_mul(h)
        .and_then(|n| n.checked_mul(4))
        .ok_or(GifError::TooLarge)
}

fn alloc(n: usize) -> Result<Vec<u8>, GifError> {
    let mut v = Vec::new();
    v.try_reserve_exact(n).map_err(|_| GifError::TooLarge)?;
    Ok(v)
}

/// BGRA → RGBA, alpha luôn 255 (alpha của ảnh chụp màn hình không tin được, mà GIF coi alpha 0
/// là trong suốt).
pub fn bgra_to_rgba(src: &[u8], w: u32, h: u32) -> Result<Vec<u8>, GifError> {
    if w == 0 || h == 0 {
        return Err(GifError::InvalidRegion);
    }
    let n = pixel_bytes(w, h)?;
    if src.len() != n {
        return Err(GifError::InvalidRegion);
    }
    let mut out = alloc(n)?;
    for p in src.as_chunks::<4>().0 {
        out.extend_from_slice(&[p[2], p[1], p[0], 255]);
    }
    Ok(out)
}

/// Kích thước khung GIF cho vùng `w`×`h`: giữ nguyên khi ≤ `MAX_FRAME_PIXELS`, nếu không thu
/// nhỏ đều hai chiều theo `s = sqrt(MAX_FRAME_PIXELS / (w·h))`, lấy phần nguyên (tối thiểu 1);
/// mỗi cạnh tối đa `MAX_SIDE`.
pub fn frame_size(w: u32, h: u32) -> Result<(u16, u16), GifError> {
    if w < MIN_SIDE || h < MIN_SIDE {
        return Err(GifError::InvalidRegion);
    }
    let (w64, h64) = (u64::from(w), u64::from(h));
    let px = w64.checked_mul(h64).ok_or(GifError::TooLarge)?;
    if px > MAX_SOURCE_PIXELS {
        return Err(GifError::TooLarge);
    }
    let (mut tw, mut th) = if px <= MAX_FRAME_PIXELS {
        (w64, h64)
    } else {
        // floor(w·s) = floor(sqrt(M·w/h)) = isqrt(floor(M·w/h)): tính đúng bằng số nguyên.
        let tw = (MAX_FRAME_PIXELS
            .checked_mul(w64)
            .ok_or(GifError::TooLarge)?
            / h64)
            .isqrt();
        let th = (MAX_FRAME_PIXELS
            .checked_mul(h64)
            .ok_or(GifError::TooLarge)?
            / w64)
            .isqrt();
        (tw, th)
    };
    let max = u64::from(MAX_SIDE);
    if tw > max {
        th = h64.checked_mul(max).ok_or(GifError::TooLarge)? / w64;
        tw = max;
    }
    if th > max {
        tw = w64.checked_mul(max).ok_or(GifError::TooLarge)? / h64;
        th = max;
    }
    let tw = u16::try_from(tw.max(1)).map_err(|_| GifError::TooLarge)?;
    let th = u16::try_from(th.max(1)).map_err(|_| GifError::TooLarge)?;
    Ok((tw, th))
}

/// Khoảng nguồn `[a, b)` của ô đích `i` trên `t` ô, nguồn dài `s` (luôn có ít nhất một pixel).
fn span(i: u64, t: u64, s: u64) -> Result<(usize, usize), GifError> {
    let a = i.checked_mul(s).ok_or(GifError::TooLarge)? / t;
    let b = (i + 1).checked_mul(s).ok_or(GifError::TooLarge)? / t;
    let b = b.max(a + 1).min(s);
    let a = usize::try_from(a).map_err(|_| GifError::TooLarge)?;
    let b = usize::try_from(b).map_err(|_| GifError::TooLarge)?;
    Ok((a, b))
}

/// Đổi cỡ ảnh BGRA về `w`×`h` bằng lọc trung bình vùng (box); bằng kích thước thì trả bản sao.
/// Ảnh kết quả có alpha 255.
pub fn scale_bgra(img: &Image, w: u16, h: u16) -> Result<Image, GifError> {
    if !img.is_valid() || w == 0 || h == 0 {
        return Err(GifError::InvalidRegion);
    }
    let (tw, th) = (u32::from(w), u32::from(h));
    if (tw, th) == (img.width, img.height) {
        return Ok(img.clone());
    }
    let stride = pixel_bytes(img.width, 1)?;
    let (sw, sh) = (u64::from(img.width), u64::from(img.height));
    let xs = (0..u64::from(tw))
        .map(|x| span(x, u64::from(tw), sw))
        .collect::<Result<Vec<_>, _>>()?;
    let mut out = alloc(pixel_bytes(tw, th)?)?;
    for ty in 0..u64::from(th) {
        let (y0, y1) = span(ty, u64::from(th), sh)?;
        for &(x0, x1) in &xs {
            let mut acc = [0u64; 3];
            for y in y0..y1 {
                let row = &img.bgra[y * stride + x0 * 4..y * stride + x1 * 4];
                for p in row.as_chunks::<4>().0 {
                    acc[0] += u64::from(p[0]);
                    acc[1] += u64::from(p[1]);
                    acc[2] += u64::from(p[2]);
                }
            }
            // Số pixel của ô ≤ số pixel ảnh, nên không tràn u64.
            let n = u64::try_from((y1 - y0) * (x1 - x0)).map_err(|_| GifError::TooLarge)?;
            for c in acc {
                out.push(u8::try_from((c + n / 2) / n).unwrap_or(u8::MAX));
            }
            out.push(255);
        }
    }
    Ok(Image {
        width: tw,
        height: th,
        bgra: out,
    })
}

/// Mốc chụp kế tiếp (ms) sau thời điểm `elapsed_ms`: mốc thứ k là `k·interval`; các mốc đã lỡ
/// bị bỏ (không xếp hàng).
fn next_tick(elapsed_ms: u64, interval_ms: u64) -> u64 {
    let iv = interval_ms.max(1);
    (elapsed_ms / iv).saturating_add(1).saturating_mul(iv)
}

/// Làm tròn ms sang cs (5 ms làm tròn lên).
fn round_cs(ms: u64) -> u64 {
    ms.saturating_add(5) / 10
}

fn ms(d: Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}

/// Bọc writer để đếm số byte đã ghi.
struct CountingWriter<W> {
    inner: W,
    bytes: u64,
}

impl<W: Write> Write for CountingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.bytes = self
            .bytes
            .saturating_add(u64::try_from(n).unwrap_or(u64::MAX));
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Bộ mã hóa GIF lặp vô hạn giữ đúng một khung chờ (RGBA) để biết độ trễ khi khung kế tới.
pub struct GifWriter<W: Write> {
    enc: Encoder<CountingWriter<W>>,
    width: u16,
    height: u16,
    len: usize,
    pending: Option<Vec<u8>>,
    /// Mốc (cs, cùng gốc với `at_ms`) nơi khung chờ bắt đầu hiện; cộng dồn độ trễ đã ghi nên
    /// không trôi.
    shown_cs: u64,
    frames: u32,
}

impl<W: Write> GifWriter<W> {
    /// Ghi phần đầu GIF (kích thước, lặp vô hạn) vào `out`.
    pub fn new(w: u16, h: u16, out: W) -> Result<Self, GifError> {
        if w == 0 || h == 0 {
            return Err(GifError::InvalidRegion);
        }
        let len = pixel_bytes(u32::from(w), u32::from(h))?;
        let counting = CountingWriter {
            inner: out,
            bytes: 0,
        };
        let mut enc = Encoder::new(counting, w, h, &[]).map_err(enc_err)?;
        enc.set_repeat(Repeat::Infinite).map_err(enc_err)?;
        Ok(GifWriter {
            enc,
            width: w,
            height: h,
            len,
            pending: None,
            shown_cs: 0,
            frames: 0,
        })
    }

    /// Thêm khung RGBA chụp lúc `at_ms`: mã hóa khung chờ trước đó (độ trễ tới `at_ms`) rồi giữ
    /// khung này làm khung chờ. Khung giống hệt khung chờ thì gộp (kéo dài độ trễ).
    pub fn push(&mut self, rgba: Vec<u8>, at_ms: u64) -> Result<(), GifError> {
        if rgba.len() != self.len {
            return Err(GifError::InvalidRegion);
        }
        match &self.pending {
            None => {
                self.shown_cs = round_cs(at_ms);
                self.pending = Some(rgba);
                self.frames = 1;
            }
            Some(p) if *p == rgba => {}
            Some(_) => {
                self.emit(round_cs(at_ms))?;
                self.pending = Some(rgba);
                self.frames = self.frames.saturating_add(1);
            }
        }
        Ok(())
    }

    /// Mã hóa khung chờ với độ trễ tới mốc `until_cs` (tối thiểu `MIN_DELAY_CS`).
    fn emit(&mut self, until_cs: u64) -> Result<(), GifError> {
        let Some(mut px) = self.pending.take() else {
            return Ok(());
        };
        let delay = until_cs
            .saturating_sub(self.shown_cs)
            .clamp(u64::from(MIN_DELAY_CS), u64::from(u16::MAX));
        // Độ dài đã kiểm ở `push`, tốc độ là hằng trong [1, 30]: không panic.
        let mut frame = Frame::from_rgba_speed(self.width, self.height, &mut px, QUANT_SPEED);
        frame.delay = u16::try_from(delay).unwrap_or(u16::MAX);
        self.enc.write_frame(&frame).map_err(enc_err)?;
        self.shown_cs = self.shown_cs.saturating_add(delay);
        Ok(())
    }

    /// Mã hóa khung chờ với độ trễ tới `end_ms`, ghi phần kết GIF và trả writer.
    pub fn finish(mut self, end_ms: u64) -> Result<W, GifError> {
        if self.pending.is_none() {
            return Err(GifError::NoFrames);
        }
        self.emit(round_cs(end_ms))?;
        let GifWriter { enc, .. } = self;
        Ok(enc.into_inner().map_err(enc_err)?.inner)
    }

    /// Số khung GIF (đã ghi và đang chờ; khung gộp không tính).
    pub fn frames(&self) -> u32 {
        self.frames
    }

    /// Số byte đã ghi ra writer (khung chờ chưa tính).
    pub fn bytes_written(&self) -> u64 {
        self.enc.get_ref().bytes
    }
}

/// Tệp tạm `.darkshot-rec-<16 ký tự>.gif.part` trong thư mục đích; bị xóa khi drop.
pub struct TempGif {
    file: Option<BufWriter<File>>,
    path: PathBuf,
}

impl TempGif {
    /// Tạo thư mục nếu chưa có, rồi tạo tệp tạm mới (`create_new`, không mở lại tệp có sẵn).
    pub fn create(dir: &Path) -> Result<TempGif, GifError> {
        fs::create_dir_all(dir).map_err(io_err)?;
        for _ in 0..8 {
            let id = naming::try_random_object_name()
                .ok_or(GifError::Fault(GifFault::RandomUnavailable))?;
            let path = dir.join(format!("{TEMP_PREFIX}{id}{TEMP_SUFFIX}"));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(f) => {
                    return Ok(TempGif {
                        file: Some(BufWriter::with_capacity(64 * 1024, f)),
                        path,
                    })
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(io_err(e)),
            }
        }
        Err(GifError::Fault(GifFault::NoTempName))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Ghi hết xuống đĩa, đóng tệp rồi đặt tên cuối `dir/name` (hoặc `_2`, `_3`…) KHÔNG ghi đè.
    /// Mọi lỗi: tệp tạm bị xóa.
    pub fn commit(mut self, dir: &Path, name: &str) -> Result<PathBuf, GifError> {
        let buf = self
            .file
            .take()
            .ok_or(GifError::Fault(GifFault::TempClosed))?;
        let file = buf.into_inner().map_err(|e| io_err(e.into_error()))?;
        file.sync_all().map_err(io_err)?;
        drop(file);
        place_final(&self.path, dir, name, |a, b| fs::hard_link(a, b))
        // `self` drop: xóa tệp tạm nếu còn.
    }
}

impl Write for TempGif {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.file.as_mut() {
            Some(f) => f.write(buf),
            None => Err(io::Error::other("temporary file closed")),
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        match self.file.as_mut() {
            Some(f) => f.flush(),
            None => Ok(()),
        }
    }
}

impl Drop for TempGif {
    fn drop(&mut self) {
        if let Some(b) = self.file.take() {
            // Bỏ phần đệm chưa ghi, đóng tệp trước khi xóa.
            let (f, _) = b.into_parts();
            drop(f);
        }
        let _ = fs::remove_file(&self.path);
    }
}

/// Đặt `tmp` (đã đóng) thành tên cuối không trùng trong `dir`. Liên kết cứng thất bại vì tên đã
/// có thì thử tên kế; thất bại vì lý do khác (hệ tệp không hỗ trợ) thì đổi tên sau khi kiểm lại
/// tên còn trống. Thành công thì tệp tạm đã xóa (hoặc đã đổi tên).
fn place_final<L>(tmp: &Path, dir: &Path, name: &str, mut link: L) -> Result<PathBuf, GifError>
where
    L: FnMut(&Path, &Path) -> io::Result<()>,
{
    for _ in 0..1000 {
        let cand = naming::unique_path(dir, name);
        match link(tmp, &cand) {
            Ok(()) => {
                let _ = fs::remove_file(tmp);
                return Ok(cand);
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(_) => {
                if cand.exists() {
                    continue;
                }
                fs::rename(tmp, &cand).map_err(io_err)?;
                return Ok(cand);
            }
        }
    }
    Err(GifError::Fault(GifFault::NoFileName))
}

fn is_temp_name(n: &str) -> bool {
    n.strip_prefix(TEMP_PREFIX)
        .and_then(|r| r.strip_suffix(TEMP_SUFFIX))
        .is_some_and(|id| id.len() == 16 && id.bytes().all(|b| b.is_ascii_alphanumeric()))
}

/// Xóa các tệp tạm sót lại (đúng mẫu `.darkshot-rec-<16 ký tự>.gif.part`) trong `dir`, sửa lần
/// cuối trước mốc `older_than` (lúc khởi động tiến trình); trả số tệp đã xóa. Tệp mới hơn mốc (lần
/// ghi vừa bắt đầu, hay bản darkshot khác đang ghi cùng thư mục) được giữ, nên gọi được trên luồng
/// nền song song với lần ghi.
pub fn sweep_stale(dir: &Path, older_than: std::time::SystemTime) -> usize {
    let Ok(rd) = fs::read_dir(dir) else {
        return 0;
    };
    rd.flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .filter(|e| e.file_name().to_str().is_some_and(is_temp_name))
        .filter(|e| {
            e.metadata()
                .and_then(|m| m.modified())
                .is_ok_and(|t| t < older_than)
        })
        .filter(|e| fs::remove_file(e.path()).is_ok())
        .count()
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
    pub fn new(dir: PathBuf, file_name: String) -> Self {
        RecordConfig {
            dir,
            file_name,
            interval_ms: FRAME_INTERVAL_MS,
            max_duration_ms: MAX_DURATION_MS,
            max_file_bytes: MAX_FILE_BYTES,
        }
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

const RUNNING: u8 = 0;
const STOPPING: u8 = 1;
const CANCELLING: u8 = 2;
/// Dừng vì vùng ghi không còn trên màn hình (giao diện báo): lưu như Dừng, lý do `SourceLost`.
const STOPPING_LOST: u8 = 3;

struct Shared {
    state: AtomicU8,
    lock: Mutex<()>,
    cv: Condvar,
}

impl Shared {
    fn state(&self) -> u8 {
        self.state.load(Ordering::Acquire)
    }

    /// `Running → to` đúng một lần; đánh thức luồng đang chờ.
    fn leave_running(&self, to: u8) -> bool {
        let ok = self
            .state
            .compare_exchange(RUNNING, to, Ordering::AcqRel, Ordering::Acquire)
            .is_ok();
        if ok {
            // Lấy rồi nhả khóa trước khi báo: luồng chờ hoặc chưa kiểm trạng thái (sẽ thấy trạng
            // thái mới), hoặc đã nằm trong `wait_timeout` (sẽ nhận báo). Không mất lượt đánh thức.
            drop(self.lock.lock().unwrap_or_else(PoisonError::into_inner));
            self.cv.notify_all();
        }
        ok
    }

    /// Chờ tới `deadline` (tính từ `start`) hoặc tới khi trạng thái rời `Running`.
    fn wait_until(&self, start: Instant, deadline: Duration) {
        let mut g = self.lock.lock().unwrap_or_else(PoisonError::into_inner);
        loop {
            if self.state() != RUNNING {
                return;
            }
            let now = start.elapsed();
            if now >= deadline {
                return;
            }
            g = match self.cv.wait_timeout(g, deadline - now) {
                Ok((g, _)) => g,
                Err(p) => p.into_inner().0,
            };
        }
    }
}

/// Lần ghi GIF trên luồng nền. Drop: yêu cầu hủy (không chặn); luồng nền vẫn dọn tệp tạm.
pub struct Recorder {
    shared: Arc<Shared>,
    rx: Receiver<Outcome>,
    done: bool,
    started: Instant,
    max_duration_ms: u64,
}

impl Recorder {
    /// Tạo tệp tạm và luồng ghi rồi mới trả về. `source` chụp một khung (gọi trên luồng nền,
    /// lần lượt); `wake` được gọi đúng một lần trên luồng nền, sau khi kết quả đã gửi.
    ///
    /// `source` và `wake` KHÔNG được panic: bản phát hành dùng `panic = "abort"` nên panic trên
    /// luồng nền thoát cả tiến trình (tệp `.part` còn lại do `sweep_stale` dọn ở lần khởi động
    /// sau); `catch_unwind` bên trong chỉ có tác dụng ở bản debug và kiểm thử. `source` báo lỗi
    /// bằng `Err`.
    pub fn start<S, W>(cfg: RecordConfig, source: S, wake: W) -> Result<Recorder, GifError>
    where
        S: FnMut() -> Result<Image, String> + Send + 'static,
        W: Fn() + Send + 'static,
    {
        let tmp = TempGif::create(&cfg.dir)?;
        let shared = Arc::new(Shared {
            state: AtomicU8::new(RUNNING),
            lock: Mutex::new(()),
            cv: Condvar::new(),
        });
        let (tx, rx) = mpsc::channel();
        let started = Instant::now();
        let max_duration_ms = cfg.max_duration_ms;
        let sh = shared.clone();
        // Lỗi spawn: closure (cùng `tmp`) bị drop nên tệp tạm bị xóa.
        thread::Builder::new()
            .name("darkshot-gif".into())
            .spawn(move || {
                let outcome = catch_unwind(AssertUnwindSafe(|| {
                    record_loop(cfg, source, tmp, &sh, started)
                }))
                // Chỉ bắt được ở bản debug/kiểm thử: bản phát hành dùng `panic = "abort"`, panic là
                // thoát tiến trình và tệp `.part` còn lại do `sweep_stale` dọn ở lần khởi động sau.
                .unwrap_or(Outcome::Failed(GifError::Fault(GifFault::ThreadPanicked)));
                // Sau khi có kết quả, Dừng/Hủy không còn tác dụng.
                sh.leave_running(STOPPING);
                let _ = tx.send(outcome);
                // Như trên: chỉ có tác dụng ở debug/kiểm thử; `wake` không được panic.
                let _ = catch_unwind(AssertUnwindSafe(&wake));
            })
            .map_err(|e| GifError::Thread(e.to_string()))?;
        Ok(Recorder {
            shared,
            rx,
            done: false,
            started,
            max_duration_ms,
        })
    }

    /// Dừng và lưu phần đã ghi. `false` khi đã dừng, đã hủy hoặc luồng đã tự dừng.
    pub fn request_stop(&self) -> bool {
        self.shared.leave_running(STOPPING)
    }

    /// Dừng và lưu phần đã ghi vì vùng ghi không còn trên màn hình (lý do `SourceLost`). Như
    /// `request_stop`: chỉ đổi được một lần, `false` khi đã dừng, đã hủy hoặc luồng đã tự dừng.
    pub fn request_stop_lost(&self) -> bool {
        self.shared.leave_running(STOPPING_LOST)
    }

    /// Hủy, không tạo GIF. `false` khi đã dừng, đã hủy hoặc luồng đã tự dừng.
    pub fn request_cancel(&self) -> bool {
        self.shared.leave_running(CANCELLING)
    }

    /// Kết quả nếu đã có; không chặn; trả `Some` đúng một lần.
    pub fn try_outcome(&mut self) -> Option<Outcome> {
        if self.done {
            return None;
        }
        let got = match self.rx.try_recv() {
            Ok(o) => o,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => lost_thread(),
        };
        self.done = true;
        Some(got)
    }

    /// Chờ kết quả tối đa `timeout`; trả `Some` đúng một lần.
    pub fn join(&mut self, timeout: Duration) -> Option<Outcome> {
        if self.done {
            return None;
        }
        let got = match self.rx.recv_timeout(timeout) {
            Ok(o) => o,
            Err(RecvTimeoutError::Timeout) => return None,
            Err(RecvTimeoutError::Disconnected) => lost_thread(),
        };
        self.done = true;
        Some(got)
    }

    /// Thời gian đã ghi (ms), tối đa bằng thời lượng tối đa.
    pub fn elapsed_ms(&self) -> u64 {
        ms(self.started.elapsed()).min(self.max_duration_ms)
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        self.request_cancel();
    }
}

fn lost_thread() -> Outcome {
    Outcome::Failed(GifError::Fault(GifFault::ThreadEnded))
}

/// Lý do lưu của trạng thái dừng `state` do giao diện yêu cầu; `None` khi không phải trạng thái dừng.
fn requested_stop(state: u8) -> Option<StopReason> {
    match state {
        STOPPING => Some(StopReason::Stopped),
        STOPPING_LOST => Some(StopReason::SourceLost),
        _ => None,
    }
}

/// Luồng nền tự dừng vì `reason`. `None` khi đã bị hủy trước đó (Hủy thắng); Dừng của giao diện
/// tới trước thì giữ lý do của nó (`Stopped` hay `SourceLost`).
fn auto_stop(sh: &Shared, reason: StopReason) -> Option<StopReason> {
    if sh.leave_running(STOPPING) {
        return Some(reason);
    }
    requested_stop(sh.state())
}

/// Thân luồng ghi. Mọi tài nguyên (tệp tạm, bộ mã hóa) là biến cục bộ: trả về hay panic đều xóa
/// tệp tạm nếu chưa đặt tên cuối.
fn record_loop<S>(
    cfg: RecordConfig,
    mut source: S,
    tmp: TempGif,
    sh: &Shared,
    started: Instant,
) -> Outcome
where
    S: FnMut() -> Result<Image, String>,
{
    // Chụp nhanh hơn độ trễ tối thiểu của GIF chỉ làm lệch thời gian (độ trễ bị nâng lên 2 cs).
    let interval = cfg.interval_ms.max(u64::from(MIN_DELAY_CS) * 10);
    let max_ms = cfg.max_duration_ms;
    let elapsed = || ms(started.elapsed());
    let mut tmp = Some(tmp);
    let mut writer: Option<GifWriter<TempGif>> = None;
    let mut src_size: Option<(u32, u32)> = None;
    let mut target = (0u16, 0u16);

    let (reason, end_ms) = loop {
        let state = sh.state();
        match (state, requested_stop(state)) {
            (RUNNING, _) => {}
            // Dừng trước khi có khung nào: vẫn chụp đúng một khung rồi lưu.
            (_, Some(reason)) if writer.as_ref().is_some_and(|w| w.frames() > 0) => {
                break (reason, elapsed().min(max_ms))
            }
            (_, Some(_)) => {}
            _ => return Outcome::Cancelled,
        }
        let frames = writer.as_ref().map_or(0, GifWriter::frames);
        // Khung đầu luôn được chụp (luồng có thể khởi động trễ) và hiện từ mốc 0, nên GIF dài
        // đúng bằng thời gian ghi.
        let at = if frames == 0 { 0 } else { elapsed() };
        // Hết giờ, hoặc còn ít hơn độ trễ tối thiểu: khung cuối kéo tới đúng mốc hết giờ.
        if frames > 0 && round_cs(at) + u64::from(MIN_DELAY_CS) > round_cs(max_ms) {
            match auto_stop(sh, StopReason::MaxDuration) {
                Some(r) => break (r, max_ms),
                None => return Outcome::Cancelled,
            }
        }

        // Một lượt: chụp → (thu nhỏ) → RGBA → mã hóa khung chờ trước đó.
        let frame = source().map_err(GifError::Capture).and_then(|img| {
            let size = (img.width, img.height);
            match src_size {
                None => {
                    target = frame_size(img.width, img.height)?;
                    src_size = Some(size);
                }
                Some(s) if s != size => {
                    return Err(GifError::Fault(GifFault::FrameSizeChanged));
                }
                Some(_) => {}
            }
            let scaled = if (u32::from(target.0), u32::from(target.1)) == size {
                if !img.is_valid() {
                    return Err(GifError::InvalidRegion);
                }
                img
            } else {
                scale_bgra(&img, target.0, target.1)?
            };
            bgra_to_rgba(&scaled.bgra, scaled.width, scaled.height)
        });
        if sh.state() == CANCELLING {
            return Outcome::Cancelled;
        }
        let rgba = match frame {
            Ok(px) => px,
            // Nguồn mất khi đã có khung: lưu phần đã ghi.
            Err(_) if frames > 0 => match auto_stop(sh, StopReason::SourceLost) {
                Some(r) => break (r, elapsed().min(max_ms)),
                None => return Outcome::Cancelled,
            },
            Err(e) => return Outcome::Failed(e),
        };
        if writer.is_none() {
            let Some(t) = tmp.take() else {
                return Outcome::Failed(GifError::Fault(GifFault::Internal));
            };
            match GifWriter::new(target.0, target.1, t) {
                Ok(w) => writer = Some(w),
                Err(e) => return Outcome::Failed(e),
            }
        }
        let Some(w) = writer.as_mut() else {
            return Outcome::Failed(GifError::Fault(GifFault::Internal));
        };
        if let Err(e) = w.push(rgba, at) {
            return Outcome::Failed(e);
        }
        if w.bytes_written() >= cfg.max_file_bytes {
            match auto_stop(sh, StopReason::MaxSize) {
                Some(r) => break (r, elapsed().min(max_ms)),
                None => return Outcome::Cancelled,
            }
        }
        let deadline = next_tick(elapsed(), interval).min(max_ms);
        sh.wait_until(started, Duration::from_millis(deadline));
    };

    let Some(w) = writer else {
        return Outcome::Failed(GifError::NoFrames);
    };
    let frames = w.frames();
    let tmp = match w.finish(end_ms) {
        Ok(t) => t,
        Err(e) => return Outcome::Failed(e),
    };
    match tmp.commit(&cfg.dir, &cfg.file_name) {
        Ok(path) => Outcome::Saved {
            path,
            reason,
            frames,
        },
        Err(e) => Outcome::Failed(e),
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
                let glyph =
                    (x / 6 + y / 10 + seed).is_multiple_of(7) && (y % 10) < 7 && (x % 6) < 4;
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
        assert_eq!(frame_size(1000, 1000), Ok((707, 707)));
        assert_eq!(frame_size(800, 600), Ok((800, 600)));
        // 4K: s = sqrt(5e5 / 8 294 400).
        assert_eq!(frame_size(3840, 2160), Ok((942, 530)));
        assert_eq!(frame_size(8000, 8000), Ok((707, 707)));
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
        assert_eq!((s.width, s.height), (942, 530));
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
        // Mốc sau mọi tệp vừa tạo: mọi tệp đúng mẫu đều cũ hơn mốc.
        let after = std::time::SystemTime::now() + Duration::from_secs(60);
        assert_eq!(sweep_stale(&d, after), 2);
        let mut want: Vec<String> = keep.iter().map(|s| s.to_string()).collect();
        want.push(".darkshot-rec-DDDDDDDDDDDDDDDD.gif.part".into());
        want.sort();
        assert_eq!(names(&d), want);
        assert_eq!(sweep_stale(&d.join("missing"), after), 0);
    }

    /// Chỉ xóa tệp tạm sửa lần cuối TRƯỚC mốc (khởi động tiến trình): tệp của một lần ghi vừa
    /// bắt đầu, hay của bản darkshot khác đang ghi cùng thư mục, mới hơn mốc nên được giữ.
    #[test]
    fn sweep_stale_keeps_temp_files_newer_than_cutoff() {
        let d = temp_dir();
        let old = d.join(".darkshot-rec-OOOOOOOOOOOOOOOO.gif.part");
        let new = d.join(".darkshot-rec-NNNNNNNNNNNNNNNN.gif.part");
        std::fs::write(&old, b"o").unwrap();
        std::fs::write(&new, b"n").unwrap();
        let cutoff = std::time::SystemTime::now();
        let f = OpenOptions::new().write(true).open(&old).unwrap();
        f.set_modified(cutoff - Duration::from_secs(3600)).unwrap();
        drop(f);
        let f = OpenOptions::new().write(true).open(&new).unwrap();
        f.set_modified(cutoff + Duration::from_secs(5)).unwrap();
        drop(f);
        assert_eq!(sweep_stale(&d, cutoff), 1);
        assert!(!old.exists());
        assert!(new.exists());
    }

    // --- Recorder ---

    const WAIT: Duration = Duration::from_secs(30);

    fn cfg(dir: &Path) -> RecordConfig {
        RecordConfig {
            dir: dir.to_path_buf(),
            file_name: "darkshot_2026-11-21_10-00-00.gif".into(),
            interval_ms: 10,
            max_duration_ms: 60_000,
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

    /// Như `counting_source`, kèm bộ đếm số lần gọi.
    fn counted_source() -> (
        Arc<AtomicUsize>,
        impl FnMut() -> Result<Image, String> + Send + 'static,
    ) {
        let calls = Arc::new(AtomicUsize::new(0));
        let c2 = calls.clone();
        let mut inner = counting_source();
        (calls, move || {
            c2.fetch_add(1, Ordering::SeqCst);
            inner()
        })
    }

    fn wait_calls(n: &AtomicUsize, want: usize) {
        let until = Instant::now() + WAIT;
        while n.load(Ordering::SeqCst) < want && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(n.load(Ordering::SeqCst) >= want);
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
        // Nhịp thật 100 ms: khoảng cách khung lớn hơn độ trễ tối thiểu nên tổng độ trễ đúng.
        let mut c = cfg(&d);
        c.interval_ms = FRAME_INTERVAL_MS;
        c.max_duration_ms = 500;
        let mut r = Recorder::start(c, counting_source(), wake).unwrap();
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
        // Khung cuối kéo tới đúng mốc hết giờ: tổng độ trễ = 500 ms.
        let sum: u32 = delays(&dec).iter().map(|&x| u32::from(x)).sum();
        assert_eq!(sum, 50);
        assert_eq!(
            names(&d),
            vec!["darkshot_2026-11-21_10-00-00.gif".to_string()]
        );
        wait_wakes(&wakes, 1);
        assert_eq!(r.try_outcome(), None, "chỉ trả một lần");
        assert_eq!(r.join(Duration::from_millis(10)), None);
        assert!(!r.request_cancel(), "hủy sau khi tự dừng không có tác dụng");
        assert!(!r.request_stop());
        assert_eq!(r.elapsed_ms(), 500);
        std::thread::sleep(Duration::from_millis(30));
        assert_eq!(wakes.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn recorder_creates_temp_before_returning_then_stop_saves() {
        let d = temp_dir();
        let (gate_tx, gate_rx) = mpsc::channel::<()>();
        let (entered_tx, entered_rx) = mpsc::channel::<()>();
        let mut first = true;
        let source = move || {
            if first {
                first = false;
                let _ = entered_tx.send(());
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
        // Luồng đang trong lượt chụp đầu: Dừng lúc này vẫn giữ khung đó.
        entered_rx.recv_timeout(WAIT).unwrap();
        assert!(r.request_stop());
        gate_tx.send(()).unwrap();
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
        let (calls, source) = counted_source();
        let mut r = Recorder::start(c, source, wake).unwrap();
        // Khung đầu đã chụp: Dừng sau đó luôn có ít nhất một khung.
        wait_calls(&calls, 1);
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

    /// Vùng ghi mất màn hình: dừng với lý do `SourceLost` (thông báo Info), đúng một lần; Dừng hay
    /// Hủy tới sau không đổi được lý do, và ngược lại.
    #[test]
    fn recorder_stop_lost_saves_with_source_lost() {
        let d = temp_dir();
        let mut c = cfg(&d);
        c.interval_ms = 5_000;
        c.max_duration_ms = 60_000;
        let (_w, wake) = wake_counter();
        let (calls, source) = counted_source();
        let mut r = Recorder::start(c, source, wake).unwrap();
        wait_calls(&calls, 1);
        std::thread::sleep(Duration::from_millis(50));
        assert!(r.request_stop_lost());
        assert!(!r.request_stop_lost());
        assert!(!r.request_stop());
        assert!(!r.request_cancel());
        let out = r.join(WAIT).expect("outcome");
        assert!(
            matches!(
                out,
                Outcome::Saved {
                    reason: StopReason::SourceLost,
                    ..
                }
            ),
            "{out:?}"
        );
        // Dừng trước: mất màn hình tới sau không đổi lý do.
        let d = temp_dir();
        let mut c = cfg(&d);
        c.interval_ms = 5_000;
        c.max_duration_ms = 60_000;
        let (_w, wake) = wake_counter();
        let (calls, source) = counted_source();
        let mut r = Recorder::start(c, source, wake).unwrap();
        wait_calls(&calls, 1);
        assert!(r.request_stop());
        assert!(!r.request_stop_lost());
        let out = r.join(WAIT).expect("outcome");
        assert!(
            matches!(
                out,
                Outcome::Saved {
                    reason: StopReason::Stopped,
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
        let mut r = Recorder::start(c, || Ok(solid(1000, 600, 5, 6, 7)), || {}).unwrap();
        let out = r.join(WAIT);
        let Some(Outcome::Saved { path, .. }) = out else {
            panic!("unexpected outcome: {out:?}");
        };
        let dec = decode(&std::fs::read(&path).unwrap());
        assert_eq!((dec.width, dec.height), frame_size(1000, 600).unwrap());
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
            for p in bgra.as_chunks_mut::<4>().0 {
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
        assert_eq!(
            r.join(WAIT),
            Some(Outcome::Failed(GifError::Fault(GifFault::ThreadPanicked)))
        );
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
    fn fault_messages_both_languages() {
        for f in GifFault::ALL {
            let vi = GifError::Fault(f).message(Lang::Vi);
            let en = GifError::Fault(f).message(Lang::En);
            assert_eq!(vi, t(Lang::Vi, f.msg()));
            assert_eq!(en, t(Lang::En, f.msg()));
            assert!(!vi.trim().is_empty() && !en.trim().is_empty(), "{f:?}");
            assert_ne!(vi, en, "{f:?}");
        }
        for (i, a) in GifFault::ALL.iter().enumerate() {
            for b in &GifFault::ALL[i + 1..] {
                assert_ne!(a.msg(), b.msg());
            }
        }
    }

    #[test]
    fn help_text_matches_frame_budget() {
        // 500 000 pixel: "0,5 triệu" / "500,000".
        assert_eq!(MAX_FRAME_PIXELS, 500_000);
        let vi = crate::i18n::help_text(Lang::Vi, &crate::i18n::HelpKeys::default());
        let en = crate::i18n::help_text(Lang::En, &crate::i18n::HelpKeys::default());
        let n = MAX_FRAME_PIXELS;
        let vi_num = format!("{} {:03}", n / 1000, n % 1000);
        let en_num = format!("{},{:03}", n / 1000, n % 1000);
        assert!(vi.contains(&vi_num), "{vi_num}");
        assert!(vi.contains("0,5 triệu điểm ảnh"));
        assert!(en.contains(&format!("about {en_num} pixels")), "{en_num}");
        let secs = (MAX_DURATION_MS / 1000).to_string();
        assert!(vi.contains(&format!("tối đa {secs} giây")));
        assert!(en.contains(&format!("up to {secs} seconds")));
        assert!(t(Lang::Vi, Msg::RecordMaxDuration).contains(&secs));
        assert!(t(Lang::En, Msg::RecordMaxDuration).contains(&secs));
    }

    #[test]
    fn recorder_stop_before_first_frame_still_saves_one() {
        for _ in 0..5 {
            let d = temp_dir();
            let (wakes, wake) = wake_counter();
            let mut r = Recorder::start(cfg(&d), counting_source(), wake).unwrap();
            assert!(r.request_stop());
            let out = r.join(WAIT).expect("outcome");
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
            assert_eq!(
                names(&d),
                vec!["darkshot_2026-11-21_10-00-00.gif".to_string()]
            );
            wait_wakes(&wakes, 1);
        }
    }

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
        // So sánh tốc độ lượng tử hóa (chỉ `from_rgba_speed`) để chọn `QUANT_SPEED`.
        for speed in [1, 10, 20, 30] {
            let mut px = frames[1].clone();
            let t0 = Instant::now();
            let f = gif::Frame::from_rgba_speed(w, h, &mut px, speed);
            let q = t0.elapsed().as_secs_f64() * 1000.0;
            let mut e = gif::Encoder::new(Vec::new(), w, h, &[]).unwrap();
            let t1 = Instant::now();
            e.write_frame(&f).unwrap();
            let l = t1.elapsed().as_secs_f64() * 1000.0;
            println!("speed {speed}: quantize {q:.1} ms, lzw {l:.1} ms");
        }
        // Giao diện phẳng (≤ 256 màu): đi đường bảng màu chính xác, không NeuQuant.
        let mut flat = rgba_solid(w, h, 30, 30, 34);
        for (i, p) in flat.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            if (i / 1000 / 20 + i % 1000 / 8) % 5 == 0 {
                p.copy_from_slice(&[(i % 7 * 30) as u8, 200, 220, 255]);
            }
        }
        let t0 = Instant::now();
        let f = gif::Frame::from_rgba_speed(w, h, &mut flat, QUANT_SPEED);
        println!(
            "flat UI frame (<=256 colors): quantize {:.1} ms, palette {} bytes",
            t0.elapsed().as_secs_f64() * 1000.0,
            f.palette.as_ref().map_or(0, Vec::len)
        );
        // Thu nhỏ 4K -> 942x530, rồi đổi RGBA.
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
