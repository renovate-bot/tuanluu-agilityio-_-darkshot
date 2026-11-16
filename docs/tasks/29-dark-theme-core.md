# Task 29: Lõi giao diện tối, Thông tin, Trợ giúp, menu khay, lớp phủ

**Agent:** `builder` · **Model:** sonnet · **Nhánh:** `task/29-dark-theme-core` · **Rà soát:** `reviewer`

Gói task tự chứa. Thuộc kế hoạch `docs/superpowers/plans/2026-11-16-darkshot-dark-theme.md`. Chạy sau Task 28.

## Cần đọc

- Thiết kế đợt 4 `docs/superpowers/specs/2026-11-16-darkshot-dark-theme-design.md` mục 7
- Kế hoạch đợt 4: "Global Constraints", "Bảng màu đã kiểm" (bảng này thắng mục 7.1 của thiết kế), "Interfaces chung", "Review Focus" dòng 4
- `crates/darkshot-windows/CLAUDE.md`
- `crates/darkshot-windows/src/{about.rs, help.rs, winutil.rs, tray.rs, overlay.rs (hằng màu dòng 76–84 và các hàm vẽ), main.rs (khởi động)}`

## Global Constraints

- Không thêm crate. Bật thêm feature `Win32_Graphics_Dwm` của crate `windows` (và feature khác nếu thiếu). `darkshot-core::theme` chỉ là số, không phụ thuộc crate `windows`.
- Không chạm `settings.rs` (Task 30 làm trọn cửa sổ Tùy chọn).
- Không đổi bố cục, chữ hay hành vi: Thông tin và Trợ giúp vẫn đóng bằng Enter, Esc, nút Đóng; không chặn ứng dụng; tự đóng khi ngôn ngữ đổi. Menu khay giữ năm mục và mục đậm.
- Ảnh chụp ra không đổi: chỉ đổi màu các phần vẽ bên trên ảnh trong lớp phủ (thanh công cụ, chú thích, nhãn kích thước, viền vùng chọn, tay nắm). Lớp tối ngoài vùng chọn giữ nguyên.
- Windows 10 và 11: API không có (DWM thuộc tính 20/19, ordinal 135/136 của `uxtheme.dll`) thì bỏ qua, không lỗi, không panic. Nạp `uxtheme.dll` bằng `GetModuleHandleW`/`LoadLibraryW` + `GetProcAddress` theo số thứ tự, kiểm `None`.
- Chổi dùng chung tạo một lần (lười) và sống suốt tiến trình; không tạo chổi trong mỗi `WM_CTLCOLOR*`. Phông đậm hoặc lớn hơn của Thông tin tạo khi mở cửa sổ và xóa ở `WM_NCDESTROY`. Mở đóng nhiều lần không rò GDI.
- Release `panic = "abort"`: không `unwrap`/`expect`/chỉ số có thể panic trên đường chạy thật. `unsafe` nhỏ, kèm `// SAFETY:`.
- Kiểm thử mới không khẳng định chữ đọc từ `lang::tr`; dùng `t(Lang::Vi, …)`.
- Commit kiểm thử trước (`test:`, thân hàm mới `todo!()`), rồi mã (`feat:`). `cargo fmt --all`. Chạy cargo qua PowerShell. Kiểm `git branch --show-current` trước khi commit; không chuyển nhánh. Không tắt tiến trình `shotme`/`darkshot` không do mình khởi động; nếu exe bị khóa thì bỏ build release và báo lại. Không đẩy lên remote. Không đọc hay ghi khóa R2, file cấu hình thật. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.

## Interfaces

**Produces** (Task 30 dùng):

```rust
// crates/darkshot-core/src/theme.rs (pub mod theme)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);
impl Rgb { pub const fn hex(v: u32) -> Rgb; pub const fn colorref(self) -> u32; }
pub fn relative_luminance(c: Rgb) -> f64;
pub fn contrast_ratio(a: Rgb, b: Rgb) -> f64;
// Mọi hằng trong "Bảng màu đã kiểm" của kế hoạch, đúng tên và giá trị, cộng WHITE = #FFFFFF.
pub const TEXT_PAIRS: &[(Rgb, Rgb)];  // ít nhất: TEXT trên BG, SURFACE, INPUT, BUTTON, BUTTON_HOVER, BUTTON_PRESSED, TAB_HOVER, OVERLAY_LABEL, OVERLAY_TIP;
                                      // TEXT_MUTED trên BG, SURFACE, INPUT; DANGER và SUCCESS trên BG, SURFACE;
                                      // WHITE trên ACCENT_BUTTON, ACCENT_BUTTON_HOVER, ACCENT_BUTTON_PRESSED
pub const UI_PAIRS: &[(Rgb, Rgb)];    // ít nhất: ACCENT trên BG, SURFACE, INPUT, OVERLAY_HOVER; CHECK_BORDER trên SURFACE;
                                      // INPUT_BORDER... không bắt buộc (viền trang trí); OVERLAY_ICON trên SURFACE

// crates/darkshot-windows/src/theme.rs
pub fn apply_dark_title_bar(hwnd: HWND);
pub fn enable_dark_menus();
#[derive(Clone, Copy)] pub enum DarkClass { Explorer, Cfd }  // "DarkMode_Explorer" | "DarkMode_CFD"
pub fn set_dark_control_theme(hwnd: HWND, class: DarkClass);
pub fn brush(c: Rgb) -> HBRUSH;
pub fn ctl_color(hdc: HDC, fg: Rgb, bg: Rgb) -> LRESULT;
pub fn colorref(c: Rgb) -> COLORREF;
```

## Các bước

- [ ] **1. Kiểm thử trước.** Commit `test: dark theme core`.
  - Core `theme`:
    - `contrast_matches_wcag_examples`: `contrast_ratio(WHITE, Rgb(0,0,0))` trong 21.0 ± 0.01; `contrast_ratio(Rgb::hex(0x777777), WHITE)` trong 4.48 ± 0.01; đối xứng `contrast_ratio(a,b) == contrast_ratio(b,a)`.
    - `palette_values_match_plan`: `TEXT == Rgb::hex(0xE7E9EC)`, `BG == Rgb::hex(0x1C1E22)`, `ACCENT == Rgb::hex(0x4C8DFF)`, `ACCENT_BUTTON_HOVER == Rgb::hex(0x2563EB)`, `DANGER == Rgb::hex(0xFF7B72)`.
    - `colorref_is_bgr`: `Rgb::hex(0x112233).colorref() == 0x0033_2211`.
    - `text_pairs_meet_aa`: mọi cặp trong `TEXT_PAIRS` có `contrast_ratio >= 4.5`; thông báo lỗi in cặp hỏng.
    - `ui_pairs_meet_non_text_contrast`: mọi cặp trong `UI_PAIRS` `>= 3.0`.
  - Windows:
    - `theme_brush_is_cached`: `brush(BG) == brush(BG)` (cùng handle).
    - `about_help_no_gdi_leak` (theo mẫu kiểm thử chống rò GDI của lớp phủ hiện có, `GetGuiResources(GR_GDIOBJECTS)`): mở rồi đóng Thông tin và Trợ giúp 20 lần (bơm thông điệp tới khi đóng hẳn), số đối tượng GDI sau lần cuối không lớn hơn sau lần đầu. Chạy thường nếu ổn định; nếu cần màn hình thật thì `#[ignore]` kèm lý do và báo lệnh chạy.
- [ ] **2. Mã.**
  - `darkshot-core/src/theme.rs` theo Interfaces.
  - `darkshot-windows/src/theme.rs` theo Interfaces. `apply_dark_title_bar` gọi `DwmSetWindowAttribute(hwnd, DWMWINDOWATTRIBUTE(20), &TRUE, 4)`, lỗi thì thử 19. `enable_dark_menus`: ordinal 135 `SetPreferredAppMode(2)` rồi ordinal 136 `FlushMenuThemes()`; gọi một lần trong `main` trước khi tạo cửa sổ ẩn.
  - Thông tin (`about.rs`): `apply_dark_title_bar` trước `ShowWindow`; nền `BG` (lớp cửa sổ dùng `brush(BG)` hoặc xử lý `WM_ERASEBKGND`); `WM_CTLCOLORSTATIC`: tên ứng dụng `TEXT`, phiên bản và mô tả `TEXT_MUTED`, nền `BG`; tên ứng dụng dùng phông hệ thống đậm (weight 600, không có thì 700) cỡ 1,5 lần; nút Đóng tạm dùng `set_dark_control_theme(Explorer)` (Task 30 đổi sang nút chính tự vẽ).
  - Trợ giúp (`help.rs`): tiêu đề tối; nền `BG`; ô chữ chỉ đọc nhiều dòng: `WM_CTLCOLORSTATIC` (ô chỉ đọc gửi thông điệp này) và `WM_CTLCOLOREDIT` trả chữ `TEXT` nền `INPUT`; `set_dark_control_theme(Explorer)` cho ô chữ (thanh cuộn tối) và nút Đóng.
  - Menu khay (`tray.rs`): không đổi mã vẽ; `enable_dark_menus` đủ. Nếu menu vẫn sáng khi chạy thử, ghi vào báo cáo, không tự vẽ menu.
  - Lớp phủ (`overlay.rs`): thay hằng màu: nền thanh công cụ `SURFACE`, viền và vạch ngăn `OVERLAY_BORDER`, biểu tượng `OVERLAY_ICON` (`ICON_ARGB` = `0xFF` + RGB), khi trỏ nền `OVERLAY_HOVER` và biểu tượng `ACCENT`; viền vùng chọn `ACCENT`; tay nắm ruột `WHITE` viền `ACCENT`; nhãn kích thước nền `OVERLAY_LABEL` chữ `TEXT`; chú thích nút nền `OVERLAY_TIP` viền `OVERLAY_BORDER` chữ `TEXT`. Không đổi kích thước, vị trí hay lớp tối ngoài vùng chọn. Bỏ các hằng cũ không còn dùng.
- [ ] **3. Checklist.** Thêm mục "Đợt 4: giao diện tối (Thông tin, Trợ giúp, menu khay, lớp phủ)" vào `docs/testing/manual-checklist.md`, mỗi dòng `[ ]`: thanh tiêu đề tối ở Windows 11 và Windows 10 bản mới; Thông tin và Trợ giúp không nhá trắng khi mở; chữ đọc rõ ở hai ngôn ngữ, DPI 100% và 150%; thanh cuộn Trợ giúp tối; Enter, Esc, nút Đóng vẫn đóng; menu khay tối và mục đầu đậm; lớp phủ trên hai màn hình: thanh công cụ tối, nút khi trỏ đổi nền và biểu tượng xanh, chú thích tối, viền và tay nắm xanh; ảnh lưu ra giống màn hình (không có màu giao diện).
- [ ] **4.** `cargo fmt --all`; `powershell -File scripts/check.ps1` in `ALL CHECKS PASSED`. Commit `feat: dark theme for About, Help, tray menu and overlay`.

**Xong khi:** check qua; báo cáo nêu các commit, dòng cuối của check, kiểm thử chống rò GDI chạy thường hay `#[ignore]`, menu khay có tối khi chạy thử không (nếu đã chạy được), và những gì chưa nhìn bằng mắt.
