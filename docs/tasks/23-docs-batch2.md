# Task 23: Tài liệu đợt 2

**Agent:** `scaffolder` · **Model:** haiku · **Nhánh:** `task/23-docs-batch2` · **Rà soát:** `reviewer`

Gói task tự chứa. Thuộc kế hoạch `docs/superpowers/plans/2026-11-16-capture-batch2.md`. Chạy sau khi chủ dự án báo kết quả thử tay đợt 2.

## Cần đọc

- Thiết kế đợt 2 `docs/superpowers/specs/2026-11-16-capture-batch2-design.md`
- Mã trên nhánh: `crates/shotme-core/src/{config,actions,geometry,i18n}.rs`, `crates/shotme-windows/src/{main,settings,hotkey,capture,overlay,help}.rs`
- `README.md`, `docs/testing/manual-checklist.md`, `memory-bank/{productContext,systemPatterns,techContext}.md`

## Global Constraints

- Chỉ sửa tài liệu liệt kê dưới đây. Không sửa mã, spec, thiết kế, `memory-bank/decisions.md`, `progress.md`, `activeContext.md`.
- Mỗi câu mô tả hành vi phải kiểm được trong mã trên nhánh: mở file mã ra đọc trước khi viết. Không chép từ gói task hay thiết kế điều bạn không thấy trong mã.
- Không viết rằng tính năng đã được thử tay hay nghiệm thu, trừ khi lời giao việc nói rõ.
- Không có khóa hay giá trị bí mật trong tài liệu. Không thêm link mới.
- Tiếng Việt, câu ngắn. Không chuyển nhánh. Không đẩy lên remote. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.

## Nội dung

- [ ] `README.md`: tab Phím tắt có ba dòng với ô tích bật/tắt và mặc định của từng dòng; hai phím toàn màn hình làm gì và "toàn màn hình" là màn hình đang có con trỏ chuột; ba ô tích mới ở tab Thông thường, gồm ý "thông báo lỗi luôn hiện" và "vị trí vùng chỉ nhớ trong lúc ứng dụng chạy"; con trỏ được vẽ vào ảnh lúc chụp nên thấy trên lớp phủ.
- [ ] `docs/testing/manual-checklist.md`: không viết lại hai mục "Đợt 2" (Task 21, 22 đã viết). Chỉ sửa các dòng ở mục cũ mà nay sai (ví dụ dòng nói tab Phím tắt có một dòng, hoặc mọi lượt chụp đều mở lớp phủ).
- [ ] `memory-bank/productContext.md`: mô tả sản phẩm sau đợt 2; chuyển đợt 2 ra khỏi phần "chưa làm"; đợt 3 vẫn chưa làm.
- [ ] `memory-bank/systemPatterns.md`: ba ô phím tắt và mã đăng ký; loại chụp đi qua cổng; chỗ lọc thông báo duy nhất; con trỏ vẽ trong lúc chụp; khung nhớ theo tọa độ màn hình; cập nhật mục "Thêm một ô hoặc một tab" nếu cấu trúc `settings.rs` đã đổi (tên hàm, kiểu, độ dài mảng).
- [ ] `memory-bank/techContext.md`: feature mới của crate `windows` nếu có; lệnh chạy các kiểm thử `--ignored` mới.
- [ ] `powershell -File scripts/check.ps1`. Commit `docs: batch 2 (full-screen hotkeys, cursor, keep selection, notifications)`.

**Xong khi:** check qua; báo cáo liệt kê từng file đã sửa, với mỗi câu hành vi mới là file mã đã dùng để kiểm, và điều gì không kiểm được nên không viết.
