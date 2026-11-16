# shotme đợt 2 (phím tắt toàn màn hình, con trỏ, giữ vị trí vùng, bật/tắt thông báo) Implementation Plan

> **For agentic workers:** kế hoạch này chạy bằng quy trình của dự án: `/next-task` giao từng gói `docs/tasks/NN-ten-ngan.md` cho agent ghi trong `memory-bank/progress.md`, rồi rà soát, kiểm tra, gộp. Các bước chi tiết (dạng `- [ ]`) nằm trong gói task; file này giữ phần chung mà mọi gói dựa vào.

**Goal:** Ba dòng phím tắt bật/tắt được, hai phím chụp ngay màn hình đang có con trỏ (lưu, tải lên nhanh), ba ô tích mới ở tab Thông thường (thông báo thành công, giữ vị trí vùng, con trỏ trong ảnh).

**Architecture:** Core giữ toàn bộ dữ liệu và luật thuần: ba ô phím tắt trong `Config`, kiểm tra trùng, loại thông báo `Success`, hình học của khung nhớ. Lớp Windows đăng ký ba phím bằng ba mã riêng, lọc thông báo ở một chỗ (`WM_NOTICE`), vẽ con trỏ vào ảnh ngay trong `capture_virtual_screen`, và cho hai phím mới đi qua cổng chụp hiện có kèm một "loại chụp". Chụp toàn màn hình là chụp màn hình ảo rồi cắt lấy màn hình chứa con trỏ.

**Tech Stack:** như các kế hoạch trước. Không thêm crate. Có thể bật thêm feature của crate `windows` nếu thiếu.

**Spec:** `docs/superpowers/specs/2026-11-16-capture-batch2-design.md` (gọi là "thiết kế đợt 2"). Thiết kế đợt 1 và spec gốc vẫn đúng ở mọi chỗ thiết kế đợt 2 không nói khác.

## Global Constraints

- Chỉ mở kết nối mạng đến hai nơi: `https://{account_id}.r2.cloudflarestorage.com` và địa chỉ công khai của kho.
- Secret Access Key không bao giờ xuất hiện ở dạng đọc được trên đĩa, trong log, trong thông báo lỗi, trong `Debug` hay đầu ra kiểm thử.
- Không ghi log ra file. Không thêm crate mới.
- `shotme-core` không phụ thuộc crate `windows` hay API riêng của hệ điều hành.
- Mọi chữ người dùng nhìn thấy lấy từ bảng `shotme_core::i18n`, đủ hai ngôn ngữ; không viết cứng. Không dịch tên phím và tên riêng. Tiêu đề thông báo khay mới phải nằm trong `NOTICE_TITLES` (giới hạn 63 đơn vị UTF-16).
- File cấu hình của bản hiện tại (không có trường mới) phải đọc được và cho đúng hành vi hiện tại: dòng 1 bật với phím đang có, dòng 2 và 3 tắt, thông báo bật, giữ vị trí tắt, con trỏ tắt. Trường `hotkey` trong file giữ nguyên tên và nghĩa.
- Bản release dùng `panic = "abort"`: mã chạy thật không `unwrap`/`expect`/chỉ số có thể panic trên dữ liệu đầu vào. `unsafe` nhỏ, kèm `// SAFETY:`. Giải phóng mọi handle trên mọi đường đi.
- Mọi tọa độ là pixel vật lý.
- Thông báo loại lỗi và thông tin luôn hiện; chỉ loại thành công bị ẩn khi người dùng tắt.
- Không làm mất ảnh: hai phím mới dùng đúng các đường lưu dự phòng hiện có (tải lỗi thì lưu; thoát lúc đang tải thì lưu).
- Hành vi đã qua rà soát giữ nguyên trừ chỗ thiết kế đợt 2 đổi: máy trạng thái bắt phím (`CaptureState`, `capture_hotkey`, quyết định 22), Hủy đăng ký lại phím cũ, Kiểm tra kết nối chạy nền, `gate_step`, hàng đợi thông báo của khay (`enqueue_notice`, `take_notice_to_show`), lớp phủ khi không có khung nhớ.
- Không làm ở đợt này: mục menu khay mới, tab Định dạng, Proxy, nhớ vị trí vùng qua lần khởi động, co giãn con trỏ, `WM_DPICHANGED`.
- Quy trình: commit kiểm thử trước (`test:`, thân hàm mới là `todo!()`), rồi commit mã (`feat:`). Chạy `cargo fmt --all` trước khi commit. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`. Không chuyển nhánh. Chạy cargo qua PowerShell. Không tắt tiến trình `shotme` không do mình khởi động; nếu exe bị khóa thì bỏ build release và báo lại. Không đẩy lên remote. Không đọc hay ghi khóa R2, không đọc file cấu hình thật trong `%APPDATA%`.
- Kiểm thử mới ở crate Windows không khẳng định chữ đọc từ `lang::tr` (ngôn ngữ hiện hành là biến toàn cục); dùng `t(Lang::Vi, …)`.

## Review Focus

Năm tình huống thiết kế hàm ý mà dễ gặp khi dùng thật. Mỗi dòng có kiểm thử ở task ghi trong ngoặc.

1. Nâng cấp từ bản đợt 1: file cấu hình chỉ có `hotkey`, không có trường mới; phím cũ phải còn nguyên và đang bật, không gì khác tự bật. (Task 20: `old_file_keeps_behaviour`)
2. Hai dòng đang bật cùng một tổ hợp thì OK bị chặn và chỉ đúng dòng đứng sau; một dòng đang tắt trùng tổ hợp thì không phải lỗi. (Task 20: `conflict_only_between_enabled_rows`; Task 21: `hotkey_conflict_is_a_form_error`)
3. Tắt thông báo mà tải lên được nhưng không chép được link, hoặc lưu thất bại: thông báo đó là lỗi và vẫn phải hiện. (Task 20: `only_success_notices_can_be_hidden`, `notice_kinds_of_outcomes`)
4. Khung đang nhớ nằm trên màn hình đã rút, hoặc gốc màn hình ảo đã đổi (màn hình phụ ở bên trái, tọa độ âm): lớp phủ mở trống, không panic, không chọn vùng ngoài ảnh. (Task 20: `restore_selection_cases`)
5. Bấm phím toàn màn hình lúc menu khay đang mở hoặc đang chờ 250 ms của "Chụp màn hình": lượt chụp diễn ra đúng một lần và đúng loại của yêu cầu sau cùng; bấm lúc lớp phủ đang mở thì bị bỏ qua và không đổi loại của lượt đang chạy. (Task 22: `capture_kind_follows_the_accepted_request`)

## Cấu trúc file

| File | Việc | Task |
|---|---|---|
| `crates/shotme-core/src/config.rs` | `HotkeyRole`, `HotkeySlot`, `Hotkeys`, ba cờ mới, đọc/ghi và nâng cấp file | 20 |
| `crates/shotme-core/src/actions.rs` | `NoticeKind::Success`, `notice_visible` | 20 |
| `crates/shotme-core/src/geometry.rs` | `Selection::with_rect`, `restore_selection` | 20 |
| `crates/shotme-core/src/i18n.rs` | chuỗi mới (20), lời văn Trợ giúp và `HelpKeys` (21) | 20, 21 |
| `crates/shotme-windows/src/hotkey.rs` | ba mã phím, đăng ký theo `HotkeyRole` | 21 |
| `crates/shotme-windows/src/settings.rs` | ba dòng phím tắt, ba ô tích, lỗi trùng phím | 21 |
| `crates/shotme-windows/src/main.rs` | đăng ký ba phím, lọc thông báo, đóng Thông tin/Trợ giúp (21); loại chụp, hai luồng toàn màn hình, khung nhớ (22) | 21, 22 |
| `crates/shotme-windows/src/help.rs` | nhận `HelpKeys` | 21 |
| `crates/shotme-windows/src/capture.rs` | con trỏ vào ảnh, vị trí con trỏ lúc chụp | 22 |
| `crates/shotme-windows/src/overlay.rs` | mở với khung ban đầu | 22 |
| `docs/testing/manual-checklist.md` | mục "Đợt 2" | 21, 22 |
| `README.md`, `memory-bank/{productContext,systemPatterns,techContext}.md` | tài liệu | 23 |

## Giao diện giữa các task

Agent của mỗi task chỉ đọc gói của mình; các tên dưới đây được chép vào gói nào cần.

**Task 20 tạo ra (`shotme_core`)**

```rust
// config.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HotkeyRole { Region, SaveFull, UploadFull }
impl HotkeyRole {
    pub const ALL: [HotkeyRole; 3] = [HotkeyRole::Region, HotkeyRole::SaveFull, HotkeyRole::UploadFull];
    pub fn label(self) -> Msg;            // nhãn của dòng trong tab Phím tắt
    pub fn default_slot(self) -> HotkeySlot;
    // Region: bật, PrintScreen. SaveFull: tắt, Shift+PrintScreen. UploadFull: tắt, Ctrl+PrintScreen.
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HotkeySlot { pub enabled: bool, pub key: Hotkey }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hotkeys { pub region: HotkeySlot, pub save_full: HotkeySlot, pub upload_full: HotkeySlot }
impl Default for Hotkeys;                 // ba default_slot
impl Hotkeys {
    pub fn get(&self, role: HotkeyRole) -> HotkeySlot;
    pub fn set(&mut self, role: HotkeyRole, slot: HotkeySlot);
    /// Cặp dòng đang bật đầu tiên có cùng tổ hợp, theo thứ tự `ALL`: (dòng trước, dòng sau).
    pub fn first_conflict(&self) -> Option<(HotkeyRole, HotkeyRole)>;
}
pub struct Config {
    /* r2, expiry, save_dir, autostart, language như cũ */
    pub hotkeys: Hotkeys,                 // thay trường `hotkey`
    pub show_success_notices: bool,       // mặc định true
    pub keep_selection: bool,             // mặc định false
    pub include_cursor: bool,             // mặc định false
}

// actions.rs
pub enum NoticeKind { Success, Info, Error }
pub fn notice_visible(kind: NoticeKind, show_success: bool) -> bool;

// geometry.rs
impl Selection {
    /// Như `new`, nhưng đã chọn sẵn `rect` nếu nó nằm trọn trong `bounds` và không nhỏ hơn cỡ tối thiểu.
    pub fn with_rect(bounds: Rect, rect: Option<Rect>) -> Selection;
}
/// Khung nhớ (tọa độ màn hình) đổi sang tọa độ ảnh; `None` nếu không nằm trọn trong ảnh
/// `image_w`×`image_h` hoặc không chạm màn hình nào trong `monitors` (tọa độ ảnh).
pub fn restore_selection(remembered: Rect, origin: Point, image_w: i32, image_h: i32, monitors: &[Rect]) -> Option<Rect>;

// i18n.rs: chuỗi mới (tên do Task 20 đặt, ghi vào báo cáo), gồm
//   nhãn dòng 2, dòng 3; ba nhãn ô tích; tiêu đề thông báo phím bị chiếm (trong NOTICE_TITLES)
pub fn hotkeys_conflict(lang: Lang, first_label: &str, second_label: &str, combo: &str) -> String;
pub fn hotkey_row_taken(lang: Lang, row_label: &str, combo: &str) -> String;
pub fn hotkeys_taken_body(lang: Lang, combos: &[String]) -> String;   // một hoặc nhiều tổ hợp
```

Trong file cấu hình: `hotkey` (như cũ, tổ hợp của dòng 1), `hotkey_enabled`, `hotkey_save_full`, `hotkey_upload_full` (mỗi cái gồm `enabled`, `ctrl`, `alt`, `shift`, `win`, `vk`), `notify_success`, `keep_selection`, `include_cursor`.

**Task 21 tạo ra (`shotme-windows`, và `help_text` ở core)**

```rust
// shotme_core::i18n
pub struct HelpKeys { pub region: Option<String>, pub save_full: Option<String>, pub upload_full: Option<String> } // None = dòng đang tắt
pub fn help_text(lang: Lang, keys: &HelpKeys) -> String;          // thay help_text(lang, hotkey: &str)

// hotkey.rs
pub fn id_of(role: HotkeyRole) -> i32;                 // Region = 1 (như HOTKEY_ID cũ), SaveFull = 2, UploadFull = 3
pub fn role_of(id: i32) -> Option<HotkeyRole>;
pub fn register(hwnd: HWND, role: HotkeyRole, hk: &Hotkey) -> bool;
pub fn unregister_all(hwnd: HWND);
/// Đăng ký mọi dòng đang bật; trả về các dòng không đăng ký được (các dòng khác vẫn được đăng ký).
pub fn register_enabled(hwnd: HWND, keys: &Hotkeys) -> Vec<HotkeyRole>;
pub fn help_keys(keys: &Hotkeys) -> HelpKeys;          // thuần

// settings.rs
pub enum Field { SaveDir, HotkeyRow(HotkeyRole), AccountId, Bucket, PublicUrl }
pub struct Form { /* như cũ, `hotkey` thành */ pub hotkeys: Hotkeys,
                  pub show_success_notices: bool, pub keep_selection: bool, pub include_cursor: bool }

// help.rs
pub fn show(parent: HWND, keys: &HelpKeys);
```

Sau Task 21: `WM_HOTKEY` của `SaveFull` và `UploadFull` đã tới `main.rs` nhưng chưa làm gì; Task 22 nối chúng.

**Task 22 tạo ra (`shotme-windows`)**

```rust
// capture.rs
pub struct Screenshot { /* như cũ */ pub cursor: Point }   // vị trí con trỏ lúc chụp, tọa độ màn hình
pub fn capture_virtual_screen(include_cursor: bool) -> windows::core::Result<Screenshot>;
pub fn to_screen_coords(image: Rect, origin: Point) -> Rect;                 // ngược của to_image_coords
pub fn cursor_draw_origin(pos: Point, hotspot: Point, origin: Point) -> Point; // thuần

// overlay.rs
pub fn pick_region(shot: &Screenshot, initial: Option<Rect>) -> Result<Option<(Action, Rect)>, String>;

// main.rs
pub enum CaptureKind { Region, SaveFull, UploadFull }
pub fn kind_of(role: HotkeyRole) -> CaptureKind;
/// Loại của lượt chụp sắp chạy sau khi cổng trả `act` cho một yêu cầu loại `requested`.
pub fn next_kind(current: CaptureKind, requested: CaptureKind, act: GateAction) -> CaptureKind;
/// Khung nhớ sau một lượt chụp vùng: Upload, Save, Copy ghi `picked`; Cancel giữ `prev`.
pub fn remember_after(prev: Option<Rect>, action: Action, picked: Rect) -> Option<Rect>;
```

## Task

| Task | Gói | Agent | Model | Rà soát |
|---|---|---|---|---|
| 20 Lõi đợt 2 | `docs/tasks/20-batch2-core.md` | builder | sonnet | reviewer |
| 21 Tùy chọn: ba phím tắt, ba ô tích, lọc thông báo, Trợ giúp | `docs/tasks/21-hotkeys-options.md` | specialist | opus | specialist |
| 22 Chụp: con trỏ, toàn màn hình, khung nhớ | `docs/tasks/22-fullscreen-cursor-selection.md` | specialist | opus | specialist |
| 23 Tài liệu đợt 2 | `docs/tasks/23-docs-batch2.md` | scaffolder | haiku | reviewer |

Thứ tự bắt buộc: 20 → 21 → 22 → 23. Dừng sau Task 22 để chủ dự án thử tay theo các mục "Đợt 2" của `docs/testing/manual-checklist.md`; Task 23 chạy sau khi họ báo kết quả.

Sau mỗi task cả workspace phải build và qua `scripts/check.ps1`. Task 20 đổi kiểu của core nên sửa tối thiểu các chỗ gọi ở lớp Windows, không đổi hành vi nhìn thấy.
