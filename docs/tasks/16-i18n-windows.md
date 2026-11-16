# Task 16: Lớp Windows dùng bảng chuỗi

**Agent:** `builder` · **Model:** sonnet · **Nhánh:** `task/16-i18n-windows`

Gói task tự chứa. Thuộc kế hoạch `docs/superpowers/plans/2026-11-16-options-tabs-i18n.md`. Task này chỉ đổi nguồn của chữ; bố cục cửa sổ và menu chưa đổi (Task 17, 18).

## Cần đọc

- `crates/shotme-windows/CLAUDE.md`, `crates/shotme-core/CLAUDE.md`
- Thiết kế đợt 1 `docs/superpowers/specs/2026-11-16-options-tabs-i18n-design.md` mục 5.3
- `crates/shotme-core/src/i18n.rs` (do Task 15 tạo)
- Mọi file trong `crates/shotme-windows/src/`, và `scripts/check.ps1`

## Global Constraints

- Mọi chữ người dùng nhìn thấy lấy từ bảng `shotme_core::i18n`, đủ hai ngôn ngữ; không viết cứng trong mã chạy thật.
- Không dịch: tên phím (`hotkey::describe` giữ nguyên), tên riêng ("Account ID", "Access Key ID", "Secret Access Key", "Cloudflare R2", "shotme"), nội dung lỗi do hệ điều hành trả về, chú thích, tên và thông điệp kiểm thử, thông điệp `#[ignore = "..."]`.
- Không thêm crate. Không thêm đích mạng. Secret Access Key không lọt vào lỗi hay `Debug`.
- Release dùng `panic = "abort"`: không `unwrap`/`expect` trên đường chạy thật.
- Hành vi của cửa sổ Cài đặt, cổng chụp, lớp phủ không đổi: task này không sửa logic.
- Commit kiểm thử trước (`test:`), rồi mã (`feat:`). `cargo fmt --all`. Chạy cargo qua PowerShell. Không chuyển nhánh. Không tắt tiến trình `shotme` không do mình khởi động. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.

## Interfaces

**Consumes** (từ Task 15): `shotme_core::i18n::{Lang, Msg, t, expiry_label, NOTICE_TITLES}`; `Config.language`; `Config::defaults(dir, lang)`; `config::load(path, store, dir, os_lang)`; `Ctx.lang`; `UploadError::message(lang)`; `UrlError::message(lang)`; `EncodeError::message(lang)`. Thêm chuỗi: thêm mục vào macro bảng trong `i18n.rs` với cả hai bản; chuỗi có tham số là một `pub fn ten(lang, ...) -> String` với `match lang` vét cạn.

**Produces:**

```rust
// crates/shotme-windows/src/lang.rs (mới)
pub fn current() -> Lang;                       // AtomicU8; đọc được từ mọi luồng; mặc định Vi
pub fn set(lang: Lang);
pub fn tr(m: Msg) -> &'static str;              // t(current(), m)
pub fn lang_from_langid(langid: u16) -> Lang;   // thuần: ngôn ngữ chính (langid & 0x3FF) == 0x2A → Vi, còn lại En
pub fn os_default() -> Lang;                    // lang_from_langid(GetUserDefaultUILanguage())

// app.rs  (thay ba hằng NOTICE_*)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigNotice { Corrupt, SecretLost, Unreadable }
impl ConfigNotice { pub fn message(self, lang: Lang) -> String; }
pub struct AppState { pub config: Config, pub config_notice: Option<ConfigNotice>, pub overlay_open: bool }
pub fn startup(path: &Path, store: &dyn SecretStore, default_save_dir: &Path, os_lang: Lang) -> AppState;
```

## Quy tắc đã chốt

- `startup`: file hỏng hoặc không đọc được → `Config::defaults(dir, os_lang)`; các trường hợp khác theo `config::load`.
- `main.rs`: lúc khởi động gọi `lang::os_default()`, truyền vào `startup`, rồi `lang::set(state.config.language)` trước khi hiện bất kỳ chữ nào. Sau khi lưu cấu hình thành công trong `open_settings`: `lang::set(new.language)` (ở task này giá trị chưa đổi được từ giao diện; Task 17 thêm hộp chọn).
- Mọi chỗ dựng `Ctx` truyền `lang: lang::current()`. Luồng nền tạo chuỗi lỗi (`clipboard.rs`, `secret.rs`, `autostart.rs`, luồng Kiểm tra kết nối) đọc `lang::current()` tại lúc tạo chuỗi.
- `ConfigNotice` được đổi thành chữ lúc mở cửa sổ (`notice.message(lang::current())`), không phải lúc khởi động.
- Đổi tên gọi cho người dùng: mọi chỗ chữ "Cài đặt" thành "Tùy chọn" / "Options" (tiêu đề cửa sổ "Tùy chọn" / "Options", không còn tiền tố "shotme –"; mục menu "Tùy chọn..." / "Options..."; các câu "Hãy mở Cài đặt…", "…trong Cài đặt"; câu của `ConfigNotice` "bấm Lưu" giữ đến Task 17). Tên nội bộ trong mã không cần đổi.
- Chép cố định:

| Chỗ | Vi | En |
|---|---|---|
| menu khay | Chụp màn hình · Tùy chọn... · Thoát | Take screenshot · Options... · Exit |
| chữ rê chuột trên thanh công cụ lớp phủ | Tải lên và chép link (Ctrl+D) · Lưu vào máy (Ctrl+S) · Chép ảnh (Ctrl+C) · Hủy (Esc) | Upload and copy link (Ctrl+D) · Save to disk (Ctrl+S) · Copy image (Ctrl+C) · Cancel (Esc) |
| hạn link trong hộp chọn | `expiry_label(lang, e)` | |

- Chuỗi lỗi nội bộ của `overlay::pick_region` (ví dụ "lớp phủ đang mở") được ghép vào thông báo "Lý do: …" nên cũng phải qua bảng.
- Tiêu đề thông báo mới thêm vào bảng thì thêm vào `i18n::NOTICE_TITLES`.
- Cổng `scripts/check.ps1`: thêm một bước `no hard-coded Vietnamese strings`. Với mỗi file `crates/*/src/*.rs` trừ `i18n.rs`: đọc bằng `Get-Content -Encoding UTF8`, chỉ xét các dòng trước dòng đầu tiên chứa `#[cfg(test)]`, bỏ phần từ `//` đến cuối dòng, và báo lỗi nếu còn chuỗi khớp `"[^"]*[À-ɏḀ-ỿ][^"]*"`. In `CHECK FAILED: hard-coded string in <file>:<dòng>` và thoát 1. File `check.ps1` phải giữ thuần ASCII (viết ký tự bằng dạng `\uXXXX` trong regex).

## Các bước

- [ ] **Bước 1: kiểm thử.**

```rust
// lang.rs
#[test] fn windows_langid_maps_to_lang() {
    assert_eq!(lang_from_langid(0x042A), Lang::Vi);   // vi-VN
    assert_eq!(lang_from_langid(0x002A), Lang::Vi);   // vi trung tính
    assert_eq!(lang_from_langid(0x0409), Lang::En);   // en-US
    assert_eq!(lang_from_langid(0x0411), Lang::En);   // ja-JP
    assert_eq!(lang_from_langid(0), Lang::En);
}
#[test] fn tr_follows_current_language() {   // một kiểm thử duy nhất đụng biến toàn cục; đặt lại Vi ở cuối
    set(Lang::En); assert_eq!(current(), Lang::En); assert_eq!(tr(Msg::/*mục menu chụp*/), "Take screenshot");
    set(Lang::Vi); assert_eq!(tr(Msg::/*mục menu chụp*/), "Chụp màn hình");
}
```

```rust
// app.rs: sửa các kiểm thử startup hiện có theo chữ ký mới, và thêm
#[test] fn corrupt_or_missing_config_uses_os_language() {
    // file "{not json" + os_lang En → config.language == En, config_notice == Some(ConfigNotice::Corrupt)
    // không có file + os_lang En → En, None
    // file "{}" + os_lang En → Vi, None
}
#[test] fn config_notice_messages_differ_by_language() {
    for n in [ConfigNotice::Corrupt, ConfigNotice::SecretLost, ConfigNotice::Unreadable] {
        assert_ne!(n.message(Lang::Vi), n.message(Lang::En));
    }
}
```

  `i18n.rs` đã có `notice_titles_fit_tray_limit`; nó sẽ tự che các tiêu đề thêm ở task này.

- [ ] **Bước 2:** chạy `cargo test -p shotme-windows`, thấy kiểm thử mới thất bại. Commit `test: windows language state and startup language`.

- [ ] **Bước 3: `lang.rs` và `app.rs`.** Hiện thực theo Interfaces; nối vào `main.rs`.

- [ ] **Bước 4: chuyển chuỗi.** Lần lượt `tray.rs`, `main.rs`, `settings.rs` (nhãn, nút, gợi ý, lỗi, kết quả kiểm tra, tiêu đề hộp chọn thư mục, tên hạn), `overlay.rs`, `clipboard.rs`, `secret.rs`, `autostart.rs`, `app.rs`. `w!("…")` có chữ hiển thị thay bằng `HSTRING::from(lang::tr(Msg::…))`. Sau mỗi file chạy `cargo test -p shotme-windows`.

- [ ] **Bước 5: cổng kiểm tra.** Thêm bước vào `scripts/check.ps1` như mục "Quy tắc đã chốt". Kiểm chứng nó bắt được lỗi: tạm thêm một chuỗi tiếng Việt vào `tray.rs`, chạy `check.ps1`, thấy `CHECK FAILED: hard-coded string`, rồi gỡ ra.

- [ ] **Bước 6:** `powershell -File scripts/check.ps1` in `ALL CHECKS PASSED`. Commit `feat: windows layer reads all text from the i18n table`.

**Xong khi:** check qua (kể cả bước mới); báo cáo liệt kê mọi chuỗi English mới thêm (để rà soát đọc), và mọi chuỗi cố ý để lại ngoài bảng kèm lý do.
