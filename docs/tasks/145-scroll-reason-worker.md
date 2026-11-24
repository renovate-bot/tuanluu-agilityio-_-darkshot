# Task 145: Chụp cuộn báo đúng lý do, chụp và ghép trên luồng nền

**Agent:** `specialist` · **Model:** opus (đồng thời, vòng đời phiên, thoát) · **Nhánh:** `task/145-scroll-reason-worker` (từ `main`), worktree `D:\2026 - Tuan\shotme-145` · **Rà soát:** `reviewer` opus (không dùng fable, quyết định 154) · **Tối đa hai vòng sửa sau rà soát.** Vẫn chưa đạt thì dừng và báo lại.

## Bối cảnh

Chủ dự án báo chụp cuộn bị chậm. Đo trên máy chủ dự án với vùng 2874×1634:

- `capture_rect` mất 50–170 ms mỗi khung, `push` mất 15–30 ms. Cả hai chạy trên luồng giao diện, trong `on_tick` của `crates/darkshot-windows/src/scroll_session.rs`.
- Mọi `Push::Uncertain(_)` đều hiện cùng một chữ "Cuộn chậm hơn" (`Msg::ScrollSlowDown`). Lời nhắc này sai khi nội dung đổi tại chỗ (video, quảng cáo chạy, thanh dính hẹp) hoặc khi người dùng cuộn lên.

Task 144 làm song song: sửa `scroll_stitch.rs` để bỏ cột đứng yên hai bên. Task này chỉ được sửa `scroll_stitch.rs` ở phần nêu trong mục 1, giữ thay đổi nhỏ để gộp không vướng.

## Yêu cầu

### 1. Lý do mới `Reason::Changed` (core, sửa tối thiểu)

- Khi `find_shift` sắp trả `Fail(NoOverlap)`, đo tỉ lệ hàng của khung mới bằng hàng cùng vị trí của khung trước (d = 0, trên phần dùng để so). Tỉ lệ ≥ một ngưỡng có tên thì trả `Fail(Reason::Changed)`. Gợi ý ngưỡng 0,5; chọn và ghi lý do bằng số đo.
- Ý nghĩa: nội dung không cuộn mà đổi tại chỗ.
- Chỉ đổi lý do, không đổi việc khung bị bỏ hay được nối. Cách tính `uncertain` cũng không đổi.
- Kiểm thử: video giả hoặc hộp nhấp nháy chiếm khoảng 10 % số hàng mà không cuộn → `Changed`; cuộn quá xa → vẫn `NoOverlap`. Mọi kiểm thử và fuzz cũ giữ nguyên kỳ vọng, trừ những kỳ vọng `NoOverlap` nay đúng ra là `Changed`; liệt kê từng cái trong báo cáo.

### 2. Lời nhắc theo lý do (thanh chụp cuộn)

- Thêm khóa i18n đủ hai ngôn ngữ và thêm vào bảng kiểm thử chuỗi. Gợi ý chữ:

  | Lý do | Tiếng Việt | English |
  |---|---|---|
  | `Backward` | "Cuộn xuống, không cuộn lên" | "Scroll down, not up" |
  | `Changed` | "Nội dung đang đổi — thử vùng hẹp hơn" | "Content is changing — try a narrower region" |
  | `NoOverlap` | giữ "Cuộn chậm hơn" | giữ "Scroll more slowly" |
  | `Ambiguous` | giữ "Cuộn chậm hơn" | giữ "Scroll more slowly" |

  Nếu nghĩ ra câu đúng hơn cho `NoOverlap`, ví dụ khi đã cuộn quá xa và phải cuộn lên lại, thì đề xuất trong báo cáo, đừng tự đổi.
- Máy trạng thái lưu lý do mới nhất và hiện chữ theo lý do đó trong `WARN_MS`. Thêm kiểm thử cho máy trạng thái.
- Chữ phải vừa thanh ở mọi lời nhắc và cả hai ngôn ngữ: xem cách thanh đo bề rộng, dành chỗ cho câu dài nhất, có kiểm thử.
- Trợ giúp: cập nhật mục chụp cuộn trong `i18n.rs` nếu có nhắc tới lời nhắc.
- Tham khảo: nhánh `task/143-scroll-column-stitch` đã làm phần này (`warn_reason`, `warning_msg`, `BarUi.warn`, kiểm thử `machine_warning_follows_latest_reason`, `bar_reserves_room_for_every_warning`, khóa `ScrollDownNotUp`, `ScrollContentChanging`). Được chép phần giao diện đó. Không chép thuật toán ghép của nhánh đó.

### 3. Chụp và ghép trên luồng nền

- Một luồng riêng có tên, ví dụ `darkshot-scroll`, giữ `Stitcher` và tự chạy vòng: chụp `capture_rect` → `push` → gửi kết quả về luồng giao diện bằng `PostMessageW`. Kết quả gồm `Push`, chiều cao ảnh và thời gian nhịp.
  - Số `WM_APP` mới: dùng số kế tiếp chưa dùng. Hiện đã dùng tới +24; kiểm lại cả repo.
  - Cửa sổ chủ đã đóng thì luồng tự dừng.
- Luồng giao diện chỉ còn: máy trạng thái, thanh, lời nhắc, nút Xong/Hủy/Esc. Không gọi `capture_rect` hay `push` trên luồng giao diện nữa, kể cả lần chụp cuối khi bấm Xong (`done_with`).
- Giãn nhịp khi máy chậm (`tick_took`) vẫn chạy như cũ, nhưng tính theo thời gian luồng nền báo về. Không chụp dồn: mỗi lúc chỉ một lượt chụp.
- **Xong:**
  1. Báo luồng nền chụp lần cuối (như `done_with` hiện nay).
  2. Luồng nền gọi `finish` rồi trả `(Image, uncertain)` về luồng giao diện.
  3. Luồng giao diện giao kết quả như `finish_ended` hiện nay, giữ đúng thứ tự `ON_END` → `ON_RESULT` → `ON_LIMIT` và hành vi khi chụp hỏng (`ON_CAPTURE_FAILED`).
- **Hủy, Esc, đạt giới hạn, thoát ứng dụng, đăng xuất:**
  - Luồng nền dừng ở nhịp kế tiếp và không giao gì.
  - Thoát phải theo hạn chót chung 4 s (quyết định 124). Không treo vì đang chờ luồng nền; luồng nền đang chụp dở thì bỏ.
- Bộ nhớ: không giữ hàng đợi khung. Mỗi lúc chỉ có khung đang ghép và ảnh ghép.
- Lớp thanh và các cửa sổ của darkshot vẫn không lọt vào khung chụp. Đọc kỹ cách hiện nay tránh việc đó (affinity, thứ tự) và giữ nguyên.
- Tham khảo luồng nền và hủy của Ghi GIF trong `crates/darkshot-windows/src/record_session.rs`.
- Kiểm thử, không cần màn hình thật nếu được:
  - máy trạng thái nhận kết quả từ thông điệp;
  - Hủy khi luồng nền đang bận;
  - Xong giao đúng ảnh;
  - thoát trong hạn;
  - không rò GDI.

  Kiểm thử `#[ignore]` chạy thật như hiện có thì giữ và cập nhật.

### 4. Tài liệu

- README mục Chụp cuộn: lời nhắc mới.
- `CHANGELOG.md` "Chưa phát hành":
  - "Mới": lời nhắc theo lý do;
  - "Sửa lỗi": chụp cuộn vùng lớn không còn làm chậm thanh và ứng dụng.
- `docs/testing/manual-checklist.md`: vài dòng.
- `memory-bank/decisions.md`: quyết định 163, gồm ngưỡng `Changed`, thiết kế luồng, số `WM_APP` và số đo thời gian.
- Website: chỉ sửa nếu có câu về chụp cuộn bị ảnh hưởng. Nếu sửa thì chạy `npm ci` và `npm run build`.

## Ràng buộc

- Không crate mới. Không panic, không chỉ số tràn.
- Không gửi đầu vào tới ứng dụng khác. Không giả lập chuột hay phím thật (chủ dự án đang dùng máy).
- Mọi chữ người dùng thấy lấy từ `darkshot_core::i18n`.
- Repo công khai: không đưa ảnh chụp thật hay dữ liệu cá nhân vào repo.
- `cargo fmt --all`; `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.
- Commit `feat:`/`fix:`/`test:`/`docs:` trên nhánh. Không gộp, không đẩy. Không dùng `fable`.

## Báo cáo

Báo cáo gồm:

- thay đổi;
- thời gian đo trước và sau (`on_tick` trên luồng giao diện, nhịp luồng nền);
- kỳ vọng kiểm thử đã đổi và lý do;
- giới hạn còn lại;
- điều chưa kiểm được;
- token nếu biết.
