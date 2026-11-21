# Task 134: Privacy Shield — tài liệu, website, memory bank

**Agent:** `scaffolder` · **Model:** sonnet · **Nhánh:** `task/134-privacy-docs` (từ `feature/privacy-shield`), worktree `D:\2026 - Tuan\shotme-privacy-d` · **Rà soát:** phiên điều phối

Thiết kế: `docs/superpowers/specs/2026-11-22-sensitive-info-redaction-design.md` mục 1, 3, 12 (cam kết và giới hạn), 18. Số đo thật: `docs/testing/privacy-quality.md` (Task 133). Luật website trong `CLAUDE.md` (mục "Website") và `website/README.md`.

## Cần đọc
`README.md`, `CHANGELOG.md` (mục "Chưa phát hành"), `docs/testing/manual-checklist.md` (mẫu mục "Đợt 20"), `website/index.html`, `website/src/vi.json`, `website/README.md`, `crates/darkshot-core/src/i18n.rs` (chuỗi Privacy* và Trợ giúp nếu có đoạn liệt kê tính năng/Tùy chọn), `crates/darkshot-windows/src/help.rs`, `scripts/demo-privacy.ps1`, `memory-bank/activeContext.md`, `progress.md`, `decisions.md` (định dạng bảng), `CLAUDE.md`.

## Files
`README.md`, `CHANGELOG.md`, `docs/testing/manual-checklist.md`, `website/index.html`, `website/src/vi.json` (+ `website/src/scene.ts` chỉ nếu có hình Tùy chọn cần khớp; không ảnh chụp mới), chuỗi Trợ giúp trong `i18n.rs` nếu Trợ giúp có danh sách tính năng (chỉ chữ), `CLAUDE.md` (một dòng "Đợt 21" theo mẫu các đợt khác: thiết kế, kế hoạch, gói 127–134, quyết định 144+, "Chưa phát hành", demo `scripts/demo-privacy.ps1`, và luật "Không thêm đường xuất ảnh nào không qua `privacy_gate`"), `memory-bank/decisions.md` (quyết định 144+ từ danh sách phiên điều phối đưa trong lời giao), `memory-bank/activeContext.md`, `memory-bank/progress.md` (bảng token do phiên điều phối đưa).

## Yêu cầu
1. Cam kết mục 12, nói đúng như thật: chạy hoàn toàn trên máy (Windows OCR); ảnh/chữ không gửi tới AI hay máy chủ nào để phân tích; cố gắng tốt nhất, có thể bỏ sót, nên tự xem lại và tự thêm vùng che; không phát hiện khuôn mặt/QR/hình; không áp cho Ghi GIF và chữ chép từ Nhận dạng chữ; mặc định TẮT. Không bao giờ "bảo đảm", "100%", "an toàn tuyệt đối". Dùng đúng số đo trong `privacy-quality.md` nếu nhắc tới chất lượng (hoặc không nhắc số).
2. README: mục tính năng mới (cách bật ở Tùy chọn > Chung, luồng: không phát hiện thì không hỏi; có phát hiện → cửa sổ Kiểm tra; lỗi OCR → Thử lại/Tiếp tục không quét với xác nhận), các đường được bảo vệ (Lưu, Chép, Tải lên, Ghim, Lịch sử Chép/Ghim, Lặp lại, toàn màn hình, chụp cuộn, Chụp từng bước và xuất), không bảo vệ (GIF, chữ OCR, Lịch sử Mở/Hiện trong thư mục, mục Lịch sử/phiên tạo khi tắt), giới hạn OCR (en-US ưu tiên; chữ nhỏ; ảnh rộng > 10000 px bị co). KHÔNG đổi mục "Tải về"/kích thước (chưa phát hành).
3. CHANGELOG "Chưa phát hành": Mới + Đã biết (danh sách "Đã biết" phiên điều phối đưa trong lời giao).
4. Checklist "Đợt 21: Privacy Shield" — mục thử tay phiên điều phối đưa trong lời giao.
5. Website EN + VI cùng khóa `data-i18n`: thêm mục/thẻ tính năng và câu hỏi FAQ về quyền riêng tư (cục bộ, best-effort); không hứa quá; không ảnh chụp có dữ liệu thật. `npm ci` rồi `npm run build` trong `website/` phải qua.
6. Không đụng mã Rust ngoài chuỗi Trợ giúp. Chạy `powershell -File scripts/check.ps1` (ALL CHECKS PASSED) và build website; commit `docs:`.
