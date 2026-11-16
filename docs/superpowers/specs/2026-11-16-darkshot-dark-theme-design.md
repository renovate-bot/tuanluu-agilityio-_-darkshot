# Thiết kế đợt 4: đổi tên thành darkshot, giao diện luôn tối

Ngày: 2026-11-16. Trạng thái: chờ chủ dự án duyệt. Đã qua một lượt đọc soát (general-purpose).
Prototype đã xem: `.superpowers/brainstorm/1649-1794846769/content/darkshot-theme.html` (không đưa lên git).

## 1. Ý định

Chủ dự án thấy tên "shotme" không phù hợp. Họ muốn phát hành và chia sẻ ứng dụng cho người khác, và muốn mắt dễ chịu khi dùng hằng ngày. Vì vậy ứng dụng đổi tên thành **darkshot** ở mọi nơi, và giao diện **luôn tối** (không theo chế độ sáng/tối của Windows). Độ tương phản, màu sắc, bố cục và phông chữ phải hài hòa.

## 2. Mục tiêu, ngoài phạm vi, điều không được xảy ra

Mục tiêu:
- Người dùng không còn thấy chữ "shotme" ở bất kỳ đâu trong ứng dụng, tên file, thư mục, registry hay tài liệu đang dùng.
- Người đang dùng shotme chạy darkshot lần đầu vẫn giữ cấu hình, khóa R2 và thiết lập khởi động cùng Windows, không phải nhập lại gì.
- Mọi cửa sổ của ứng dụng (Tùy chọn, Thông tin, Trợ giúp), menu khay và lớp phủ chọn vùng đều tối, giống prototype đã duyệt.
- Mọi chữ đạt tối thiểu 4,5:1 so với nền ngay sau nó (WCAG AA).

Ngoài phạm vi:
- Bộ cài, ký số exe, đổi số phiên bản, đưa repo thành công khai.
- Thông báo ở khay (Windows vẽ, theo chế độ của Windows) và hộp chọn thư mục (`IFileOpenDialog`, theo chế độ của Windows).
- Đổi bố cục, thứ tự ô, chữ hay hành vi của bất kỳ cửa sổ nào ngoài phần nhìn.
- Biểu tượng ứng dụng: giữ nguyên biểu tượng xanh hiện tại (chủ dự án chọn phương án A). Chỉ đổi tên file `shotme.ico` thành `darkshot.ico`.
- Sửa các spec và kế hoạch đã duyệt: chúng giữ tên cũ (luật dự án). Việc đổi tên ghi vào `memory-bank/decisions.md`.

Điều không được xảy ra (dù chạy được vẫn coi là hỏng):
- Mất khóa R2 hoặc cấu hình của người đang dùng.
- Khởi động cùng Windows chạy cả shotme cũ lẫn darkshot, hoặc không chạy cái nào dù người dùng đã bật.
- Chữ tối trên nền tối ở bất kỳ ô nào (ví dụ chữ của ô tích còn đen).
- Mất khả năng dùng bàn phím, thứ tự Tab, phím tắt gạch dưới hay trình đọc màn hình đã có.
- Ảnh chụp ra bị đổi màu: chỉ phần giao diện vẽ bên trên đổi.
- Rò rỉ GDI (bút, chổi, phông) khi mở đóng cửa sổ nhiều lần.

## 3. Ràng buộc

- Rust thuần, crate `windows` 0.62.2; không thêm crate mới (ngoại lệ duy nhất vẫn là `jpeg-encoder`). Có thể bật thêm feature của crate `windows` (ví dụ `Win32_Graphics_Dwm`, `Win32_UI_Accessibility` nếu cần).
- Mọi chữ người dùng thấy lấy từ `shotme_core::i18n` (sau đổi tên là `darkshot_core::i18n`).
- Hỗ trợ Windows 10 và 11. Thanh tiêu đề tối trên Windows 10 trước bản 20H1 không có: chấp nhận thanh tiêu đề sáng ở đó.
- Hỗ trợ DPI theo màn hình như hiện nay: mọi kích thước vẽ tay tính theo DPI.
- `scripts/check.ps1` phải in `ALL CHECKS PASSED`.

## 4. Các cách đã cân nhắc

### 4.1 Làm tối các điều khiển Windows gốc

- **A. Kết hợp (đã chọn).** Giữ điều khiển gốc (lớp `BUTTON`, `EDIT`, `COMBOBOX`, `SysTabControl32`, trackbar), nên hành vi bàn phím, tiêu điểm và trợ năng vẫn do Windows lo. Thanh tiêu đề tối bằng `DwmSetWindowAttribute(DWMWA_USE_IMMERSIVE_DARK_MODE)`. Phần nhìn phải **giống prototype đúng bảng màu ở mục 7** (chủ dự án chọn mức "giống hệt" sau đọc soát): nút bấm, thanh tab, ô tích, nút chọn và thanh trượt được tự vẽ; viền ô nhập, hộp chọn và ô bắt phím (kể cả gạch dưới màu nhấn khi có tiêu điểm) được tự vẽ; nền và chữ bên trong ô nhập qua `WM_CTLCOLOR*`. Thanh cuộn và danh sách thả xuống của hộp chọn dùng giao diện tối có sẵn của Windows (`SetWindowTheme` với `DarkMode_Explorer` hoặc `DarkMode_CFD`). Đây là phần dễ làm hỏng hành vi đã nghiệm thu nhất: mỗi điều khiển tự vẽ phải giữ thứ tự Tab, phím gạch dưới, Enter/Esc, khung tiêu điểm và tên đọc được bằng trình đọc màn hình. Menu khay dùng chế độ tối không công khai của `uxtheme` (`SetPreferredAppMode`, thứ tự 135); không có thì menu sáng chứ không hỏng. Gần giống prototype, ít rủi ro cho ba đợt đã nghiệm thu.
- **B. Tự vẽ toàn bộ.** Thay mọi điều khiển bằng điều khiển tự viết. Giống prototype tuyệt đối nhưng phải viết lại phần lớn `settings.rs` (khoảng 3.000 dòng) cùng bàn phím, tiêu điểm và trợ năng. Bị loại vì rủi ro và chi phí.
- **C. Chỉ đổi màu nền.** Bị loại: khi có visual styles, chữ của ô tích và nút chọn luôn đen, không đọc được trên nền tối.

### 4.2 Dữ liệu của người đang dùng shotme

- **A. Tự chuyển ở lần chạy đầu (đã chọn).** Xem mục 6.
- **B. Không chuyển.** Bị loại: người dùng phải nhập lại R2 và tự dọn mục khởi động cũ.

## 5. Đổi tên

Đổi "shotme" thành "darkshot" (viết thường, như tên cũ) ở:

| Chỗ | Cũ | Mới |
|---|---|---|
| Crate | `shotme-core`, `shotme-windows` (thư mục `crates/...`) | `darkshot-core`, `darkshot-windows` |
| Exe | `shotme.exe` | `darkshot.exe` |
| Biểu tượng | `res/shotme.ico` | `res/darkshot.ico` (nội dung không đổi) |
| Thư mục cấu hình | `%APPDATA%\shotme` | `%APPDATA%\darkshot` |
| Thư mục lưu mặc định | `Pictures\shotme` | `Pictures\darkshot` (chỉ cho cấu hình mới) |
| Tên file ảnh | `shotme_YYYY-MM-DD_hh-mm-ss.png` | `darkshot_...png` |
| Khóa Run | giá trị `shotme` | giá trị `darkshot` |
| WER | `shotme.exe` | `darkshot.exe` |
| Lớp cửa sổ, mutex, tên luồng | `shotme-...`, `Local\shotme-single-instance` | `darkshot-...`, `Local\darkshot-single-instance` |
| File thử của "Kiểm tra kết nối" | `shotme-test-...`, nội dung `shotme` | `darkshot-test-...`, `darkshot` |
| Chuỗi i18n, Thông tin, Trợ giúp, chú giải khay | "shotme" | "darkshot" |
| Tài liệu đang dùng | README, `docs/testing/manual-checklist.md`, `CLAUDE.md`, `memory-bank/*`, `.claude/agents/*`, `.claude/skills/*`, `scripts/*` | darkshot |
| Kiểm thử | tên thư mục tạm, chuỗi mong đợi | darkshot |

Không đổi: spec và kế hoạch đã duyệt, gói task cũ trong `docs/tasks/`, lịch sử git, `prototype/`.

Repo GitHub `tuanluu-agilityio/shotme` và thư mục `D:\2026 - Tuan\shotme`: phiên điều phối hỏi chủ dự án riêng trước khi đổi (thao tác bên ngoài, đổi đường dẫn bộ nhớ của Claude Code). Thiết kế này không phụ thuộc vào việc đổi hai thứ đó.

`scripts/check.ps1` có thêm một bước: không còn chữ `shotme` (không phân biệt hoa thường) trong `crates/`, `scripts/`, `README.md`, `docs/testing/`. Bước này bỏ qua đúng các chỗ mã chuyển dữ liệu cần tên cũ (mục 6); các chỗ đó gom trong một module (đề xuất `crates/darkshot-windows/src/legacy.rs`) để bước kiểm tra chỉ cần miễn một file.

## 6. Chuyển dữ liệu từ shotme

Chạy một lần, lúc khởi động, trước khi đọc cấu hình. Không hiện hộp thoại; nếu một bước lỗi thì bỏ qua bước đó, không chặn ứng dụng, và ghi một thông báo loại Info sau khi khay đã sẵn sàng.

1. **Cấu hình.** Nếu `%APPDATA%\darkshot\config.json` chưa có và `%APPDATA%\shotme\config.json` có: chép file sang (tạo thư mục nếu cần). Khóa R2 trong file là dữ liệu DPAPI gắn với tài khoản Windows, không gắn với đường dẫn, nên vẫn giải mã được. Không xóa thư mục cũ (người dùng có thể quay lại shotme).
2. **Thư mục lưu ảnh.** Nếu file cũ không có trường `save_dir`, file mới ghi rõ `save_dir` = `Pictures\shotme` (thư mục mặc định cũ), để ảnh tiếp tục vào chỗ cũ. Nếu có thì giữ nguyên. Chỉ cấu hình hoàn toàn mới mới dùng `Pictures\darkshot`.
3. **Khởi động cùng Windows.** Cấu hình darkshot (sau bước 1) là nguồn đúng duy nhất. Mỗi lần khởi động: nếu khóa Run có giá trị `shotme` thì xóa nó; rồi giá trị `darkshot` có mặt khi và chỉ khi `autostart` của cấu hình darkshot bật (ghi đường dẫn exe đang chạy như hiện nay, xóa nếu tắt). Không bao giờ bật autostart chỉ vì thấy giá trị `shotme`: người dùng đã có cấu hình darkshot với autostart tắt thì vẫn tắt. Người chuyển từ shotme có autostart bật thì cấu hình chép sang đã mang `autostart` bật, nên vẫn tự khởi động. (Quyết định của phiên điều phối sau đọc soát.)
4. **WER.** Xóa giá trị `shotme.exe` trong `ExcludedApplications` nếu có; giá trị `darkshot.exe` được ghi như hiện nay.
5. **Hai bản cùng chạy.** Mutex đổi tên nên shotme và darkshot có thể chạy cùng lúc và tranh phím tắt. Không chặn: phím tắt bị giữ đã có thông báo "Phím tắt chưa hoạt động". README ghi "thoát shotme trước khi chạy darkshot".

Logic quyết định (chép hay không, `save_dir` nào, ghi hay xóa giá trị Run) nằm ở `darkshot-core` dưới dạng hàm thuần có kiểm thử. Lớp Windows chỉ đọc, ghi file và registry.

## 7. Giao diện tối

### 7.1 Bảng màu (đã duyệt qua prototype)

| Vai trò | Màu | Tương phản |
|---|---|---|
| Thanh tiêu đề (DWM tự vẽ; màu này chỉ để tham chiếu) | `#16181B` | |
| Nền cửa sổ, hàng nút dưới | `#1C1E22` | |
| Nền khung tab | `#24272C` | |
| Nền ô nhập | `#15171A` | |
| Viền ô, viền nút | `#4A4F57` | |
| Đường kẻ, viền thanh tab | `#383C43` | |
| Nền nút thường / khi trỏ | `#33373E` / `#3C4048` | |
| Chữ chính | `#E7E9EC` | 14,6:1 trên `#1C1E22` |
| Chữ phụ, chữ chỉ đọc, ghi chú | `#A3A9B2` | 7,4:1 |
| Màu nhấn (xanh lam) | `#4C8DFF` | 5,6:1 |
| Nền nút chính (OK, Đóng) / khi trỏ | `#2F6FE4` / `#3D7BF0`, chữ trắng | 4,9:1 |
| Lỗi | `#FF7B72` | trên 4,5:1 |
| Thành công | `#4ADE80` | trên 4,5:1 |
| Chữ khi điều khiển bị vô hiệu | `#7C828B` | 4,6:1 |

Tương phản phải đúng cả trên nền khung tab `#24272C`; người làm kiểm lại bằng kiểm thử đơn vị (hàm tính độ tương phản theo WCAG trong core, khẳng định mỗi cặp chữ/nền đang dùng ≥ 4,5:1).

Bảng màu là hằng số ở một chỗ (đề xuất `darkshot-core::theme`, chỉ là số, không phụ thuộc Windows). Lớp Windows tạo chổi một lần cho mỗi cửa sổ và giải phóng khi đóng.

### 7.2 Phông chữ

Giữ phông hệ thống như hiện nay (`lfMessageFont`, thường là Segoe UI 9pt). Tên tab đang chọn và nhãn nút chính dùng bản đậm (weight 600 nếu có, không thì 700). Không thêm phông riêng.

### 7.3 Từng phần

**Mọi cửa sổ cấp cao** (Tùy chọn, Thông tin, Trợ giúp): thanh tiêu đề tối (`DWMWA_USE_IMMERSIVE_DARK_MODE` = 20, thử 19 nếu 20 lỗi); nền `#1C1E22`. Gọi cho cửa sổ trước khi hiện, để không nhá trắng.

**Cửa sổ Tùy chọn:**
- Thanh tab: tự vẽ. Không có hộp quanh tab; tab đang chọn chữ chính đậm và gạch dưới màu nhấn dày 3 DIP; tab khác chữ phụ; khi trỏ nền `#2E3238`. Đường kẻ `#383C43` dưới thanh tab. Có khung tiêu điểm khi đi bằng bàn phím. Điều khiển tab gốc vẫn giữ để có Ctrl+Tab và trợ năng; chỉ phần vẽ đổi.
- Khung tab: nền `#24272C`.
- Ô nhập: nền `#15171A`, chữ chính, viền `#4A4F57`; ô đang có tiêu điểm có gạch dưới màu nhấn. Ô chỉ đọc chữ phụ.
- Hộp chọn (Ngôn ngữ, Hạn của link): nền như ô nhập; danh sách thả xuống tối.
- Nút thường: nền `#33373E`, viền `#454A52`, chữ chính. Nút mặc định (OK): nền màu nhấn đậm, chữ trắng đậm. Có khung tiêu điểm.
- Ô tích: hộp 18 DIP bo 4 DIP; tắt là viền `#8A9099`; bật là nền `#2F6FE4` và dấu tích trắng. Chữ bên cạnh chữ chính.
- Nút chọn (PNG/JPEG): vòng tròn; bật là viền và chấm màu nhấn.
- Thanh trượt chất lượng: rãnh `#454A52`, phần đã qua màu nhấn, núm tròn sáng có viền màu nhấn.
- Chữ lỗi và chữ thành công (mục 7.1) thay chữ đỏ `0x0000B4` hiện tại.
- Ô bắt phím đang chờ: gạch dưới màu nhấn, lời nhắc chữ phụ.

**Thông tin:** như prototype: biểu tượng, tên darkshot cỡ lớn hơn, đậm; phiên bản và mô tả chữ phụ; nút Đóng là nút chính.

**Trợ giúp:** ô chữ nhiều dòng nền `#15171A`, chữ chính; thanh cuộn tối.

**Menu khay:** chế độ tối không công khai (`SetPreferredAppMode(AllowDark hoặc ForceDark)` rồi `FlushMenuThemes`, nạp theo thứ tự từ `uxtheme.dll` bằng `GetProcAddress`). Không nạp được thì bỏ qua. Mục "Chụp màn hình" vẫn đậm.

**Lớp phủ chọn vùng:** thay các hằng màu trong `overlay.rs`: thanh công cụ nền `#24272C`, viền `#3F434A`, vạch ngăn `#3F434A`, biểu tượng `#D5D9DF`, khi trỏ nền `#363A41` và biểu tượng màu nhấn; viền vùng chọn và viền tay nắm màu nhấn, ruột tay nắm trắng; nhãn kích thước nền gần đen `#16181B`, chữ chính; chú thích nút nền `#2B2D31`. Lớp tối ngoài vùng chọn giữ như hiện nay.

### 7.4 Những gì vẫn sáng

Thông báo ở khay và hộp chọn thư mục theo chế độ của Windows; README ghi rõ điều này. Thanh tiêu đề trên Windows 10 trước 20H1 vẫn sáng.

## 8. Kiểm thử

- Core: chuyển dữ liệu (mọi tổ hợp có/không file cũ, file mới, `save_dir`, giá trị Run cũ/mới); độ tương phản của mọi cặp màu đang dùng.
- Windows: không rò GDI khi mở đóng Tùy chọn, Thông tin, Trợ giúp nhiều lần (theo mẫu kiểm thử lớp phủ hiện có); lớp cửa sổ và mutex mang tên mới.
- `scripts/check.ps1`: bước không còn chữ `shotme` (mục 5).
- Thử tay: thêm mục "Đợt 4" vào `docs/testing/manual-checklist.md`: cài sẵn shotme có R2 và khởi động cùng Windows, chạy darkshot, kiểm giữ được mọi thứ; nhìn từng tab ở 100% và 150% DPI, cả hai ngôn ngữ; đi hết cửa sổ bằng bàn phím; menu khay; lớp phủ trên hai màn hình.

## 9. Để người làm quyết

- Cách vẽ cụ thể từng điều khiển (owner-draw `BS_OWNERDRAW` hay `NM_CUSTOMDRAW`, subclass hay xử lý ở cửa sổ cha), miễn giữ hành vi bàn phím và trợ năng.
- Chia task và agent theo luật dự án (kế hoạch riêng, gói task từ số 27). Gợi ý: đổi tên và chuyển dữ liệu tách khỏi giao diện tối; đổi tên làm trước.
- Chỗ đặt hằng màu và hàm tương phản (đề xuất `darkshot-core::theme`).
