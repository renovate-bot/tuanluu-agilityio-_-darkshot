//! Kiểu kết quả của Privacy Shield. Không kiểu nào ở đây mang chữ OCR hay giá trị trùng khớp.

use crate::geometry::Rect;
use crate::i18n::{t, Lang, Msg, PRIVACY_EMAIL_LABEL};

/// Loại thông tin nhạy cảm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Email,
    Phone,
    CreditCard,
    ApiKey,
    Token,
    Password,
    PrivateKey,
}

impl Category {
    pub fn label(self, lang: Lang) -> &'static str {
        let m = match self {
            Category::Email => return PRIVACY_EMAIL_LABEL,
            Category::Phone => Msg::PrivacyCatPhone,
            Category::CreditCard => Msg::PrivacyCatCard,
            Category::ApiKey => Msg::PrivacyCatApiKey,
            Category::Token => Msg::PrivacyCatToken,
            Category::Password => Msg::PrivacyCatPassword,
            Category::PrivateKey => Msg::PrivacyCatPrivateKey,
        };
        t(lang, m)
    }

    /// Thứ tự ưu tiên khi chồng nhau: số nhỏ thắng
    /// (PEM > JWT/token > Khóa API > Thẻ > Mật khẩu > Email > Điện thoại).
    pub(crate) fn priority(self) -> u8 {
        match self {
            Category::PrivateKey => 0,
            Category::Token => 1,
            Category::ApiKey => 2,
            Category::CreditCard => 3,
            Category::Password => 4,
            Category::Email => 5,
            Category::Phone => 6,
        }
    }
}

/// Độ tin cậy: Cao (cấu trúc đã kiểm chứng) hoặc Trung bình (dựa vào nhãn/ngữ cảnh).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Confidence {
    High,
    Medium,
}

impl Confidence {
    pub fn label(self, lang: Lang) -> &'static str {
        match self {
            Confidence::High => t(lang, Msg::PrivacyConfHigh),
            Confidence::Medium => t(lang, Msg::PrivacyConfMedium),
        }
    }
}

/// Khung lấy từ khung từ (`Word`) hay phải dùng khung cả dòng (`Line`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mapping {
    Word,
    Line,
}

/// Luật đã cho ra phát hiện (chỉ tên luật, không chữ).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleId {
    EmailPlain,
    EmailSpaced,
    PhoneVnMobile,
    PhoneVnFixed,
    PhoneIntl,
    PhoneLabeled,
    CardBrand,
    CardLabeled,
    ApiKeyAws,
    ApiKeyGoogle,
    ApiKeyGithub,
    ApiKeyGitlab,
    ApiKeySlack,
    ApiKeyOpenAi,
    ApiKeyStripe,
    ApiKeySendgrid,
    ApiKeyNpm,
    ApiKeyLabeled,
    TokenJwt,
    TokenBearer,
    TokenLabeled,
    PasswordLabeled,
    PasswordEnv,
    PemBlock,
}

/// Một phát hiện. KHÔNG có trường chữ nào.
#[derive(Clone, Debug, PartialEq)]
pub struct Finding {
    /// Tạm, duy nhất trong một `ScanReport` (đánh lại 0..n sau khi gộp).
    pub id: u32,
    pub category: Category,
    pub confidence: Confidence,
    /// px ảnh gốc, ≥ 1 khung.
    pub rects: Vec<Rect>,
    pub rule: RuleId,
    pub mapping: Mapping,
}

/// Kết quả một lượt quét.
#[derive(Clone, Debug, PartialEq)]
pub struct ScanReport {
    pub findings: Vec<Finding>,
    pub tiles: u32,
    pub elapsed_ms: u32,
}

/// Lỗi quét. Không mang chuỗi.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanError {
    NoLanguage,
    Failed,
    Timeout,
    TooLarge,
}

impl ScanError {
    pub fn message(self, lang: Lang) -> &'static str {
        let m = match self {
            ScanError::NoLanguage => Msg::PrivacyErrNoLanguage,
            ScanError::Failed => Msg::PrivacyErrFailed,
            ScanError::Timeout => Msg::PrivacyErrTimeout,
            ScanError::TooLarge => Msg::PrivacyErrTooLarge,
        };
        t(lang, m)
    }
}

/// Một vùng che trong cửa sổ Duyệt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Redaction {
    pub rect: Rect,
    pub source: RedactionSource,
    pub selected: bool,
}

/// Nguồn của vùng che.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RedactionSource {
    Detected { finding: u32, category: Category },
    Manual,
}

/// Các vùng đã chọn để che (px ảnh gốc).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RedactionPlan {
    pub rects: Vec<Rect>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const CATS: [Category; 7] = [
        Category::Email,
        Category::Phone,
        Category::CreditCard,
        Category::ApiKey,
        Category::Token,
        Category::Password,
        Category::PrivateKey,
    ];

    #[test]
    fn labels_both_languages() {
        let want = [
            ("Email", "Email"),
            ("Số điện thoại", "Phone number"),
            ("Thẻ thanh toán", "Payment card"),
            ("Khóa API", "API key"),
            ("Token truy cập", "Access token"),
            ("Mật khẩu", "Password"),
            ("Khóa riêng", "Private key"),
        ];
        for (c, (vi, en)) in CATS.iter().zip(want) {
            assert_eq!(c.label(Lang::Vi), vi);
            assert_eq!(c.label(Lang::En), en);
        }
        assert_eq!(Confidence::High.label(Lang::Vi), "Cao");
        assert_eq!(Confidence::High.label(Lang::En), "High");
        assert_eq!(Confidence::Medium.label(Lang::Vi), "Trung bình");
        assert_eq!(Confidence::Medium.label(Lang::En), "Medium");
    }

    #[test]
    fn scan_errors_have_messages() {
        let all = [
            ScanError::NoLanguage,
            ScanError::Failed,
            ScanError::Timeout,
            ScanError::TooLarge,
        ];
        for e in all {
            for l in Lang::ALL {
                assert!(!e.message(l).trim().is_empty());
            }
            assert_ne!(e.message(Lang::Vi), e.message(Lang::En));
        }
        assert_eq!(ScanError::Timeout.message(Lang::Vi), "Quét quá lâu.");
    }

    #[test]
    fn priorities_are_distinct_and_ordered() {
        let mut p: Vec<u8> = CATS.iter().map(|c| c.priority()).collect();
        p.sort_unstable();
        p.dedup();
        assert_eq!(p.len(), 7);
        assert!(Category::PrivateKey.priority() < Category::Token.priority());
        assert!(Category::Token.priority() < Category::ApiKey.priority());
        assert!(Category::ApiKey.priority() < Category::CreditCard.priority());
        assert!(Category::CreditCard.priority() < Category::Password.priority());
        assert!(Category::Password.priority() < Category::Email.priority());
        assert!(Category::Email.priority() < Category::Phone.priority());
    }
}
