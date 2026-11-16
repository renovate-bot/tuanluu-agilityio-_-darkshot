# Kiến trúc và quy ước

Nguồn đầy đủ: spec mục 10, kế hoạch mục "Cấu trúc file" và các khối Interfaces trong gói task.

## Hai lớp

```
crates/shotme-core      thư viện, KHÔNG phụ thuộc hệ điều hành
crates/shotme-windows   file chạy shotme.exe, hiện thực các trait bằng Win32
```

Thêm macOS hoặc Linux sau này là thêm một crate nền tảng mới, không sửa core.

## Core chứa gì

| Module | Trách nhiệm |
|---|---|
| `geometry` | `Point`, `Rect`, `Selection` (máy trạng thái kéo thả), `place_toolbar`, `monitor_containing` |
| `image` | `Image` (BGRA, từ trên xuống), `crop`, `encode_png` |
| `naming` | tên file local, tên ngẫu nhiên trên kho, đường dẫn không trùng |
| `config` | `Config`, `Expiry`, `Hotkey`, `R2Config`, `load`, `save`, chuẩn hóa địa chỉ công khai |
| `sigv4` | chữ ký AWS Signature V4 |
| `upload` | trait `Uploader`, `R2Uploader`, `test_connection` |
| `actions` | `Action`, `run_action`, trait `Clipboard`, `Notifier` |

## Trait mà nền tảng phải hiện thực

- `config::SecretStore` (Windows: DPAPI)
- `actions::Clipboard`
- `actions::Notifier` (Windows: thông báo của biểu tượng khay)
- `upload::Uploader` đã có hiện thực chung `R2Uploader`; để ngỏ cho nơi chứa khác.

Mọi trait có ràng buộc `Send + Sync`.

## Mẫu phải theo

- Logic nằm ở core và có kiểm thử tự động; lớp nền tảng chỉ vẽ, nhận sự kiện và gọi core.
- Lớp phủ dùng `geometry::Selection` cho mọi trạng thái kéo thả, không tự tính lại.
- Mọi hành động sau khi chọn vùng đi qua `actions::run_action`.
- Thời gian máy do nền tảng cung cấp qua `naming::LocalTime`; core không gọi API thời gian của hệ điều hành.
- Tải lên chạy ở luồng nền, mỗi lần một luồng. Thông báo từ luồng nền được chuyển về luồng chính.
- Lỗi hiển thị cho người dùng bằng tiếng Việt và không bao giờ chứa khóa bí mật.
- Viết kiểm thử trước, rồi mới viết mã.

## Mẫu mới phát sinh

- Không panic trên đường chạy thật: bản release dùng `panic = "abort"`, một panic sẽ tắt cả ứng dụng khay. Dùng `Result`, và với ảnh thì kiểm `Image::is_valid()` trước.
- Ghi file cho chắc: ghi vào file tạm cùng thư mục, rồi đổi tên; dọn file tạm khi lỗi (`config::save`, lưu PNG trong `actions`).
- Đầu vào do người dùng nhập mà đi vào URL (account id, tên kho, địa chỉ công khai) phải được kiểm ký tự trước; sai thì coi như chưa cấu hình và không gửi gì.
