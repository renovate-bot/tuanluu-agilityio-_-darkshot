# Thiết kế: Ghi màn hình thành GIF (đợt 20)

Ngày: 2026-11-21. Yêu cầu của chủ dự án (lời giao "Implement Lightweight Screen Recording (GIF)"). Đợt 16–19 và task 115–121 đã được kế hoạch MCP (nhánh `task/115-mcp-design`, chưa gộp) giữ chỗ, nên đợt này là **đợt 20**, task **122–124**.

Nguyên tắc: phần mở rộng nhỏ của chụp màn hình, không phải trình dựng video. Không trang cài đặt, không trình sửa GIF, không xem trước, không tải lên, không Lịch sử.

## 1. Luồng người dùng

1. Mở lớp phủ như thường (PrintScreen, menu khay). Chọn vùng.
2. Thanh ngang của lớp phủ có thêm nút **Ghi GIF** (giữa Chụp cuộn và Hủy; phím `G` và `Ctrl+G`, như `L` của Chụp cuộn). Chú thích: "Ghi GIF (Ctrl+G) · Không gồm hình đánh dấu" (dùng lại `Msg::ScrollNoAnnotations`).
3. Bấm: lớp phủ đóng, bắt đầu ghi NGAY vùng đã chọn (tọa độ màn hình vật lý, đổi bằng `capture::to_screen_coords` sau khi kẹp vào ảnh, như `scroll_region`).
4. Khung mảnh quanh vùng (như chụp cuộn) và thanh ghi nổi: `● Đang ghi 00:05 / 00:15   [Dừng] [Hủy]`.
5. **Dừng**: nút đổi trạng thái "Đang lưu…" (nút mờ), luồng nền chốt GIF, đổi tên tệp tạm sang tên cuối; thanh và khung đóng; thông báo "Đã lưu GIF" kèm tên tệp. Thông báo thành công tuân theo công tắc "hiện thông báo thành công" như Lưu (loại `NoticeKind::Success`).
6. **Hủy**: dừng, xóa tệp tạm, không tạo GIF, không thông báo.
7. Đủ 15 s: tự chốt và lưu như Dừng; thông báo "Đã đạt thời lượng ghi tối đa (15 giây). Đã lưu GIF: …" (loại Info, luôn hiện).

Không có phím tắt toàn cục mới, không phím Esc toàn cục tạm (Esc phải tới được ứng dụng đang được ghi).

## 2. Mặc định (hằng số trong lõi, không cấu hình)

| Hằng | Giá trị |
|---|---|
| `FPS` / `FRAME_INTERVAL_MS` | 10 / 100 |
| `MAX_DURATION_MS` | 15 000 |
| `MAX_FRAME_PIXELS` | 500 000 (khung lớn hơn được thu nhỏ đều hai chiều, giữ tỉ lệ) |
| `MAX_SIDE` | 65 535 (giới hạn của GIF; sau thu nhỏ luôn thỏa) |
| `MIN_SIDE` | 2 px mỗi chiều (nhỏ hơn: "Vùng ghi không hợp lệ") |
| `MAX_FILE_BYTES` | 256 MiB (quá thì dừng và lưu phần đã ghi, như đạt thời lượng) |
| `QUANT_SPEED` | 20 (`Frame::from_rgba_speed`; 1 = đẹp chậm, 30 = nhanh) |
| `MIN_DELAY_CS` | 2 (trình duyệt coi < 2 cs là 10 cs) |
| Lặp | `Repeat::Infinite` |
| Con trỏ, âm thanh | Không |

Thu nhỏ: hệ số `s = sqrt(MAX_FRAME_PIXELS / (w·h))`, kích thước đích `floor(w·s)`, `floor(h·s)` (tối thiểu 1), lọc trung bình vùng (box) trên BGRA. Một vùng 4K thành khoảng 942×530 (đo ở Task 122: 1 MP mất ~270 ms mỗi khung). Ghi rõ trong README là GIF có thể nhỏ hơn vùng chọn với vùng rất lớn.

## 3. Kiến trúc

### 3.1 Lõi: `darkshot-core/src/gif_record.rs` (mới, crate `gif = "0.14"`)

Không phụ thuộc `windows`. Thuần Rust, kiểm thử được hết.

- `bgra_to_rgba(src: &[u8], w: u32, h: u32) -> Result<Vec<u8>, GifError>`: kiểm `w·h·4` bằng phép nhân có kiểm tra, độ dài khớp.
- `frame_size(w: u32, h: u32) -> Result<(u16, u16), GifError>`: kiểm `MIN_SIDE`, áp thu nhỏ, trả kích thước khung GIF.
- `scale_bgra(img: &Image, tw: u16, th: u16) -> Result<Image, GifError>` (bỏ qua khi bằng kích thước).
- `GifWriter<W: Write>`: `new(w: u16, h: u16, out: W)`, `push(rgba: Vec<u8>, at_ms: u64)`, `finish(end_ms: u64) -> Result<W, GifError>`. Giữ đúng **một** khung chờ (RGBA) để biết độ trễ khi khung kế tới: độ trễ = `round(t_next/10) − round(t_prev/10)` theo mốc tích lũy (không trôi), tối thiểu `MIN_DELAY_CS`, tối đa `u16::MAX`. Khung giống hệt khung chờ thì gộp (kéo dài độ trễ), không mã hóa lại. Khung cuối nhận độ trễ tới `end_ms`. Không khung nào → `GifError::NoFrames`.
- `TempGif`: tệp tạm `.darkshot-rec-<16 ký tự ngẫu nhiên>.gif.part` trong thư mục đích, `create_new`. `commit(self, dir, name) -> Result<PathBuf, GifError>`: `flush` + `sync_all`, đóng tệp, rồi đặt tên cuối KHÔNG ghi đè: `naming::unique_path` + `std::fs::hard_link` (lỗi `AlreadyExists` → thử tên kế), hệ tệp không hỗ trợ liên kết cứng thì `rename` sau khi kiểm lại `exists`; xóa tệp tạm. `Drop` khi chưa commit: xóa tệp tạm. `sweep_stale(dir)`: xóa các tệp đúng mẫu `.darkshot-rec-*.gif.part` (không đụng tệp khác).
- `naming::local_gif_name(t) -> String` = `darkshot_YYYY-MM-DD_HH-MM-SS.gif` (dùng chung định dạng với `local_file_name`).
- Lịch chụp `Pacer`: mốc thứ k là `k·100 ms`; sau một lượt chậm, bỏ các mốc đã lỡ (không xếp hàng), chờ tới mốc kế tiếp chưa qua. Không vòng bận: `Condvar::wait_timeout` (hoặc `park_timeout`) để Dừng/Hủy đánh thức ngay.
- `Recorder` (luồng nền, chung cho mọi nền tảng):
  - `Recorder::start(cfg: RecordConfig, source: S, wake: W) -> Result<Recorder, GifError>` với `S: FnMut() -> Result<Image, String> + Send + 'static` (một lượt chụp), `W: Fn() + Send + 'static` (đánh thức luồng giao diện khi có kết quả). Tạo tệp tạm và luồng (`thread::Builder::spawn`, lỗi → `GifError::Thread`, tệp tạm đã xóa) TRƯỚC khi trả về.
  - Trạng thái chung `AtomicU8`: `Running → Stopping | Cancelling`, chuyển bằng `compare_exchange` một lần; `request_stop()`/`request_cancel()` trả `bool` (có đổi được không), gọi nhiều lần an toàn. Luồng nền tự chuyển `Running → Stopping` khi đạt thời lượng, kích thước tệp hay lỗi chụp; Hủy tới sau đó thì không có tác dụng (không đua với chốt tệp).
  - Vòng lặp: chụp → (thu nhỏ) → RGBA → `GifWriter::push` (mã hóa khung chờ trước đó) → giải phóng. Chỉ một lượt chụp tại một thời điểm. Khung chụp lỗi: có ≥ 1 khung thì dừng và lưu phần đã có (`Outcome::Saved { reason: SourceLost }`), chưa có khung nào thì `Failed(Capture)`. Khung đổi kích thước (đổi DPI/màn hình): như lỗi chụp.
  - Kết quả qua `mpsc` một lần: `Outcome::{Saved { path, reason: Stopped | MaxDuration | MaxSize | SourceLost, frames }, Cancelled, Failed(GifError)}`; rồi gọi `wake()`. `try_outcome()` không chặn; `join(timeout)` chờ có hạn (cho lúc thoát).
  - `Drop` của `Recorder`: `request_cancel()` (không chặn); luồng nền vẫn dọn tệp tạm.
- `GifError`: `InvalidRegion`, `TooLarge`, `NoFrames`, `Capture(String)`, `Encode(String)`, `Io(String)`, `Thread(String)`; `message(lang)` lấy chữ từ `i18n`.

### 3.2 Windows: `darkshot-windows/src/record_session.rs` (mới)

Theo mẫu `scroll_session.rs` (thanh, khung, đặt vị trí, `WDA_EXCLUDEFROMCAPTURE`, DPI), nhưng không dùng chung trạng thái với phiên chụp cuộn hay chụp từng bước. Được phép đổi một số hàm thuần của `scroll_session.rs` thành `pub(crate)` (đặt vị trí, khung, bản dựng Windows) để không chép lại; không đổi hành vi của chúng.

- `start(region: Rect, save_dir: PathBuf) -> Result<(), StartError>`: một phiên tối đa (`Busy`), vùng rỗng (`Empty`), không có chỗ đặt thanh (`NoRoom`), lỗi cửa sổ (`Failed`), lỗi `Recorder::start` (`Recorder(GifError)`). Thanh và khung tạo xong mới bắt đầu ghi; lỗi ở bất kỳ bước nào dọn sạch cái đã tạo.
- Nguồn chụp: `capture::capture_rect(region)` gọi trên luồng nền (BitBlt + DIB của riêng luồng, không có HWND; `DpiScope` đặt theo luồng). Không gọi hàm giao diện Win32 nào trên luồng nền.
- Đánh thức: `PostMessageW(host, WM_RECORD_DONE = WM_APP + 18)`; luồng giao diện gọi `try_outcome()`, dọn cửa sổ, gọi callback kết quả của `main`.
- Thanh: `WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW`, `MA_NOACTIVATE`, `SetWindowDisplayAffinity(WDA_EXCLUDEFROMCAPTURE)` khi bản dựng ≥ 19041; khung quanh vùng cũng loại khỏi ảnh. Bản dựng cũ hơn: không vẽ khung, thanh đặt NGOÀI vùng (không có chỗ → `NoRoom`), giống chụp cuộn, nên không cần ẩn/hiện quanh từng khung. Kiểm thử chạy tay xác nhận `GetWindowDisplayAffinity` và pixel của vùng chụp không chứa thanh (mẫu `scroll_session::live`).
- Hẹn giờ giao diện 250 ms chỉ để vẽ lại thời gian (tính từ `Instant` lúc bắt đầu, dừng ở 15 s); không chụp trên luồng giao diện.
- `WM_DISPLAYCHANGE`/`WM_DPICHANGED`: đặt lại thanh; vùng không còn nằm trọn trên màn hình nào → `request_stop()` (lưu phần đã ghi).
- `active() -> bool`, `stop()`, `cancel()`, `finish_on_exit(deadline)` (Dừng rồi chờ `join` tới hạn chót; quá hạn thì `request_cancel` và bỏ; tệp tạm còn sót được `sweep_stale` dọn lần khởi động sau).

### 3.3 Nối vào ứng dụng

- `Action::RecordGif` (lõi, sau `Scroll`): `run_action` no-op, `last_capture::remember` giữ cái cũ, không ghi "record" vào JSON, `remember_after` giữ khung cũ.
- `overlay.rs`: ô `BarItem::Act(Action::RecordGif)` giữa Scroll và Cancel; biểu tượng tự vẽ (vòng tròn đặc nhỏ trong khung bo góc, cùng lưới 24 với các ô khác); phím `G`/`Ctrl+G`; kiểm thử bố cục thanh ở 96/120/144/192 DPI có và không có phiên các bước.
- `main.rs`: nhánh `Action::RecordGif` như `Action::Scroll` (bỏ ảnh tĩnh, hình đánh dấu); `save_dir` lấy từ cấu hình như Lưu; trong lúc ghi cổng chụp bỏ qua yêu cầu chụp (`scroll_blocks` mở rộng thành "đang bận": chụp cuộn hoặc đang ghi), không bắt đầu chụp từng bước, không mở chụp cuộn. Thanh phiên Chụp từng bước không loại được → giữ ẩn tới khi ghi xong (như `STEPS_BAR_HELD`). Thoát (`exit_cleanup`, `WM_ENDSESSION`, `WM_DESTROY`): **chính sách thoát = lưu phần đã ghi** trong hạn chót chung `EXIT_BUDGET`. Khởi động: `sweep_stale(save_dir)`.
- Thông báo: thành công (Success) "Đã lưu GIF" + tên tệp; đạt tối đa (Info); lỗi (Error) "Ghi GIF thất bại" + lý do; vùng không hợp lệ; mã hóa GIF thất bại.
- Không đổi định dạng cấu hình, không đổi PNG/JPEG, không tải lên, không Lịch sử.

### 3.4 Chuỗi (`i18n.rs`, Tiếng Việt và English)

`OverlayRecordGif` (Ghi GIF / Record GIF), `RecordStatus` (Đang ghi / Recording), `RecordSaving` (Đang lưu… / Saving…), `RecordStop` (Dừng / Stop), `RecordCancel` (Hủy / Cancel), `RecordSavedTitle` (Đã lưu GIF / GIF saved), `RecordFailedTitle` (Ghi GIF thất bại / Recording failed), `RecordInvalidRegion`, `RecordMaxDuration`, `RecordMaxSize`, `RecordSourceLost`, `RecordEncodeFailed`, `RecordBusy`, chữ Trợ giúp cho phím `G`. Tên chính xác do Task 122/123 chốt.

## 4. Bộ nhớ và hiệu năng

Tối đa trong bộ nhớ: một khung BGRA đang chụp (≤ kích thước vùng), một bản thu nhỏ/RGBA (≤ 2 MB) và một khung chờ RGBA (≤ 2 MB), cộng bộ đệm của bộ mã hóa. Vùng rất lớn (8K, 33 MP): khung BGRA thô ~133 MB chỉ tồn tại trong một lượt; giới hạn trên `w·h ≤ 64 MP` (`TooLarge`) để không cấp phát quá. Không `Vec<Image>`, không hàng đợi khung.

## 5. Kiểm thử

Lõi (xác định, ảnh tổng hợp): đổi BGRA→RGBA; kích thước sai/tràn; thu nhỏ đúng kích thước và màu trung bình; độ trễ (100 ms → 10 cs, mốc lệch 95/105/250 ms không trôi, tối thiểu 2); gộp khung giống hệt; `Repeat::Infinite` và số khung, kích thước khung, độ trễ đọc lại bằng `gif::Decoder`; không khung → lỗi, không tệp; tệp tạm: commit không ghi đè (`_2`), drop xóa, sweep chỉ xóa đúng mẫu. `Recorder` với nguồn giả và đồng hồ giả hoặc thời lượng rút ngắn qua `RecordConfig`: bắt đầu → dừng → GIF hợp lệ; hủy → không có tệp nào; dừng/hủy nhiều lần; hủy sau khi luồng tự dừng không đua; nguồn lỗi ngay → `Failed`, không tệp; nguồn lỗi sau vài khung → lưu phần đã có; đạt thời lượng → `MaxDuration`; quá `MAX_FILE_BYTES` (giá trị nhỏ trong cấu hình kiểm thử) → `MaxSize`; không có tệp `.part` nào sau mọi đường.

Windows: bố cục thanh ghi theo DPI, chạm nút, chuỗi; `remember_after`/cổng bận; kiểm thử chạy tay `#[ignore]` chụp vùng thật và xác nhận thanh không có trong khung chụp.

Thử tay: mục "Đợt 20: Ghi GIF" trong `docs/testing/manual-checklist.md` (15 mục của lời giao).

## 6. Giới hạn đã biết

Không con trỏ chuột, không âm thanh, tối đa 15 s, 10 khung/giây (máy chậm thì ít hơn, độ trễ vẫn đúng thời gian thật), bảng màu 256 màu mỗi khung (gradient có thể bị dải màu), vùng > 0,5 MP được thu nhỏ, cửa sổ thông báo của Windows hiện trong lúc ghi có thể lọt vào GIF.
