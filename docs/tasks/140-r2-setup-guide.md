# Task 140: Hướng dẫn đăng ký Cloudflare R2 và cấu hình darkshot (có hình)

**Agent:** `scaffolder` · **Model:** sonnet (tài liệu + hình SVG + một kiểm thử ảnh `#[ignore]`, rủi ro thấp) · **Nhánh:** `task/140-r2-setup-guide` (từ `main`), worktree `D:\2026 - Tuan\shotme-140` · **Rà soát:** `reviewer` (sonnet; không dùng fable, quyết định 154)

## Yêu cầu của chủ dự án (2026-11-23)

"Tạo một markdown file step by step hướng dẫn đăng ký R2 storage trên Cloudflare có hình ảnh minh họa. Hướng dẫn config R2 cho darkshot và gắn vào README file."

## Cần đọc

- `README.md`, mục "Chuẩn bị Cloudflare R2", "Kiểm tra quy tắc xóa", "Hạn của link", "Tùy chọn" (nguồn sự thật hiện có; hướng dẫn mới phải khớp).
- Tài liệu Cloudflare (đọc trực tiếp bằng công cụ đọc web, không trả lời theo trí nhớ): https://developers.cloudflare.com/r2/get-started/ , https://developers.cloudflare.com/r2/pricing/ , https://developers.cloudflare.com/r2/buckets/create-buckets/ , https://developers.cloudflare.com/r2/buckets/public-buckets/ , https://developers.cloudflare.com/r2/api/tokens/ , https://developers.cloudflare.com/r2/buckets/object-lifecycles/ . Chỉ đọc trang công khai; không gửi dữ liệu dự án đi đâu.
- `crates/darkshot-windows/src/settings.rs`: kiểm thử `webshot_options` (~dòng 4780) và `crates/darkshot-windows/src/webshot.rs`; cách chọn tab R2 trong mã (tìm hàm/hằng chọn tab).
- `crates/darkshot-core/src/i18n.rs`: nhãn thật của tab Lưu trữ R2 (VI) để hướng dẫn dùng đúng chữ.

## Files

- Tạo `docs/r2-setup.md` (tiếng Việt).
- Tạo `docs/img/r2/*.svg` (hình minh họa trang Cloudflare) và `docs/img/r2/options-r2-vi.png` (ảnh thật tab Lưu trữ R2 của darkshot).
- Sửa `crates/darkshot-windows/src/settings.rs` (chỉ thêm kiểm thử `#[ignore]`).
- Sửa `README.md` (gắn link).
- `memory-bank/decisions.md` (một quyết định mới, số kế tiếp sau mục cuối, ngày 2026-11-23).

## Steps

1. **Nội dung `docs/r2-setup.md`** — từng bước đánh số, mỗi bước: việc cần làm, nút/mục cần bấm (in đậm, tên tiếng Anh như trên Cloudflare), kết quả mong đợi, một hình. Các phần:
   1. Trước khi bắt đầu: cần gì (email, phương thức thanh toán nếu Cloudflare yêu cầu khi bật R2 — xác minh trên trang pricing/get-started; ghi hạn mức miễn phí đúng như trang pricing nói và ngày đối chiếu 2026-11-23; nói rõ có thể thay đổi).
   2. Đăng ký tài khoản Cloudflare (dash.cloudflare.com/sign-up, xác minh email).
   3. Bật R2 (mục **R2 object storage**, đăng ký gói R2 nếu được hỏi).
   4. Tạo bucket (quy tắc tên như README; gợi ý vị trí Automatic).
   5. Bật truy cập công khai (tên miền riêng hoặc r2.dev), lấy **Địa chỉ công khai**.
   6. Tạo API token **Object Read & Write** chỉ cho bucket đó; lưu Access Key ID và Secret Access Key ngay (chỉ hiện một lần). Ghi Account ID.
   7. Tạo ba quy tắc vòng đời `d1/` 1 ngày, `d7/` 7 ngày, `d30/` 30 ngày; không tạo cho `keep/`.
   8. Cấu hình darkshot: mở Tùy chọn → tab Lưu trữ R2, bảng ánh xạ "giá trị trên Cloudflare → ô trong darkshot" (Account ID, Access Key ID, Secret Access Key, Tên kho, Địa chỉ công khai, Hạn của link), bấm **Kiểm tra kết nối**, chờ thông báo thành công đúng chữ trong i18n, bấm **OK**. Kèm ảnh thật tab R2.
   9. Thử: chụp vùng, `Ctrl+D`, link dạng `{địa chỉ công khai}/d30/{16 ký tự}.png`.
   10. Xử lý sự cố: bảng lỗi thường gặp ↔ cách sửa (chỉ những lỗi/thông báo có thật trong mã hoặc README).
   11. Bảo mật: khóa lưu trên máy (mã hóa như README nói), không chia sẻ khóa, thu hồi token khi lộ; ứng dụng không xóa được ảnh đã tải lên.
   12. Tài liệu tham khảo (link Cloudflare) và dòng "đối chiếu ngày 2026-11-23, giao diện Cloudflare có thể đổi tên nút".
   Chỉ viết điều có thật trong mã, README và tài liệu Cloudflare đã đọc. Chỗ Cloudflare và README hiện tại khác nhau: theo tài liệu Cloudflare và sửa luôn README cho khớp, ghi trong báo cáo.
2. **Hình minh họa Cloudflare (`docs/img/r2/01-…svg` đến `~07-…svg`)**: SVG viết tay, giản lược dạng khung dây (wireframe) theo tông tối, chữ dễ đọc, khoanh/đánh số chỗ cần bấm bằng màu nhấn `#4C8DFF`. KHÔNG dùng logo, biểu tượng hay màu nhận diện của Cloudflare, không chép ảnh chụp trang của họ; mỗi hình có dòng nhỏ "Hình minh họa giản lược, không phải ảnh chụp giao diện Cloudflare". Giá trị trong hình là mẫu rõ ràng (Account ID `1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d`, bucket `darkshot-images`, `https://img.example.com`, khóa dạng `xxxxxxxx…`). Mỗi SVG hợp lệ (mở được bằng trình duyệt, có `viewBox`, `width`/`height`), không script, không tham chiếu ngoài, mỗi file < 20 KB.
3. **Ảnh thật tab Lưu trữ R2:** thêm kiểm thử `#[ignore]` `webshot_options_r2` cạnh `webshot_options`: mở Tùy chọn với cấu hình giả (giá trị mẫu như trên, KHÔNG có khóa thật, thư mục lưu `C:/Screenshots`, ngôn ngữ Vi), chọn tab Lưu trữ R2 bằng mã (không chuột/phím thật), PrintWindow, lưu `options-r2-vi`. Chạy: `$env:DARKSHOT_WEBSHOTS="target\webshots"; cargo test -p darkshot-windows -- --ignored --nocapture webshot_options_r2`. Chép PNG vào `docs/img/r2/options-r2-vi.png` (giảm cỡ nếu > 150 KB bằng công cụ có sẵn; không cài gì mới). Kiểm tra bằng mắt (Read ảnh) rằng không có dữ liệu cá nhân. Nếu không làm được, dừng và báo; không thay bằng hình vẽ.
4. **README:** ở đầu mục "Chuẩn bị Cloudflare R2" thêm một dòng nổi bật trỏ tới `docs/r2-setup.md` (hướng dẫn từng bước có hình); thêm link tương tự ở bước 3 của mục cài đặt (dòng "điền thông tin R2"). Giữ nội dung README hiện có (chỉ sửa chỗ sai so với Cloudflare).
5. Quyết định mới trong `memory-bank/decisions.md` (hướng dẫn R2 có hình; hình Cloudflare là SVG giản lược không logo/nhận diện vì repo công khai và giao diện Cloudflare đổi; ảnh darkshot là ảnh thật từ `webshot_options_r2`).
6. Không đổi chữ người dùng thấy trong app, không sửa website (thay đổi chỉ là tài liệu). `cargo fmt --all`; `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`. Commit `docs:` / `test:` trên nhánh, không gộp, không đẩy. Báo cáo ngắn: file, chỗ README khác Cloudflare đã sửa, kết quả check, token nếu biết.

## Ràng buộc

Không yêu cầu, đọc hay ghi khóa R2 thật; không đọc `%APPDATA%\darkshot\config.json`. Repo công khai: không dữ liệu cá nhân trong ảnh. Không giả lập chuột/phím thật (chủ dự án đang dùng máy). Không dùng mã từ `prototype/`.

## Bổ sung 2026-11-23: bản tiếng Anh

Chủ dự án yêu cầu thêm bản tiếng Anh:

1. `docs/r2-setup.en.md`: cùng cấu trúc và nội dung với `docs/r2-setup.md`, tiếng Anh tự nhiên; tên ô/nút darkshot và thông báo lấy đúng chữ English trong `darkshot_core::i18n` (tab "R2 storage", "Success: the upload worked and the link opens."). Đầu mỗi file có dòng "Tiếng Việt | English" trỏ qua lại.
2. Hình SVG Cloudflare dùng chung hai bản: chỉ chữ giao diện Cloudflare (tiếng Anh) và số chú thích; ghi chú trong SVG chỉ tiếng Anh ("Simplified illustration, not a Cloudflare screenshot"); câu chú thích tiếng Việt nằm trong markdown.
3. `webshot_options_r2` xuất cả hai ngôn ngữ (`each_lang`): `docs/img/r2/options-r2-vi.png` và `docs/img/r2/options-r2-en.png`.
4. README (tiếng Việt) trỏ tới bản tiếng Việt, kèm "(English: docs/r2-setup.en.md)".
