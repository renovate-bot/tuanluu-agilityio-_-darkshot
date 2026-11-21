# Task 124: Ghi GIF — tài liệu và website

**Agent:** `scaffolder` · **Model:** sonnet · **Nhánh:** `task/124-gif-docs` (từ `feature/gif-recording` sau khi gộp 123) · **Rà soát:** phiên điều phối

Thiết kế: `docs/superpowers/specs/2026-11-21-gif-recording-design.md` mục 1, 2, 6.

## Cần đọc
`README.md` (mục Chụp cuộn làm mẫu), `CHANGELOG.md` (định dạng), `docs/testing/manual-checklist.md` (mục "Đợt 13: Chụp cuộn" làm mẫu), `website/README.md`, `website/index.html` và `website/src/vi.json` (mục tính năng Chụp từng bước/chụp cuộn), `crates/darkshot-core/src/gif_record.rs` (hằng số thật), `crates/darkshot-core/src/i18n.rs` (chữ thật của nút và thông báo).

## Files
`README.md`, `CHANGELOG.md`, `docs/testing/manual-checklist.md`, `website/index.html`, `website/src/vi.json`. Không sửa mã. Không ảnh mới (chưa có ảnh chụp thật; không vẽ minh họa giả).

## Yêu cầu
1. README: mục "Ghi GIF" (tiếng Việt như phần còn lại): cách bắt đầu (lớp phủ → Ghi GIF hoặc Ctrl+G), Dừng/Hủy, giới hạn (10 khung/giây, 15 s, vùng > 0,5 MP thu nhỏ, 256 màu mỗi khung, 256 MiB), nơi lưu (thư mục Lưu, tên `darkshot_…gif`, không ghi đè), không tải lên, không Lịch sử, thoát ứng dụng thì lưu phần đã ghi, giới hạn đã biết (thiết kế 6). Thêm Ctrl+G vào bảng phím lớp phủ nếu có.
2. CHANGELOG: mục `## [Chưa phát hành]` (hoặc tên tiền lệ của dự án nếu khác) ở trên cùng, nhóm Mới/Đã biết. KHÔNG nói đã phát hành, không số phiên bản.
3. Checklist: mục "## Đợt 20: Ghi GIF" với đủ 15 mục thử tay: vùng nhỏ; cửa sổ chuyển động; GIF lặp liên tục; thanh không có trong GIF; Dừng và Hủy; tối đa 15 s; nhiều màn hình; màn hình tọa độ âm; DPI hỗn hợp; vùng chồng lên chỗ thanh; phím chụp trong lúc ghi; thoát ứng dụng trong lúc ghi; ghi nhiều lần liên tiếp; trùng tên tệp; hồi quy (chụp, đánh dấu, chụp cuộn, chụp từng bước, OCR, Lịch sử, ghim). Mỗi mục `- [ ]`.
4. Website: thêm "Record GIF" vào danh sách tính năng (và Ctrl+G nếu trang có bảng phím) ở `index.html` (EN) và `vi.json` (VI) bằng cùng khóa `data-i18n`; chỉ điều có thật. Chạy `npm ci` rồi `npm run build` trong `website/` (phải qua).
5. `powershell -File scripts/check.ps1` → `ALL CHECKS PASSED`. Commit `docs:`.
