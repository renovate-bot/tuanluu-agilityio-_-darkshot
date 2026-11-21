# Task 122: Ghi GIF — lõi (mã hóa, tệp tạm, luồng ghi)

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/122-gif-core` (từ `feature/gif-recording`) · **Rà soát:** `reviewer` (fable)

Thiết kế: `docs/superpowers/specs/2026-11-21-gif-recording-design.md` mục 2, 3.1, 3.3 (phần lõi), 3.4, 4, 5. Global Constraints: kế hoạch `docs/superpowers/plans/2026-11-21-gif-recording.md` (đọc mục đó).

## Cần đọc
`crates/darkshot-core/src/image.rs` (`Image` BGRA, `EncodeError::message`), `naming.rs` (`local_file_name`, `unique_path`, `random_object_name`), `actions.rs` (`Action`, nhánh no-op của `Scroll`), `last_capture.rs`, `i18n.rs` (cách thêm `Msg`, kiểm thử đủ hai ngôn ngữ). `grep -rn "Action::Scroll" crates/` để thêm nhánh `RecordGif` mọi nơi. Tài liệu crate `gif` 0.14 (docs.rs): `Encoder::new`, `set_repeat(Repeat::Infinite)`, `Frame::from_rgba_speed`, `Frame.delay`, `write_frame`, `into_inner`; `Decoder` cho kiểm thử. Đối chiếu tài liệu, không dựa trí nhớ.

## Files
Tạo `crates/darkshot-core/src/gif_record.rs`; sửa `crates/darkshot-core/Cargo.toml` (thêm `gif`, kiểm feature mặc định có `color_quant`; tắt feature không cần nếu được), `lib.rs`, `naming.rs`, `actions.rs`, `last_capture.rs`, `i18n.rs`, `Cargo.lock`. Lớp Windows chỉ sửa tối thiểu để biên dịch (nhánh `RecordGif` không làm gì ở `main.rs`, tooltip map trong `overlay.rs`; KHÔNG thêm ô thanh ngang — Task 123).

## Interfaces (hợp đồng với Task 123)
```rust
pub const FPS: u32 = 10;
pub const FRAME_INTERVAL_MS: u64 = 100;
pub const MAX_DURATION_MS: u64 = 15_000;
pub const MAX_FRAME_PIXELS: u64 = 1_000_000;
pub const MAX_SOURCE_PIXELS: u64 = 64_000_000;
pub const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;
pub const QUANT_SPEED: i32 = 10;
pub const MIN_DELAY_CS: u16 = 2;

pub enum GifError { InvalidRegion, TooLarge, NoFrames, Capture(String), Encode(String), Io(String), Thread(String) }
impl GifError { pub fn message(&self, lang: Lang) -> String }

pub fn bgra_to_rgba(src: &[u8], w: u32, h: u32) -> Result<Vec<u8>, GifError>;
pub fn frame_size(w: u32, h: u32) -> Result<(u16, u16), GifError>;
pub fn scale_bgra(img: &Image, w: u16, h: u16) -> Result<Image, GifError>;
pub struct GifWriter<W: Write> { .. }  // new(w, h, out) -> Result, push(rgba, at_ms) -> Result<()>, finish(end_ms) -> Result<W>, frames() -> u32
pub struct TempGif { .. }             // create(dir) -> Result, commit(self, dir, name) -> Result<PathBuf>; Drop xóa
pub fn sweep_stale(dir: &Path) -> usize;

pub struct RecordConfig { pub dir: PathBuf, pub file_name: String, pub interval_ms: u64, pub max_duration_ms: u64, pub max_file_bytes: u64 }
impl RecordConfig { pub fn new(dir: PathBuf, file_name: String) -> Self } // dùng hằng mặc định
pub enum StopReason { Stopped, MaxDuration, MaxSize, SourceLost }
pub enum Outcome { Saved { path: PathBuf, reason: StopReason, frames: u32 }, Cancelled, Failed(GifError) }
pub struct Recorder { .. }
impl Recorder {
    pub fn start<S, W>(cfg: RecordConfig, source: S, wake: W) -> Result<Recorder, GifError>
      where S: FnMut() -> Result<Image, String> + Send + 'static, W: Fn() + Send + 'static;
    pub fn request_stop(&self) -> bool;
    pub fn request_cancel(&self) -> bool;
    pub fn try_outcome(&mut self) -> Option<Outcome>;   // không chặn; trả đúng một lần
    pub fn join(&mut self, timeout: Duration) -> Option<Outcome>;
    pub fn elapsed_ms(&self) -> u64;
}
// naming.rs
pub fn local_gif_name(t: &LocalTime) -> String;  // darkshot_YYYY-MM-DD_HH-MM-SS.gif
// actions.rs: Action::RecordGif sau Scroll (no-op trong run_action); last_capture không nhớ.
```
Tên/kiểu nhỏ có thể chỉnh nếu tài liệu `gif` buộc, nhưng ghi rõ trong báo cáo (Task 123 dựa vào đây).

## Yêu cầu
1. Mọi phép tính kích thước/độ dài dùng `checked_mul`/`try_from`; không `as` cắt cụt âm thầm. `w·h > MAX_SOURCE_PIXELS` → `TooLarge`; cạnh < 2 → `InvalidRegion`.
2. `GifWriter`: một khung chờ; độ trễ theo mốc tích lũy làm tròn cs (thiết kế 3.1); khung giống hệt gộp; `finish` ghi trailer (`into_inner`) và trả writer; đếm byte đã ghi qua writer đếm, vượt giới hạn → báo cho vòng lặp.
3. `TempGif` và `sweep_stale` như thiết kế 3.1; `commit` không bao giờ ghi đè (liên kết cứng + `AlreadyExists` → tên kế; không hỗ trợ → kiểm `exists` rồi `rename`); mọi lỗi I/O → `GifError::Io`, tệp tạm bị xóa.
4. `Recorder`: tạo tệp tạm và luồng trước khi trả về; lỗi spawn → `Thread`, không còn tệp. Trạng thái `AtomicU8` + `Mutex<()>`/`Condvar` (hoặc `park_timeout`) để Dừng/Hủy đánh thức ngay; không vòng bận. Lịch chụp bỏ mốc đã lỡ. Đạt `max_duration_ms` → tự dừng `MaxDuration` (khung cuối có độ trễ tới đúng mốc hết giờ). Nguồn lỗi/đổi kích thước: ≥ 1 khung → lưu `SourceLost`, 0 khung → `Failed(Capture)`. Luồng nền bọc thân bằng xử lý lỗi, không `unwrap`; `wake()` gọi đúng một lần sau khi gửi kết quả. `Drop` → `request_cancel()`.
5. Hiệu năng: đo trong kiểm thử `#[ignore]` thời gian mã hóa một khung 1000×1000 (ảnh tổng hợp có chữ/gradient) ở `--release`; ghi kết quả vào báo cáo. Nếu > 90 ms, đề xuất chỉnh `QUANT_SPEED`/`MAX_FRAME_PIXELS` (không tự đổi thiết kế, ghi lý do).
6. Chuỗi i18n thiết kế 3.4 (vi + en), kể cả `GifError::message`; đoạn Trợ giúp phím `G` nếu bảng phím của Trợ giúp nằm ở `i18n`.

## Kiểm thử (viết trước; ảnh tổng hợp, xác định)
Thiết kế mục 5, phần lõi, đủ từng ca. Đọc lại GIF bằng `gif::DecodeOptions` để khẳng định: số khung, kích thước, độ trễ từng khung, lặp vô hạn (`Decoder::repeat()`), màu điểm ảnh gần đúng (sai số lượng tử hóa). Kiểm thử `Recorder` dùng `RecordConfig` rút ngắn (vd. `interval_ms = 10`, `max_duration_ms = 200`) và thư mục tạm riêng; khẳng định thư mục chỉ còn đúng tệp mong đợi (không `.part`). Mỗi kiểm thử luồng có hạn chờ (không treo CI).

TDD: commit `test:` rồi `feat:`. `cargo fmt --all`; `powershell -File scripts/check.ps1` → `ALL CHECKS PASSED`.

## Review focus
Đua Dừng/Hủy/tự dừng với chốt tệp; không `.part`/GIF dở sót trên mọi đường; không ghi đè; tràn số; bộ nhớ một khung chờ; độ trễ đúng; GIF giải mã lại được.
