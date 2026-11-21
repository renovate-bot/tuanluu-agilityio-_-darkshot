//! Bảng chuỗi hai ngôn ngữ. Mỗi chuỗi phải có đủ bản Việt và bản English, thiếu là lỗi biên dịch.

use crate::config::Expiry;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    Vi,
    En,
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::Vi, Lang::En];

    pub fn code(self) -> &'static str {
        match self {
            Lang::Vi => "vi",
            Lang::En => "en",
        }
    }

    pub fn from_code(s: &str) -> Option<Lang> {
        Lang::ALL.into_iter().find(|l| l.code() == s)
    }

    pub fn native_name(self) -> &'static str {
        match self {
            Lang::Vi => "Tiếng Việt",
            Lang::En => "English",
        }
    }
}

macro_rules! messages {
    ($($name:ident => ($vi:expr, $en:expr),)*) => {
        /// Mỗi chuỗi không tham số một biến thể.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Msg { $($name,)* }

        impl Msg {
            pub const ALL: &'static [Msg] = &[$(Msg::$name,)*];
        }

        pub fn t(lang: Lang, m: Msg) -> &'static str {
            match (lang, m) {
                $(
                    (Lang::Vi, Msg::$name) => $vi,
                    (Lang::En, Msg::$name) => $en,
                )*
            }
        }
    };
}

messages! {
    TrayCapture => ("Chụp màn hình", "Take screenshot"),
    NoImageTitle => ("Không có ảnh để xử lý", "No image to process"),
    NoImageBody => ("Vùng chọn trống hoặc ảnh không hợp lệ.", "The selection is empty or the image is invalid."),
    SavedTitle => ("Đã lưu ảnh", "Image saved"),
    CopiedTitle => ("Đã chép ảnh", "Image copied"),
    CopiedBody => ("Ảnh đã nằm trong clipboard.", "The image is on the clipboard."),
    CopyFailedTitle => ("Không chép được ảnh", "Could not copy the image"),
    UploadedTitle => ("Đã chép link", "Link copied"),
    UploadedNoCopyTitle => ("Đã tải lên nhưng chưa chép được link", "Uploaded, but the link was not copied"),
    UploadFailedTitle => ("Không tải lên được", "Upload failed"),
    SaveFailedTitle => ("Không lưu được ảnh", "Could not save the image"),
    UploadNotConfigured => (
        "Chưa cấu hình kho lưu trữ. Hãy nhập đủ thông tin trong Tùy chọn.",
        "Storage is not configured. Enter all the details in Options."
    ),
    UploadNetwork => (
        "Không kết nối được tới máy chủ. Kiểm tra mạng rồi thử lại.",
        "Could not connect to the server. Check your network and try again."
    ),
    UploadAuth => (
        "Khóa truy cập không hợp lệ hoặc không đủ quyền. Kiểm tra lại khóa.",
        "The access key is invalid or lacks permission. Check the key."
    ),
    UploadBucketNotFound => (
        "Không tìm thấy kho. Kiểm tra lại tên kho và mã tài khoản.",
        "Bucket not found. Check the bucket name and account ID."
    ),
    UploadPublicUrl => (
        "Không đọc được ảnh qua địa chỉ công khai. Kiểm tra lại địa chỉ công khai của kho.",
        "The image could not be read through the public address. Check the bucket's public address."
    ),
    UploadClockSkew => (
        "Giờ máy không đúng. Hãy chỉnh lại ngày giờ của Windows rồi thử lại.",
        "The system clock is wrong. Fix the date and time in Windows and try again."
    ),
    ClaimTooMany => (
        "quá nhiều ảnh trùng tên trong cùng một giây",
        "too many images with the same name in one second"
    ),
    EncodeInvalidImage => (
        "ảnh rỗng hoặc sai độ dài dữ liệu",
        "the image is empty or its data length is wrong"
    ),
    UrlNotHttps => (
        "Địa chỉ công khai phải bắt đầu bằng https://",
        "The public address must start with https://"
    ),
    UrlMissingHost => (
        "Địa chỉ công khai thiếu tên miền sau https://",
        "The public address is missing a domain after https://"
    ),
    UrlWhitespace => (
        "Địa chỉ công khai không được chứa khoảng trắng",
        "The public address must not contain spaces"
    ),
    UrlForbiddenChar => (
        "Địa chỉ công khai không được chứa các ký tự ?, # hoặc @",
        "The public address must not contain ?, # or @"
    ),
    NoticeConfigCorrupt => (
        "File cấu hình bị hỏng. Đang dùng tùy chọn mặc định; bấm OK để ghi lại.",
        "The config file is corrupt. Using default options; click OK to write it again."
    ),
    NoticeSecretLost => (
        "Không đọc được khóa bí mật đã lưu. Hãy nhập lại Secret Access Key.",
        "Could not read the saved secret key. Enter the Secret Access Key again."
    ),
    NoticeConfigUnreadable => (
        "Không đọc được file cấu hình. Đang dùng tùy chọn mặc định; bấm OK để ghi lại.",
        "Could not read the config file. Using default options; click OK to write it again."
    ),
    NoticeMigrationIncompleteTitle => ("Chưa chuyển xong thiết lập", "Settings not fully moved"),
    NoticeMigrationIncomplete => (
        "Chưa chuyển được hết thiết lập từ phiên bản cũ. Hãy mở Tùy chọn để kiểm tra.",
        "Some settings from the previous version could not be moved. Open Options to check them."
    ),
    ClipboardUnusedOnExit => ("không dùng clipboard lúc thoát", "the clipboard is not used on exit"),
    TrayOptions => ("Tùy chọn...", "Options..."),
    TrayAbout => ("Thông tin...", "About..."),
    TrayHelp => ("Trợ giúp", "Help"),
    AboutTitle => ("Thông tin", "About"),
    HelpTitle => ("Trợ giúp", "Help"),
    BtnHelp => ("Trợ giúp", "Help"),
    BtnClose => ("Đóng", "Close"),
    AboutDescription => (
        "Chụp vùng màn hình, lưu vào máy hoặc tải lên kho Cloudflare R2 của chính bạn và chép link.",
        "Capture a region of the screen, save it locally or upload it to your own Cloudflare R2 bucket and copy the link."
    ),
    TrayExit => ("Thoát", "Exit"),
    OverlayUpload => ("Tải lên và chép link (Ctrl+D)", "Upload and copy link (Ctrl+D)"),
    OverlaySave => ("Lưu vào máy (Ctrl+S)", "Save to disk (Ctrl+S)"),
    OverlayCopy => ("Chép ảnh (Ctrl+C)", "Copy image (Ctrl+C)"),
    OverlayCancel => ("Hủy (Esc)", "Cancel (Esc)"),
    OverlayErrTooLarge => ("ảnh chụp quá lớn", "the screenshot is too large"),
    OverlayErrInvalid => ("ảnh chụp không hợp lệ", "the screenshot is invalid"),
    OverlayErrAlreadyOpen => ("lớp phủ đang mở", "the overlay is already open"),
    OverlayErrMemory => ("không đủ bộ nhớ cho lớp phủ", "not enough memory for the overlay"),
    OverlayErrWindow => ("không tạo được cửa sổ lớp phủ", "could not create the overlay window"),
    OverlayErrLoop => ("vòng lặp thông điệp lỗi", "the message loop failed"),
    CaptureFailedTitle => ("Không chụp được màn hình", "Could not capture the screen"),
    CaptureFailedBody => (
        "Windows không cho chụp lúc này (ví dụ màn hình khóa hoặc hộp thoại quyền quản trị). Hãy thử lại.",
        "Windows does not allow capturing right now (for example the lock screen or an administrator prompt). Try again."
    ),
    OverlayFailedTitle => ("Không mở được lớp phủ chọn vùng", "Could not open the region selector"),
    UploadStartFailedTitle => ("Không bắt đầu được việc tải lên", "Could not start the upload"),
    AutostartFailedTitle => ("Không đổi được khởi động cùng Windows", "Could not change the Start with Windows setting"),
    OptionsSaveFailedTitle => ("Không lưu được tùy chọn", "Could not save the options"),
    OptionsTitle => ("Tùy chọn", "Options"),
    ErrAccountChars => (
        "Account ID chỉ gồm chữ cái, chữ số và dấu gạch ngang.",
        "Account ID may only contain letters, digits and hyphens."
    ),
    ErrBucketChars => (
        "Tên kho chỉ gồm chữ cái, chữ số, dấu chấm, gạch dưới và gạch ngang.",
        "The bucket name may only contain letters, digits, dots, underscores and hyphens."
    ),
    ErrPickSaveDir => ("Hãy chọn thư mục lưu ảnh.", "Choose a folder to save images in."),
    HotkeyCapturePrompt => (
        "Nhấn tổ hợp phím mới… (Esc để bỏ qua)",
        "Press the new key combination... (Esc to skip)"
    ),
    HotkeyNeedModifier => (
        "Phím này cần kèm Ctrl, Alt hoặc Win. Thử lại…",
        "This key needs Ctrl, Alt or Win with it. Try again..."
    ),
    PickFolderTitle => ("Chọn thư mục lưu ảnh", "Choose the folder to save images in"),
    TestFillAll => (
        "Hãy điền đủ Account ID, Access Key ID, Secret Access Key, Tên kho và Địa chỉ công khai trước.",
        "Fill in Account ID, Access Key ID, Secret Access Key, Bucket and Public address first."
    ),
    TestSuccess => (
        "Thành công: tải lên được và link mở được.",
        "Success: the upload worked and the link opens."
    ),
    TestThreadFailed => (
        "Không tạo được luồng kiểm tra. Hãy thử lại.",
        "Could not start the check. Try again."
    ),
    TestRunning => ("Đang kiểm tra…", "Checking..."),
    TestTimeout => (
        "Kiểm tra quá thời gian chờ. Hãy thử lại.",
        "The check timed out. Try again."
    ),
    LabelBucket => ("Tên kho (bucket)", "Bucket"),
    SecretNone => ("Chưa có khóa.", "No key saved."),
    SecretSaved => (
        "Đã lưu khóa; để trống nếu không đổi.",
        "Key saved; leave empty to keep it."
    ),
    LabelPublicUrl => ("Địa chỉ công khai", "Public address"),
    BtnTest => ("Kiểm tra kết nối", "Test connection"),
    LabelExpiry => ("Hạn của link", "Link expiry"),
    LabelSaveDir => ("Thư mục lưu", "Save folder"),
    BtnChoose => ("Chọn…", "Choose..."),
    LabelHotkey => ("Chụp vùng màn hình", "Capture a region"),
    TabGeneral => ("Thông thường", "General"),
    TabHotkeys => ("Phím tắt", "Hotkeys"),
    TabR2 => ("Lưu trữ R2", "R2 storage"),
    LabelLanguage => ("Ngôn ngữ", "Language"),
    BtnChange => ("Đổi…", "Change..."),
    ChkAutostart => ("Khởi động cùng Windows", "Start with Windows"),
    BtnCancel => ("Hủy", "Cancel"),
    ClipInvalidImage => ("Ảnh không hợp lệ", "The image is invalid"),
    ClipImageTooBig => ("Ảnh quá lớn để chép vào clipboard", "The image is too large to copy to the clipboard"),
    ClipBusy => ("Clipboard đang bị ứng dụng khác giữ", "Another app is holding the clipboard"),
    ClipAllocFailed => ("Không cấp phát được bộ nhớ cho clipboard", "Could not allocate memory for the clipboard"),
    ClipWindowFailed => ("Không tạo được cửa sổ cho clipboard", "Could not create a window for the clipboard"),
    ClipSetFailed => ("Không đặt được dữ liệu vào clipboard", "Could not put the data on the clipboard"),
    ClipEmptyFailed => ("Không xóa được clipboard", "Could not clear the clipboard"),
    SecretEncryptFailed => ("Không mã hóa được khóa bí mật", "Could not encrypt the secret key"),
    SecretDecryptFailed => ("Không giải mã được khóa bí mật", "Could not decrypt the secret key"),
    SecretTooLarge => ("Dữ liệu quá lớn để mã hóa", "The data is too large to encrypt"),
    AutostartExePath => ("Không xác định được đường dẫn của darkshot", "Could not determine the path of darkshot"),
    AutostartEnableFailed => ("Không bật được khởi động cùng Windows", "Could not enable starting with Windows"),
    AutostartDisableFailed => ("Không tắt được khởi động cùng Windows", "Could not disable starting with Windows"),
    LabelHotkeySaveFull => ("Lưu ảnh (toàn màn hình)", "Save screenshot (full screen)"),
    LabelHotkeyUploadFull => ("Tải lên nhanh (toàn màn hình)", "Quick upload (full screen)"),
    LabelHotkeyRepeatLast => ("Chụp lại vùng trước", "Repeat last capture"),
    MenuRepeatLast => ("Chụp lại vùng trước", "Repeat last capture"),
    RepeatRegionGoneTitle => ("Không lặp lại được", "Couldn't repeat"),
    RepeatRegionGoneBody => (
        "Vùng trước không còn trên màn hình. Hãy chọn lại.",
        "The previous region is no longer on screen. Select it again."
    ),
    ChkShowSuccessNotices => (
        "Hiện các thông báo lưu và sao chép",
        "Show save and copy notifications"
    ),
    ChkCheckUpdates => ("Tự kiểm tra bản mới", "Check for updates automatically"),
    UpdateNoticeTitle => ("Có bản darkshot mới", "darkshot update available"),
    ChkKeepSelection => ("Giữ vị trí vùng đã chọn", "Remember the selected region's position"),
    ChkIncludeCursor => (
        "Bao gồm con trỏ chuột vào ảnh chụp màn hình",
        "Include the mouse pointer in screenshots"
    ),
    HotkeysNotWorkingTitle => ("Phím tắt chưa hoạt động", "Hotkeys are not working"),
    TabFormat => ("Định dạng", "Format"),
    LabelUploadFormat => ("Định dạng ảnh tải lên", "Upload image format"),
    RadioPng => (
        "PNG (nét nguyên gốc, file nặng hơn)",
        "PNG (original sharpness, larger file)"
    ),
    RadioJpeg => (
        "JPEG (file nhẹ hơn, giảm chút chất lượng)",
        "JPEG (smaller file, slightly lower quality)"
    ),
    LabelJpegQuality => ("Chất lượng JPEG", "JPEG quality"),
    NoteSavedAlwaysPng => ("Ảnh lưu vào máy luôn là PNG.", "Images saved to disk are always PNG."),
    EncodeTooLargeForJpeg => (
        "Ảnh quá lớn để nén JPEG (mỗi cạnh tối đa 65.535 pixel)",
        "The image is too large for JPEG (each side at most 65,535 pixels)"
    ),
    AutostartKeyFailed => ("Không mở được khóa khởi động trong registry", "Could not open the startup key in the registry"),
    ToolPen => ("Bút vẽ (P)", "Pen (P)"),
    ToolLine => ("Đường thẳng (L)", "Line (L)"),
    ToolArrow => ("Mũi tên (A)", "Arrow (A)"),
    ToolRect => ("Hình chữ nhật (R)", "Rectangle (R)"),
    ToolMarker => ("Bút dạ quang (H)", "Highlighter (H)"),
    ToolBlur => ("Làm mờ (B)", "Blur (B)"),
    ToolText => ("Chữ (T)", "Text (T)"),
    ToolColor => ("Màu", "Color"),
    ToolUndo => ("Hoàn tác (Ctrl+Z)", "Undo (Ctrl+Z)"),
    ToolSelect => ("Chọn (V)", "Select (V)"),
    ToolEllipse => ("Hình elip (E)", "Ellipse (E)"),
    ToolStep => ("Đánh số (N)", "Numbered steps (N)"),
    ToolSize => ("Cỡ ([ ])", "Size ([ ])"),
    ToolRedo => ("Làm lại (Ctrl+Y)", "Redo (Ctrl+Y)"),
    ToolShapes => ("Hình (L, A, R, E)", "Shapes (L, A, R, E)"),
    SizeSmall => ("Nhỏ", "Small"),
    SizeMedium => ("Vừa", "Medium"),
    SizeLarge => ("Lớn", "Large"),
    SizeHuge => ("Rất lớn", "Extra large"),
    AnnotateFailedTitle => ("Không xử lý được ảnh", "Could not process the image"),
    AnnotateFailedBody => (
        "Không vẽ được các nét đánh dấu lên ảnh nên ảnh chưa được lưu, chép hay tải lên. Hãy chụp lại.",
        "Could not draw the markings onto the image, so it was not saved, copied or uploaded. Please capture again."
    ),
    PickerBasic => ("Màu cơ bản", "Basic colors"),
    PickerCustom => ("Màu tự chọn", "Custom colors"),
    PickerDefine => ("Định nghĩa màu tự chọn >>", "Define custom colors >>"),
    PickerAdd => ("Thêm vào màu tự chọn", "Add to custom colors"),
    PickerHue => ("Sắc độ", "Hue"),
    PickerSat => ("Bão hòa", "Sat"),
    PickerLum => ("Độ sáng", "Lum"),
    PickerRed => ("Đỏ", "Red"),
    PickerGreen => ("Lục", "Green"),
    PickerBlue => ("Lam", "Blue"),
    TrayHistory => ("Lịch sử ảnh chụp", "Screenshot history"),
    HistoryTitle => ("Lịch sử ảnh chụp", "Screenshot history"),
    HistorySearchPlaceholder => ("Tìm theo tên, ngày hoặc liên kết", "Search by name, date or link"),
    HistorySearchClear => ("Xóa chữ tìm", "Clear search"),
    HistoryFilterAll => ("Tất cả", "All"),
    HistoryFilterSaved => ("Đã lưu", "Saved"),
    HistoryFilterUploaded => ("Đã tải lên", "Uploaded"),
    HistoryFilterCopied => ("Đã chép", "Copied"),
    HistoryKindSaved => ("Đã lưu", "Saved"),
    HistoryKindUploaded => ("Đã tải lên", "Uploaded"),
    HistoryKindCopied => ("Đã chép", "Copied"),
    HistoryKindFallback => ("Lưu dự phòng", "Saved as fallback"),
    HistoryClear => ("Xóa lịch sử", "Clear history"),
    HistoryOpen => ("Mở", "Open"),
    HistoryOpenLink => ("Mở liên kết", "Open link"),
    HistoryCopyImage => ("Sao chép ảnh", "Copy image"),
    HistoryCopyLink => ("Sao chép liên kết", "Copy link"),
    HistoryShowInFolder => ("Mở thư mục chứa ảnh", "Show in folder"),
    HistoryRemove => ("Xóa khỏi lịch sử", "Remove from history"),
    HistoryLabelType => ("Loại", "Type"),
    HistoryLabelDimensions => ("Kích thước", "Dimensions"),
    HistoryLabelFileSize => ("Dung lượng", "File size"),
    HistoryLabelFile => ("Tệp", "File"),
    HistoryLabelLink => ("Liên kết", "Link"),
    HistoryLabelExpires => ("Hết hạn", "Expires"),
    HistoryExpired => ("Đã hết hạn", "Expired"),
    HistoryNeverExpires => ("Không hết hạn", "Never expires"),
    HistoryEmptyTitle => ("Chưa có ảnh nào", "No screenshots yet"),
    HistoryEmptyBody => (
        "Ảnh bạn chụp, lưu hoặc tải lên từ bây giờ sẽ xuất hiện ở đây.",
        "Screenshots you capture, save or upload from now on will appear here."
    ),
    HistoryDisabledTitle => ("Lịch sử đang tắt", "History is turned off"),
    HistoryDisabledBody => (
        "Bật \"Bật lịch sử ảnh chụp\" trong Tùy chọn để giữ lại ảnh bạn chụp. Ảnh chỉ nằm trên máy này.",
        "Turn on \"Enable Screenshot History\" in Options to keep what you capture. Images stay on this device."
    ),
    HistoryDisabledBanner => (
        "Lịch sử đang tắt: ảnh mới không được ghi lại.",
        "History is off: new screenshots aren't being recorded."
    ),
    HistoryOpenOptions => ("Mở Tùy chọn", "Open Options"),
    HistoryNoMatchTitle => ("Không tìm thấy ảnh nào", "No matching screenshots"),
    HistoryNoMatchBody => (
        "Thử từ khóa khác hoặc chọn bộ lọc Tất cả.",
        "Try another keyword or switch the filter to All."
    ),
    HistoryClearFilters => ("Xóa bộ lọc", "Clear filters"),
    HistorySelectPrompt => ("Chọn một ảnh để xem trước", "Select a screenshot to preview"),
    HistoryPreviewLoading => ("Đang tải bản xem trước…", "Loading preview…"),
    HistoryFileMissing => ("Không tìm thấy tệp ảnh", "Image file not found"),
    OverlayPin => ("Ghim lên màn hình (Ctrl+P)", "Pin to screen (Ctrl+P)"),
    PinWindowTitle => ("darkshot — Ảnh ghim", "darkshot — Pinned image"),
    PinMenuCopy => ("Sao chép ảnh\tCtrl+C", "Copy image\tCtrl+C"),
    PinMenuSave => ("Lưu ảnh\tCtrl+S", "Save image\tCtrl+S"),
    PinMenuZoom => ("Thu phóng", "Zoom"),
    PinMenuOpacity => ("Độ mờ", "Opacity"),
    PinMenuClose => ("Đóng\tEsc", "Close\tEsc"),
    PinMenuCloseAll => ("Đóng mọi ảnh ghim", "Close all pinned images"),
    HistoryPin => ("Ghim", "Pin"),
    OverlayOcr => ("Nhận dạng chữ (Ctrl+T)", "Recognize text (Ctrl+T)"),
    PinMenuOcr => ("Nhận dạng chữ", "Recognize text"),
    OcrWindowTitle => ("Chữ nhận dạng", "Recognized text"),
    OcrBusy => ("Đang nhận dạng chữ…", "Recognizing text…"),
    OcrEmpty => ("Không tìm thấy chữ trong ảnh.", "No text found in the image."),
    OcrNoLanguage => (
        "Windows chưa có gói nhận dạng chữ cho ngôn ngữ của bạn. Thêm ngôn ngữ trong Cài đặt > Thời gian & ngôn ngữ > Ngôn ngữ & khu vực.",
        "Windows has no text recognition pack for your languages. Add a language in Settings > Time & language > Language & region."
    ),
    OcrFailed => ("Không nhận dạng được chữ. Thử lại.", "Couldn't recognize text. Try again."),
    OcrCopyAll => ("Chép tất cả", "Copy all"),
    OcrSingleLine => ("Gộp thành một dòng", "Single line"),
    OcrParagraphs => ("Tách đoạn", "Separate paragraphs"),
    OcrAutoCopy => ("Tự chép khi xong", "Copy automatically when done"),
    OcrPickHint => (
        "Bấm khung để chép dòng · Ctrl+bấm để chọn nhiều dòng",
        "Click a box to copy the line · Ctrl+click to pick several lines"
    ),
    OcrCopiedTip => ("Đã chép", "Copied"),
    OcrClose => ("Đóng", "Close"),
    OcrCopiedTitle => ("Đã chép chữ", "Text copied"),
    OcrCopyFailedTitle => ("Không chép được chữ", "Couldn't copy the text"),
    PinLimitTitle => ("Đã đủ ảnh ghim", "Too many pinned images"),
    PinFailedTitle => ("Không ghim được ảnh", "Couldn't pin the image"),
    PinFailedBody => (
        "Không tạo được cửa sổ ảnh ghim. Thử lại.",
        "Couldn't create the pinned image window. Try again."
    ),
    PinTooLargeBody => ("Ảnh quá lớn để ghim.", "The image is too large to pin."),
    HistoryFileMissingHint => (
        "Tệp đã bị di chuyển hoặc xóa. Vẫn có thể xóa mục này khỏi lịch sử.",
        "The file was moved or deleted. You can still remove this entry from history."
    ),
    HistoryFileUnreadable => ("Không đọc được ảnh này", "This image can't be read"),
    HistoryClearConfirmTitle => ("Xóa toàn bộ lịch sử?", "Clear all history?"),
    HistoryClearConfirmYes => ("Xóa lịch sử", "Clear history"),
    HistoryCancel => ("Hủy", "Cancel"),
    HistoryStatusImageCopied => ("Đã sao chép ảnh", "Image copied"),
    HistoryStatusLinkCopied => ("Đã sao chép liên kết", "Link copied"),
    HistoryStatusRemoved => ("Đã xóa khỏi lịch sử", "Removed from history"),
    HistoryStatusCleared => ("Đã xóa lịch sử", "History cleared"),
    HistoryStatusFailed => ("Không thực hiện được thao tác này", "That action didn't work"),
    HistoryToday => ("Hôm nay", "Today"),
    HistoryYesterday => ("Hôm qua", "Yesterday"),
    ChkHistory => ("Bật lịch sử ảnh chụp", "Enable Screenshot History"),
    ChkSnapWindows => ("Gợi ý cửa sổ khi chọn vùng", "Suggest windows while selecting"),
    OverlayHintSnap => (
        "Bấm: chọn cửa sổ · Kéo: chọn vùng · Esc: thoát",
        "Click: window · Drag: region · Esc: cancel"
    ),
    OverlayHintDrag => ("Kéo để chọn vùng · Esc để thoát", "Drag to select · Esc to cancel"),
    NoteHistory => (
        "Lưu lịch sử ảnh chụp trên thiết bị này. Ảnh không được gửi đến máy chủ bổ sung.",
        "Keep a local history of screenshots on this device. Images are never sent to an additional server."
    ),
    TrayStepsStart => ("Chụp từng bước", "Step capture"),
    OverlayAddStep => ("Thêm bước (Ctrl+N)", "Add step (Ctrl+N)"),
    StepsBarTitle => ("Chụp từng bước", "Step capture"),
    StepsCapture => ("Chụp bước", "Capture step"),
    StepsReview => ("Xem lại", "Review"),
    StepsFinish => ("Kết thúc", "Finish"),
    StepsWindowTitle => ("Các bước", "Steps"),
    StepsCaption => ("Ghi chú", "Caption"),
    StepsUp => ("Lên", "Up"),
    StepsDown => ("Xuống", "Down"),
    StepsDelete => ("Xóa bước", "Delete step"),
    StepsExport => ("Xuất…", "Export…"),
    StepsEndSession => ("Kết thúc phiên", "End session"),
    StepsClose => ("Đóng", "Close"),
    StepsEmpty => (
        "Chưa có bước nào. Bấm Chụp bước trên thanh phiên.",
        "No steps yet. Click Capture step on the session bar."
    ),
    StepsEndConfirmYes => ("Kết thúc", "End"),
    StepsCancel => ("Hủy", "Cancel"),
    StepsFullTitle => ("Đã đủ 99 bước", "99 steps reached"),
    StepsFullBody => (
        "Xuất hoặc kết thúc phiên để chụp thêm.",
        "Export or end the session to capture more."
    ),
    StepsExportFailedTitle => ("Không xuất được các bước", "Couldn't export the steps"),
    StepsExportCancelledTitle => ("Đã hủy xuất", "Export cancelled"),
    StepsExportCancelledBody => ("Chưa có tệp nào được tạo.", "No files were created."),
    StepsImageUnreadable => ("Không đọc được ảnh của bước.", "Couldn't read the step image."),
    StepsStartFailedTitle => (
        "Không bắt đầu được phiên chụp từng bước",
        "Couldn't start step capture"
    ),
    StepsSessionSaveFailedTitle => (
        "Không lưu được phiên chụp từng bước",
        "Couldn't save the step capture session"
    ),
    StepsWorkerFailed => (
        "Không chạy được tác vụ nền.",
        "The background task couldn't run."
    ),
    StepsEndFailedTitle => ("Không xóa hết ảnh của phiên", "Couldn't delete all session images"),
    OverlayScroll => ("Chụp cuộn (Ctrl+L)", "Scrolling capture (Ctrl+L)"),
    ScrollNoAnnotations => ("Không kèm hình đánh dấu", "Annotations are not included"),
    ScrollBarTitle => ("Chụp cuộn", "Scrolling capture"),
    ScrollHint => ("Cuộn xuống từ từ", "Scroll down slowly"),
    ScrollSlowDown => ("Cuộn chậm hơn", "Scroll more slowly"),
    ScrollDone => ("Xong", "Done"),
    ScrollCancel => ("Hủy", "Cancel"),
    ScrollLimitTitle => ("Đã đạt giới hạn chiều cao", "Height limit reached"),
    ScrollNoRoomTitle => ("Không có chỗ cho thanh chụp cuộn", "No room for the scrolling capture bar"),
    ScrollNoRoomBody => ("Chọn vùng nhỏ hơn màn hình.", "Select a region smaller than the screen."),
    ScrollResultTitle => ("Ảnh chụp cuộn", "Scrolling capture"),
    ScrollSave => ("Lưu", "Save"),
    ScrollCopy => ("Chép", "Copy"),
    ScrollUpload => ("Tải lên", "Upload"),
    ScrollPin => ("Ghim", "Pin"),
    ScrollClose => ("Đóng", "Close"),
    ScrollManyOpenTitle => ("Có nhiều ảnh chụp cuộn chưa lưu", "Several scrolling captures are not saved"),
    ScrollManyOpenBody => (
        "Lưu hoặc đóng bớt các cửa sổ Ảnh chụp cuộn.",
        "Save or close some Scrolling capture windows."
    ),
    ScrollPreparing => ("Đang chuẩn bị…", "Preparing…"),
    ScrollCopyMemoryTitle => ("Không đủ bộ nhớ", "Not enough memory"),
    ScrollCopyMemoryBody => ("Đóng bớt cửa sổ rồi thử lại.", "Close some windows and try again."),
    OverlayRecordGif => ("Ghi GIF (Ctrl+G)", "Record GIF (Ctrl+G)"),
    RecordStatus => ("Đang ghi", "Recording"),
    RecordSaving => ("Đang lưu…", "Saving…"),
    RecordNotScanned => ("GIF không được quét", "GIF is not scanned"),
    RecordStop => ("Dừng", "Stop"),
    RecordCancel => ("Hủy", "Cancel"),
    RecordSavedTitle => ("Đã lưu GIF", "GIF saved"),
    RecordFailedTitle => ("Ghi GIF thất bại", "Recording failed"),
    RecordInvalidRegion => ("Vùng ghi không hợp lệ.", "Invalid recording region."),
    RecordTooLarge => ("Vùng ghi quá lớn.", "The recording region is too large."),
    RecordNoFrames => ("Chưa ghi được khung hình nào.", "No frames were recorded."),
    RecordMaxDuration => ("Đã đạt thời lượng ghi tối đa (60 giây).", "Maximum recording length reached (60 seconds)."),
    RecordMaxSize => ("Tệp GIF đã đạt kích thước tối đa (256 MB).", "The GIF file reached its maximum size (256 MB)."),
    RecordSourceLost => ("Không chụp tiếp được vùng ghi nên đã dừng.", "The recording region could no longer be captured, so recording stopped."),
    RecordEncodeFailed => ("Không mã hóa được GIF", "Could not encode the GIF"),
    RecordFaultRandom => ("Không lấy được số ngẫu nhiên từ hệ điều hành.", "The operating system's random source is unavailable."),
    RecordFaultNoTempName => ("Không tạo được tên tệp tạm.", "Could not create a temporary file name."),
    RecordFaultTempClosed => ("Tệp tạm đã đóng.", "The temporary file is already closed."),
    RecordFaultNoFileName => ("Không tìm được tên tệp còn trống.", "Could not find a free file name."),
    RecordFaultPanicked => ("Luồng ghi GIF gặp lỗi không mong đợi.", "The GIF recording thread failed unexpectedly."),
    RecordFaultThreadEnded => ("Luồng ghi GIF dừng mà không có kết quả.", "The GIF recording thread ended without a result."),
    RecordFaultSizeChanged => ("Kích thước vùng ghi đã thay đổi.", "The size of the recording region changed."),
    RecordFaultInternal => ("Lỗi nội bộ của bộ ghi GIF.", "Internal error in the GIF recorder."),
    RecordNoRoomTitle => ("Không có chỗ cho thanh ghi GIF", "No room for the recording bar"),
    PrivacyCatPhone => ("Số điện thoại", "Phone number"),
    PrivacyCatCard => ("Thẻ thanh toán", "Payment card"),
    PrivacyCatApiKey => ("Khóa API", "API key"),
    PrivacyCatToken => ("Token truy cập", "Access token"),
    PrivacyCatPassword => ("Mật khẩu", "Password"),
    PrivacyCatPrivateKey => ("Khóa riêng", "Private key"),
    PrivacyConfHigh => ("Cao", "High"),
    PrivacyConfMedium => ("Trung bình", "Medium"),
    PrivacyErrNoLanguage => ("Windows chưa có ngôn ngữ nhận dạng chữ nào.", "Windows has no text recognition language installed."),
    PrivacyErrFailed => ("Không nhận dạng được chữ trong ảnh.", "Could not recognize text in the image."),
    PrivacyErrTimeout => ("Quét quá lâu.", "Scanning took too long."),
    PrivacyErrTooLarge => ("Ảnh quá lớn để quét.", "The image is too large to scan."),
    PrivacyReviewTitle => ("Kiểm tra thông tin nhạy cảm", "Check for sensitive information"),
    PrivacyScanning => ("Đang quét thông tin nhạy cảm…", "Scanning for sensitive information…"),
    PrivacySelectAll => ("Chọn tất cả", "Select all"),
    PrivacyDeselectAll => ("Bỏ chọn hết", "Deselect all"),
    PrivacyManual => ("Vùng tự thêm", "Added area"),
    PrivacyRemove => ("Xóa", "Remove"),
    PrivacyNote => ("Tự động có thể bỏ sót. Hãy xem lại ảnh.", "Automatic detection can miss things. Review the image."),
    PrivacyCancel => ("Hủy", "Cancel"),
    PrivacyAddArea => ("+ Thêm vùng che", "+ Add cover area"),
    PrivacyApply => ("Che và tiếp tục", "Cover and continue"),
    PrivacyDragHint => ("Kéo trên ảnh để thêm vùng che · Esc để thôi", "Drag on the image to add a cover area · Esc to stop"),
    PrivacyConfirmEmpty => ("Không có vùng nào được che. Tiếp tục mà không che?", "Nothing will be covered. Continue without covering?"),
    PrivacyBack => ("Quay lại", "Go back"),
    PrivacyContinueNone => ("Tiếp tục không che", "Continue without covering"),
    PrivacyErrHeading => ("Không quét được ảnh này", "Could not scan this image"),
    PrivacyErrUnchecked => ("Ảnh CHƯA được kiểm tra.", "The image has NOT been checked."),
    PrivacyRetry => ("Thử lại", "Try again"),
    PrivacyContinueUnscanned => ("Tiếp tục không quét", "Continue without scanning"),
    PrivacyConfirmUnscanned => ("Ảnh chưa được kiểm tra. Vẫn tiếp tục?", "The image has not been checked. Continue anyway?"),
    PrivacyContinueAnyway => ("Vẫn tiếp tục", "Continue anyway"),
    PrivacyActionSave => ("Lưu ảnh", "Save image"),
    PrivacyActionCopy => ("Chép ảnh", "Copy image"),
    PrivacyActionUpload => ("Tải lên và chép link", "Upload and copy link"),
    PrivacyActionPin => ("Ghim lên màn hình", "Pin to screen"),
    PrivacyActionAddStep => ("Thêm bước", "Add step"),
    PrivacyActionExportSteps => ("Xuất các bước", "Export steps"),
    ChkPrivacyShield => (
        "Quét thông tin nhạy cảm trước khi xuất ảnh",
        "Scan for sensitive information before exporting"
    ),
    NotePrivacyShield => (
        "Nhận dạng chữ trên máy rồi che email, số điện thoại, thẻ, khóa API, token, mật khẩu, khóa riêng. Không phát hiện được mọi thứ; nên tự xem lại ảnh.",
        "Reads text on this PC, then covers emails, phone numbers, cards, API keys, tokens, passwords and private keys. It can miss things; review your image."
    ),
    PrivacyQueueFull => (
        "Đang có ảnh chờ kiểm tra. Ảnh mới chưa được xuất.",
        "Images are already waiting to be checked. The new image was not exported."
    ),
    PrivacyWindowFailed => (
        "Không mở được cửa sổ kiểm tra; ảnh chưa được xuất.",
        "Could not open the review window; the image was not exported."
    ),
    PrivacyScanStalled => (
        "Quét quá lâu; ảnh chưa được xuất.",
        "Scanning took too long; the image was not exported."
    ),
    PrivacyStepSessionGone => (
        "Phiên chụp từng bước đã kết thúc hoặc đã đổi; bước chưa được thêm.",
        "The step capture session ended or changed; the step was not added."
    ),
    PrivacyRedactFailed => (
        "Không che được ảnh; ảnh chưa được xuất.",
        "Could not cover the image; it was not exported."
    ),
}

/// Mọi mục dùng làm tiêu đề thông báo.
pub const NOTICE_TITLES: &[Msg] = &[
    Msg::NoImageTitle,
    Msg::SavedTitle,
    Msg::CopiedTitle,
    Msg::CopyFailedTitle,
    Msg::UploadedTitle,
    Msg::UploadedNoCopyTitle,
    Msg::UploadFailedTitle,
    Msg::SaveFailedTitle,
    Msg::CaptureFailedTitle,
    Msg::OverlayFailedTitle,
    Msg::UploadStartFailedTitle,
    Msg::AutostartFailedTitle,
    Msg::OptionsSaveFailedTitle,
    Msg::HotkeysNotWorkingTitle,
    Msg::AnnotateFailedTitle,
    Msg::OcrCopiedTitle,
    Msg::OcrCopyFailedTitle,
    Msg::StepsFullTitle,
    Msg::StepsExportFailedTitle,
    Msg::StepsExportCancelledTitle,
    Msg::StepsStartFailedTitle,
    Msg::StepsSessionSaveFailedTitle,
    Msg::StepsEndFailedTitle,
    Msg::ScrollLimitTitle,
    Msg::ScrollNoRoomTitle,
    Msg::ScrollManyOpenTitle,
    Msg::ScrollCopyMemoryTitle,
    Msg::RecordSavedTitle,
    Msg::RecordFailedTitle,
    Msg::RecordNoRoomTitle,
];

/// Chữ trên nút OK: giống nhau ở mọi ngôn ngữ nên nằm ngoài bảng `Msg`
/// (bảng đòi hai bản khác nhau).
pub const OK_LABEL: &str = "OK";

/// Tên loại phát hiện "Email": giống nhau ở mọi ngôn ngữ nên nằm ngoài bảng `Msg`.
pub const PRIVACY_EMAIL_LABEL: &str = "Email";

pub fn history_clear_confirm_body(lang: Lang, n: usize) -> String {
    match (lang, n) {
        (Lang::Vi, n) => format!(
            "{n} mục sẽ bị gỡ khỏi lịch sử. Ảnh bạn đã lưu và các liên kết đã tải lên không bị xóa."
        ),
        (Lang::En, 1) => "1 entry will be removed from history. Screenshots you saved and links you uploaded are not deleted.".to_string(),
        (Lang::En, n) => format!(
            "{n} entries will be removed from history. Screenshots you saved and links you uploaded are not deleted."
        ),
    }
}

pub fn expiry_label(lang: Lang, e: Expiry) -> &'static str {
    match (lang, e) {
        (Lang::Vi, Expiry::D1) => "1 ngày",
        (Lang::Vi, Expiry::D7) => "7 ngày",
        (Lang::Vi, Expiry::D30) => "30 ngày",
        (Lang::Vi, Expiry::Keep) => "Không hết hạn",
        (Lang::En, Expiry::D1) => "1 day",
        (Lang::En, Expiry::D7) => "7 days",
        (Lang::En, Expiry::D30) => "30 days",
        (Lang::En, Expiry::Keep) => "Never expires",
    }
}

pub fn copy_failed_body(lang: Lang, err: &str) -> String {
    match lang {
        Lang::Vi => format!("Không đưa được ảnh vào clipboard: {err}"),
        Lang::En => format!("Could not put the image on the clipboard: {err}"),
    }
}

pub fn ocr_copy_failed_body(lang: Lang, err: &str) -> String {
    match lang {
        Lang::Vi => format!("Không đưa được chữ vào clipboard: {err}"),
        Lang::En => format!("Could not put the text on the clipboard: {err}"),
    }
}

pub fn ocr_status(lang: Lang, name: &str, lines: usize) -> String {
    match lang {
        Lang::Vi => format!("Ngôn ngữ: {name} · {lines} dòng"),
        Lang::En => format!("Language: {name} · {lines} lines"),
    }
}

pub fn pin_limit_body(lang: Lang, max: usize) -> String {
    match lang {
        Lang::Vi => format!("Đã đủ {max} ảnh ghim. Đóng bớt rồi thử lại."),
        Lang::En => format!("You already have {max} pinned images. Close some and try again."),
    }
}

pub fn tray_close_pins(lang: Lang, n: usize) -> String {
    match lang {
        Lang::Vi => format!("Đóng mọi ảnh ghim ({n})"),
        Lang::En => format!("Close all pinned images ({n})"),
    }
}

pub fn pin_opacity_badge(lang: Lang, p: u8) -> String {
    match lang {
        Lang::Vi => format!("Độ mờ {p}%"),
        Lang::En => format!("Opacity {p}%"),
    }
}

pub fn encode_failed(lang: Lang, err: &str) -> String {
    match lang {
        Lang::Vi => format!("Không mã hóa được ảnh: {err}"),
        Lang::En => format!("Could not encode the image: {err}"),
    }
}

pub fn encoder_error(lang: Lang, err: &str) -> String {
    match lang {
        Lang::Vi => format!("lỗi bộ mã hóa ảnh: {err}"),
        Lang::En => format!("Image encoder error: {err}"),
    }
}

pub fn upload_failed_body(lang: Lang, reason: &str, path: &str) -> String {
    match lang {
        Lang::Vi => format!(
            "{reason}
Ảnh đã được lưu tại {path}"
        ),
        Lang::En => format!(
            "{reason}
The image was saved at {path}"
        ),
    }
}

pub fn save_failed_in_clipboard(lang: Lang, err: &str) -> String {
    match lang {
        Lang::Vi => {
            format!("Không lưu được ảnh ({err}). Ảnh đã được đặt trong clipboard, hãy dán ra ngay.")
        }
        Lang::En => format!(
            "Could not save the image ({err}). The image is on the clipboard, paste it now."
        ),
    }
}

pub fn save_failed_no_clipboard(lang: Lang, err: &str) -> String {
    match lang {
        Lang::Vi => format!("Không lưu được ảnh ({err}) và cũng không đưa được vào clipboard."),
        Lang::En => format!(
            "Could not save the image ({err}) and could not put it on the clipboard either."
        ),
    }
}

pub fn http_error(lang: Lang, code: u16) -> String {
    match lang {
        Lang::Vi => format!("Máy chủ trả về lỗi {code}."),
        Lang::En => format!("The server returned error {code}."),
    }
}

/// Thêm mã lỗi của hệ điều hành sau một thông báo.
pub fn with_error_code(lang: Lang, base: &str, code: &str) -> String {
    match lang {
        Lang::Vi => format!("{base} (mã lỗi {code})"),
        Lang::En => format!("{base} (error code {code})"),
    }
}

pub fn reason(lang: Lang, why: &str) -> String {
    match lang {
        Lang::Vi => format!("Lý do: {why}"),
        Lang::En => format!("Reason: {why}"),
    }
}

pub fn reason_try_again(lang: Lang, why: &str) -> String {
    match lang {
        Lang::Vi => format!("Lý do: {why}. Hãy thử lại."),
        Lang::En => format!("Reason: {why}. Try again."),
    }
}

pub fn upload_start_failed_body(lang: Lang, err: &str) -> String {
    match lang {
        Lang::Vi => format!("{err}. Ảnh sẽ được lưu vào máy."),
        Lang::En => format!("{err}. The image will be saved to disk."),
    }
}

pub fn options_save_failed_body(lang: Lang, why: &str) -> String {
    match lang {
        Lang::Vi => format!("Lý do: {why}. Tùy chọn cũ vẫn được dùng."),
        Lang::En => format!("Reason: {why}. The previous options remain in use."),
    }
}

/// Tổ hợp phím của từng dòng phím tắt cho Trợ giúp; `None` là dòng đang tắt.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct HelpKeys {
    pub region: Option<String>,
    pub save_full: Option<String>,
    pub upload_full: Option<String>,
    pub repeat_last: Option<String>,
}

/// Nội dung cửa sổ Trợ giúp: văn bản thuần, các dòng ngăn bằng `\n`, không có địa chỉ web.
/// `keys` là tổ hợp của các dòng phím tắt đang bật (ví dụ "PrintScreen").
pub fn help_text(lang: Lang, keys: &HelpKeys) -> String {
    let change = t(lang, Msg::BtnChange);
    let options = t(lang, Msg::OptionsTitle);
    let hotkeys_tab = t(lang, Msg::TabHotkeys);
    let region_label = t(lang, Msg::LabelHotkey);
    let save_label = t(lang, Msg::LabelHotkeySaveFull);
    let upload_label = t(lang, Msg::LabelHotkeyUploadFull);
    let repeat_label = t(lang, Msg::LabelHotkeyRepeatLast);
    let repeat_menu = t(lang, Msg::MenuRepeatLast);
    let notices = t(lang, Msg::ChkShowSuccessNotices);
    let keep = t(lang, Msg::ChkKeepSelection);
    let cursor = t(lang, Msg::ChkIncludeCursor);
    let format_tab = t(lang, Msg::TabFormat);
    let (saved, copied, link) = (
        t(lang, Msg::SavedTitle),
        t(lang, Msg::CopiedTitle),
        t(lang, Msg::UploadedTitle),
    );
    // Dòng đang tắt: nói "chưa bật" và chỉ chỗ bật.
    let off = |label: &str| match lang {
        Lang::Vi => format!("- {label}: chưa bật. Bật ở {options}, tab {hotkeys_tab}."),
        Lang::En => {
            format!("- {label}: not turned on. Turn it on in {options}, {hotkeys_tab} tab.")
        }
    };
    let region = match (lang, &keys.region) {
        (Lang::Vi, Some(k)) => format!("Bấm {k} hoặc chọn \"Chụp màn hình\" ở menu khay."),
        (Lang::Vi, None) => format!(
            "Phím \"{region_label}\" đang tắt: chọn \"Chụp màn hình\" ở menu khay (bật lại phím ở {options}, tab {hotkeys_tab})."
        ),
        (Lang::En, Some(k)) => {
            format!("Press {k} or choose \"Take screenshot\" in the tray menu.")
        }
        (Lang::En, None) => format!(
            "The \"{region_label}\" hotkey is off: choose \"Take screenshot\" in the tray menu (turn the key back on in {options}, {hotkeys_tab} tab)."
        ),
    };
    let save_full = match (lang, &keys.save_full) {
        (Lang::Vi, Some(k)) => format!(
            "- {save_label}: bấm {k} để chụp ngay màn hình đang chứa con trỏ chuột, không chọn vùng, và lưu ảnh vào thư mục lưu."
        ),
        (Lang::En, Some(k)) => format!(
            "- {save_label}: press {k} to immediately capture the screen the mouse pointer is on, without selecting a region, and save it to the save folder."
        ),
        (_, None) => off(save_label),
    };
    let upload_full = match (lang, &keys.upload_full) {
        (Lang::Vi, Some(k)) => format!(
            "- {upload_label}: bấm {k} để chụp ngay màn hình đang chứa con trỏ chuột, tải ảnh lên kho R2 và chép link."
        ),
        (Lang::En, Some(k)) => format!(
            "- {upload_label}: press {k} to immediately capture the screen the mouse pointer is on, upload it to your R2 bucket and copy the link."
        ),
        (_, None) => off(upload_label),
    };
    let repeat_last = match (lang, &keys.repeat_last) {
        (Lang::Vi, Some(k)) => format!(
            "- {repeat_label}: bấm {k} hoặc chọn \"{repeat_menu}\" ở menu khay để chụp lại đúng vùng của lần chụp vùng trước và làm lại hành động đã chọn, không cần chọn vùng."
        ),
        (Lang::En, Some(k)) => format!(
            "- {repeat_label}: press {k} or choose \"{repeat_menu}\" in the tray menu to capture the same region as the previous region capture and repeat the action you chose, without selecting a region."
        ),
        (Lang::Vi, None) => format!(
            "- {repeat_label}: mặc định tắt; bật ở {options}, tab {hotkeys_tab}. Mục \"{repeat_menu}\" ở menu khay dùng được sau lần chụp vùng đầu tiên (trước đó mục này mờ)."
        ),
        (Lang::En, None) => format!(
            "- {repeat_label}: off by default; turn it on in {options}, {hotkeys_tab} tab. The \"{repeat_menu}\" tray menu item becomes available after your first region capture (until then it is grayed out)."
        ),
    };
    match lang {
        Lang::Vi => format!(
            "1. Chụp màn hình\n\
{region} Màn hình đứng yên và tối đi. Kéo chuột để chọn vùng; kéo các tay nắm để đổi cỡ, kéo bên trong khung để di chuyển. Chưa có vùng chọn thì rê chuột lên một cửa sổ hoặc màn hình rồi bấm (hoặc Enter) để chọn đúng khung đó.\n\
Chụp cả màn hình bằng phím tắt:\n\
{save_full}\n\
{upload_full}\n\
{repeat_last}\n\
\n\
2. Thanh công cụ và phím tắt\n\
Sau khi chọn vùng, dùng nút trên thanh công cụ hoặc phím tắt:\n\
- Ctrl+D: tải ảnh lên kho R2 và chép link.\n\
- Ctrl+S: lưu ảnh vào thư mục lưu.\n\
- Ctrl+C: chép ảnh vào clipboard.\n\
- Esc lùi một bước (đóng bảng, hủy chữ, bỏ chọn) rồi đóng lớp phủ.\n\
- Ctrl+P: ghim ảnh lên màn hình (hoặc bấm nút Ghim trong Lịch sử ảnh chụp). Ảnh ghim luôn nằm trên cùng: kéo để dời, cuộn chuột để phóng, Ctrl+cuộn để làm mờ, chuột phải để mở menu, Esc để đóng.\n\
- Ctrl+T: nhận dạng chữ trong vùng đã chọn rồi chép ra (ảnh ghim có mục \"Nhận dạng chữ\" trong menu chuột phải).\n\
- Cửa sổ chữ nhận dạng: bấm một khung để chép dòng đó, Ctrl+bấm để chọn nhiều dòng.\n\
- Windows chưa có bộ nhận dạng tiếng Việt, chữ có dấu có thể bị mất dấu.\n\
\n\
3. Tùy chọn\n\
Mở bằng menu khay, mục \"Tùy chọn...\". Bấm OK để lưu, Hủy để bỏ thay đổi.\n\
- Thông thường: khởi động cùng Windows, thư mục lưu, ngôn ngữ và ba ô tích:\n\
\x20\x20+ {notices}: bỏ tích thì không hiện các thông báo \"{saved}\", \"{copied}\", \"{link}\". Thông báo lỗi luôn hiện.\n\
\x20\x20+ {keep}: lần chụp vùng sau mở sẵn vùng đã chọn lần trước, kể cả sau khi khởi động lại darkshot.\n\
\x20\x20+ {cursor}: ảnh chụp có cả con trỏ chuột.\n\
- Phím tắt: bốn dòng \"{region_label}\", \"{save_label}\", \"{upload_label}\" và \"{repeat_label}\". Ô tích đầu dòng bật hoặc tắt phím của dòng đó; dòng tắt không giữ phím nào. Bấm {change} rồi nhấn tổ hợp mới; Esc để bỏ qua.\n\
- {format_tab}: chọn định dạng cho ảnh tải lên, PNG (mặc định) hoặc JPEG, và chất lượng JPEG từ 50 đến 100. Ảnh lưu vào máy luôn là PNG. Chất lượng càng cao thì file càng nặng.\n\
- Lưu trữ R2: thông tin kho Cloudflare R2, hạn của link và nút Kiểm tra kết nối.\n\
\n\
4. Chuẩn bị Cloudflare R2\n\
Chỉ cần làm một lần, để tải ảnh lên và chép link.\n\
a) Trên trang quản lý Cloudflare, mở mục R2 và tạo một kho (bucket). Ghi lại Account ID.\n\
b) Bật truy cập công khai cho kho: gắn tên miền riêng (nên dùng lâu dài) hoặc bật địa chỉ r2.dev (chỉ để thử). Ghi lại địa chỉ công khai của kho. Không bật liệt kê nội dung công khai.\n\
c) Tạo khóa API với quyền Object Read & Write, chỉ cho đúng kho này. Cloudflare chỉ hiện Access Key ID và Secret Access Key một lần, hãy lưu ngay.\n\
d) Tạo ba quy tắc vòng đời cho kho, theo tiền tố d1/ (1 ngày), d7/ (7 ngày) và d30/ (30 ngày). Không có quy tắc thì ảnh không tự hết hạn.\n\
e) Mở Tùy chọn, tab Lưu trữ R2, nhập Account ID, Access Key ID, Secret Access Key, tên kho và địa chỉ công khai.\n\
f) Bấm Kiểm tra kết nối, chờ báo thành công, rồi bấm OK.\n\
\n\
5. Khi tải lên không được\n\
Ảnh luôn được lưu vào thư mục lưu, nên không bị mất. Kiểm tra mạng và thông tin trong tab Lưu trữ R2, rồi thử lại.
\n
\n6. Lịch sử ảnh chụp
\nBật \"Bật lịch sử ảnh chụp\" ở Tùy chọn, tab Thông thường, rồi mở \"Lịch sử ảnh chụp\" từ menu khay. Cửa sổ giữ 50 ảnh gần nhất: xem trước, mở lại, sao chép ảnh hoặc liên kết, mở thư mục chứa ảnh, tìm và lọc. Dữ liệu chỉ nằm trên máy này. Xóa một mục hay xóa lịch sử không xóa ảnh bạn đã lưu và không xóa ảnh đã tải lên R2.\n\n7. Chụp từng bước\nChọn \"Chụp từng bước\" ở menu khay để mở thanh phiên, rồi bấm \"Chụp bước\" (hoặc Ctrl+N trong lúc chọn vùng) để thêm ảnh có hình đánh dấu vào phiên, tối đa 99 bước. \"Xem lại\" cho đổi thứ tự, ghi chú, xóa bước và xuất ra một thư mục gồm ảnh đánh số và tệp steps.md. Ảnh bước chỉ nằm trên máy cho đến khi bạn xuất.

8. Chụp cuộn
Để chụp một trang dài thành một ảnh dọc: chọn vùng, bấm \"Chụp cuộn\" trên thanh công cụ (hoặc Ctrl+L), rồi tự cuộn nội dung xuống từ từ. darkshot ghép các phần lại; bấm \"Xong\" để xem kết quả và lưu, chép, tải lên hoặc ghim, \"Hủy\" hoặc Esc để bỏ. Hình đánh dấu không được kèm. Cuộn quá nhanh hay cuộn ngược thì phần đó không được ghép và kết quả có cảnh báo; ảnh dài tối đa 30 000 px.

9. Ghi GIF
Để ghi một đoạn ngắn trên màn hình thành ảnh động: chọn vùng, bấm \"Ghi GIF\" trên thanh công cụ (hoặc Ctrl+G), rồi thao tác như bình thường. Bấm \"Dừng\" để lưu tệp GIF vào thư mục lưu, \"Hủy\" để bỏ. Mỗi lần ghi tối đa 60 giây, 10 khung hình mỗi giây, không có con trỏ chuột và âm thanh; hình đánh dấu không được kèm. Vùng rất lớn được thu nhỏ còn khoảng 0,5 triệu điểm ảnh (500 000)."
        ),
        Lang::En => format!(
            "1. Taking a screenshot\n\
{region} The screen dims and freezes. Drag to select a region; drag the handles to resize it, or drag inside the frame to move it. Before you select, hover a window or screen and click (or press Enter) to select exactly that frame.\n\
Full-screen captures with a hotkey:\n\
{save_full}\n\
{upload_full}\n\
{repeat_last}\n\
\n\
2. Toolbar and keys\n\
Once a region is selected, use the toolbar buttons or the keys:\n\
- Ctrl+D: upload the image to your R2 bucket and copy the link.\n\
- Ctrl+S: save the image to the save folder.\n\
- Ctrl+C: copy the image to the clipboard.\n\
- Esc steps back (closes a panel, cancels text, deselects), then closes the overlay.\n\
- Ctrl+P: pin the image to the screen (or click Pin in Screenshot History). A pinned image stays on top: drag to move, scroll to zoom, Ctrl+scroll to change opacity, right-click for a menu, Esc to close.\n\
- Ctrl+T: recognize the text in the selected region and copy it (a pinned image has \"Recognize text\" in its right-click menu).\n\
- In the recognized-text window, click a box to copy that line; Ctrl+click picks several lines.\n\
- Windows has no Vietnamese text recognizer yet, so accented letters may lose their marks.\n\
\n\
3. Options\n\
Open it from the tray menu, \"Options...\". Click OK to save, Cancel to discard changes.\n\
- General: start with Windows, save folder, language and three check boxes:\n\
\x20\x20+ {notices}: when unchecked, the \"{saved}\", \"{copied}\" and \"{link}\" notifications are not shown. Error notifications always appear.\n\
\x20\x20+ {keep}: the next region capture starts with the previously selected region, even after darkshot restarts.\n\
\x20\x20+ {cursor}: screenshots include the mouse pointer.\n\
- Hotkeys: four rows, \"{region_label}\", \"{save_label}\", \"{upload_label}\" and \"{repeat_label}\". The check box at the start of a row turns its key on or off; a row that is off holds no key. Click {change}, then press the new combination; Esc skips.\n\
- {format_tab}: choose the format for uploaded images, PNG (the default) or JPEG, and the JPEG quality from 50 to 100. Images saved to disk are always PNG. Higher quality means a larger file.\n\
- R2 storage: your Cloudflare R2 bucket details, the link expiry and the Test connection button.\n\
\n\
4. Setting up Cloudflare R2\n\
You only need to do this once, to upload images and copy links.\n\
a) In the Cloudflare dashboard, open R2 and create a bucket. Note the Account ID.\n\
b) Turn on public access for the bucket: attach a custom domain (best for long-term use) or enable the r2.dev address (for trying it out only). Note the public address of the bucket. Do not enable public listing.\n\
c) Create an API key with Object Read & Write permission, limited to this bucket only. Cloudflare shows the Access Key ID and Secret Access Key just once, so save them right away.\n\
d) Create three lifecycle rules for the bucket, for the prefixes d1/ (1 day), d7/ (7 days) and d30/ (30 days). Without the rules, images never expire.\n\
e) Open Options, the R2 storage tab, and enter the Account ID, Access Key ID, Secret Access Key, bucket name and public address.\n\
f) Click Test connection, wait for the success message, then click OK.\n\
\n\
5. When an upload fails\n\
The image is always saved to the save folder, so it is not lost. Check your network and the details in the R2 storage tab, then try again.
\n
\n6. Screenshot history
\nTurn on \"Enable Screenshot History\" in Options, General tab, then open \"Screenshot history\" from the tray menu. The window keeps your 50 latest screenshots: preview, reopen, copy the image or link, show the file in its folder, search and filter. Data stays on this device. Removing an entry or clearing history never deletes images you saved or images uploaded to R2.\n\n7. Step capture\nChoose \"Step capture\" in the tray menu to open the session bar, then click \"Capture step\" (or press Ctrl+N while selecting a region) to add an annotated image to the session, up to 99 steps. \"Review\" lets you reorder steps, add captions, delete steps and export a folder of numbered images plus a steps.md file. Step images stay on this device until you export.

8. Scrolling capture
To capture a long page as one tall image: select a region, click \"Scrolling capture\" on the toolbar (or press Ctrl+L), then scroll the content down slowly yourself. darkshot joins the parts; click \"Done\" to see the result and save, copy, upload or pin it, or \"Cancel\" (or Esc) to discard it. Annotations are not included. If you scroll too fast or scroll up, that part is not joined and the result shows a warning; the image can be up to 30,000 px tall.

9. Recording a GIF
To record a short part of the screen as an animated image: select a region, click \"Record GIF\" on the toolbar (or press Ctrl+G), then work as usual. Click \"Stop\" to save the GIF file to the save folder, or \"Cancel\" to discard it. Each recording is up to 60 seconds at 10 frames per second, without the mouse pointer or sound; annotations are not included. A very large region is scaled down to about 500,000 pixels."
        ),
    }
}

pub fn hotkeys_conflict(lang: Lang, first_label: &str, second_label: &str, combo: &str) -> String {
    match lang {
        Lang::Vi => format!(
            "\"{first_label}\" và \"{second_label}\" đang dùng cùng phím {combo}. Hãy đổi một trong hai."
        ),
        Lang::En => format!(
            "\"{first_label}\" and \"{second_label}\" use the same key {combo}. Change one of them."
        ),
    }
}

pub fn hotkey_row_taken(lang: Lang, row_label: &str, combo: &str) -> String {
    match lang {
        Lang::Vi => format!(
            "Phím {combo} của \"{row_label}\" đang được ứng dụng khác dùng. Hãy chọn phím khác."
        ),
        Lang::En => format!(
            "The {combo} key for \"{row_label}\" is in use by another app. Choose a different key."
        ),
    }
}

pub fn hotkeys_taken_body(lang: Lang, combos: &[String]) -> String {
    match (lang, combos) {
        (_, []) => String::new(),
        (Lang::Vi, [one]) => {
            format!("{one} đang được ứng dụng khác dùng. Mở Tùy chọn để chọn phím khác.")
        }
        (Lang::En, [one]) => {
            format!("{one} is in use by another app. Open Options to choose a different key.")
        }
        (Lang::Vi, many) => format!(
            "Các phím {} đang được ứng dụng khác dùng. Mở Tùy chọn để chọn phím khác.",
            many.join(", ")
        ),
        (Lang::En, many) => format!(
            "The keys {} are in use by other apps. Open Options to choose different keys.",
            many.join(", ")
        ),
    }
}

/// Mục menu khay khi đang có phiên: "Xem lại các bước (N)".
pub fn tray_steps_review(lang: Lang, n: usize) -> String {
    match lang {
        Lang::Vi => format!("Xem lại các bước ({n})"),
        Lang::En => format!("Review steps ({n})"),
    }
}

/// Thanh phiên: "N bước" | "N step(s)".
pub fn steps_count(lang: Lang, n: usize) -> String {
    match (lang, n) {
        (Lang::Vi, n) => format!("{n} bước"),
        (Lang::En, 1) => "1 step".to_string(),
        (Lang::En, n) => format!("{n} steps"),
    }
}

/// "Bước k" | "Step k" (k tính từ 1).
pub fn step_label(lang: Lang, k: usize) -> String {
    match lang {
        Lang::Vi => format!("Bước {k}"),
        Lang::En => format!("Step {k}"),
    }
}

/// Thẻ xác nhận kết thúc phiên chưa xuất.
pub fn steps_end_confirm(lang: Lang, n: usize) -> String {
    match (lang, n) {
        (Lang::Vi, n) => format!("Kết thúc mà không xuất? {n} bước sẽ bị xóa."),
        (Lang::En, 1) => "End without exporting? 1 step will be deleted.".to_string(),
        (Lang::En, n) => format!("End without exporting? {n} steps will be deleted."),
    }
}

/// Thông báo sau khi xuất: "Đã xuất N bước" | "Exported N step(s)".
pub fn steps_exported(lang: Lang, n: usize) -> String {
    match (lang, n) {
        (Lang::Vi, n) => format!("Đã xuất {n} bước"),
        (Lang::En, 1) => "Exported 1 step".to_string(),
        (Lang::En, n) => format!("Exported {n} steps"),
    }
}
/// Tiêu đề cửa sổ Duyệt, có "Bước k/n" khi duyệt một bước của lượt xuất các bước.
pub fn privacy_review_title(lang: Lang, step: Option<(u32, u32)>) -> String {
    let base = t(lang, Msg::PrivacyReviewTitle);
    match (lang, step) {
        (_, None) => base.to_string(),
        (Lang::Vi, Some((k, n))) => format!("{base} · Bước {k}/{n}"),
        (Lang::En, Some((k, n))) => format!("{base} · Step {k}/{n}"),
    }
}

/// Tóm tắt của cửa sổ Duyệt: "Tìm thấy N mục · <hành động>".
pub fn privacy_found(lang: Lang, n: usize, action: &str) -> String {
    match (lang, n) {
        (Lang::Vi, n) => format!("Tìm thấy {n} mục · {action}"),
        (Lang::En, 1) => format!("Found 1 item · {action}"),
        (Lang::En, n) => format!("Found {n} items · {action}"),
    }
}

/// Tiến độ quét theo dải: "dải k/n" | "part k of n".
pub fn privacy_scan_tile(lang: Lang, k: u32, n: u32) -> String {
    match lang {
        Lang::Vi => format!("dải {k}/{n}"),
        Lang::En => format!("part {k} of {n}"),
    }
}

/// Chiều cao hiện có trên thanh chụp cuộn: "1840 px".
pub fn scroll_height(lang: Lang, px: u32) -> String {
    match lang {
        Lang::Vi | Lang::En => format!("{px} px"),
    }
}

/// Cỡ ảnh trên dòng thông tin của cửa sổ kết quả chụp cuộn: "900 × 12000 px".
pub fn scroll_size(lang: Lang, w: u32, h: u32) -> String {
    match lang {
        Lang::Vi | Lang::En => format!("{w} × {h} px"),
    }
}

/// Dải cảnh báo của cửa sổ kết quả chụp cuộn.
pub fn scroll_uncertain(lang: Lang, n: u32) -> String {
    match lang {
        Lang::Vi => format!("Có {n} chỗ ghép không chắc chắn. Kiểm tra ảnh trước khi dùng."),
        Lang::En => format!("{n} join(s) may be inaccurate. Check the image before using it."),
    }
}

/// "Phiên bản 0.1.0" | "Version 0.1.0".
/// Thân thông báo khi GIF đã lưu kèm lý do dừng (`note`, ví dụ `Msg::RecordMaxDuration`).
pub fn record_saved_with_note(lang: Lang, note: &str, path: &str) -> String {
    match lang {
        Lang::Vi => format!("{note} Đã lưu GIF: {path}"),
        Lang::En => format!("{note} GIF saved: {path}"),
    }
}

pub fn record_capture_failed(lang: Lang, err: &str) -> String {
    match lang {
        Lang::Vi => format!("Không chụp được vùng ghi: {err}"),
        Lang::En => format!("Could not capture the recording region: {err}"),
    }
}

pub fn record_encode_failed(lang: Lang, err: &str) -> String {
    format!("{}: {err}", t(lang, Msg::RecordEncodeFailed))
}

pub fn record_io_failed(lang: Lang, err: &str) -> String {
    match lang {
        Lang::Vi => format!("Không ghi được tệp GIF: {err}"),
        Lang::En => format!("Could not write the GIF file: {err}"),
    }
}

pub fn record_thread_failed(lang: Lang, err: &str) -> String {
    match lang {
        Lang::Vi => format!("Luồng ghi GIF gặp lỗi: {err}"),
        Lang::En => format!("The GIF recording thread failed: {err}"),
    }
}

pub fn about_version(lang: Lang, version: &str) -> String {
    match lang {
        Lang::Vi => format!("Phiên bản {version}"),
        Lang::En => format!("Version {version}"),
    }
}

/// "Có bản mới 0.3.0" | "New version 0.3.0 available" (mục menu khay).
pub fn update_menu(lang: Lang, v: &str) -> String {
    match lang {
        Lang::Vi => format!("Có bản mới {v}"),
        Lang::En => format!("New version {v} available"),
    }
}

/// Nội dung thông báo có bản mới.
pub fn update_notice_body(lang: Lang, v: &str) -> String {
    match lang {
        Lang::Vi => format!("Bản {v} đã có. Bấm để mở trang tải về."),
        Lang::En => format!("Version {v} is available. Click to open the download page."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_message_has_both_languages_non_empty() {
        for m in Msg::ALL {
            for l in Lang::ALL {
                assert!(!t(l, *m).trim().is_empty(), "{m:?} {l:?}");
            }
        }
    }

    #[test]
    fn languages_differ_for_every_message() {
        for m in Msg::ALL {
            assert_ne!(t(Lang::Vi, *m), t(Lang::En, *m), "{m:?}");
        }
    }

    #[test]
    fn update_texts() {
        assert_eq!(update_menu(Lang::Vi, "0.3.0"), "Có bản mới 0.3.0");
        assert_eq!(
            update_menu(Lang::En, "0.3.0"),
            "New version 0.3.0 available"
        );
        assert_eq!(
            update_notice_body(Lang::Vi, "0.3.0"),
            "Bản 0.3.0 đã có. Bấm để mở trang tải về."
        );
        assert_eq!(
            update_notice_body(Lang::En, "0.3.0"),
            "Version 0.3.0 is available. Click to open the download page."
        );
        assert_eq!(t(Lang::Vi, Msg::ChkCheckUpdates), "Tự kiểm tra bản mới");
        assert_eq!(
            t(Lang::En, Msg::ChkCheckUpdates),
            "Check for updates automatically"
        );
        assert_eq!(t(Lang::Vi, Msg::UpdateNoticeTitle), "Có bản darkshot mới");
        assert_eq!(
            t(Lang::En, Msg::UpdateNoticeTitle),
            "darkshot update available"
        );
    }

    #[test]
    fn fixed_strings() {
        assert_eq!(t(Lang::Vi, Msg::UploadedTitle), "Đã chép link");
        assert_eq!(t(Lang::En, Msg::UploadedTitle), "Link copied");
        assert!(t(Lang::Vi, Msg::UploadNotConfigured)
            .ends_with("Hãy nhập đủ thông tin trong Tùy chọn."));
        assert!(t(Lang::En, Msg::UploadNotConfigured).ends_with("in Options."));
        assert_eq!(OK_LABEL, "OK");
        // Câu thông báo cấu hình lỗi nhắc đúng nút OK của cửa sổ Tùy chọn.
        for m in [Msg::NoticeConfigCorrupt, Msg::NoticeConfigUnreadable] {
            for l in Lang::ALL {
                assert!(t(l, m).contains(OK_LABEL), "{m:?} {l:?}");
            }
        }
    }

    #[test]
    fn annotate_strings_match_plan() {
        let want = [
            (Msg::ToolPen, "Bút vẽ (P)", "Pen (P)"),
            (Msg::ToolLine, "Đường thẳng (L)", "Line (L)"),
            (Msg::ToolArrow, "Mũi tên (A)", "Arrow (A)"),
            (Msg::ToolRect, "Hình chữ nhật (R)", "Rectangle (R)"),
            (Msg::ToolMarker, "Bút dạ quang (H)", "Highlighter (H)"),
            (Msg::ToolBlur, "Làm mờ (B)", "Blur (B)"),
            (Msg::ToolText, "Chữ (T)", "Text (T)"),
            (Msg::ToolColor, "Màu", "Color"),
            (Msg::ToolUndo, "Hoàn tác (Ctrl+Z)", "Undo (Ctrl+Z)"),
            (Msg::ToolSelect, "Chọn (V)", "Select (V)"),
            (Msg::ToolEllipse, "Hình elip (E)", "Ellipse (E)"),
            (Msg::ToolStep, "Đánh số (N)", "Numbered steps (N)"),
            (Msg::ToolSize, "Cỡ ([ ])", "Size ([ ])"),
            (Msg::ToolRedo, "Làm lại (Ctrl+Y)", "Redo (Ctrl+Y)"),
            (Msg::ToolShapes, "Hình (L, A, R, E)", "Shapes (L, A, R, E)"),
            (Msg::SizeSmall, "Nhỏ", "Small"),
            (Msg::SizeMedium, "Vừa", "Medium"),
            (Msg::SizeLarge, "Lớn", "Large"),
            (Msg::SizeHuge, "Rất lớn", "Extra large"),
            (
                Msg::AnnotateFailedTitle,
                "Không xử lý được ảnh",
                "Could not process the image",
            ),
            (
                Msg::AnnotateFailedBody,
                "Không vẽ được các nét đánh dấu lên ảnh nên ảnh chưa được lưu, chép hay tải lên. Hãy chụp lại.",
                "Could not draw the markings onto the image, so it was not saved, copied or uploaded. Please capture again.",
            ),
        ];
        for (m, vi, en) in want {
            assert_eq!(t(Lang::Vi, m), vi);
            assert_eq!(t(Lang::En, m), en);
            assert_ne!(vi, en);
        }
    }

    #[test]
    fn lang_codes_round_trip() {
        assert_eq!(Lang::Vi.code(), "vi");
        assert_eq!(Lang::En.code(), "en");
        for l in Lang::ALL {
            assert_eq!(Lang::from_code(l.code()), Some(l));
        }
        assert_eq!(Lang::from_code("fr"), None);
        assert_eq!(Lang::from_code("VI"), None);
        assert_eq!(Lang::from_code(""), None);
        assert_eq!(Lang::Vi.native_name(), "Tiếng Việt");
        assert_eq!(Lang::En.native_name(), "English");
        assert_eq!(Lang::default(), Lang::Vi);
    }

    #[test]
    fn expiry_labels() {
        assert_eq!(
            Expiry::ALL.map(|e| expiry_label(Lang::Vi, e)),
            ["1 ngày", "7 ngày", "30 ngày", "Không hết hạn"]
        );
        assert_eq!(
            Expiry::ALL.map(|e| expiry_label(Lang::En, e)),
            ["1 day", "7 days", "30 days", "Never expires"]
        );
    }

    #[test]
    fn notice_titles_fit_tray_limit() {
        for m in NOTICE_TITLES {
            for l in Lang::ALL {
                assert!(t(l, *m).encode_utf16().count() <= 63, "{m:?} {l:?}");
            }
        }
    }

    #[test]
    fn parameterised_strings_contain_their_arguments() {
        for l in Lang::ALL {
            assert!(copy_failed_body(l, "ERR1").contains("ERR1"), "{l:?}");
            assert!(encode_failed(l, "ERR2").contains("ERR2"), "{l:?}");
            assert!(encoder_error(l, "ERR3").contains("ERR3"), "{l:?}");
            let b = upload_failed_body(l, "REASON", "C:/x/y.png");
            assert!(b.contains("REASON") && b.contains("C:/x/y.png"), "{l:?}");
            assert!(
                save_failed_in_clipboard(l, "ERR4").contains("ERR4"),
                "{l:?}"
            );
            assert!(
                save_failed_no_clipboard(l, "ERR5").contains("ERR5"),
                "{l:?}"
            );
            assert!(http_error(l, 503).contains("503"), "{l:?}");
        }
        assert_ne!(http_error(Lang::Vi, 503), http_error(Lang::En, 503));
        assert_ne!(
            copy_failed_body(Lang::Vi, "x"),
            copy_failed_body(Lang::En, "x")
        );
        assert_ne!(encode_failed(Lang::Vi, "x"), encode_failed(Lang::En, "x"));
        assert_ne!(encoder_error(Lang::Vi, "x"), encoder_error(Lang::En, "x"));
        assert_ne!(
            upload_failed_body(Lang::Vi, "a", "b"),
            upload_failed_body(Lang::En, "a", "b")
        );
        assert_ne!(
            save_failed_in_clipboard(Lang::Vi, "x"),
            save_failed_in_clipboard(Lang::En, "x")
        );
        assert_ne!(
            save_failed_no_clipboard(Lang::Vi, "x"),
            save_failed_no_clipboard(Lang::En, "x")
        );
    }

    #[test]
    fn windows_layer_strings_contain_their_arguments() {
        for l in Lang::ALL {
            let coded = with_error_code(l, "BASE", "0x1");
            assert!(coded.contains("BASE") && coded.contains("0x1"), "{l:?}");
            assert!(reason(l, "A1").contains("A1"));
            assert!(reason_try_again(l, "A2").contains("A2"));
            assert!(upload_start_failed_body(l, "A3").contains("A3"));
            assert!(options_save_failed_body(l, "A4").contains("A4"));
        }
        assert_ne!(reason(Lang::Vi, "x"), reason(Lang::En, "x"));
        assert_ne!(
            with_error_code(Lang::Vi, "x", "1"),
            with_error_code(Lang::En, "x", "1")
        );
    }

    #[test]
    fn help_text_names_the_hotkey_and_keys() {
        let keys = |k: &str| HelpKeys {
            region: Some(k.into()),
            ..Default::default()
        };
        for l in Lang::ALL {
            let s = help_text(l, &keys("Ctrl+Shift+Q"));
            assert!(s.contains("Ctrl+Shift+Q"), "{l:?}");
            for k in [
                "Ctrl+D",
                "Ctrl+S",
                "Ctrl+C",
                "Esc",
                "d1/",
                "d7/",
                "d30/",
                "Object Read & Write",
            ] {
                assert!(s.contains(k), "{l:?} thiếu {k}");
            }
            assert!(!s.contains("http"), "{l:?} có địa chỉ web");
            assert!(s.contains(t(l, Msg::BtnChange)), "{l:?} sai nhãn nút Đổi");
        }
        assert_ne!(
            help_text(Lang::Vi, &keys("X")),
            help_text(Lang::En, &keys("X"))
        );
    }

    #[test]
    fn no_legacy_name_in_strings() {
        let old = ["shot", "me"].concat();
        let all = HelpKeys {
            region: Some("A".into()),
            save_full: Some("B".into()),
            upload_full: Some("C".into()),
            repeat_last: Some("D".into()),
        };
        for l in Lang::ALL {
            for m in Msg::ALL {
                assert!(!t(l, *m).to_lowercase().contains(&old), "{m:?} {l:?}");
            }
            for h in [
                help_text(l, &all),
                help_text(l, &HelpKeys::default()),
                about_version(l, "1.0.0"),
            ] {
                assert!(!h.to_lowercase().contains(&old), "{l:?}");
            }
            assert!(help_text(l, &all).contains("darkshot"), "{l:?}");
        }
    }

    #[test]
    fn migration_notice_in_both_languages() {
        assert!(Msg::ALL.contains(&Msg::NoticeMigrationIncomplete));
        assert!(Msg::ALL.contains(&Msg::NoticeMigrationIncompleteTitle));
        assert_eq!(
            t(Lang::Vi, Msg::NoticeMigrationIncompleteTitle),
            "Chưa chuyển xong thiết lập"
        );
        assert_eq!(
            t(Lang::En, Msg::NoticeMigrationIncompleteTitle),
            "Settings not fully moved"
        );
        let (vi, en) = (
            t(Lang::Vi, Msg::NoticeMigrationIncomplete),
            t(Lang::En, Msg::NoticeMigrationIncomplete),
        );
        assert_ne!(vi, en);
        assert_eq!(
            vi,
            "Chưa chuyển được hết thiết lập từ phiên bản cũ. Hãy mở Tùy chọn để kiểm tra."
        );
        assert_eq!(
            en,
            "Some settings from the previous version could not be moved. Open Options to check them."
        );
    }

    #[test]
    fn help_text_lists_enabled_and_disabled_hotkeys() {
        for l in Lang::ALL {
            let all = HelpKeys {
                region: Some("KEY_A".into()),
                save_full: Some("KEY_B".into()),
                upload_full: Some("KEY_C".into()),
                repeat_last: Some("KEY_D".into()),
            };
            let s = help_text(l, &all);
            for k in [
                "KEY_A", "KEY_B", "KEY_C", "KEY_D", "Ctrl+D", "Ctrl+S", "Ctrl+C", "Esc",
            ] {
                assert!(s.contains(k), "{l:?} thiếu {k}");
            }
            assert!(!s.contains("http"));
            assert!(s.contains(t(l, Msg::BtnChange)));
            let none = help_text(l, &HelpKeys::default());
            for k in ["KEY_A", "KEY_B", "KEY_C", "KEY_D"] {
                assert!(!none.contains(k));
            }
            assert_ne!(none, s);
            // mỗi dòng tắt riêng lẻ cho văn bản khác với khi bật
            assert_ne!(
                help_text(
                    l,
                    &HelpKeys {
                        save_full: None,
                        ..all.clone()
                    }
                ),
                s
            );
            assert_ne!(
                help_text(
                    l,
                    &HelpKeys {
                        upload_full: None,
                        ..all.clone()
                    }
                ),
                s
            );
            assert_ne!(
                help_text(
                    l,
                    &HelpKeys {
                        region: None,
                        ..all.clone()
                    }
                ),
                s
            );
        }
    }

    #[test]
    fn repeat_last_strings_and_help() {
        for m in [
            Msg::LabelHotkeyRepeatLast,
            Msg::MenuRepeatLast,
            Msg::RepeatRegionGoneTitle,
            Msg::RepeatRegionGoneBody,
        ] {
            assert!(!t(Lang::Vi, m).is_empty() && !t(Lang::En, m).is_empty());
            assert_ne!(t(Lang::Vi, m), t(Lang::En, m));
        }
        for l in Lang::ALL {
            let label = t(l, Msg::LabelHotkeyRepeatLast);
            assert!(help_text(l, &HelpKeys::default()).contains(label), "{l:?}");
            // Đoạn tab Phím tắt của mục Tùy chọn nêu cả bốn dòng.
            let h = help_text(l, &HelpKeys::default());
            let line = h
                .lines()
                .find(|x| x.starts_with("- Phím tắt:") || x.starts_with("- Hotkeys:"))
                .expect("hotkeys tab line");
            assert!(line.contains(label), "{l:?}");
            assert!(
                !line.contains("ba dòng") && !line.contains("three rows"),
                "{l:?}"
            );
            let on = HelpKeys {
                repeat_last: Some("KEY_D".into()),
                ..HelpKeys::default()
            };
            assert!(help_text(l, &on).contains("KEY_D"));
            assert_ne!(help_text(l, &on), help_text(l, &HelpKeys::default()));
        }
    }

    #[test]
    fn snap_strings_and_help() {
        for m in [
            Msg::ChkSnapWindows,
            Msg::OverlayHintSnap,
            Msg::OverlayHintDrag,
        ] {
            assert!(!t(Lang::Vi, m).is_empty() && !t(Lang::En, m).is_empty());
            assert_ne!(t(Lang::Vi, m), t(Lang::En, m));
        }
        let vi = help_text(Lang::Vi, &HelpKeys::default());
        let en = help_text(Lang::En, &HelpKeys::default());
        assert!(vi.contains("Enter") && vi.contains("rê chuột lên một cửa sổ"));
        assert!(en.contains("Enter") && en.contains("hover a window"));
        assert!(vi.contains("Esc lùi một bước"));
        assert!(en.contains("Esc steps back"));
    }

    #[test]
    fn help_text_mentions_format_tab() {
        for l in Lang::ALL {
            let s = help_text(l, &HelpKeys::default());
            assert!(s.contains(t(l, Msg::TabFormat)), "{l:?} thiếu tên tab");
            assert!(s.contains("PNG"), "{l:?} thiếu PNG");
            assert!(s.contains("JPEG"), "{l:?} thiếu JPEG");
        }
    }

    #[test]
    fn pin_texts() {
        assert!(tray_close_pins(Lang::Vi, 3).contains("(3)"));
        assert!(tray_close_pins(Lang::En, 3).contains("(3)"));
        assert!(pin_limit_body(Lang::Vi, 32).contains("32"));
        assert_eq!(pin_opacity_badge(Lang::Vi, 60), "Độ mờ 60%");
        assert_eq!(pin_opacity_badge(Lang::En, 60), "Opacity 60%");
        assert_eq!(t(Lang::Vi, Msg::PinTooLargeBody), "Ảnh quá lớn để ghim.");
        assert_eq!(
            t(Lang::En, Msg::PinTooLargeBody),
            "The image is too large to pin."
        );
    }

    #[test]
    fn help_text_keep_selection_and_repeat_menu_are_accurate() {
        let on = HelpKeys {
            repeat_last: Some("Ctrl+Shift+PrintScreen".into()),
            ..HelpKeys::default()
        };
        for keys in [HelpKeys::default(), on] {
            let vi = help_text(Lang::Vi, &keys);
            let en = help_text(Lang::En, &keys);
            // "Giữ vị trí vùng" nhớ cả sau khi khởi động lại.
            assert!(!vi.contains("cho tới khi thoát darkshot"));
            assert!(!en.contains("until you exit darkshot"));
            assert!(vi.contains("kể cả sau khi khởi động lại darkshot"));
            assert!(en.contains("even after darkshot restarts"));
            // Mục menu khay lặp lại mờ khi chưa có lần chụp vùng.
            assert!(!vi.contains("luôn dùng được"));
            assert!(!en.contains("always available"));
        }
        let vi = help_text(Lang::Vi, &HelpKeys::default());
        let en = help_text(Lang::En, &HelpKeys::default());
        assert!(vi.contains("dùng được sau lần chụp vùng đầu tiên"));
        assert!(en.contains("becomes available after your first region capture"));
    }

    #[test]
    fn help_text_mentions_pin_hotkey() {
        for l in Lang::ALL {
            assert!(
                help_text(l, &HelpKeys::default()).contains("Ctrl+P"),
                "{l:?}"
            );
        }
    }

    #[test]
    fn help_text_mentions_ocr_hotkey() {
        for l in Lang::ALL {
            assert!(
                help_text(l, &HelpKeys::default()).contains("Ctrl+T"),
                "{l:?}"
            );
        }
    }

    #[test]
    fn ocr_copy_failed_strings() {
        for l in Lang::ALL {
            let b = ocr_copy_failed_body(l, "ERR9");
            assert!(b.contains("ERR9"), "{l:?}");
            assert_ne!(b, copy_failed_body(l, "ERR9"), "{l:?}");
            assert_ne!(
                t(l, Msg::OcrCopyFailedTitle),
                t(l, Msg::CopyFailedTitle),
                "{l:?}"
            );
        }
        assert_eq!(t(Lang::Vi, Msg::OcrCopyFailedTitle), "Không chép được chữ");
        assert_eq!(
            t(Lang::En, Msg::OcrCopyFailedTitle),
            "Couldn't copy the text"
        );
        assert!(ocr_copy_failed_body(Lang::Vi, "x").contains("chữ"));
        assert!(ocr_copy_failed_body(Lang::En, "x").contains("text"));
        assert_ne!(
            ocr_copy_failed_body(Lang::Vi, "x"),
            ocr_copy_failed_body(Lang::En, "x")
        );
        assert!(NOTICE_TITLES.contains(&Msg::OcrCopyFailedTitle));
        assert!(NOTICE_TITLES.contains(&Msg::OcrCopiedTitle));
    }

    #[test]
    fn ocr_status_has_name_and_count() {
        for l in Lang::ALL {
            let s = ocr_status(l, "vi-VN", 7);
            assert!(s.contains("vi-VN") && s.contains('7'), "{l:?}");
        }
        assert_ne!(ocr_status(Lang::Vi, "x", 1), ocr_status(Lang::En, "x", 1));
    }

    #[test]
    fn about_version_contains_version() {
        for l in Lang::ALL {
            assert!(about_version(l, "1.2.3").contains("1.2.3"));
        }
    }

    #[test]
    fn batch2_strings_insert_their_parameters() {
        use crate::config::HotkeyRole;
        for l in Lang::ALL {
            let s = hotkeys_conflict(l, "AAA", "BBB", "Ctrl+X");
            assert!(s.contains("AAA") && s.contains("BBB") && s.contains("Ctrl+X"));
            let s = hotkey_row_taken(l, "AAA", "Ctrl+X");
            assert!(s.contains("AAA") && s.contains("Ctrl+X"));
            assert!(hotkeys_taken_body(l, &["F8".into()]).contains("F8"));
            let s = hotkeys_taken_body(l, &["F8".into(), "Ctrl+X".into()]);
            assert!(s.contains("F8") && s.contains("Ctrl+X"));
            assert_ne!(s, hotkeys_taken_body(l, &["F8".into()]));
            assert_eq!(hotkeys_taken_body(l, &[]), "");
        }
        for r in HotkeyRole::ALL {
            assert_ne!(t(Lang::Vi, r.label()), t(Lang::En, r.label()));
        }
    }

    #[test]
    fn picker_strings_match_design() {
        let want = [
            (Msg::PickerBasic, "Màu cơ bản", "Basic colors"),
            (Msg::PickerCustom, "Màu tự chọn", "Custom colors"),
            (
                Msg::PickerDefine,
                "Định nghĩa màu tự chọn >>",
                "Define custom colors >>",
            ),
            (
                Msg::PickerAdd,
                "Thêm vào màu tự chọn",
                "Add to custom colors",
            ),
            (Msg::PickerHue, "Sắc độ", "Hue"),
            (Msg::PickerSat, "Bão hòa", "Sat"),
            (Msg::PickerLum, "Độ sáng", "Lum"),
            (Msg::PickerRed, "Đỏ", "Red"),
            (Msg::PickerGreen, "Lục", "Green"),
            (Msg::PickerBlue, "Lam", "Blue"),
            (Msg::BtnCancel, "Hủy", "Cancel"),
        ];
        for (m, vi, en) in want {
            assert_eq!(t(Lang::Vi, m), vi, "{m:?}");
            assert_eq!(t(Lang::En, m), en, "{m:?}");
        }
    }

    #[test]
    fn scroll_capture_strings() {
        let pairs = [
            (
                Msg::OverlayScroll,
                "Chụp cuộn (Ctrl+L)",
                "Scrolling capture (Ctrl+L)",
            ),
            (
                Msg::ScrollNoAnnotations,
                "Không kèm hình đánh dấu",
                "Annotations are not included",
            ),
            (Msg::ScrollBarTitle, "Chụp cuộn", "Scrolling capture"),
            (Msg::ScrollHint, "Cuộn xuống từ từ", "Scroll down slowly"),
            (Msg::ScrollSlowDown, "Cuộn chậm hơn", "Scroll more slowly"),
            (Msg::ScrollDone, "Xong", "Done"),
            (Msg::ScrollCancel, "Hủy", "Cancel"),
            (
                Msg::ScrollLimitTitle,
                "Đã đạt giới hạn chiều cao",
                "Height limit reached",
            ),
            (Msg::ScrollResultTitle, "Ảnh chụp cuộn", "Scrolling capture"),
            (Msg::ScrollSave, "Lưu", "Save"),
            (Msg::ScrollCopy, "Chép", "Copy"),
            (Msg::ScrollUpload, "Tải lên", "Upload"),
            (Msg::ScrollPin, "Ghim", "Pin"),
            (Msg::ScrollClose, "Đóng", "Close"),
            (
                Msg::ScrollNoRoomTitle,
                "Không có chỗ cho thanh chụp cuộn",
                "No room for the scrolling capture bar",
            ),
            (
                Msg::ScrollNoRoomBody,
                "Chọn vùng nhỏ hơn màn hình.",
                "Select a region smaller than the screen.",
            ),
        ];
        for (m, vi, en) in pairs {
            assert_eq!(t(Lang::Vi, m), vi);
            assert_eq!(t(Lang::En, m), en);
        }
        assert!(NOTICE_TITLES.contains(&Msg::ScrollLimitTitle));
        assert!(NOTICE_TITLES.contains(&Msg::ScrollNoRoomTitle));
        assert_eq!(scroll_height(Lang::Vi, 1840), "1840 px");
        assert_eq!(scroll_height(Lang::En, 0), "0 px");
        // Cửa sổ kết quả (Task 91).
        let result = [
            (
                Msg::ScrollManyOpenTitle,
                "Có nhiều ảnh chụp cuộn chưa lưu",
                "Several scrolling captures are not saved",
            ),
            (
                Msg::ScrollManyOpenBody,
                "Lưu hoặc đóng bớt các cửa sổ Ảnh chụp cuộn.",
                "Save or close some Scrolling capture windows.",
            ),
            (Msg::ScrollPreparing, "Đang chuẩn bị…", "Preparing…"),
        ];
        for (m, vi, en) in result {
            assert_eq!(t(Lang::Vi, m), vi, "{m:?}");
            assert_eq!(t(Lang::En, m), en, "{m:?}");
        }
        assert!(NOTICE_TITLES.contains(&Msg::ScrollManyOpenTitle));
        assert!(NOTICE_TITLES.contains(&Msg::ScrollCopyMemoryTitle));
        assert_eq!(t(Lang::Vi, Msg::ScrollCopyMemoryTitle), "Không đủ bộ nhớ");
        assert_eq!(t(Lang::En, Msg::ScrollCopyMemoryTitle), "Not enough memory");
        assert_eq!(
            t(Lang::Vi, Msg::ScrollCopyMemoryBody),
            "Đóng bớt cửa sổ rồi thử lại."
        );
        assert_eq!(
            t(Lang::En, Msg::ScrollCopyMemoryBody),
            "Close some windows and try again."
        );
        assert_eq!(scroll_size(Lang::Vi, 900, 12000), "900 × 12000 px");
        assert_eq!(scroll_size(Lang::En, 1, 2), "1 × 2 px");
        assert_eq!(
            scroll_uncertain(Lang::Vi, 3),
            "Có 3 chỗ ghép không chắc chắn. Kiểm tra ảnh trước khi dùng."
        );
        assert_eq!(
            scroll_uncertain(Lang::En, 3),
            "3 join(s) may be inaccurate. Check the image before using it."
        );
        for l in Lang::ALL {
            let h = help_text(l, &HelpKeys::default());
            assert!(h.contains("Ctrl+L"), "{l:?}");
            assert!(h.contains(t(l, Msg::ScrollBarTitle)), "{l:?}");
            assert!(h.contains("8. "), "{l:?}");
        }
    }

    #[test]
    fn step_capture_strings() {
        assert_eq!(t(Lang::Vi, Msg::TrayStepsStart), "Chụp từng bước");
        assert_eq!(t(Lang::En, Msg::TrayStepsStart), "Step capture");
        assert_eq!(t(Lang::Vi, Msg::OverlayAddStep), "Thêm bước (Ctrl+N)");
        assert_eq!(t(Lang::En, Msg::OverlayAddStep), "Add step (Ctrl+N)");
        assert_eq!(t(Lang::En, Msg::StepsExport), "Export…");
        assert_eq!(tray_steps_review(Lang::Vi, 3), "Xem lại các bước (3)");
        assert_eq!(tray_steps_review(Lang::En, 3), "Review steps (3)");
        assert_eq!(steps_count(Lang::Vi, 3), "3 bước");
        assert_eq!(steps_count(Lang::En, 1), "1 step");
        assert_eq!(steps_count(Lang::En, 2), "2 steps");
        assert_eq!(steps_count(Lang::En, 0), "0 steps");
        assert_eq!(step_label(Lang::Vi, 2), "Bước 2");
        assert_eq!(step_label(Lang::En, 2), "Step 2");
        assert_eq!(
            steps_end_confirm(Lang::Vi, 4),
            "Kết thúc mà không xuất? 4 bước sẽ bị xóa."
        );
        assert_eq!(
            steps_end_confirm(Lang::En, 1),
            "End without exporting? 1 step will be deleted."
        );
        assert_eq!(
            steps_end_confirm(Lang::En, 4),
            "End without exporting? 4 steps will be deleted."
        );
        assert_eq!(steps_exported(Lang::Vi, 5), "Đã xuất 5 bước");
        assert_eq!(steps_exported(Lang::En, 1), "Exported 1 step");
        assert_eq!(steps_exported(Lang::En, 5), "Exported 5 steps");
        for (a, b) in [
            (
                tray_steps_review(Lang::Vi, 2),
                tray_steps_review(Lang::En, 2),
            ),
            (steps_count(Lang::Vi, 2), steps_count(Lang::En, 2)),
            (step_label(Lang::Vi, 2), step_label(Lang::En, 2)),
            (
                steps_end_confirm(Lang::Vi, 2),
                steps_end_confirm(Lang::En, 2),
            ),
            (steps_exported(Lang::Vi, 2), steps_exported(Lang::En, 2)),
        ] {
            assert_ne!(a, b);
        }
        assert!(NOTICE_TITLES.contains(&Msg::StepsFullTitle));
        assert!(NOTICE_TITLES.contains(&Msg::StepsExportFailedTitle));
        assert!(NOTICE_TITLES.contains(&Msg::StepsStartFailedTitle));
        assert!(NOTICE_TITLES.contains(&Msg::StepsSessionSaveFailedTitle));
        assert!(NOTICE_TITLES.contains(&Msg::StepsEndFailedTitle));
        assert_eq!(
            t(Lang::Vi, Msg::StepsEndFailedTitle),
            "Không xóa hết ảnh của phiên"
        );
        assert_eq!(
            t(Lang::En, Msg::StepsEndFailedTitle),
            "Couldn't delete all session images"
        );
        assert_eq!(
            t(Lang::Vi, Msg::StepsStartFailedTitle),
            "Không bắt đầu được phiên chụp từng bước"
        );
        assert_eq!(
            t(Lang::En, Msg::StepsStartFailedTitle),
            "Couldn't start step capture"
        );
        assert_eq!(
            t(Lang::Vi, Msg::StepsWorkerFailed),
            "Không chạy được tác vụ nền."
        );
        assert_eq!(
            t(Lang::En, Msg::StepsWorkerFailed),
            "The background task couldn't run."
        );
        assert_eq!(
            t(Lang::En, Msg::StepsSessionSaveFailedTitle),
            "Couldn't save the step capture session"
        );
    }

    #[test]
    fn help_text_mentions_step_capture() {
        for l in Lang::ALL {
            let h = help_text(l, &HelpKeys::default());
            assert!(h.contains(t(l, Msg::TrayStepsStart)), "{l:?}");
            assert!(h.contains("Ctrl+N"), "{l:?}");
        }
    }

    #[test]
    fn ocr_pro_strings_present_in_both_languages() {
        for m in [
            Msg::OcrCopyAll,
            Msg::OcrSingleLine,
            Msg::OcrParagraphs,
            Msg::OcrAutoCopy,
            Msg::OcrPickHint,
            Msg::OcrCopiedTip,
        ] {
            for l in Lang::ALL {
                assert!(!t(l, m).trim().is_empty(), "{m:?} {l:?}");
            }
        }
        assert_eq!(t(Lang::Vi, Msg::OcrCopyAll), "Chép tất cả");
        assert_eq!(t(Lang::En, Msg::OcrCopiedTip), "Copied");
    }

    #[test]
    fn help_text_mentions_ocr_pick_and_vietnamese_caveat() {
        let vi = help_text(Lang::Vi, &HelpKeys::default());
        let en = help_text(Lang::En, &HelpKeys::default());
        assert!(vi.contains("Ctrl+bấm"));
        assert!(
            vi.contains("Windows chưa có bộ nhận dạng tiếng Việt, chữ có dấu có thể bị mất dấu.")
        );
        assert!(en.contains("Ctrl+click"));
        assert!(en.contains(
            "Windows has no Vietnamese text recognizer yet, so accented letters may lose their marks."
        ));
    }

    #[test]
    fn record_gif_strings() {
        let pairs = [
            (
                Msg::OverlayRecordGif,
                "Ghi GIF (Ctrl+G)",
                "Record GIF (Ctrl+G)",
            ),
            (Msg::RecordStatus, "Đang ghi", "Recording"),
            (Msg::RecordSaving, "Đang lưu…", "Saving…"),
            (
                Msg::RecordNotScanned,
                "GIF không được quét",
                "GIF is not scanned",
            ),
            (
                Msg::StepsExportCancelledTitle,
                "Đã hủy xuất",
                "Export cancelled",
            ),
            (
                Msg::StepsExportCancelledBody,
                "Chưa có tệp nào được tạo.",
                "No files were created.",
            ),
            (
                Msg::StepsImageUnreadable,
                "Không đọc được ảnh của bước.",
                "Couldn't read the step image.",
            ),
            (Msg::RecordStop, "Dừng", "Stop"),
            (Msg::RecordCancel, "Hủy", "Cancel"),
            (Msg::RecordSavedTitle, "Đã lưu GIF", "GIF saved"),
            (
                Msg::RecordFailedTitle,
                "Ghi GIF thất bại",
                "Recording failed",
            ),
            (
                Msg::RecordInvalidRegion,
                "Vùng ghi không hợp lệ.",
                "Invalid recording region.",
            ),
            (
                Msg::RecordMaxDuration,
                "Đã đạt thời lượng ghi tối đa (60 giây).",
                "Maximum recording length reached (60 seconds).",
            ),
        ];
        for (m, vi, en) in pairs {
            assert_eq!(t(Lang::Vi, m), vi, "{m:?}");
            assert_eq!(t(Lang::En, m), en, "{m:?}");
        }
        assert!(NOTICE_TITLES.contains(&Msg::RecordSavedTitle));
        assert!(NOTICE_TITLES.contains(&Msg::RecordFailedTitle));
        assert!(NOTICE_TITLES.contains(&Msg::RecordNoRoomTitle));
        assert_eq!(
            t(Lang::Vi, Msg::RecordNoRoomTitle),
            "Không có chỗ cho thanh ghi GIF"
        );
        assert_eq!(
            t(Lang::En, Msg::RecordNoRoomTitle),
            "No room for the recording bar"
        );
        assert_eq!(
            record_saved_with_note(Lang::Vi, t(Lang::Vi, Msg::RecordMaxDuration), "C:/a.gif"),
            "Đã đạt thời lượng ghi tối đa (60 giây). Đã lưu GIF: C:/a.gif"
        );
        assert_eq!(
            record_saved_with_note(Lang::En, "Note.", "a.gif"),
            "Note. GIF saved: a.gif"
        );
        for l in Lang::ALL {
            for f in [
                record_capture_failed,
                record_encode_failed,
                record_io_failed,
                record_thread_failed,
            ] {
                assert!(f(l, "DETAIL42").contains("DETAIL42"), "{l:?}");
            }
            assert!(record_encode_failed(l, "x").starts_with(t(l, Msg::RecordEncodeFailed)));
            let h = help_text(l, &HelpKeys::default());
            assert!(h.contains("Ctrl+G"), "{l:?}");
            assert!(h.contains("9. "), "{l:?}");
            assert!(
                h.contains(&(crate::gif_record::MAX_DURATION_MS / 1000).to_string()),
                "{l:?}"
            );
        }
        let vi = help_text(Lang::Vi, &HelpKeys::default());
        let en = help_text(Lang::En, &HelpKeys::default());
        assert!(vi.contains("9. Ghi GIF"));
        assert!(en.contains("9. Recording a GIF"));
    }

    #[test]
    fn privacy_review_texts() {
        assert_eq!(
            privacy_review_title(Lang::Vi, None),
            "Kiểm tra thông tin nhạy cảm"
        );
        assert_eq!(
            privacy_review_title(Lang::Vi, Some((3, 12))),
            "Kiểm tra thông tin nhạy cảm · Bước 3/12"
        );
        assert_eq!(
            privacy_review_title(Lang::En, Some((3, 12))),
            "Check for sensitive information · Step 3/12"
        );
        let save_vi = t(Lang::Vi, Msg::PrivacyActionSave);
        assert_eq!(
            privacy_found(Lang::Vi, 3, save_vi),
            "Tìm thấy 3 mục · Lưu ảnh"
        );
        assert_eq!(
            privacy_found(Lang::En, 1, t(Lang::En, Msg::PrivacyActionSave)),
            "Found 1 item · Save image"
        );
        assert_eq!(privacy_found(Lang::En, 2, "Pin"), "Found 2 items · Pin");
        assert_eq!(privacy_scan_tile(Lang::Vi, 2, 5), "dải 2/5");
        assert_eq!(privacy_scan_tile(Lang::En, 2, 5), "part 2 of 5");
        assert_eq!(
            t(Lang::Vi, Msg::PrivacyNote),
            "Tự động có thể bỏ sót. Hãy xem lại ảnh."
        );
        assert_eq!(
            t(Lang::Vi, Msg::PrivacyConfirmEmpty),
            "Không có vùng nào được che. Tiếp tục mà không che?"
        );
        assert_eq!(
            t(Lang::Vi, Msg::PrivacyConfirmUnscanned),
            "Ảnh chưa được kiểm tra. Vẫn tiếp tục?"
        );
        assert_eq!(
            t(Lang::Vi, Msg::PrivacyScanning),
            "Đang quét thông tin nhạy cảm…"
        );
        assert_eq!(t(Lang::Vi, Msg::PrivacyManual), "Vùng tự thêm");
        assert_eq!(t(Lang::Vi, Msg::PrivacyApply), "Che và tiếp tục");
    }
}
