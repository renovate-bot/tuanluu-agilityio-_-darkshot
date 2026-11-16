# Task 25: Tab "Định dạng"

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/25-format-tab` · **Rà soát:** `specialist`

Gói task tự chứa. Thuộc kế hoạch `docs/superpowers/plans/2026-11-16-upload-format.md`. Chạy sau Task 24.

## Cần đọc

- `crates/shotme-windows/CLAUDE.md`
- Thiết kế đợt 3 `docs/superpowers/specs/2026-11-16-upload-format-design.md` mục 5.1, 5.7, 5.8
- `memory-bank/systemPatterns.md` mục "Thêm một ô hoặc một tab"
- `crates/shotme-windows/src/settings.rs` (toàn bộ), `crates/shotme-core/src/{config,image,i18n}.rs`
- `docs/testing/manual-checklist.md` (cách viết các mục "Đợt 2")

## Global Constraints

- Không thêm crate. Có thể bật thêm feature của crate `windows` nếu thiếu (`Win32_UI_Controls`, `Win32_Graphics_GdiPlus` đã bật).
- Mọi chữ người dùng thấy lấy từ `shotme_core::i18n`, đủ hai ngôn ngữ; không viết cứng. Không dịch "PNG", "JPEG".
- Hành vi đã rà soát của cửa sổ Tùy chọn giữ nguyên: chuyển tab bằng chuột, Ctrl+Tab, Ctrl+Shift+Tab (không thêm phím chuyển tab mới); máy trạng thái bắt phím; lỗi nhảy đúng tab và đúng ô; OK, Hủy, Esc, Enter; Kiểm tra kết nối chạy nền; đăng ký lại phím tắt.
- Thứ tự tab: Thông thường, Phím tắt, Định dạng, Lưu trữ R2.
- Cài đặt chỉ có tác dụng sau OK; Hủy thì bỏ. Tab mới không sinh lỗi kiểm tra.
- Release dùng `panic = "abort"`: không `unwrap`/`expect`/chỉ số có thể panic trên đường chạy thật. `unsafe` nhỏ, kèm `// SAFETY:`. Giải phóng mọi handle GDI và GDI+ trên mọi đường đi.
- Mọi kích thước tính theo DPI của cửa sổ như các ô hiện có.
- Kiểm thử mới không khẳng định chữ đọc từ `lang::tr`; dùng `t(Lang::Vi, …)`.
- Commit kiểm thử trước (`test:`, thân hàm mới `todo!()`), rồi mã (`feat:`). `cargo fmt --all`. Chạy cargo qua PowerShell. Kiểm `git branch --show-current` trước khi commit. Không chuyển nhánh. Không tắt tiến trình `shotme` không do mình khởi động; nếu exe bị khóa thì bỏ build release và báo lại. Không đẩy lên remote. Không đọc hay ghi khóa R2, không đọc file cấu hình thật trong `%APPDATA%`. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.

## Interfaces

**Consumes (Task 24):** `config::{UploadFormat, JPEG_QUALITY_MIN, JPEG_QUALITY_MAX, JPEG_QUALITY_DEFAULT, clamp_jpeg_quality}`, `Config.{upload_format, jpeg_quality}`, `image::encode_jpeg`, `Msg::{TabFormat, LabelUploadFormat, RadioPng, RadioJpeg, LabelJpegQuality, NoteSavedAlwaysPng}`, `UploadFormat::label()`.

**Produces:**

- `settings.rs`: `Tab::Format` đứng thứ ba trong `Tab::ALL` (độ dài 4); khung thứ tư trong `frames`; `Form` thêm `upload_format: UploadFormat` và `jpeg_quality: u8`; nhóm ô mới (ví dụ `FormatCtl`): nhãn nhóm, hai nút radio (`BS_AUTORADIOBUTTON`, nút đầu có `WS_GROUP | WS_TABSTOP`), nhãn chất lượng, thanh trượt `msctls_trackbar32` (`TBS_HORZ`, khoảng 50–100, bước dòng 1, bước trang 10, `WS_TABSTOP`), ô chữ hiện số, dòng ghi chú. ID ô mới chọn trong khoảng còn trống, không trùng 101–117, 120–122, 130–135.
- Hàm thuần `fn quality_enabled(format: UploadFormat) -> bool` (đúng khi `Jpeg`) dùng cho việc làm mờ ba ô chất lượng.
- Hàm thuần đo dải tab, ví dụ `fn tabs_fit(tab_widths: &[i32], client_w: i32) -> bool` hoặc phép tính bề rộng tối thiểu đưa vào `Layout`, để cửa sổ rộng ra khi bốn tab không vừa một hàng.
- `i18n::help_text`: thêm một đoạn về tab Định dạng (dùng `t(lang, Msg::TabFormat)` cho tên tab): chỉ áp dụng cho ảnh tải lên; ảnh lưu vào máy luôn là PNG; chất lượng cao hơn thì file nặng hơn. Không gọi mức 100 là "nguyên gốc".

## Các bước

- [ ] **1. Kiểm thử trước.** Commit `test: format tab`.
  - `tab_order_has_four_tabs`: `Tab::ALL == [General, Hotkeys, Format, R2]`; đi tới bằng bước +1 qua đủ bốn tab rồi quay về đầu, bước lùi tương tự. Sửa kiểm thử cũ về chỉ số của `Tab::R2` (nay là 3).
  - `form_reads_and_writes_format`: `Config` với `Jpeg`, 75 → form → `Config` cho lại `Jpeg`, 75.
  - `quality_survives_png_selection`: form có `Png` và chất lượng 70 → `Config` lưu ra có `Png` và 70.
  - `quality_enabled_only_for_jpeg`.
  - Kiểm thử của hàm đo dải tab: tổng bề rộng lớn hơn bề rộng client thì không vừa; bằng hoặc nhỏ hơn thì vừa.
  - `help_text_mentions_format_tab` (core): ở cả hai ngôn ngữ, lời Trợ giúp chứa `t(lang, Msg::TabFormat)` và chứa "PNG" và "JPEG".
  - `jpeg_decodes_with_gdiplus` (lớp Windows): tạo ảnh 64×48 gồm bốn ô màu phẳng (đỏ, lục, lam, xám), `encode_jpeg(img, 90)`, giải nén bằng GDI+ từ bộ nhớ (`SHCreateMemStream` hoặc `CreateStreamOnHGlobal` + `GdipCreateBitmapFromStream`), khẳng định rộng 64, cao 48 và màu tại tâm mỗi ô lệch không quá 6 trên mỗi kênh so với ảnh gốc. Lặp lại với chất lượng 50 và 100 (sai số 12 ở mức 50). Chạy thường nếu ổn định; nếu không thì `#[ignore]` kèm lý do và báo lệnh chạy. Giải phóng bitmap, stream và tắt GDI+ trên mọi đường đi.
- [ ] **2. Mã.**
  - Khởi tạo thêm `ICC_BAR_CLASSES` cạnh `ICC_TAB_CLASSES`.
  - Dựng tab mới theo mục "Thêm một ô hoặc một tab": khung, ô, mảng `all`, `first_ctl`, `tab_of`, ẩn/hiện theo tab.
  - Điền form: radio theo `upload_format`; thanh trượt `TBM_SETRANGE` 50–100, `TBM_SETPOS` theo `jpeg_quality`; ô số.
  - `WM_HSCROLL` từ thanh trượt: đọc `TBM_GETPOS`, cập nhật ô số ngay khi kéo và khi dùng phím.
  - Bấm radio: gọi `EnableWindow` cho nhãn chất lượng, thanh trượt và ô số theo `quality_enabled`. Khi mở cửa sổ cũng áp dụng.
  - Đọc form khi OK: `upload_format` từ radio; `jpeg_quality` = `clamp_jpeg_quality(TBM_GETPOS)`, đọc cả khi đang chọn PNG.
  - Bề rộng cửa sổ: đo bốn nhãn tab ở cả hai ngôn ngữ (như cách `Layout` đo nhãn phím tắt) và nới `client_w` nếu dải tab không vừa một hàng; không dùng `TCS_MULTILINE`.
  - Ctrl+Tab và Ctrl+Shift+Tab phải chuyển tab khi tiêu điểm ở thanh trượt hoặc ở nút radio (đi qua `key_route` hiện có; kiểm bằng cách đọc mã và ghi vào checklist).
  - `info_windows_stale` và mọi chỗ so sánh `Config`: xét xem có cần thêm hai trường mới không (lời Trợ giúp mới không phụ thuộc cài đặt, nên có thể không cần); ghi kết luận vào báo cáo.
  - Đoạn Trợ giúp trong `i18n.rs`.
- [ ] **3. Checklist.** Thêm mục "Đợt 3: định dạng" vào `docs/testing/manual-checklist.md`, mỗi dòng `[ ]`: tab hiện đúng ở hai ngôn ngữ, ở DPI 100% và 150%, bốn tab trên một hàng; thanh trượt bằng chuột, mũi tên, Page Up/Down, Home/End và số đổi theo; chọn PNG thì ba ô chất lượng mờ, chọn lại JPEG thì sáng với giá trị cũ; OK rồi mở lại giữ đúng; Hủy thì bỏ; Ctrl+Tab và Ctrl+Shift+Tab đi đủ bốn tab kể cả khi tiêu điểm ở thanh trượt; bắt phím ở tab Phím tắt vẫn như trước; lỗi ở tab Lưu trữ R2 vẫn nhảy đúng tab; tải lên JPEG bằng Ctrl+D và bằng phím tải lên nhanh, link kết thúc `.jpg`, mở được trong trình duyệt và trong ứng dụng Ảnh của Windows; so chữ nhỏ có màu ở 50, 90, 100; so dung lượng với PNG cùng vùng; rút mạng rồi tải lên JPEG thì file lưu là `.png`; Ctrl+S khi đang chọn JPEG vẫn ra `.png`; nâng cấp với file cấu hình cũ vẫn tải lên PNG; trình đọc màn hình đọc tên thanh trượt.
- [ ] **4.** `cargo fmt --all`; `powershell -File scripts/check.ps1` in `ALL CHECKS PASSED`. Commit `feat: Format tab (PNG or JPEG upload, JPEG quality)`.

**Xong khi:** check qua; báo cáo nêu các commit, dòng cuối của check, bề rộng client mới (DIP) ở hai ngôn ngữ nếu đổi, ID các ô mới, kiểm thử GDI+ chạy thường hay `#[ignore]`, kết luận về `info_windows_stale`, và những gì chưa nhìn bằng mắt trên màn hình thật.
