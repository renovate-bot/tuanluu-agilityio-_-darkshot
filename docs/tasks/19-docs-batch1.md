# Task 19: Tài liệu đợt 1

**Agent:** `scaffolder` · **Model:** haiku · **Nhánh:** `task/19-docs-batch1`

Gói task tự chứa. Thuộc kế hoạch `docs/superpowers/plans/2026-11-16-options-tabs-i18n.md`. Chạy sau khi chủ dự án đã thử tay Task 15–18. Task này chỉ sửa tài liệu, không sửa mã.

## Cần đọc

- Thiết kế đợt 1 `docs/superpowers/specs/2026-11-16-options-tabs-i18n-design.md`
- `README.md`, `docs/testing/manual-checklist.md`
- `memory-bank/productContext.md`, `systemPatterns.md`, `techContext.md`, `activeContext.md`
- Để viết đúng theo mã, không theo trí nhớ: `crates/shotme-core/src/i18n.rs` (phần đầu, không cần đọc cả bảng), `crates/shotme-windows/src/{lang,tray,settings,about,help,winutil}.rs` (phần chú thích đầu file và các hàm `pub`)

## Global Constraints

- Chỉ viết điều đúng với mã đang có trên nhánh. Điều gì không kiểm được trong mã thì không viết.
- Không sửa spec gốc `docs/superpowers/specs/2026-11-16-shotme-design.md`, không sửa thiết kế đợt 1, không sửa `memory-bank/decisions.md` và `progress.md` (phiên điều phối lo).
- Tài liệu viết bằng tiếng Việt, câu ngắn, không thêm mục không ai cần.
- Không ghi khóa R2 hay bất kỳ giá trị bí mật nào. Không thêm link ngoài những link README đã có.
- Không chuyển nhánh. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.

## Nội dung

- [ ] **Bước 1: `README.md`.**
  - Mọi chỗ gọi "Cài đặt" (tên cửa sổ, mục menu, "nút Lưu") đổi thành "Tùy chọn", "OK", và nêu đúng tab chứa từng ô (Thông thường, Phím tắt, Lưu trữ R2).
  - Mục cách dùng: menu khay có năm mục; có cửa sổ Thông tin và Trợ giúp trong ứng dụng.
  - Thêm một đoạn ngắn "Ngôn ngữ": Tiếng Việt và English; đổi ở Tùy chọn › Thông thường, có hiệu lực ngay; lần chạy đầu theo ngôn ngữ Windows.
  - Mục "Bảo mật": thêm một câu rằng Thông tin và Trợ giúp không mở trang web và không gọi mạng.

- [ ] **Bước 2: `docs/testing/manual-checklist.md`.** Đọc lại hai mục "Đợt 1" do Task 17 và 18 thêm: gộp dòng trùng, sửa dòng nào không còn đúng với mã. Ở các mục cũ, đổi "Cài đặt" thành "Tùy chọn" và "Lưu" thành "OK" cho khớp giao diện.

- [ ] **Bước 3: `memory-bank/productContext.md`.** Thêm hành vi mới: menu năm mục; Tùy chọn ba tab và ô nào ở tab nào; hai ngôn ngữ và hai quy tắc chọn mặc định; Thông tin và Trợ giúp không chặn ứng dụng. Ghi rõ đợt 2 (chụp) và đợt 3 (định dạng) chưa làm, Proxy không làm.

- [ ] **Bước 4: `memory-bank/systemPatterns.md`.** Thêm các mẫu:
  - Bảng chuỗi: ở đâu, cách thêm một chuỗi, vì sao thiếu bản dịch là lỗi biên dịch; core nhận `Lang` qua tham số, lớp Windows giữ ngôn ngữ hiện hành trong `lang.rs`.
  - Cửa sổ dạng tab: dải tab, khung con, chuyển tiếp `WM_COMMAND`, hàm thuần `tab_of`, `initial_tab`, `key_route`.
  - Cửa sổ không chặn: `show`/`close`/`pre_translate`, phải gọi `pre_translate` ở mọi vòng lặp thông điệp.
  - Cách thêm một tab hoặc một ô cho đợt 2 và 3 (ba đến năm dòng, theo đúng mã).

- [ ] **Bước 5: `memory-bank/techContext.md`.** Bước mới của `scripts/check.ps1` (chặn chuỗi tiếng Việt viết cứng) và feature mới của crate `windows` nếu Task 17 có bật; lệnh chạy các kiểm thử chạy tay của ba cửa sổ.

- [ ] **Bước 6:** `powershell -File scripts/check.ps1` in `ALL CHECKS PASSED`. Commit `docs: batch 1 (options tabs, languages, About, Help)`.

**Xong khi:** check qua; báo cáo liệt kê từng file đã sửa và, với mỗi câu mô tả hành vi mới, tên file mã đã dùng để kiểm.
