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
    ClipboardUnusedOnExit => ("không dùng clipboard lúc thoát", "the clipboard is not used on exit"),
    TrayOptions => ("Tùy chọn...", "Options..."),
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
    HotkeyReregisterFailedBody => (
        "Không đăng ký lại được phím tắt cũ. Hãy mở Tùy chọn và chọn lại phím tắt.",
        "Could not register the previous hotkey again. Open Options and pick the hotkey again."
    ),
    HotkeyTakenBody => ("Hãy đổi phím tắt trong Tùy chọn.", "Change the hotkey in Options."),
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
    AutostartExePath => ("Không xác định được đường dẫn của shotme", "Could not determine the path of shotme"),
    AutostartEnableFailed => ("Không bật được khởi động cùng Windows", "Could not enable starting with Windows"),
    AutostartDisableFailed => ("Không tắt được khởi động cùng Windows", "Could not disable starting with Windows"),
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
        Lang::Vi => format!("lỗi bộ mã hóa PNG: {err}"),
        Lang::En => format!("PNG encoder error: {err}"),
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

pub fn hotkey_inactive_title(lang: Lang, key: &str) -> String {
    match lang {
        Lang::Vi => format!("Phím {key} hiện không hoạt động"),
        Lang::En => format!("The {key} hotkey is not working"),
    }
}

pub fn hotkey_taken_title(lang: Lang, key: &str) -> String {
    match lang {
        Lang::Vi => format!("Phím {key} đang bị ứng dụng khác chiếm"),
        Lang::En => format!("Another app is using the {key} hotkey"),
    }
}

pub fn hotkey_taken_choose_other(lang: Lang, key: &str) -> String {
    match lang {
        Lang::Vi => format!("Phím {key} đang bị ứng dụng khác chiếm. Hãy chọn phím khác."),
        Lang::En => format!("Another app is using the {key} hotkey. Choose a different one."),
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
            assert!(hotkey_inactive_title(l, "A5").contains("A5"));
            assert!(hotkey_taken_title(l, "A6").contains("A6"));
            assert!(hotkey_taken_choose_other(l, "A7").contains("A7"));
        }
        assert_ne!(reason(Lang::Vi, "x"), reason(Lang::En, "x"));
        assert_ne!(
            hotkey_taken_title(Lang::Vi, "x"),
            hotkey_taken_title(Lang::En, "x")
        );
        assert_ne!(
            with_error_code(Lang::Vi, "x", "1"),
            with_error_code(Lang::En, "x", "1")
        );
    }

    #[test]
    fn hotkey_titles_fit_tray_limit() {
        let key = "Ctrl+Alt+Shift+Win+PrintScreen";
        for l in Lang::ALL {
            for title in [hotkey_taken_title(l, key), hotkey_inactive_title(l, key)] {
                assert!(title.encode_utf16().count() <= 63, "{l:?} {title}");
            }
        }
    }
}
