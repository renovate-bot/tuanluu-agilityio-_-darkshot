# shotme đợt 1 (menu mới, Tùy chọn dạng tab, hai ngôn ngữ) Implementation Plan

> **For agentic workers:** kế hoạch này chạy bằng quy trình của dự án: `/next-task` giao từng gói `docs/tasks/NN-ten-ngan.md` cho agent ghi trong `memory-bank/progress.md`, rồi rà soát, kiểm tra, gộp. Các bước chi tiết (dạng `- [ ]`) nằm trong gói task; file này giữ phần chung mà mọi gói dựa vào.

**Goal:** Menu khay năm mục, cửa sổ Tùy chọn ba tab, cửa sổ Thông tin và Trợ giúp, toàn bộ chữ có bản Tiếng Việt và English đổi được ngay.

**Architecture:** Một bảng chuỗi duy nhất trong `shotme-core::i18n`, mỗi mục bắt buộc có đủ hai bản lúc biên dịch. Core không giữ ngôn ngữ hiện hành: hàm nào sinh chữ thì nhận `Lang`. `shotme-windows` giữ ngôn ngữ hiện hành trong một biến nguyên tử (`lang.rs`). Cửa sổ Tùy chọn giữ cách dựng bằng mã, thêm `SysTabControl32` với ba khung con ẩn/hiện. Thông tin và Trợ giúp là cửa sổ không chặn, đứng ngoài cổng chụp.

**Tech Stack:** như kế hoạch gốc. Không thêm crate. Có thể phải bật thêm feature `Win32_UI_Controls` của crate `windows`.

**Spec:** `docs/superpowers/specs/2026-11-16-options-tabs-i18n-design.md` (gọi là "thiết kế đợt 1"). Spec gốc `2026-11-16-shotme-design.md` vẫn đúng ở mọi chỗ thiết kế đợt 1 không nói khác.

## Global Constraints

- Chỉ mở kết nối mạng đến hai nơi: `https://{account_id}.r2.cloudflarestorage.com` và địa chỉ công khai của kho. Thông tin và Trợ giúp không có link, không gọi mạng.
- HTTPS dùng `rustls` với `webpki-roots`.
- Secret Access Key không bao giờ xuất hiện ở dạng đọc được trên đĩa, trong log, trong thông báo lỗi, trong `Debug` hay đầu ra kiểm thử.
- Không ghi log ra file. Không thêm crate mới.
- `shotme-core` không phụ thuộc crate `windows` hay API riêng của hệ điều hành.
- Mọi chữ người dùng nhìn thấy lấy từ bảng `shotme_core::i18n`, đủ hai ngôn ngữ; không viết cứng trong mã chạy thật. (Thay dòng "Giao diện và thông báo bằng tiếng Việt" của kế hoạch gốc.) Câu cố định: tải lên thành công báo "Đã chép link" / "Link copied".
- Không dịch: tên phím, tên riêng ("Account ID", "Access Key ID", "Secret Access Key", "Cloudflare R2", "shotme"), nội dung lỗi do hệ điều hành trả về, chú thích mã, tên và thông điệp kiểm thử.
- File cấu hình cũ (không có trường `language`) phải đọc được và không tự đổi sang English.
- Bản release dùng `panic = "abort"`: mã chạy thật không `unwrap`/`expect`/chỉ số có thể panic trên dữ liệu đầu vào.
- Không làm ở đợt này: Proxy, Đăng nhập, tab Định dạng, mọi ô của đợt 2 và 3, `WM_DPICHANGED`. Không có tab rỗng hay ô không tác dụng.
- Hành vi đã qua rà soát của cửa sổ hiện tại giữ nguyên: bắt phím tắt (quyết định 22), đăng ký lại phím khi Hủy, Kiểm tra kết nối chạy nền có thế hệ và hẹn giờ dự phòng, cổng chụp `gate_step`.
- Quy trình: commit kiểm thử trước (`test:`, thân hàm mới là `todo!()`), rồi commit mã (`feat:`). Chạy `cargo fmt --all` trước khi commit. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`. Không chuyển nhánh. Chạy cargo qua PowerShell. Không tắt tiến trình `shotme` không do mình khởi động; nếu exe bị khóa thì bỏ build release và báo lại.

## Review Focus

Năm tình huống thiết kế hàm ý mà dễ gặp khi dùng thật. Mỗi dòng có kiểm thử ở task ghi trong ngoặc.

1. Nâng cấp từ bản hiện tại trên máy Windows không phải tiếng Việt: file cấu hình không có trường ngôn ngữ, hoặc có giá trị lạ (`"fr"`), phải ra Tiếng Việt; file hỏng hoặc không có file thì theo Windows. (Task 15: `config`; Task 16: `app::startup`)
2. Gõ sai ở tab này, sang tab khác rồi bấm OK: cửa sổ nhảy về tab có ô sai; mọi giá trị đã gõ ở cả ba tab, kể cả Secret Access Key, còn nguyên. (Task 17: `tab_of`, `apply_form` trả `FormError` có `field`)
3. Đổi ngôn ngữ rồi bấm Hủy, hoặc bấm OK nhưng ghi file cấu hình thất bại: ngôn ngữ đang dùng không được đổi. (Task 17: `lang_after_save`)
4. Bản English dài hơn bản Việt: tiêu đề thông báo khay vượt 63 đơn vị UTF-16 sẽ bị cắt cụt giữa chừng. Mọi tiêu đề thông báo phải vừa ở cả hai ngôn ngữ. (Task 16: `notice_titles_fit_tray_limit`)
5. Đang ở chế độ bắt phím tắt mà bấm Ctrl+Tab: đó là một tổ hợp hợp lệ nên phải được bắt làm phím tắt, không đổi tab; bấm chuột vào tab khác thì thoát chế độ bắt. (Task 17: `key_route`)

## Cấu trúc file

| File | Việc | Task |
|---|---|---|
| `crates/shotme-core/src/i18n.rs` (mới) | `Lang`, `Msg`, `t`, các hàm chuỗi có tham số. Bảng chuỗi duy nhất của cả ứng dụng | 15, rồi 16–18 thêm mục |
| `crates/shotme-core/src/config.rs` | `Config.language`, đọc/ghi trường `language`, `UrlError` | 15 |
| `crates/shotme-core/src/actions.rs`, `upload.rs`, `image.rs` | nhận `Lang`, bỏ chuỗi viết cứng | 15 |
| `crates/shotme-windows/src/lang.rs` (mới) | ngôn ngữ hiện hành, dò ngôn ngữ Windows | 16 |
| `crates/shotme-windows/src/{main,app,tray,overlay,clipboard,secret,autostart,settings}.rs` | chuyển chuỗi sang bảng | 16 |
| `scripts/check.ps1` | chặn chuỗi tiếng Việt viết cứng ngoài `i18n.rs` | 16 |
| `crates/shotme-windows/src/settings.rs` | ba tab, `FormError`, `Tab`, `OpenReason`, hộp chọn ngôn ngữ | 17 |
| `crates/shotme-windows/src/main.rs` | lý do mở Tùy chọn, áp dụng ngôn ngữ sau khi lưu | 17 |
| `crates/shotme-windows/src/winutil.rs` (mới) | phần dùng chung của ba cửa sổ: phông hệ thống, căn giữa, biểu tượng | 18 |
| `crates/shotme-windows/src/about.rs`, `help.rs` (mới) | hai cửa sổ không chặn | 18 |
| `crates/shotme-windows/src/tray.rs` | menu năm mục | 18 |
| `docs/testing/manual-checklist.md` | mục thử tay mới | 17, 18 |
| `README.md`, `memory-bank/{productContext,systemPatterns,techContext}.md` | tài liệu | 19 |

## Giao diện giữa các task

Agent của mỗi task chỉ đọc gói của mình; các tên dưới đây được chép vào gói nào cần.

**Task 15 tạo ra (`shotme_core`)**

```rust
// i18n.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Lang { #[default] Vi, En }
impl Lang {
    pub const ALL: [Lang; 2] = [Lang::Vi, Lang::En];
    pub fn code(self) -> &'static str;              // "vi" | "en"
    pub fn from_code(s: &str) -> Option<Lang>;      // khớp chính xác, không đoán
    pub fn native_name(self) -> &'static str;       // "Tiếng Việt" | "English"
}
pub enum Msg { /* mỗi chuỗi không tham số một biến thể */ }
impl Msg { pub const ALL: &'static [Msg]; }
pub fn t(lang: Lang, m: Msg) -> &'static str;
pub fn expiry_label(lang: Lang, e: Expiry) -> &'static str;
/// Các mục dùng làm tiêu đề thông báo khay (Task 16 kiểm độ dài).
pub const NOTICE_TITLES: &[Msg];
// chuỗi có tham số: mỗi chuỗi một hàm `pub fn ten(lang: Lang, ...) -> String`

// config.rs
pub struct Config { /* như cũ */ pub language: Lang }
impl Config { pub fn defaults(default_save_dir: &Path, language: Lang) -> Config; }
pub fn load(path: &Path, store: &dyn SecretStore, default_save_dir: &Path, os_lang: Lang)
    -> Result<Loaded, ConfigError>;
pub enum UrlError { NotHttps, MissingHost, Whitespace, ForbiddenChar }
impl UrlError { pub fn message(&self, lang: Lang) -> String; }
pub fn normalize_public_base_url(input: &str) -> Result<String, UrlError>;

// actions.rs
pub struct Ctx<'a> { /* như cũ */ pub lang: Lang }

// upload.rs: thay message_vi()
impl UploadError { pub fn message(&self, lang: Lang) -> String; }

// image.rs
pub enum EncodeError { InvalidImage, Encoder(String) }
impl EncodeError { pub fn message(&self, lang: Lang) -> String; }
pub fn encode_png(img: &Image) -> Result<Vec<u8>, EncodeError>;
```

Cách thêm một chuỗi (mọi task): thêm một mục vào bảng trong `i18n.rs` với cả hai bản; chuỗi có tham số thì thêm một hàm có `match lang` vét cạn. Không có nhánh `_ =>`.

**Task 16 tạo ra (`shotme-windows`)**

```rust
// lang.rs
pub fn current() -> Lang;                 // đọc được từ mọi luồng
pub fn set(lang: Lang);
pub fn tr(m: Msg) -> &'static str;        // t(current(), m)
pub fn lang_from_langid(langid: u16) -> Lang;   // thuần
pub fn os_default() -> Lang;              // GetUserDefaultUILanguage

// app.rs
pub enum ConfigNotice { Corrupt, SecretLost, Unreadable }
impl ConfigNotice { pub fn message(self, lang: Lang) -> String; }
pub struct AppState { pub config: Config, pub config_notice: Option<ConfigNotice>, pub overlay_open: bool }
pub fn startup(path: &Path, store: &dyn SecretStore, default_save_dir: &Path, os_lang: Lang) -> AppState;
```

**Task 17 tạo ra**

```rust
// settings.rs
pub enum Tab { General, Hotkeys, R2 }
pub enum Field { SaveDir, AccountId, Bucket, PublicUrl }
pub struct FormError { pub field: Field, pub message: String }
pub fn tab_of(field: Field) -> Tab;
pub enum OpenReason { User, NeedsR2 }
pub fn initial_tab(reason: OpenReason) -> Tab;
pub fn apply_form(current: &Config, form: &Form, lang: Lang) -> Result<Config, FormError>;
pub fn show(parent: HWND, current: &Config, notice: Option<&str>, reason: OpenReason)
    -> Option<SettingsResult>;
// main.rs
pub fn lang_after_save(old: Lang, new: Lang, saved: bool) -> Lang;
```

**Task 18 tạo ra**

```rust
// tray.rs
pub enum MenuCommand { Capture, Options, About, Help, Quit }
// about.rs / help.rs
pub fn show(owner: HWND);                 // help::show(owner: HWND, hotkey: &Hotkey)
pub fn close();
pub fn pre_translate(msg: &MSG) -> bool;  // true = đã xử lý, vòng lặp bỏ qua thông điệp
```

## Các task

| Task | Gói | Nhánh | Agent làm | Model | Rà soát |
|---|---|---|---|---|---|
| 15 Bảng chuỗi và ngôn ngữ ở core | `docs/tasks/15-i18n-core.md` | `task/15-i18n-core` | builder | sonnet | reviewer |
| 16 Lớp Windows dùng bảng chuỗi | `docs/tasks/16-i18n-windows.md` | `task/16-i18n-windows` | builder | sonnet | reviewer |
| 17 Cửa sổ Tùy chọn dạng tab | `docs/tasks/17-options-tabs.md` | `task/17-options-tabs` | specialist | opus | specialist |
| 18 Menu, Thông tin, Trợ giúp | `docs/tasks/18-menu-about-help.md` | `task/18-menu-about-help` | builder | sonnet | reviewer |
| 19 Tài liệu đợt 1 | `docs/tasks/19-docs-batch1.md` | `task/19-docs-batch1` | scaffolder | haiku | reviewer |

Thứ tự bắt buộc 15 → 16 → 17 → 18 → 19. Sau mỗi task cả workspace build và qua `scripts/check.ps1`.

**Điểm dừng duy nhất:** sau Task 18, chủ dự án thử tay theo các mục "Đợt 1" của `docs/testing/manual-checklist.md` và đọc lời văn Trợ giúp. Lỗi họ báo: sửa trên nhánh `task/18b-...` bằng agent theo độ khó. Task 19 chạy sau khi họ báo kết quả.

**Đã làm ở bước lập kế hoạch** (không thuộc task nào): năm gói task; dòng Task 15–19 trong `memory-bank/progress.md`; quyết định 27–32 trong `memory-bank/decisions.md`; `memory-bank/activeContext.md`; `.claude/commands/next-task.md` (Task 17 do `specialist` rà soát, điểm dừng sau Task 18); `CLAUDE.md` gốc và của hai crate (quy tắc chuỗi lấy từ bảng `i18n`).
