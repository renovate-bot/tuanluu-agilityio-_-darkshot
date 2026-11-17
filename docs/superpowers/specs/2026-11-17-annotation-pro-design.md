# darkshot – Annotation Pro (đợt 6): thiết kế

Ngày: 2026-11-17. Trạng thái: thiết kế của phiên điều phối theo yêu cầu "Annotation Pro" của chủ dự án; chủ dự án cho phép tự quyết các chi tiết và chạy đến hết không hỏi lại (quyết định 75). Các thiết kế trước (đợt 5 `2026-11-17-annotate-design.md`, màu tự chọn, Làm mờ) vẫn đúng ở mọi chỗ thiết kế này không nói khác.

## 1. Mục đích

Chụp → đánh dấu → **sửa** → chia sẻ. Hôm nay người dùng chỉ hoàn tác được hình cuối; vẽ sai một hình ở giữa thì phải hoàn tác hết. Đợt này cho phép chọn, di chuyển, đổi cỡ, đổi kiểu, xóa từng hình; hoàn tác và làm lại nhiều bước; thêm hai công cụ (Hình elip, Đánh số bước) và chỉnh cỡ nét / cỡ chữ.

Đối tượng: người viết hướng dẫn, báo lỗi, tài liệu nội bộ, cần ảnh chú thích nhanh mà không mở trình sửa ảnh.

### Mục tiêu
1. Hoàn tác (Ctrl+Z) và làm lại (Ctrl+Y, Ctrl+Shift+Z) nhiều bước cho mọi thao tác: tạo, xóa, di chuyển, đổi cỡ, đổi màu, đổi cỡ nét.
2. Công cụ Chọn: chọn một hình, di chuyển, đổi cỡ bằng tay nắm, xóa bằng Delete.
3. Công cụ Hình elip (Shift: hình tròn).
4. Công cụ Đánh số bước: mỗi cú bấm đặt một vòng tròn có số 1, 2, 3…
5. Cỡ: bốn mức (Nhỏ, Vừa, Lớn, Rất lớn) áp cho độ dày nét, cỡ chữ, cỡ vòng số; đổi được cho hình đang chọn.

### Ngoài phạm vi
- Chọn nhiều hình, khung chọn kéo, đổi thứ tự trên dưới (z-order), xoay, nhân bản, chép/dán hình.
- Sửa lại nội dung chữ đã chốt (chỉ di chuyển, đổi cỡ, đổi màu).
- Di chuyển bằng phím mũi tên.
- Lưu mức cỡ vào file cấu hình (chỉ nhớ trong tiến trình, như màu nét).
- Nút Xóa trên thanh công cụ (Delete và Backspace là đủ; thanh đã dài).

### Điều không được xảy ra
- Khung chọn, tay nắm, viền tạm in vào ảnh ra.
- Vùng che (Làm mờ) để lộ điểm ảnh gốc bên trong nó sau khi di chuyển, đổi cỡ, hoàn tác, làm lại.
- Ảnh ra khác với thứ đang thấy trên màn hình.
- Không chọn công cụ nào thì lớp phủ chạy khác hôm nay.
- Lớp phủ giật khi kéo hình trên hai màn hình 4K.

## 2. Cách tiếp cận đã cân nhắc

**Lịch sử thao tác**
- (A) Chụp nguyên danh sách hình sau mỗi thao tác. Đơn giản nhưng mỗi bước chép mọi nét tự do: 100 hình × nét dài là hàng chục MB, và tốn thời gian chép khi kéo.
- (B) **Nhật ký thay đổi theo hình** (chọn): mỗi mục ghi đúng một thay đổi trên một hình: `Add` (vị trí, hình), `Remove` (vị trí, hình), `Replace` (vị trí, hình trước, hình sau). Bộ nhớ mỗi mục ≤ hai bản của một hình; khôi phục đúng vị trí trong danh sách nên thứ tự vẽ (trên dưới) giữ nguyên; màu che lưu trong hình nên hoàn tác trả đúng màu cũ.
- (C) Lệnh có tham số (dịch dx, dy; tỉ lệ). Nhỏ hơn B nhưng mỗi loại hình cần phép ngược riêng, làm tròn số thực có thể trôi sau nhiều lần hoàn tác/làm lại, và Làm mờ phải tính lại màu. Không đáng.

**Số thứ tự của Đánh số bước**
- (A) Số cố định lúc tạo (số lớn nhất + 1): xóa số 2 trong 1-2-3 để lại 1-3; dễ gây lỗ hổng và trùng sau hoàn tác.
- (B) **Đánh lại số theo thứ tự trong danh sách** (chọn): số của một vòng = 1 + số vòng đứng trước nó trong danh sách hình. Luôn liên tục 1..n, không trùng; hoàn tác/làm lại trả đúng vị trí nên trả đúng số. Số được ghi vào hình (trường `n`) và lõi đánh lại sau mỗi thay đổi, để `draw_shapes` vẽ từng hình độc lập và `burn` nhận danh sách đã đánh số.

**Chỉnh cỡ**
- (A) Thanh trượt độ dày riêng và ô cỡ chữ riêng: chiếm nhiều chỗ trên thanh dọc vốn đã dài.
- (B) **Một nút Cỡ với bốn mức dùng chung** (chọn): mở một bảng nhỏ cạnh nút (như bảng màu), phím `[` và `]` giảm/tăng. Một mức áp cho mọi loại hình theo bảng ở mục 5.6. Đổi cỡ tự do của chữ và vòng số vẫn có qua tay nắm.

## 3. Thanh công cụ dọc

Thứ tự mới (vạch ngăn ký hiệu `|`):

`Chọn | Bút vẽ, Đường thẳng, Mũi tên, Hình chữ nhật, Hình elip, Bút dạ quang, Làm mờ, Chữ, Đánh số | Cỡ, Màu | Hoàn tác, Làm lại`

14 mục, 3 vạch ngăn. Phím tắt chọn công cụ (không kèm Ctrl/Alt, như hôm nay; bấm lại để bỏ chọn): V Chọn, P, L, A, R, **E** Hình elip, H, B, T, **N** Đánh số. `[` / `]` giảm / tăng mức cỡ.

- Nút Cỡ vẽ một chấm tròn có đường kính theo mức hiện tại. Bấm mở bảng Cỡ (bốn ô ngang, mỗi ô một chấm lớn dần, ô đang dùng có viền nhấn `ACCENT`), cạnh nút, cùng quy tắc đặt chỗ và đóng như bảng màu: bấm ngoài bảng chỉ đóng bảng; Esc đóng bảng; mở bảng này thì đóng bảng màu và ngược lại.
- Hoàn tác mờ khi không có gì để hoàn tác (và không có ô chữ); Làm lại mờ khi ngăn làm lại rỗng.
- Gợi ý khi rê chuột: mọi nút mới có chữ trong `i18n` (hai ngôn ngữ), kèm phím tắt như các nút cũ.
- **Không đủ chiều cao:** nếu thanh một cột cao hơn màn hình chứa vùng chọn (trừ lề `GAP`), thanh xếp thành **hai cột** (đi theo cột: hết cột trái sang cột phải, vạch ngăn giữ nguyên vị trí tương đối; vạch ngăn nằm ở đầu cột thì bỏ). Quy tắc đặt chỗ `place_side_toolbar` không đổi, chỉ nhận cỡ mới. Ví dụ: 1920×1080 ở 200% (540 DIP) cần hai cột.

## 4. Lịch sử: hoàn tác và làm lại

- Lõi giữ `Annotations { shapes, undo: Vec<Edit>, redo: Vec<Edit> }`. Mọi thay đổi danh sách hình đi qua ba thao tác có ghi sổ: thêm (vào cuối), xóa (tại vị trí i), thay (tại vị trí i). Thay bằng hình bằng hệt hình cũ thì không ghi (bấm mà không kéo không sinh mục).
- Thao tác mới xóa sạch ngăn làm lại.
- Hoàn tác/làm lại trả về mô tả thay đổi (vị trí, hình cũ nếu có) để lớp Windows cập nhật `mark_bounds` cùng vị trí và làm mới đúng vùng cũ và mới.
- Sau mỗi thay đổi (kể cả hoàn tác/làm lại) lõi đánh lại số các vòng Đánh số; lớp Windows làm mới các vòng có số đổi.
- **Một cử chỉ là một mục:** trong lúc kéo (di chuyển, đổi cỡ), hình đang kéo là bản xem trước không nằm trong sổ; thả chuột ghi đúng một `Replace`. Esc trong lúc kéo hủy cử chỉ: hình về như cũ, không ghi. Mất chuột giữa chừng (`WM_CAPTURECHANGED`) thì chốt như thả chuột, giống nét vẽ hôm nay.
- Không giới hạn số mục (mỗi mục ≤ hai bản một hình; nét tự do 10.000 điểm ≈ 80 KB một bản). Sổ mất khi lớp phủ đóng.
- **Phím:** Ctrl+Z hoàn tác; Ctrl+Y và Ctrl+Shift+Z làm lại. Không làm gì trong lúc đang kéo.
- **Ô chữ đang mở** giữ đúng hành vi hôm nay cho Ctrl+Z: ô có chữ thì Ctrl+Z xóa ô (không ghi sổ); ô rỗng thì hủy ô rồi hoàn tác một bước. Ctrl+Y khi ô chữ mở: không làm gì. Chốt ô chữ có nội dung là một `Add`.
- Hoàn tác hoặc làm lại thì bỏ chọn hình (tránh chỉ số chọn trỏ sai hình).
- Nút Hoàn tác/Làm lại trên thanh dọc làm đúng như phím.

## 5. Chọn, di chuyển, đổi cỡ, xóa

### 5.1 Khi nào chọn được
Chỉ khi công cụ Chọn đang bật. Đổi sang công cụ khác hoặc bỏ chọn công cụ thì bỏ chọn hình. Không bật công cụ nào thì lớp phủ chạy đúng như hôm nay.

### 5.2 Bấm chuột khi công cụ Chọn bật (thứ tự ưu tiên)
1. Thanh bốn nút, thanh dọc, bảng màu/bảng Cỡ: như hôm nay.
2. Tay nắm của hình đang chọn → bắt đầu đổi cỡ.
3. Hình (trên cùng trước, tức là duyệt danh sách từ cuối) trúng điểm bấm, trong vùng chọn → chọn hình đó và bắt đầu di chuyển (cùng một cú bấm-kéo).
4. Tay nắm của vùng chọn → đổi cỡ vùng chọn như hôm nay.
5. Còn lại (trong hay ngoài vùng chọn) → bỏ chọn hình; không tạo vùng mới.

### 5.3 Trúng hình (khoảng dung sai `tol` = 4 DIP đổi theo DPI màn hình chứa điểm bấm)
- Bút vẽ, Bút dạ quang: khoảng cách tới đường gấp khúc ≤ nửa độ dày + tol.
- Đường thẳng, Mũi tên: khoảng cách tới đoạn a–b ≤ nửa độ dày + tol; mũi tên thêm tam giác đầu.
- Hình chữ nhật, Hình elip: chỉ **viền** (khoảng cách tới viền ≤ nửa độ dày + tol), để bấm vào lòng hình vẫn chọn được hình nằm bên trong.
- Làm mờ, Đánh số: cả lòng hình (đặc).
- Chữ: hộp bao đo bằng GDI+ (`mark_bounds`).

### 5.4 Hiển thị
Hình đang chọn có khung chữ nhật 1 px màu `ACCENT` quanh hộp bao (nới 2 px) và tay nắm vuông như tay nắm vùng chọn (cỡ `HANDLE_SIZE` theo DPI). Đường thẳng và Mũi tên: chỉ hai tay nắm ở hai đầu, không khung. Con trỏ: `IDC_SIZEALL` trên hình (khi công cụ Chọn bật), con trỏ đổi cỡ đúng hướng trên tay nắm. Khung và tay nắm chỉ vẽ trên màn hình, không bao giờ qua `draw_shapes`/`burn`.

### 5.5 Biến đổi theo loại hình

Tọa độ điểm chuột trong lúc kéo được **kẹp vào vùng chọn** (điểm nắm luôn nằm trong vùng nên hình không trôi mất ra ngoài). Dời = điểm chuột hiện tại − điểm bấm (số nguyên, nên hộp bao dời đúng số nguyên).

| Hình | Tay nắm | Đổi cỡ |
|---|---|---|
| Hình chữ nhật, Hình elip, Làm mờ | 8 (4 góc, 4 cạnh) của hộp a–b | Cạnh/góc đối diện đứng yên; kéo qua cạnh đối diện thì lật; mỗi chiều ≥ `MIN_EXTENT_DIP` = 6 DIP; góc + Shift giữ tỉ lệ |
| Đường thẳng, Mũi tên | 2 (hai đầu) | Kéo một đầu; Shift khóa 46° theo đầu kia; dài ≥ `MIN_DRAG_PX` |
| Bút vẽ, Bút dạ quang | 4 góc của hộp các điểm | Co giãn các điểm quanh góc đối diện (mỗi chiều riêng; Shift giữ tỉ lệ); chiều đang bằng 0 thì không co giãn chiều đó; độ dày nét giữ nguyên |
| Chữ | 4 góc của hộp bao | Phóng đều quanh góc đối diện: hệ số k = hình chiếu của (chuột − góc đối) lên (góc nắm − góc đối), chia độ dài bình phương; `origin' = neo + (origin − neo)·k`, `size' = size·k`, cỡ kẹp trong [8, 400] px |
| Đánh số | 4 góc của hộp vòng | Như Chữ: phóng đều quanh góc đối diện, bán kính kẹp trong [6, 300] px |

Di chuyển: dời mọi điểm của hình; giữ màu, độ dày, nội dung chữ, cỡ chữ, độ trong.

### 5.6 Cỡ và màu cho hình mới và hình đang chọn

Bốn mức; mức 2 (Vừa) đúng bằng giá trị hôm nay nên mặc định không đổi gì:

| Mức | Nét (Bút vẽ, Đường thẳng, Mũi tên, Hình chữ nhật, Hình elip) | Bút dạ quang | Chữ | Bán kính vòng số |
|---|---|---|---|---|
| 1 Nhỏ | 2 | 10 | 14 | 10 |
| 2 Vừa | 3 | 16 | 18 | 13 |
| 3 Lớn | 5 | 24 | 26 | 17 |
| 4 Rất lớn | 8 | 32 | 36 | 22 |

(đơn vị DIP, đổi sang px theo DPI màn hình chứa điểm đầu của hình, như hôm nay).

- Chọn mức hoặc màu luôn đặt mặc định cho hình vẽ sau đó (mức nhớ trong tiến trình như `LAST_COLOR`, không ghi cấu hình).
- Có hình đang chọn: chọn mức hoặc màu còn áp cho hình đó, ghi một `Replace`. Mức đổi sang px theo DPI màn hình chứa tâm hộp bao của hình. Làm mờ bỏ qua cả màu và mức (màu che luôn lấy từ ảnh gốc). Chữ: mức đặt `size`, giữ `origin`. Đánh số: mức đặt bán kính, giữ tâm.
- Ô chữ đang mở: màu áp cho ô như hôm nay; mức đặt cỡ chữ của ô.

### 5.7 Xóa
Delete hoặc Backspace khi có hình đang chọn (và không có ô chữ mở) → xóa, ghi `Remove`, bỏ chọn.

### 5.8 Esc (thứ tự)
Đóng bảng màu/bảng Cỡ → (ô chữ: như hôm nay) → đang kéo hình: hủy cử chỉ → có hình đang chọn: bỏ chọn → đóng lớp phủ như hôm nay.

## 6. Hai công cụ mới

**Hình elip** (`Shape::Ellipse { a, b, width, color }`): kéo như Hình chữ nhật; Shift khóa hình tròn (`constrain_square`); quá ngắn (`too_short`) thì bỏ; vẽ bằng `GdipDrawEllipse` khử răng cưa trong `draw_shapes`; hộp bao = hộp a–b nới nửa độ dày + 1.

**Đánh số** (`Shape::Step { center, radius, n, color }`): bấm trong vùng chọn đặt ngay một vòng tại điểm bấm (không cần kéo). Vòng tròn đặc màu `color`, viền mảnh màu `outline_color(color)` dày `radius × 0.12`, số ở giữa màu `outline_color(color)`, Segoe UI đậm, cỡ chữ vừa trong vòng (1 chữ số ≈ `radius × 1.1`; nhiều chữ số thì thu nhỏ để bề rộng ≤ `1.5 × radius`). Viền tương phản giữ vòng đọc được trên nền sáng lẫn tối. Hộp bao = hình vuông tâm `center`, cạnh `2·radius` nới nửa viền + 1.

## 7. Làm mờ: bảo toàn khi sửa

- Mọi lần dời hoặc đổi cỡ (cả bản xem trước khi kéo và lúc chốt) tính lại `color = redact_color(ảnh gốc, blur_pixel_rect(a', b'))` trên **ảnh chụp gốc** (`State.image`), không bao giờ trên ảnh đã vẽ; hàm biến đổi ở lõi nhận ảnh gốc.
- Vẫn tô đặc không khử răng cưa: mọi điểm ảnh trong vùng bằng đúng màu che ở cả màn hình lẫn ảnh ra.
- Hoàn tác/làm lại trả nguyên hình đã lưu (gồm màu), không tính lại.
- Không có đổi thứ tự trên dưới, nên một hình vẽ sau vùng che vẫn nằm trên nó như lúc vẽ; vùng che không bao giờ bị đưa xuống dưới hình khác.
- Màn hình hiển thị đúng thứ sẽ ra ảnh: khi dời vùng che, chỗ cũ lộ lại nội dung gốc — đó là ý người dùng và thấy được trước khi xuất.

## 8. Vẽ và hiệu năng

- Một hàm vẽ duy nhất `annotate_draw::draw_shapes` cho màn hình và `burn` (thêm hai loại hình). Khung chọn và tay nắm vẽ bằng lớp phủ sau các hình.
- Kéo hình: mỗi lần chuột di chuyển chỉ làm mới hợp của hộp bao cũ và mới của hình đang kéo (cộng tay nắm), không vẽ lại cả màn hình; không chép ảnh cỡ màn hình.
- Danh sách `mark_bounds` luôn cùng độ dài và thứ tự với danh sách hình (thêm, xóa, thay tại vị trí).
- Trúng hình: duyệt từ trên xuống, bỏ nhanh hình có hộp bao (nới tol) không chứa điểm.
- Đổi cỡ chữ khi kéo cần đo lại hộp bao; được phép ước lượng hộp xem trước bằng cách phóng hộp cũ cùng hệ số (nới thêm 2 px), đo thật lúc chốt.

## 9. Tương thích

Windows 10/11; DPI 100–200%; nhiều màn hình, tọa độ âm, DPI khác nhau giữa các màn hình (độ dày tính theo màn hình chứa điểm; tol theo màn hình chứa điểm bấm). Mọi luồng hôm nay giữ nguyên: PrintScreen, chọn/di chuyển/đổi cỡ vùng, Giữ vị trí vùng, Ctrl+D/S/C, Esc và phím Esc toàn cục, bảng màu và màu tự chọn, Làm mờ, hai phím chụp toàn màn hình, PNG/JPEG, hai ngôn ngữ. Không hình nào thì ảnh ra đúng từng byte như hôm nay.

## 10. Kiểm thử và nghiệm thu

Lõi (kiểm thử đơn vị): hoàn tác/làm lại nhiều bước; thao tác mới xóa làm lại; `Replace` bằng hình cũ không ghi; xóa hoàn tác được; vị trí khôi phục đúng thứ tự; đánh số 1..n sau thêm, xóa giữa, hoàn tác, làm lại; trúng hình từng loại và hình trên cùng thắng; tay nắm từng loại; biến đổi từng loại (lật, kích thước tối thiểu, Shift, chiều bằng 0, kẹp cỡ chữ); vùng che tính lại màu trên ảnh gốc sau dời/đổi cỡ; xếp hai cột khi thấp.

Vẽ (crate Windows): elip và vòng số khớp hộp bao; `burn` một Step và Ellipse cho ra giống `draw_shapes`; số đọc được (điểm ảnh tâm có màu chữ); không có màu `ACCENT` của khung chọn trong ảnh ra.

Lớp phủ (thông điệp giả, kiểu `live::*` hiện có): chọn → kéo → thả = một mục sổ; Esc khi kéo không ghi; Delete rồi Ctrl+Z trả hình về đúng vị trí; Ctrl+Y; công cụ khác bỏ chọn; không công cụ thì kéo tạo vùng như cũ; kiểm thử chống rò GDI vẫn qua.

Thử tay (checklist mục "Đợt 6"): mọi mục ở phần 8 của yêu cầu, kể cả 100 hình, nét tự do dài, hai màn hình 4K, DPI 100/125/150/200%.

`powershell -File scripts/check.ps1` in `ALL CHECKS PASSED`.

## 11. Chốt sau đọc soát (quyết định của phiên điều phối)

1. **Phím:** `[`/`]` là `VK_OEM_4`/`VK_OEM_6`, đi cùng nhánh phím chọn công cụ (không Ctrl/Alt, bỏ lặp phím); bàn phím khác bố cục có thể không có hai phím này, nút Cỡ vẫn đủ. Ctrl+Y và Ctrl+Shift+Z nằm ở nhánh Ctrl cạnh Ctrl+Z. Delete/Backspace nằm ở nhánh không Ctrl, chỉ khi không có ô chữ (ô chữ mở thì `text_key` nhận như hôm nay). Mọi phím mới chỉ có tác dụng khi đã có vùng chọn (như phím công cụ hôm nay).
2. **Công cụ Chọn chỉ có khi đã có vùng chọn** (thanh dọc chỉ hiện khi có vùng). `Tool::Select` là một phần tử của `Tool`; `press_mode` thêm `PressMode::Pick` cho `(Some(Select), Inside | Outside)`; `Handle` của vùng vẫn là `Selection`. Lớp phủ xử lý `Pick` theo thứ tự 5.2 (tay nắm hình đang chọn được xét **trước** `press_mode`, vì tay nắm có thể nằm trên tay nắm vùng).
3. **Hộp bao của Chữ** do lớp Windows đo: các hàm lõi trúng hình, tay nắm, biến đổi nhận `bounds: Rect` (hoặc lát `&[Rect]` song song với danh sách hình) từ `mark_bounds`; lõi không đo chữ.
4. **`n` của Đánh số là dẫn xuất:** lõi đánh lại sau mỗi lần áp thay đổi (thêm, xóa, thay, hoàn tác, làm lại). Bản hình lưu trong sổ có thể mang `n` cũ; không sao vì luôn đánh lại sau khi áp. So sánh "thay bằng hình bằng hệt" làm sau khi đánh lại hình mới theo vị trí của nó (thay không đổi vị trí nên `n` bằng nhau).
5. **API `Annotations` được viết lại** (bỏ `undo() -> Option<Shape>` cũ), cập nhật mọi chỗ gọi và kiểm thử cũ. Mô tả thay đổi trả cho lớp Windows: vị trí, loại (thêm, xóa, thay), hình cũ khi có, và khoảng vị trí các vòng số đổi `n`.
6. **Đơn vị:** hàm lõi nhận dung sai, cỡ tay nắm, kích thước tối thiểu bằng px; lớp Windows đổi DIP→px theo DPI màn hình chứa điểm bấm. Hàm áp mức cỡ cho hình nhận `dpi` làm tham số.
7. **Hai cột:** bố cục thanh dọc (cỡ, ô của mục thứ i, vạch ngăn, mục dưới điểm) thành một hàm thuần ở lõi nhận số cột; `place_side_toolbar` không đổi, nhận cỡ (rộng 1 hoặc 2 nút). Lớp phủ chọn số cột theo chiều cao màn hình chứa vùng chọn.
8. **Esc** vẫn qua phím tắt Esc toàn cục (và `WM_KEYDOWN` khi ô chữ mở) tới `escape()`; `escape()` thêm hai bậc hủy kéo hình và bỏ chọn theo 5.8. Hủy kéo phải nhả chuột (`ReleaseCapture`).

## 12. Để lại cho người làm
Tên hàm và chữ ký cụ thể (kế hoạch chốt), hình icon các nút mới (đường nét theo kiểu icon hiện có), cách ước lượng hộp xem trước khi đổi cỡ chữ, thuật toán khoảng cách tới viền elip (gần đúng chấp nhận được nếu sai số < 1 px trong tol).
