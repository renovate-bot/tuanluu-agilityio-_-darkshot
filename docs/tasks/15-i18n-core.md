# Task 15: Bảng chuỗi và ngôn ngữ ở core

**Agent:** `builder` · **Model:** sonnet · **Nhánh:** `task/15-i18n-core`

Gói task tự chứa. Thuộc kế hoạch `docs/superpowers/plans/2026-11-16-options-tabs-i18n.md`.

## Cần đọc

- `crates/shotme-core/CLAUDE.md`
- Thiết kế đợt 1 `docs/superpowers/specs/2026-11-16-options-tabs-i18n-design.md` mục 5.3 và 5.7
- `crates/shotme-core/src/{config,actions,upload,image,lib}.rs`

## Global Constraints

- Core không phụ thuộc crate `windows` hay API hệ điều hành. Không thêm crate mới.
- Mọi chữ người dùng nhìn thấy lấy từ bảng `i18n`, đủ hai ngôn ngữ (Tiếng Việt, English); không viết cứng trong mã chạy thật. Câu cố định: "Đã chép link" / "Link copied".
- Không dịch: tên phím, tên riêng ("Account ID", "Secret Access Key", "Cloudflare R2", "shotme"), nội dung lỗi do hệ điều hành trả về (`io::Error`), chú thích, tên và thông điệp kiểm thử.
- Secret Access Key không lọt vào lỗi, `Debug` hay đầu ra kiểm thử.
- File cấu hình cũ (không có `language`) phải đọc được.
- Release dùng `panic = "abort"`: không `unwrap`/`expect` trên đường chạy thật.
- Commit kiểm thử trước (`test:`, thân hàm mới `todo!()`), rồi commit mã (`feat:`). `cargo fmt --all` trước khi commit. Chạy cargo qua PowerShell. Không chuyển nhánh. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.

## Interfaces

**Produces** (Task 16–18 dựa vào đúng các tên này):

```rust
// crates/shotme-core/src/i18n.rs  (thêm `pub mod i18n;` vào lib.rs)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Lang { #[default] Vi, En }
impl Lang {
    pub const ALL: [Lang; 2] = [Lang::Vi, Lang::En];
    pub fn code(self) -> &'static str;           // "vi" | "en"
    pub fn from_code(s: &str) -> Option<Lang>;   // khớp chính xác "vi"/"en"
    pub fn native_name(self) -> &'static str;    // "Tiếng Việt" | "English"
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Msg { /* mỗi chuỗi không tham số một biến thể */ }
impl Msg { pub const ALL: &'static [Msg]; }
pub fn t(lang: Lang, m: Msg) -> &'static str;
pub fn expiry_label(lang: Lang, e: Expiry) -> &'static str;
pub const NOTICE_TITLES: &[Msg];   // mọi mục dùng làm tiêu đề thông báo (`Notifier::notify` tham số title)
// chuỗi có tham số: mỗi chuỗi một `pub fn ten(lang: Lang, ...) -> String` với `match lang` vét cạn

// config.rs
pub struct Config { /* các trường cũ */ pub language: Lang }
impl Config { pub fn defaults(default_save_dir: &Path, language: Lang) -> Config; }
pub fn load(path: &Path, store: &dyn SecretStore, default_save_dir: &Path, os_lang: Lang)
    -> Result<Loaded, ConfigError>;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UrlError { NotHttps, MissingHost, Whitespace, ForbiddenChar }
impl UrlError { pub fn message(&self, lang: Lang) -> String; }
pub fn normalize_public_base_url(input: &str) -> Result<String, UrlError>;

// actions.rs
pub struct Ctx<'a> { /* các trường cũ */ pub lang: Lang }

// upload.rs  (xóa message_vi)
impl UploadError { pub fn message(&self, lang: Lang) -> String; }

// image.rs
#[derive(Debug, PartialEq, Eq)]
pub enum EncodeError { InvalidImage, Encoder(String) }
impl EncodeError { pub fn message(&self, lang: Lang) -> String; }
pub fn encode_png(img: &Image) -> Result<Vec<u8>, EncodeError>;
```

## Quy tắc đã chốt

- Bảng chuỗi: một `macro_rules!` nhận danh sách `Ten => ("bản Việt", "English"),` và sinh ra `enum Msg`, `Msg::ALL`, và `t`. Thiếu một bản là lỗi biên dịch. Không có chuỗi dự phòng lúc chạy, không có nhánh `_ =>`.
- Bản Việt của mỗi chuỗi giữ nguyên chữ đang có trong mã. Bản English do bạn viết: ngắn, cùng nghĩa, cùng giọng. Mọi mục trong `NOTICE_TITLES` dài tối đa 63 đơn vị UTF-16 ở cả hai ngôn ngữ.
- Chép cố định:

| Mục | Vi | En |
|---|---|---|
| tải lên thành công (tiêu đề) | Đã chép link | Link copied |
| `expiry_label` D1, D7, D30, Keep | 1 ngày, 7 ngày, 30 ngày, Không hết hạn | 1 day, 7 days, 30 days, Never expires |
| câu nhắc mở cài đặt (trong `UploadError::NotConfigured` và `actions`) | …Hãy nhập đủ thông tin trong Tùy chọn. | …in Options. |

- Ngôn ngữ khi đọc cấu hình:
  - không có file → `Config::defaults(dir, os_lang)`;
  - file đọc được, `language` là `"vi"` hoặc `"en"` → giá trị đó;
  - file đọc được, thiếu `language` hoặc giá trị khác → `Lang::Vi`;
  - file hỏng hoặc không đọc được → vẫn trả `Err` như cũ (bên gọi tự dựng mặc định với `os_lang`).
- File cấu hình: thêm khóa `"language"` kiểu chuỗi; `save` luôn ghi.
- `UrlError` tương ứng bốn nhóm lỗi hiện có của `normalize_public_base_url` (thiếu `https://`; thiếu tên miền, kể cả host rỗng; có khoảng trắng; có `?`, `#`, `@`).
- Câu `"quá nhiều ảnh trùng tên trong cùng một giây"` trong `actions::claim_path` và `"nội dung không khớp"` trong `upload.rs`: nếu chúng tới được người dùng thì đưa qua bảng (truyền `lang` xuống); nếu chỉ dùng nội bộ hoặc trong kiểm thử thì ghi rõ trong báo cáo.
- Lớp Windows phải còn build: sửa tối thiểu các chỗ gọi trong `crates/shotme-windows` (truyền `Lang::Vi` cho `Ctx.lang`, `load`, `Config::defaults`, `.message(Lang::Vi)`; `apply_form` chép `current.language`). Không chuyển chuỗi của lớp Windows ở task này.

## Các bước

- [ ] **Bước 1: kiểm thử `i18n`.** Tạo `i18n.rs` với kiểu và chữ ký ở trên, thân `todo!()`, và các kiểm thử:

```rust
#[test] fn every_message_has_both_languages_non_empty() {
    for m in Msg::ALL { for l in Lang::ALL { assert!(!t(l, *m).trim().is_empty(), "{m:?} {l:?}"); } }
}
#[test] fn lang_codes_round_trip() {
    assert_eq!(Lang::Vi.code(), "vi"); assert_eq!(Lang::En.code(), "en");
    for l in Lang::ALL { assert_eq!(Lang::from_code(l.code()), Some(l)); }
    assert_eq!(Lang::from_code("fr"), None); assert_eq!(Lang::from_code("VI"), None);
    assert_eq!(Lang::from_code(""), None);
    assert_eq!(Lang::Vi.native_name(), "Tiếng Việt"); assert_eq!(Lang::En.native_name(), "English");
}
#[test] fn expiry_labels() {
    assert_eq!(Expiry::ALL.map(|e| expiry_label(Lang::Vi, e)), ["1 ngày", "7 ngày", "30 ngày", "Không hết hạn"]);
    assert_eq!(Expiry::ALL.map(|e| expiry_label(Lang::En, e)), ["1 day", "7 days", "30 days", "Never expires"]);
}
#[test] fn notice_titles_fit_tray_limit() {
    for m in NOTICE_TITLES { for l in Lang::ALL { assert!(t(l, *m).encode_utf16().count() <= 63, "{m:?} {l:?}"); } }
}
```

  Thêm một kiểm thử cho mỗi hàm chuỗi có tham số: tham số xuất hiện trong kết quả ở cả hai ngôn ngữ.

- [ ] **Bước 2: kiểm thử `config`.** Trong `config.rs`:

```rust
#[test] fn missing_file_uses_os_language() {
    let l = load(&temp_dir().join("config.json"), &XorStore, Path::new("C:/pics"), Lang::En).unwrap();
    assert_eq!(l.config.language, Lang::En);
}
#[test] fn file_without_language_field_is_vietnamese() {
    let p = temp_dir().join("config.json"); std::fs::write(&p, "{}").unwrap();
    assert_eq!(load(&p, &XorStore, Path::new("C:/pics"), Lang::En).unwrap().config.language, Lang::Vi);
}
#[test] fn unknown_language_value_is_vietnamese() {
    let p = temp_dir().join("config.json"); std::fs::write(&p, r#"{"language":"fr"}"#).unwrap();
    assert_eq!(load(&p, &XorStore, Path::new("C:/pics"), Lang::En).unwrap().config.language, Lang::Vi);
}
#[test] fn language_round_trips() {
    let p = temp_dir().join("config.json");
    let mut c = sample(); c.language = Lang::En;
    save(&p, &c, &XorStore).unwrap();
    assert!(std::fs::read_to_string(&p).unwrap().contains(r#""language": "en""#));
    assert_eq!(load(&p, &XorStore, Path::new("C:/pics"), Lang::Vi).unwrap().config.language, Lang::En);
}
```

  Sửa hai kiểm thử URL hiện có để so với `UrlError` (ví dụ `"http://x"` → `NotHttps`, `"https://"` và `"https://:443"` → `MissingHost`, `"https://img.exa mple.com"` → `Whitespace`, `"https://a.com?x"` → `ForbiddenChar`), và thêm: mỗi biến thể `UrlError` có `message` khác rỗng và khác nhau giữa hai ngôn ngữ.

- [ ] **Bước 3: kiểm thử `upload`, `actions`, `image`.**
  - `upload.rs`: `message_in_both_languages`: với mỗi biến thể `UploadError`, `message(Vi)` và `message(En)` khác rỗng và khác nhau; `Http(503)` chứa `"503"` ở cả hai; `ClockSkew.message(Vi)` bằng đúng câu hiện có; nội dung của `Network(s)` và `PublicUrl(s)` không xuất hiện trong thông báo.
  - `actions.rs`: `notices_follow_ctx_lang`: tải lên thành công với `lang: Lang::En` cho tiêu đề `"Link copied"`, với `Lang::Vi` cho `"Đã chép link"`; lưu thành công ở hai ngôn ngữ cho hai tiêu đề khác nhau. Các kiểm thử hiện có truyền `lang: Lang::Vi` và giữ nguyên chuỗi mong đợi (trừ câu nhắc "cài đặt" nay là "Tùy chọn").
  - `image.rs`: ảnh không hợp lệ → `Err(EncodeError::InvalidImage)`.

- [ ] **Bước 4:** `cargo test -p shotme-core` biên dịch được và các kiểm thử mới thất bại vì `todo!()`. Commit `test: i18n table, config language, typed message errors`.

- [ ] **Bước 5: hiện thực** `i18n.rs` (macro, bảng, hàm có tham số, `NOTICE_TITLES`), rồi `config.rs`, `image.rs`, `upload.rs`, `actions.rs`. Mọi chuỗi tiếng Việt trong mã chạy thật của core chuyển vào bảng.

- [ ] **Bước 6: lớp Windows còn build.** Sửa tối thiểu các chỗ gọi như mục "Quy tắc đã chốt".

- [ ] **Bước 7:** `powershell -File scripts/check.ps1` in `ALL CHECKS PASSED`. Commit `feat: i18n table and language in core`.

**Xong khi:** check qua; trong `crates/shotme-core/src` không còn chuỗi tiếng Việt viết cứng trong mã chạy thật ngoài `i18n.rs` (báo cáo liệt kê ngoại lệ nếu có và lý do); báo cáo ghi số mục trong bảng và các chỗ đã sửa tối thiểu ở lớp Windows.
