# Kiến trúc và quy ước

Nguồn đầy đủ: spec mục 10, kế hoạch mục "Cấu trúc file" và các khối Interfaces trong gói task.

## Hai lớp

```
crates/darkshot-core      thư viện, KHÔNG phụ thuộc hệ điều hành
crates/darkshot-windows   file chạy darkshot.exe, hiện thực các trait bằng Win32
```

Thêm macOS hoặc Linux sau này là thêm một crate nền tảng mới, không sửa core.

## Core chứa gì

| Module | Trách nhiệm |
|---|---|
| `geometry` | `Point`, `Rect`, `Selection` (máy trạng thái kéo thả), `place_toolbar`, `monitor_containing` |
| `image` | `Image` (BGRA, từ trên xuống), `crop`, `encode_png` |
| `naming` | tên file local, tên ngẫu nhiên trên kho, đường dẫn không trùng |
| `config` | `Config`, `Expiry`, `Hotkey`, `R2Config`, `load`, `save`, chuẩn hóa địa chỉ công khai |
| `sigv4` | chữ ký AWS Signature V4 |
| `upload` | trait `Uploader`, `R2Uploader`, `test_connection` |
| `actions` | `Action`, `run_action`, trait `Clipboard`, `Notifier` |

## Trait mà nền tảng phải hiện thực

- `config::SecretStore` (Windows: DPAPI)
- `actions::Clipboard`
- `actions::Notifier` (Windows: thông báo của biểu tượng khay)
- `upload::Uploader` đã có hiện thực chung `R2Uploader`; để ngỏ cho nơi chứa khác.

Mọi trait có ràng buộc `Send + Sync`.

## Mẫu phải theo

- Logic nằm ở core và có kiểm thử tự động; lớp nền tảng chỉ vẽ, nhận sự kiện và gọi core.
- Lớp phủ dùng `geometry::Selection` cho mọi trạng thái kéo thả, không tự tính lại.
- Mọi hành động sau khi chọn vùng đi qua `actions::run_action`.
- Thời gian máy do nền tảng cung cấp qua `naming::LocalTime`; core không gọi API thời gian của hệ điều hành.
- Tải lên chạy ở luồng nền, mỗi lần một luồng. Thông báo từ luồng nền được chuyển về luồng chính.
- Lỗi hiển thị cho người dùng bằng tiếng Việt và không bao giờ chứa khóa bí mật.
- Viết kiểm thử trước, rồi mới viết mã.

## Mẫu mới phát sinh

- Không panic trên đường chạy thật: bản release dùng `panic = "abort"`, một panic sẽ tắt cả ứng dụng khay. Dùng `Result`, và với ảnh thì kiểm `Image::is_valid()` trước.
- Ghi file cho chắc: ghi vào file tạm cùng thư mục, rồi đổi tên; dọn file tạm khi lỗi (`config::save`, lưu PNG trong `actions`).
- Đầu vào do người dùng nhập mà đi vào URL (account id, tên kho, địa chỉ công khai) phải được kiểm ký tự trước; sai thì coi như chưa cấu hình và không gửi gì.

## Mẫu của đợt 1 (chuỗi, cửa sổ tab, cửa sổ không chặn)

### Bảng chuỗi

- Ở `crates/darkshot-core/src/i18n.rs`. Macro `messages!` sinh `enum Msg` và hàm `t(lang, msg)`. Mỗi dòng có cặp (bản Việt, bản English).
- Thêm một chuỗi: thêm một dòng vào `messages!` với đủ hai bản. `t` là `match` trên cặp `(Lang, Msg)`, nên thiếu một bản là lỗi biên dịch.
- Chuỗi có tham số: viết hàm riêng nhận `lang: Lang` và `match` theo `Lang` (ví dụ `help_text`, `upload_failed_body`). `OK_LABEL` nằm ngoài bảng vì "OK" giống nhau ở hai ngôn ngữ. Tên riêng "Account ID", "Access Key ID", "Secret Access Key" viết thẳng trong `settings.rs` (`build_r2`), không dịch.
- Core nhận `Lang` qua tham số, không tự biết ngôn ngữ hiện hành.
- Lớp Windows giữ ngôn ngữ hiện hành trong `crates/darkshot-windows/src/lang.rs` (biến nguyên tử, đọc được từ mọi luồng). Lấy chuỗi bằng `lang::tr(Msg)` hoặc `lang::tr_code`. Khởi động đặt bằng `lang::set`; sau khi lưu Tùy chọn thành công, `main.rs` gọi `lang::set` lần nữa. Lưu thất bại thì giữ ngôn ngữ cũ.
- `lang::os_default()` đọc ngôn ngữ giao diện Windows, chỉ dùng khi chưa có file cấu hình hoặc file hỏng.

### Cửa sổ dạng tab (Tùy chọn)

- Trong `settings.rs`: dải tab `SysTabControl32` (`WC_TABCONTROLW`) và ba khung con `ID_FRAME0 + i`. Mỗi tab là một nhóm điều khiển dựng trong `build_general`, `build_hotkeys`, `build_r2`.
- Khung không hiện thì bị ẩn và `EnableWindow(false)`, để không nhận Tab. Thứ tự Z của các điều khiển chính là thứ tự Tab (chú thích ở `build_controls`).
- `WM_COMMAND` từ nút trong khung được `frame_proc` chuyển lên cửa sổ chính.
- Hàm thuần có kiểm thử: `tab_of(Field)`, `initial_tab(OpenReason)`, `key_route(...)`. `apply_form` trả `FormError { field, message }`; cửa sổ chuyển đến tab chứa ô sai.
- `show` chạy vòng `GetMessageW` riêng và gọi `about::pre_translate` và `help::pre_translate` trong vòng đó.

### Cửa sổ không chặn (Thông tin, Trợ giúp)

- Hàm công khai: `show(owner, ...)`, `close()`, `pre_translate(&MSG) -> bool`. Mỗi cửa sổ tối đa một bản; HWND lưu trong `thread_local`. Dọn tài nguyên ở `WM_NCDESTROY`.
- `winutil::dialog_pre_translate` lo Enter và Esc (đóng cửa sổ) và `IsDialogMessageW` lo Tab.
- Phải gọi `pre_translate` ở mọi vòng lặp thông điệp, trừ lớp phủ (lớp phủ có vòng riêng). Hiện có hai nơi gọi: `main.rs` (vòng chính) và `settings.rs` (vòng của Tùy chọn). Thiếu một nơi thì phím Tab và Enter không hoạt động trong cửa sổ đó.

### Thêm một ô hoặc một tab (đợt 2, 3, 4)

- Bề rộng lấy từ `Layout` (`settings.rs`), tính một lần trong `build_controls` từ nhãn dài nhất của dòng phím tắt và lưu ở `Builder.layout`. Ô mới dùng `b.layout.client_w` và `b.layout.ew`, không ghi số cứng.
- Ô trong tab có sẵn (`settings.rs`):
  1. Thêm `const ID_...` (dãy đang dùng: 101 đến 119 cho ô, nút và nút chọn; khung tab là 120 đến 123; dòng phím tắt là `ID_HK_CHANGE0 + i` (130 đến 132) và `ID_HK_CHECK0 + i` (133 đến 135); thanh trượt và ô số của tab Định dạng là 140 và 141). Tạo điều khiển trong `build_general` (trả `GeneralCtl`), `build_hotkeys` (trả `HotkeysCtl`), `build_format` (trả `FormatCtl`) hoặc `build_r2` (trả `R2Ctl`) bằng `row_edit`, `combo` hoặc `b.button`. Ô tích là `b.button` với `BS_AUTOCHECKBOX`, như `check` trong `build_general`.
  2. Thêm HWND vào `struct Ctl` và gán trong `build_controls`: trong chỗ dựng `let ctl = Ctl { ... }`, và thêm vào mảng `all` (kiểm HWND rỗng) phía trên. Lưu ý: các ô của dòng phím tắt không nằm trong `all`; `build_controls` kiểm chúng riêng qua `hk.chk`, `hk.edit` và `hk.btn`.
  3. Điền giá trị ban đầu trong `show` (ô nhập bằng `set_text(c.account, ...)`, ô tích bằng `set_checked`), và đọc giá trị trong `read_form` vào trường mới của `Form` (ô tích bằng `is_checked`).
  4. Kiểm trong `apply_form`; trường cấu hình thêm vào `Config` ở core, rồi vào `FileConfig` và hai chiều trong `config::load`/`save`.
  5. Nếu ô có thể sai: thêm biến thể vào `Field`, vào `tab_of` và `field_ctl`. Nếu ô đứng đầu tab (theo thứ tự Tab) thì sửa `first_ctl`.
- Dòng phím tắt mới, ở core (`crates/darkshot-core/src/config.rs`): thêm biến thể vào `HotkeyRole`, rồi sửa `HotkeyRole::ALL` (`[HotkeyRole; 3]`), `label()` (trỏ tới một `Msg` nhãn mới trong `i18n.rs`, có đủ hai ngôn ngữ), `default_slot`, `Hotkeys` (trường mới, `Default`), `get`, `set` và `first_conflict`. Thêm trường của dòng vào `FileConfig` và ánh xạ hai chiều trong `config::load`/`save`.
- Trong `crates/darkshot-core/src/i18n.rs`: thêm trường vào `HelpKeys` và nhánh tương ứng trong `help_text`.
- Trong `crates/darkshot-windows/src/hotkey.rs`: thêm nhánh vào `id_of` và trường trong `help_keys`. `role_of` duyệt `HotkeyRole::ALL` nên không cần sửa. Mã đăng ký phải khác các mã đang có (1 đến 3).
- Trong `settings.rs`: `HotkeyRole::ALL` được duyệt trong `show`, `read_form`, `build_hotkeys` và `save`, nên các vòng này tự nhận dòng mới. Độ dài các mảng `hk_chk`, `hk_edit`, `hk_btn` (`[HWND; 3]`) trong `struct Ctl`, và `struct HotkeysCtl` (`chk`, `edit`, `btn`, cùng độ dài 3) phải đổi cùng nhau.
- ID của dòng là dãy liền nhau: `ID_HK_CHANGE0 + i` (130 đến 132) và `ID_HK_CHECK0 + i` (133 đến 135). Khi số dòng tăng, dãy nút "Đổi…" sẽ đè lên ô tích, nên phải dời `ID_HK_CHECK0` (hoặc đổi cả hai) trước khi thêm dòng.
- Nếu dòng là loại chụp mới: thêm vào `CaptureKind` và `kind_of` trong `crates/darkshot-windows/src/main.rs`, rồi thêm nhánh trong `capture_flow`.
- Tab mới (`settings.rs`):
  1. Thêm biến thể vào `Tab`, rồi `Tab::ALL` (hiện `[Tab; 4]`: Thông thường, Phím tắt, Định dạng, Lưu trữ R2; đổi độ dài), `index` và `label` (kèm `Msg` mới).
  2. Đổi độ dài mảng `frames: [HWND; 4]` trong `struct Ctl`. Trong `build_controls`, thêm `b.frame(ID_FRAME0 + n)` vào mảng `frames`, thêm `frames[n]` vào mảng `all`, và đặt `b.parent = frames[Tab::X.index()]` trước khi gọi `build_*`.
  3. `show_frame` duyệt mảng `frames` để ẩn và tắt các khung còn lại, nên không cần sửa.
  4. Thêm hàm `build_*` cho tab, và nhánh trong `first_ctl`. Trình biên dịch báo các `match` còn thiếu nhánh.
- Nút chọn (đợt 3, `build_format`, `fmt_png` và `fmt_jpeg`): tạo bằng `BS_AUTORADIOBUTTON`. Nút đầu của nhóm dựng bằng `b.button` nên có `WS_GROUP | WS_TABSTOP`; nút sau trong cùng nhóm dựng bằng `b.make` không có hai cờ này. Ô kế sau có `WS_GROUP` để khép nhóm. Chỉ nút đang được chọn có điểm dừng Tab: `first_ctl` trả đúng nút đó. Đọc và đặt bằng `is_checked` và `set_checked`.
- Thanh trượt (đợt 3, `TRACKBAR_CLASSW`, trường `Ctl.quality`): cần lớp `ICC_BAR_CLASSES` trong `InitCommonControlsEx`, cùng `ICC_TAB_CLASSES`. Đặt khoảng bằng `TBM_SETRANGE` (wParam 0, lParam = max << 16 | min), rồi `TBM_SETLINESIZE`, `TBM_SETPAGESIZE`, `TBM_SETPOS`. Đọc bằng `TBM_GETPOS`, hằng tự định nghĩa vì crate `windows` không có. Thanh trượt gửi `WM_HSCROLL` cho khung cha; `frame_proc` chuyển lên cửa sổ chính, và `wndproc` nhận HWND thanh trượt trong `lp`. Ô số cập nhật trong nhánh `WM_HSCROLL` đó.
- Làm mờ (đợt 3, `apply_quality_enabled`): nhãn, thanh trượt và ô số gọi `EnableWindow` theo `quality_enabled(format)`. Gọi khi mở cửa sổ và khi đổi nút chọn. Điều khiển mờ không nhận Tab, và giá trị vẫn được đọc khi lưu.
- Tô màu (đợt 4): ô mới không dùng màu mặc định của Windows. Làm theo mục "Giao diện tối (đợt 4)" dưới đây; mỗi loại điều khiển có một cách vẽ đã chọn.

## Giao diện tối (đợt 4)

- Hằng màu ở `darkshot_core::theme` (`crates/darkshot-core/src/theme.rs`): chỉ là số (`Rgb`, `Rgb::hex(0xRRGGBB)`), không phụ thuộc Windows. `contrast_ratio` tính theo WCAG 2.x.
- Mọi cặp chữ và nền mới thêm vào `TEXT_PAIRS` (phải đạt 4,5:1). Thành phần giao diện (viền, tiêu điểm, ô tích) thêm vào `UI_PAIRS` (đạt 3:1). Kiểm thử của core đọc hai bảng này nên cặp không đạt sẽ làm hỏng `cargo test`.
- `crates/darkshot-windows/src/theme.rs` (lớp Windows) là chỗ chung của mọi cửa sổ:
  - `apply_dark_title_bar(hwnd)`: gọi cho mọi cửa sổ cấp cao trước khi hiện, để không nhá trắng.
  - `enable_dark_menus()`: gọi một lần lúc khởi động (menu khay tối, chế độ không công khai của `uxtheme`).
  - `set_dark_control_theme(hwnd, DarkClass::Explorer | Cfd)`: thanh cuộn và danh sách thả xuống của hộp chọn.
  - `brush(c)`: chổi dùng chung, tạo lười, cache trong `static` và sống suốt tiến trình. Không tạo chổi mới trong `WM_CTLCOLOR*`.
  - `ctl_color(hdc, fg, bg)`: xử lý `WM_CTLCOLOR*`, đặt màu chữ và nền rồi trả chổi nền.
- Cách vẽ đã chọn ở `settings.rs` (báo cáo Task 30):
  - Nút bấm, ô tích, nút chọn và thanh trượt vẽ bằng `NM_CUSTOMDRAW` (`custom_draw_button`, `custom_draw_trackbar` trong `theme.rs`), giữ kiểu điều khiển gốc. Không dùng `BS_OWNERDRAW`, để giữ Tab, phím gạch dưới và tên đọc được.
  - Dải tab, hộp chọn và nhãn bị vô hiệu: subclass `WM_PAINT` (`SetWindowSubclass`, `SUBCLASS_ID` = 0xDA5, gỡ ở `WM_NCDESTROY`).
  - Ô nhập: subclass `WM_NCCALCSIZE` và `WM_NCPAINT` (viền 2 DIP, gạch dưới ACCENT khi có tiêu điểm).
  - Ô `ES_PASSWORD` không bao giờ tự vẽ chữ.
- Kích thước vẽ tay (viền, bo góc, thanh trượt, ô tích) tính theo DPI của cửa sổ.
- GDI: chổi dùng chung sống suốt tiến trình; mọi handle khác (bút, phông, DC) giải phóng trên mọi đường đi. Kiểm thử `*_no_gdi_leak` đếm đối tượng GDI sau khi mở đóng cửa sổ.

## Mẫu của đợt 3 (định dạng tải lên)

- Đường tải lên: `Config.upload_format` và `jpeg_quality` → `main.rs` (`capture_flow`) chép vào `UploadJob` từ bản sao `Config` của lượt đó → `app::spawn_upload` đưa vào `actions::Ctx` → `actions::upload` gọi `image::encode_for_upload(img, format, quality)`. Khóa đi qua `upload::object_key(expiry, name, format)`, content type lấy từ `format.content_type()`.
- Lỗi nén (kể cả ảnh có cạnh quá 65.535 pixel) đi vào `fallback`: lưu PNG và báo lỗi, không tự tải PNG.
- Mọi đường lưu vào máy dùng `save_png`: `Action::Save`, lưu dự phòng trong `fallback`, và `app::save_pending` (đi qua `Action::Save`). Các `Ctx` lưu đặt `upload_format: Png`.
- `encode_jpeg` đặt 4:4:4 tường minh (`set_sampling_factor(R_4_4_4)`). Crate `jpeg-encoder` 0.7.1 tự chọn 4:2:0 khi chất lượng dưới 90, nên không được bỏ dòng này.

## Mẫu của đợt 2 (phím tắt, chụp, thông báo)

- **Phím tắt:** nguồn là `config::Hotkeys` (ba `HotkeySlot`, mỗi slot có `enabled` và `key`). `hotkey::id_of` cho mã đăng ký (Region 1, SaveFull 2, UploadFull 3); `role_of(id)` đổi ngược khi nhận `WM_HOTKEY`. `hotkey::register_enabled` luôn hủy cả ba trước rồi đăng ký dòng đang bật, và trả về các dòng không đăng ký được (vì `RegisterHotKey` không thay phím cũ). Dùng khi khởi động (`main.rs`, qua `register_hotkeys_or_notify`), khi lưu lỗi (`main.rs`) và khi Hủy (`settings.rs`, `cancel`). Khi OK, `settings.rs` (`save`) chỉ đăng ký lại khi `s.hk_unregistered` (đã hủy một dòng trong phiên mở này, qua "Đổi…" hoặc một lần OK lỗi) hoặc khi `cfg.hotkeys` khác `s.current.hotkeys` (cấu hình lúc mở cửa sổ). Khi đó nó gọi `hotkey::unregister_all` rồi `hotkey::register` cho từng dòng đang bật. `help_keys` và `taken_combos` lấy tổ hợp cho Trợ giúp và thông báo.
- **Cổng chụp:** `main::request(hwnd, ev, requested)` là lối vào duy nhất. Nó chạy `gate_step` (hàm thuần, có kiểm thử) và đặt loại chụp `CAPTURE_KIND` bằng `next_kind`, chỉ đổi khi yêu cầu được nhận (`StartTimer` hoặc `CaptureNow`). `capture_flow` đọc loại từ `CAPTURE_KIND`. Không gọi `capture_flow` từ chỗ khác.
- **Lọc thông báo:** chỗ duy nhất là nhánh `WM_NOTICE` trong `wndproc` (`main.rs`), gọi `actions::notice_visible(kind, show_success_notices)`. Mọi thông báo của `TrayNotifier` đi qua `WM_NOTICE`, và hàng đợi `pending` chỉ nhận thông báo đã qua bước lọc. Thông báo `Success` bị ẩn theo ô tích; lỗi và thông tin luôn hiện.
- **Con trỏ:** `capture::capture_virtual_screen(include_cursor)` vẽ con trỏ bằng `draw_cursor` (`DrawIconEx`) vào DIB khi DIB còn được chọn trong DC bộ nhớ, rồi `GdiFlush`. `Screenshot.cursor` giữ vị trí con trỏ theo tọa độ màn hình, dùng cho "toàn màn hình" (`full_screen_rect`). Lớp phủ hiển thị chính ảnh này nên con trỏ đứng yên như lúc chụp.
- **Khung nhớ:** `LAST_SELECTION` (thread-local, `Cell<Option<Rect>>`) lưu khung theo tọa độ màn hình, chỉ trong bộ nhớ. `remember_after` nhớ khung khi Lưu, Chép, Tải lên và giữ khung cũ khi Hủy. Khi ghi, `capture::to_screen_coords` đổi khung ảnh sang tọa độ màn hình trước khi nhớ. `geometry::restore_selection` nhận khung tọa độ màn hình và tự trừ `origin`, nên danh sách màn hình truyền vào cũng phải là tọa độ màn hình. `shot.monitors` ở tọa độ ảnh, phải đổi bằng `to_screen_coords` trước (đã làm trong `capture_flow`). Truyền thẳng là lệch khi gốc âm.
