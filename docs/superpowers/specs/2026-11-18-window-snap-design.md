# Đợt 8: Bắt cửa sổ khi chọn vùng (và gia cố Lịch sử)

Ngày: 2026-11-18. Tác giả: phiên điều phối, theo quyền tự quyết chủ dự án giao (2026-11-18). Chủ dự án duyệt trên bản demo cuối, không duyệt từng bước.

## 0. Phạm vi của đợt

Đợt 8 gồm hai phần, làm theo thứ tự:

1. **Gia cố Lịch sử (Task 59)** — xử lý các điểm còn mở của quyết định 92 trước khi thêm tính năng mới, vì hai trong số đó làm sai dữ liệu lịch sử.
2. **Bắt cửa sổ (Task 60–63)** — trong lớp phủ chọn vùng, rê chuột làm nổi khung cửa sổ (hoặc màn hình) dưới con trỏ; bấm một lần để chọn đúng khung đó.

## 1. Gia cố Lịch sử

### 1.1 Kết quả rà soát quyết định 92 (mã tại `2797bb9`)

| # | Vấn đề | Còn không | Nguyên nhân gốc | Mức |
|---|---|---|---|---|
| 1 | Mất mục khi thoát ngay sau Lưu/Chép/Tải lên | Còn | `history_service::record_outcome` sinh một luồng tách rời cho mỗi mục; khi `main()` trả về, tiến trình kết thúc và cắt ngang mọi luồng đang ghi. Không ai chờ chúng. | Cao (mất dữ liệu) |
| 2 | Hai mục khi thoát đúng lúc một lượt tải lên vừa xong | Còn | Luồng tải lên gọi `PendingUploads::remove(id)` rồi **bỏ qua kết quả** và luôn gọi móc ghi lịch sử. Nếu `save_pending_uploads` lúc thoát đã `drain()` lượt đó trước, lượt bị ghi hai lần: `UploadFallback` (lúc thoát) và `Uploaded` (luồng tải lên). Quyền sở hữu lượt không được trao đúng một lần. | Trung bình (trùng dữ liệu) |
| 3 | Thanh cuộn danh sách không mờ khi thẻ xác nhận mở | Còn | Danh sách bị `EnableWindow(false)`, nhưng thanh cuộn `WS_VSCROLL` là vùng ngoài client, hệ thống vẫn vẽ nó như bình thường; lớp mờ chỉ phủ client của cửa sổ cha. | Thấp (hình ảnh) |
| 4 | Hạn của mục tải lên tính lúc móc chạy | Đúng như mô tả, **không phải lỗi** | R2 tính hạn theo vòng đời đối tượng, kể từ lúc đối tượng được tạo, tức lúc lệnh PUT xong. Móc chạy ngay sau PUT nên ngày của móc là mốc sát nhất. Lấy mốc lúc bấm chụp sẽ sai nhiều hơn khi tải lên chậm qua nửa đêm. Hạn hiện ở mức ngày; R2 xóa trễ tối đa khoảng một ngày, nên ngày hiển thị là "còn ít nhất tới". | Không sửa; ghi quyết định |
| 5 | DPI 100/150%, Windows 10, mở tệp/trình duyệt/clipboard thật | Chưa thử | Môi trường của phiên điều phối chỉ có Windows 11, một màn hình. | Giữ trong checklist, ghi rõ trạng thái |

Thêm khi rà soát: chỉ mục và ảnh được ghi bằng `fs::write` + `rename` nhưng không `sync_all`; mất điện ngay sau đổi tên có thể để lại `index.json` rỗng. Tệp hỏng đã được giữ lại thành `index.corrupt*.json` khi nạp, nên không mất cả kho, nhưng mất mọi mục. Sửa: `sync_all` tệp tạm trước khi đổi tên (chi phí vài mili giây trên luồng nền).

### 1.2 Cách sửa

- **Một luồng ghi duy nhất, có hàng đợi** (`darkshot-history`, sống suốt tiến trình, sinh lần đầu khi cần). `record_outcome` chỉ đẩy việc vào hàng (không chặn, không sao ảnh khi tắt). Ghi tuần tự nên không còn tranh khóa chỉ mục giữa nhiều luồng.
- **`history_service::flush(timeout)`**: chờ hàng đợi rỗng và việc đang ghi xong, tối đa `timeout`. `main()` gọi nó sau `save_pending_uploads()` ở cuối vòng lặp thông điệp và ở `WM_ENDSESSION` (giới hạn 3 giây; Windows cho khoảng 5 giây). Hết giờ thì bỏ, không treo khi thoát.
- **Trao quyền đúng một lần**: `PendingUploads::finish(id, f)` chạy `f` **trong khi giữ khóa sổ** và chỉ khi lượt còn trong sổ; trả `false` nếu lúc thoát đã lấy lượt đó. Luồng tải lên chỉ ghi lịch sử qua `finish`. Vì `f` chỉ đẩy việc vào hàng (nhanh), giữ khóa không đáng kể. Thứ tự lúc thoát: `drain()` → ghi dự phòng → `flush()`; mọi việc được đẩy trước `drain()` nằm trong hàng khi `flush()` chạy.
- **Thanh cuộn khi thẻ mở**: `EnableScrollBar(list, SB_VERT, ESB_DISABLE_BOTH)` khi thẻ mở (sau `SetScrollInfo`, vì `SIF_DISABLENOSCROLL` có thể bật lại), khôi phục khi thẻ đóng. Thanh cuộn chủ đề tối vẽ trạng thái vô hiệu bằng màu mờ, khớp lớp phủ mờ; bấm vào thanh cuộn không cuộn nữa.
- **Bền vững**: `sync_all` trước `rename` cho chỉ mục và ảnh của kho.

## 2. Bắt cửa sổ — mục tiêu sản phẩm

### 2.1 Vấn đề người dùng

- Muốn chụp **đúng một cửa sổ** (hộp thoại lỗi, trang web, một ứng dụng) thì nay phải kéo tay sát mép; mép lệch vài pixel lộ nền hoặc cắt mất viền. Đây là thao tác chụp phổ biến nhất sau chọn vùng tự do.
- Muốn chụp **một màn hình** trong hệ nhiều màn hình thì phải kéo hết màn hình, hoặc dùng phím toàn màn hình (chụp mọi màn hình).

### 2.2 Câu chuyện người dùng

1. Là người viết báo lỗi, tôi bấm phím tắt, rê chuột vào hộp thoại lỗi, bấm một lần là có đúng khung hộp thoại, rồi đánh dấu và Tải lên như mọi khi.
2. Là người dùng hai màn hình, tôi rê chuột ra vùng trống của màn nền và bấm: chọn đúng màn hình đó.
3. Là người đã quen kéo vùng, tôi kéo như cũ; không có gì thay đổi khi tôi kéo.
4. Là người không thích gợi ý, tôi tắt "Gợi ý cửa sổ khi chọn vùng" trong Tùy chọn → Chụp.
5. Là người dùng bàn phím, tôi rê tới cửa sổ rồi bấm Enter để chọn nó.

### 2.3 Vì sao chọn tính năng này (đánh giá)

| Ứng viên | Giá trị | UX | Khả thi Rust/Win32 | Rủi ro đường chụp | Hiệu năng | Chi phí | Hợp triết lý |
|---|---|---|---|---|---|---|---|
| **Bắt cửa sổ trong lớp phủ** | Cao: thao tác phổ biến | Không thêm bước, không thêm phím | Cao: `EnumWindows` + DWM, lõi thuần kiểm thử được | Thấp: chỉ thêm khi chưa có vùng chọn | Không đáng kể (liệt kê vài ms lúc chụp) | 4 task | Rất hợp: nhẹ, một chạm |
| Chụp có hẹn giờ | Trung bình | Thêm mục menu, đồng hồ đếm | Cao | Thấp | Không | 2–3 task | Hợp |
| Chụp cuộn và ghép | Cao với một nhóm nhỏ | Phức tạp, dễ hỏng | Thấp: phụ thuộc từng ứng dụng, ghép ảnh dễ sai | Cao | Nặng | Lớn | Kém: nặng, khó tin cậy |
| Mở rộng Lịch sử (gắn thẻ, nhiều chọn) | Thấp lúc này | Lịch sử mới ra, chưa có phản hồi | Cao | Không | Không | 3 task | Hợp nhưng sớm |

Không chọn: hẹn giờ (darkshot đã đóng băng màn hình ngay khi bấm phím tắt, nên đa số menu/tooltip vẫn chụp được; để đợt sau nếu có phản hồi), chụp cuộn (rủi ro và kích thước), mở rộng Lịch sử (chờ phản hồi thực tế). Một phím tắt riêng "chụp cửa sổ đang hoạt động" để ngoài phạm vi: bắt cửa sổ đã cho kết quả trong hai thao tác mà không thêm dòng phím tắt thứ tư vào Tùy chọn.

## 3. Hành vi

### 3.1 Khi nào có gợi ý

Gợi ý cửa sổ chỉ chạy khi **cả bốn** điều kiện đúng: ô "Gợi ý cửa sổ khi chọn vùng" bật (mặc định bật); lớp phủ **chưa có vùng chọn** (kể cả vùng nhớ từ lần trước); **không đang kéo**; không có bảng nổi nào mở. Khi đã có vùng chọn, lớp phủ chạy y như hôm nay.

### 3.2 Ứng viên

Lúc chụp (cùng thời điểm đóng băng màn hình), darkshot liệt kê các cửa sổ cấp cao theo thứ tự Z từ trên xuống. Một cửa sổ là ứng viên khi: hiện (`IsWindowVisible`), không thu nhỏ (`IsIconic`), không bị che giấu bởi DWM (`DWMWA_CLOAKED` = 0: cửa sổ ở màn hình ảo khác, ứng dụng UWP đang ngủ), không trong suốt với chuột (`WS_EX_TRANSPARENT`), không phải lớp nền của màn hình (lớp `Progman`, `WorkerW`), và khung (sau khi cắt theo màn hình ảo) rộng và cao ít nhất 16 pixel vật lý. Khung dùng `DWMWA_EXTENDED_FRAME_BOUNDS` (bỏ viền đổi cỡ vô hình 7 px của Windows 10/11); lỗi thì dùng `GetWindowRect`. Tối đa 512 ứng viên. Sau các cửa sổ là **các màn hình** (thứ tự như `Screenshot::monitors`): rê chuột vào nền màn hình thì ứng viên là màn hình đó.

Ứng viên tại điểm `p` = ứng viên **đầu tiên** (trên cùng) chứa `p`. Vì màn hình nằm cuối danh sách, luôn có ứng viên khi `p` nằm trong một màn hình.

### 3.3 Chuột

| Hành động | Kết quả |
|---|---|
| Rê chuột (chưa có vùng chọn) | Ứng viên dưới con trỏ hiện như một vùng chọn xem trước: ảnh sáng bên trong, nền tối bên ngoài, viền hai lớp như vùng chọn nhưng **không** tay nắm, không thanh công cụ; nhãn cỡ "W × H" như vùng chọn. Đổi ứng viên chỉ vẽ lại phần thay đổi. |
| Bấm và nhả **không kéo** (vùng sinh ra < 3 px như hôm nay) | Ứng viên dưới con trỏ thành vùng chọn thật (có tay nắm, thanh công cụ). |
| Bấm rồi kéo | Chọn vùng tự do như hôm nay; gợi ý biến mất ngay khi bắt đầu kéo. |
| Có vùng chọn, bấm ra ngoài (không kéo) | Như hôm nay: bỏ vùng chọn; gợi ý xuất hiện lại. |

### 3.4 Bàn phím

- **Enter** khi chưa có vùng chọn và có ứng viên: chọn ứng viên (như bấm). Enter khi đã có vùng chọn: giữ hành vi hiện tại.
- Esc: không đổi (đóng lớp phủ khi không còn gì để lùi).

### 3.5 Lời nhắc

Khi chưa có vùng chọn, một viên nhắc nhỏ ở giữa mép trên của màn hình chứa con trỏ: "Bấm: chọn cửa sổ · Kéo: chọn vùng · Esc: thoát" (EN: "Click: window · Drag: region · Esc: cancel"). Khi ô gợi ý tắt: "Kéo để chọn vùng · Esc để thoát" (EN: "Drag to select · Esc to cancel"). Viên nhắc ẩn khi có vùng chọn, khi đang kéo, và khi con trỏ cách viên nhắc dưới 48 DIP (để không che chỗ người dùng muốn nhìn). Viên nhắc dùng màu `OVERLAY_LABEL`/chữ `TEXT` của `theme`, bo góc, phông giao diện, cỡ theo DPI của màn hình đó. Viên nhắc không bao giờ nằm trong ảnh xuất ra (chỉ vẽ trên lớp phủ).

### 3.6 Tùy chọn

Tab "Chụp" (cùng nhóm "Giữ vị trí vùng chọn", "Chụp cả con trỏ"): thêm ô tích "Gợi ý cửa sổ khi chọn vùng" (EN "Suggest windows while selecting"), mặc định **bật**. Lưu trong cấu hình là `snap_windows` (thiếu hoặc sai kiểu → `true`).

### 3.7 Trợ giúp

Văn bản Trợ giúp thêm một câu về bấm để chọn cửa sổ/màn hình và Enter.

## 4. Trạng thái

- **Không có cửa sổ nào** (liệt kê lỗi): vẫn có màn hình làm ứng viên; tính năng tự thoái hóa thành "bấm để chọn màn hình".
- **Ô gợi ý tắt**: không liệt kê cửa sổ, không hiện xem trước, lời nhắc rút gọn.
- **Lỗi DWM** cho một cửa sổ: dùng `GetWindowRect`.
- **Không có trạng thái tải** (liệt kê đồng bộ, dưới 10 ms với vài trăm cửa sổ).

## 5. Kiến trúc và luồng dữ liệu

```
phím tắt vùng ─► main::request(Region)
                  ├─ snap::enumerate_windows()   (Windows: EnumWindows + DWM, tọa độ màn hình)  [chỉ khi snap_windows bật]
                  ├─ capture::capture_virtual_screen() ─► Screenshot { image, origin, monitors, … , windows }
                  └─ overlay::pick_region(shot, initial, …)
                         └─ hover/click: darkshot_core::snap::{candidates, candidate_at}
```

- **Lõi** (`darkshot-core/src/snap.rs`, thuần, kiểm thử đầy đủ):
  - `pub fn candidates(windows: &[Rect], monitors: &[Rect], bounds: Rect) -> Vec<Rect>`: cắt mỗi khung theo `bounds`, bỏ khung < `MIN_SNAP` (16) mỗi chiều, bỏ trùng liên tiếp, nối thêm màn hình (đã cắt). Giữ thứ tự.
  - `pub fn candidate_at(cands: &[Rect], p: Point) -> Option<Rect>`: ứng viên đầu tiên chứa `p`.
  - `pub const MIN_SNAP: i32 = 16; pub const MAX_WINDOWS: usize = 512;`
  - `config::Config.snap_windows: bool` (mặc định `true`, JSON `snap_windows`).
  - `i18n`: `Msg::ChkSnapWindows`, `Msg::OverlayHintSnap`, `Msg::OverlayHintDrag`; câu Trợ giúp mới.
- **Windows** (`darkshot-windows/src/snap.rs`): `pub fn enumerate_windows() -> Vec<Rect>` (tọa độ màn hình, pixel vật lý, thứ tự Z trên xuống; tối đa `MAX_WINDOWS`). Gọi trong ngữ cảnh DPI `PerMonitorV2` như `capture`. Bỏ cửa sổ của chính lớp phủ là không cần vì liệt kê trước khi lớp phủ được tạo.
- `capture::Screenshot.windows: Vec<Rect>` ở **tọa độ ảnh** (đã trừ `origin`), rỗng khi tắt. Chụp toàn màn hình không liệt kê.
- `overlay`: trạng thái `snap: Vec<Rect>` (ứng viên đã tính một lần lúc mở), `hover: Option<Rect>`; vẽ xem trước; lời nhắc; bấm/Enter chọn.

## 6. Riêng tư và bảo mật

- Chỉ đọc khung (tọa độ) của cửa sổ; **không** đọc tiêu đề, tên tiến trình, nội dung. Không lưu, không gửi đi, không ghi nhật ký. Danh sách sống trong bộ nhớ của một lần chụp.
- Không thêm đích mạng, không thêm crate. Có thể cần feature `Win32_Graphics_Dwm` của crate `windows` (đã dùng cho thanh tiêu đề tối; task kiểm tra).

## 7. Hiệu năng

- Liệt kê: một lần mỗi lần chụp vùng, trước khi đóng băng màn hình; mục tiêu < 10 ms với 300 cửa sổ. Ảnh chụp đóng băng ngay sau đó, chênh lệch không nhận thấy.
- Rê chuột: `candidate_at` O(n), n ≤ 512 + số màn hình. Chỉ vẽ lại khi ứng viên đổi, và chỉ vùng hợp của khung cũ, khung mới, nhãn cỡ và viên nhắc.
- Kích thước exe: dự kiến tăng < 20 KB.

## 8. Kiểm thử

- Lõi: `candidates` (cắt, bỏ nhỏ, bỏ trùng, giữ thứ tự, màn hình ở cuối, tọa độ âm, khung nằm ngoài hẳn), `candidate_at` (chồng lớp: lấy trên cùng; điểm ở mép; không chứa), cấu hình `snap_windows` (mặc định, sai kiểu, tệp cũ), i18n đủ hai ngôn ngữ.
- Windows: `enumerate_windows` trả khung hợp lệ và có ít nhất một cửa sổ khi chạy kiểm thử có cửa sổ thử của chính nó (tạo cửa sổ hiện, kiểm khung của nó có trong danh sách; cửa sổ ẩn thì không); hàm thuần lọc lớp `Progman`/`WorkerW`; lớp phủ: hàm thuần quyết định có gợi ý không (`snap_active`), bấm không kéo chọn ứng viên, kéo không chọn, Enter chọn, đã có vùng chọn thì không gợi ý; kiểm thử chạy tay mở lớp phủ thật không rò GDI.
- Gia cố Lịch sử: `flush` chờ việc trong hàng; `flush` hết giờ trả `false`; `finish` sau `drain` trả `false` và không chạy `f`; `drain` sau `finish` không thấy lượt; nhiều mục đẩy liên tiếp đều được ghi đúng thứ tự; tắt → không tạo luồng.
- Thử tay: mục "Đợt 8" của checklist.

## 9. Tiêu chí nghiệm thu

1. Chưa có vùng chọn: rê chuột lên một cửa sổ thì khung cửa sổ sáng lên kèm nhãn cỡ; rê ra nền thì cả màn hình sáng lên.
2. Bấm không kéo chọn đúng khung (không viền vô hình), sau đó mọi công cụ đánh dấu và Lưu/Chép/Tải lên chạy như vùng kéo tay.
3. Kéo vẫn chọn vùng tự do; có vùng chọn thì không có gợi ý.
4. Enter chọn ứng viên.
5. Tắt ô gợi ý thì lớp phủ như bản đợt 7, lời nhắc rút gọn.
6. Chữ không bị cắt ở Việt và English; DPI 100–250%.
7. Thoát ngay sau Lưu/Chép/Tải lên thì mục vẫn có trong Lịch sử; không bao giờ có hai mục cho một lượt.
8. `scripts/check.ps1` in `ALL CHECKS PASSED`.

## 10. Ngoài phạm vi

Phím tắt riêng chụp cửa sổ đang hoạt động; bắt phần tử con trong cửa sổ (nút, khung trong trình duyệt); chụp cửa sổ bị che (PrintWindow); bóng đổ cửa sổ, nền trong suốt cho góc bo; chụp có hẹn giờ; chụp cuộn; đổi ứng viên bằng phím Tab hay bánh xe.

## 11. Ảnh hưởng tới tính năng có sẵn

- Bấm không kéo khi chưa có vùng chọn: trước đây không làm gì, nay chọn ứng viên. Đây là thay đổi hành vi duy nhất.
- Vùng nhớ từ lần trước (`keep_selection`) vẫn mở sẵn; gợi ý chỉ xuất hiện sau khi bỏ vùng đó.
- Chụp toàn màn hình, đánh dấu, Lịch sử, Tải lên không đổi.
