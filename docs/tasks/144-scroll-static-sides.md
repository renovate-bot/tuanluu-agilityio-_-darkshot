# Task 144: Chụp cuộn bỏ cột đứng yên hai bên

**Agent:** `specialist` · **Model:** opus (thuật toán ghép: sai thì sai âm thầm) · **Nhánh:** `task/144-scroll-static-sides` (từ `main`), worktree `D:\2026 - Tuan\shotme-144` · **Rà soát:** `reviewer` opus (không dùng fable, quyết định 154) · **Tối đa hai vòng sửa sau rà soát.** Vẫn chưa đạt thì dừng và báo lại, không vá thêm.

## Bối cảnh

Chủ dự án chụp cuộn một vùng rộng trên trang tin. Mọi khung sau khung đầu đều bị bỏ với `Uncertain(NoOverlap)`. Đo trên khung thật (cỡ 2874×1634 và 2870×1407) cho ba nguyên nhân:

1. **Chính:** ảnh nền hoặc quảng cáo cố định ở hai bên. Các cột này giống hệt nhau ở mọi khung, nên hàng của khung mới không khớp với hàng đã cuộn.
2. Thanh dính chỉ rộng một phần vùng chụp.
3. Nội dung động (video, quảng cáo chạy).

Task 143 thử ghép theo cột để xử lý cả ba nguyên nhân. Sau bốn vòng sửa, fuzz vẫn tìm ra trường hợp mất nội dung không cảnh báo. Chủ dự án quyết dừng hẳn hướng đó (quyết định 161). Nhánh `task/143-scroll-column-stitch` chỉ còn để tra cứu, không gộp và không chép thuật toán từ đó.

Task này chỉ xử lý nguyên nhân 1, theo cách nhỏ và dễ chứng minh. Nguyên nhân 2 và 3 vẫn làm khung bị bỏ như hiện nay. Task 145 sẽ đổi lời nhắc cho các trường hợp đó.

## Cách làm (chủ dự án đã duyệt hướng)

Đọc kỹ toàn bộ `crates/darkshot-core/src/scroll_stitch.rs`, gồm kiểm thử và fuzz, cùng quyết định 129 trong `memory-bank/decisions.md`.

1. **Đo cột đứng yên.**
   - `Stitcher` giữ khung đầu, hoặc đủ dữ liệu của khung đầu để so.
   - Mỗi lần `push` một khung hợp lệ:
     - `L` là số cột liền nhau tính từ mép trái mà mọi điểm ảnh (B, G, R, mọi hàng) giống khung đầu.
     - `R` tính tương tự từ mép phải của phần chính. Dải phải `right_strip` (chỗ thanh cuộn, con trượt di chuyển) không tính và không chặn việc đo.
   - `L` và `R` là số nhỏ nhất qua mọi khung đã nhận, kể cả khung bị bỏ. Chúng chỉ giảm, không bao giờ tăng.
   - Ý nghĩa: cột bị coi là đứng yên thì đã giống hệt khung đầu ở **mọi** khung, nên không thể chứa nội dung đang cuộn.
2. **So khớp trên phần giữa.**
   - Khi `L + R > 0`, băm hàng và mọi phép so của đường hiện có dùng cột `[L, W − strip − R)` thay cho `[0, W − strip)`. Các phép so gồm `confirm`, `rows_equal_bgr`, `uniform`, `static_bands`, `repeats`, `other_shift_on_full_frame`, `moved_with`.
   - Khung trước phải được băm lại theo cùng khoảng cột khi `L`/`R` đổi, vì hai khung phải so trên cùng khoảng.
   - Mọi ngưỡng, luật và lý do giữ nguyên. Khi `L = R = 0`, kết quả phải giống hệt `main` từng bit.
3. **Phần giữa tối thiểu.** Phần giữa phải rộng ít nhất một ngưỡng có tên, có chú thích tiếng Việt. Gợi ý: lớn hơn của 128 px và 1/4 bề rộng phần chính. Hẹp hơn ngưỡng thì không bỏ cột nào và so trên cả hàng như cũ.
4. **Cắt khi `finish`.**
   - Ảnh ra chỉ giữ cột `[L, W − R')`, với `L`, `R` là giá trị cuối.
   - Khi `R > 0`, `R'` gồm cả dải thanh cuộn, nên ảnh bỏ luôn thanh cuộn.
   - Khi `R = 0`, ảnh giữ hết bề rộng bên phải như hiện nay.
   - `body` cứ giữ đủ bề rộng trong suốt phiên, chỉ cắt lúc `finish`. Không cấp gấp đôi bộ nhớ: cắt tại chỗ hoặc theo từng hàng, giữ giới hạn `MAX_BYTES`.
5. **`L`/`R` giảm giữa phiên** (ví dụ quảng cáo bên đổi hình): các lần nối trước đó vẫn đúng, vì lúc so, các cột ấy giống khung đầu. Lần so kế tiếp dùng khoảng rộng hơn và có thể thất bại; khi đó khung bị bỏ và có báo như hiện nay. Không thêm luật nào khác.
6. Không thêm `Reason`, không đổi API công khai, trừ khi thật cần. Nếu đổi thì ghi rõ trong báo cáo.

## Kiểm thử bắt buộc

Kiểm thử tổng hợp, không dùng khung thật:

- Trang có cột ảnh cố định không đều màu ở hai bên. Phải có một kiểm thử ghi rõ rằng `main` cũ thất bại với trang này, ví dụ bằng chú thích kèm số đo. Bản mới phải ghép đúng từng điểm ảnh so với ảnh mong đợi đã cắt.
- Có dải thanh cuộn với con trượt di chuyển ở mép phải, cạnh cột quảng cáo phải.
- Cột bên đổi giữa phiên, làm `L` hoặc `R` giảm: không mất nội dung âm thầm.
- Cột bên chỉ giống nhau ở một phần số khung: không bị cắt.
- Lề trắng đứng yên hai bên trên trang thường: bị cắt, ảnh vẫn đúng.
- Phần giữa hẹp hơn ngưỡng: không cắt.
- Mọi kiểm thử và fuzz hiện có phải qua mà không đổi kỳ vọng. Ghi số nối và số sai trước và sau.
- Thêm fuzz có cột cố định hai bên, độ rộng và vị trí ngẫu nhiên: 0 lần sai âm thầm, chạy 2000 seed qua `DARKSHOT_FUZZ_SEEDS` nếu fuzz hiện có hỗ trợ.
- Thời gian `push` ở 2874×1634, bản release, ảnh tổng hợp: ghi số khi có và khi không có cột đứng yên. Mục tiêu không chậm hơn `main` quá 30 %.
- Kiểm thử `#[ignore]` `replay_real_frames`:
  - đọc các tệp `*.raw` trong thư mục `DARKSHOT_SCROLL_REPLAY`, theo thứ tự tên; định dạng: width u32 LE, height u32 LE, rồi BGRA;
  - in số khung `Appended`/`NoChange`/`Uncertain(lý do)`, `L`, `R`, cỡ ảnh ra;
  - nếu có biến `DARKSHOT_SCROLL_REPLAY_OUT` thì ghi ảnh ra cùng định dạng thô.

  Được chép phần đọc/ghi tệp của kiểm thử cùng tên trên nhánh `task/143-scroll-column-stitch`. Builder không có khung thật; phiên điều phối sẽ chạy.

## Nơi khác cần xem

- `crates/darkshot-windows/src/scroll_result.rs` và chỗ nhận `(Image, uncertain)` trong `main.rs`: ảnh có thể hẹp hơn vùng chụp. Kiểm rằng không có chỗ nào giả định ảnh rộng bằng vùng.
- Tài liệu:
  - README mục Chụp cuộn: một câu rằng cột đứng yên hai bên được bỏ khỏi ảnh;
  - `CHANGELOG.md` "Chưa phát hành" → "Sửa lỗi";
  - `docs/testing/manual-checklist.md`: một dòng;
  - `memory-bank/decisions.md`: quyết định 162, gồm luật, hằng số, số đo và giới hạn còn lại.
- Website: tìm câu về chụp cuộn trong `website/index.html` và `website/src/vi.json`. Chỉ sửa nếu câu đó bị ảnh hưởng; nếu sửa thì chạy `npm ci` và `npm run build`.

## Ràng buộc

- Không crate mới. Không panic, không chỉ số tràn.
- Repo công khai: không đưa khung thật, ảnh chụp thật hay tên nhãn hàng quảng cáo vào repo.
- Không giả lập chuột hay phím thật.
- Không sửa `scroll_session.rs` hay chữ i18n: Task 145 đang làm song song ở đó.
- `cargo fmt --all`; `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.
- Commit `fix:`/`test:`/`docs:` trên nhánh. Không gộp, không đẩy. Không dùng `fable`.

## Báo cáo

Báo cáo gồm:

- thay đổi;
- số đo thật;
- giới hạn còn lại;
- điều chưa kiểm được;
- token nếu biết.
