//! Kiểm thử bộ dò. Mọi dữ liệu là giả; chuỗi trông như khóa thật được ghép lúc chạy.
//! Chữ có dấu viết bằng `\u{..}`. Không in chữ mẫu ra đâu cả.

use super::*;
use crate::geometry::Rect;
use crate::ocr_text::{OcrLine, OcrWord};

const H: i32 = 20;

fn rc(x: i32, y: i32, w: i32, h: i32) -> Rect {
    Rect { x, y, w, h }
}

/// Dòng ở hàng `row`: mỗi từ rộng 10 px/ký tự, cách nhau `gap` px.
fn line_gap(text: &str, row: i32, gap: i32) -> OcrLine {
    let mut x = 0;
    let mut words = Vec::new();
    for w in text.split(' ').filter(|w| !w.is_empty()) {
        let width = 10 * w.chars().count() as i32;
        words.push(OcrWord {
            text: w.to_string(),
            rect: rc(x, row * 30, width, H),
        });
        x += width + gap;
    }
    let right = words.last().map_or(0, |w| w.rect.right());
    OcrLine {
        text: text.to_string(),
        rect: rc(0, row * 30, right, H),
        words,
    }
}

fn line(text: &str, row: i32) -> OcrLine {
    line_gap(text, row, 10)
}

fn scan(lines: &[&str]) -> Vec<Finding> {
    let v: Vec<OcrLine> = lines
        .iter()
        .enumerate()
        .map(|(i, t)| line(t, i as i32))
        .collect();
    detect(&v)
}

fn one(text: &str) -> Vec<Finding> {
    scan(&[text])
}

fn cats(f: &[Finding]) -> Vec<Category> {
    f.iter().map(|f| f.category).collect()
}

/// Đúng một phát hiện loại `c`, trả nó.
fn single(text: &str, c: Category) -> Finding {
    let f = one(text);
    assert_eq!(cats(&f), vec![c], "unexpected findings");
    f.into_iter().next().unwrap()
}

fn none(text: &str) {
    let f = one(text);
    assert!(f.is_empty(), "false positive: {:?}", cats(&f));
}

/// Khung từ thứ `k` (đếm từ 0) của dòng 0 khi dàn bằng `line`.
fn word_rect(text: &str, k: usize) -> Rect {
    line(text, 0).words[k].rect
}

// ---------- Email ----------

#[test]
fn email_plain_and_spaced() {
    let f = single("mail user.name+tag@example.co.uk now", Category::Email);
    assert_eq!(
        (f.confidence, f.rule),
        (Confidence::High, RuleId::EmailPlain)
    );
    assert_eq!(
        f.rects,
        vec![word_rect("mail user.name+tag@example.co.uk now", 1)]
    );
    assert_eq!(f.mapping, Mapping::Word);
    let f = single("user @ example.com", Category::Email);
    assert_eq!(
        (f.confidence, f.rule),
        (Confidence::Medium, RuleId::EmailSpaced)
    );
    let t = "user @ example.com";
    let want = text::union(word_rect(t, 0), word_rect(t, 2));
    assert_eq!(f.rects, vec![want]);
}

#[test]
fn email_rejects() {
    none("ping @user please");
    none("icon x@2x.png");
    none("logo@3x.jpg");
    none("a@b.c");
    none("mail @ ");
    none("name@localhost");
}

// ---------- Phone ----------

#[test]
fn phone_vn_mobile() {
    for t in [
        "0912 345 678",
        "0912.345.678",
        "0912-345-678",
        "0912345678",
        "+84 912 345 678",
        "84912345678",
        "call 0387654321 now",
    ] {
        let f = single(t, Category::Phone);
        assert_eq!(
            (f.confidence, f.rule),
            (Confidence::High, RuleId::PhoneVnMobile)
        );
    }
}

#[test]
fn phone_vn_fixed_and_intl() {
    let f = single("028 3823 4567", Category::Phone);
    assert_eq!(
        (f.confidence, f.rule),
        (Confidence::Medium, RuleId::PhoneVnFixed)
    );
    let f = single("+84 28 3823 4567", Category::Phone);
    assert_eq!(
        (f.confidence, f.rule),
        (Confidence::High, RuleId::PhoneVnFixed)
    );
    for t in ["+1 415 555 2671", "+44 20 7946 0958", "+81-3-1234-5678"] {
        let f = single(t, Category::Phone);
        assert_eq!(
            (f.confidence, f.rule),
            (Confidence::High, RuleId::PhoneIntl)
        );
    }
    none("+12 345"); // quá ngắn
}

#[test]
fn phone_labeled_same_line_or_line_above() {
    none("1900 1234 56");
    let f = single("Hotline: 1900 1234 56", Category::Phone);
    assert_eq!(
        (f.confidence, f.rule),
        (Confidence::Medium, RuleId::PhoneLabeled)
    );
    // "Số điện thoại" (viết bằng escape)
    let f = single(
        "S\u{1ed1} \u{111}i\u{1ec7}n tho\u{1ea1}i 415 555 2671",
        Category::Phone,
    );
    assert_eq!(f.rule, RuleId::PhoneLabeled);
    let f = scan(&["Phone", "415 555 2671"]);
    assert_eq!(cats(&f), vec![Category::Phone]);
    assert_eq!(f[0].rects[0].y, 30);
    // nhãn hai dòng trên: không tính
    assert!(scan(&["Phone", "x", "415 555 2671"]).is_empty());
    // "hotel" không phải nhãn "tel"
    none("hotel 415 555 2671");
}

#[test]
fn phone_false_positives() {
    for t in [
        "2026-11-22",
        "22/11/2026",
        "22-11-2026",
        "at 10:30:15",
        "10.0.0.1",
        "192.168.100.200",
        "v1.2.3",
        "version 10.0.19045",
        "1,000,000",
        "price 1.500.000 \u{111}",
        "phone 1234567890 USD",
        "tel 1234567890$",
        "abc0912345678",
        "0912345678abc",
        "id 123456789012",
    ] {
        none(t);
    }
}

// ---------- Thẻ ----------

#[test]
fn card_luhn_and_brand() {
    let f = single("4111 1111 1111 1111", Category::CreditCard);
    assert_eq!(
        (f.confidence, f.rule),
        (Confidence::High, RuleId::CardBrand)
    );
    let t = "4111 1111 1111 1111";
    assert_eq!(f.rects, vec![text::union(word_rect(t, 0), word_rect(t, 3))]);
    let f = single("Amex 3782 822463 10005", Category::CreditCard);
    assert_eq!(f.confidence, Confidence::High);
    let f = single("4111-1111-1111-1111", Category::CreditCard);
    assert_eq!(f.confidence, Confidence::High);
    let f = single("4111111111111111", Category::CreditCard);
    assert_eq!(f.confidence, Confidence::High);
    none("4111 1111 1111 1112"); // Luhn sai
    none("0000 0000 0000 0000"); // toàn một chữ số
}

#[test]
fn card_ocr_confusions_only_inside_digit_runs() {
    let f = single("4111 1111 1111 111l", Category::CreditCard);
    assert_eq!(f.rule, RuleId::CardBrand);
    let f = single("4111 1111 1111 11I1", Category::CreditCard);
    assert_eq!(f.rule, RuleId::CardBrand);
    none("lOlO SSSS BBBB IIII");
}

#[test]
fn card_unknown_prefix_needs_label() {
    assert!(detect::card_luhn_for_tests(&[
        1, 2, 3, 4, 5, 6, 7, 8, 1, 2, 3, 4, 5, 6, 7, 0
    ]));
    none("1234567812345670"); // 16 số liền, không tiền tố, không nhãn
    none("1234 5678 1234 5670");
    let f = single("card 1234 5678 1234 5670", Category::CreditCard);
    assert_eq!(
        (f.confidence, f.rule),
        (Confidence::Medium, RuleId::CardLabeled)
    );
    // "Thẻ" (escape)
    let f = single("Th\u{1ebb}: 1234 5678 1234 5670", Category::CreditCard);
    assert_eq!(f.rule, RuleId::CardLabeled);
}

// ---------- JWT / token ----------

fn jwt() -> String {
    // đầu {"alg":"HS256"}, thân {"sub":"demo"}, chữ ký giả
    format!(
        "{}.{}.{}",
        concat!("eyJhbGciOi", "JIUzI1NiJ9"),
        concat!("eyJzdWIiOi", "JkZW1vIn0"),
        concat!("ZmFrZXNp", "Z25hdHVyZQ")
    )
}

#[test]
fn jwt_decodes_or_not() {
    let t = jwt();
    let f = single(&t, Category::Token);
    assert_eq!((f.confidence, f.rule), (Confidence::High, RuleId::TokenJwt));
    let f = single(&format!("{t}."), Category::Token);
    assert_eq!(f.rule, RuleId::TokenJwt);
    // alg:none, chữ ký rỗng
    let f = single(
        &format!(
            "{}.{}.",
            concat!("eyJhbGciOi", "JIUzI1NiJ9"),
            concat!("eyJzdWIiOi", "JkZW1vIn0")
        ),
        Category::Token,
    );
    assert_eq!(f.rule, RuleId::TokenJwt);
    let broken = concat!("eyJxxxxxxxx", ".yyyyyyyyyy", ".zzzzzzzzzz");
    none(broken);
    let f = single(&format!("token: {broken}"), Category::Token);
    assert_eq!(
        (f.confidence, f.rule),
        (Confidence::Medium, RuleId::TokenLabeled)
    );
}

#[test]
fn bearer_and_labels() {
    let f = single("Authorization: Bearer abcdEFGH12345678xyz", Category::Token);
    assert_eq!(
        (f.confidence, f.rule),
        (Confidence::High, RuleId::TokenBearer)
    );
    let t = "Authorization: Bearer abcdEFGH12345678xyz";
    assert_eq!(f.rects, vec![word_rect(t, 2)]);
    none("Bearer short");
    let f = single("access_token=abcdEFGH12345678xyz", Category::Token);
    assert_eq!(f.rule, RuleId::TokenLabeled);
    none("token: short");
}

#[test]
fn jwt_inside_bearer_is_one_finding() {
    let t = format!("Authorization: Bearer {}", jwt());
    let f = single(&t, Category::Token);
    assert_eq!(f.confidence, Confidence::High);
    assert_eq!(f.rects, vec![word_rect(&t, 2)]);
}

#[test]
fn jwt_beats_password_label() {
    let t = format!("password: {}", jwt());
    let f = single(&t, Category::Token);
    assert_eq!(f.rule, RuleId::TokenJwt);
}

// ---------- Khóa API ----------

#[test]
fn api_key_prefixes() {
    let cases: Vec<(String, RuleId)> = vec![
        (
            format!("{}{}", "AK", "IAEXAMPLEKEY012345"),
            RuleId::ApiKeyAws,
        ),
        (
            format!("{}{}", "AS", "IAEXAMPLEKEY012345"),
            RuleId::ApiKeyAws,
        ),
        (
            format!("{}{}", "AI", "zaSyEXAMPLE_example-0123456789abcdefg"),
            RuleId::ApiKeyGoogle,
        ),
        (
            format!("{}{}", "gh", "p_EXAMPLEexample0123456789EXAMPLEexamp"),
            RuleId::ApiKeyGithub,
        ),
        (
            format!("{}{}", "gh", "s_EXAMPLEexample0123456789EXAMPLEexamp"),
            RuleId::ApiKeyGithub,
        ),
        (
            format!("{}{}", "github", "_pat_EXAMPLE0123456789_example"),
            RuleId::ApiKeyGithub,
        ),
        (
            format!("{}{}", "glp", "at-EXAMPLEexample012345"),
            RuleId::ApiKeyGitlab,
        ),
        (
            format!("{}{}", "xo", "xb-1234567890-EXAMPLE"),
            RuleId::ApiKeySlack,
        ),
        (
            format!("{}{}", "sk", "-EXAMPLEexample0123456789"),
            RuleId::ApiKeyOpenAi,
        ),
        (
            format!("{}{}", "sk", "-proj-EXAMPLEexample0123456789"),
            RuleId::ApiKeyOpenAi,
        ),
        (
            format!("{}{}", "sk", "-ant-EXAMPLEexample0123456789"),
            RuleId::ApiKeyOpenAi,
        ),
        (
            format!("{}{}", "sk_", "live_EXAMPLEexample0123"),
            RuleId::ApiKeyStripe,
        ),
        (
            format!("{}{}", "sk_", "test_EXAMPLEexample0123"),
            RuleId::ApiKeyStripe,
        ),
        (
            format!("{}{}", "pk_", "live_EXAMPLEexample0123"),
            RuleId::ApiKeyStripe,
        ),
        (
            format!(
                "{}{}.{}",
                "SG", ".EXAMPLEexample01234567", "EXAMPLEexample0123456789EXAMPLEexample01234"
            ),
            RuleId::ApiKeySendgrid,
        ),
        (
            format!("{}{}", "np", "m_EXAMPLEexample0123456789EXAMPLEexamp"),
            RuleId::ApiKeyNpm,
        ),
    ];
    for (k, rule) in cases {
        let f = single(&format!("key {k} end"), Category::ApiKey);
        assert_eq!(
            (f.confidence, f.rule),
            (Confidence::High, rule),
            "rule {rule:?}"
        );
        let f = single(&format!("AWS_ID={k}"), Category::ApiKey);
        assert_eq!(f.rule, rule);
    }
}

#[test]
fn api_key_wrong_length_or_no_prefix() {
    none(&format!("{}{}", "AK", "IAEXAMPLE")); // quá ngắn
    none(&format!("{}{}", "AK", "IAEXAMPLEKEY0123456789")); // quá dài
    none(&format!("{}{}", "gh", "p_short"));
    none("sk-short");
    none("risk-assessment-2026-report-final-v2"); // "sk-" giữa từ
}

#[test]
fn api_key_labeled() {
    let f = single("api_key=AbCdEfGh12345678XY", Category::ApiKey);
    assert_eq!(
        (f.confidence, f.rule),
        (Confidence::Medium, RuleId::ApiKeyLabeled)
    );
    let f = single("\"x-api-key\": \"AbCdEfGh12345678XY\"", Category::ApiKey);
    assert_eq!(f.rule, RuleId::ApiKeyLabeled);
    let t = "client_secret: AbCdEfGh12345678XY";
    let f = single(t, Category::ApiKey);
    assert_eq!(f.rects, vec![word_rect(t, 1)]);
    none("api_key=abcdefghijklmnopqrst"); // một nhóm ký tự
    none("api_key=Ab12"); // ngắn
}

// ---------- Mật khẩu ----------

#[test]
fn password_value_only() {
    let t = "password: hunter2";
    let f = single(t, Category::Password);
    assert_eq!(
        (f.confidence, f.rule),
        (Confidence::Medium, RuleId::PasswordLabeled)
    );
    assert_eq!(f.rects, vec![word_rect(t, 1)]);
    // nhãn và giá trị cùng một từ → che cả từ
    let t = "pwd:hunter2";
    let f = single(t, Category::Password);
    assert_eq!(f.rects, vec![word_rect(t, 0)]);
    // giá trị có khoảng trắng ≤ 64 ký tự → tới hết dòng
    let t = "Password: correct horse battery";
    let f = single(t, Category::Password);
    assert_eq!(f.rects, vec![text::union(word_rect(t, 1), word_rect(t, 3))]);
    // dừng trước nhãn kế
    let t = "pass: abc123 user: bob";
    let f = single(t, Category::Password);
    assert_eq!(f.rects, vec![word_rect(t, 1)]);
    // "mật khẩu" (escape)
    let t = "M\u{1ead}t kh\u{1ea9}u: abc123";
    let f = single(t, Category::Password);
    assert_eq!(f.rects, vec![word_rect(t, 2)]);
}

#[test]
fn password_rejects() {
    none("Password");
    none("Forgot password?");
    none("password:");
    none("password: ******");
    none("password: \u{2022}\u{2022}\u{2022}\u{2022}");
    none("Password: \u{25CF}\u{25CF}\u{25CF}");
    none("passport: B1234567");
    none("spinner = on");
}

#[test]
fn password_env() {
    let t = "DB_PASSWORD=s3cretValue";
    let f = single(t, Category::Password);
    assert_eq!(f.rects, vec![word_rect(t, 0)]);
    let f = single("export APP_KEY=abc", Category::Password);
    assert_eq!(f.rule, RuleId::PasswordEnv);
    none("monkey=banana");
    none("MODE=prod");
}

// ---------- PEM ----------

fn begin() -> String {
    format!("-----BEGIN {}-----", concat!("RSA PRIVATE", " KEY"))
}
fn end() -> String {
    format!("-----END {}-----", concat!("RSA PRIVATE", " KEY"))
}

#[test]
fn pem_with_end() {
    let (b, e) = (begin(), end());
    let lines = [
        "before",
        b.as_str(),
        "MIIEowIBAAKCAQEAexampleEXAMPLE0123",
        "EXAMPLEexample0123456789+/abcdEF==",
        e.as_str(),
        "after text",
    ];
    let f = scan(&lines);
    assert_eq!(cats(&f), vec![Category::PrivateKey]);
    assert_eq!(
        (f[0].confidence, f[0].rule),
        (Confidence::High, RuleId::PemBlock)
    );
    let ys: Vec<i32> = f[0].rects.iter().map(|r| r.y).collect();
    assert_eq!(ys, vec![30, 60, 90, 120]);
}

#[test]
fn pem_without_end_stops_at_non_base64() {
    let b = format!("-----BEGIN {}-----", concat!("OPENSSH PRIVATE", " KEY"));
    let lines = [
        b.as_str(),
        "b3BlbnNzaC1rZXktdjEAAAAAexample",
        "EXAMPLEexample0123456789+/abcdEF",
        "Some normal text here.",
    ];
    let f = scan(&lines);
    assert_eq!(cats(&f), vec![Category::PrivateKey]);
    assert_eq!(f[0].rects.len(), 3);
    let b = format!("-----BEGIN {}-----", concat!("PUBLIC", " KEY"));
    assert!(scan(&[b.as_str(), "MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8A"]).is_empty());
}

// ---------- Nhiều phát hiện, nhiều dòng ----------

#[test]
fn many_findings_one_line() {
    let f = one("mail a@example.com or b@example.org call 0912 345 678");
    assert_eq!(
        cats(&f),
        vec![Category::Email, Category::Email, Category::Phone]
    );
    let ids: Vec<u32> = f.iter().map(|f| f.id).collect();
    assert_eq!(ids, vec![0, 1, 2]);
}

#[test]
fn many_lines() {
    let f = scan(&[
        "Contact: user@example.com",
        "nothing here",
        "4111 1111 1111 1111",
        "password: hunter2",
    ]);
    assert_eq!(
        cats(&f),
        vec![Category::Email, Category::CreditCard, Category::Password]
    );
    assert_eq!(f[1].rects[0].y, 60);
}

// ---------- Dương tính giả ----------

#[test]
fn false_positive_corpus() {
    for t in [
        "Meeting 2026-11-22 at 10:30",
        "22/11/2026",
        "Server 10.0.0.1 and 192.168.1.20",
        "darkshot 0.5.0 build 1.2.3",
        "Total: $1,234.56 or 1.500.000 VND",
        "commit 3f786850e387550fdab836ed7e6dc881de23001b",
        "id 123e4567-e89b-12d3-a456-426614174000",
        "thanks @mention and @team",
        "assets/icon@2x.png",
        "1234567812345670",
        "Password",
        "password: ****",
        "Order #2026112212345678",
        "Page 1 of 12",
    ] {
        none(t);
    }
}

// ---------- Hình học ----------

#[test]
fn split_token_words_are_joined() {
    // OCR tách khóa thành hai từ, khe 2 px < 0,35 × 20
    let k1 = format!("{}{}", "AK", "IAEXAMPLE");
    let t = format!("{k1} KEY012345");
    let l = line_gap(&t, 0, 2);
    let f = detect(std::slice::from_ref(&l));
    assert_eq!(cats(&f), vec![Category::ApiKey]);
    assert_eq!(
        f[0].rects,
        vec![text::union(l.words[0].rect, l.words[1].rect)]
    );
    assert_eq!(f[0].mapping, Mapping::Word);
    // khe lớn: không ghép, không phát hiện
    assert!(detect(&[line_gap(&t, 0, 10)]).is_empty());
}

#[test]
fn split_card_groups_found_once() {
    let l = line_gap("4111 1111 1111 1111", 0, 2);
    let f = detect(std::slice::from_ref(&l));
    assert_eq!(cats(&f), vec![Category::CreditCard]);
    assert_eq!(f[0].rects.len(), 1);
}

#[test]
fn line_fallback_when_no_words_or_bad_rect() {
    let l = OcrLine {
        text: "mail user@example.com".into(),
        rect: rc(5, 6, 300, 20),
        words: Vec::new(),
    };
    let f = detect(&[l]);
    assert_eq!(cats(&f), vec![Category::Email]);
    assert_eq!(f[0].rects, vec![rc(5, 6, 300, 20)]);
    assert_eq!(f[0].mapping, Mapping::Line);

    let mut l = line("mail user@example.com", 0);
    l.words[1].rect.w = 0;
    let lr = l.rect;
    let f = detect(&[l]);
    assert_eq!(f[0].rects, vec![lr]);
    assert_eq!(f[0].mapping, Mapping::Line);
}

#[test]
fn results_carry_no_text() {
    let k = format!("{}{}", "AK", "IAEXAMPLEKEY012345");
    let f = scan(&[
        "user@example.com",
        "password: hunter2",
        k.as_str(),
        "4111 1111 1111 1111",
    ]);
    assert_eq!(f.len(), 4);
    let d = format!("{f:?}");
    for s in ["example", "hunter", "EXAMPLE", "4111"] {
        assert!(!d.contains(s), "finding debug leaks text");
    }
    let r = ScanReport {
        findings: f,
        tiles: 1,
        elapsed_ms: 3,
    };
    assert!(!format!("{r:?}").contains("hunter"));
}

#[test]
fn empty_input() {
    assert!(detect(&[]).is_empty());
    assert!(one("").is_empty());
}
