# Task 147: Lời nhắc khi cuộn quá xa: "Cuộn lên một chút rồi cuộn chậm hơn"

**Agent:** `builder` · **Model:** sonnet (một khóa i18n, ít rủi ro) · **Nhánh:** `task/147-scroll-back-up-hint` (từ `main`, thư mục `D:\2026 - Tuan\shotme`) · **Rà soát:** phiên điều phối · **Tối đa hai vòng sửa.**

## Bối cảnh

Khi khung mới không còn chồng lấp với khung trước (`Reason::NoOverlap`, người dùng cuộn quá xa), khung bị bỏ. Phần đã ghép vẫn dừng ở khung trước, nên muốn ghép tiếp thì người dùng phải cuộn lên lại tới chỗ còn chồng lấp. Lời nhắc hiện nay "Cuộn chậm hơn" không nói điều đó. Builder Task 145 đề xuất câu mới, chủ dự án đồng ý (2026-11-24).

## Yêu cầu

1. **Khóa i18n mới** `ScrollBackUp`, thêm cả vào bảng kiểm thử chuỗi:
   - Tiếng Việt: "Cuộn lên một chút rồi cuộn chậm hơn"
   - English: "Scroll up a little, then more slowly"
2. **`crates/darkshot-windows/src/scroll_session.rs`:**
   - `warn_msg`: `Reason::NoOverlap` → `Msg::ScrollBackUp`. `Ambiguous` và `SizeMismatch` vẫn → `Msg::ScrollSlowDown`.
   - Thêm `Msg::ScrollBackUp` vào `WARN_MSGS` (cỡ mảng 4) để thanh dành chỗ cho chữ dài nhất.
   - Sửa kiểm thử máy trạng thái hiện có cho khớp; kiểm thử `bar_reserves_room_for_every_warning` phải qua với câu mới ở cả hai ngôn ngữ.
3. **Tài liệu:**
   - README mục Chụp cuộn (dòng nói các lời nhắc): thêm câu mới cho trường hợp cuộn quá xa.
   - `CHANGELOG.md`, mục "Chưa phát hành" → "Sửa lỗi": một dòng.
   - `docs/testing/manual-checklist.md`: một dòng.
   - `memory-bank/decisions.md`: quyết định 165, một dòng.
   - Trợ giúp trong `i18n.rs`: sửa nếu có nhắc tới lời nhắc này.
   - Website: không có câu nào về các lời nhắc này, nên không sửa.

## Ràng buộc

- Không đổi `scroll_stitch.rs`. Không crate mới.
- Mọi chữ người dùng thấy lấy từ `darkshot_core::i18n`.
- Mọi lệnh đọc/ghi tệp phải dùng **đường dẫn tuyệt đối** trong `D:\2026 - Tuan\shotme`.
- Không giả lập chuột hay phím thật.
- `cargo fmt --all`; `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.
- Commit `fix:` trên nhánh, kết thúc bằng dòng `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Không gộp, không đẩy. Không dùng `fable`.

## Báo cáo

Báo cáo gồm: thay đổi, kết quả `check.ps1`, và token nếu biết.
