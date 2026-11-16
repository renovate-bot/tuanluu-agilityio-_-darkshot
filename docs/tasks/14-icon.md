# Task 14: Biểu tượng ứng dụng mới

**Agent:** `builder` · **Model:** sonnet · **Nhánh:** `task/14-icon`

Chủ dự án yêu cầu cập nhật biểu tượng. Biểu tượng hiện tại (`crates/shotme-windows/res/shotme.ico`, 410 byte, ba cỡ) là hình tạm của Task 7.

## Cần đọc

- `crates/shotme-windows/CLAUDE.md`, `crates/shotme-windows/build.rs`, `res/app.rc`, `src/tray.rs` (chỗ nạp icon)

## Files

- Modify: `crates/shotme-windows/res/shotme.ico`
- Create: `crates/shotme-windows/res/make_icon.py` (script sinh icon, chạy lại được, chỉ dùng thư viện chuẩn của Python hoặc Pillow nếu máy đã có; không cài thêm gì)
- Modify: `crates/shotme-windows/src/tray.rs` (nạp đúng cỡ nhỏ cho khay)
- Modify: `docs/testing/manual-checklist.md`

## Thiết kế

- Nền: hình vuông bo góc (bán kính khoảng 22% cạnh), xanh `#2563eb`, chuyển nhẹ từ `#3b82f6` (trên) xuống `#1d4ed8` (dưới). Ngoài hình bo góc là trong suốt.
- Hình: khung chọn vùng màu trắng gồm bốn góc chữ L (không phải khung kín), nằm trong lề khoảng 24% mỗi bên; nét dày khoảng 9% cạnh, đầu nét bo tròn. Ở góc dưới phải, bên trong khung, một chấm tròn trắng nhỏ (đường kính khoảng 14% cạnh) gợi con trỏ đang kéo. Ở cỡ 16 và 20: bỏ chấm tròn, chỉ còn bốn góc, nét 2 px căn đúng lưới pixel để không nhòe.
- Khử răng cưa bằng cách vẽ ở độ phân giải gấp 4 đến 8 lần rồi thu nhỏ (trừ cỡ 16 và 20 vẽ tay theo lưới pixel).
- Cỡ trong file `.ico`: 16, 20, 24, 32, 40, 48, 64, 256, đều 32 bit có alpha. Cỡ 256 lưu dạng PNG; các cỡ còn lại theo dạng mà `rc.exe` của bản build hiện tại chấp nhận (thử build để chắc).

## Mã

- Khay hệ thống hiện nạp icon bằng `LoadIconW` (cỡ lớn rồi Windows co lại). Đổi sang `LoadImageW` với `IMAGE_ICON`, kích thước `GetSystemMetrics(SM_CXSMICON/SM_CYSMICON)`, để khay dùng đúng bản 16/20/24 đã vẽ theo lưới. Thất bại thì quay về cách cũ. Icon nạp bằng `LoadImageW` không có `LR_SHARED` phải được `DestroyIcon` khi `Tray` bị hủy; hoặc dùng `LR_SHARED`. Không `unwrap`/`expect`.
- Cửa sổ Cài đặt và file exe tự dùng icon từ resource id 1; kiểm rằng cửa sổ Cài đặt (đã có nút trên thanh tác vụ) đặt icon lớn và nhỏ cho lớp cửa sổ hoặc qua `WM_SETICON`; nếu chưa thì thêm.

## Kiểm tra

- Kiểm thử trước (commit `test:`): một kiểm thử đọc `res/shotme.ico` bằng `include_bytes!`, phân tích ICONDIR và khẳng định có đủ tám cỡ trên, mỗi mục 32 bit, offset và kích thước nằm trong file. Nó thất bại với icon cũ. Rồi `feat:`.
- Xuất thêm ảnh xem trước `preview.png` (các cỡ 16, 24, 32, 48, 64, 256 đặt cạnh nhau trên nền sáng và nền tối) vào thư mục scratchpad được giao trong lời giao việc, KHÔNG commit.
- `cargo build --release` qua; `powershell -File scripts/check.ps1` in `ALL CHECKS PASSED`.
- Checklist: sửa dòng mô tả biểu tượng khay cho khớp hình mới; thêm dòng biểu tượng hiện đúng ở khay, trên thanh tác vụ của cửa sổ Cài đặt, và trong Explorer.
