# Task 22: Con trỏ trong ảnh, hai phím toàn màn hình, lớp phủ mở với khung nhớ

**Agent:** `specialist` · **Model:** opus · **Nhánh:** `task/22-fullscreen-cursor-selection` · **Rà soát:** `specialist`

Gói task tự chứa. Thuộc kế hoạch `docs/superpowers/plans/2026-11-16-capture-batch2.md`.

## Cần đọc

- `crates/shotme-windows/CLAUDE.md`, `memory-bank/systemPatterns.md`
- Thiết kế đợt 2 `docs/superpowers/specs/2026-11-16-capture-batch2-design.md` mục 2, 5.2, 5.5, 5.6
- `crates/shotme-windows/src/capture.rs`, `overlay.rs`, `main.rs`, `app.rs`, `hotkey.rs` (toàn bộ)
- `crates/shotme-core/src/geometry.rs`, `actions.rs`, `config.rs`

## Global Constraints

- Mọi chữ người dùng thấy lấy từ bảng `shotme_core::i18n`, đủ hai ngôn ngữ; không viết cứng.
- Không thêm crate (được bật thêm feature của crate `windows`). Không thêm đích mạng.
- Mọi tọa độ là pixel vật lý; chụp trong ngữ cảnh DPI `PerMonitorV2` như hiện nay.
- Release dùng `panic = "abort"`: không `unwrap`/`expect`/chỉ số có thể panic trên đường chạy thật. `unsafe` nhỏ, kèm `// SAFETY:`. Giải phóng mọi handle GDI trên mọi đường đi, kể cả hai bitmap mà `GetIconInfo` trả về.
- Không làm mất ảnh: hai phím mới dùng đúng đường đi của Lưu và Tải lên ở lớp phủ, gồm sổ `PENDING`, lưu dự phòng khi tải lỗi, lưu khi không tạo được luồng, `on_needs_settings`.
- `gate_step` không đổi. Không bao giờ có hai lớp phủ. Phím toàn màn hình bấm lúc cổng không ở trạng thái nhận thì không chụp.
- Lớp phủ khi không có khung ban đầu phải y như hiện nay.
- Ô "con trỏ" tắt: ảnh giống hệt hiện nay (không gọi API con trỏ để vẽ).
- Không co giãn con trỏ; không nhớ khung qua lần khởi động; không thêm mục menu.
- Kiểm thử mới không khẳng định chữ đọc từ `lang::tr`.
- Commit kiểm thử trước (`test:`, thân hàm mới `todo!()`, không kèm mã thật), rồi mã (`feat:`). `cargo fmt --all`. Chạy cargo qua PowerShell. Không chuyển nhánh. Không tắt tiến trình `shotme` không do mình khởi động. Không đẩy lên remote. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.

## Interfaces

**Consumes:**

```rust
// shotme_core (Task 20)
pub enum HotkeyRole { Region, SaveFull, UploadFull }
pub struct Config { pub keep_selection: bool, pub include_cursor: bool, pub hotkeys: Hotkeys, .. }
impl Selection { pub fn with_rect(bounds: Rect, rect: Option<Rect>) -> Selection; }
pub fn restore_selection(remembered: Rect, origin: Point, image_w: i32, image_h: i32, monitors: &[Rect]) -> Option<Rect>;
pub fn monitor_containing(p: Point, monitors: &[Rect]) -> Rect;   // đã có; không chứa thì gần nhất
// shotme-windows (Task 21)
pub fn hotkey::role_of(id: i32) -> Option<HotkeyRole>;
// main.rs: nhánh WM_HOTKEY đã phân theo role; SaveFull và UploadFull đang để trống
```

**Produces:**

```rust
// capture.rs
pub struct Screenshot { pub image: Image, pub origin: Point, pub monitors: Vec<Rect>, pub cursor: Point }
pub fn capture_virtual_screen(include_cursor: bool) -> windows::core::Result<Screenshot>;
pub fn to_screen_coords(image: Rect, origin: Point) -> Rect;
pub fn cursor_draw_origin(pos: Point, hotspot: Point, origin: Point) -> Point;

// overlay.rs
pub fn pick_region(shot: &Screenshot, initial: Option<Rect>) -> Result<Option<(Action, Rect)>, String>;

// main.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum CaptureKind { Region, SaveFull, UploadFull }
pub fn kind_of(role: HotkeyRole) -> CaptureKind;
pub fn next_kind(current: CaptureKind, requested: CaptureKind, act: GateAction) -> CaptureKind;
pub fn remember_after(prev: Option<Rect>, action: Action, picked: Rect) -> Option<Rect>;
```

## Thiết kế đã chốt

**Con trỏ (`capture.rs`).**

- `Screenshot.cursor`: vị trí con trỏ lúc chụp, tọa độ màn hình, lấy ngay cạnh `BitBlt` (luôn lấy, kể cả khi `include_cursor` là `false`; không lấy được thì dùng tâm màn hình ảo).
- `include_cursor`: sau `BitBlt` và trước khi đọc pixel, khi DIB còn được chọn trong DC bộ nhớ, vẽ con trỏ đang hiện vào DC đó tại `cursor_draw_origin(pos, hotspot, origin)` = `pos - hotspot - origin` từng trục. Cách gợi ý: `GetCursorInfo` (cờ `CURSOR_SHOWING`), `GetIconInfo` cho điểm nóng, `DrawIconEx` với cỡ 0 và `DI_NORMAL`. Con trỏ đang ẩn, hoặc bất kỳ lời gọi nào thất bại: bỏ qua việc vẽ, không trả lỗi.
- Kênh alpha vẫn được đặt đục cho toàn ảnh sau đó (đã có).
- Con trỏ ở kích thước Windows trả về; không tự co giãn (giới hạn đã chấp nhận, ghi vào checklist).

**Loại chụp và cổng (`main.rs`).**

- `kind_of`: `Region`→`Region`, `SaveFull`→`SaveFull`, `UploadFull`→`UploadFull`.
- Một `thread_local` `CAPTURE_KIND: Cell<CaptureKind>` cạnh `GATE`. Mọi yêu cầu chụp (phím tắt ba loại, "Chụp màn hình" ở menu = `Region`) đi qua một hàm duy nhất: tính `gate_step`, đặt `CAPTURE_KIND = next_kind(current, requested, act)`, rồi thực hiện `act` như `gate` hiện nay. `capture_flow` đọc `CAPTURE_KIND` lúc bắt đầu.
- `next_kind`: `act` là `StartTimer` hoặc `CaptureNow` → `requested`; mọi `act` khác → `current`.
- `WM_HOTKEY` cho cả ba loại dùng chung luật hiện có của phím chụp vùng: menu khay đang mở thì `EndMenu` rồi đưa sự kiện `MenuCapture` (chờ 250 ms), không thì sự kiện `Hotkey`. `TimerFired` không đổi `CAPTURE_KIND`.
- Hệ quả phải đúng: lớp phủ đang mở → phím bị bỏ qua, loại của lượt đang chạy không đổi; Tùy chọn đang mở → đưa Tùy chọn ra trước; đang chờ 250 ms của menu mà bấm phím khác loại → chụp ngay một lần với loại của phím vừa bấm.

**Luồng chụp (`capture_flow`).**

- Đọc `config.include_cursor`, gọi `capture_virtual_screen(include_cursor)`.
- `Region`: `initial` = nếu `config.keep_selection` và có khung nhớ thì `restore_selection(khung, shot.origin, w, h, &shot.monitors)`, không thì `None`. Gọi `pick_region(&shot, initial)`. Sau khi có `(action, rect)`: khung nhớ = `remember_after(prev, action, to_screen_coords(rect, shot.origin))`, cập nhật kể cả khi `keep_selection` đang tắt. Phần còn lại như hiện nay.
- `SaveFull` / `UploadFull`: không mở lớp phủ, không đặt `overlay_open`. Vùng = `monitor_containing(con trỏ đổi sang tọa độ ảnh, &shot.monitors)`; nếu vùng rỗng (`w` hoặc `h` bằng 0) thì dùng cả ảnh. Cắt ảnh rồi làm đúng nhánh `Action::Save` hoặc `Action::Upload` hiện có. Tách phần "thực hiện hành động trên một ảnh đã cắt" thành một hàm dùng chung cho cả ba loại, không chép mã.
- Khung nhớ: `thread_local` `LAST_SELECTION: Cell<Option<Rect>>` (tọa độ màn hình), chỉ đọc/ghi ở lượt `Region`.
- Lỗi chụp: thông báo hiện có.

**Lớp phủ (`overlay.rs`).** `pick_region(shot, initial)`: khởi tạo `sel` bằng `Selection::with_rect(bounds, initial)`. Khi có khung ban đầu, lần vẽ đầu tiên phải cho đúng trạng thái "đã chọn xong": vùng sáng, viền, 8 tay nắm, nhãn kích thước, thanh công cụ đặt theo `place_toolbar` với màn hình chứa khung; con trỏ chuột theo vùng nó đang đứng (trong khung, trên tay nắm, bên ngoài); Ctrl+D, Ctrl+S, Ctrl+C có tác dụng ngay; kéo bên ngoài chọn lại, kéo bên trong di chuyển, kéo tay nắm đổi cỡ. `initial` là `None`: không một dòng mã nào chạy khác hiện nay.

## Các bước

- [ ] **Bước 1: kiểm thử hàm thuần** (thân hàm mới `todo!()`).

```rust
// capture.rs
#[test] fn screen_and_image_coords_are_inverse() {
    let o = Point { x: -1280, y: 40 };
    let r = Rect { x: 10, y: 20, w: 30, h: 40 };
    assert_eq!(to_screen_coords(r, o), Rect { x: -1270, y: 60, w: 30, h: 40 });
    assert_eq!(to_image_coords(to_screen_coords(r, o), o), r);
}
#[test] fn cursor_origin_subtracts_hotspot_and_screen_origin() {
    assert_eq!(cursor_draw_origin(Point { x: 100, y: 50 }, Point { x: 0, y: 0 }, Point { x: 0, y: 0 }), Point { x: 100, y: 50 });
    // màn hình phụ bên trái: gốc âm; con trỏ chữ I có điểm nóng ở giữa
    assert_eq!(cursor_draw_origin(Point { x: -200, y: 10 }, Point { x: 8, y: 12 }, Point { x: -1280, y: 0 }), Point { x: 1072, y: -2 });
}

// main.rs
#[test] fn kinds_of_roles() {
    assert_eq!(kind_of(HotkeyRole::Region), CaptureKind::Region);
    assert_eq!(kind_of(HotkeyRole::SaveFull), CaptureKind::SaveFull);
    assert_eq!(kind_of(HotkeyRole::UploadFull), CaptureKind::UploadFull);
}
#[test] fn capture_kind_follows_the_accepted_request() {
    use CaptureGate::*; use CaptureKind::*; use GateEvent::*;
    let step = |state, ev, cur, req| { let (s, a) = gate_step(state, ev); (s, a, next_kind(cur, req, a)) };
    // rảnh, bấm phím lưu toàn màn hình: chụp ngay loại đó
    assert_eq!(step(Idle, Hotkey, Region, SaveFull), (Capturing, GateAction::CaptureNow, SaveFull));
    // menu đang mở: chờ, loại được nhớ; hết giờ không đổi loại
    let (s, a, k) = step(Idle, MenuCapture, Region, UploadFull);
    assert_eq!((s, a, k), (PendingMenu, GateAction::StartTimer, UploadFull));
    assert_eq!(gate_step(s, TimerFired), (Capturing, GateAction::CaptureNow));
    // đang chờ menu (Region) mà bấm phím loại khác: chụp ngay một lần, loại của phím vừa bấm
    assert_eq!(step(PendingMenu, Hotkey, Region, SaveFull), (Capturing, GateAction::CaptureNow, SaveFull));
    // lớp phủ đang mở: bỏ qua, loại không đổi
    assert_eq!(step(Capturing, Hotkey, Region, SaveFull), (Capturing, GateAction::Ignore, Region));
    assert_eq!(step(Capturing, MenuCapture, Region, UploadFull).2, Region);
    // Tùy chọn đang mở: đưa ra trước, loại không đổi
    assert_eq!(step(SettingsOpen, Hotkey, Region, UploadFull), (SettingsOpen, GateAction::FocusSettings, Region));
}
#[test] fn selection_is_remembered_only_when_used() {
    let old = Some(Rect { x: 1, y: 2, w: 3, h: 4 });
    let new = Rect { x: 10, y: 20, w: 30, h: 40 };
    for a in [Action::Upload, Action::Save, Action::Copy] {
        assert_eq!(remember_after(old, a, new), Some(new));
        assert_eq!(remember_after(None, a, new), Some(new));
    }
    assert_eq!(remember_after(old, Action::Cancel, new), old);
    assert_eq!(remember_after(None, Action::Cancel, new), None);
}
```

- [ ] **Bước 2:** `cargo test -p shotme-windows` thấy kiểm thử mới thất bại. Commit `test: capture kinds, cursor origin, selection memory`.
- [ ] **Bước 3:** hàm thuần. Kiểm thử qua.
- [ ] **Bước 4:** `capture.rs`: `cursor`, `include_cursor`. Thêm kiểm thử chạy tay `#[ignore]` `capture_with_cursor_does_not_leak_gdi`: chụp 50 lần với `include_cursor = true`, số đối tượng GDI của tiến trình (`GetGuiResources`) sau cùng không lớn hơn sau lượt chụp thứ hai; ảnh hợp lệ và cùng kích thước với lượt `false`.
- [ ] **Bước 5:** `overlay.rs`: `pick_region(shot, initial)`.
- [ ] **Bước 6:** `main.rs`: `CAPTURE_KIND`, một lối vào cho mọi yêu cầu chụp, `capture_flow` ba loại, `LAST_SELECTION`, nối `WM_HOTKEY` của hai phím mới.
- [ ] **Bước 7:** chạy `cargo test -p shotme-windows -- --ignored capture_with_cursor --test-threads=1` một lần, ghi kết quả. Kiểm thử chạy tay hiện có của lớp phủ và của chụp sửa theo chữ ký mới.
- [ ] **Bước 8: checklist.** Thêm vào `docs/testing/manual-checklist.md` mục "Đợt 2: chụp" (mọi ô `[ ]`): phím lưu toàn màn hình lưu đúng màn hình đang có con trỏ, trên một và hai màn hình, DPI khác nhau, màn hình phụ ở bên trái; phím tải lên nhanh chép link, link mở ra đúng ảnh; tải lên nhanh khi chưa có R2 (ảnh được lưu, Tùy chọn mở ở tab Lưu trữ R2) và khi mất mạng (ảnh được lưu, có thông báo lỗi); bấm hai phím lúc lớp phủ đang mở (không gì xảy ra), lúc Tùy chọn mở (Tùy chọn ra trước), lúc menu khay mở (menu đóng, ảnh không có menu); bấm nhanh nhiều lần liên tiếp; thoát ứng dụng ngay sau tải lên nhanh (ảnh có trong thư mục lưu); con trỏ: bật ô rồi chụp vùng (con trỏ đóng băng thấy trên lớp phủ, vào ảnh khi nằm trong vùng), chụp toàn màn hình, với con trỏ mũi tên, chữ I, bàn tay; con trỏ đang ẩn thì ảnh không có con trỏ và không báo lỗi; con trỏ phóng to trong Cài đặt Windows có thể ra cỡ thường (giới hạn đã biết); tắt ô thì ảnh không có con trỏ; giữ vị trí: bật ô, chụp và lưu một vùng, chụp lại thấy khung cũ đúng chỗ kèm thanh công cụ, Ctrl+S dùng được ngay, chỉnh khung và chọn lại được; Esc không đổi khung nhớ; khung trên màn hình phụ rồi rút màn hình đó: lớp phủ mở trống; thoát ứng dụng rồi mở lại: lớp phủ mở trống; tắt ô: lớp phủ mở trống.
- [ ] **Bước 9:** `powershell -File scripts/check.ps1` in `ALL CHECKS PASSED`. Commit `feat: full-screen hotkeys, cursor in screenshots, keep selection`.

**Xong khi:** check qua; kiểm thử chạy tay đã chạy; báo cáo nêu cách vẽ con trỏ và mọi handle được giải phóng ở đâu, lối vào duy nhất của yêu cầu chụp, bằng chứng rằng đường `initial = None` của lớp phủ không đổi, và mọi điểm của hành vi cũ mà bạn không chắc còn nguyên.
