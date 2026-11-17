# Task 37: Công cụ Chữ trong lớp phủ

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/37-annotate-text` · **Rà soát:** `specialist`

Gói task tự chứa. Thuộc kế hoạch `docs/superpowers/plans/2026-11-17-annotate.md`. Chạy sau Task 36.

## Cần đọc

- Thiết kế đợt 5 `docs/superpowers/specs/2026-11-17-annotate-design.md` mục 5.2 (Esc, Ctrl+Z, Ctrl+D/S/C), 5.3 dòng Chữ, 5.4, 5.5 (đổi màu khi đang gõ)
- Kế hoạch đợt 5: "Global Constraints", "Interfaces chung" (`TextEdit`, `sanitize_paste`, `caret_rect`, `shape_bounds`), "Review Focus" dòng 4
- `crates/darkshot-windows/CLAUDE.md`
- Báo cáo Task 36 trong `memory-bank/activeContext.md` (chỗ cắm ô chữ), `overlay.rs` (toàn bộ, gồm `EscHotkey`, `take_focus`, vòng lặp thông điệp của `pick_region`), `annotate_draw.rs`, `crates/darkshot-core/src/annotate.rs`, `clipboard.rs` (đọc clipboard nếu đã có hàm)

## Global Constraints

- Không thêm crate. Có thể bật feature `Win32_UI_Input_Ime` (và feature đọc clipboard nếu thiếu). `darkshot-core` không phụ thuộc crate `windows`.
- Khi không dùng công cụ Chữ, hành vi Task 36 và hành vi cũ giữ nguyên (kiểm thử hiện có qua không sửa khẳng định).
- Không chặn ứng dụng: không vòng lặp thông điệp lồng mới, không hộp thoại.
- Release `panic = "abort"`. `unsafe` nhỏ, kèm `// SAFETY:`. Giải phóng mọi đối tượng GDI/GDI+, ngữ cảnh IME (`ImmReleaseContext`), bộ hẹn giờ, và đăng ký lại phím tắt Esc toàn cục nếu đã gỡ, trên mọi đường đi (kể cả khi lớp phủ đóng giữa lúc đang gõ).
- Kiểm thử mới không khẳng định chữ đọc từ `lang::tr`.
- Commit kiểm thử trước (`test:`, thân hàm mới `todo!()`), rồi mã (`feat:`). `cargo fmt --all`. Chạy cargo qua PowerShell. Kiểm `git branch --show-current` trước khi commit; không chuyển nhánh. Không tắt tiến trình `darkshot` không do mình khởi động; exe bị khóa thì `--target-dir target/task37`. Không đẩy lên remote. Không đọc hay ghi khóa R2. Kiểm thử `#[ignore]` chạy từng cái với `--test-threads=1`. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.

## Hành vi cần làm (thiết kế 5.4)

- Công cụ Chữ đang chọn, bấm trong vùng chọn: chốt ô đang gõ (nếu có) rồi tạo `TextEdit::new(điểm bấm, dip_to_px(TEXT_DIP, dpi), color)`. Ô đang gõ vẽ bằng `draw_shapes` (qua `to_shape`, hoặc trực tiếp cùng hàm vẽ chữ) cộng khung nét đứt mảnh `ACCENT` quanh `shape_bounds` (nới 3 DIP; ô rỗng thì khung cao một dòng, rộng tối thiểu 40 DIP) và con trỏ nháy `caret_rect` (nháy theo `GetCaretBlinkTime`, bằng bộ hẹn giờ của lớp phủ); khung và con trỏ không in ra ảnh.
- Phím: `WM_CHAR` (ký tự in được → `insert`; `\r` → `insert("\n")`; Backspace `0x08` → `backspace`), `WM_KEYDOWN` cho Delete, mũi tên, Home, End; Ctrl+V dán chữ thuần qua `sanitize_paste`; Ctrl+Z: ô có chữ thì xóa ô đó, ô rỗng thì hủy ô rồi hoàn tác hình cuối. Khi đang gõ, P/L/A/R/H/T đi vào ô chữ, không đổi công cụ.
- Chốt: bấm ra chỗ khác (trong vùng: tạo ô mới; ngoài vùng hoặc trên thanh: chỉ chốt), đổi công cụ, nút hành động, Ctrl+D/S/C (chốt rồi làm hành động). `to_shape()` `None` thì bỏ.
- Esc: đang gõ thì hủy ô đang gõ (không chốt), lớp phủ còn; đang soạn trong bộ gõ (IME có chuỗi soạn) thì Esc thuộc về bộ gõ. Phím tắt Esc toàn cục đang đăng ký (`EscHotkey`) phải không nuốt Esc lúc này: ví dụ gỡ khi ô chữ mở và đăng ký lại khi ô đóng; nêu cách chọn trong báo cáo.
- Bộ gõ Windows: khi ô chữ mở, đặt cửa sổ soạn tại con trỏ (`ImmSetCompositionWindow` với `CFS_POINT`, `ImmSetCompositionFont` cỡ gần đúng); kết quả nhận qua `WM_CHAR` (mặc định `DefWindowProc` của `WM_IME_CHAR`). Khi không có ô chữ, giữ hành vi IME như hiện nay.
- Đổi màu khi đang gõ: `set_color` cho ô đang gõ và làm màu hiện tại.

## Ghi chú từ rà soát Task 35

- `shape_bounds` của chữ rỗng hoặc toàn khoảng trắng trả hộp ở gốc ảnh chụp, không ở `origin`: không gọi nó cho ô rỗng; khung ô rỗng tính từ `caret_rect`.
- Hộp của `shape_bounds` đo theo phần mực, không gồm khoảng trắng cuối dòng: vùng làm mới và khung ô chữ phải gộp thêm `caret_rect`.
- `shape_bounds`/`caret_rect` tốn công (khởi động GDI+, tạo phông): gọi mỗi lần gõ phím được, không gọi khi di chuột hay mỗi nhịp nháy con trỏ (lưu lại kết quả).

## Các bước

- [ ] **1. Kiểm thử trước.** Commit `test: annotate text tool`.
  - `wm_char_sequence_builds_text` (Review Focus 4): mở lớp phủ có vùng, chọn Chữ, bấm trong vùng, gửi `WM_CHAR` theo chuỗi Unikey ("V","i","e",BS,"ê","t",BS,BS,"ệ","t") (Unikey xóa cả "êt" rồi gửi lại "ệt"; chuỗi giống kiểm thử `text_edit_unikey_sequence` của Task 34), Enter, "2", rồi Ctrl+S → `Picked.shapes` có đúng một `Shape::Text` với `text == "Việt\n2"`, origin là điểm bấm (tọa độ ảnh chụp).
  - `esc_cancels_text_not_overlay`: đang gõ, Esc (cả `WM_KEYDOWN` và `WM_HOTKEY` Esc) → ô hủy, lớp phủ còn; Esc lần nữa → `Ok(None)`.
  - `click_elsewhere_commits`: gõ "A", bấm chỗ khác trong vùng, gõ "B", Lưu → hai hình Text.
  - `empty_text_is_dropped`: bấm, không gõ, đổi công cụ, Lưu → không hình.
  - `ctrl_z_while_typing`: có một nét Pen; mở ô, gõ "x", Ctrl+Z → ô bị xóa, nét Pen còn; Ctrl+Z lần nữa → không hình.
  - `tool_letters_go_into_text`: đang gõ, `WM_CHAR` 'p' → chữ "p", công cụ vẫn là Chữ.
  - `escape_hotkey_restored`: đóng lớp phủ giữa lúc đang gõ → lần mở sau phím tắt Esc toàn cục vẫn đóng lớp phủ (hoặc kiểm `RegisterHotKey` lại thành công).
  - Vòng chống rò GDI của lớp phủ thêm mở ô chữ, gõ, chốt.
- [ ] **2. Mã** theo "Hành vi cần làm".
- [ ] **3. Checklist.** Thêm vào mục "Đợt 5: công cụ đánh dấu" của `docs/testing/manual-checklist.md` các dòng `[ ]`: gõ tiếng Việt bằng Unikey (Telex và VNI), EVKey, và bộ gõ tiếng Việt của Windows; cửa sổ soạn của bộ gõ hiện cạnh con trỏ; Esc khi đang soạn chỉ hủy chuỗi soạn; Enter xuống dòng; Ctrl+V dán nhiều dòng; chữ đọc được trên nền sáng và tối (viền); khung nét đứt và con trỏ không có trong ảnh ra; DPI 100% và 150%.
- [ ] **4.** `cargo fmt --all`; `powershell -File scripts/check.ps1` in `ALL CHECKS PASSED`. Commit `feat: text tool in overlay`.

**Xong khi:** check qua; báo cáo nêu các commit, dòng cuối của check, cách xử lý Esc và IME, kiểm thử `#[ignore]` và kết quả khi chạy tay, và những gì chỉ kiểm được bằng tay.
