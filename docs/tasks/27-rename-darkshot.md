# Task 27: Đổi tên shotme thành darkshot

**Agent:** `builder` · **Model:** sonnet · **Nhánh:** `task/27-rename-darkshot` · **Rà soát:** `reviewer`

Gói task tự chứa. Thuộc kế hoạch `docs/superpowers/plans/2026-11-16-darkshot-dark-theme.md`. Chạy đầu tiên của đợt 4.

## Cần đọc

- Thiết kế đợt 4 `docs/superpowers/specs/2026-11-16-darkshot-dark-theme-design.md` mục 5
- Kế hoạch đợt 4 mục "Global Constraints"
- `scripts/check.ps1`, hai file `Cargo.toml` của crate, `Cargo.toml` gốc
- `crates/shotme-core/CLAUDE.md`, `crates/shotme-windows/CLAUDE.md`

## Global Constraints

- Tên mới viết thường: `darkshot`. Crate `darkshot-core` (thư viện `darkshot_core`), `darkshot-windows`, exe `darkshot.exe`.
- Không thêm crate; `Cargo.lock` chỉ đổi tên gói, không thêm gói. `darkshot-core` không phụ thuộc crate `windows`.
- Mọi chữ người dùng thấy lấy từ bảng `darkshot_core::i18n`, đủ hai ngôn ngữ.
- Không đổi hành vi nào ngoài tên. Không đổi giá trị mong đợi của kiểm thử trừ chữ `shotme` thành `darkshot`.
- Không sửa spec, kế hoạch đã duyệt, gói task cũ (`docs/tasks/01` đến `26`), `prototype/`, `memory-bank/progress.md` (sổ lịch sử).
- Không làm việc chuyển dữ liệu (Task 28): không đọc `%APPDATA%\shotme`, không xóa giá trị Run `shotme`.
- Release `panic = "abort"`; không `unwrap`/`expect` trên đường chạy thật.
- Commit kiểm thử trước (`test:`), rồi mã (`feat:`); với task đổi tên, commit `test:` gồm các kiểm thử đổi chuỗi mong đợi và kiểm thử mới ở bước 1 (chúng phải thất bại trước khi đổi). `cargo fmt --all`. Chạy cargo qua PowerShell. Kiểm `git branch --show-current` trước khi commit; không chuyển nhánh. Không tắt tiến trình `shotme`/`darkshot` không do mình khởi động; nếu exe bị khóa thì bỏ build release và báo lại. Không đẩy lên remote. Không đọc hay ghi khóa R2, file cấu hình thật, khóa Run thật. `powershell -File scripts/check.ps1` phải in `ALL CHECKS PASSED`.

## Interfaces

**Produces:** thư mục `crates/darkshot-core`, `crates/darkshot-windows`; mọi `use shotme_core::` thành `use darkshot_core::`; `paths::config_path()` trả `%APPDATA%\darkshot\config.json`; `paths::default_save_dir()` trả `Pictures\darkshot`; `naming::local_file_name` sinh `darkshot_YYYY-MM-DD_hh-mm-ss.png`; `autostart::VALUE_NAME == "darkshot"`; lớp cửa sổ `darkshot-hidden-window`, `darkshot-about`, `darkshot-help` (và mọi lớp `shotme-*` khác), mutex `Local\darkshot-single-instance`, luồng `darkshot-upload`; tiêu đề cửa sổ ẩn `darkshot`; file thử của Kiểm tra kết nối `d1/darkshot-test-<ngẫu nhiên>.txt` với nội dung `darkshot`; `res/darkshot.ico` (đổi tên bằng `git mv`, nội dung giữ nguyên), `app.rc` và `build.rs` trỏ tới nó; `wer.rs` mặc định `darkshot.exe`.

Bước kiểm tra mới trong `scripts/check.ps1` tên `no legacy name`: tìm `shotme` (không phân biệt hoa thường) trong `crates\**\*` (mọi file, kể cả `.rs`, `.toml`, `.rc`, `.md`, `.manifest`, `.py`) và `scripts\*`, trừ đúng file `crates\darkshot-windows\src\legacy.rs` và `crates\darkshot-core\src\migrate.rs` (Task 28 tạo). Có chỗ nào thì in `CHECK FAILED: legacy name in <file>:<dòng>` và thoát 1. Chính `check.ps1` không được chứa chữ đó ở dạng thường: viết mẫu tìm ghép từ hai nửa (ví dụ `'shot' + 'me'`). Task 31 mở rộng bước này sang `README.md` và `docs\testing\`.

## Các bước

- [ ] **1. Kiểm thử trước.** Commit `test: expect darkshot names`.
  - Sửa chuỗi mong đợi trong kiểm thử hiện có: `naming` (`darkshot_2026-11-16_14-03-05.png`, `_2`, `_3`), `actions`, `upload` (`PUT /bkt/d1/darkshot-test-`, thân `darkshot`), `about.rs` (`FindWindowW(w!("darkshot-about"))`), mọi thư mục tạm `shotme_*` thành `darkshot_*`.
  - Thêm `paths::tests::names_are_darkshot`: `config_path()` kết thúc bằng `darkshot\config.json`; `default_save_dir()` kết thúc bằng `darkshot`.
  - Thêm `autostart::tests::value_name_is_darkshot`: `VALUE_NAME == "darkshot"`.
  - Thêm `i18n::tests::no_legacy_name_in_strings` (core): với cả hai ngôn ngữ, không chuỗi nào của mọi `Msg`, `help_text` và `about_version` chứa `shotme` (không phân biệt hoa thường); mẫu tìm ghép từ hai nửa để bước kiểm tra mới không bắt chính nó. Nếu có thể, khẳng định thêm `help_text` chứa `darkshot`.
  - Đổi tên crate trong cùng commit này là được (để kiểm thử biên dịch), miễn mã chạy thật chưa đổi chuỗi.
- [ ] **2. Đổi tên.**
  - `git mv crates/shotme-core crates/darkshot-core`, `git mv crates/shotme-windows crates/darkshot-windows`; sửa `name` của hai package, `[[bin]] name = "darkshot"`, phụ thuộc `darkshot-core = { path = "../darkshot-core" }`; mọi `shotme_core` thành `darkshot_core`.
  - `git mv res/shotme.ico res/darkshot.ico`; sửa `app.rc`, `build.rs` (`rerun-if-changed`), `make_icon.py` (tên file ra), `app.manifest` (tên assembly nếu có).
  - Đổi mọi chuỗi và chú thích còn lại theo mục Produces; chuỗi trong `i18n.rs` (cả lời Trợ giúp, `AutostartExePath`, chữ ở Thông tin).
  - `scripts/check.ps1`: `cargo tree -p darkshot-core`; tiêu đề bước `darkshot-core dependencies`; thêm bước `no legacy name` ngay trước `ALL CHECKS PASSED`.
  - Đổi đường dẫn máy móc `crates/shotme-` thành `crates/darkshot-` và `shotme_core` thành `darkshot_core` trong `CLAUDE.md`, `crates/darkshot-*/CLAUDE.md`, `.claude/agents/*.md`, `.claude/commands/*.md`, `memory-bank/systemPatterns.md`, `memory-bank/techContext.md` (chỉ đường dẫn và tên crate; câu chữ để Task 31). Trong hai `CLAUDE.md` của crate, đổi luôn chữ `shotme` thành `darkshot` (chúng nằm trong `crates\`, bước kiểm tra mới quét tới).
- [ ] **3. Kiểm tra.** `cargo fmt --all`; `powershell -File scripts/check.ps1` in `ALL CHECKS PASSED`. Build release `cargo build --release` ra `target/release/darkshot.exe` (nếu bị khóa thì bỏ và báo). Commit `feat: rename shotme to darkshot`.

**Xong khi:** check qua; `git grep -i shotme -- crates scripts` không còn kết quả; báo cáo nêu các commit, dòng cuối của check, kích thước `darkshot.exe` nếu build được, và danh sách file ngoài `crates/` đã đổi đường dẫn.
