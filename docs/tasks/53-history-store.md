# Task 53: Lịch sử – kho trên đĩa, ảnh nhỏ, ghi nguyên tử và đồng thời

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/53-history-store` · **Rà soát:** `specialist`

Thiết kế: `docs/superpowers/specs/2026-11-18-screenshot-history-design.md` mục 1, 4, 5. Global Constraints: mục cùng tên của `docs/superpowers/plans/2026-11-18-screenshot-history.md`. Task 52 đã ở nhánh phát triển (`darkshot_core::history`: `Entry`, `Kind`, `parse_index`, `serialize_index`, `new_id`, `is_valid_id`, `expires_on`, `MAX_ENTRIES`). Thuần lõi, chỉ dùng `std::fs`. Không đụng `darkshot-windows`.

## Cần đọc
`crates/darkshot-core/src/history.rs`, `image.rs` (`Image`, `encode_png`), `actions.rs` (`save_png`: mẫu ghi tệp tạm rồi đổi tên), `Cargo.toml` (crate `png` dùng chung cho giải mã).

## Files
Tạo `crates/darkshot-core/src/history_store.rs`, `crates/darkshot-core/src/thumb.rs` (giải mã PNG, thu nhỏ); sửa `lib.rs`.

## Interfaces

```rust
// thumb.rs
pub const THUMB_MAX_W: u32 = 320;
pub const THUMB_MAX_H: u32 = 200;
pub const MAX_DECODE_SIDE: u32 = 16384;
pub const MAX_DECODE_PIXELS: u64 = 100_000_000;
pub enum DecodeError { TooLarge, Corrupt, Io(String) }
pub fn decode_png(bytes: &[u8]) -> Result<Image, DecodeError>;     // mọi kiểu màu/độ sâu → BGRA 8 bit, alpha giữ nguyên
pub fn decode_png_file(path: &Path) -> Result<Image, DecodeError>; // kiểm kích thước từ header TRƯỚC khi cấp phát
pub fn fit_within(w: u32, h: u32, max_w: u32, max_h: u32) -> (u32, u32); // giữ tỉ lệ, không phóng to, tối thiểu 1×1
pub fn downscale(img: &Image, max_w: u32, max_h: u32) -> Image;   // lọc trung bình diện tích; không phóng to (trả bản sao nếu đã vừa)

// history_store.rs
pub struct Store { root: PathBuf }
impl Store {
    pub fn new(root: PathBuf) -> Store;
    pub fn root(&self) -> &Path;
    /// Nạp chỉ mục. Không có tệp → rỗng. Hỏng cú pháp hoặc version lạ → đổi tên `index.corrupt.json`
    /// (đè bản hỏng cũ), trả rỗng, `Loaded.recovered = true`. Không bao giờ panic.
    pub fn load(&self) -> Loaded;
    /// Ghi một mục mới (đồng bộ). Xem dưới.
    pub fn record(&self, new: NewEntry, img: &Image) -> Result<Entry, StoreError>;
    pub fn remove(&self, id: &str) -> Result<bool, StoreError>;      // xóa mục + images/<id>.png + thumbs/<id>.png
    pub fn clear(&self) -> Result<usize, StoreError>;                // số mục đã xóa; xóa mọi tệp do ứng dụng giữ
    /// Ảnh nhỏ của mục: đọc `thumbs/<id>.png` (giải mã); thiếu/hỏng thì sinh lại từ `image_path` rồi ghi.
    pub fn thumbnail(&self, e: &Entry) -> Result<Image, DecodeError>;
}
pub struct Loaded { pub entries: Vec<Entry>, pub skipped: usize, pub recovered: bool }
pub struct NewEntry {
    pub kind: Kind, pub unix_millis: u64, pub local: LocalTime,
    pub path: Option<PathBuf>,   // Saved/UploadFallback
    pub url: Option<String>,     // Uploaded
    pub expires: Option<String>,
}
#[derive(Debug)] pub enum StoreError { Io(String), Invalid }
pub fn lock_for_tests() -> std::sync::MutexGuard<'static, ()>; // chỉ nếu cần; khóa chính là static Mutex<()> toàn tiến trình dùng cho mọi Store
```

### Hành vi `record`
1. Kiểm `img.is_valid()` và `kind`/`path`/`url` hợp lệ (Saved/UploadFallback cần `path`; Uploaded cần `url` bắt đầu `https://`; Copied không cần gì) → sai thì `Invalid`.
2. TRƯỚC khi lấy khóa: sinh `id = new_id(unix_millis)`; ảnh nhỏ = `downscale(img, 320, 200)` → `encode_png`; nếu `kind` là Uploaded/Copied thì `encode_png(img)` làm bản sao quản lý (`managed = true`), `bytes = len`; nếu Saved/UploadFallback thì `bytes = metadata(path).len()` (không có thì `None`).
3. Ghi `images/<id>.png` và `thumbs/<id>.png` bằng tệp tạm cùng thư mục + `rename` (tạo thư mục khi cần).
4. Lấy khóa toàn tiến trình `static LOCK: Mutex<()>` (bị nhiễm độc thì vẫn dùng `into_inner`): đọc chỉ mục MỚI NHẤT từ đĩa (như `load`), chèn mục mới ở đầu, nếu hơn `MAX_ENTRIES` thì bỏ mục cuối, ghi chỉ mục bằng `index.json.tmp` + `rename` đè, thả khóa.
5. SAU khi thả khóa: xóa tệp của các mục bị đẩy ra (images, thumbs) – chỉ khi `is_valid_id(id)` và đường dẫn nằm trong `root`. Quét mồ côi: tệp `<id>.png` hợp lệ trong `images/`, `thumbs/` không có trong chỉ mục (đọc lại dưới khóa) và cũ hơn 60 giây thì xóa; tệp lạ (tên không khớp mẫu) giữ nguyên.
6. Lỗi ở bước ghi chỉ mục: xóa hai tệp vừa ghi, trả `Err`. Chỉ mục cũ không đổi.

### `remove`/`clear`
Dưới khóa: đọc, bỏ mục, ghi nguyên tử; rồi xóa tệp của mục (images/thumbs) theo cùng quy tắc an toàn. Không bao giờ xóa `path` của người dùng. `clear` ghi chỉ mục rỗng rồi xóa mọi `<id>.png` hợp lệ trong `images/`, `thumbs/` (không xóa tệp lạ, không xóa `index.corrupt.json`).

### `thumbnail`
Thư mục `thumbs` thiếu hoặc tệp hỏng: giải mã `image_path` (`decode_png_file`), `downscale`, ghi lại (nguyên tử, không cần khóa chỉ mục), trả ảnh. Tệp nguồn mất → `Err(Io)`.

## Kiểm thử (viết trước; thư mục tạm riêng mỗi kiểm thử; tên `darkshot_hist_*`)
- `decode_png` đủ kiểu màu (Gray, GrayA, Rgb, Rgba, Indexed, 16-bit) sinh bằng `png::Encoder` trong kiểm thử; thứ tự BGRA đúng; bom giải mã: header khai 20000×20000 → `TooLarge` mà không cấp phát; byte rác → `Corrupt`.
- `fit_within`: không phóng to (100×50 trong 320×200 → 100×50), giữ tỉ lệ (1920×1080 → 320×180), cạnh cực mỏng (10000×1 → 320×1), không bao giờ 0. `downscale` ra đúng kích thước; ảnh một màu giữ màu; kênh alpha giữ.
- `record` Saved: không tạo `images/`, có `thumbs/`, `bytes` = cỡ tệp, `path` giữ. Uploaded: có bản sao, `url`, `expires`; Copied: có bản sao. `Uploaded` với `url` không phải https → `Invalid` và không để lại tệp nào.
- Thứ tự mới nhất trước; vòng `load` giữ mọi trường.
- Giữ 50: ghi 55 mục → `load` còn 50 là 50 mục mới nhất; tệp `images/`, `thumbs/` của 5 mục cũ bị xóa; tệp `path` của mục Saved bị đẩy ra VẪN CÒN.
- `remove` xóa đúng mục và tệp của nó, trả false khi không có; `path` người dùng còn nguyên. `remove("../index.json")` → không xóa gì (id không hợp lệ → `Ok(false)`).
- `clear` xóa hết, tệp người dùng còn, tệp lạ trong `images/` còn.
- Chỉ mục hỏng: ghi rác vào `index.json` → `load` trả rỗng, `recovered`, `index.corrupt.json` có nội dung rác, và `record` kế tiếp chạy bình thường. `version:99` tương tự. Chỉ mục có 2 mục tốt + 1 mục hỏng → `skipped = 1`.
- Ghi nguyên tử: sau `record` không còn `index.json.tmp`. Giả lập lỗi ghi chỉ mục (đặt một THƯ MỤC tên `index.json` để `rename` thất bại) → `Err`, không để lại `images/<id>.png` mới.
- Đồng thời: 8 luồng × 10 `record` đồng thời (cùng `Store`, và cũng hai `Store` cùng `root`) → `load` đủ 80 mục, id đều khác nhau, chỉ mục hợp lệ; sau đó 4 luồng vừa `record` vừa `remove` không sinh chỉ mục hỏng. Trần 50 vẫn đúng dưới tải (ghi 120 từ 8 luồng → đúng 50, và tệp mồ côi được quét bớt).
- `thumbnail`: có sẵn → đọc; xóa `thumbs/<id>.png` → sinh lại từ `image_path`; tệp nguồn Saved bị xóa và không có thumb → `Err`; thumb hỏng (ghi rác) → sinh lại.
- Mất tệp Saved không làm `load` lỗi.

## Việc cần chủ dự án thử tay
Không.

## Báo cáo
Theo mẫu `.claude/agents/specialist.md`. Nêu rõ chỗ chọn thời gian "cũ hơn 60 giây" cho quét mồ côi và lý do.
