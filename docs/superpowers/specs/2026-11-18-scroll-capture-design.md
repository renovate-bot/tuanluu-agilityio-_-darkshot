# Thiết kế: Chụp cuộn (Giai đoạn 3 sản phẩm, đợt 13 nội bộ)

Ngày: 2026-11-18. Người quyết: phiên điều phối (quyết định 114: ứng viên cuối của danh sách chủ dự án).

## 1. Mục đích và phạm vi hỗ trợ

Chụp một trang web dài, một tài liệu, một danh sách mà không vừa màn hình, thành **một ảnh dọc**. Không hứa tương thích mọi ứng dụng: bản đầu hỗ trợ **cuộn dọc** nội dung nằm trong **một vùng chữ nhật cố định** do người dùng chọn, khi **người dùng tự cuộn** (con lăn, thanh cuộn, phím) trong lúc darkshot chụp. darkshot **không** tự gửi lệnh cuộn vào ứng dụng khác (kém ổn định giữa các ứng dụng và phải can thiệp vào chúng; ghi ở quyết định 127).

Nguyên tắc tin cậy: **không bao giờ âm thầm ghép sai**. Khung nào khớp không chắc chắn thì bỏ, và kết quả ghi rõ "có chỗ ghép không chắc chắn" để người dùng kiểm.

## 2. Luồng

1. Chọn vùng trong lớp phủ như thường (có thể dùng gợi ý cửa sổ). Bấm mục mới **Chụp cuộn** trên thanh ngang (biểu tượng mũi tên xuống trong khung, giữa Nhận dạng chữ và Hủy — trước Thêm bước nếu có phiên) hoặc **Ctrl+L**. Hình đánh dấu (nếu có) bị bỏ qua cho chụp cuộn (vẽ trên ảnh tĩnh không khớp nội dung cuộn); tooltip nói rõ.
2. Lớp phủ đóng. Hiện:
   - **Khung vùng**: viền nét đứt màu nhấn 2 DIP bao NGOÀI vùng (bốn cửa sổ mảnh, luôn trên cùng, xuyên chuột, loại khỏi ảnh chụp `WDA_EXCLUDEFROMCAPTURE`; Windows cũ hơn 2004: không vẽ khung).
   - **Thanh điều khiển** nổi (tối, không lấy tiêu điểm, loại khỏi ảnh chụp như thanh phiên đợt 12) ngay dưới vùng (hoặc trên nếu không đủ chỗ, hoặc trong vùng làm việc gần nhất), nội dung: chấm đỏ, "Chụp cuộn · Cuộn xuống từ từ", chiều cao hiện có "1 840 px", trạng thái, nút **Xong** (chính) và **Hủy**. Esc = Hủy (phím tắt toàn cục tạm như lớp phủ), Enter không gán (tránh nuốt phím của ứng dụng đang cuộn).
3. darkshot chụp vùng mỗi 100 ms (BitBlt chỉ vùng đó), so với khung trước:
   - Không đổi → bỏ qua.
   - Dịch xuống dy px khớp chắc → nối dy hàng mới vào đáy ảnh.
   - Khớp không chắc / cuộn ngược / nhảy quá xa (không còn chồng lấp) → bỏ khung, đếm "khung không ghép được", trạng thái chuyển vàng "Cuộn chậm hơn" trong 1,5 s; nếu đã có nối trước đó thì đánh dấu kết quả **không chắc chắn**.
4. Dừng khi: Xong; Hủy (bỏ hết); đạt giới hạn 30 000 px chiều cao hoặc 256 MB ảnh (tự dừng như Xong, thông báo Info "Đã đạt giới hạn chiều cao"); vùng không còn hợp lệ (màn hình đổi) → tự dừng như Xong.
5. **Cửa sổ kết quả** "Ảnh chụp cuộn": xem trước thu nhỏ theo bề rộng, cuộn được bằng con lăn/thanh cuộn; dòng thông tin "W × H px"; nếu không chắc chắn: dải cảnh báo vàng "Có N chỗ ghép không chắc chắn. Kiểm tra ảnh trước khi dùng."; nút **Lưu**, **Chép**, **Tải lên**, **Ghim**, **Đóng**. Lưu/Chép/Tải lên đi đúng đường `run_on_image` (thông báo, Lịch sử, dự phòng). Ghim mở ảnh ghim giữa màn hình (thu vừa như Ghim từ Lịch sử). Đóng không lưu thì bỏ ảnh (không hỏi — người dùng đã thấy nút Lưu).
6. Chỉ có 1 khung (không cuộn) → kết quả là ảnh vùng bình thường, không cảnh báo.

## 3. Thuật toán ghép (`darkshot-core::scroll_stitch`, thuần, kiểm thử bằng ảnh tổng hợp)

- Đầu vào: chuỗi khung cùng cỡ W×H (BGRA). Mỗi hàng → băm 64 bit (FNV-1a trên byte BGR).
- **Dải tĩnh**: so khung mới với khung trước; `static_top` = số hàng đầu giống hệt nhau (tối đa H/3), `static_bottom` tương tự ở đáy. Dải tĩnh được cập nhật theo giá trị nhỏ nhất quan sát được khi có chuyển động (tiêu đề dính, chân trang dính).
- **Vùng chạy** = hàng [static_top, H − static_bottom). Tìm dy ∈ [1, run_h − MIN_OVERLAP] (MIN_OVERLAP = max(24 px, 15% run_h)) sao cho hàng chạy của khung mới [i] khớp hàng chạy của khung trước [i + dy] với tỉ lệ ≥ 98% trên vùng chồng lấp.
- **Độ chắc chắn**: số hàng "phân biệt" (băm khác băm hàng liền trên và không phải hàng một màu) trong vùng chồng lấp ≥ 8; và không có dy thứ hai đạt cùng điểm (mơ hồ do nền lặp). Không đạt → `Uncertain`.
- Ghép: ảnh kết quả = dải tĩnh trên (một lần, của khung đầu) + hàng chạy khung đầu + với mỗi khung chắc chắn: dy hàng chạy mới ở đáy + dải tĩnh dưới (của khung cuối, một lần ở cuối).
- `Stitcher { push(&Image) -> Push::{NoChange, Appended(dy), Uncertain(reason)}, height(), uncertain_count(), finish() -> (Image, uncertain_count) }`, giới hạn `MAX_HEIGHT = 30_000`, `MAX_BYTES = 256 MiB` → `Push::Limit`.

## 4. Kiến trúc

- `darkshot-core`: `scroll_stitch.rs`; `Action::Scroll` (no-op ở `run_action`, không nhớ là lần chụp trước); chuỗi i18n mới.
- `darkshot-windows`:
  - `capture.rs`: `capture_rect(screen_rect) -> Image` (BitBlt một vùng, không con trỏ).
  - `scroll_session.rs`: khung vùng, thanh điều khiển, hẹn giờ 100 ms trên luồng giao diện (BitBlt + băm hàng nhanh; nếu một lượt > 50 ms thì đo và ghi), Esc toàn cục tạm, dừng, giới hạn.
  - `scroll_result.rs`: cửa sổ kết quả.
  - `overlay.rs` (ô Chụp cuộn + Ctrl+L), `main.rs` (nối, cổng chụp: trong lúc chụp cuộn, phím chụp khác bị bỏ qua như khi lớp phủ mở; thoát: hủy phiên chụp cuộn).
- Không crate mới.

## 5. Chữ (Việt / English)

`OverlayScroll` "Chụp cuộn (Ctrl+L)" / "Scrolling capture (Ctrl+L)"; `ScrollBarTitle` "Chụp cuộn" / "Scrolling capture"; `ScrollHint` "Cuộn xuống từ từ" / "Scroll down slowly"; `ScrollSlowDown` "Cuộn chậm hơn" / "Scroll more slowly"; hàm `scroll_height(lang, px)` "{px} px"; `ScrollDone` "Xong" / "Done"; `ScrollCancel` "Hủy" / "Cancel"; `ScrollLimitTitle` "Đã đạt giới hạn chiều cao" / "Height limit reached"; `ScrollResultTitle` "Ảnh chụp cuộn" / "Scrolling capture"; hàm `scroll_uncertain(lang, n)` "Có {n} chỗ ghép không chắc chắn. Kiểm tra ảnh trước khi dùng." / "{n} join(s) may be inaccurate. Check the image before using it."; `ScrollSave` "Lưu", `ScrollCopy` "Chép", `ScrollUpload` "Tải lên", `ScrollPin` "Ghim", `ScrollClose` "Đóng" (English Save/Copy/Upload/Pin/Close); `ScrollNoAnnotations` (thêm vào tooltip) "Không kèm hình đánh dấu" / "Annotations are not included". Trợ giúp: một đoạn ngắn.

## 6. Tiêu chí nghiệm thu

1. Trang tổng hợp có chữ đa dạng, cuộn dọc đều: ảnh ghép đúng thứ tự, không lặp, không mất hàng (kiểm thử core với ảnh tổng hợp có tiêu đề dính và chân trang dính).
2. Nền một màu / lặp: không ghép mò, báo không chắc chắn.
3. Cuộn quá nhanh (không chồng lấp): báo "Cuộn chậm hơn", kết quả có cảnh báo.
4. Khung và thanh điều khiển không có trong ảnh; thanh không lấy tiêu điểm của ứng dụng đang cuộn.
5. Hủy/Esc bỏ hết; thoát giữa chừng không treo, không rò.
6. Lưu/Chép/Tải lên/Ghim từ cửa sổ kết quả như các đường thường; hai ngôn ngữ; DPI; `scripts/check.ps1` → `ALL CHECKS PASSED`; không hồi quy đợt 1–12.
