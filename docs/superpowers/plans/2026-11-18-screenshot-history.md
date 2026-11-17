# Kế hoạch đợt 7: Lịch sử ảnh chụp

Thiết kế: `docs/superpowers/specs/2026-11-18-screenshot-history-design.md` (đọc mục 3–7 khi cần). Làm trên nhánh phát triển `feature/screenshot-history` (tách từ `main` tại `afaea38`). Mỗi task có nhánh `task/NN-...` tách từ nhánh phát triển và gộp lại vào nhánh phát triển, KHÔNG gộp vào `main`, KHÔNG đẩy lên `origin` cho đến khi chủ dự án duyệt demo (quyết định 86).

## Global Constraints

- **Phụ thuộc:** không thêm crate, không bật feature mới của `windows` trừ khi gói task nêu rõ (cần: `Win32_UI_Shell` đã có). `darkshot-core` không phụ thuộc hệ điều hành và không phụ thuộc `windows`.
- **Chữ hiển thị:** mọi chữ người dùng thấy từ `darkshot_core::i18n` (Việt + English). `scripts/check.ps1` cấm chuỗi tiếng Việt cứng.
- **Màu:** từ `darkshot_core::theme`. Giao diện luôn tối.
- **Lịch sử không được làm hỏng hành động chụp:** mọi lỗi ghi lịch sử bị nuốt; không bao giờ `unwrap`/`expect`/chỉ số có thể panic trên đường ghi lịch sử.
- **Riêng tư:** mọi dữ liệu lịch sử cục bộ dưới `%LOCALAPPDATA%\darkshot\history\`. Không mạng. Không ghi nhật ký ảnh, đường dẫn đầy đủ, khóa R2. Không đọc hay ghi khóa R2 thật; kiểm thử dùng thư mục tạm.
- **Xóa an toàn:** chỉ xóa tệp trong `history\images` và `history\thumbs` do chính ứng dụng tạo (kiểm tra tên `<id>.png` và đường dẫn nằm trong thư mục gốc lịch sử). Không bao giờ xóa tệp của người dùng, không đụng R2.
- **Luồng:** không mã hóa/giải mã ảnh, không I/O tệp lớn trên luồng giao diện; không giữ khóa giao diện khi làm I/O.
- **An toàn:** mỗi khối `unsafe` có `// SAFETY:`; giải phóng mọi đối tượng GDI; kiểm thử chống rò phải qua.
- **Không sửa:** spec/kế hoạch đã duyệt, gói task cũ, `prototype/`, dòng cũ của `decisions.md`.
- **Quy trình:** commit `test:` trước rồi `feat:` rồi `docs:`; `git add` từng file, `git commit` là lệnh riêng; `cargo fmt --all`; `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`; không chuyển nhánh, không đẩy; build release dùng `--target-dir target/task<NN>`; không tắt tiến trình `darkshot` không do mình mở; kiểm thử `#[ignore]` chạy từng cái với `--test-threads=1`.

## Bảng task

| Task | Nhánh | Agent | Model | Nội dung | Phụ thuộc |
|---|---|---|---|---|---|
| 52 | task/52-history-model | builder | sonnet | Mô hình, JSON chỉ mục, tìm/lọc, định dạng, bảng chuỗi i18n | – |
| 53 | task/53-history-store | specialist | opus | Kho trên đĩa: ghi nguyên tử, khóa, giữ 50, ảnh nhỏ, xóa, ghi đồng thời | 52 |
| 54 | task/54-history-integration | specialist | opus | `history_enabled`, đường dẫn, dịch vụ ghi nền, nối vào Lưu/Chép/Tải lên/dự phòng, chế độ cách ly, `--history` | 53 |
| 55 | task/55-history-entry-points | builder | sonnet | Hộp tích trong Tùy chọn, mục menu khay, mở cửa sổ (khung rỗng), đóng khi đổi ngôn ngữ | 54 |
| 56 | task/56-history-window | specialist | opus | Cửa sổ: bố cục, vẽ danh sách, ảnh nhỏ, xem trước, cuộn, bàn phím | 55 |
| 57 | task/57-history-actions | specialist | opus | Tìm, lọc, hành động, menu chuột phải, trạng thái đặc biệt, xác nhận xóa hết | 56 |
| 58 | task/58-history-docs | phiên điều phối | opus | Đồng bộ tài liệu, checklist, demo, ảnh chụp, báo cáo | 57 |

Rà soát: `reviewer` cho 52, 55; `specialist` cho 53, 54, 56, 57. Sau 57: rà soát toàn đợt bằng `specialist`.

## Review Focus (mọi task)

1. Tệp người dùng và R2 không bao giờ bị xóa; xóa tệp chỉ trong thư mục lịch sử.
2. Lịch sử tắt thì không có tác dụng phụ nào; lịch sử lỗi không làm hỏng hành động.
3. Hai kết quả ghi đồng thời không mất mục, không trùng, chỉ mục không hỏng; tệp chỉ mục hỏng không làm sập.
4. Không mã hóa/giải mã trên luồng giao diện.
5. Chữ qua i18n, đủ hai ngôn ngữ, không bị cắt (Việt dài hơn English khoảng 20%).
6. Không rò GDI; DPI 100–250%.
