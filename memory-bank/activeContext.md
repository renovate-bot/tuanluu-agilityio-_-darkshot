# Bối cảnh hiện tại

Cập nhật: 2026-11-16

## Đang ở đâu

- Giai đoạn: 5. Xây dựng.
- Task vừa xong: Task 2 (hình học vùng chọn), đã gộp vào `main`.
- Task đang làm: **Task 3, `docs/tasks/03-naming-config.md`**, nhánh `task/03-naming-config`, agent `builder`, trạng thái `đang làm`.

## Bước kế tiếp

Gõ `/next-task`.

## Đang chờ chủ dự án

- Chưa có.

## Điểm dừng sắp tới cần chủ dự án

- Sau Task 8: thử lớp phủ chọn vùng theo mục "Lớp phủ" của `docs/testing/manual-checklist.md`.
- Sau Task 12: nghiệm thu với kho R2 thật.

## Ghi chú cho phiên sau

- Agent trong `.claude/agents/` được nạp khi khởi động phiên. Nếu phiên hiện tại chưa thấy chúng, giao việc bằng agent `general-purpose` với model tương ứng và dán nội dung file agent vào lời giao.
- Luôn kiểm tra `git branch --show-current` trước khi commit: một agent con từng để kho ở nhánh task sau khi phiên chính đã về `main`. Trong lời giao, ghi rõ "không chuyển nhánh".
- Agent `builder` ở Task 2 viết mã và kiểm thử cùng lúc, bỏ bước thấy kiểm thử thất bại. Từ Task 3, lời giao yêu cầu commit kiểm thử trước (commit riêng), rồi mới commit mã.
- Task 2, lựa chọn của agent đã được rà soát chấp nhận: tay nắm nhận trên cả dải cạnh; `release` bỏ khung khi một chiều nhỏ hơn 3; `place_toolbar` kẹp cả hai chiều. Task 8 (lớp phủ) cần biết: khung nhỏ hơn hai lần bán kính tay nắm thì không còn vùng "Inside" để di chuyển.
- Cargo báo cảnh báo "unused dependency shotme-core" ở `shotme-windows`; sẽ hết khi Task 7 dùng core. Không cần sửa.
- Chủ dự án có thể điều khiển phiên từ claude.ai bằng `/remote-control`.
