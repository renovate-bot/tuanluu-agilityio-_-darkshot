# Task 24: Lõi đợt 3 (định dạng ảnh tải lên)

**Agent:** `builder` · **Model:** sonnet · **Nhánh:** `task/24-upload-format-core` · **Rà soát:** `reviewer`

Gói task tự chứa. Thuộc kế hoạch `docs/superpowers/plans/2026-11-16-upload-format.md`.

## Cần đọc

- `crates/shotme-core/CLAUDE.md`
- Thiết kế đợt 3 `docs/superpowers/specs/2026-11-16-upload-format-design.md` mục 5.2 đến 5.6 và 5.8
- `crates/shotme-core/src/{config,image,upload,actions,i18n}.rs`
- `crates/shotme-windows/src/app.rs` (`UploadJob`, `spawn_upload`, `save_pending`, các `Uploader` giả trong kiểm thử) và các chỗ tạo `Ctx` trong `crates/shotme-windows/src/main.rs`
- Mã nguồn crate `jpeg-encoder` 0.7.1 trong `~/.cargo/registry/src/*/jpeg-encoder-0.7.1/src/encoder.rs` (`Encoder::new`, `set_sampling_factor`, `set_progressive`, `encode`, `ColorType::Bgra`). Lưu ý: mặc định của crate là 4:2:0 khi chất lượng dưới 90, nên phải đặt `SamplingFactor::R_4_4_4` tường minh.

## Global Constraints

- Chỉ thêm đúng một crate `jpeg-encoder = "=0.7.1"` vào `crates/shotme-core/Cargo.toml`, không bật `simd`. Không thêm crate nào khác, kể cả `dev-dependencies`. `Cargo.lock` chỉ thêm đúng một gói (kiểm bằng `git diff Cargo.lock`).
- `shotme-core` không phụ thuộc crate `windows` hay API hệ điều hành.
- Mọi chữ người dùng thấy nằm trong bảng `i18n`, đủ hai ngôn ngữ, không có nhánh `_ =>`. Không dịch "PNG", "JPEG".
- File cấu hình của bản hiện tại (không có trường mới) phải đọc được và cho tải lên PNG. Mọi trường cũ giữ tên và nghĩa. Trường mới sai kiểu làm file thành `Corrupt` như mọi trường hiện nay.
- Chọn PNG thì yêu cầu tải lên giống hệt hôm nay. Các kiểm thử tải lên cũ không được sửa giá trị mong đợi (được sửa tham số gọi hàm vì chữ ký đổi).
- Mọi ảnh ghi vào thư mục lưu là PNG nén từ ảnh gốc: `save_png`, `fallback`, `save_pending`, đường "không tạo được luồng tải lên" không đọc cài đặt mới.
- JPEG: baseline (không progressive), 8 bit, ba kênh, 4:4:4 ở mọi mức chất lượng, không EXIF, không bình luận, mật độ mặc định của crate.
- Release dùng `panic = "abort"`: không `unwrap`/`expect`/chỉ số có thể panic trên đường chạy thật; ép `u32` sang `u16` phải kiểm trước, không dùng `as` cắt cụt.
- Khóa bí mật không lọt vào lỗi, `Debug`, đầu ra kiểm thử. Không đọc hay tìm khóa R2, không đọc file cấu hình thật trong `%APPDATA%`.
- Lớp Windows chỉ sửa tối thiểu để build và để hai giá trị từ `Config` đến được luồng tải lên: không thêm ô, không thêm tab.
- Commit kiểm thử trước (`test:`, thân hàm mới `todo!()`, **không** kèm mã thật), rồi mã (`feat:`). `cargo fmt --all`. Chạy cargo qua PowerShell (`$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"`). Kiểm `git branch --show-current` trước khi commit. Không chuyển nhánh. Không tắt tiến trình `shotme` nào. Không đẩy lên remote. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.

## Interfaces

**Produces (`shotme_core`):**

```rust
// config.rs
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UploadFormat { #[default] Png, Jpeg }
impl UploadFormat {
    pub const ALL: [UploadFormat; 2] = [UploadFormat::Png, UploadFormat::Jpeg];
    pub fn extension(self) -> &'static str;     // "png" | "jpg"
    pub fn content_type(self) -> &'static str;  // "image/png" | "image/jpeg"
    pub fn label(self) -> Msg;                  // Msg::RadioPng | Msg::RadioJpeg
}
pub const JPEG_QUALITY_MIN: u8 = 50;
pub const JPEG_QUALITY_MAX: u8 = 100;
pub const JPEG_QUALITY_DEFAULT: u8 = 90;
pub fn clamp_jpeg_quality(v: i64) -> u8;
// struct Config: thêm `pub upload_format: UploadFormat` và `pub jpeg_quality: u8`
// FileConfig: `upload_format: String` ("png" | "jpeg"; chuỗi khác -> Png, như `expiry`), `jpeg_quality: i64` (kẹp khi đọc)

// image.rs
pub fn encode_jpeg(img: &Image, quality: u8) -> Result<Vec<u8>, EncodeError>;
pub fn encode_for_upload(img: &Image, format: UploadFormat, quality: u8) -> Result<Vec<u8>, EncodeError>;
// enum EncodeError: thêm `TooLarge`; `message()` trả `t(lang, Msg::EncodeTooLargeForJpeg)`

// upload.rs
pub fn object_key(expiry: Expiry, name: &str, format: UploadFormat) -> String;
pub trait Uploader: Send + Sync {
    fn upload(&self, key: &str, body: &[u8], content_type: &str) -> Result<String, UploadError>;
}

// actions.rs: struct Ctx thêm `pub upload_format: UploadFormat`, `pub jpeg_quality: u8`
// i18n.rs: Msg thêm TabFormat, LabelUploadFormat, RadioPng, RadioJpeg, LabelJpegQuality, NoteSavedAlwaysPng, EncodeTooLargeForJpeg
```

`shotme_windows::app::UploadJob` thêm `pub upload_format: UploadFormat` và `pub jpeg_quality: u8`; `main.rs` điền từ bản `Config` đã chụp cho lượt đó. Các `Ctx` chỉ dùng để lưu (không tải lên) điền `UploadFormat::Png` và `JPEG_QUALITY_DEFAULT`.

Lời văn tiếng Việt (chép nguyên): `TabFormat` "Định dạng"; `LabelUploadFormat` "Định dạng ảnh tải lên"; `RadioPng` "PNG (nét nguyên gốc, file nặng hơn)"; `RadioJpeg` "JPEG (file nhẹ hơn, giảm chút chất lượng)"; `LabelJpegQuality` "Chất lượng JPEG"; `NoteSavedAlwaysPng` "Ảnh lưu vào máy luôn là PNG."; `EncodeTooLargeForJpeg` "Ảnh quá lớn để nén JPEG (mỗi cạnh tối đa 65.535 pixel)". English cùng ý, ví dụ `TabFormat` "Format", `LabelUploadFormat` "Upload image format", `LabelJpegQuality` "JPEG quality".

## Các bước

- [ ] **1. Kiểm thử trước.** Thêm phụ thuộc và các chữ ký ở trên với thân `todo!()` (kiểu, hằng, biến thể enum và chuỗi i18n được có thật vì kiểm thử cần biên dịch), sửa mọi chỗ gọi để build. Viết các kiểm thử dưới đây; chạy `cargo test -p shotme-core` và thấy chúng thất bại vì `todo!()`. Commit `test: batch 3 core (upload format, JPEG encoder)`.

  `config.rs`
  - `old_file_uploads_png`: file không có hai trường mới → `upload_format == Png`, `jpeg_quality == 90`.
  - `upload_format_round_trip`: lưu `Jpeg` và 75 rồi đọc lại đúng; file ghi ra có `"upload_format": "jpeg"` và `"jpeg_quality": 75`.
  - `jpeg_quality_is_clamped_on_load`: -1, 0, 49 → 50; 101, 255, 300 → 100; 50, 90, 100 giữ nguyên.
  - `unknown_format_is_png`: `"upload_format": "webp"` → `Png`, không lỗi.
  - `wrong_type_quality_is_corrupt`: `"jpeg_quality": "90"` và `90.5` → `ConfigError::Corrupt`; `"upload_format": 1` → `Corrupt`.
  - `format_extension_and_content_type`: `Png` → `"png"`, `"image/png"`; `Jpeg` → `"jpg"`, `"image/jpeg"`.

  `image.rs` (viết một hàm trợ giúp trong kiểm thử duyệt các đoạn marker JPEG tới SOS)
  - `jpeg_structure`: ảnh 37×21 nhiều màu, chất lượng 50, 89, 90, 100: bắt đầu `FF D8`, kết thúc `FF D9`; có đúng một SOF0 (`FF C0`) với độ chính xác 8, cao 21, rộng 37, 3 thành phần, hệ số lấy mẫu của cả ba là `0x11`; không có `FF C2`; không có APP1 (`FF E1`) và không có COM (`FF FE`).
  - `jpeg_quality_changes_size`: ảnh 64×64 nhiễu giả ngẫu nhiên (sinh bằng công thức cố định, không dùng crate): kích thước ở 100 lớn hơn ở 50.
  - `jpeg_rejects_oversized_and_invalid`: ảnh rỗng và ảnh có `bgra` thiếu byte → `Err(InvalidImage)`; ảnh rộng 65.536 cao 1 (hợp lệ, 262.144 byte) → `Err(TooLarge)`; ảnh 65.535×1 → `Ok`.
  - `encode_for_upload_picks_encoder`: `Png` trả đúng bằng `encode_png(img)`; `Jpeg` trả đúng bằng `encode_jpeg(img, q)`.

  `upload.rs`
  - Sửa các lệnh gọi cũ thành `object_key(.., UploadFormat::Png)` và `upload(key, body, "image/png")`; giá trị mong đợi giữ nguyên.
  - `object_key_follows_format`: `object_key(Expiry::D7, "abc", Jpeg) == "d7/abc.jpg"`.
  - `upload_sends_given_content_type`: `upload("d30/abc.jpg", body, "image/jpeg")` → dòng yêu cầu `PUT /bkt/d30/abc.jpg HTTP/1.1`, header `content-type: image/jpeg`, link kết thúc `/d30/abc.jpg`.

  `actions.rs` (mở rộng `FakeUploader` để ghi lại khóa, thân và kiểu nội dung)
  - `png_upload_is_unchanged`: `Ctx` với `Png`: khóa kết thúc `.png`, kiểu `image/png`, thân đúng bằng `encode_png(img)`.
  - `jpeg_upload_sends_jpeg`: `Ctx` với `Jpeg`, 80: khóa kết thúc `.jpg`, kiểu `image/jpeg`, thân đúng bằng `encode_jpeg(img, 80)`; link được chép vào clipboard; thông báo loại `Success`.
  - `jpeg_upload_failure_saves_png`: `Jpeg`, uploader trả lỗi mạng: `Outcome` là thất bại đã lưu; file lưu có đuôi `.png` và nội dung đúng bằng `encode_png(img)`; thư mục lưu không có file nào khác.
  - `jpeg_encode_failure_falls_back`: `Jpeg` với ảnh 65.536×1: uploader không được gọi; file `.png` được lưu; thông báo loại `Error`.
  - `save_ignores_upload_format`: `Action::Save` với `Jpeg` lưu file `.png` đúng bằng `encode_png(img)`.

- [ ] **2. Mã.** Thay các `todo!()`:
  - `encode_jpeg`: kiểm `is_valid()` → `InvalidImage`; `u16::try_from` cho rộng và cao → `TooLarge`; `Encoder::new(&mut out, quality)` với `quality` đã kẹp bằng `clamp_jpeg_quality`, `set_sampling_factor(SamplingFactor::R_4_4_4)`, `set_progressive(false)`, `encode(&img.bgra, w, h, ColorType::Bgra)`; lỗi của crate → `EncodeError::Encoder(e.to_string())`.
  - `actions::upload`: thay `encode_png(img)` bằng `encode_for_upload(img, ctx.upload_format, ctx.jpeg_quality)`; khóa bằng `object_key(ctx.expiry, &name, ctx.upload_format)`; gọi `upload(&key, &body, ctx.upload_format.content_type())`. Nhánh lỗi nén giữ đúng cách đang làm (`i18n::encode_failed` rồi `fallback`).
  - `R2Uploader::upload` dùng `content_type` nhận vào thay cho hằng `"image/png"`; `test_connection` không đổi.
  - `config`: đọc, ghi, mặc định.
  - `shotme-windows`: `UploadJob` và các `Ctx`.
- [ ] **3.** `cargo fmt --all`; `powershell -File scripts/check.ps1` in `ALL CHECKS PASSED`; `git diff main -- Cargo.lock` chỉ thêm gói `jpeg-encoder`. Commit `feat: upload as PNG or JPEG (core)`.

**Xong khi:** check qua; báo cáo nêu hai commit, dòng cuối của check, số gói thêm vào `Cargo.lock`, mọi chỗ ở lớp Windows đã sửa, và bất kỳ kiểm thử cũ nào đã phải đổi (đổi gì, vì sao).
