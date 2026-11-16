# darkshot đợt 4 (đổi tên, giao diện luôn tối) Implementation Plan

> **For agentic workers:** kế hoạch này chạy bằng quy trình của dự án: `/next-task` giao từng gói `docs/tasks/NN-ten-ngan.md` cho agent ghi trong `memory-bank/progress.md`, rồi rà soát, kiểm tra, gộp. Các bước chi tiết (dạng `- [ ]`) nằm trong gói task; file này giữ phần chung mà mọi gói dựa vào.

**Goal:** Ứng dụng mang tên darkshot ở mọi nơi, người dùng shotme chuyển sang không mất gì, và mọi cửa sổ, menu khay, lớp phủ đều tối giống prototype đã duyệt.

**Architecture:** Đổi tên là thay thế máy móc (Task 27), trừ các tên cũ cần cho việc chuyển dữ liệu, gom trong `legacy.rs` (Task 28). Logic chuyển dữ liệu và bảng màu là hàm và hằng thuần trong `darkshot-core` (`migrate`, `theme`), có kiểm thử. Lớp Windows có một module `theme.rs` (thanh tiêu đề tối, chổi, chế độ tối của menu, vẽ nút) mà mọi cửa sổ dùng chung; điều khiển gốc được giữ, chỉ phần vẽ đổi (owner-draw, custom draw, `WM_CTLCOLOR*`, vẽ viền ô nhập).

**Tech Stack:** như các đợt trước. Không thêm crate. Có thể bật thêm feature của crate `windows`: `Win32_Graphics_Dwm` (thanh tiêu đề), `Win32_UI_Accessibility` nếu cần.

**Spec:** `docs/superpowers/specs/2026-11-16-darkshot-dark-theme-design.md` (gọi là "thiết kế đợt 4"). Các thiết kế trước vẫn đúng ở mọi chỗ thiết kế đợt 4 không nói khác.

## Global Constraints

- Tên mới viết thường: `darkshot`. Crate `darkshot-core` (thư viện `darkshot_core`), `darkshot-windows`, exe `darkshot.exe`.
- Không thêm crate, kể cả `dev-dependencies`; `Cargo.lock` không thêm gói. `darkshot-core` không phụ thuộc crate `windows` hay API riêng của hệ điều hành.
- Mọi chữ người dùng thấy lấy từ bảng `darkshot_core::i18n`, đủ hai ngôn ngữ; không viết cứng.
- Không đổi bố cục, thứ tự ô, chữ hay hành vi của cửa sổ nào ngoài phần nhìn và tên ứng dụng. Hành vi đã rà soát của Tùy chọn giữ nguyên: chuyển tab bằng chuột, Ctrl+Tab, Ctrl+Shift+Tab; máy trạng thái bắt phím; lỗi nhảy đúng tab và đúng ô; OK, Hủy, Esc, Enter; Kiểm tra kết nối chạy nền; đăng ký lại phím tắt; phím gạch dưới (Alt+chữ); thứ tự Tab.
- Ảnh chụp ra không đổi màu: chỉ phần giao diện vẽ bên trên lớp phủ đổi.
- Không được mất cấu hình, khóa R2 hay thiết lập khởi động của người dùng shotme. Không xóa `%APPDATA%\shotme`.
- File cấu hình hiện tại đọc được nguyên vẹn; mọi trường giữ tên và nghĩa.
- Hỗ trợ Windows 10 và 11; API không có thì bỏ qua, không lỗi. Mọi kích thước vẽ tay tính theo DPI của cửa sổ.
- Bản release dùng `panic = "abort"`: mã chạy thật không `unwrap`/`expect`/chỉ số có thể panic trên dữ liệu đầu vào. `unsafe` nhỏ, kèm `// SAFETY:`. Giải phóng mọi handle GDI (chổi, bút, phông, DC) trên mọi đường đi; chổi dùng chung tạo một lần và sống suốt tiến trình hoặc suốt cửa sổ, không tạo trong mỗi `WM_CTLCOLOR*`.
- Không sửa spec, kế hoạch đã duyệt và gói task cũ (`docs/tasks/01` đến `26`), `prototype/`.
- Quy trình: commit kiểm thử trước (`test:`, thân hàm mới là `todo!()`), rồi commit mã (`feat:`). Chạy `cargo fmt --all` trước khi commit. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`. Kiểm `git branch --show-current` trước khi commit; không chuyển nhánh. Chạy cargo qua PowerShell. Không tắt tiến trình `shotme` hay `darkshot` không do mình khởi động; nếu exe bị khóa thì bỏ build release và báo lại. Không đẩy lên remote. Không đọc hay ghi khóa R2, không đọc hay ghi file cấu hình thật, khóa Run thật hay WER thật của máy (kiểm thử dùng thư mục tạm và tên giá trị giả).
- Kiểm thử mới ở crate Windows không khẳng định chữ đọc từ `lang::tr`; dùng `t(Lang::Vi, …)`.

## Bảng màu đã kiểm

Thiết kế đợt 4 mục 7.1 ghi vài tỉ lệ tương phản cao hơn thực tế, và màu nút chính khi trỏ `#3D7BF0` chỉ đạt 3,97:1 với chữ trắng. Kế hoạch dùng bảng dưới (đã tính theo công thức WCAG 2.x); đổi duy nhất là màu khi trỏ của nút chính. Ghi vào `decisions.md` ở Task 31.

| Hằng (`darkshot_core::theme`) | Màu | Dùng cho |
|---|---|---|
| `BG` | `#1C1E22` | nền cửa sổ, hàng nút dưới, nền Thông tin |
| `SURFACE` | `#24272C` | nền khung tab, thanh công cụ lớp phủ |
| `INPUT` | `#15171A` | nền ô nhập, hộp chọn, ô Trợ giúp |
| `LINE` | `#383C43` | đường kẻ dưới thanh tab |
| `INPUT_BORDER` | `#4A4F57` | viền ô nhập, hộp chọn |
| `BUTTON` / `BUTTON_HOVER` / `BUTTON_PRESSED` | `#33373E` / `#3C4048` / `#2B2E33` | nền nút thường |
| `BUTTON_BORDER` | `#454A52` | viền nút thường |
| `TAB_HOVER` | `#2E3238` | nền tab khi trỏ |
| `CHECK_BORDER` | `#8A9099` | viền ô tích và nút chọn khi tắt |
| `TRACK` | `#454A52` | rãnh thanh trượt |
| `TEXT` | `#E7E9EC` | chữ chính, nhãn kích thước lớp phủ |
| `TEXT_MUTED` | `#A3A9B2` | chữ phụ, ô chỉ đọc, ghi chú, tab không chọn |
| `TEXT_DISABLED` | `#7C828B` | chữ của điều khiển bị vô hiệu (WCAG miễn) |
| `ACCENT` | `#4C8DFF` | gạch dưới tab, viền tiêu điểm ô nhập, ô tích bật (viền), nút chọn, thanh trượt, viền vùng chọn, biểu tượng lớp phủ khi trỏ |
| `ACCENT_BUTTON` / `ACCENT_BUTTON_HOVER` / `ACCENT_BUTTON_PRESSED` | `#2F6FE4` / `#2563EB` / `#1D4ED8` | nền nút chính (OK, Đóng), nền ô tích bật; chữ `#FFFFFF` |
| `DANGER` | `#FF7B72` | chữ lỗi |
| `SUCCESS` | `#4ADE80` | chữ thành công |
| `OVERLAY_BORDER` | `#3F434A` | viền và vạch ngăn thanh công cụ lớp phủ, viền chú thích |
| `OVERLAY_ICON` | `#D5D9DF` | biểu tượng lớp phủ |
| `OVERLAY_HOVER` | `#363A41` | nền nút lớp phủ khi trỏ |
| `OVERLAY_LABEL` | `#16181B` | nền nhãn kích thước |
| `OVERLAY_TIP` | `#2B2D31` | nền chú thích nút lớp phủ |

Tỉ lệ thật (chữ trên nền): `TEXT`/`BG` 13,72; `TEXT`/`SURFACE` 12,32; `TEXT`/`BUTTON_HOVER` 8,55; `TEXT_MUTED`/`BG` 7,05; `TEXT_MUTED`/`SURFACE` 6,33; `DANGER`/`SURFACE` 5,94; `SUCCESS`/`SURFACE` 8,60; trắng/`ACCENT_BUTTON` 4,65; trắng/`ACCENT_BUTTON_HOVER` 5,17. Thành phần giao diện (WCAG 1.4.11, ≥ 3:1): `ACCENT`/`SURFACE` 4,68; `ACCENT`/`OVERLAY_HOVER` 3,57; `CHECK_BORDER`/`SURFACE` 4,66.

## Interfaces chung

```rust
// darkshot_core::theme (Task 29)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);
impl Rgb {
    pub const fn hex(v: u32) -> Rgb;          // 0xRRGGBB
    pub const fn colorref(self) -> u32;       // 0x00BBGGRR cho GDI
}
pub fn relative_luminance(c: Rgb) -> f64;      // WCAG 2.x
pub fn contrast_ratio(a: Rgb, b: Rgb) -> f64;  // (L1 + 0.05) / (L2 + 0.05), L1 >= L2
pub const WHITE: Rgb;  // và mọi hằng trong bảng trên
pub const TEXT_PAIRS: &[(Rgb, Rgb)];           // (chữ, nền) đang dùng, phải >= 4.5
pub const UI_PAIRS: &[(Rgb, Rgb)];             // (thành phần, nền) đang dùng, phải >= 3.0

// darkshot_core::migrate (Task 28)
pub const LEGACY_APP_DIR: &str = "shotme";     // tên cũ chỉ được nằm ở đây và ở legacy.rs
pub fn migrated_config(old_json: &[u8], legacy_default_save_dir: &Path) -> Result<Vec<u8>, MigrateError>;
pub fn should_copy_config(new_exists: bool, old_exists: bool) -> bool;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunAction { Write, Delete, Nothing }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunPlan { pub delete_legacy: bool, pub darkshot: RunAction }
pub fn plan_run(legacy_present: bool, darkshot_present: bool, autostart: bool) -> RunPlan;

// darkshot-windows: theme.rs (Task 29, Task 30 mở rộng)
pub fn apply_dark_title_bar(hwnd: HWND);       // DWMWA_USE_IMMERSIVE_DARK_MODE = 20, lỗi thì thử 19, lỗi nữa thì bỏ
pub fn enable_dark_menus();                    // uxtheme ordinal 135 SetPreferredAppMode(ForceDark = 2) + ordinal 136 FlushMenuThemes; gọi một lần lúc khởi động
pub fn set_dark_control_theme(hwnd: HWND, class_hint: DarkClass); // SetWindowTheme DarkMode_Explorer / DarkMode_CFD
pub fn brush(c: Rgb) -> HBRUSH;                // chổi dùng chung, tạo lười, sống suốt tiến trình
pub fn ctl_color(hdc: HDC, fg: Rgb, bg: Rgb) -> LRESULT; // SetTextColor, SetBkColor, trả brush(bg)
// Task 30: pub fn draw_button(dis: &DRAWITEMSTRUCT, kind: ButtonKind, dpi: u32); ButtonKind { Normal, Primary }
```

## Review Focus

Năm tình huống thiết kế hàm ý mà dễ gặp khi dùng thật. Mỗi dòng có kiểm thử ở task ghi trong ngoặc.

1. Người dùng shotme có cấu hình đầy đủ (khóa R2 mã hóa, `autostart` bật, `save_dir` mặc định cũ) chạy darkshot lần đầu: cấu hình chép nguyên byte trừ việc chèn `save_dir` khi thiếu; khóa vẫn giải mã; Run có `darkshot`, không còn `shotme`. (Task 28: `copied_config_keeps_every_field`, `missing_save_dir_points_to_legacy_folder`, `plan_run_*`, `migration_round_trip_in_temp_dir`)
2. Đã có cấu hình darkshot với `autostart` tắt mà khóa Run còn `shotme`: xóa `shotme`, không ghi `darkshot`, không ghi đè cấu hình darkshot. (Task 28: `plan_run_legacy_does_not_enable`, `existing_new_config_is_never_overwritten`)
3. File cấu hình shotme hỏng hoặc không phải JSON: không chép, không panic, darkshot chạy với mặc định và báo một thông báo Info. (Task 28: `corrupt_old_config_is_not_copied`)
4. Mở đóng Tùy chọn, Thông tin, Trợ giúp nhiều lần: số đối tượng GDI của tiến trình không tăng. (Task 29: `about_help_no_gdi_leak`; Task 30: `options_no_gdi_leak`)
5. Điều khiển tự vẽ vẫn đi được bằng bàn phím: Tab qua đủ các ô, Space bật tắt ô tích, mũi tên đổi nút chọn và thanh trượt, Enter là OK, Esc là Hủy, Alt+chữ gạch dưới. (Task 30: kiểm thử gửi thông điệp phím tới cửa sổ thật nếu ổn định, không thì `#[ignore]`; luôn có mục thử tay)

## Task

| Task | Gói | Agent | Model | Rà soát |
|---|---|---|---|---|
| 27 Đổi tên | `docs/tasks/27-rename-darkshot.md` | builder | sonnet | reviewer |
| 28 Chuyển dữ liệu từ shotme | `docs/tasks/28-legacy-migration.md` | specialist | opus | specialist |
| 29 Lõi giao diện tối, Thông tin, Trợ giúp, menu khay, lớp phủ | `docs/tasks/29-dark-theme-core.md` | builder | sonnet | reviewer |
| 30 Tùy chọn tự vẽ | `docs/tasks/30-dark-options.md` | specialist | opus | specialist |
| 31 Tài liệu đợt 4 | `docs/tasks/31-docs-batch4.md` | scaffolder | haiku | reviewer |

Thứ tự 27 → 28 → 29 → 30 → 31. Chủ dự án bảo chạy đến hết không hỏi lại (2026-11-16): không dừng sau Task 30; việc thử tay theo mục "Đợt 4" của checklist được báo ở cuối và tài liệu không được ghi là đã thử tay. Vướng mắc ghi vào `memory-bank/activeContext.md` mục "Vấn đề chờ chủ dự án xem".

Giữa Task 29 và Task 30, nền cửa sổ Tùy chọn chưa đổi (Task 29 không chạm `settings.rs`), nên không có giai đoạn chữ tối trên nền tối. Không phát hành bản giữa các task.
