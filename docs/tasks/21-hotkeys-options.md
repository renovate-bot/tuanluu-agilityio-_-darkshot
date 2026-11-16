# Task 21: Tùy chọn có ba phím tắt, ba ô tích mới, lọc thông báo, Trợ giúp

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/21-hotkeys-options` · **Rà soát:** `specialist`

Gói task tự chứa. Thuộc kế hoạch `docs/superpowers/plans/2026-11-16-capture-batch2.md`.

## Cần đọc

- `crates/shotme-windows/CLAUDE.md`, `crates/shotme-core/CLAUDE.md`, `memory-bank/systemPatterns.md` (mục "Thêm một ô hoặc một tab")
- Thiết kế đợt 2 `docs/superpowers/specs/2026-11-16-capture-batch2-design.md` mục 2, 5.1, 5.3, 5.4, 5.8
- `crates/shotme-windows/src/settings.rs` (toàn bộ), `hotkey.rs`, `main.rs`, `help.rs`, `tray.rs`
- `crates/shotme-core/src/config.rs`, `actions.rs`, `i18n.rs`

## Global Constraints

- Mọi chữ người dùng thấy lấy từ bảng `shotme_core::i18n`, đủ hai ngôn ngữ; không viết cứng. Không dịch tên phím và tên riêng.
- Không thêm crate. Không thêm đích mạng. Trợ giúp không có địa chỉ web.
- Secret Access Key không lọt vào lỗi, `Debug`, đầu ra kiểm thử; ô Secret luôn mở ra trống.
- Release dùng `panic = "abort"`: không `unwrap`/`expect` trên đường chạy thật. `unsafe` nhỏ, kèm `// SAFETY:`. Giải phóng mọi handle.
- Giữ nguyên hành vi đã qua rà soát: máy trạng thái bắt phím (`CaptureState`, `capture_hotkey`, quyết định 22) dùng cho cả ba dòng; `key_route`; Hủy đăng ký lại phím cũ; Kiểm tra kết nối chạy nền; `show` không ghi đĩa, không gọi DPAPI, không đổi autostart; `gate_step`; `enqueue_notice`, `take_notice_to_show`.
- Thông báo loại lỗi và thông tin luôn hiện.
- Một dòng đã bỏ tích không bao giờ được đăng ký với Windows.
- Không làm ở task này: chụp toàn màn hình, vẽ con trỏ, khung nhớ (Task 22). Ô tích của chúng chỉ lưu giá trị.
- Kiểm thử mới không khẳng định chữ đọc từ `lang::tr`; dùng `t(Lang::Vi, …)`.
- Commit kiểm thử trước (`test:`, thân hàm mới `todo!()`, không kèm mã thật), rồi mã (`feat:`). `cargo fmt --all`. Chạy cargo qua PowerShell. Không chuyển nhánh. Không tắt tiến trình `shotme` không do mình khởi động. Không đẩy lên remote. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.

## Interfaces

**Consumes (Task 20, `shotme_core`):**

```rust
pub enum HotkeyRole { Region, SaveFull, UploadFull }   // ALL, label() -> Msg, default_slot()
pub struct HotkeySlot { pub enabled: bool, pub key: Hotkey }
pub struct Hotkeys { pub region, pub save_full, pub upload_full }   // get, set, first_conflict
pub struct Config { pub hotkeys: Hotkeys, pub show_success_notices: bool, pub keep_selection: bool, pub include_cursor: bool, .. }
pub enum NoticeKind { Success, Info, Error }
pub fn notice_visible(kind: NoticeKind, show_success: bool) -> bool;
pub fn hotkeys_conflict(lang, first_label: &str, second_label: &str, combo: &str) -> String;
pub fn hotkey_row_taken(lang, row_label: &str, combo: &str) -> String;
pub fn hotkeys_taken_body(lang, combos: &[String]) -> String;
```

Tên các mục `Msg` mới (nhãn ô tích, tiêu đề "Phím tắt chưa hoạt động") có trong `i18n.rs`.

**Produces:**

```rust
// shotme_core::i18n
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct HelpKeys { pub region: Option<String>, pub save_full: Option<String>, pub upload_full: Option<String> }
pub fn help_text(lang: Lang, keys: &HelpKeys) -> String;

// hotkey.rs
pub fn id_of(role: HotkeyRole) -> i32;          // Region = 1, SaveFull = 2, UploadFull = 3
pub fn role_of(id: i32) -> Option<HotkeyRole>;
pub fn register(hwnd: HWND, role: HotkeyRole, hk: &Hotkey) -> bool;
pub fn unregister_all(hwnd: HWND);
pub fn register_enabled(hwnd: HWND, keys: &Hotkeys) -> Vec<HotkeyRole>;
pub fn help_keys(keys: &Hotkeys) -> HelpKeys;
pub fn taken_combos(keys: &Hotkeys, failed: &[HotkeyRole]) -> Vec<String>;   // describe() của từng dòng lỗi, theo thứ tự ALL

// settings.rs
pub enum Field { SaveDir, HotkeyRow(HotkeyRole), AccountId, Bucket, PublicUrl }
pub struct Form { /* `hotkey` thành */ pub hotkeys: Hotkeys,
                  pub show_success_notices: bool, pub keep_selection: bool, pub include_cursor: bool, .. }

// help.rs
pub fn show(parent: HWND, keys: &HelpKeys);

// main.rs
pub fn info_windows_stale(old: &Config, new: &Config) -> bool;   // ngôn ngữ hoặc `hotkeys` khác
```

Task 22 sẽ nối `WM_HOTKEY` của `SaveFull` và `UploadFull` vào luồng chụp. Ở task này `wndproc` nhận ba mã, `Region` làm như hiện nay, hai mã kia chưa làm gì (để một nhánh rõ ràng kèm chú thích "Task 22").

## Thiết kế đã chốt

**Tab Phím tắt.** Ba dòng theo `HotkeyRole::ALL`. Mỗi dòng: ô tích mang nhãn `role.label()`, ô chỉ đọc hiện `hotkey::describe`, nút "Đổi…".

```
☑ Chụp vùng màn hình             [PrintScreen      ] [Đổi…]
☐ Lưu ảnh (toàn màn hình)        [Shift+PrintScreen] [Đổi…]
☐ Tải lên nhanh (toàn màn hình)  [Ctrl+PrintScreen ] [Đổi…]
```

- Bỏ tích: ô phím và nút Đổi… của dòng bị mờ (`EnableWindow(false)`), tổ hợp giữ nguyên. Tích lại: sáng lên. Bỏ tích dòng đang ở chế độ bắt: thoát chế độ bắt trước.
- Chế độ bắt: `St.capturing` thành `Option<HotkeyRole>`; `key_route` vẫn nhận `capturing: bool` (`is_some()`). Bấm Đổi… của dòng khác khi một dòng đang bắt: dòng trước thoát (giữ phím cũ của nó), dòng mới vào chế độ bắt. Bấm Đổi… lần đầu trong phiên mở cửa sổ: `unregister_all` và đặt `hk_unregistered` (như hiện nay với một phím). Mọi luật khác của chế độ bắt không đổi.
- Nhãn dài của bản English không được bị cắt: bề rộng cột nhãn tính cho chuỗi dài nhất của cả hai ngôn ngữ, hoặc thu ô phím.

**Tab Thông thường.** Thêm ba ô tích liền nhau, ngay dưới "Khởi động cùng Windows", theo thứ tự: thông báo, giữ vị trí vùng, con trỏ. Giá trị ban đầu từ `Config`, đọc trong `read_form`, đi qua `apply_form` nguyên vẹn. Nếu tab này cao hơn tab Lưu trữ R2 thì chiều cao cửa sổ tăng theo tab cao nhất.

**`apply_form`.** Thứ tự kiểm: `SaveDir`; trùng tổ hợp (`form.hotkeys.first_conflict()` → `FormError { field: Field::HotkeyRow(dòng sau), message: hotkeys_conflict(lang, nhãn dòng trước, nhãn dòng sau, describe) }`); `AccountId`; `Bucket`; `PublicUrl`. `tab_of(HotkeyRow(_))` là `Tab::Hotkeys`. Ô nhận tiêu điểm cho `HotkeyRow(r)` là nút Đổi… của dòng `r`. `apply_form` nằm ở `settings.rs` và cần tên tổ hợp: dùng `hotkey::describe` (thuần).

**OK (`save`).** Sau `apply_form` thành công: nếu `hk_unregistered` hoặc `form.hotkeys != current.hotkeys` thì `unregister_all`, rồi đăng ký lần lượt các dòng đang bật theo `ALL`. Dòng đầu tiên thất bại: `unregister_all`, đặt `hk_unregistered = true`, chuyển tab Phím tắt, tiêu điểm vào nút Đổi… của dòng đó, dòng lỗi `hotkey_row_taken(lang, nhãn, tổ hợp)`, cửa sổ vẫn mở. Thành công cả: `hk_unregistered = false`, đóng cửa sổ.

**Hủy.** Nếu `hk_unregistered`: `register_enabled(parent, &current.hotkeys)`, bỏ qua kết quả (như hiện nay).

**`main.rs`.**

- Khởi động: `register_enabled`; nếu có dòng lỗi thì **một** thông báo `Error` với tiêu đề "Phím tắt chưa hoạt động" (mục `Msg` của Task 20) và nội dung `hotkeys_taken_body(lang, &taken_combos(..))`. Bỏ cách báo từng phím cũ ở đây.
- `open_settings`, nhánh ghi file thất bại: `unregister_all`, `register_enabled(hwnd, &config.hotkeys)` với cấu hình cũ; có dòng lỗi thì cùng thông báo gộp ở trên (thay cặp `hotkey_inactive_title` / `HotkeyReregisterFailedBody` nếu chúng không còn chỗ dùng; mục `Msg` không còn dùng thì xóa khỏi bảng).
- `WM_HOTKEY`: `hotkey::role_of(wp)`; `Region` như hiện nay.
- `WM_DESTROY`: `unregister_all`.
- Lọc thông báo: trong nhánh `WM_NOTICE`, chỉ gọi `show_notice` khi `notice_visible(notice.kind, show)` với `show` đọc từ `APP` (`config.show_success_notices`); `APP` chưa có thì coi là `true`. Đây là chỗ lọc duy nhất.
- `info_windows_stale`: `old.language != new.language || old.hotkeys != new.hotkeys`.
- Mở Trợ giúp từ menu: `help::show(hwnd, &hotkey::help_keys(&config.hotkeys))`. Nút Trợ giúp trong Tùy chọn dùng `hotkeys` đang hiện trên biểu mẫu.

**Trợ giúp (`help_text`).** Giữ năm phần hiện có. Thay đổi:

- Phần 1 (chụp): nêu phím chụp vùng nếu `keys.region` là `Some`; nếu `None` thì nói phím đang tắt và chụp bằng menu khay. Thêm hai dòng cho lưu toàn màn hình và tải lên nhanh toàn màn hình: `Some(k)` thì nêu tổ hợp `k` và việc nó làm (màn hình đang có con trỏ chuột; lưu vào thư mục lưu / tải lên và chép link); `None` thì nói "chưa bật" và chỉ chỗ bật (Tùy chọn, tab Phím tắt).
- Phần 3 (Tùy chọn): tab Thông thường nêu thêm ba ô tích, mỗi ô một ý ngắn; nêu rõ thông báo lỗi luôn hiện. Tab Phím tắt nêu ba dòng và ô tích bật/tắt.
- Vẫn dùng nhãn nút Đổi… lấy từ bảng (`Msg::BtnChange`), không có "http".

## Các bước

- [ ] **Bước 1: kiểm thử hàm thuần** (thân hàm mới `todo!()`).

```rust
// i18n.rs (core)
#[test] fn help_text_lists_enabled_and_disabled_hotkeys() {
    for l in Lang::ALL {
        let all = HelpKeys { region: Some("KEY_A".into()), save_full: Some("KEY_B".into()), upload_full: Some("KEY_C".into()) };
        let s = help_text(l, &all);
        for k in ["KEY_A", "KEY_B", "KEY_C", "Ctrl+D", "Ctrl+S", "Ctrl+C", "Esc"] { assert!(s.contains(k), "{l:?} thiếu {k}"); }
        assert!(!s.contains("http"));
        assert!(s.contains(t(l, Msg::BtnChange)));
        let none = help_text(l, &HelpKeys::default());
        for k in ["KEY_A", "KEY_B", "KEY_C"] { assert!(!none.contains(k)); }
        assert_ne!(none, s);
        // mỗi dòng tắt riêng lẻ cho văn bản khác với khi bật
        assert_ne!(help_text(l, &HelpKeys { save_full: None, ..all.clone() }), s);
        assert_ne!(help_text(l, &HelpKeys { upload_full: None, ..all.clone() }), s);
        assert_ne!(help_text(l, &HelpKeys { region: None, ..all.clone() }), s);
    }
}

// hotkey.rs
#[test] fn ids_round_trip() {
    assert_eq!(id_of(HotkeyRole::Region), 1);
    let ids: Vec<i32> = HotkeyRole::ALL.iter().map(|r| id_of(*r)).collect();
    assert_eq!(ids, vec![1, 2, 3]);
    for r in HotkeyRole::ALL { assert_eq!(role_of(id_of(r)), Some(r)); }
    assert_eq!(role_of(0), None); assert_eq!(role_of(4), None);
}
#[test] fn help_keys_skip_disabled_rows() {
    let mut h = Hotkeys::default();
    assert_eq!(help_keys(&h), HelpKeys { region: Some("PrintScreen".into()), save_full: None, upload_full: None });
    h.upload_full.enabled = true; h.region.enabled = false;
    assert_eq!(help_keys(&h), HelpKeys { region: None, save_full: None, upload_full: Some("Ctrl+PrintScreen".into()) });
}
#[test] fn taken_combos_follow_row_order() {
    let h = Hotkeys::default();
    assert_eq!(taken_combos(&h, &[HotkeyRole::UploadFull, HotkeyRole::Region]),
               vec!["PrintScreen".to_string(), "Ctrl+PrintScreen".to_string()]);
    assert!(taken_combos(&h, &[]).is_empty());
}

// settings.rs
#[test] fn hotkey_rows_belong_to_hotkeys_tab() {
    for r in HotkeyRole::ALL { assert_eq!(tab_of(Field::HotkeyRow(r)), Tab::Hotkeys); }
}
#[test] fn hotkey_conflict_is_a_form_error() {
    let c = base(); let mut f = form_of(&c);
    f.hotkeys.save_full = HotkeySlot { enabled: true, key: f.hotkeys.region.key };
    let e = apply_form(&c, &f, Lang::Vi).unwrap_err();
    assert_eq!(e.field, Field::HotkeyRow(HotkeyRole::SaveFull));
    assert!(e.message.contains("PrintScreen"));
    assert!(e.message.contains(t(Lang::Vi, HotkeyRole::Region.label())));
    assert!(e.message.contains(t(Lang::Vi, HotkeyRole::SaveFull.label())));
    f.hotkeys.save_full.enabled = false;            // dòng tắt không tính
    assert!(apply_form(&c, &f, Lang::Vi).is_ok());
}
#[test] fn error_order_is_savedir_hotkeys_r2() {
    let c = base(); let mut f = form_of(&c);
    f.hotkeys.save_full = HotkeySlot { enabled: true, key: f.hotkeys.region.key };
    f.bucket = "x y".into();
    assert_eq!(apply_form(&c, &f, Lang::Vi).unwrap_err().field, Field::HotkeyRow(HotkeyRole::SaveFull));
    f.save_dir = "".into();
    assert_eq!(apply_form(&c, &f, Lang::Vi).unwrap_err().field, Field::SaveDir);
}
#[test] fn new_options_are_carried_from_form() {
    let c = base(); let mut f = form_of(&c);
    f.show_success_notices = false; f.keep_selection = true; f.include_cursor = true;
    f.hotkeys.upload_full.enabled = true; f.hotkeys.region.enabled = false;
    let n = apply_form(&c, &f, Lang::Vi).unwrap();
    assert!(!n.show_success_notices && n.keep_selection && n.include_cursor);
    assert_eq!(n.hotkeys, f.hotkeys);
}

// main.rs
#[test] fn info_windows_close_when_any_hotkey_row_changes() {
    // bật/tắt một dòng, hoặc đổi tổ hợp của dòng 2 hay 3: stale; đổi show_success_notices: không stale
}
```

  Kiểm thử hiện có dùng `help_text(lang, "…")`, `form.hotkey`, `Field` thì sửa theo kiểu mới, giữ nguyên ý.

- [ ] **Bước 2:** chạy kiểm thử của hai crate, thấy kiểm thử mới thất bại. Commit `test: three hotkey rows, new options, help keys`.
- [ ] **Bước 3:** hàm thuần: `help_text`, `id_of`, `role_of`, `help_keys`, `taken_combos`, `tab_of`, `apply_form`, `info_windows_stale`. Kiểm thử qua.
- [ ] **Bước 4:** `hotkey.rs` (`register`, `unregister_all`, `register_enabled`) và `main.rs` (khởi động, ghi file thất bại, `WM_HOTKEY`, `WM_DESTROY`, lọc thông báo, Trợ giúp).
- [ ] **Bước 5:** `settings.rs`: ba dòng, ba ô tích, `Ctl`, `St`, `read_form`, `save`, `cancel`, bắt phím theo dòng, bật/tắt mờ.
- [ ] **Bước 6:** kiểm thử chạy tay `settings_window_opens_and_cancels` và `help_window_opens_and_closes` sửa theo chữ ký mới; chạy `cargo test -p shotme-windows -- --ignored settings_window help_window --test-threads=1` một lần, ghi kết quả vào báo cáo.
- [ ] **Bước 7: checklist.** Thêm vào `docs/testing/manual-checklist.md` mục "Đợt 2: Tùy chọn và phím tắt" (mọi ô `[ ]`): ba dòng đúng nhãn ở hai ngôn ngữ, không chữ nào bị cắt; bỏ tích thì ô phím và nút mờ, tích lại thì sáng và tổ hợp còn nguyên; đổi phím từng dòng; bấm Đổi… dòng này khi dòng kia đang bắt; bỏ tích dòng đang bắt; hai dòng bật trùng tổ hợp thì OK báo lỗi và đứng ở dòng sau; dòng tắt trùng tổ hợp thì OK được; đặt một dòng vào phím ứng dụng khác đang giữ thì OK báo lỗi nêu tên dòng, sau đó Hủy thì phím cũ vẫn chạy; bỏ tích dòng 1 rồi OK: PrintScreen trả về cho Windows, menu khay vẫn chụp được; khởi động khi hai phím bị chiếm: một thông báo liệt kê cả hai; ba ô tích ở tab Thông thường lưu và mở lại đúng; tắt thông báo: lưu, chép ảnh, tải lên không hiện thông báo, còn lỗi (rút mạng rồi tải lên) vẫn hiện; Trợ giúp nêu đúng phím của dòng bật và "chưa bật" cho dòng tắt, ở hai ngôn ngữ; đổi bất kỳ dòng nào rồi OK thì Thông tin và Trợ giúp đang mở tự đóng; nâng cấp từ file cấu hình đợt 1: phím cũ còn, không gì tự bật.
- [ ] **Bước 8:** `powershell -File scripts/check.ps1` in `ALL CHECKS PASSED`. Commit `feat: three hotkey rows, new options, success-notice filter`.

**Xong khi:** check qua; kiểm thử chạy tay đã chạy; báo cáo nêu lời văn Trợ giúp mới ở hai ngôn ngữ, các mục `Msg` đã xóa hoặc thêm, cách bảo đảm dòng đã bỏ tích không bao giờ được đăng ký (kể cả sau Hủy và sau ghi file thất bại), và mọi điểm của hành vi cũ mà bạn không chắc còn nguyên.
