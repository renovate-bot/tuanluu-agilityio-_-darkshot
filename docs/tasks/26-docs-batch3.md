# Task 26: Tài liệu đợt 3

**Agent:** `scaffolder` · **Model:** haiku · **Nhánh:** `task/26-docs-batch3` · **Rà soát:** `reviewer`

Gói task tự chứa. Thuộc kế hoạch `docs/superpowers/plans/2026-11-16-upload-format.md`. Chạy sau Task 25.

## Cần đọc

- Thiết kế đợt 3 `docs/superpowers/specs/2026-11-16-upload-format-design.md`
- Mã trên nhánh: `crates/shotme-core/Cargo.toml`, `crates/shotme-core/src/{config,image,upload,actions,i18n}.rs`, `crates/shotme-windows/src/{settings,app,main}.rs`
- `README.md`, `docs/testing/manual-checklist.md`, `memory-bank/{productContext,systemPatterns,techContext}.md`

## Global Constraints

- Chỉ sửa tài liệu liệt kê dưới đây. Không sửa mã, spec, thiết kế, kế hoạch, gói task, `memory-bank/decisions.md`, `progress.md`, `activeContext.md`.
- Mỗi câu mô tả hành vi phải kiểm được trong mã trên nhánh: mở file mã ra đọc trước khi viết. Không chép từ gói task hay thiết kế điều bạn không thấy trong mã.
- Không viết rằng tính năng đã được thử tay hay nghiệm thu, trừ khi lời giao việc nói rõ.
- Không có khóa hay giá trị bí mật trong tài liệu. Không thêm link mới.
- Tiếng Việt, câu ngắn. Không chuyển nhánh. Không đẩy lên remote. Không tắt tiến trình `shotme` nào. Không đọc file cấu hình thật trong `%APPDATA%`. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.

## Nội dung

- [ ] `README.md`: tab Định dạng (hai lựa chọn, mặc định PNG; chất lượng JPEG 50–100, mặc định 90); chỉ áp dụng cho ảnh tải lên, ảnh lưu vào máy luôn là PNG kể cả bản lưu dự phòng; link JPEG kết thúc `.jpg`; mức 100 vẫn là nén có mất mát. Thêm mục ngắn "Thư viện bên thứ ba" chỉ gồm một dòng: ảnh JPEG được nén bằng crate `jpeg-encoder`; phần mềm này dựa một phần trên công trình của Independent JPEG Group.
- [ ] `docs/testing/manual-checklist.md`: không viết lại mục "Đợt 3: định dạng" (Task 25 đã viết). Chỉ sửa các dòng ở mục cũ mà nay sai (ví dụ dòng nói cửa sổ Tùy chọn có ba tab, hoặc mọi ảnh tải lên là PNG). Không đổi trạng thái `[ ]`/`[x]` của dòng nào.
- [ ] `memory-bank/productContext.md`: mô tả sản phẩm sau đợt 3; chuyển đợt 3 ra khỏi phần "chưa làm".
- [ ] `memory-bank/systemPatterns.md`: cập nhật mục "Thêm một ô hoặc một tab" theo `settings.rs` hiện tại (bốn tab, độ dài mảng, ID mới, nút radio và thanh trượt là hai loại ô mới và thông điệp chúng dùng); thêm mục "Mẫu của đợt 3": định dạng tải lên đi từ `Config` qua `UploadJob`/`Ctx` tới `actions::upload`; mọi đường lưu vào máy dùng `save_png`; `encode_jpeg` đặt 4:4:4 tường minh vì mặc định của crate thì khác.
- [ ] `memory-bank/techContext.md`: crate `jpeg-encoder` (phiên bản ghim, không `simd`, giấy phép, đây là ngoại lệ duy nhất của luật không thêm crate); lệnh chạy kiểm thử `--ignored` mới nếu có.
- [ ] `powershell -File scripts/check.ps1`. Commit `docs: batch 3 (upload format PNG or JPEG)`.

**Xong khi:** check qua; báo cáo liệt kê từng file đã sửa, với mỗi câu hành vi mới là file mã đã dùng để kiểm, và điều gì không kiểm được nên không viết.
