# Task 148: Tùy chọn bỏ khoảng trống thừa, kiểm bố cục ở mọi DPI

**Agent:** `specialist` · **Model:** sonnet (giao diện Win32 và DPI, không đụng dữ liệu) · **Nhánh:** `task/148-options-spacing` (từ `main`), thư mục `D:\2026 - Tuan\shotme` · **Rà soát:** `reviewer` opus · **Tối đa hai vòng sửa sau rà soát.** Vẫn chưa đạt thì dừng và báo lại.

## Bối cảnh

CHANGELOG 0.6.1 "Đã biết" ghi hai mục:

- Cửa sổ Tùy chọn còn khoảng trống thừa dưới mô tả Lịch sử (tab Thông thường) và phía trên các nút.
- Khoảng cách dải tab (quyết định 157) chỉ được đo ở DPI 150 %; chưa kiểm ở 100 %, 125 % và 200 %.

Mã ở `crates/darkshot-windows/src/settings.rs`:

- `build_general`, `general_layout`, `note_height_dip`: hiện đo mô tả ở mọi ngôn ngữ rồi lấy chiều cao lớn nhất.
- `Layout`, `widen_for_tabs`, `TAB_PAD_X`.
- Kiểm thử `#[ignore]` `options_tab_gaps`, `webshot_options`, `webshot_options_r2`.

## Yêu cầu

1. **Tìm nguyên nhân trước khi sửa.** Đo bằng số trên cửa sổ thật của tiến trình kiểm thử (PrintWindow hoặc `GetWindowRect` từng điều khiển, như `webshot_options`):
   - khoảng trống dưới mô tả Lịch sử;
   - khoảng trống trên hàng nút.

   Ghi số ở cả hai ngôn ngữ và nói rõ khoảng trống đến từ đâu. Ví dụ: chiều cao mô tả tính cho ngôn ngữ dài hơn, dự phòng `unwrap_or`, chiều cao cửa sổ lấy theo tab cao nhất, hằng số khe.
2. **Sửa để khoảng cách đều.**
   - Khe giữa các nhóm trong tab Thông thường bằng nhau.
   - Khoảng từ điều khiển cuối của tab cao nhất tới hàng nút bằng lề chuẩn đang dùng ở chỗ khác.
   - Không cắt chữ ở ngôn ngữ dài hơn.
   - Nếu cần thì tính chiều cao theo ngôn ngữ đang hiện, và dựng lại bố cục khi đổi ngôn ngữ. Xem cách đổi ngôn ngữ hiện làm gì trước khi chọn.
   - Không đổi thứ tự điều khiển, thứ tự Tab hay hành vi lưu.
3. **Kiểm ở mọi DPI không cần màn hình thật nếu được.** Thêm một kiểm thử thường (không `#[ignore]`) chạy ở DPI 96, 120, 144 và 192, hai ngôn ngữ. Kiểm thử này tính bố cục bằng phông tạo theo từng DPI và khẳng định:
   - khoảng trống giữa chữ hai tab liền kề ≥ `MIN_TAB_GAP_DIP`;
   - dải tab vừa một hàng;
   - không điều khiển nào chồng lên điều khiển khác hay tràn khỏi vùng client;
   - các khe dọc theo đúng luật mục 2.

   Nếu một phần chỉ đo được trên cửa sổ thật thì để phần đó là `#[ignore]` và ghi rõ lý do.
4. **Ảnh website.** Nếu bố cục đổi thì chụp lại `website/public/img/options-*.webp` bằng `webshot_options`, làm như Task 139 (xem `docs/tasks/139-options-tab-spacing.md`). Chụp lại cả ảnh `docs/img/r2/options-r2-{vi,en}.png` bằng `webshot_options_r2` nếu tab R2 đổi. Hồ sơ tạm, không dữ liệu thật. Rồi chạy `npm ci` và `npm run build` trong `website/`.
5. **Tài liệu.**
   - `CHANGELOG.md`, mục "Chưa phát hành" → "Sửa lỗi": một dòng.
   - `docs/testing/manual-checklist.md`: một dòng.
   - `memory-bank/decisions.md`: quyết định 166, gồm nguyên nhân, số đo trước và sau, luật khe.

## Ràng buộc

- Không crate mới. Không panic.
- Mọi chữ người dùng thấy lấy từ `darkshot_core::i18n`.
- Mọi lệnh đọc/ghi tệp dùng **đường dẫn tuyệt đối** trong `D:\2026 - Tuan\shotme`.
- Không gọi `python`: alias Windows Store treo.
- Nếu `target\release\darkshot.exe` bị khóa thì build bằng `--target-dir target/test-build`. Không tắt tiến trình darkshot của chủ dự án.
- Không giả lập chuột hay phím thật: chủ dự án đang dùng máy.
- Repo công khai: ảnh chỉ từ hồ sơ tạm, không khóa, không dữ liệu cá nhân.
- `cargo fmt --all`; `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.
- Commit `fix:`/`test:`/`docs:` trên nhánh, kết thúc bằng dòng `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Không gộp, không đẩy. Không dùng `fable`.

## Báo cáo

Báo cáo gồm:

- nguyên nhân;
- số đo trước và sau, ở mỗi ngôn ngữ và mỗi DPI đã kiểm;
- thay đổi;
- ảnh đã chụp lại;
- điều chưa kiểm được;
- token nếu biết.
