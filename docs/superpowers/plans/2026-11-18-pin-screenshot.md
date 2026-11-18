# Kế hoạch đợt 9 (Giai đoạn 3 sản phẩm): Ghim ảnh lên màn hình

Thiết kế: `docs/superpowers/specs/2026-11-18-pin-screenshot-design.md`. Nhánh phát triển `feature/pin-screenshot` (tách từ `main` tại `976eda0` = `v0.3.0`). Mỗi task có nhánh `task/NN-...` tách từ nhánh phát triển và gộp lại vào đó (`--no-ff`). KHÔNG gộp vào `main`, không tag, không phát hành cho đến khi chủ dự án duyệt demo. Được đẩy nhánh phát triển và nhánh task lên `origin` sau khi kiểm diff (repo công khai).

Số task: Task 64 (tài liệu/demo đợt 8, không có gói), 64b và 65 (phát hành v0.3.0) đã dùng; đợt này bắt đầu từ 66.

## Global Constraints

- **Phụ thuộc:** không thêm crate. Feature mới của `windows` chỉ khi gói task nêu và `Cargo.toml` chưa có. `darkshot-core` không phụ thuộc hệ điều hành và không phụ thuộc `windows`.
- **Chữ hiển thị:** mọi chữ người dùng thấy từ `darkshot_core::i18n` (Việt + English). `scripts/check.ps1` cấm chuỗi tiếng Việt cứng. Việt dài hơn English khoảng 20%: không được cắt chữ.
- **Màu:** từ `darkshot_core::theme`. Giao diện luôn tối.
- **Ảnh ghim không được làm hỏng hành động chụp:** lỗi tạo ảnh ghim chỉ ra thông báo; không `unwrap`/`expect`/chỉ số có thể panic trên đường chụp hay trong thủ tục cửa sổ (bản release `panic = "abort"`).
- **Riêng tư:** không mạng mới, không ghi nhật ký ảnh/đường dẫn đầy đủ/khóa R2. Kiểm thử dùng thư mục tạm.
- **Luồng:** cửa sổ ảnh ghim chỉ trên luồng giao diện chính; không I/O lớn hay mã hóa ảnh trên luồng giao diện ngoài những chỗ hiện đã làm vậy (Lưu/Chép hiện chạy trên luồng giao diện).
- **An toàn:** mỗi khối `unsafe` có `// SAFETY:`; giải phóng mọi handle/GDI trên mọi đường đi; kiểm thử chống rò phải qua.
- **Không sửa:** spec/kế hoạch đã duyệt của đợt trước, gói task cũ, `prototype/`, dòng cũ của `decisions.md`.
- **Quy trình:** TDD: commit `test:` (kiểm thử thất bại) trước rồi `feat:`/`fix:`; `git add` từng file, `git commit` là lệnh riêng; `cargo fmt --all`; `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`; không chuyển nhánh, không gộp, không đẩy; build release dùng `--target-dir target/task<NN>`; không tắt tiến trình `darkshot` không do mình mở; kiểm thử `#[ignore]` chạy từng cái với `--test-threads=1`; không sửa kiểm thử cho khớp mã, không `#[ignore]` để né lỗi. Kiểm thử mới của lớp phủ đặt `dpis` tường minh (quyết định 102). Trong bash của máy này `cargo` không có trong PATH: chạy qua PowerShell.

## Bảng task và phân bổ model

Model là lựa chọn của phiên điều phối cho từng task (CLAUDE.md "Agent và model", quyết định 95), truyền qua tham số `model` của công cụ Agent (`fable`, `opus`, `sonnet`, `haiku` đều có trong phiên này); model trong `.claude/agents/*.md` chỉ là mặc định.

| Task | Nhánh | Agent | Model | Lý do chọn model | Nội dung | Phụ thuộc |
|---|---|---|---|---|---|---|
| 66 | task/66-snap-hint-click | specialist | opus | Điều tra lỗi vẽ lại trong lớp phủ 9.000 dòng (máy trạng thái chuột, chụp chuột, vẽ từng phần); cần suy luận sâu, không sửa mò | Nguyên nhân gốc viên nhắc đọng sau khi bấm chọn; sửa + kiểm thử hồi quy nếu có nguyên nhân | – |
| 67 | task/67-pin-core | builder | sonnet | Hàm thuần có đặc tả rõ, kiểm thử đơn vị; không Win32 | `darkshot_core::pin`, `Action::Pin`, chuỗi i18n, câu Trợ giúp | 66 (thứ tự gộp) |
| 68 | task/68-pin-window | specialist | opus | Cửa sổ Win32 mới: vòng đời, sở hữu tài nguyên GDI, DPI theo màn hình, layered window, chuột/phím, hẹn giờ | `darkshot-windows/src/pin.rs`: cửa sổ, sổ, vẽ, thao tác, menu ngữ cảnh, kiểm thử rò | 67 |
| 69 | task/69-pin-entry-points | specialist | opus | Sửa lớp phủ (thanh ngang, phím, biểu tượng), luồng chụp và vòng đời thoát ở `main`; hồi quy rủi ro cao | Nút Ghim + Ctrl+P, `capture_flow`, menu khay, dọn khi thoát | 68 |
| 70 | task/70-pin-history | builder | sonnet | Thêm một nút theo mẫu sẵn có của khung chi tiết Lịch sử; bố cục có kiểm thử thuần | Nút Ghim trong Lịch sử | 69 |
| 71 | task/71-pin-docs-demo | scaffolder (tài liệu) + phiên điều phối (QA, memory bank) | haiku (tài liệu) / opus (phiên điều phối) | Tài liệu cơ học dùng model nhẹ; QA hình ảnh và tổng hợp do phiên điều phối | README, CHANGELOG (Chưa phát hành), checklist, `scripts/demo-pin.ps1`, ảnh QA | 70 |

Rà soát (agent khác agent làm, model mạnh bằng hoặc hơn với task rủi ro cao): 66, 68, 69 → `reviewer` model **fable** (model mạnh nhất có trong phiên, khác model của người làm); 67 → `reviewer` sonnet; 70 → `reviewer` opus (cửa sổ Lịch sử 3.900 dòng, lỗi bố cục khó thấy); 71 → phiên điều phối đọc diff tài liệu. Sau Task 70: rà soát toàn đợt bằng `specialist` model fable, tập trung vòng đời cửa sổ, GDI, DPI, hồi quy lớp phủ.

Đánh giá lại model khi một task vượt dự kiến: ba vòng sửa chưa qua rà soát, hoặc agent báo vướng về kiến trúc → nâng model và ghi ở `progress.md`.

## Review Focus (mọi task)

1. Không hồi quy luồng chụp: chụp vùng, bắt cửa sổ, toàn màn hình, Lưu, Chép, Tải lên, lưu dự phòng, Annotation Pro, Hoàn tác/Làm lại, Lịch sử, Tùy chọn, khay, kiểm tra cập nhật.
2. Vòng đời: mỗi ảnh ghim giải phóng ảnh, DIB, DC, phông, hẹn giờ ở `WM_NCDESTROY`; sổ không giữ HWND chết; thoát khi còn ảnh ghim không treo.
3. DPI và tọa độ: 100% = 1 điểm ảnh ảnh/1 điểm ảnh màn hình; ghim từ lớp phủ khít vùng cũ kể cả màn hình tọa độ âm; `WM_DPICHANGED` không phóng ảnh.
4. Không panic trong thủ tục cửa sổ; tràn số khi phóng (`i32`) được kẹp.
5. Chữ qua i18n, hai ngôn ngữ, không cắt.
6. Không rò GDI/USER (kiểm thử chạy tay mở/đóng nhiều lần).
