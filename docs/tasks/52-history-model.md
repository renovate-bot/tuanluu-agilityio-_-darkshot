# Task 52: Lịch sử – mô hình, chỉ mục JSON, tìm/lọc, định dạng, chuỗi i18n

**Agent:** `builder` · **Model:** sonnet · **Nhánh:** `task/52-history-model` (tách từ `feature/screenshot-history`) · **Rà soát:** `reviewer`

Thiết kế: `docs/superpowers/specs/2026-11-18-screenshot-history-design.md` mục 4, 6, 7 (đọc các mục này). Global Constraints: mục cùng tên của `docs/superpowers/plans/2026-11-18-screenshot-history.md`. Task này thuần lõi, không I/O, không đụng `darkshot-windows`.

## Cần đọc
`crates/darkshot-core/src/lib.rs`, `naming.rs` (kiểu `LocalTime`), `config.rs` (kiểu `Expiry`, cách đọc JSON khoan dung), `i18n.rs` (macro `messages!`, các hàm có tham số, kiểm thử bảng chuỗi).

## Files
Tạo `crates/darkshot-core/src/history.rs`; sửa `lib.rs` (`pub mod history;`), `i18n.rs` (chuỗi mới).

## Interfaces (hợp đồng với Task 53+)

```rust
pub const INDEX_VERSION: u32 = 1;
pub const MAX_ENTRIES: usize = 50;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind { Saved, Uploaded, Copied, UploadFallback }
impl Kind { pub fn as_str(self) -> &'static str /* "saved" "uploaded" "copied" "upload_fallback" */; pub fn from_str(s:&str)->Option<Kind>; pub fn label(self, lang: Lang) -> &'static str /* Msg::HistoryKind* */ }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filter { All, Saved, Uploaded, Copied }
impl Filter { pub fn accepts(self, k: Kind) -> bool /* Saved = Saved+UploadFallback */ }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub id: String,             // 13 hex (ms UNIX) + 8 hex ngẫu nhiên; chỉ [0-9a-f]
    pub kind: Kind,
    pub unix: u64,              // giây UTC
    pub local: LocalTime,       // giờ địa phương lúc chụp
    pub width: u32, pub height: u32,
    pub bytes: Option<u64>,
    pub path: Option<PathBuf>,  // tệp của người dùng (Saved, UploadFallback)
    pub managed: bool,          // có images/<id>.png do ứng dụng giữ
    pub url: Option<String>,
    pub expires: Option<String>,// "YYYY-MM-DD"
}
impl Entry {
    pub fn file_name(&self) -> Option<String>;               // tên tệp của `path`
    pub fn image_path(&self, root: &Path) -> Option<PathBuf>;// managed → root/images/<id>.png, nếu không `path`
    pub fn thumb_path(&self, root: &Path) -> PathBuf;        // root/thumbs/<id>.png
}
pub fn is_valid_id(id: &str) -> bool;                        // đúng 21 ký tự [0-9a-f]

pub fn new_id(unix_millis: u64) -> String;                   // dùng getrandom
pub fn expires_on(local: &LocalTime, expiry: Expiry) -> Option<String>; // D1→+1 ngày, D7, D30, Keep→None; tính lịch đúng năm nhuận
pub fn is_expired(expires: &str, today: &LocalTime) -> bool; // ngày hết hạn < hôm nay; chuỗi sai → false

/// Chỉ mục: nạp khoan dung. Trả (entries, số mục bị bỏ vì hỏng). JSON không đọc được → Err.
pub fn parse_index(text: &str) -> Result<(Vec<Entry>, usize), IndexError>;
pub fn serialize_index(entries: &[Entry]) -> String;         // {"version":1,"entries":[…]} đẹp, đủ trường
pub enum IndexError { Syntax, UnsupportedVersion(u32) }      // version > INDEX_VERSION → UnsupportedVersion

pub fn filter_entries<'a>(entries: &'a [Entry], filter: Filter, query: &str, lang: Lang) -> Vec<&'a Entry>;
pub fn matches_query(e: &Entry, query: &str, lang: Lang) -> bool;
pub fn fold(s: &str) -> String;   // hạ chữ thường + bỏ dấu tiếng Việt (à→a, đ→d, ...)

pub fn format_size(bytes: u64) -> String;                    // "0 B", "412 KB", "1,2 MB" (Vi dùng dấu phẩy) → nhận Lang: format_size(lang, bytes)
pub fn format_dims(w: u32, h: u32) -> String;                // "1920 × 1080"
pub fn format_when(lang: Lang, when: &LocalTime, now: &LocalTime) -> String; // "Hôm nay 14:32" / "Hôm qua 09:10" / "18/11/2026 14:32"
pub fn format_full(lang: Lang, when: &LocalTime) -> String;  // "18/11/2026 14:32:05"
```

Chữ ký `format_size` cuối cùng là `format_size(lang: Lang, bytes: u64)`. Mục tiêu `parse_index`: mỗi phần tử `entries` đọc riêng (`serde_json::Value`); phần tử thiếu `id` hợp lệ, `kind` lạ, kích thước 0, hoặc `path` không phải chuỗi thì bỏ và đếm, các phần tử còn lại giữ nguyên thứ tự. Id trùng: giữ cái đầu. `managed` mặc định false, `bytes/path/url/expires` tùy chọn. Trường lạ bỏ qua. Cắt ở `MAX_ENTRIES`.

## Chuỗi i18n (thêm vào `messages!`, đúng tên, đủ hai ngôn ngữ; Vi trước)

| Tên | Việt | English |
|---|---|---|
| TrayHistory | Lịch sử ảnh chụp | Screenshot history |
| HistoryTitle | Lịch sử ảnh chụp | Screenshot history |
| HistorySearchPlaceholder | Tìm theo tên, ngày hoặc liên kết | Search by name, date or link |
| HistorySearchClear | Xóa chữ tìm | Clear search |
| HistoryFilterAll / Saved / Uploaded / Copied | Tất cả / Đã lưu / Đã tải lên / Đã chép | All / Saved / Uploaded / Copied |
| HistoryKindSaved / Uploaded / Copied / Fallback | Đã lưu / Đã tải lên / Đã chép / Lưu dự phòng | Saved / Uploaded / Copied / Saved as fallback |
| HistoryClear | Xóa lịch sử | Clear history |
| HistoryOpen | Mở | Open |
| HistoryOpenLink | Mở liên kết | Open link |
| HistoryCopyImage | Sao chép ảnh | Copy image |
| HistoryCopyLink | Sao chép liên kết | Copy link |
| HistoryShowInFolder | Mở thư mục chứa ảnh | Show in folder |
| HistoryRemove | Xóa khỏi lịch sử | Remove from history |
| HistoryLabelType / Dimensions / FileSize / File / Link / Expires | Loại / Kích thước / Dung lượng / Tệp / Liên kết / Hết hạn | Type / Dimensions / File size / File / Link / Expires |
| HistoryExpired | Đã hết hạn | Expired |
| HistoryNeverExpires | Không hết hạn | Never expires |
| HistoryEmptyTitle | Chưa có ảnh nào | No screenshots yet |
| HistoryEmptyBody | Ảnh bạn chụp, lưu hoặc tải lên từ bây giờ sẽ xuất hiện ở đây. | Screenshots you capture, save or upload from now on will appear here. |
| HistoryDisabledTitle | Lịch sử đang tắt | History is turned off |
| HistoryDisabledBody | Bật "Bật lịch sử ảnh chụp" trong Tùy chọn để giữ lại ảnh bạn chụp. Ảnh chỉ nằm trên máy này. | Turn on "Enable Screenshot History" in Options to keep what you capture. Images stay on this device. |
| HistoryDisabledBanner | Lịch sử đang tắt: ảnh mới không được ghi lại. | History is off: new screenshots aren't being recorded. |
| HistoryOpenOptions | Mở Tùy chọn | Open Options |
| HistoryNoMatchTitle | Không tìm thấy ảnh nào | No matching screenshots |
| HistoryNoMatchBody | Thử từ khóa khác hoặc chọn bộ lọc Tất cả. | Try another keyword or switch the filter to All. |
| HistoryClearFilters | Xóa bộ lọc | Clear filters |
| HistorySelectPrompt | Chọn một ảnh để xem trước | Select a screenshot to preview |
| HistoryPreviewLoading | Đang tải bản xem trước… | Loading preview… |
| HistoryFileMissing | Không tìm thấy tệp ảnh | Image file not found |
| HistoryFileMissingHint | Tệp đã bị di chuyển hoặc xóa. Vẫn có thể xóa mục này khỏi lịch sử. | The file was moved or deleted. You can still remove this entry from history. |
| HistoryFileUnreadable | Không đọc được ảnh này | This image can't be read |
| HistoryClearConfirmTitle | Xóa toàn bộ lịch sử? | Clear all history? |
| HistoryClearConfirmYes | Xóa lịch sử | Clear history |
| HistoryCancel | Hủy | Cancel |
| HistoryStatusImageCopied | Đã sao chép ảnh | Image copied |
| HistoryStatusLinkCopied | Đã sao chép liên kết | Link copied |
| HistoryStatusRemoved | Đã xóa khỏi lịch sử | Removed from history |
| HistoryStatusCleared | Đã xóa lịch sử | History cleared |
| HistoryStatusFailed | Không thực hiện được thao tác này | That action didn't work |
| HistoryToday / HistoryYesterday | Hôm nay / Hôm qua | Today / Yesterday |
| ChkHistory | Bật lịch sử ảnh chụp | Enable Screenshot History |
| NoteHistory | Lưu lịch sử ảnh chụp trên thiết bị này. Ảnh không được gửi đến máy chủ bổ sung. | Keep a local history of screenshots on this device. Images are never sent to an additional server. |

Hàm có tham số: `history_clear_confirm_body(lang, n: usize) -> String` ("{n} mục sẽ bị gỡ khỏi lịch sử. Ảnh bạn đã lưu và các liên kết đã tải lên không bị xóa." / "{n} entries will be removed from history. Screenshots you saved and links you uploaded are not deleted."; n=1 dùng "1 mục"/"1 entry will be removed…").

Dấu `"` trong chuỗi Việt/Anh dùng dấu ngoặc kép thẳng đã thoát.

## Kiểm thử (viết trước, thành test thật)
- `Kind`/`Filter`: vòng `as_str`/`from_str`; `Filter::Saved` nhận `UploadFallback`, `Uploaded` không nhận `Saved`.
- `new_id`: `is_valid_id`, hai lần gọi khác nhau; `is_valid_id("../x")`, độ dài sai, chữ hoa → false.
- `expires_on`: 2026-11-18 + D7 = 2026-11-25; + D30 = 2026-12-18; 2026-12-20 + D30 = 2027-01-19; 2028-02-20 + D30 = 2028-03-21 (năm nhuận); Keep → None. `is_expired("2026-11-17", 2026-11-18)` true; ngày bằng hôm nay false; `"abc"` false.
- `parse_index`/`serialize_index`: vòng đầy đủ (mọi trường); thiếu `version` → coi là 1; `version:2` → `UnsupportedVersion(2)`; JSON hỏng → `Syntax`; phần tử hỏng bị bỏ và đếm; id trùng giữ đầu; hơn 50 phần tử bị cắt còn 50; `kind` lạ bị bỏ; trường lạ bị bỏ qua.
- Tìm: "luu" khớp mục có `path` tên "lưu_ảnh.png" (bỏ dấu); "2026-11-18" và "18/11/2026" khớp; "da tai len" khớp loại Uploaded ở Vi, "uploaded" ở En; nhiều từ AND; chuỗi rỗng khớp hết; khớp một phần URL; bộ lọc + tìm kết hợp, giữ thứ tự.
- Định dạng: `format_size(En, 0)` = "0 B", 1023 = "1023 B", 1024 = "1 KB", 421888 = "412 KB", 1_258_291 = "1.2 MB"; Vi dùng "1,2 MB". `format_dims(1920,1080)` = "1920 × 1080". `format_when`: cùng ngày "Hôm nay 14:32" (Vi) / "Today 14:32" (En); ngày hôm trước (kể cả qua ranh giới tháng và năm) "Hôm qua 09:10"; khác "18/11/2026 14:32" (Vi) và "2026-11-18 14:32" (En). `format_full` tương tự có giây.
- Kiểm thử chuỗi i18n có sẵn của repo phải qua (mọi `Msg` có hai bản, không rỗng).

## Việc cần chủ dự án thử tay
Không.

## Báo cáo
Theo mẫu của `.claude/agents/builder.md`.
