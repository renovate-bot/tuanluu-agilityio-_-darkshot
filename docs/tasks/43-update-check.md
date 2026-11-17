# Task 43: Báo có bản mới

**Agent:** `builder` · **Model:** sonnet · **Nhánh:** `task/43-update-check` · **Rà soát:** `specialist`

Thay đổi nhỏ sau v0.2.0. Thiết kế đã duyệt: `docs/superpowers/specs/2026-11-17-update-check-design.md` (đọc toàn bộ, ngắn). Ràng buộc chung: mục "Global Constraints" của `docs/superpowers/plans/2026-11-17-annotate.md`. Rà soát do `specialist` làm vì task thêm một đích mạng và mở link trong trình duyệt.

## Cần đọc

- Thiết kế trên.
- `crates/darkshot-core/src/upload.rs`: cách dựng `ureq::Agent` có thời gian chờ; máy chủ thử cục bộ `TcpListener` và `PORT_LOCK` trong kiểm thử.
- `crates/darkshot-core/src/config.rs`: `Config`, `FileConfig`, `load`/`save`, các kiểm thử định dạng file.
- `crates/darkshot-core/src/i18n.rs`.
- `crates/darkshot-windows/src/main.rs`:
  - hẹn giờ (`SetTimer`, `WM_TIMER`, `RETRY_TIMER_ID`);
  - `WM_TRAY`;
  - luồng nền tải lên và cách nó báo kết quả về luồng chính;
  - luật ghi cấu hình của màu tự chọn (`custom_colors_need_save`, `store_custom_colors`, quyết định 72);
  - chỗ áp cấu hình sau khi Tùy chọn lưu.
- `crates/darkshot-windows/src/tray.rs`: menu, `MenuCommand`, `notify`, `WM_NOTICE`.
- `crates/darkshot-windows/src/settings.rs`: ô tích `check(b, Msg::ChkShowSuccessNotices, ID_NOTICES)` và các bước ở mục "Thêm một ô hoặc một tab" của `memory-bank/systemPatterns.md`.

## Interfaces

```rust
// crates/darkshot-core/src/update.rs   (pub mod update)
pub const LATEST_URL: &str =
    "https://api.github.com/repos/tuanluu-agilityio/darkshot/releases/latest";
pub const RELEASE_URL_PREFIX: &str = "https://github.com/tuanluu-agilityio/darkshot/";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version { pub major: u32, pub minor: u32, pub patch: u32 }
impl Version {
    /// "1.2.3" hoặc "v1.2.3" (đúng ba số, không thừa ký tự); khác thì None.
    pub fn parse(s: &str) -> Option<Version>;
}
impl std::fmt::Display for Version { /* "1.2.3" */ }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release { pub version: Version, pub url: String }

/// Đọc JSON của /releases/latest. None khi: JSON lỗi, thiếu tag_name hoặc html_url,
/// draft hoặc prerelease là true, tag không đọc được, html_url không bắt đầu bằng RELEASE_URL_PREFIX.
pub fn parse_latest(json: &str) -> Option<Release>;

/// Release mới hơn `current`, hoặc None.
pub fn newer_than(current: Version, latest: Option<Release>) -> Option<Release>;

/// GET `url` với User-Agent "darkshot/<current>" và Accept "application/vnd.github+json";
/// chờ kết nối 10 s, tổng 20 s. Mã 200 thì trả parse_latest(body); mọi lỗi khác trả None.
/// Đọc thân tối đa 256 KiB.
pub fn fetch_latest(url: &str, current: Version) -> Option<Release>;
```

`Config` có thêm hai trường:
- `pub check_updates: bool`, mặc định `true`.
- `pub update_notified: Option<Version>`, mặc định `None`.

Trong file cấu hình, hai trường lưu là `check_updates` (bool) và `update_notified` (chuỗi `"X.Y.Z"` hoặc `null`). Thiếu trường hoặc giá trị lạ thì dùng mặc định.

i18n:
- `Msg::ChkCheckUpdates`: "Tự kiểm tra bản mới" / "Check for updates automatically".
- `pub fn update_menu(lang, v: &str) -> String`: "Có bản mới {v}" / "New version {v} available".
- `Msg::UpdateNoticeTitle`: "Có bản darkshot mới" / "darkshot update available".
- `pub fn update_notice_body(lang, v: &str) -> String`: "Bản {v} đã có. Bấm để mở trang tải về." / "Version {v} is available. Click to open the download page."

Lớp Windows:
- **Luật thuần** (`main.rs`):
  ```rust
  pub fn update_outcome(found: Option<&Release>, notified: Option<Version>)
      -> (bool /*hiện mục menu*/, bool /*thông báo*/)
  ```
  - Có bản mới: luôn hiện mục menu.
  - Chỉ thông báo khi `notified != Some(found.version)`.
- **Hẹn giờ** `UPDATE_TIMER_ID`:
  - Đặt 60 s khi khởi động nếu `check_updates` bật.
  - Khi hẹn giờ chạy, đặt lại 24 giờ và mở luồng nền gọi `fetch_latest(LATEST_URL, current)`.
  - Kết quả gửi về luồng chính bằng `PostMessageW` với thông điệp riêng; làm theo cách luồng tải lên đang dùng để chuyển dữ liệu.
  - Không mở luồng mới khi một lần kiểm tra trước chưa xong.
- **Tùy chọn lưu:** đổi `check_updates` từ tắt sang bật thì đặt hẹn giờ 60 s. Đổi từ bật sang tắt thì `KillTimer`, bỏ mục menu và bỏ qua kết quả của lần kiểm tra đang chạy dở.
- **Kết quả về tới luồng chính** (bỏ qua nếu `check_updates` đã tắt):
  - Lưu `Option<Release>` hiện có trong trạng thái ứng dụng; menu khay đọc giá trị này.
  - Nếu cần thông báo: gọi `notify` với kiểu Info, không qua bộ lọc "thông báo thành công". Ghi `update_notified` vào cấu hình trong bộ nhớ. Ghi ra file theo đúng luật của quyết định 72, như `store_custom_colors`; không ghi được thì bỏ qua.
- **Menu khay:** khi có bản mới, thêm `update_menu(...)` làm mục đầu tiên, kèm `MF_SEPARATOR` ngay sau.
  - Thêm `MenuCommand::OpenUpdate` với ID mới.
  - Bấm mục này thì mở `release.url`.
- **Bấm vào thông báo** (`NIN_BALLOONUSERCLICK` trong `WM_TRAY`):
  - Mở link chỉ khi thông báo khay gần nhất là thông báo bản mới. Giữ một cờ, gán khi hiện thông báo bản mới và xóa khi hiện thông báo khác.
  - Các thông báo khác giữ nguyên hành vi hiện nay.
- **Mở link:** `ShellExecuteW(None, "open", url, None, None, SW_SHOWNORMAL)`.
  - Trước khi mở, kiểm lại `url.starts_with(RELEASE_URL_PREFIX)` (phòng thủ hai lớp).
  - Lỗi thì bỏ qua.
- **Tùy chọn, tab Thông thường:** ô tích `ID_CHECK_UPDATES` (chọn số chưa dùng), đặt ngay dưới ô "thông báo thành công". Theo đủ 5 bước của systemPatterns. Màu tối được lo sẵn vì ô tích dùng `NM_CUSTOMDRAW` như các ô khác.

## Review Focus

1. **Máy chủ trả dữ liệu lạ:** JSON rất lớn, sai kiểu, `tag_name` = "v1.2", `html_url` là `http://` hoặc thuộc repo khác, hay chứa `..`. Mong đợi: không panic, không mở link, không hiện menu.
2. **Không có mạng hoặc mạng treo:** không chặn giao diện; tắt ứng dụng trong lúc đang kiểm tra không bị treo, vì luồng nền có thời gian chờ và luồng chính không đợi nó.
3. **File cấu hình hỏng hoặc `SecretLost`:** việc ghi `update_notified` không được ghi đè file (quyết định 72). Thông báo chỉ được nhắc lại mỗi lần chạy.
4. **Tắt "Tự kiểm tra bản mới":** không còn yêu cầu mạng nào sau đó, kể cả kết quả của lần kiểm tra đang dở.
5. **Bản đang chạy mới hơn release** (bản dựng thử): không báo gì.

## Các bước

- [ ] **1. Kiểm thử trước** (commit `test: update check`, thân hàm mới `todo!()`):
  - `version_parse`:
    - "0.2.0", "v0.2.0" và "10.20.30" đọc đúng.
    - "", "v", "1.2", "1.2.3.4", "1.2.x", "v1.2.3-beta", " 1.2.3" và "01.2.3x" trả None. Số có số 0 đứng đầu như "01.2.3" thì chấp nhận và bằng 1.2.3.
    - So sánh: 0.10.0 > 0.9.9, 1.0.0 > 0.99.99.
    - `to_string()` của 0.2.0 là "0.2.0".
  - `parse_latest_cases`:
    - Mẫu JSON thật (rút gọn) có `tag_name` "v0.3.0" và `html_url` ".../darkshot/releases/tag/v0.3.0" → `Some`.
    - `draft:true` hoặc `prerelease:true` → None.
    - `html_url` có dạng "http://github.com/tuanluu-agilityio/darkshot/…", "https://github.com/other/darkshot/…", hoặc "https://github.com/tuanluu-agilityio/darkshot2/…" → None.
    - Thiếu trường, sai kiểu (`tag_name: 3`), JSON cắt dở, hoặc chuỗi rỗng → None.
  - `newer_than_cases`: bằng, cũ hơn, hoặc None → None; mới hơn → Some.
  - `fetch_latest_local`: dùng máy chủ cục bộ với `PORT_LOCK` như `upload.rs`.
    - Trả 200 với JSON hợp lệ → Some.
    - Máy chủ ghi nhận header `User-Agent` bằng "darkshot/0.2.0".
    - Trả 404 → None; trả 200 với thân 1 MiB → None.
    - Cổng đóng → None.
  - Config `check_updates_round_trip`: mặc định là `true`/`None`; save rồi load giữ đúng; file cũ không có hai trường → mặc định; `update_notified: "abc"` → None.
  - i18n: chữ đúng ở hai ngôn ngữ; `update_menu(Vi, "0.3.0") == "Có bản mới 0.3.0"`.
  - Windows `update_outcome_rules`:
    - None → (false, false).
    - Có bản mới, chưa báo → (true, true).
    - Đã báo đúng bản đó → (true, false).
    - Đã báo bản cũ hơn → (true, true).
  - Windows: kiểm thử menu khay (nếu `tray.rs` có hàm thuần dựng danh sách mục thì kiểm qua hàm đó; không có thì tách ra một hàm thuần): có bản mới thì mục đầu là `OpenUpdate`, kế đó là vạch ngăn.
- [ ] **2. Mã** theo "Interfaces".
- [ ] **3. Tài liệu.**
  - README: thêm mục "Cập nhật" nói rõ bốn ý:
    - ứng dụng tự hỏi GitHub khi khởi động và mỗi ngày;
    - chỉ gửi tên và phiên bản ứng dụng;
    - tắt bằng ô "Tự kiểm tra bản mới";
    - cách thay exe.

    Sửa mọi câu trong README nói ứng dụng chỉ kết nối tới R2.
  - `memory-bank/decisions.md`: thêm quyết định số kế tiếp. Nội dung: kiểm tra bản mới qua GitHub releases, đích mạng thứ ba `api.github.com` chỉ khi ô bật; thay phần mạng của quyết định 31; chủ dự án chọn. Ghi kèm mọi lựa chọn riêng của task.
  - Checklist (`docs/testing/manual-checklist.md`): thêm mục "Cập nhật" với các dòng `[ ]`:
    - chạy bản có phiên bản cũ hơn release mới nhất: sau khoảng 1 phút có thông báo, bấm vào thì mở đúng trang release;
    - menu khay có mục "Có bản mới";
    - mở lại ứng dụng thì không còn thông báo nhưng menu vẫn có mục đó;
    - tắt ô thì mục menu biến mất;
    - rút mạng thì không có lỗi nào hiện ra.
  - Nếu `scripts/check.ps1` hoặc một kiểm thử đang khẳng định "chỉ hai đích mạng", sửa cho đúng thiết kế mới và ghi lại.
- [ ] **4.** Chạy `cargo fmt --all`. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`. Commit `feat: update check`.

Quy trình chung:
- Không thêm crate. Có thể bật feature của crate `windows` nếu thiếu.
- Core không phụ thuộc `windows`.
- Không `unwrap`/`expect`/chỉ số có thể panic trên đường chạy thật. `unsafe` kèm `// SAFETY:`.
- Kiểm `git branch --show-current` trước khi commit. Không chuyển nhánh, không đẩy.
- Không tắt tiến trình `darkshot` không do mình khởi động. Build release thì dùng `--target-dir target/task43`.
- Không đọc hay ghi khóa R2 hay file cấu hình thật.
- Kiểm thử không gọi mạng thật: chỉ dùng máy chủ cục bộ.

**Xong khi:** check qua. Báo cáo nêu:
- các commit và dòng cuối của check;
- chỗ khác gói task;
- những gì chưa thử bằng mắt: thông báo thật, bấm thông báo, gọi GitHub thật.
