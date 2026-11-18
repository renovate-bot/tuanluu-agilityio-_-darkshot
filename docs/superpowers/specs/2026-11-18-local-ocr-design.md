# Thiết kế: Nhận dạng chữ cục bộ (OCR) (Giai đoạn 3 sản phẩm, đợt 11 nội bộ)

Ngày: 2026-11-18. Người quyết: phiên điều phối (quyết định 114: làm lần lượt các ứng viên Giai đoạn 3).

## 1. Mục đích

Người dùng thường chụp để lấy chữ: thông báo lỗi, đoạn văn trong ảnh, số liệu trong hộp thoại không cho bôi đen. darkshot cho chọn một vùng rồi lấy chữ trong vùng đó để sửa và chép, **hoàn toàn trên máy**: dùng bộ nhận dạng có sẵn của Windows (`Windows.Media.Ocr`), không gửi ảnh đi đâu, không AI từ xa, không crate mới.

## 2. Phạm vi

Làm:
1. Lớp phủ: mục mới **Nhận dạng chữ** trên thanh ngang, giữa Ghim và Hủy (`Hoàn tác, Làm lại | Tải lên, Lưu, Chép, Ghim, Nhận dạng chữ, Hủy`), phím **Ctrl+T**. Ảnh nhận dạng = ảnh vùng chọn có hình đánh dấu (như các hành động khác; người dùng có thể che chữ không muốn lấy bằng Làm mờ).
2. Ảnh ghim: menu chuột phải thêm "Nhận dạng chữ" (sau "Lưu ảnh"). Nhận dạng ảnh gốc của ảnh ghim.
3. Cửa sổ kết quả **"Chữ nhận dạng"**: một cửa sổ tối, không chặn, đổi cỡ được; ô chữ nhiều dòng sửa được (chọn sẵn toàn bộ), dòng trạng thái (ngôn ngữ đã dùng, số dòng), nút **Chép** (chính, Enter khi tiêu điểm không ở ô chữ, Ctrl+Enter ở mọi nơi) và **Đóng** (Esc). Chép = đưa chữ hiện có trong ô (kể cả đã sửa) vào clipboard rồi đóng cửa sổ, thông báo Success "Đã chép chữ". Mỗi lần nhận dạng mở một cửa sổ mới (tối đa 8; quá thì đóng cái cũ nhất).
4. Trạng thái: đang nhận dạng → cửa sổ hiện ngay với chữ "Đang nhận dạng chữ…" (ô chữ chỉ đọc), nhận dạng chạy trên luồng nền; xong thì điền chữ. Không có chữ → "Không tìm thấy chữ trong ảnh." Lỗi → thông báo rõ trong cửa sổ.
5. Ngôn ngữ: `OcrEngine::TryCreateFromUserProfileLanguages()` (ngôn ngữ trong cài đặt Windows của người dùng). Không tạo được (máy không có gói OCR nào) → cửa sổ báo "Windows chưa có gói nhận dạng chữ cho ngôn ngữ của bạn. Thêm ngôn ngữ trong Cài đặt > Thời gian và ngôn ngữ > Ngôn ngữ và vùng." Dòng trạng thái ghi tên ngôn ngữ đã dùng (thẻ BCP-47 + tên hiển thị của Windows).
6. Ảnh lớn hơn `OcrEngine::MaxImageDimension` → thu nhỏ giữ tỉ lệ trước khi nhận dạng. Ảnh rất nhỏ (cạnh ngắn < 40 px) → phóng ×2 (chữ nhỏ dễ đọc hơn) miễn không vượt giới hạn.
7. Ghép chữ: mỗi `OcrLine` một dòng (`\r\n` trong ô chữ, `\n` khi chép theo chuẩn clipboard văn bản Windows: dùng `\r\n`). Bỏ khoảng trắng thừa đầu/cuối dòng; giữ thứ tự Windows trả về.
8. Không ghi Lịch sử, không thông báo khi mở cửa sổ (chính cửa sổ là phản hồi). Lớp phủ đóng khi chọn Nhận dạng chữ.

Không làm (quyết định 118): chọn ngôn ngữ OCR trong Tùy chọn, OCR từ Lịch sử, giữ bố cục cột/bảng, nhận dạng chữ viết tay, dịch, tìm trong chữ, tự chép khi xong (giữ clipboard của người dùng cho tới khi họ bấm Chép).

## 3. Kiến trúc

- `darkshot-core`:
  - `actions.rs`: `Action::Ocr` (`run_action` không làm gì, trả `Cancelled`, như `Pin`).
  - `ocr_text.rs` (mới, thuần): `join_lines(lines: &[String]) -> String` (trim, bỏ dòng rỗng đầu/cuối, nối `\r\n`), `ocr_scale(w, h, max_dim) -> Option<(u32,u32)>` (thu theo `max_dim`, phóng ×2 khi cạnh ngắn < 40 mà vẫn ≤ max_dim; None khi giữ nguyên), `line_count(text)`.
  - `i18n.rs`: chuỗi mới (mục 5).
- `darkshot-windows`:
  - `ocr.rs` (mới): `recognize(image: &Image) -> Result<OcrResult, OcrError>` chạy ĐỒNG BỘ trên luồng gọi (luồng nền), khởi tạo WinRT MTA trên luồng đó (`RoInitialize(RO_INIT_MULTITHREADED)` hoặc tương đương của `windows` 0.62), tạo `SoftwareBitmap` BGRA8 Premultiplied từ điểm ảnh, `OcrEngine::TryCreateFromUserProfileLanguages`, `RecognizeAsync(..).get()`; `OcrResult { text: String, lang_tag: String, lang_name: String, lines: usize }`; `OcrError::{NoLanguage, Failed(String)}`. `start(image, hwnd_result)`: tạo luồng `darkshot-ocr`, gửi kết quả bằng `PostMessage` (`Box<…>` trong LPARAM, giải phóng nếu gửi lỗi).
  - `ocr_window.rs` (mới): cửa sổ kết quả theo mẫu `help.rs`/`about.rs` (không chặn, `pre_translate`, tối, `theme::*`), ô EDIT nhiều dòng có thanh cuộn dọc, nút tự vẽ như Tùy chọn, `WM_DPICHANGED`, đổi cỡ, tối thiểu 420×260 DIP, mặc định 560×380 DIP, đặt giữa màn hình chứa con trỏ.
  - `overlay.rs`: `BarItem::Act(Action::Ocr)` + biểu tượng (khung góc + chữ "T"), Ctrl+T, chú thích "Nhận dạng chữ (Ctrl+T)".
  - `pin.rs`: mục menu "Nhận dạng chữ".
  - `main.rs`: `Action::Ocr` → mở cửa sổ kết quả + `ocr::start`; đóng mọi cửa sổ OCR khi thoát.
  - `Cargo.toml`: feature `windows` mới `Foundation`, `Globalization`, `Graphics_Imaging`, `Media_Ocr`, `Storage_Streams` (chỉ những gì cần; người làm đo kích thước exe trước/sau và ghi lại).

## 4. Riêng tư

Ảnh chỉ ở bộ nhớ, chuyển cho thư viện OCR của Windows trên máy. Không mạng mới, không ghi tệp, không nhật ký chữ nhận dạng.

## 5. Chữ (Việt / English)

`OverlayOcr` "Nhận dạng chữ (Ctrl+T)" / "Recognize text (Ctrl+T)"; `PinMenuOcr` "Nhận dạng chữ" / "Recognize text"; `OcrWindowTitle` "Chữ nhận dạng" / "Recognized text"; `OcrBusy` "Đang nhận dạng chữ…" / "Recognizing text…"; `OcrEmpty` "Không tìm thấy chữ trong ảnh." / "No text found in the image."; `OcrNoLanguage` (câu mục 2.5) / "Windows has no text recognition pack for your languages. Add a language in Settings > Time & language > Language & region."; `OcrFailed` "Không nhận dạng được chữ. Thử lại." / "Couldn't recognize text. Try again."; `OcrCopy` "Chép" / "Copy"; `OcrClose` "Đóng" / "Close"; `OcrCopiedTitle` "Đã chép chữ" / "Text copied"; hàm `ocr_status(lang, name, lines)` "Ngôn ngữ: {name} · {lines} dòng" / "Language: {name} · {lines} lines". Trợ giúp: một câu về Ctrl+T và menu ảnh ghim.

## 6. Tiêu chí nghiệm thu

1. Chọn vùng có chữ tiếng Anh, Ctrl+T: cửa sổ hiện "Đang nhận dạng…" rồi chữ đúng; Chép đưa chữ vào clipboard.
2. Máy có gói tiếng Việt: chữ tiếng Việt có dấu đúng phần lớn; không có gói: dùng ngôn ngữ có sẵn, dòng trạng thái ghi rõ.
3. Vùng không có chữ: "Không tìm thấy chữ trong ảnh."
4. Menu ảnh ghim → Nhận dạng chữ chạy như trên.
5. Không chặn giao diện khi nhận dạng ảnh 4K; đóng cửa sổ khi đang nhận dạng không rò, không panic.
6. Hai ngôn ngữ, DPI 100–200%, không cắt chữ; `scripts/check.ps1` → `ALL CHECKS PASSED`; không hồi quy đợt 1–10.
