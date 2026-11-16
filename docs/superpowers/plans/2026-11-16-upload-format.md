# shotme đợt 3 (định dạng ảnh tải lên: PNG hoặc JPEG) Implementation Plan

> **For agentic workers:** kế hoạch này chạy bằng quy trình của dự án: `/next-task` giao từng gói `docs/tasks/NN-ten-ngan.md` cho agent ghi trong `memory-bank/progress.md`, rồi rà soát, kiểm tra, gộp. Các bước chi tiết (dạng `- [ ]`) nằm trong gói task; file này giữ phần chung mà mọi gói dựa vào.

**Goal:** Tab thứ tư "Định dạng" trong Tùy chọn cho chọn PNG hoặc JPEG và chất lượng JPEG (50–100) cho ảnh tải lên; ảnh lưu vào máy luôn là PNG.

**Architecture:** Core giữ hết luật: kiểu `UploadFormat` và chất lượng trong `Config`, bộ nén `encode_jpeg` (crate `jpeg-encoder`, baseline, 4:4:4), đuôi khóa và kiểu nội dung theo định dạng, luồng `actions::upload` chọn bộ nén. Lớp Windows chỉ truyền hai giá trị từ `Config` vào `Ctx`/`UploadJob` (đã là bản chụp riêng cho từng lượt) và thêm tab với hai nút radio và một thanh trượt. Mọi đường lưu vào máy vẫn đi qua `save_png`, không đọc cài đặt mới.

**Tech Stack:** như các kế hoạch trước, thêm đúng một crate `jpeg-encoder = "=0.7.1"` (`default-features` giữ nguyên là `std`, không bật `simd`) trong `shotme-core`. Lớp Windows dùng `msctls_trackbar32` (`ICC_BAR_CLASSES`); feature `Win32_UI_Controls` và `Win32_Graphics_GdiPlus` đã bật sẵn.

**Spec:** `docs/superpowers/specs/2026-11-16-upload-format-design.md` (gọi là "thiết kế đợt 3"). Thiết kế đợt 1, đợt 2 và spec gốc vẫn đúng ở mọi chỗ thiết kế đợt 3 không nói khác.

## Global Constraints

- Chỉ mở kết nối mạng đến hai nơi: `https://{account_id}.r2.cloudflarestorage.com` và địa chỉ công khai của kho.
- Secret Access Key không bao giờ xuất hiện ở dạng đọc được trên đĩa, trong log, trong thông báo lỗi, trong `Debug` hay đầu ra kiểm thử.
- Không ghi log ra file. Chỉ thêm đúng một crate `jpeg-encoder` (ghim `=0.7.1`, không bật `simd`) vào `shotme-core`; không thêm crate nào khác, kể cả `dev-dependencies`. `Cargo.lock` chỉ thêm đúng một gói.
- `shotme-core` không phụ thuộc crate `windows` hay API riêng của hệ điều hành.
- Mọi chữ người dùng nhìn thấy lấy từ bảng `shotme_core::i18n`, đủ hai ngôn ngữ; không viết cứng. Không dịch tên riêng "PNG", "JPEG".
- File cấu hình của bản hiện tại (không có trường mới) phải đọc được và cho đúng hành vi hiện tại: tải lên PNG. Mọi trường cũ giữ tên và nghĩa.
- Chọn PNG thì yêu cầu tải lên giống hệt hôm nay; các kiểm thử tải lên cũ không được sửa giá trị mong đợi (được sửa tham số gọi hàm nếu chữ ký đổi).
- Mọi ảnh ghi vào thư mục lưu là PNG nén từ ảnh gốc: Ctrl+S, phím lưu toàn màn hình, lưu dự phòng khi tải lên hoặc nén lỗi, lưu khi thoát lúc đang tải, lưu khi không tạo được luồng tải lên.
- JPEG: baseline, 8 bit, ba kênh, lấy mẫu 4:4:4 ở mọi mức chất lượng, không EXIF, không bình luận. Tên trên kho đuôi `.jpg`, kiểu nội dung `image/jpeg`.
- Bản release dùng `panic = "abort"`: mã chạy thật không `unwrap`/`expect`/chỉ số có thể panic trên dữ liệu đầu vào. `unsafe` nhỏ, kèm `// SAFETY:`. Giải phóng mọi handle trên mọi đường đi.
- Hành vi đã qua rà soát của cửa sổ Tùy chọn giữ nguyên trừ chỗ thiết kế đợt 3 đổi: chuyển tab bằng chuột, Ctrl+Tab, Ctrl+Shift+Tab; máy trạng thái bắt phím; lỗi nhảy đúng tab; OK, Hủy; Kiểm tra kết nối chạy nền. Không thêm phím chuyển tab mới.
- Không làm ở đợt này: JPEG cho ảnh lưu vào máy, định dạng khác, progressive, xem trước, ước lượng dung lượng, Proxy.
- Quy trình: commit kiểm thử trước (`test:`, thân hàm mới là `todo!()`), rồi commit mã (`feat:`). Chạy `cargo fmt --all` trước khi commit. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`. Không chuyển nhánh. Chạy cargo qua PowerShell. Không tắt tiến trình `shotme` không do mình khởi động; nếu exe bị khóa thì bỏ build release và báo lại. Không đẩy lên remote. Không đọc hay ghi khóa R2, không đọc file cấu hình thật trong `%APPDATA%`.
- Kiểm thử mới ở crate Windows không khẳng định chữ đọc từ `lang::tr` (ngôn ngữ hiện hành là biến toàn cục); dùng `t(Lang::Vi, …)`.

## Review Focus

Năm tình huống thiết kế hàm ý mà dễ gặp khi dùng thật. Mỗi dòng có kiểm thử ở task ghi trong ngoặc.

1. Nâng cấp từ bản đợt 2: file cấu hình không có trường mới thì tải lên PNG, khóa `.png`, `image/png`, thân đúng bằng `encode_png`. (Task 24: `old_file_uploads_png`, `png_upload_is_unchanged`)
2. Đang chọn JPEG mà tải lên lỗi (rút mạng): file lưu dự phòng có đuôi `.png` và nội dung đúng bằng `encode_png` của ảnh gốc, không phải JPEG đổi tên. (Task 24: `jpeg_upload_failure_saves_png`)
3. File cấu hình sửa tay: chất lượng -1, 0, 49, 101, 255, 300 được kẹp về 50 hoặc 100; tên định dạng lạ là PNG; 90.5 hay "90" là file hỏng như mọi trường sai kiểu. (Task 24: `jpeg_quality_is_clamped_on_load`, `unknown_format_is_png`, `wrong_type_quality_is_corrupt`)
4. Chọn JPEG với ảnh không nén được (ảnh không hợp lệ, hoặc cạnh dài hơn 65.535 pixel): đi đường lưu dự phòng PNG và báo lỗi, không panic, không cắt cụt kích thước khi ép kiểu. (Task 24: `jpeg_rejects_oversized_and_invalid`, `jpeg_encode_failure_falls_back`)
5. Chọn PNG rồi OK khi thanh trượt đang ở 70: giá trị 70 vẫn được lưu và hiện lại khi mở cửa sổ; thanh mờ không nhận phím; Ctrl+Tab từ thanh trượt vẫn chuyển tab và đi đủ bốn tab. (Task 25: `quality_survives_png_selection`, `tab_order_has_four_tabs`, và mục checklist)

## Cấu trúc file

| File | Thay đổi | Task |
|---|---|---|
| `crates/shotme-core/Cargo.toml`, `Cargo.lock` | thêm `jpeg-encoder` | 24 |
| `crates/shotme-core/src/config.rs` | `UploadFormat`, hằng chất lượng, `clamp_jpeg_quality`, hai trường `Config`, hai trường file | 24 |
| `crates/shotme-core/src/image.rs` | `encode_jpeg`, `encode_for_upload`, `EncodeError::TooLarge` | 24 |
| `crates/shotme-core/src/upload.rs` | `object_key` nhận định dạng, `Uploader::upload` nhận kiểu nội dung | 24 |
| `crates/shotme-core/src/actions.rs` | `Ctx.upload_format`, `Ctx.jpeg_quality`, `upload` chọn bộ nén | 24 |
| `crates/shotme-core/src/i18n.rs` | chuỗi của tab (24), đoạn Trợ giúp (25) | 24, 25 |
| `crates/shotme-windows/src/{app,main}.rs` | truyền hai giá trị vào `UploadJob` và `Ctx` | 24 |
| `crates/shotme-windows/src/settings.rs` | `Tab::Format`, ô radio, thanh trượt, form | 25 |
| `crates/shotme-windows/src/capture.rs` hoặc file kiểm thử phù hợp | kiểm thử giải nén JPEG bằng GDI+ | 25 |
| `docs/testing/manual-checklist.md` | mục "Đợt 3: định dạng" | 25 |
| `README.md`, `memory-bank/{productContext,systemPatterns,techContext}.md` | tài liệu | 26 |

## Interfaces chung

```rust
// shotme_core::config
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
pub fn clamp_jpeg_quality(v: i64) -> u8;        // kẹp vào MIN..=MAX
// Config thêm: pub upload_format: UploadFormat, pub jpeg_quality: u8 (luôn trong MIN..=MAX sau khi load)
// File: "upload_format": "png" | "jpeg" (chuỗi lạ -> Png), "jpeg_quality": số nguyên (i64 rồi kẹp)

// shotme_core::image
pub fn encode_jpeg(img: &Image, quality: u8) -> Result<Vec<u8>, EncodeError>;
pub fn encode_for_upload(img: &Image, format: UploadFormat, quality: u8) -> Result<Vec<u8>, EncodeError>;
// EncodeError thêm biến thể TooLarge (cạnh > 65_535), lời văn qua Msg::EncodeTooLargeForJpeg

// shotme_core::upload
pub fn object_key(expiry: Expiry, name: &str, format: UploadFormat) -> String;
pub trait Uploader: Send + Sync {
    fn upload(&self, key: &str, body: &[u8], content_type: &str) -> Result<String, UploadError>;
}

// shotme_core::actions::Ctx thêm: pub upload_format: UploadFormat, pub jpeg_quality: u8
// shotme_windows::app::UploadJob thêm hai trường cùng tên

// shotme_core::i18n::Msg thêm (Task 24):
// TabFormat, LabelUploadFormat, RadioPng, RadioJpeg, LabelJpegQuality, NoteSavedAlwaysPng, EncodeTooLargeForJpeg
```

Lời văn tiếng Việt chốt: `TabFormat` "Định dạng" / "Format"; `LabelUploadFormat` "Định dạng ảnh tải lên"; `RadioPng` "PNG (nét nguyên gốc, file nặng hơn)"; `RadioJpeg` "JPEG (file nhẹ hơn, giảm chút chất lượng)"; `LabelJpegQuality` "Chất lượng JPEG"; `NoteSavedAlwaysPng` "Ảnh lưu vào máy luôn là PNG."; lời English do người làm viết cùng ý.

## Task

| Task | Gói | Agent | Model | Rà soát |
|---|---|---|---|---|
| 24 Lõi đợt 3 | `docs/tasks/24-upload-format-core.md` | builder | sonnet | reviewer |
| 25 Tab Định dạng | `docs/tasks/25-format-tab.md` | specialist | opus | specialist |
| 26 Tài liệu đợt 3 | `docs/tasks/26-docs-batch3.md` | scaffolder | haiku | reviewer |

Thứ tự 24 → 25 → 26. Chủ dự án đã bảo chạy đến hết không hỏi lại (2026-11-16), nên không dừng sau Task 25; việc thử tay theo mục "Đợt 3: định dạng" được báo ở cuối và tài liệu không được ghi là đã thử tay.
