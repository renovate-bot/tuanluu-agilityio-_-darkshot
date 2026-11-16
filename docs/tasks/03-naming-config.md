# Task 3: Đặt tên và cấu hình

**Agent:** `builder` · **Model:** sonnet · **Nhánh:** `task/03-naming-config`

Gói task tự chứa. Chỉ đọc file này và các file ở mục "Cần đọc".

## Cần đọc

- `memory-bank/systemPatterns.md`
- `memory-bank/techContext.md`
- docs/superpowers/specs/2026-11-16-shotme-design.md mục 5.3 (phần Lưu), 5.4, 6
- `crates/shotme-core/CLAUDE.md`

## Global Constraints

- Chỉ mở kết nối mạng đến hai nơi: `https://{account_id}.r2.cloudflarestorage.com` và địa chỉ công khai của kho do người dùng nhập. Không thống kê, không kiểm tra cập nhật, không báo lỗi tự động.
- HTTPS dùng `rustls` với `webpki-roots`. Không dùng `native-tls`, không dùng bộ kiểm chứng chứng chỉ của hệ điều hành.
- Secret Access Key không bao giờ xuất hiện ở dạng đọc được trên đĩa, trong log, trong thông báo lỗi hay trong đầu ra kiểm thử.
- Không ghi log ra file.
- `shotme-core` không được phụ thuộc crate `windows` hay bất kỳ API riêng của hệ điều hành nào. `cargo tree -p shotme-core` không chứa `windows`.
- Mọi trait nền tảng trong core có ràng buộc `Send + Sync`.
- Giao diện và thông báo bằng tiếng Việt. Câu cố định: tải lên thành công báo "Đã chép link".
- Tên ảnh trên kho: 16 ký tự `[A-Za-z0-9]` từ bộ sinh số ngẫu nhiên của hệ điều hành, không lệch phân phối.
- Tên file local: `shotme_YYYY-MM-DD_HH-MM-SS.png`, trùng thì thêm `_2`, `_3`.
- Thư mục hạn trên kho: `d1`, `d7`, `d30`, `keep`. Mặc định `d30`.
- Phím tắt mặc định: `PrintScreen` (VK `0x2C`), không phím bổ trợ.
- File cấu hình: `%APPDATA%\shotme\config.json`. Thư mục lưu mặc định: `Pictures\shotme`.
- Không xây tính năng ngoài phạm vi của spec (vẽ lên ảnh, lịch sử, xóa từ ứng dụng).
- `cargo test --workspace` và `cargo clippy --workspace -- -D warnings` phải qua trước khi gộp nhánh.

## Nội dung task
**Files:**
- Create: `crates/shotme-core/src/naming.rs`, `crates/shotme-core/src/config.rs`
- Modify: `crates/shotme-core/src/lib.rs`, `crates/shotme-core/Cargo.toml` (thêm `serde` có `derive`, `serde_json`, `base64`, `getrandom`)

**Interfaces:**
- Produces:

```rust
// naming.rs
#[derive(Clone, Copy, Debug)]
pub struct LocalTime { pub year: u16, pub month: u8, pub day: u8, pub hour: u8, pub minute: u8, pub second: u8 }
pub fn local_file_name(t: &LocalTime) -> String;
pub fn unique_path(dir: &Path, name: &str) -> PathBuf;
pub fn random_object_name() -> String;

// config.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)] pub enum Expiry { D1, D7, #[default] D30, Keep }
impl Expiry { pub fn prefix(self) -> &'static str; pub const ALL: [Expiry; 4]; }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hotkey { pub ctrl: bool, pub alt: bool, pub shift: bool, pub win: bool, pub vk: u32 }  // Default: vk 0x2C, không bổ trợ
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct R2Config { pub account_id: String, pub access_key_id: String, pub secret_access_key: String,
                      pub bucket: String, pub public_base_url: String }
impl R2Config { pub fn is_complete(&self) -> bool; }   // mọi trường khác rỗng
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config { pub r2: R2Config, pub expiry: Expiry, pub hotkey: Hotkey, pub save_dir: PathBuf, pub autostart: bool }
impl Config { pub fn defaults(default_save_dir: &Path) -> Config; }
pub trait SecretStore: Send + Sync {
    fn protect(&self, plain: &[u8]) -> Result<Vec<u8>, String>;
    fn unprotect(&self, cipher: &[u8]) -> Result<Vec<u8>, String>;
}
#[derive(Debug)] pub enum ConfigError { Io(String), Corrupt(String), Secret(String) }
pub struct Loaded { pub config: Config, pub secret_lost: bool }
pub fn load(path: &Path, store: &dyn SecretStore, default_save_dir: &Path) -> Result<Loaded, ConfigError>;
pub fn save(path: &Path, cfg: &Config, store: &dyn SecretStore) -> Result<(), ConfigError>;
pub fn normalize_public_base_url(input: &str) -> Result<String, String>;
```

Quy tắc:
- `load`: file không tồn tại trả `Config::defaults`, `secret_lost = false`. JSON hỏng trả `Err(Corrupt)` và không đụng vào file. `unprotect` thất bại: trả cấu hình với `secret_access_key` rỗng và `secret_lost = true`.
- `save`: tạo thư mục cha nếu thiếu. Khóa bí mật ghi ở trường `secret_access_key_protected` dạng base64 của `protect(...)`. Ghi ra file tạm cùng thư mục rồi đổi tên, để mất điện không làm hỏng file.
- `normalize_public_base_url`: cắt khoảng trắng hai đầu, bỏ mọi `/` ở cuối. Lỗi (thông điệp tiếng Việt) nếu không bắt đầu bằng `https://`, nếu phần sau `https://` rỗng, hoặc nếu chứa khoảng trắng.
- `random_object_name`: lấy byte từ `getrandom`, loại bỏ byte >= 248 rồi `byte % 62` (lấy mẫu loại trừ, không lệch).

- [ ] **Step 1: Viết kiểm thử thất bại.** Trong kiểm thử dùng `XorStore` (hiện thực `SecretStore` bằng XOR `0x5A`) và `FailStore` (`unprotect` luôn lỗi), thư mục tạm dưới `std::env::temp_dir()` có tên ngẫu nhiên.

```rust
// naming.rs
#[test] fn file_name_format() {
    let t = LocalTime { year: 2026, month: 11, day: 16, hour: 14, minute: 3, second: 5 };
    assert_eq!(local_file_name(&t), "shotme_2026-11-16_14-03-05.png");
}
#[test] fn unique_path_adds_suffix() {
    let d = temp_dir(); let n = "shotme_2026-11-16_14-03-05.png";
    assert_eq!(unique_path(&d, n), d.join(n));
    std::fs::write(d.join(n), b"x").unwrap();
    assert_eq!(unique_path(&d, n), d.join("shotme_2026-11-16_14-03-05_2.png"));
    std::fs::write(d.join("shotme_2026-11-16_14-03-05_2.png"), b"x").unwrap();
    assert_eq!(unique_path(&d, n), d.join("shotme_2026-11-16_14-03-05_3.png"));
}
#[test] fn random_name_shape_and_uniqueness() {
    let names: std::collections::HashSet<String> = (0..1000).map(|_| random_object_name()).collect();
    assert_eq!(names.len(), 1000);
    assert!(names.iter().all(|n| n.len() == 16 && n.bytes().all(|b| b.is_ascii_alphanumeric())));
}
// config.rs
#[test] fn expiry_prefixes_and_default() {
    assert_eq!(Expiry::ALL.map(Expiry::prefix), ["d1", "d7", "d30", "keep"]);
    assert_eq!(Expiry::default(), Expiry::D30);
}
#[test] fn missing_file_gives_defaults() {
    let l = load(&temp_dir().join("config.json"), &XorStore, Path::new("C:/pics")).unwrap();
    assert_eq!(l.config, Config::defaults(Path::new("C:/pics")));
    assert_eq!(l.config.hotkey.vk, 0x2C);
    assert!(!l.config.r2.is_complete() && !l.secret_lost && !l.config.autostart);
}
#[test] fn round_trip_and_secret_not_in_plaintext() {
    let path = temp_dir().join("sub").join("config.json");     // thư mục cha chưa có
    let mut c = Config::defaults(Path::new("C:/pics"));
    c.r2 = R2Config { account_id: "acc".into(), access_key_id: "kid".into(),
        secret_access_key: "TOP-SECRET-VALUE".into(), bucket: "b".into(), public_base_url: "https://img.example.com".into() };
    c.expiry = Expiry::D7; c.autostart = true;
    save(&path, &c, &XorStore).unwrap();
    assert!(!std::fs::read_to_string(&path).unwrap().contains("TOP-SECRET-VALUE"));
    assert_eq!(load(&path, &XorStore, Path::new("C:/pics")).unwrap().config, c);
}
#[test] fn corrupt_file_is_error_and_untouched() {              // Review Focus 3
    let path = temp_dir().join("config.json");
    std::fs::write(&path, "{not json").unwrap();
    assert!(matches!(load(&path, &XorStore, Path::new("C:/pics")), Err(ConfigError::Corrupt(_))));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "{not json");
}
#[test] fn undecryptable_secret_keeps_other_fields() {          // Review Focus 3
    // lưu bằng XorStore, đọc bằng FailStore
    let l = load(&path, &FailStore, Path::new("C:/pics")).unwrap();
    assert!(l.secret_lost);
    assert_eq!(l.config.r2.secret_access_key, "");
    assert_eq!(l.config.r2.bucket, "b");
}
#[test] fn public_base_url_normalization() {                    // Review Focus 2
    assert_eq!(normalize_public_base_url("  https://img.example.com// ").unwrap(), "https://img.example.com");
    assert_eq!(normalize_public_base_url("https://pub-1a2b.r2.dev").unwrap(), "https://pub-1a2b.r2.dev");
    assert!(normalize_public_base_url("http://img.example.com").is_err());
    assert!(normalize_public_base_url("img.example.com").is_err());
    assert!(normalize_public_base_url("https://").is_err());
    assert!(normalize_public_base_url("https://img.exa mple.com").is_err());
}
```

- [ ] **Step 2:** Chạy `cargo test -p shotme-core naming config`. Mong đợi: lỗi biên dịch.
- [ ] **Step 3:** Hiện thực theo Interfaces và Quy tắc.
- [ ] **Step 4:** Chạy lại. Mong đợi: 9 test PASS.
- [ ] **Step 5:** Commit.
