# Task 20: Lõi đợt 2

**Agent:** `builder` · **Model:** sonnet · **Nhánh:** `task/20-batch2-core` · **Rà soát:** `reviewer`

Gói task tự chứa. Thuộc kế hoạch `docs/superpowers/plans/2026-11-16-capture-batch2.md`.

## Cần đọc

- `crates/shotme-core/CLAUDE.md`
- Thiết kế đợt 2 `docs/superpowers/specs/2026-11-16-capture-batch2-design.md` mục 5.1, 5.4, 5.5, 5.7
- `crates/shotme-core/src/{config,actions,geometry,i18n}.rs` (toàn bộ)
- Các chỗ dùng `config.hotkey` và `NoticeKind` ở `crates/shotme-windows/src` (tìm bằng grep)

## Global Constraints

- `shotme-core` không phụ thuộc crate `windows` hay API hệ điều hành. Không thêm crate.
- Mọi chữ người dùng thấy nằm trong bảng `i18n`, đủ hai ngôn ngữ, không có nhánh `_ =>`. Không dịch tên phím. Tiêu đề thông báo khay mới thêm vào `NOTICE_TITLES` (giới hạn 63 đơn vị UTF-16, đã có kiểm thử).
- File cấu hình của bản hiện tại (không có trường mới) phải đọc được và cho đúng hành vi hiện tại. Trường `hotkey` trong file giữ nguyên tên và nghĩa. Trường mới có kiểu sai làm file thành "hỏng" như mọi trường hiện nay; không viết cách đọc khoan dung riêng. Tổ hợp đọc từ file không kiểm theo quyết định 22.
- Release dùng `panic = "abort"`: không `unwrap`/`expect`/chỉ số có thể panic trên đường chạy thật.
- Khóa bí mật không lọt vào lỗi, `Debug`, đầu ra kiểm thử.
- Lớp Windows chỉ được sửa tối thiểu để build và giữ nguyên hành vi nhìn thấy: không thêm ô, không đăng ký phím mới, không lọc thông báo.
- Commit kiểm thử trước (`test:`, thân hàm mới `todo!()`, **không** kèm mã thật), rồi mã (`feat:`). `cargo fmt --all`. Chạy cargo qua PowerShell. Không chuyển nhánh. Không tắt tiến trình `shotme` nào. Không đẩy lên remote. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.

## Interfaces

**Produces (`shotme_core`):**

```rust
// config.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HotkeyRole { Region, SaveFull, UploadFull }
impl HotkeyRole {
    pub const ALL: [HotkeyRole; 3] = [HotkeyRole::Region, HotkeyRole::SaveFull, HotkeyRole::UploadFull];
    pub fn label(self) -> Msg;
    pub fn default_slot(self) -> HotkeySlot;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HotkeySlot { pub enabled: bool, pub key: Hotkey }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hotkeys { pub region: HotkeySlot, pub save_full: HotkeySlot, pub upload_full: HotkeySlot }
impl Default for Hotkeys;
impl Hotkeys {
    pub fn get(&self, role: HotkeyRole) -> HotkeySlot;
    pub fn set(&mut self, role: HotkeyRole, slot: HotkeySlot);
    pub fn first_conflict(&self) -> Option<(HotkeyRole, HotkeyRole)>;
}
pub struct Config { /* như cũ, bỏ `hotkey` */ pub hotkeys: Hotkeys,
    pub show_success_notices: bool, pub keep_selection: bool, pub include_cursor: bool }

// actions.rs
pub enum NoticeKind { Success, Info, Error }
pub fn notice_visible(kind: NoticeKind, show_success: bool) -> bool;

// geometry.rs
impl Selection { pub fn with_rect(bounds: Rect, rect: Option<Rect>) -> Selection; }
pub fn restore_selection(remembered: Rect, origin: Point, image_w: i32, image_h: i32, monitors: &[Rect]) -> Option<Rect>;

// i18n.rs
pub fn hotkeys_conflict(lang: Lang, first_label: &str, second_label: &str, combo: &str) -> String;
pub fn hotkey_row_taken(lang: Lang, row_label: &str, combo: &str) -> String;
pub fn hotkeys_taken_body(lang: Lang, combos: &[String]) -> String;
```

## Thiết kế đã chốt

**Mặc định.** `Region`: bật, `PrintScreen` (`vk` 0x2C, không phím bổ trợ). `SaveFull`: tắt, Shift+PrintScreen. `UploadFull`: tắt, Ctrl+PrintScreen. `show_success_notices` true; `keep_selection` false; `include_cursor` false. `Config::defaults` dùng các giá trị này.

**File cấu hình.** Trường mới trong `FileConfig`:

| Trường | Kiểu | Thiếu thì |
|---|---|---|
| `hotkey` (đã có) | như cũ | như cũ; là tổ hợp của `hotkeys.region` |
| `hotkey_enabled` | bool | `true` |
| `hotkey_save_full` | `{enabled, ctrl, alt, shift, win, vk}` | `default_slot(SaveFull)`; thiếu trường con thì trường con lấy từ mặc định của dòng đó |
| `hotkey_upload_full` | như trên | `default_slot(UploadFull)` |
| `notify_success` | bool | `true` |
| `keep_selection` | bool | `false` |
| `include_cursor` | bool | `false` |

`save` ghi đủ mọi trường. Bản đợt 1 đọc file mới vẫn được (nó bỏ qua trường lạ; kiểm bằng cách xác nhận `FileConfig` hiện không dùng `deny_unknown_fields`).

**Trùng tổ hợp.** `first_conflict` duyệt theo `ALL`, chỉ xét dòng `enabled`; hai `Hotkey` bằng nhau là trùng. Trả `(dòng trước, dòng sau)` của cặp đầu tiên tìm thấy khi duyệt dòng sau tăng dần (với ba dòng cùng tổ hợp: `(Region, SaveFull)`).

**Thông báo.** `Success` dùng cho đúng ba thông báo: đã lưu ảnh, đã chép ảnh, đã chép link (ba chỗ đang là `Info` trong `actions.rs`). Mọi chỗ khác giữ loại hiện tại. `notice_visible(kind, show_success)`: `Success` → `show_success`; `Info`, `Error` → luôn `true`. Ở lớp Windows, `tray.rs` cho `Success` cùng biểu tượng với `Info`; `take_notice_to_show` không đổi.

**Hình học.**

- `Selection::with_rect(bounds, rect)`: `rect` là `Some(r)` với `r` nằm trọn trong `bounds` và `r.w`, `r.h` không nhỏ hơn `MIN_SIZE` thì `rect()` trả `Some(r)` và không ở trạng thái kéo; ngược lại giống `Selection::new(bounds)`.
- `restore_selection`: đổi `remembered` sang tọa độ ảnh (trừ `origin`). Trả `Some` khi khung có `w > 0`, `h > 0`, nằm trọn trong `0..image_w` × `0..image_h`, và có phần giao khác rỗng với ít nhất một phần tử của `monitors`. Dùng phép tính không tràn số (`i64` hoặc `checked_*`).

**Chuỗi mới** (bản Việt cố định; bản English do bạn viết, cùng nghĩa):

| Chỗ dùng | Tiếng Việt |
|---|---|
| nhãn dòng 2 | Lưu ảnh (toàn màn hình) |
| nhãn dòng 3 | Tải lên nhanh (toàn màn hình) |
| ô tích | Hiện các thông báo lưu và sao chép |
| ô tích | Giữ vị trí vùng đã chọn |
| ô tích | Bao gồm con trỏ chuột vào ảnh chụp màn hình |
| tiêu đề thông báo (`NOTICE_TITLES`) | Phím tắt chưa hoạt động |
| `hotkeys_taken_body`, một tổ hợp | {combo} đang được ứng dụng khác dùng. Mở Tùy chọn để chọn phím khác. |
| `hotkeys_taken_body`, nhiều tổ hợp | Các phím {a}, {b} đang được ứng dụng khác dùng. Mở Tùy chọn để chọn phím khác. |
| `hotkeys_conflict` | "{first}" và "{second}" đang dùng cùng phím {combo}. Hãy đổi một trong hai. |
| `hotkey_row_taken` | Phím {combo} của "{row}" đang được ứng dụng khác dùng. Hãy chọn phím khác. |

`HotkeyRole::label`: `Region` dùng mục nhãn "Chụp vùng màn hình" đã có trong bảng. `hotkeys_taken_body` với danh sách rỗng trả chuỗi rỗng.

**Lớp Windows (tối thiểu).** Thay `config.hotkey` bằng `config.hotkeys.region.key` ở mọi chỗ đọc; chỗ dựng `Config` từ biểu mẫu (`apply_form`) giữ nguyên ba trường mới và hai dòng mới từ `current`, chỉ thay `hotkeys.region.key`. `info_windows_stale` so `hotkeys` thay cho `hotkey`. `tray.rs` thêm nhánh `Success`. Không gì khác.

## Các bước

- [ ] **Bước 1: kiểm thử** (thân hàm mới `todo!()`; kiểu và trường mới phải có để biên dịch được).

```rust
// config.rs
#[test] fn default_slots() {
    let h = Hotkeys::default();
    assert!(h.region.enabled && !h.save_full.enabled && !h.upload_full.enabled);
    assert_eq!(h.region.key, Hotkey::default());
    assert_eq!(h.save_full.key, Hotkey { shift: true, ..Hotkey::default() });
    assert_eq!(h.upload_full.key, Hotkey { ctrl: true, ..Hotkey::default() });
    for r in HotkeyRole::ALL { assert_eq!(h.get(r), r.default_slot()); }
}
#[test] fn old_file_keeps_behaviour() {
    // file của đợt 1: chỉ có `hotkey` = Ctrl+Shift+S, không có trường mới
    let l = load_text(r#"{"hotkey":{"ctrl":true,"alt":false,"shift":true,"win":false,"vk":83},"language":"vi"}"#);
    assert_eq!(l.config.hotkeys.region, HotkeySlot { enabled: true,
        key: Hotkey { ctrl: true, alt: false, shift: true, win: false, vk: 83 } });
    assert_eq!(l.config.hotkeys.save_full, HotkeyRole::SaveFull.default_slot());
    assert_eq!(l.config.hotkeys.upload_full, HotkeyRole::UploadFull.default_slot());
    assert!(l.config.show_success_notices && !l.config.keep_selection && !l.config.include_cursor);
}
#[test] fn new_fields_round_trip() {
    // đặt mọi trường mới khác mặc định (ba dòng, ba cờ), save rồi load: Config bằng nhau;
    // JSON ghi ra vẫn có khóa "hotkey" mang tổ hợp của dòng 1
}
#[test] fn partial_slot_uses_that_rows_defaults() {
    // {"hotkey_save_full":{"enabled":true}} → enabled, key = Shift+PrintScreen
}
#[test] fn wrong_type_is_corrupt() {
    // {"notify_success":"yes"} → Err(ConfigError::Corrupt(_))
}
#[test] fn conflict_only_between_enabled_rows() {
    let mut h = Hotkeys::default();
    assert_eq!(h.first_conflict(), None);
    h.save_full.key = h.region.key;               // dòng 2 đang tắt
    assert_eq!(h.first_conflict(), None);
    h.save_full.enabled = true;
    assert_eq!(h.first_conflict(), Some((HotkeyRole::Region, HotkeyRole::SaveFull)));
    h.region.enabled = false;
    assert_eq!(h.first_conflict(), None);
    h.upload_full = h.save_full;
    assert_eq!(h.first_conflict(), Some((HotkeyRole::SaveFull, HotkeyRole::UploadFull)));
    h.region.enabled = true;                      // cả ba trùng
    assert_eq!(h.first_conflict(), Some((HotkeyRole::Region, HotkeyRole::SaveFull)));
}

// actions.rs
#[test] fn only_success_notices_can_be_hidden() {
    assert!(notice_visible(NoticeKind::Success, true));
    assert!(!notice_visible(NoticeKind::Success, false));
    for k in [NoticeKind::Info, NoticeKind::Error] {
        assert!(notice_visible(k, false) && notice_visible(k, true));
    }
}
#[test] fn notice_kinds_of_outcomes() {
    // lưu được, chép ảnh được, tải lên và chép link được: Success
    // tải lên được nhưng set_text lỗi; tải lên lỗi rồi lưu dự phòng; lưu lỗi; chép ảnh lỗi: Error
}

// geometry.rs
#[test] fn with_rect_starts_selected() {
    let b = Rect { x: 0, y: 0, w: 100, h: 100 };
    let r = Rect { x: 10, y: 20, w: 30, h: 40 };
    let s = Selection::with_rect(b, Some(r));
    assert_eq!(s.rect(), Some(r)); assert!(!s.is_dragging());
    assert_eq!(s.hit_test(Point { x: 20, y: 30 }, 4), Hit::Inside);
    assert_eq!(Selection::with_rect(b, None).rect(), None);
    assert_eq!(Selection::with_rect(b, Some(Rect { x: 90, y: 0, w: 30, h: 10 })).rect(), None); // tràn
    assert_eq!(Selection::with_rect(b, Some(Rect { x: 0, y: 0, w: 1, h: 50 })).rect(), None);   // quá nhỏ
}
#[test] fn restore_selection_cases() {
    let mons = [Rect { x: 0, y: 0, w: 1920, h: 1080 }, Rect { x: 1920, y: 0, w: 1280, h: 1024 }];
    let o = Point { x: 0, y: 0 };
    let r = Rect { x: 100, y: 100, w: 300, h: 200 };
    assert_eq!(restore_selection(r, o, 3200, 1080, &mons), Some(r));
    // gốc âm: màn hình phụ ở bên trái
    let o2 = Point { x: -1280, y: 0 };
    assert_eq!(restore_selection(r, o2, 3200, 1080, &mons), Some(Rect { x: 1380, ..r }));
    // màn hình phải đã rút: khung nằm ngoài ảnh
    let far = Rect { x: 2000, y: 100, w: 300, h: 200 };
    assert_eq!(restore_selection(far, o, 1920, 1080, &mons[..1]), None);
    // trong ảnh nhưng ở vùng chết (dưới màn hình thấp hơn): không chạm màn hình nào
    assert_eq!(restore_selection(Rect { x: 2000, y: 1040, w: 100, h: 30 }, o, 3200, 1080, &mons), None);
    // tràn một phần; rỗng; giá trị cực lớn không panic
    assert_eq!(restore_selection(Rect { x: 1800, y: 900, w: 300, h: 300 }, o, 1920, 1080, &mons[..1]), None);
    assert_eq!(restore_selection(Rect { x: 5, y: 5, w: 0, h: 10 }, o, 1920, 1080, &mons[..1]), None);
    assert_eq!(restore_selection(Rect { x: i32::MAX, y: i32::MIN, w: i32::MAX, h: i32::MAX },
        Point { x: i32::MIN, y: i32::MAX }, 1920, 1080, &mons[..1]), None);
}

// i18n.rs
#[test] fn batch2_strings_insert_their_parameters() {
    for l in Lang::ALL {
        let s = hotkeys_conflict(l, "AAA", "BBB", "Ctrl+X");
        assert!(s.contains("AAA") && s.contains("BBB") && s.contains("Ctrl+X"));
        let s = hotkey_row_taken(l, "AAA", "Ctrl+X");
        assert!(s.contains("AAA") && s.contains("Ctrl+X"));
        assert!(hotkeys_taken_body(l, &["F8".into()]).contains("F8"));
        let s = hotkeys_taken_body(l, &["F8".into(), "Ctrl+X".into()]);
        assert!(s.contains("F8") && s.contains("Ctrl+X"));
        assert_ne!(s, hotkeys_taken_body(l, &["F8".into()]));
        assert_eq!(hotkeys_taken_body(l, &[]), "");
    }
    for r in HotkeyRole::ALL { assert_ne!(t(Lang::Vi, r.label()), t(Lang::En, r.label())); }
}
```

  `load_text` là hàm phụ trong module kiểm thử: ghi chuỗi vào file tạm rồi gọi `load` với kho bí mật giả đã có. Hai kiểm thử chỉ có chú thích phải được viết đầy đủ theo chú thích. Kiểm thử hiện có dùng `config.hotkey` hoặc so `NoticeKind::Info` cho ba thông báo thành công thì sửa theo kiểu mới, giữ nguyên ý.

- [ ] **Bước 2:** `cargo test -p shotme-core` thấy kiểm thử mới thất bại (panic `todo!()`), kiểm thử cũ vẫn qua trừ chỗ chạm hàm `todo!()`. Commit `test: batch 2 core (hotkey slots, notice kinds, selection restore)`.
- [ ] **Bước 3:** hiện thực theo "Thiết kế đã chốt". `cargo test -p shotme-core` qua.
- [ ] **Bước 4:** sửa tối thiểu lớp Windows cho build; `cargo test -p shotme-windows` qua.
- [ ] **Bước 5:** `powershell -File scripts/check.ps1` in `ALL CHECKS PASSED`. Commit `feat: batch 2 core (hotkey slots, success notices, selection restore)`.

**Xong khi:** check qua; báo cáo liệt kê tên các mục `Msg` mới, bản English của từng chuỗi, mọi file của lớp Windows đã chạm và vì sao, và điều gì bạn chưa chắc.
