# Task 32: Biểu tượng tối (phương án B)

**Agent:** `builder` · **Model:** sonnet · **Nhánh:** `task/32-dark-icon` · **Rà soát:** `reviewer`

Chủ dự án hỏi vì sao biểu tượng không giống prototype, rồi chọn đổi sang phương án B (2026-11-17). Việc này thay quyết định 47 (giữ biểu tượng xanh). Thiết kế đợt 4 đã duyệt, không sửa; ghi quyết định mới vào `memory-bank/decisions.md`.

## Đích (prototype `.superpowers/brainstorm/1649-1794846769/content/darkshot-theme.html`, dòng 229 và 287–289)

SVG viewBox 56: `rect x=2 y=2 w=52 h=52 rx=13 fill=#202328 stroke=#3A3E45` (viền 1); bốn góc chữ L `M14 22v-8h8 M34 14h8v8 M42 34v8h-8 M22 42h-8v-8` nét `#4C8DFF` dày 4, đầu bo tròn (`stroke-linecap=round`); chấm giữa `cx=28 cy=28 r=3.5 fill=#E7E9EC`. Tức là chỉ đổi màu so với biểu tượng hiện có (nền chuyển màu xanh → nền than chì phẳng có viền xám nhạt; nét trắng → nét màu nhấn; chấm trắng → chấm `#E7E9EC`).

## Global Constraints

- Chỉ sửa `crates/darkshot-windows/res/make_icon.py`, sinh lại `crates/darkshot-windows/res/darkshot.ico`, thêm dòng vào `memory-bank/decisions.md` và `docs/testing/manual-checklist.md`. Không sửa mã Rust, spec, kế hoạch, gói task cũ.
- Giữ tám cỡ `SIZES`, định dạng ICO hiện có (256 PNG, còn lại DIB 32 bit), cách vẽ tay theo lưới cho 16/20/24 và siêu mẫu cho cỡ khác. Hình học khung và chấm giữ như hiện tại (đã được chủ dự án duyệt ở đợt trước); chỉ đổi màu và thêm viền nền.
- Viền nền `#3A3E45` phải nhìn thấy ở mọi cỡ (ở 16/20/24 là 1 px đúng lưới, các góc bo vẽ tay cho sạch), để biểu tượng không chìm trên thanh tác vụ tối.
- Sửa `self_check` cho hợp màu mới (nét `#4C8DFF` không còn "trắng"): kiểm giữa cạnh trên/trái là màu nền, góc là màu nhấn, khe giữa hai góc như cũ, và mép ngoài có viền.
- Pillow có sẵn hay không: nếu thiếu thì `pip install --user pillow` (chỉ máy cục bộ). Không gửi dữ liệu dự án ra ngoài.
- Kiểm `git branch --show-current` trước khi commit; không chuyển nhánh. Không đẩy lên remote. Không tắt tiến trình `darkshot` không do mình khởi động (chủ dự án đang chạy bản trong `target/fix30b` hoặc `target/fix30c`). Build release nếu cần thì dùng `--target-dir target/task32`. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.

## Các bước

- [ ] 1. Sửa `make_icon.py`; chạy `python make_icon.py <scratch>/icon32` sinh `darkshot.ico` và `preview.png` (hàng nền sáng và nền tối). Thư mục scratch: `C:/Users/Client/AppData/Local/Temp/claude/9bda5e44761d6d15/a55dcc96-f74b-4a96-9ec7-65a267b4934d/scratchpad/icon32`. Xem `preview.png` bằng mắt: cỡ 16 và 24 trên nền tối vẫn rõ khung và viền.
- [ ] 2. Thêm vào `docs/testing/manual-checklist.md` mục Đợt 4 một dòng `[ ]`: biểu tượng tối ở khay, thanh tác vụ, Alt+Tab, file exe trong Explorer, cửa sổ Thông tin, thanh tiêu đề Tùy chọn; rõ trên thanh tác vụ sáng và tối, DPI 100% và 150% (có thể phải khởi động lại Explorer vì bộ đệm biểu tượng).
- [ ] 3. Thêm quyết định mới (số kế tiếp) vào `memory-bank/decisions.md` theo mẫu bảng: biểu tượng đổi sang phương án B (nền `#202328`, viền `#3A3E45`, khung `#4C8DFF`, chấm `#E7E9EC`), thay quyết định 47 phần biểu tượng; lý do: chủ dự án muốn giống prototype.
- [ ] 4. `cargo build --release --target-dir target/task32` (để chắc tài nguyên biên dịch được); `powershell -File scripts/check.ps1` in `ALL CHECKS PASSED`. Commit `feat: dark app icon (option B)`.

**Xong khi:** check qua; báo cáo nêu commit, dòng cuối của check, đường dẫn `preview.png`, kích thước file `.ico` trước/sau, và những gì chưa nhìn bằng mắt.
