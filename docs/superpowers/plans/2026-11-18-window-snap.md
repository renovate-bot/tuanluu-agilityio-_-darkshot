# Kế hoạch đợt 8: Gia cố Lịch sử và Bắt cửa sổ

Thiết kế: `docs/superpowers/specs/2026-11-18-window-snap-design.md`. Nhánh phát triển `feature/window-snap` (tách từ `origin/main` tại `2797bb9`). Mỗi task có nhánh `task/NN-...` tách từ nhánh phát triển và gộp lại vào đó (`--no-ff`). KHÔNG gộp vào `main`, không tag, không phát hành cho đến khi chủ dự án duyệt demo. Được đẩy nhánh phát triển và nhánh task lên `origin` sau khi kiểm diff (repo công khai).

## Global Constraints

- **Phụ thuộc:** không thêm crate. Feature mới của `windows` chỉ khi gói task nêu (Task 61 có thể cần `Win32_Graphics_Dwm`; kiểm `Cargo.toml` trước, có rồi thì thôi). `darkshot-core` không phụ thuộc hệ điều hành và không phụ thuộc `windows`.
- **Chữ hiển thị:** mọi chữ người dùng thấy từ `darkshot_core::i18n` (Việt + English). `scripts/check.ps1` cấm chuỗi tiếng Việt cứng. Việt dài hơn English khoảng 20%: không được cắt chữ.
- **Màu:** từ `darkshot_core::theme`. Giao diện luôn tối.
- **Lịch sử không được làm hỏng hành động chụp:** mọi lỗi ghi lịch sử bị nuốt; không `unwrap`/`expect`/chỉ số có thể panic trên đường ghi lịch sử hay đường chụp.
- **Riêng tư:** không đọc tiêu đề/tiến trình của cửa sổ; không ghi nhật ký ảnh, đường dẫn đầy đủ, khóa R2; không mạng mới. Kiểm thử dùng thư mục tạm.
- **Luồng:** không I/O tệp lớn hay mã hóa ảnh trên luồng giao diện; luồng nền chỉ báo về bằng `PostMessage`.
- **An toàn:** mỗi khối `unsafe` có `// SAFETY:`; giải phóng mọi handle/GDI trên mọi đường đi; kiểm thử chống rò phải qua.
- **Không sửa:** spec/kế hoạch đã duyệt, gói task cũ, `prototype/`, dòng cũ của `decisions.md`.
- **Quy trình:** TDD: commit `test:` (kiểm thử thất bại) trước rồi `feat:`/`fix:`; `git add` từng file, `git commit` là lệnh riêng; `cargo fmt --all`; `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`; không chuyển nhánh, không gộp, không đẩy; build release dùng `--target-dir target/task<NN>`; không tắt tiến trình `darkshot` không do mình mở; kiểm thử `#[ignore]` chạy từng cái với `--test-threads=1`; không sửa kiểm thử cho khớp mã, không `#[ignore]` để né lỗi.

## Bảng task và phân bổ model

Model là lựa chọn của phiên điều phối cho từng task (CLAUDE.md, mục "Agent và model"), truyền qua tham số `model` của công cụ Agent; model trong `.claude/agents/*.md` chỉ là mặc định.

| Task | Nhánh | Agent | Model | Lý do chọn model | Nội dung | Phụ thuộc |
|---|---|---|---|---|---|---|
| 59 | task/59-history-hardening | specialist | opus | Đồng thời, vòng đời tiến trình, toàn vẹn dữ liệu: cấm model nhẹ | Hàng đợi ghi một luồng + `flush`, `PendingUploads::finish`, thanh cuộn vô hiệu khi thẻ mở, `sync_all` | – |
| 60 | task/60-snap-core | builder | sonnet | Logic thuần có đặc tả rõ, kiểm thử đơn vị; không đụng Win32 | `darkshot_core::snap`, `Config.snap_windows`, chuỗi i18n, câu Trợ giúp | 59 (chỉ thứ tự gộp) |
| 61 | task/61-snap-enumerate | specialist | opus | Win32 + DWM + DPI + lọc cửa sổ hệ thống; lỗi ở đây làm sai mọi khung | `darkshot-windows/src/snap.rs`, `Screenshot.windows`, nối vào `main::request` | 60 |
| 62 | task/62-snap-overlay | specialist | opus | Lớp phủ 8.000 dòng, máy trạng thái chuột/phím, vẽ lại từng phần, GDI | Xem trước ứng viên, bấm/Enter chọn, lời nhắc, con trỏ | 61 |
| 63 | task/63-snap-options | builder | sonnet | Mẫu "thêm một ô" đã có trong `systemPatterns.md`; việc cơ học có kiểm thử | Ô tích trong tab Chụp, giữ/lưu `snap_windows`, bố cục không cắt chữ | 62 |
| 64 | task/64-docs-demo | phiên điều phối | opus | Tài liệu tổng hợp nhiều nguồn, ảnh chụp thật, demo | README, checklist, memory bank, demo, ảnh | 63 |

Rà soát (agent độc lập với agent làm): 59, 61, 62 → `specialist` model opus; 60, 63 → `reviewer` model sonnet. Sau Task 63: rà soát toàn đợt bằng `specialist` model **fable** (model mạnh nhất hiện có, khác model của các agent làm), tập trung đồng thời, Win32, hồi quy lớp phủ.

Đánh giá lại model khi một task vượt dự kiến: ba vòng sửa chưa qua rà soát, hoặc agent báo vướng về kiến trúc → nâng lên model mạnh hơn và ghi lại ở `progress.md`.

## Review Focus (mọi task)

1. Lịch sử: không mất mục khi thoát (trong giới hạn `flush`), không trùng mục, không treo khi thoát, tắt thì không tác dụng phụ.
2. Lớp phủ: khi đã có vùng chọn hoặc ô gợi ý tắt, hành vi giống hệt đợt 7; kéo không bao giờ bị bắt cửa sổ chen vào.
3. Khung cửa sổ đúng ở DPI 100–250% và màn hình có tọa độ âm (tọa độ màn hình → ảnh trừ `origin` đúng một lần).
4. Không đọc dữ liệu cửa sổ ngoài khung; không mạng; không ghi nhật ký.
5. Không rò GDI; vẽ lại chỉ phần thay đổi.
6. Chữ qua i18n, đủ hai ngôn ngữ, không bị cắt.
