# Task 28: Chuyển dữ liệu từ shotme

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/28-legacy-migration` · **Rà soát:** `specialist`

Gói task tự chứa. Thuộc kế hoạch `docs/superpowers/plans/2026-11-16-darkshot-dark-theme.md`. Chạy sau Task 27.

## Cần đọc

- Thiết kế đợt 4 `docs/superpowers/specs/2026-11-16-darkshot-dark-theme-design.md` mục 2 (điều không được xảy ra), mục 6
- Kế hoạch đợt 4: "Global Constraints", "Interfaces chung" (phần `migrate`), "Review Focus" dòng 1–3
- `crates/darkshot-windows/CLAUDE.md`
- `crates/darkshot-core/src/config.rs` (định dạng file, `load`, `save` ghi file tạm rồi đổi tên), `crates/darkshot-windows/src/{main.rs (hàm khởi động quanh app::startup), app.rs, autostart.rs, wer.rs, paths.rs, secret.rs}`

## Global Constraints

- Không thêm crate (core đã có `serde_json`). `darkshot-core` không phụ thuộc crate `windows`. Có thể bật thêm feature của crate `windows` nếu thiếu.
- Mọi chữ người dùng thấy lấy từ `darkshot_core::i18n`, đủ hai ngôn ngữ. Kiểm thử `no_legacy_name_in_strings` của Task 27 cấm chữ tên cũ trong bảng chuỗi: lời thông báo nói "phiên bản cũ", không nêu tên.
- Chữ `shotme` chỉ được xuất hiện trong `crates/darkshot-core/src/migrate.rs` và `crates/darkshot-windows/src/legacy.rs` (bước `no legacy name` của `check.ps1` miễn đúng hai file này). Mọi chỗ khác cần tên cũ phải gọi hằng ở hai file đó.
- Không được mất cấu hình, khóa R2 hay thiết lập khởi động. Không xóa, không sửa gì trong `%APPDATA%\shotme`. Không bao giờ ghi đè `%APPDATA%\darkshot\config.json` đã có.
- Không bao giờ bật khởi động cùng Windows chỉ vì thấy giá trị Run `shotme`: cấu hình darkshot là nguồn đúng duy nhất.
- Không hiện hộp thoại. Một bước lỗi thì bỏ qua bước đó, không chặn khởi động, không panic; có ít nhất một bước lỗi thì hiện đúng một thông báo Info sau khi khay sẵn sàng.
- Release `panic = "abort"`: không `unwrap`/`expect`/chỉ số có thể panic trên đường chạy thật. `unsafe` nhỏ, kèm `// SAFETY:`; đóng mọi khóa registry trên mọi đường đi.
- Kiểm thử không đọc hay ghi `%APPDATA%` thật, khóa Run thật hay WER thật: dùng thư mục tạm; phần registry chỉ kiểm qua hàm thuần `plan_run`.
- Commit kiểm thử trước (`test:`, thân hàm mới `todo!()`), rồi mã (`feat:`). `cargo fmt --all`. Chạy cargo qua PowerShell. Kiểm `git branch --show-current` trước khi commit; không chuyển nhánh. Không tắt tiến trình `shotme`/`darkshot` không do mình khởi động. Không đẩy lên remote. Không đọc hay ghi khóa R2. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.

## Interfaces

**Consumes (Task 27):** `paths::config_path()` (`%APPDATA%\darkshot\config.json`), `autostart::{is_enabled, set_enabled, VALUE_NAME}`, `wer::exclude_from_error_reporting`, `app::startup`.

**Produces:**

```rust
// crates/darkshot-core/src/migrate.rs  (pub mod migrate trong lib.rs)
pub const LEGACY_APP_DIR: &str = "shotme";          // %APPDATA%\shotme, Pictures\shotme
pub const LEGACY_RUN_VALUE: &str = "shotme";
pub const LEGACY_EXE: &str = "shotme.exe";
#[derive(Debug, PartialEq, Eq)]
pub enum MigrateError { Corrupt }                  // không phải JSON object
pub fn should_copy_config(new_exists: bool, old_exists: bool) -> bool;   // !new_exists && old_exists
/// Nội dung file mới: mọi trường của file cũ giữ nguyên giá trị; nếu thiếu `save_dir` (hoặc là null)
/// thì thêm `save_dir` = legacy_default_save_dir (chuỗi đường dẫn). Không thêm hay đổi trường nào khác.
pub fn migrated_config(old_json: &[u8], legacy_default_save_dir: &Path) -> Result<Vec<u8>, MigrateError>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunAction { Write, Delete, Nothing }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunPlan { pub delete_legacy: bool, pub darkshot: RunAction }
/// delete_legacy = legacy_present; darkshot = Write nếu autostart (luôn ghi lại để cập nhật đường dẫn exe),
/// Delete nếu !autostart && darkshot_present, còn lại Nothing.
pub fn plan_run(legacy_present: bool, darkshot_present: bool, autostart: bool) -> RunPlan;

// i18n: Msg::NoticeMigrationIncomplete
//   vi "Chưa chuyển được hết thiết lập từ phiên bản cũ. Hãy mở Tùy chọn để kiểm tra."
//   en "Some settings from the previous version could not be moved. Open Options to check them."

// crates/darkshot-windows/src/legacy.rs
/// Bước 1–2 của thiết kế mục 6 với thư mục gốc truyền vào (để kiểm thử bằng thư mục tạm).
/// Ok(true) = đã chép; Ok(false) = không cần chép; Err = có bước lỗi (đã bỏ qua).
pub fn migrate_config(appdata: &Path, pictures: &Path) -> Result<bool, String>;
/// Gọi lúc khởi động, trước app::startup: migrate_config với thư mục thật, rồi
/// WerRemoveExcludedApplication(LEGACY_EXE, FALSE) (lỗi "không có" không tính là lỗi).
pub fn run_before_startup() -> bool;                // true nếu có bước lỗi
/// Gọi sau app::startup, thay khối "if autostart::is_enabled() { set_enabled(true) }" hiện có:
/// đọc hai giá trị Run, plan_run(..., config.autostart), áp dụng.
pub fn sync_autostart(autostart: bool) -> bool;     // true nếu có bước lỗi
```

`autostart.rs` cho `legacy.rs` dùng các hàm đọc, xóa giá trị theo tên (ví dụ `pub(crate) fn value_exists(name: &str) -> bool`, `pub(crate) fn delete_value(name: &str) -> Result<(), String>`); không đổi hành vi của `is_enabled`, `set_enabled`.

Ghi file mới như `config::save`: tạo thư mục, ghi file tạm cạnh đích, `sync_all`, đổi tên; không bao giờ đổi tên đè lên file đích đã có (kiểm lại ngay trước khi đổi tên).

## Các bước

- [ ] **1. Kiểm thử trước.** Commit `test: legacy migration`.
  - Core `migrate`:
    - `copy_only_when_new_missing_and_old_present`: bảng đủ bốn tổ hợp của `should_copy_config`.
    - `copied_config_keeps_every_field`: lấy JSON do `config::save` sinh ra với mọi trường khác mặc định (có `secret` mã hóa giả qua `SecretStore` giả, `autostart: true`, `upload_format` JPEG, ba phím tắt), `migrated_config` rồi parse lại: mọi khóa và giá trị bằng nhau (so `serde_json::Value`).
    - `missing_save_dir_points_to_legacy_folder`: JSON không có `save_dir` → kết quả có `save_dir` bằng đường dẫn truyền vào; JSON có `"save_dir": null` → như thiếu; JSON có `save_dir` khác → giữ nguyên.
    - `copied_config_loads_with_secret`: kết quả của `migrated_config` ghi vào thư mục tạm, `config::load` với cùng `SecretStore` giả cho lại đúng khóa bí mật, `secret_lost == false`.
    - `corrupt_old_config_is_not_copied`: `b"not json"`, `b"[]"`, `b""` → `Err(MigrateError::Corrupt)`.
    - `plan_run_*`: đủ tám tổ hợp; riêng `plan_run_legacy_does_not_enable`: `(true, false, false)` → `{ delete_legacy: true, darkshot: Nothing }`.
  - Windows `legacy`:
    - `migration_round_trip_in_temp_dir`: thư mục tạm có `appdata\shotme\config.json` → `migrate_config` trả `Ok(true)`, tạo `appdata\darkshot\config.json`, file cũ còn nguyên byte.
    - `existing_new_config_is_never_overwritten`: có cả hai file → `Ok(false)`, file mới giữ nguyên byte.
    - `no_legacy_config_does_nothing`: chỉ có thư mục rỗng → `Ok(false)`, không tạo thư mục `darkshot`.
    - `corrupt_legacy_config_reports_error`: file cũ là `not json` → `Err`, không tạo file mới.
  - `i18n`: `NoticeMigrationIncomplete` có trong bảng, hai bản khác nhau.
- [ ] **2. Mã.**
  - `migrate.rs` theo Interfaces; `serde_json::Value` (đọc vào `Value`, kiểm là object, chèn `save_dir` khi thiếu hoặc null, ghi ra `to_vec_pretty`).
  - `legacy.rs` theo Interfaces; thư mục thật lấy như `paths.rs` (`FOLDERID_RoamingAppData`, `FOLDERID_Pictures`, cùng phương án dự phòng); có thể tách trong `paths.rs` hàm trả hai thư mục gốc để dùng chung.
  - `main.rs`: gọi `legacy::run_before_startup()` ngay trước `app::startup`; thay khối ghi lại Run bằng `legacy::sync_autostart(state.config.autostart)`; nếu một trong hai trả `true` thì hiện thông báo Info `NoticeMigrationIncomplete` sau khi khay sẵn sàng, cùng đường với `config_notice` (sau nó nếu cả hai có), không lọc bởi ô "Hiện các thông báo lưu và sao chép".
  - `wer.rs` không đổi ngoài việc `legacy.rs` gọi `WerRemoveExcludedApplication`.
- [ ] **3. Checklist.** Thêm mục "Đợt 4: chuyển từ shotme" vào `docs/testing/manual-checklist.md`, mỗi dòng `[ ]`: (a) máy có shotme đã cấu hình R2 và bật khởi động cùng Windows, thoát shotme, chạy `darkshot.exe` lần đầu: Tùy chọn hiện đủ thiết lập cũ, Kiểm tra kết nối qua (khóa giải mã được), ảnh lưu vào thư mục cũ, `HKCU\...\Run` có `darkshot` và không còn `shotme`, `%APPDATA%\shotme` còn nguyên; (b) khởi động lại Windows: chỉ darkshot tự chạy; (c) đã có cấu hình darkshot với khởi động tắt, thêm tay giá trị Run `shotme`, chạy darkshot: `shotme` bị xóa, `darkshot` không được ghi; (d) máy chưa từng có shotme: không có thông báo nào, ảnh lưu vào `Pictures\darkshot`; (e) file `%APPDATA%\shotme\config.json` hỏng: darkshot vẫn chạy với mặc định và hiện thông báo "phiên bản cũ".
- [ ] **4.** `cargo fmt --all`; `powershell -File scripts/check.ps1` in `ALL CHECKS PASSED`. Commit `feat: migrate settings from the previous name`.

**Xong khi:** check qua; báo cáo nêu các commit, dòng cuối của check, thứ tự khởi động mới trong `main.rs` (gọi gì trước `app::startup`, gì sau), và những gì chỉ kiểm được bằng tay.
