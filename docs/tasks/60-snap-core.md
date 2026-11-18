# Task 60: Bắt cửa sổ – lõi (`snap`), cấu hình `snap_windows`, chuỗi i18n

**Agent:** `builder` · **Model:** sonnet (logic thuần, đặc tả rõ) · **Nhánh:** `task/60-snap-core` (tách từ `feature/window-snap`) · **Rà soát:** `reviewer` (sonnet)

Thiết kế: `docs/superpowers/specs/2026-11-18-window-snap-design.md` mục 3.2, 3.5, 3.6, 3.7, 5. Global Constraints: mục cùng tên của `docs/superpowers/plans/2026-11-18-window-snap.md`.

## Cần đọc
`crates/darkshot-core/src/geometry.rs` (`Point`, `Rect`, `Rect::contains`, `right`, `bottom`), `config.rs` (trường `include_cursor`/`keep_selection` làm mẫu: struct, mặc định, đọc khoan dung, ghi, kiểm thử), `i18n.rs` (cách khai báo `Msg`, bảng hai ngôn ngữ, `help_text` và kiểm thử của nó), `lib.rs`.

## Files
Tạo `darkshot-core/src/snap.rs`. Sửa `darkshot-core/src/{lib.rs, config.rs, i18n.rs}`. Nếu đổi `Config` làm lớp Windows không biên dịch (dựng `Config` bằng struct literal ở `settings.rs`), sửa tối thiểu ở đó để giữ giá trị (`snap_windows: cfg.snap_windows` / mặc định) và nêu trong báo cáo; KHÔNG thêm ô tích (Task 63).

## Interfaces
```rust
// darkshot_core::snap
pub const MIN_SNAP: i32 = 16;
pub const MAX_WINDOWS: usize = 512;
/// Cắt mỗi khung theo `bounds`; bỏ khung có chiều nào < MIN_SNAP sau khi cắt; bỏ khung trùng hệt khung ngay trước;
/// giữ thứ tự; chỉ lấy tối đa MAX_WINDOWS khung cửa sổ đầu; rồi nối thêm `monitors` (cũng cắt, cũng lọc < MIN_SNAP).
pub fn candidates(windows: &[Rect], monitors: &[Rect], bounds: Rect) -> Vec<Rect>;
/// Ứng viên đầu tiên chứa `p` (theo `Rect::contains`).
pub fn candidate_at(cands: &[Rect], p: Point) -> Option<Rect>;
/// Giao hai khung; None khi không giao hoặc rỗng. Dùng i64 để không tràn.
pub fn clip(r: Rect, bounds: Rect) -> Option<Rect>;
```
- `Config.snap_windows: bool`, mặc định `true`, JSON `"snap_windows"`; thiếu hoặc sai kiểu → `true`; luôn ghi ra.
- `Msg::ChkSnapWindows` — Vi "Gợi ý cửa sổ khi chọn vùng", En "Suggest windows while selecting".
- `Msg::OverlayHintSnap` — Vi "Bấm: chọn cửa sổ · Kéo: chọn vùng · Esc: thoát", En "Click: window · Drag: region · Esc: cancel".
- `Msg::OverlayHintDrag` — Vi "Kéo để chọn vùng · Esc để thoát", En "Drag to select · Esc to cancel".
- `help_text`: thêm một câu (cả hai ngôn ngữ) ngay sau câu về kéo chọn vùng: Vi "Chưa có vùng chọn thì rê chuột lên một cửa sổ hoặc màn hình rồi bấm (hoặc Enter) để chọn đúng khung đó.", En "Before you select, hover a window or screen and click (or press Enter) to select exactly that frame." Đồng thời sửa câu Esc đã cũ nếu `help_text` còn nói Esc chỉ đóng lớp phủ (thiết kế đợt 6 quyết định 80: Esc lùi từng bậc) — viết ngắn: Vi "Esc lùi một bước (đóng bảng, hủy chữ, bỏ chọn) rồi đóng lớp phủ.", En "Esc steps back (closes a panel, cancels text, deselects), then closes the overlay." Chỉ sửa nếu câu hiện có sai.

## Kiểm thử (viết trước)
- `clip`: giao một phần, nằm trọn, ngoài hẳn → None, chạm mép (chiều rộng 0) → None, tọa độ âm, giá trị gần `i32::MAX` không tràn.
- `candidates`: cắt theo bounds; bỏ < 16 (15×100 bỏ, 16×16 giữ); bỏ trùng liên tiếp (giữ trùng không liền kề); màn hình nối cuối; thứ tự giữ nguyên; 600 cửa sổ → đúng 512 cửa sổ + màn hình; cửa sổ ngoài bounds bị bỏ.
- `candidate_at`: hai khung chồng → lấy khung đầu; điểm trên mép trái/trên thuộc khung, mép phải/dưới theo đúng ngữ nghĩa `contains` hiện có (đọc mã, khẳng định đúng như nó); điểm chỉ thuộc màn hình → màn hình; ngoài mọi khung → None; danh sách rỗng → None.
- Cấu hình: mặc định `true`; JSON thiếu trường → `true`; `"snap_windows": false` → `false`; sai kiểu (`"no"`, `0`) → `true`; vòng ghi/đọc giữ giá trị; tệp cấu hình cũ nguyên văn (mẫu kiểm thử có sẵn) vẫn nạp.
- i18n: ba `Msg` mới có chữ khác rỗng ở cả hai ngôn ngữ và hai bản khác nhau; `help_text` hai ngôn ngữ chứa "Enter" và chữ tương ứng câu mới.

## Báo cáo
Theo mẫu `.claude/agents/builder.md`.
