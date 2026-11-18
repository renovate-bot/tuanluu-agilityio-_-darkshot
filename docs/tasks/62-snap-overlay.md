# Task 62: Bắt cửa sổ – lớp phủ: xem trước ứng viên, bấm/Enter chọn, lời nhắc

**Agent:** `specialist` · **Model:** opus (máy trạng thái lớp phủ lớn, vẽ lại từng phần, GDI) · **Nhánh:** `task/62-snap-overlay` (tách từ `feature/window-snap`) · **Rà soát:** `specialist` (opus)

Thiết kế: `docs/superpowers/specs/2026-11-18-window-snap-design.md` mục 3 (toàn bộ), 4, 7, 9, 11. Global Constraints: mục cùng tên của `docs/superpowers/plans/2026-11-18-window-snap.md`. Task 60 (`darkshot_core::snap`, chuỗi i18n) và 61 (`Screenshot.windows`) đã ở nhánh phát triển.

## Cần đọc
`crates/darkshot-core/src/snap.rs`, `geometry.rs` (`Selection`: `press`/`drag_to`/`release`, `MIN_SIZE`), `crates/darkshot-windows/src/overlay.rs`: `pick_region`, struct trạng thái, `dragging`, `cursor_at`, `on_down`, `on_move`, `on_up`, `on_key` (nhánh `VK_RETURN`), `paint` và các hàm vẽ khung chọn/nhãn cỡ/viền hai lớp, cách tính vùng vẽ lại (`invalidate…`), và mô-đun kiểm thử `live` (mẫu kiểm thử chạy tay). Đọc theo từng đoạn bằng `grep -n`, không đọc cả tệp một lần.

## Files
Sửa `darkshot-windows/src/overlay.rs`. Nếu cần một hàm thuần hình học mới (ví dụ vị trí viên nhắc), đặt ở `darkshot-core/src/snap.rs` hoặc `geometry.rs` kèm kiểm thử.

## Yêu cầu
1. Lúc mở (`pick_region`): nếu `shot.windows` là `Some(w)` thì `cands = snap::candidates(&w, &shot.monitors, bounds_ảnh)`; `None` thì không có ứng viên và lời nhắc dùng `Msg::OverlayHintDrag`.
2. Hàm thuần `snap_active(has_cands, has_selection, dragging, panel_open) -> bool` (thiết kế 3.1) và kiểm thử.
3. `hover: Option<Rect>` cập nhật ở `on_move` khi `snap_active`; đổi thì chỉ vẽ lại hợp (khung cũ ∪ khung mới, mở rộng đủ cho viền và nhãn cỡ) và vùng viên nhắc nếu trạng thái hiện/ẩn đổi. Khi không còn `snap_active` thì `hover = None` (vẽ lại phần cũ).
4. Vẽ `hover`: dùng chính đường vẽ khung chọn hiện có (ảnh sáng trong khung, mờ ngoài khung, viền hai lớp, nhãn cỡ "W × H") nhưng KHÔNG tay nắm, KHÔNG thanh công cụ. Tốt nhất là truyền "khung hiển thị" = `selection.rect().or(hover)` vào đường vẽ kèm cờ `preview`.
5. Bấm không kéo: trong `on_up`, nếu trước lần bấm `snap_active` đúng, và sau `Selection::release` không có vùng chọn (vùng < `MIN_SIZE`), và có ứng viên tại điểm nhả → đặt vùng chọn = ứng viên (dùng `Selection::with_rect` hoặc thêm một phương thức nhỏ `Selection::set_rect(Rect)` ở lõi có kiểm thử), hiện thanh công cụ như khi kéo xong. Kéo (vùng ≥ `MIN_SIZE`) → như cũ.
6. Enter: khi không có vùng chọn và `hover` có giá trị → chọn như bấm. Đã có vùng chọn → giữ hành vi hiện tại (không đổi đường xử lý Enter cũ).
7. Viên nhắc (thiết kế 3.5): giữa mép trên màn hình chứa con trỏ, cách mép trên 16 DIP; đệm ngang 12 DIP, cao theo phông + 2×6 DIP; nền `theme::OVERLAY_LABEL` (hoặc màu nhãn cỡ đang dùng), chữ `theme::TEXT`, bo 6 DIP; DPI theo màn hình đó (đã có bảng `dpis` theo màn hình). Ẩn khi có vùng chọn, khi đang kéo, khi con trỏ trong viên mở rộng 48 DIP. Hàm thuần vị trí `hint_rect(monitor: Rect, text_size: (i32,i32), dpi: u32) -> Rect` + `hint_visible(...)` có kiểm thử. Chữ đo bằng phông thật (không cắt ở Việt).
8. Con trỏ: giữ con trỏ chữ thập hiện có khi rê ở chế độ gợi ý (không đổi `cursor_at` trừ khi cần).
9. Không đổi gì khi đã có vùng chọn, khi có hình đánh dấu, khi đang gõ chữ.

## Kiểm thử (viết trước)
- Thuần: `snap_active` (bảng đủ 16 tổ hợp hoặc các trường hợp chính), `hint_rect` (giữa màn hình, màn hình tọa độ âm trong hệ ảnh, DPI 96/144/192), `hint_visible` (gần/xa con trỏ, có vùng chọn, đang kéo), `Selection::set_rect` nếu thêm.
- Trạng thái (không mở cửa sổ thật nếu kiến trúc cho phép dựng trạng thái trong kiểm thử; nếu không, dùng kiểu kiểm thử chạy tay `live` sẵn có, `#[ignore]` + lệnh chạy):
  - rê vào ứng viên rồi bấm-nhả tại chỗ → vùng chọn = ứng viên;
  - bấm-kéo 50 px → vùng kéo tay, không phải ứng viên;
  - đã có vùng chọn (vùng nhớ) → rê không đổi `hover` (None);
  - `shot.windows = None` → bấm không chọn gì (hành vi đợt 7);
  - Enter khi có `hover` → vùng chọn = ứng viên;
  - mở/đóng lớp phủ 20 lần với gợi ý bật → không rò GDI (theo mẫu `overlay_round_trip_without_gdi_leaks`).

## Việc cần chủ dự án thử tay
Mục "Đợt 8 – Bắt cửa sổ" của checklist (phiên điều phối thêm).

## Báo cáo
Theo mẫu `.claude/agents/specialist.md`. Ghi rõ những chỗ đổi trong đường vẽ khung chọn và rủi ro hồi quy.
