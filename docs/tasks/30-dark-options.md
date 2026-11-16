# Task 30: Cửa sổ Tùy chọn tự vẽ (giao diện tối)

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/30-dark-options` · **Rà soát:** `specialist`

Gói task tự chứa. Thuộc kế hoạch `docs/superpowers/plans/2026-11-16-darkshot-dark-theme.md`. Chạy sau Task 29.

## Cần đọc

- Thiết kế đợt 4 `docs/superpowers/specs/2026-11-16-darkshot-dark-theme-design.md` mục 4.1 (cách A, mức "giống hệt"), 7.2, 7.3
- Kế hoạch đợt 4: "Global Constraints", "Bảng màu đã kiểm" (thắng mục 7.1 của thiết kế), "Interfaces chung", "Review Focus" dòng 4–5
- Prototype (mở bằng trình duyệt, không đưa vào git): `.superpowers/brainstorm/1649-1794846769/content/darkshot-theme.html`, phần "1. Cửa sổ Tùy chọn" và CSS đầu file (kích thước, bo góc, độ dày gạch dưới)
- `crates/darkshot-windows/CLAUDE.md`, `memory-bank/systemPatterns.md` mục "Thêm một ô hoặc một tab"
- `crates/darkshot-windows/src/settings.rs` (toàn bộ), `theme.rs` và `winutil.rs` (Task 29), `about.rs`, `help.rs`

## Global Constraints

- Không thêm crate. Có thể bật thêm feature của crate `windows`.
- Giữ điều khiển gốc (`BUTTON`, `EDIT`, `COMBOBOX`, `SysTabControl32`, `msctls_trackbar32`) để Windows lo bàn phím, tiêu điểm và trợ năng; chỉ đổi phần vẽ. Không thay bằng điều khiển tự viết.
- Hành vi đã rà soát giữ nguyên: chuyển tab bằng chuột, Ctrl+Tab, Ctrl+Shift+Tab; máy trạng thái bắt phím; lỗi nhảy đúng tab và đúng ô; OK (Enter), Hủy (Esc); Kiểm tra kết nối chạy nền; đăng ký lại phím tắt; Alt+chữ gạch dưới; thứ tự Tab; nút chọn PNG/JPEG đổi bằng mũi tên; Space bật tắt ô tích; thanh trượt bằng mũi tên, Page Up/Down, Home/End; làm mờ ba ô chất lượng khi chọn PNG.
- Không đổi bố cục (vị trí, kích thước, thứ tự ô), chữ, bề rộng client hay `Layout`. Nếu ô tích hoặc nút chọn tự vẽ cần bề rộng khác thì giữ vùng bấm như cũ.
- Màu đúng "Bảng màu đã kiểm"; mọi kích thước vẽ tay (hộp ô tích 18 DIP bo 4 DIP, gạch dưới tab 3 DIP, gạch dưới ô nhập có tiêu điểm 2 DIP, núm thanh trượt 18 DIP viền 4 DIP, bo góc nút 4 DIP) nhân theo DPI của cửa sổ.
- Mọi điều khiển có thể nhận tiêu điểm phải có dấu hiệu tiêu điểm nhìn thấy được khi đi bằng bàn phím (khung `TEXT` 1 DIP hoặc gạch dưới `ACCENT`), tôn trọng `UISF_HIDEFOCUS` như Windows.
- Chổi dùng chung qua `theme::brush`; bút và phông tạo trong vẽ thì xóa ngay; phông đậm của tab chọn và nút chính tạo khi mở cửa sổ, xóa ở `WM_NCDESTROY`. Mở đóng nhiều lần không rò GDI.
- Release `panic = "abort"`: không `unwrap`/`expect`/chỉ số có thể panic trên đường chạy thật. `unsafe` nhỏ, kèm `// SAFETY:`.
- Kiểm thử mới không khẳng định chữ đọc từ `lang::tr`; dùng `t(Lang::Vi, …)`.
- Commit kiểm thử trước (`test:`, thân hàm mới `todo!()`), rồi mã (`feat:`). `cargo fmt --all`. Chạy cargo qua PowerShell. Kiểm `git branch --show-current` trước khi commit; không chuyển nhánh. Không tắt tiến trình `shotme`/`darkshot` không do mình khởi động; nếu exe bị khóa thì bỏ build release và báo lại. Không đẩy lên remote. Không đọc hay ghi khóa R2, file cấu hình thật. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.

## Interfaces

**Consumes (Task 29):** `darkshot_core::theme::*`, `theme::{apply_dark_title_bar, set_dark_control_theme, DarkClass, brush, ctl_color, colorref}`.

**Produces:** trong `theme.rs` (dùng chung cho Tùy chọn, Thông tin, Trợ giúp):

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum ButtonKind { Normal, Primary }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControlState { pub hot: bool, pub pressed: bool, pub focused: bool, pub enabled: bool, pub checked: bool }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Colors { pub fill: Rgb, pub border: Rgb, pub text: Rgb }
pub fn button_colors(kind: ButtonKind, s: ControlState) -> Colors;
pub fn check_colors(s: ControlState) -> Colors;      // hộp ô tích: tắt viền CHECK_BORDER, bật nền ACCENT_BUTTON; text = TEXT hoặc TEXT_DISABLED
pub fn radio_colors(s: ControlState) -> Colors;      // bật: viền và chấm ACCENT
pub fn tab_colors(selected: bool, hot: bool) -> Colors; // chọn: TEXT, gạch ACCENT; không chọn: TEXT_MUTED; hot: nền TAB_HOVER
pub fn edit_border(focused: bool, enabled: bool) -> (Rgb, u32 /* độ dày gạch dưới DIP: 2 khi focused, 1 khi không */);
```

Hàm vẽ (chữ ký tùy người làm, ví dụ `draw_button(hdc, rc, text, kind, state, dpi, font)`) cũng nằm ở `theme.rs` để Thông tin và Trợ giúp dùng.

Cách vẽ do người làm chọn, gợi ý ưu tiên giữ kiểu gốc: nút bấm, ô tích, nút chọn và thanh trượt dùng `NM_CUSTOMDRAW` (comctl32 v6, trả `CDRF_SKIPDEFAULT` sau khi tự vẽ) để giữ `BS_AUTOCHECKBOX`, `BS_AUTORADIOBUTTON`, `BS_DEFPUSHBUTTON`; thanh tab dùng `TCS_OWNERDRAWFIXED` hoặc subclass `WM_PAINT`, kèm vẽ nền phần ngoài tab; ô nhập dùng `WM_CTLCOLOREDIT`/`WM_CTLCOLORSTATIC` cho nền và chữ, viền vẽ ở `WM_PAINT` của khung cha (bỏ `WS_BORDER`/`WS_EX_CLIENTEDGE`) hoặc subclass `WM_NCPAINT`, vẽ lại khi `EN_SETFOCUS`/`EN_KILLFOCUS`; hộp chọn dùng `DarkMode_CFD` cho mặt và `WM_CTLCOLORLISTBOX` cho danh sách, viền như ô nhập. Nếu `BS_OWNERDRAW` cần cho nút bấm, phải tự giữ hành vi nút mặc định (Enter = OK) và ghi lý do vào báo cáo.

## Các bước

- [ ] **1. Kiểm thử trước.** Commit `test: dark options`.
  - `primary_button_colors`: `button_colors(Primary, bình thường)` = `{ fill: ACCENT_BUTTON, text: WHITE }`; hot → `ACCENT_BUTTON_HOVER`; pressed → `ACCENT_BUTTON_PRESSED`; không enabled → text `TEXT_DISABLED`.
  - `normal_button_colors`: `{ fill: BUTTON, border: BUTTON_BORDER, text: TEXT }`; hot `BUTTON_HOVER`; pressed `BUTTON_PRESSED`.
  - `check_and_radio_colors`: bốn trạng thái (bật/tắt × enabled/không) cho mỗi loại theo Interfaces.
  - `tab_colors_selected_and_hot`.
  - `edit_border_focus`: `edit_border(true, true) == (ACCENT, 2)`; `edit_border(false, true) == (INPUT_BORDER, 1)`.
  - `every_drawn_text_pair_is_listed`: mọi cặp (text, fill) mà `button_colors`/`check_colors`/`tab_colors` có thể trả với `enabled == true` đều có trong `darkshot_core::theme::TEXT_PAIRS` (để kiểm thử tương phản của Task 29 bao luôn chúng).
  - `options_no_gdi_leak`: mở và đóng Tùy chọn 20 lần (theo cách các kiểm thử chạy tay của `settings.rs` mở cửa sổ), mỗi lần chuyển qua đủ bốn tab; số đối tượng GDI sau lần cuối không lớn hơn sau lần đầu. `#[ignore]` nếu cần màn hình thật; báo lệnh chạy.
  - `options_keyboard_still_works` (nếu ổn định, không thì `#[ignore]` hoặc chỉ ghi checklist): gửi `VK_TAB` qua các ô của tab Thông thường và khẳng định tiêu điểm đi đúng thứ tự cũ; `VK_SPACE` trên ô tích đổi `BM_GETCHECK`; Ctrl+Tab đổi tab.
- [ ] **2. Mã.**
  - `theme.rs`: các hàm màu và hàm vẽ theo Interfaces.
  - `settings.rs`: `apply_dark_title_bar` trước khi hiện; nền cửa sổ và hàng nút `BG`; khung tab `SURFACE`; đường kẻ `LINE` dưới thanh tab; thanh tab tự vẽ (tab chọn chữ `TEXT` đậm, gạch dưới `ACCENT` 3 DIP bo hai góc trên; tab khác `TEXT_MUTED`; hot nền `TAB_HOVER`; khung tiêu điểm khi đi bằng bàn phím); nhãn `TEXT` (nhãn chỉ đọc, ghi chú `TEXT_MUTED`); chữ lỗi `DANGER` thay `0x0000_00B4`; kết quả Kiểm tra kết nối thành công `SUCCESS`, thất bại `DANGER`; ô nhập, ô bí mật, ô bắt phím, hộp chọn theo Interfaces; ô bắt phím đang chờ: gạch dưới `ACCENT`, lời nhắc `TEXT_MUTED`; nút Trợ giúp, Chọn…, Đổi…, Kiểm tra kết nối, Hủy là `Normal`, OK là `Primary`; ô tích, nút chọn, thanh trượt (rãnh `TRACK`, phần đã qua `ACCENT`, núm `TEXT` viền `ACCENT`; mờ khi vô hiệu) tự vẽ; chữ của điều khiển bị vô hiệu `TEXT_DISABLED`.
  - `about.rs`, `help.rs`: nút Đóng thành `Primary` tự vẽ (thay `set_dark_control_theme` của Task 29).
  - Vẽ không nháy: vẽ vào DC tạm rồi chép, hoặc `WS_CLIPCHILDREN` và không xóa nền thừa.
- [ ] **3. Checklist.** Thêm mục "Đợt 4: cửa sổ Tùy chọn tối" vào `docs/testing/manual-checklist.md`, mỗi dòng `[ ]`: so với prototype từng tab ở hai ngôn ngữ, DPI 100% và 150%, và khi kéo cửa sổ sang màn hình DPI khác; không nháy trắng khi mở và khi đổi tab; Tab đi đủ các ô theo thứ tự cũ và luôn thấy ô nào đang có tiêu điểm; Space bật tắt ô tích; mũi tên đổi PNG/JPEG; thanh trượt bằng chuột và phím; chọn PNG thì ba ô chất lượng mờ; Ctrl+Tab, Ctrl+Shift+Tab kể cả khi tiêu điểm ở thanh trượt; Alt+chữ gạch dưới; Enter là OK, Esc là Hủy; bắt phím ở tab Phím tắt; lỗi ở tab Lưu trữ R2 nhảy đúng ô và chữ lỗi đỏ đọc rõ; Kiểm tra kết nối hiện chữ xanh hoặc đỏ; danh sách thả xuống của hộp chọn tối; nút khi trỏ và khi nhấn đổi màu; Trình tường thuật (Narrator) đọc đúng tên ô tích, nút chọn, nút bấm và thanh trượt; mở đóng 20 lần không chậm dần; Thông tin và Trợ giúp có nút Đóng xanh.
- [ ] **4.** `cargo fmt --all`; `powershell -File scripts/check.ps1` in `ALL CHECKS PASSED`. Build release nếu không bị khóa. Commit `feat: dark Options window`.

**Xong khi:** check qua; báo cáo nêu các commit, dòng cuối của check, cách vẽ đã chọn cho từng loại điều khiển và lý do, kiểm thử nào `#[ignore]` (kèm lệnh chạy), hành vi nào chỉ kiểm được bằng tay, và những gì chưa nhìn bằng mắt.
