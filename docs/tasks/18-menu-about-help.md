# Task 18: Menu năm mục, cửa sổ Thông tin và Trợ giúp

**Agent:** `builder` · **Model:** sonnet · **Nhánh:** `task/18-menu-about-help`

Gói task tự chứa. Thuộc kế hoạch `docs/superpowers/plans/2026-11-16-options-tabs-i18n.md`.

## Cần đọc

- `crates/shotme-windows/CLAUDE.md`
- Thiết kế đợt 1 `docs/superpowers/specs/2026-11-16-options-tabs-i18n-design.md` mục 5.1, 5.2 (đoạn nút Trợ giúp), 5.3, 5.4, 5.5, 5.6
- `crates/shotme-windows/src/{tray,main,settings,lang,hotkey}.rs`, `crates/shotme-core/src/i18n.rs`
- `README.md` mục "Chuẩn bị Cloudflare R2" (nguồn cho phần 4 của Trợ giúp)

## Global Constraints

- Mọi chữ người dùng nhìn thấy lấy từ bảng `shotme_core::i18n`, đủ hai ngôn ngữ. `scripts/check.ps1` chặn chuỗi tiếng Việt viết cứng ngoài `i18n.rs`.
- Thông tin và Trợ giúp không có link, không mở trình duyệt, không gọi mạng. Không thêm crate.
- Release dùng `panic = "abort"`: không `unwrap`/`expect` trên đường chạy thật. `unsafe` nhỏ, kèm `// SAFETY:`. Giải phóng mọi handle (cửa sổ, phông) trên mọi đường đi, kể cả khi thoát ứng dụng.
- `gate_step` không đổi. Không sửa logic bắt phím hay Kiểm tra kết nối của cửa sổ Tùy chọn.
- Không làm tính năng của đợt 2, 3.
- Commit kiểm thử trước (`test:`), rồi mã (`feat:`). `cargo fmt --all`. Chạy cargo qua PowerShell. Không chuyển nhánh. Không tắt tiến trình `shotme` không do mình khởi động. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.

## Interfaces

**Consumes:** `shotme_core::i18n::{Lang, Msg, t}` (thêm chuỗi bằng cách thêm mục vào macro bảng, đủ hai bản; chuỗi có tham số là `pub fn ten(lang, ...) -> String`); `lang::{current, tr}`; `hotkey::describe(&Hotkey) -> String`; `settings::show(parent, &Config, notice, reason)`; hàng nút của cửa sổ Tùy chọn có chỗ trống bên trái từ x = 12 DIP.

**Produces:**

```rust
// tray.rs
pub enum MenuCommand { Capture, Options, About, Help, Quit }

// winutil.rs (mới): phần ba cửa sổ dùng chung, chuyển từ settings.rs sang, không đổi hành vi
pub fn system_font(dpi: u32) -> Option<HFONT>;
pub fn center_on_cursor_monitor(hwnd: HWND, width: i32, height: i32);
pub fn set_app_icons(hwnd: HWND);

// about.rs, help.rs (mới)
pub fn show(owner: HWND);                        // about
pub fn show(owner: HWND, hotkey: &Hotkey);       // help
pub fn close();                                  // không có cửa sổ thì không làm gì
pub fn pre_translate(msg: &MSG) -> bool;         // true: thông điệp đã được xử lý, vòng lặp bỏ qua

// i18n.rs
pub fn help_text(lang: Lang, hotkey: &str) -> String;
pub fn about_version(lang: Lang, version: &str) -> String;   // "Phiên bản 0.1.0" | "Version 0.1.0"

// main.rs
pub fn info_windows_stale(old: &Config, new: &Config) -> bool;   // ngôn ngữ hoặc phím tắt đã đổi
```

## Thiết kế đã chốt

**Menu khay**, không có đường kẻ, theo thứ tự:

| Vi | En | Việc |
|---|---|---|
| Chụp màn hình | Take screenshot | như cũ |
| Tùy chọn... | Options... | như mục "Cài đặt" cũ |
| Thông tin... | About... | `about::show(hwnd)` |
| Trợ giúp | Help | `help::show(hwnd, &config.hotkey)` |
| Thoát | Exit | như cũ |

Thông tin và Trợ giúp không đi qua cổng chụp (`gate`). Menu chỉ hiện khi cổng cho phép như hiện nay.

**Hai cửa sổ.** Không chặn: tạo xong là trả về, thông điệp do vòng lặp đang chạy phân phối. Kiểu `WS_CAPTION | WS_SYSMENU`, `WS_EX_APPWINDOW`, không đổi cỡ được, có biểu tượng shotme, hiện giữa màn hình đang có con trỏ. Mỗi loại tối đa một cửa sổ: `show` khi đã mở thì khôi phục nếu đang thu nhỏ và đưa ra trước. Enter và Esc đóng cửa sổ; nút X đóng. Đóng thì hủy cửa sổ và phông.

- **Thông tin / About** (tiêu đề "Thông tin" / "About"): biểu tượng ứng dụng 48×48; "shotme"; `about_version(lang, env!("CARGO_PKG_VERSION"))`; dòng mô tả "Chụp vùng màn hình, lưu vào máy hoặc tải lên kho Cloudflare R2 của chính bạn và chép link." / "Capture a region of the screen, save it locally or upload it to your own Cloudflare R2 bucket and copy the link."; nút "Đóng" / "Close" (mặc định, giữ tiêu điểm).
- **Trợ giúp / Help** (tiêu đề "Trợ giúp" / "Help"): vùng client 520×420 DIP; một ô `EDIT` nhiều dòng, chỉ đọc, có thanh cuộn dọc, chứa `help_text(lang::current(), &hotkey::describe(hotkey))` (xuống dòng bằng `\r\n`); nút "Đóng" / "Close" giữ tiêu điểm ban đầu để chữ không bị bôi chọn.
- `help_text` gồm năm phần theo thiết kế mục 5.5: chụp (nêu đúng tổ hợp phím truyền vào), thanh công cụ và phím (Ctrl+D, Ctrl+S, Ctrl+C, Esc), Tùy chọn (đổi gì ở tab nào), chuẩn bị Cloudflare R2 (rút gọn từ README: tạo kho, bật địa chỉ công khai, tạo khóa quyền Object Read & Write, tạo quy tắc vòng đời cho `d1/`, `d7/`, `d30/`, nhập vào tab Lưu trữ R2, bấm Kiểm tra kết nối), và khi tải lên không được (ảnh luôn được lưu vào thư mục lưu). Văn bản thuần, không có địa chỉ web.

**Bàn phím.** `pre_translate(msg)`: nếu cửa sổ của module đang mở và `msg` thuộc nó hoặc con nó, gọi `IsDialogMessageW`; cửa sổ xử lý `WM_COMMAND` với `IDOK` (1) và `IDCANCEL` (2) bằng cách đóng. Gọi `about::pre_translate(&msg) || help::pre_translate(&msg)` ở đầu thân vòng lặp trong `main()` và trong `settings::run_loop` (trước mọi xử lý khác của vòng lặp đó), để hai cửa sổ dùng được cả khi Tùy chọn đang mở.

**Nút "Trợ giúp" / "Help" trong cửa sổ Tùy chọn.** Nằm bên trái hàng nút (x = 12 DIP), cùng cỡ với nút Hủy, không phải nút mặc định, đứng trước OK trong thứ tự Tab. Bấm thì gọi `help::show(parent, &phím đang hiện trong biểu mẫu)` với `parent` là cửa sổ ẩn của ứng dụng (không phải cửa sổ Tùy chọn), để đóng Tùy chọn không kéo Trợ giúp theo. Đang bắt phím mà bấm nút này: thoát chế độ bắt trước, như bấm ra ngoài.

**Đóng khi chữ cũ đi.** Trong `open_settings`, sau khi lưu cấu hình thành công: nếu `info_windows_stale(&old, &new)` thì `about::close(); help::close();`. Khi thoát ứng dụng (`WM_DESTROY` của cửa sổ ẩn): gọi cả hai `close()`.

## Các bước

- [ ] **Bước 1: kiểm thử.**

```rust
// i18n.rs
#[test] fn help_text_names_the_hotkey_and_keys() {
    for l in Lang::ALL {
        let s = help_text(l, "Ctrl+Shift+Q");
        assert!(s.contains("Ctrl+Shift+Q"), "{l:?}");
        for k in ["Ctrl+D", "Ctrl+S", "Ctrl+C", "Esc", "d1/", "d7/", "d30/", "Object Read & Write"] {
            assert!(s.contains(k), "{l:?} thiếu {k}");
        }
        assert!(!s.contains("http"), "{l:?} có địa chỉ web");
    }
    assert_ne!(help_text(Lang::Vi, "X"), help_text(Lang::En, "X"));
}
#[test] fn about_version_contains_version() {
    for l in Lang::ALL { assert!(about_version(l, "1.2.3").contains("1.2.3")); }
}
// main.rs
#[test] fn info_windows_close_when_language_or_hotkey_changes() {
    let a = Config::defaults(Path::new("C:/p"), Lang::Vi);
    assert!(!info_windows_stale(&a, &a.clone()));
    let mut b = a.clone(); b.language = Lang::En; assert!(info_windows_stale(&a, &b));
    let mut b = a.clone(); b.hotkey.ctrl = true; assert!(info_windows_stale(&a, &b));
    let mut b = a.clone(); b.autostart = true; assert!(!info_windows_stale(&a, &b));
}
```

  Kiểm thử chạy tay `#[ignore]`, mỗi cửa sổ một cái (`about_window_opens_and_closes`, `help_window_opens_and_closes`): `show`, tìm cửa sổ theo tên lớp, gọi `show` lần hai và xác nhận vẫn chỉ một cửa sổ, `close`, xác nhận không còn cửa sổ; chạy vòng lặp ba lần và xác nhận số đối tượng GDI của tiến trình không tăng dần.

- [ ] **Bước 2:** kiểm thử mới thất bại. Commit `test: help text, about version, info window staleness`.

- [ ] **Bước 3: `winutil.rs`.** Chuyển `system_font`, đoạn căn giữa theo màn hình có con trỏ, và đoạn đặt biểu tượng từ `settings.rs` sang; `settings.rs` gọi lại chúng. Không đổi hành vi. `cargo test -p shotme-windows` qua.

- [ ] **Bước 4: chuỗi và hàm thuần.** `help_text`, `about_version`, các mục `Msg` mới, `info_windows_stale`.

- [ ] **Bước 5: `about.rs`, `help.rs`.** Theo "Thiết kế đã chốt".

- [ ] **Bước 6: nối.** `tray.rs` (năm mục, `MenuCommand` mới), `main.rs` (xử lý `About`, `Help`; `pre_translate` trong vòng lặp; đóng khi lưu và khi thoát), `settings.rs` (nút Trợ giúp; `pre_translate` trong `run_loop`).

- [ ] **Bước 7: chạy tay.** `cargo test -p shotme-windows -- --ignored about_window help_window settings_window`, ghi kết quả vào báo cáo.

- [ ] **Bước 8: checklist.** Thêm vào `docs/testing/manual-checklist.md` mục "Đợt 1: menu, Thông tin, Trợ giúp": menu đủ năm mục đúng thứ tự ở cả hai ngôn ngữ; Thông tin hiện đúng phiên bản; chọn lại mục khi cửa sổ đang mở chỉ đưa nó ra trước; Enter, Esc, nút X đóng được; Trợ giúp cuộn được, không bôi chọn sẵn, nêu đúng phím tắt đang đặt; lúc Thông tin hoặc Trợ giúp đang mở: phím tắt chụp, menu khay và Tùy chọn vẫn dùng được; nút Trợ giúp trong Tùy chọn mở Trợ giúp, gõ tiếp vào Tùy chọn được, đóng Tùy chọn thì Trợ giúp còn; đổi ngôn ngữ hoặc phím tắt rồi OK thì hai cửa sổ tự đóng; Thoát khi hai cửa sổ đang mở; đọc lại toàn bộ lời văn Trợ giúp ở cả hai ngôn ngữ.

- [ ] **Bước 9:** `powershell -File scripts/check.ps1` in `ALL CHECKS PASSED`. Commit `feat: five-item tray menu, About and Help windows`.

**Xong khi:** check qua; ba kiểm thử chạy tay đã chạy; báo cáo dán toàn văn `help_text` ở cả hai ngôn ngữ để chủ dự án đọc.
