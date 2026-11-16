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
    ChkShowSuccessNotices => (
        "Hiện các thông báo lưu và sao chép",
        "Show save and copy notifications"
    ),
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
];

/// Chữ trên nút OK: giống nhau ở mọi ngôn ngữ nên nằm ngoài bảng `Msg`
/// (bảng đòi hai bản khác nhau).
pub const OK_LABEL: &str = "OK";

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
    match lang {
        Lang::Vi => format!(
            "1. Chụp màn hình\n\
{region} Màn hình đứng yên và tối đi. Kéo chuột để chọn vùng; kéo các tay nắm để đổi cỡ, kéo bên trong khung để di chuyển.\n\
Chụp cả màn hình bằng phím tắt:\n\
{save_full}\n\
{upload_full}\n\
\n\
2. Thanh công cụ và phím tắt\n\
Sau khi chọn vùng, dùng nút trên thanh công cụ hoặc phím tắt:\n\
- Ctrl+D: tải ảnh lên kho R2 và chép link.\n\
- Ctrl+S: lưu ảnh vào thư mục lưu.\n\
- Ctrl+C: chép ảnh vào clipboard.\n\
- Esc: hủy và đóng lớp phủ.\n\
\n\
3. Tùy chọn\n\
Mở bằng menu khay, mục \"Tùy chọn...\". Bấm OK để lưu, Hủy để bỏ thay đổi.\n\
- Thông thường: khởi động cùng Windows, thư mục lưu, ngôn ngữ và ba ô tích:\n\
\x20\x20+ {notices}: bỏ tích thì không hiện các thông báo \"{saved}\", \"{copied}\", \"{link}\". Thông báo lỗi luôn hiện.\n\
\x20\x20+ {keep}: lần chụp vùng sau mở sẵn vùng đã chọn lần trước, cho tới khi thoát darkshot.\n\
\x20\x20+ {cursor}: ảnh chụp có cả con trỏ chuột.\n\
- Phím tắt: ba dòng \"{region_label}\", \"{save_label}\" và \"{upload_label}\". Ô tích đầu dòng bật hoặc tắt phím của dòng đó; dòng tắt không giữ phím nào. Bấm {change} rồi nhấn tổ hợp mới; Esc để bỏ qua.\n\
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
Ảnh luôn được lưu vào thư mục lưu, nên không bị mất. Kiểm tra mạng và thông tin trong tab Lưu trữ R2, rồi thử lại."
        ),
        Lang::En => format!(
            "1. Taking a screenshot\n\
{region} The screen dims and freezes. Drag to select a region; drag the handles to resize it, or drag inside the frame to move it.\n\
Full-screen captures with a hotkey:\n\
{save_full}\n\
{upload_full}\n\
\n\
2. Toolbar and keys\n\
Once a region is selected, use the toolbar buttons or the keys:\n\
- Ctrl+D: upload the image to your R2 bucket and copy the link.\n\
- Ctrl+S: save the image to the save folder.\n\
- Ctrl+C: copy the image to the clipboard.\n\
- Esc: cancel and close the overlay.\n\
\n\
3. Options\n\
Open it from the tray menu, \"Options...\". Click OK to save, Cancel to discard changes.\n\
- General: start with Windows, save folder, language and three check boxes:\n\
\x20\x20+ {notices}: when unchecked, the \"{saved}\", \"{copied}\" and \"{link}\" notifications are not shown. Error notifications always appear.\n\
\x20\x20+ {keep}: the next region capture starts with the previously selected region, until you exit darkshot.\n\
\x20\x20+ {cursor}: screenshots include the mouse pointer.\n\
- Hotkeys: three rows, \"{region_label}\", \"{save_label}\" and \"{upload_label}\". The check box at the start of a row turns its key on or off; a row that is off holds no key. Click {change}, then press the new combination; Esc skips.\n\
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
The image is always saved to the save folder, so it is not lost. Check your network and the details in the R2 storage tab, then try again."
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

/// "Phiên bản 0.1.0" | "Version 0.1.0".
pub fn about_version(lang: Lang, version: &str) -> String {
    match lang {
        Lang::Vi => format!("Phiên bản {version}"),
        Lang::En => format!("Version {version}"),
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
            };
            let s = help_text(l, &all);
            for k in [
                "KEY_A", "KEY_B", "KEY_C", "Ctrl+D", "Ctrl+S", "Ctrl+C", "Esc",
            ] {
                assert!(s.contains(k), "{l:?} thiếu {k}");
            }
            assert!(!s.contains("http"));
            assert!(s.contains(t(l, Msg::BtnChange)));
            let none = help_text(l, &HelpKeys::default());
            for k in ["KEY_A", "KEY_B", "KEY_C"] {
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
    fn help_text_mentions_format_tab() {
        for l in Lang::ALL {
            let s = help_text(l, &HelpKeys::default());
            assert!(s.contains(t(l, Msg::TabFormat)), "{l:?} thiếu tên tab");
            assert!(s.contains("PNG"), "{l:?} thiếu PNG");
            assert!(s.contains("JPEG"), "{l:?} thiếu JPEG");
        }
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
}
