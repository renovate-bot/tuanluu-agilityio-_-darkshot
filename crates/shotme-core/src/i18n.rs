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
        todo!()
    }

    pub fn from_code(_s: &str) -> Option<Lang> {
        todo!()
    }

    pub fn native_name(self) -> &'static str {
        todo!()
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
            let _ = (lang, m);
            todo!()
        }
    };
}

messages! {
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
];

pub fn expiry_label(_lang: Lang, _e: Expiry) -> &'static str {
    todo!()
}

pub fn copy_failed_body(_lang: Lang, _err: &str) -> String {
    todo!()
}

pub fn encode_failed(_lang: Lang, _err: &str) -> String {
    todo!()
}

pub fn encoder_error(_lang: Lang, _err: &str) -> String {
    todo!()
}

pub fn upload_failed_body(_lang: Lang, _reason: &str, _path: &str) -> String {
    todo!()
}

pub fn save_failed_in_clipboard(_lang: Lang, _err: &str) -> String {
    todo!()
}

pub fn save_failed_no_clipboard(_lang: Lang, _err: &str) -> String {
    todo!()
}

pub fn http_error(_lang: Lang, _code: u16) -> String {
    todo!()
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
}
