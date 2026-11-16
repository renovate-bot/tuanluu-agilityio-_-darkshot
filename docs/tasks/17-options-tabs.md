# Task 17: Cửa sổ Tùy chọn dạng tab

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/17-options-tabs` · **Rà soát:** `specialist`

Gói task tự chứa. Thuộc kế hoạch `docs/superpowers/plans/2026-11-16-options-tabs-i18n.md`.

## Cần đọc

- `crates/shotme-windows/CLAUDE.md`, `memory-bank/systemPatterns.md`
- Thiết kế đợt 1 `docs/superpowers/specs/2026-11-16-options-tabs-i18n-design.md` mục 2, 5.2, 5.3, 5.7
- `crates/shotme-windows/src/settings.rs` (toàn bộ), `main.rs`, `lang.rs`, `app.rs`
- `crates/shotme-core/src/i18n.rs`, `config.rs`

## Global Constraints

- Mọi chữ người dùng nhìn thấy lấy từ bảng `shotme_core::i18n`, đủ hai ngôn ngữ. `scripts/check.ps1` chặn chuỗi tiếng Việt viết cứng ngoài `i18n.rs`.
- Không dịch tên phím và tên riêng ("Account ID", "Access Key ID", "Secret Access Key", "Cloudflare R2").
- Không thêm crate (được bật thêm feature của crate `windows`, ví dụ `Win32_UI_Controls`). Không thêm đích mạng.
- Secret Access Key không lọt vào lỗi, `Debug`, đầu ra kiểm thử; ô Secret luôn mở ra trống.
- Release dùng `panic = "abort"`: không `unwrap`/`expect` trên đường chạy thật. `unsafe` nhỏ, kèm dòng `// SAFETY:`. Giải phóng mọi handle trên mọi đường đi.
- Giữ nguyên hành vi đã qua rà soát: máy trạng thái bắt phím (`CaptureState`, `capture_hotkey`, quyết định 22); Hủy đăng ký lại phím cũ; OK đăng ký phím mới trước khi trả về; Kiểm tra kết nối chạy nền với thế hệ (`GENERATION`) và hẹn giờ dự phòng; `show` không ghi đĩa, không gọi DPAPI, không đổi autostart; `gate_step` không đổi.
- Không vẽ tab hay ô nào của đợt 2, 3 (không có tab Định dạng, không có Proxy). Không xử lý `WM_DPICHANGED`.
- Commit kiểm thử trước (`test:`, thân hàm mới `todo!()`), rồi mã (`feat:`). `cargo fmt --all`. Chạy cargo qua PowerShell. Không chuyển nhánh. Không tắt tiến trình `shotme` không do mình khởi động. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.

## Interfaces

**Consumes:** `shotme_core::i18n::{Lang, Msg, t, expiry_label}` (thêm chuỗi bằng cách thêm mục vào macro bảng, đủ hai bản); `Config.language`; `UrlError::message(lang)`; `lang::{current, set, tr}`; `app::ConfigNotice`.

**Produces:**

```rust
// settings.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Tab { General, Hotkeys, R2 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Field { SaveDir, AccountId, Bucket, PublicUrl }
#[derive(Debug, PartialEq, Eq)] pub struct FormError { pub field: Field, pub message: String }
pub fn tab_of(field: Field) -> Tab;
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum OpenReason { User, NeedsR2 }
pub fn initial_tab(reason: OpenReason) -> Tab;
pub struct Form { /* các trường cũ */ pub language: Lang }
pub fn apply_form(current: &Config, form: &Form, lang: Lang) -> Result<Config, FormError>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyRoute { Capture, NextTab, PrevTab, Cancel, Accept, Dialog }
pub fn key_route(capturing: bool, vk: u32, ctrl: bool, shift: bool, combo_dropped: bool) -> KeyRoute;
pub fn show(parent: HWND, current: &Config, notice: Option<&str>, reason: OpenReason)
    -> Option<SettingsResult>;

// main.rs
pub fn merge_reason(a: OpenReason, b: OpenReason) -> OpenReason;   // NeedsR2 thắng
pub fn lang_after_save(old: Lang, new: Lang, saved: bool) -> Lang; // saved ? new : old
```

Task 18 sẽ thêm nút "Trợ giúp" vào hàng nút: để trống vùng bên trái hàng nút (từ x = 12 DIP, rộng ít nhất 100 DIP).

## Thiết kế đã chốt

**Bố cục.** Tiêu đề "Tùy chọn" / "Options". Vùng client rộng 480 DIP. Từ trên xuống: dòng thông báo (chỉ khi `notice` có), dải tab với ba khung, dòng lỗi màu đỏ, hàng nút. Chiều cao cố định bằng tab cao nhất, không đổi khi chuyển tab.

| Tab (Vi / En) | Nội dung, theo thứ tự |
|---|---|
| Thông thường / General | ô tích "Khởi động cùng Windows"; "Thư mục lưu" + ô nhập + nút "Chọn…"; "Ngôn ngữ" + hộp chọn (`Lang::ALL`, hiện `native_name()`) |
| Phím tắt / Hotkeys | "Chụp vùng màn hình" + ô chỉ đọc + nút "Đổi…" |
| Lưu trữ R2 / R2 storage | Account ID; Access Key ID; Secret Access Key + dòng gợi ý; Tên kho (bucket); Địa chỉ công khai; "Hạn của link" + hộp chọn; nút "Kiểm tra kết nối" + dòng kết quả |

Nút: "OK" (mặc định) và "Hủy" / "OK" và "Cancel". Câu của `ConfigNotice` đổi "bấm Lưu" thành "bấm OK".

**Dựng.** `SysTabControl32` làm con của cửa sổ; mỗi tab một cửa sổ con (khung) có `WS_EX_CONTROLPARENT`, đặt vào vùng hiển thị của dải tab (`TCM_ADJUSTRECT`); các ô là con của khung. Ô của cả ba khung tồn tại suốt đời cửa sổ; chuyển tab chỉ `ShowWindow` ẩn/hiện khung. Hai điều dễ sai:

- `WM_COMMAND` và `WM_CTLCOLORSTATIC` của các ô đi tới khung, không tới cửa sổ chính: thủ tục cửa sổ của khung phải chuyển tiếp chúng lên cửa sổ chính.
- Nền của khung và của nhãn trong khung phải trùng nền dải tab (không có ô xám trên nền trắng).

**Hành vi.**

- Tab mở đầu: `initial_tab(reason)`: `User` → `General`, `NeedsR2` → `R2`. Tiêu điểm ban đầu ở ô đầu tiên của tab đó.
- OK: `read_form` đọc cả ba tab → `apply_form`. `Err(FormError)`: chuyển sang `tab_of(field)`, đặt tiêu điểm vào ô đó, hiện `message` ở dòng lỗi. Phím tắt bị ứng dụng khác chiếm: chuyển sang tab Phím tắt và hiện lỗi như hiện nay. Giá trị đã gõ ở mọi tab giữ nguyên sau lỗi.
- `apply_form`: kiểm tra theo thứ tự `SaveDir` (rỗng), `AccountId`, `Bucket`, `PublicUrl`; `language` lấy từ `form`.
- Kiểm tra kết nối bấm khi thiếu thông tin hoặc ô không hợp lệ: lỗi hiện như hiện nay (tab R2 đang mở sẵn). Đang chạy mà sang tab khác: vẫn chạy; kết quả nằm trong dòng kết quả của tab R2.
- Phím (trong `run_loop`, trước `IsDialogMessageW`), theo `key_route`:
  - `capturing` → `Capture`: mọi phím đi vào `capture_msg` như hiện nay, kể cả Ctrl+Tab (nó là một tổ hợp hợp lệ).
  - Ctrl+Tab → `NextTab`, Ctrl+Shift+Tab → `PrevTab` (vòng tròn).
  - Esc → `Cancel`, Enter → `Accept` (bấm nút đang giữ tiêu điểm nếu là nút bấm, không thì OK), trừ khi một hộp chọn đang thả (`combo_dropped` đúng khi hộp Hạn hoặc hộp Ngôn ngữ đang thả) → `Dialog`.
  - còn lại → `Dialog`.
- Đổi tab bằng chuột hoặc phím khi đang bắt phím: `end_capture()` trước (xử lý ở `TCN_SELCHANGING` hoặc tương đương).
- Tab/Shift+Tab đi qua dải tab, các ô của khung đang hiện, OK, Hủy. Mũi tên trái/phải đổi tab khi dải tab giữ tiêu điểm. Ô của khung ẩn không bao giờ nhận tiêu điểm.

**`main.rs`.**

- Lý do mở: giữ trong một `thread_local` cạnh `GATE`. Chọn "Tùy chọn..." ở menu ghi `User`; `WM_OPEN_SETTINGS` (khởi động thiếu cấu hình, `on_needs_settings`) ghi `NeedsR2`; nhiều yêu cầu chờ gộp bằng `merge_reason`. `run_settings` lấy giá trị ra (đặt lại `User`) và truyền vào `show`.
- Sau `show` trả `Some`: ghi đĩa như hiện nay, rồi `lang::set(lang_after_save(old.language, new.language, saved))`. Ghi thất bại thì ngôn ngữ giữ nguyên, cùng với cấu hình cũ và phím cũ.

## Các bước

- [ ] **Bước 1: kiểm thử hàm thuần** (thân hàm `todo!()`).

```rust
// settings.rs
#[test] fn field_to_tab() {
    assert_eq!(tab_of(Field::SaveDir), Tab::General);
    for f in [Field::AccountId, Field::Bucket, Field::PublicUrl] { assert_eq!(tab_of(f), Tab::R2); }
}
#[test] fn initial_tab_by_reason() {
    assert_eq!(initial_tab(OpenReason::User), Tab::General);
    assert_eq!(initial_tab(OpenReason::NeedsR2), Tab::R2);
}
#[test] fn form_errors_name_their_field() {
    let c = base();
    let mut f = form_of(&c); f.save_dir = "  ".into();
    assert_eq!(apply_form(&c, &f, Lang::Vi).unwrap_err().field, Field::SaveDir);
    let mut f = form_of(&c); f.account_id = "a/b".into();
    assert_eq!(apply_form(&c, &f, Lang::Vi).unwrap_err().field, Field::AccountId);
    let mut f = form_of(&c); f.bucket = "my bucket".into();
    assert_eq!(apply_form(&c, &f, Lang::Vi).unwrap_err().field, Field::Bucket);
    let mut f = form_of(&c); f.public_base_url = "http://x".into();
    assert_eq!(apply_form(&c, &f, Lang::Vi).unwrap_err().field, Field::PublicUrl);
}
#[test] fn first_error_wins_in_tab_order() {   // sai cả SaveDir lẫn Bucket → báo SaveDir
    let c = base(); let mut f = form_of(&c); f.save_dir = "".into(); f.bucket = "x y".into();
    assert_eq!(apply_form(&c, &f, Lang::Vi).unwrap_err().field, Field::SaveDir);
}
#[test] fn form_error_message_follows_language() {
    let c = base(); let mut f = form_of(&c); f.bucket = "x y".into();
    assert_ne!(apply_form(&c, &f, Lang::Vi).unwrap_err().message,
               apply_form(&c, &f, Lang::En).unwrap_err().message);
}
#[test] fn language_is_carried_from_form() {
    let c = base(); let mut f = form_of(&c); f.language = Lang::En;
    assert_eq!(apply_form(&c, &f, Lang::Vi).unwrap().language, Lang::En);
}
#[test] fn key_routing() {
    const TAB: u32 = 0x09; const ESC: u32 = 0x1B; const ENTER: u32 = 0x0D;
    // đang bắt phím: mọi thứ là của chế độ bắt, kể cả Ctrl+Tab, Esc, Enter
    for (vk, ctrl) in [(TAB, true), (ESC, false), (ENTER, false), (0x41, true)] {
        assert_eq!(key_route(true, vk, ctrl, false, false), KeyRoute::Capture);
    }
    assert_eq!(key_route(false, TAB, true, false, false), KeyRoute::NextTab);
    assert_eq!(key_route(false, TAB, true, true, false), KeyRoute::PrevTab);
    assert_eq!(key_route(false, TAB, false, false, false), KeyRoute::Dialog);
    assert_eq!(key_route(false, ESC, false, false, false), KeyRoute::Cancel);
    assert_eq!(key_route(false, ENTER, false, false, false), KeyRoute::Accept);
    assert_eq!(key_route(false, ESC, false, false, true), KeyRoute::Dialog);
    assert_eq!(key_route(false, ENTER, false, false, true), KeyRoute::Dialog);
    assert_eq!(key_route(false, TAB, true, false, true), KeyRoute::NextTab);
}
// main.rs
#[test] fn open_reason_merge_prefers_r2() {
    use settings::OpenReason::*;
    assert_eq!(merge_reason(User, User), User);
    assert_eq!(merge_reason(User, NeedsR2), NeedsR2);
    assert_eq!(merge_reason(NeedsR2, User), NeedsR2);
}
#[test] fn language_changes_only_when_saved() {
    assert_eq!(lang_after_save(Lang::Vi, Lang::En, true), Lang::En);
    assert_eq!(lang_after_save(Lang::Vi, Lang::En, false), Lang::Vi);
}
```

  Các kiểm thử `apply_form` hiện có sửa theo chữ ký mới (`Lang::Vi`, `form_of` thêm `language`), giữ nguyên ý. `errors_and_debug_never_contain_secret` kiểm cả `FormError.message` và `{:?}` của `FormError`.

- [ ] **Bước 2:** `cargo test -p shotme-windows` thấy kiểm thử mới thất bại. Commit `test: options tabs pure logic`.

- [ ] **Bước 3: hàm thuần.** Hiện thực `tab_of`, `initial_tab`, `apply_form`, `key_route`, `merge_reason`, `lang_after_save`. Kiểm thử qua.

- [ ] **Bước 4: cửa sổ.** Dựng lại `build_controls` theo "Thiết kế đã chốt": dải tab, ba khung, chuyển tiếp thông điệp, hộp chọn ngôn ngữ, nút OK/Hủy, chiều cao cố định. Cập nhật `Ctl`, `read_form`, `save` (nhảy tab khi lỗi), `run_loop` (dùng `key_route`), `show` (tham số `reason`).

- [ ] **Bước 5: `main.rs`.** Lý do mở và áp dụng ngôn ngữ sau khi lưu.

- [ ] **Bước 6: kiểm thử chạy tay.** Cập nhật `settings_window_opens_and_cancels` theo chữ ký mới và cho nó chạy với cả hai `OpenReason`. Chạy `cargo test -p shotme-windows -- --ignored settings_window` một lần, ghi kết quả vào báo cáo.

- [ ] **Bước 7: checklist.** Thêm vào `docs/testing/manual-checklist.md` mục "Đợt 1: cửa sổ Tùy chọn" gồm các dòng: ba tab đúng nội dung và không có ô thừa; cửa sổ không đổi cỡ khi chuyển tab; giá trị gõ ở một tab còn nguyên sau khi sang tab khác và quay lại; Tab/Shift+Tab, mũi tên trên dải tab, Ctrl+Tab, Ctrl+Shift+Tab; Enter và Esc; ô sai ở tab khác thì OK nhảy về đúng tab và ô; đổi phím tắt, bấm chuột sang tab khác lúc đang bắt phím; Ctrl+Tab lúc đang bắt phím được nhận làm phím tắt; Kiểm tra kết nối rồi sang tab khác và quay lại; lần chạy đầu mở ở tab Lưu trữ R2, mở từ menu thì ở Thông thường; đổi ngôn ngữ rồi OK: menu khay, thông báo, chữ rê chuột lớp phủ và cửa sổ Tùy chọn mở lại đều đổi; đổi ngôn ngữ rồi Hủy: không đổi; file cấu hình của bản cũ: vẫn Tiếng Việt, mọi cài đặt còn nguyên; chữ English không bị cắt ở ô nào.

- [ ] **Bước 8:** `powershell -File scripts/check.ps1` in `ALL CHECKS PASSED`. Commit `feat: tabbed Options window with language choice`.

**Xong khi:** check qua; kiểm thử chạy tay đã chạy; báo cáo nêu cách bảo đảm ô của khung ẩn không nhận tiêu điểm, cách chuyển tiếp thông điệp từ khung, và mọi điểm của hành vi cũ mà bạn không chắc còn nguyên.
