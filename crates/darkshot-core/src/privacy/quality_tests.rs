//! Đo chất lượng bộ dò trên bộ mẫu tổng hợp (Task 133): độ chính xác, độ phủ, tỉ lệ nhận nhầm
//! mỗi loại và tổng. Mọi dữ liệu là GIẢ; chuỗi trông như khóa thật được ghép lúc chạy. Chữ có dấu
//! viết bằng `\u{..}`. Không in chữ mẫu ra đâu cả: kiểm thử chỉ so các con số (và chỉ số mẫu).
//!
//! Ngưỡng trong `quality_does_not_regress` = số đo thật lúc viết (ghi ở
//! `docs/testing/privacy-quality.md`); sửa bộ dò làm tốt hơn thì nâng ngưỡng, không hạ.

use super::*;
use crate::geometry::Rect;
use crate::ocr_text::{OcrLine, OcrWord};

const CATS: [Category; 7] = [
    Category::Email,
    Category::Phone,
    Category::CreditCard,
    Category::ApiKey,
    Category::Token,
    Category::Password,
    Category::PrivateKey,
];

fn cat_index(c: Category) -> usize {
    CATS.iter().position(|&x| x == c).unwrap()
}

/// Một mẫu: các dòng, khe giữa từ (px; < 7 thì OCR coi như tách một từ).
struct Sample {
    lines: Vec<String>,
    gap: i32,
}

fn s(t: &str) -> Sample {
    Sample {
        lines: vec![t.to_string()],
        gap: 10,
    }
}

fn so(t: String) -> Sample {
    Sample {
        lines: vec![t],
        gap: 10,
    }
}

fn multi(lines: Vec<String>) -> Sample {
    Sample { lines, gap: 10 }
}

/// Dòng ở hàng `row`: mỗi từ rộng 10 px/ký tự, cách nhau `gap` px, cao 20.
fn ocr_line(text: &str, row: i32, gap: i32) -> OcrLine {
    let mut x = 0;
    let mut words = Vec::new();
    for w in text.split(' ').filter(|w| !w.is_empty()) {
        let width = 10 * w.chars().count() as i32;
        words.push(OcrWord {
            text: w.to_string(),
            rect: Rect {
                x,
                y: row * 30,
                w: width,
                h: 20,
            },
        });
        x += width + gap;
    }
    let right = words.last().map_or(0, |w| w.rect.right());
    OcrLine {
        text: text.to_string(),
        rect: Rect {
            x: 0,
            y: row * 30,
            w: right,
            h: 20,
        },
        words,
    }
}

fn found(sample: &Sample) -> [bool; 7] {
    let lines: Vec<OcrLine> = sample
        .lines
        .iter()
        .enumerate()
        .map(|(i, t)| ocr_line(t, i as i32, sample.gap))
        .collect();
    let mut out = [false; 7];
    for f in detect(&lines) {
        out[cat_index(f.category)] = true;
    }
    out
}

fn k(a: &str, b: &str) -> String {
    format!("{a}{b}")
}

/// base64url không đệm (để dựng JWT giả lúc chạy).
fn b64url(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    for ch in data.chunks(3) {
        let n = ch.len();
        let v = (u32::from(ch[0]) << 16)
            | (u32::from(*ch.get(1).unwrap_or(&0)) << 8)
            | u32::from(*ch.get(2).unwrap_or(&0));
        for i in 0..=n {
            out.push(T[((v >> (18 - 6 * i)) & 63) as usize] as char);
        }
    }
    out
}

fn jwt(header: &str, payload: &str, sig: &str) -> String {
    format!(
        "{}.{}.{}",
        b64url(header.as_bytes()),
        b64url(payload.as_bytes()),
        sig
    )
}

fn pem_line(kind: &str, end: bool) -> String {
    let word = if end { "END" } else { "BEGIN" };
    let pk = concat!("PRIVATE", " KEY");
    if kind.is_empty() {
        format!("-----{word} {pk}-----")
    } else {
        format!("-----{word} {kind} {pk}-----")
    }
}

const B64: [&str; 3] = [
    "MIIEowIBAAKCAQEAexampleEXAMPLEexampleEXAMPLE0123",
    "fakeFAKEfakeFAKEfakeFAKE0123456789abcdefABCDEF+/",
    "AAAAexampleKeyMaterialNotRealAtAll0123456789==",
];

fn pem(kind: &str, with_end: bool) -> Sample {
    let mut lines = vec![pem_line(kind, false)];
    lines.extend(B64.iter().map(|x| x.to_string()));
    if with_end {
        lines.push(pem_line(kind, true));
    }
    multi(lines)
}

fn positives() -> Vec<(Category, Sample)> {
    use Category::*;
    let mut v: Vec<(Category, Sample)> = Vec::new();
    // ---- Email ----
    for t in [
        "user@example.com",
        "Contact: jane.doe@example.com",
        "mail first.last+tag@example.co.uk now",
        "Email: nguyen.van.a@example.vn",
        "a_b-c@sub.example.org",
        "support @ example.com",
        "user @example.com",
        "G\u{1eed}i t\u{1edb}i hr@example.net nh\u{00e9}",
        "<admin@example.com>",
        "(billing@example.io)",
        "To: x.y.z@example.travel;",
        "USER@EXAMPLE.COM",
        "user%ops@example.com",
        "reply-to: no-reply@mail.example.com",
        "test123@example.com, others",
        "mailto:info@example.org",
        "Email c\u{1ee7}a t\u{00f4}i: tuan.demo@example.com",
        "ops@example-domain.com",
        "user@exarnple.com",
        "dev.team@example.co",
        "jane@example.com and bob@example.org",
        "user@example,com",
        "user@example .com",
        "us3r@examp1e.c0m",
    ] {
        v.push((Email, s(t)));
    }
    // ---- Điện thoại ----
    for t in [
        "0912345678",
        "0912 345 678",
        "091.234.5678",
        "091-234-5678",
        "+84 912 345 678",
        "+84912345678",
        "84912345678",
        "+84(0)912345678",
        "(+84) 912 345 678",
        "028 3822 1234",
        "+84 28 3822 1234",
        "+1 415 555 0132",
        "+44 20 7946 0000",
        "Tel: 415 555 0132",
        "S\u{0110}T: 0987 654 321",
        "Hotline 1900 1234 56",
        "Zalo 0356789012",
        "\u{0110}i\u{1ec7}n tho\u{1ea1}i: 0703 456 789",
        "+65 6123 4567",
        "+49 30 901820",
        "mobile: (415) 555-0132",
        "O912345678",
        "+84 9l2 345 678",
        "Call 09I2 345 678",
        "+84(0) 912 345 678",
    ] {
        v.push((Phone, s(t)));
    }
    v.push((
        Phone,
        multi(vec!["Phone:".to_string(), "212 555 0198".to_string()]),
    ));
    // ---- Thẻ ----
    for t in [
        "4111 1111 1111 1111",
        "4012 8888 8888 1881",
        "4242-4242-4242-4242",
        "4000 0566 5566 5556",
        "5555 5555 5555 4444",
        "5105 1051 0510 5100",
        "2223 0031 2200 3222",
        "5200 8282 8282 8210",
        "3782 822463 10005",
        "3714 496353 98431",
        "6011 1111 1111 1117",
        "6011 0009 9013 9424",
        "3530 1113 3330 0000",
        "3566 0020 2036 0505",
        "6200 0000 0000 0005",
        "6205 5000 0000 0000 004",
        "Card: 3056 9300 0902 0004",
        "4111111111111111",
        "S\u{1ed1} th\u{1ebb}: 4242 4242 4242 4242",
        "card number 4000056655665556",
        "4111 1111 1111 l111",
        "4O12 8888 8888 1881",
        "5S55 5555 5555 4444",
        "6011 1111 1111 11I7",
        "Visa 4242 4242 4242 4242 exp 12/30",
    ] {
        v.push((CreditCard, s(t)));
    }
    // ---- Khóa API ----
    let api = [
        k("AK", "IAEXAMPLEKEY012345"),
        k("AS", "IAEXAMPLEKEY012345"),
        format!("AWS_ACCESS_KEY_ID={}", k("AK", "IAEXAMPLE0123456XY")),
        k("AI", "zaSyEXAMPLE_example-0123456789abcdefg"),
        k("gh", "p_EXAMPLEexample0123456789EXAMPLEexamp"),
        k("gh", "o_EXAMPLEexample0123456789EXAMPLEexamp"),
        k("gh", "u_EXAMPLEexample0123456789EXAMPLEexamp"),
        k("gh", "s_EXAMPLEexample0123456789EXAMPLEexamp"),
        k("gh", "r_EXAMPLEexample0123456789EXAMPLEexamp"),
        k("github", "_pat_EXAMPLE0123456789_example"),
        k("glp", "at-EXAMPLEexample012345"),
        k("xo", "xb-1234567890-EXAMPLE"),
        k("xo", "xp-1234567890-1234567890-EXAMPLE"),
        k("sk", "-EXAMPLEexample0123456789"),
        k("sk", "-proj-EXAMPLEexample0123456789"),
        k("sk", "-ant-EXAMPLEexample0123456789"),
        k("sk_", "live_EXAMPLEexample0123"),
        k("sk_", "test_EXAMPLEexample0123"),
        k("rk_", "live_EXAMPLEexample0123"),
        k("pk_", "live_EXAMPLEexample0123"),
        format!(
            "{}{}.{}",
            "SG", ".EXAMPLEexample01234567", "EXAMPLEexample0123456789EXAMPLEexample01234"
        ),
        k("np", "m_EXAMPLEexample0123456789EXAMPLEexamp"),
        "api_key=AbCdEfGh12345678XY".to_string(),
        "\"x-api-key\": \"AbCdEfGh12345678XY\"".to_string(),
        "client_secret: AbCdEfGh12345678XY".to_string(),
        "secret_key = EXAMPLEexample0123".to_string(),
        k("AK", "lAEXAMPLEKEY012345"),
        k("AK", "|AEXAMP1EKEY0I2345"),
        k("gh", "p_EXAMPLEexamp|e0123456789EXAMPLEexamp"),
        k("AK", "1AEXAMPLEKEY012345"),
    ];
    for t in api {
        v.push((ApiKey, so(t)));
    }
    // OCR tách khóa thành hai từ sát nhau (khe 2 px).
    v.push((
        ApiKey,
        Sample {
            lines: vec![format!("{} KEY012345", k("AK", "IAEXAMPLE"))],
            gap: 2,
        },
    ));
    // ---- Token ----
    let hs = "{\"alg\":\"HS256\",\"typ\":\"JWT\"}";
    let rs = "{\"alg\":\"RS256\",\"kid\":\"demo\"}";
    let none_alg = "{\"alg\":\"none\"}";
    let typ = "{\"typ\":\"JWT\",\"alg\":\"ES256\"}";
    let p1 = "{\"sub\":\"demo\",\"name\":\"Example User\"}";
    let p2 = "{\"iss\":\"example.com\",\"exp\":1900000000}";
    let sig = "ZmFrZXNpZ25hdHVyZQ";
    let tokens = [
        jwt(hs, p1, sig),
        jwt(rs, p2, sig),
        jwt(typ, p1, "c2lnbmF0dXJlX2V4YW1wbGU"),
        format!("{}.", jwt(none_alg, p2, "").trim_end_matches('.')),
        format!("Authorization: Bearer {}", jwt(hs, p2, sig)),
        format!("token={}", jwt(rs, p1, sig)),
        format!("\"id_token\": \"{}\"", jwt(hs, p1, sig)),
        format!("Cookie: session={};", jwt(typ, p2, sig)),
        "Bearer abcd1234efgh5678ijkl".to_string(),
        "Authorization: Bearer abcdefghijklmnopqrstuv".to_string(),
        "access_token: EXAMPLEexample0123456789".to_string(),
        "refresh_token=example0123456789abcdef".to_string(),
        "id_token: abcdefghij0123456789".to_string(),
        "token: abcdefghijklmnopqrst".to_string(),
        "\"access_token\":\"EXAMPLE0123456789abcd\"".to_string(),
        "bearer: EXAMPLEexample0123456789".to_string(),
        "Authorization: bearer abcd1234efgh5678ijkl".to_string(),
        format!("jwt {}", jwt(hs, p2, "c2lnbmF0dXJl")),
        // OCR đọc `eyJ` thành `eyj`
        format!("{}{}", "eyj", &jwt(hs, p1, sig)[3..]),
        format!("x {}", jwt(rs, p2, "YWJjZGVmZ2hpams")),
        format!("{} ok", jwt(typ, p1, sig)),
        "Bearer EXAMPLE.example-0123_abcdefgh".to_string(),
        "access token: EXAMPLEexample0123456789".to_string(),
    ];
    for t in tokens {
        v.push((Token, so(t)));
    }
    // ---- Mật khẩu ----
    for t in [
        "password: hunter2",
        "Password = Tr0ub4dor&3",
        "pwd:abc123",
        "passwd: s3cr3t!",
        "pass: letmein99",
        "M\u{1ead}t kh\u{1ea9}u: matkhau123",
        "mat khau: abc@123",
        "mk: 123456a",
        "secret: blue-moon-42",
        "passphrase: correct horse battery staple",
        "PIN: 4821",
        "DB_PASSWORD=Example123",
        "MYSQL_PWD=example",
        "APP_SECRET=xyz987",
        "GITHUB_TOKEN=abc",
        "MASTERKEY=abc",
        "SSHKEY=abc",
        "PGPASSWORD=demo",
        "export API_KEY=abc",
        "\"password\": \"example\"",
        "Password : hunter2",
        "pwd = qwerty",
        "passw0rd: abc",
        "Pass word: hunter2",
        "PASSWORD=/not/a/path?x",
        "SECRETS=abc",
        "APP_SECRETS=abc123",
        "CLIENT_SECRETS_FILE=x",
    ] {
        v.push((Password, s(t)));
    }
    // ---- Khóa riêng PEM ----
    for kind in ["RSA", "EC", "OPENSSH", "ENCRYPTED", "", "DSA"] {
        v.push((PrivateKey, pem(kind, true)));
        v.push((PrivateKey, pem(kind, false)));
    }
    // gạch ngắn / gạch dài do OCR
    for kind in ["RSA", "", "OPENSSH"] {
        let pk = concat!("PRIVATE", " KEY");
        let b = if kind.is_empty() {
            format!("\u{2014}BEGIN {pk}\u{2014}")
        } else {
            format!("--BEGIN {kind} {pk}--")
        };
        let mut lines = vec![b];
        lines.extend(B64.iter().map(|x| x.to_string()));
        v.push((PrivateKey, multi(lines)));
    }
    // OCR đọc `I` thành `l`, `O` thành `0` trong dòng BEGIN
    let pk = concat!("PRIVATE", " KEY");
    for b in [
        format!("-----BEGlN RSA {pk}-----"),
        format!("-----BEGIN RSA PRlVATE {}-----", "KEY"),
        format!("-----BEGIN {pk}-----"),
        format!("-----BEGIN 0PENSSH {pk}-----"),
        format!("----- BEGIN RSA {pk} -----"),
    ] {
        let mut lines = vec![b];
        lines.extend(B64.iter().map(|x| x.to_string()));
        lines.push(pem_line("RSA", true));
        v.push((PrivateKey, multi(lines)));
    }
    v
}

fn negatives() -> Vec<String> {
    let mut v: Vec<String> = [
        // ngày, giờ
        "Meeting 2026-11-22 at 10:30",
        "22/11/2026",
        "11/22/2026 09:15:00 AM",
        "2026.11.22",
        "Ng\u{00e0}y 22 th\u{00e1}ng 11 n\u{0103}m 2026",
        "Updated: Sun, 22 Nov 2026 08:00:00 GMT",
        "Timestamp 2026-11-22T10:30:00Z",
        "14:05:33.123",
        "Duration 01:02:03",
        "Deadline: 30/12/2026 23:59",
        // IP, mạng
        "Server 10.0.0.1 and 192.168.1.20",
        "Gateway 172.16.254.1/24",
        "IPv6 fe80::1ff:fe23:4567:890a",
        "Listening on 0.0.0.0:8080",
        "MAC 00:1A:2B:3C:4D:5E",
        "DNS 8.8.8.8 8.8.4.4",
        "ping 127.0.0.1 -n 4",
        "Port 443 open",
        // phiên bản
        "darkshot 0.5.0 build 1.2.3",
        "Version 10.0.26300.1",
        "rustc 1.89.0 (stable)",
        "node v20.11.1",
        "Windows 11 23H2 build 22631.4460",
        "Chrome/131.0.6778.86",
        "pkg@1.2.3",
        "API v2.1",
        // tiền
        "Total: $1,234.56 or 1.500.000 VND",
        "Gi\u{00e1}: 250.000 \u{0111}",
        "Price 19.99 USD",
        "\u{20ac}1.299,00",
        "Balance 12 345 678 VND",
        "Thanh to\u{00e1}n 1 200 000 \u{0111}",
        "Subtotal 3,450.00",
        "Tax 8% = 276.00",
        // mã băm, UUID, mã
        "commit 3f786850e387550fdab836ed7e6dc881de23001b",
        "sha256 e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "md5 d41d8cd98f00b204e9800998ecf8427e",
        "id 123e4567-e89b-12d3-a456-426614174000",
        "Request-Id: 9b2f1c4e-7a3d-4e8b-b1f0-2c5d6e7f8a9b",
        "git checkout a1b2c3d",
        "Build #4512",
        "Order #2026112212345678",
        "Invoice INV-2026-000123",
        "Tracking 1Z999AA10123456784",
        "ISBN 978-3-16-148410-0",
        "SKU AB-1234-XYZ",
        "Ticket JIRA-4821",
        "Error 0x80070005",
        "HRESULT 0x8000FFFF",
        "Color #1E1E1E",
        "rgb(30, 30, 30)",
        // đường dẫn
        "C:\\Users\\demo\\Pictures\\darkshot",
        "/home/demo/projects/darkshot/src/main.rs",
        "~/Documents/report.pdf",
        "\\\\server\\share\\folder",
        "%LOCALAPPDATA%\\darkshot\\history",
        "https://example.com/path?query=1&page=2",
        "http://localhost:3000/api/v1/users",
        "file:///C:/tmp/a.txt",
        "assets/icon@2x.png",
        "src/privacy/detect/api_key.rs:120",
        // env thường gặp
        "PWD=/home/demo",
        "OLDPWD=/home/demo/src",
        "HOME=/home/demo",
        "PATH=/usr/local/bin:/usr/bin:/bin",
        "SHELL=/bin/bash",
        "LANG=en_US.UTF-8",
        "TERM=xterm-256color",
        "USER=demo",
        "HOSTNAME=devbox",
        "EDITOR=vim",
        "SECRETARY=alice",
        "KEYBOARD=us",
        "MONKEY=banana",
        "HOTKEY=ctrl+shift+s",
        "TURKEY=bird",
        "SECRETSANTA=bob",
        "DONKEY=kong",
        "TOKENIZER=bpe",
        "NODE_ENV=production",
        "RUST_LOG=info",
        "PORT=8080",
        "DEBUG=false",
        "TZ=Asia/Ho_Chi_Minh",
        "PWD=C:\\Users\\demo",
        "PROCESSOR_IDENTIFIER=Intel64 Family 6",
        // giao diện, chữ thường
        "Password",
        "Forgot password?",
        "Enter your password",
        "Show password",
        "password: ****",
        "Password: \u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}",
        "Change password",
        "Password must be at least 8 characters",
        "Qu\u{00ea}n m\u{1ead}t kh\u{1ea9}u?",
        "Nh\u{1ead}p m\u{1ead}t kh\u{1ea9}u",
        "Token expired, please sign in again",
        "API key settings",
        "Generate new token",
        "Secret Santa party",
        "Private key file not found",
        "Use your card to pay",
        "Phone number (optional)",
        "S\u{1ed1} \u{0111}i\u{1ec7}n tho\u{1ea1}i",
        "Email address",
        "thanks @mention and @team",
        "@darkshot please review",
        "Page 1 of 12",
        "Step 3/12",
        "Chapter 4.2.1",
        "The quick brown fox jumps over the lazy dog",
        "Lorem ipsum dolor sit amet, consectetur adipiscing elit",
        "Hello darkshot 123",
        "Local text recognition works on this PC.",
        "Xin ch\u{00e0}o, h\u{00f4}m nay tr\u{1edd}i \u{0111}\u{1eb9}p",
        "Ch\u{1ee5}p m\u{00e0}n h\u{00ec}nh v\u{00e0} l\u{01b0}u PNG",
        "C\u{1ea3}m \u{01a1}n b\u{1ea1}n \u{0111}\u{00e3} d\u{00f9}ng darkshot",
        "Tr\u{1ee3} gi\u{00fa}p",
        "Settings > General > Privacy",
        "File Edit View Help",
        "Save Copy Upload Pin",
        "Ctrl+Shift+S",
        "Score 98.6%",
        "Temperature 36.6 C",
        "Distance 42 km",
        "Resolution 1920x1080",
        "3840 x 2160 @ 60 Hz",
        "Size 2,335,744 bytes",
        "Room 1204, floor 12",
        "Postal code 700000",
        "Population 98 000 000",
        "Year 2026",
        "1 2 3 4 5 6 7 8 9 10",
        "1234567812345670",
        "0000 0000 0000 0000",
        "1111 1111 1111 1111",
        "Batch 1234 5678 9012",
        "Serial 4111 2222",
        "x = 42; y = x * 2",
        "let key = map.get(\"key\");",
        "fn token_count(s: &str) -> usize",
        "if password.is_empty() { return; }",
        "SELECT * FROM users WHERE id = 42",
        "<div class=\"secret-box\">",
        "for i in 0..10 { sum += i; }",
        "config.api_key_name = \"default\"",
        "-----BEGIN CERTIFICATE-----",
        "-----BEGIN PUBLIC KEY-----",
        "ssh-rsa AAAAB3NzaC1yc2E user@host",
        "eyJ is how JSON looks in base64",
        "Bearer token required",
        "Authorization header missing",
        "sk-learn is a library",
        "risk-assessment-2026-report-final-v2",
        "ghp is a prefix",
        "AKIA is an AWS prefix",
        // khó hơn: gần giống dữ liệu nhạy cảm
        "Coordinates 10.762622, 106.660172",
        "Lat 21.028511 Long 105.804817",
        "Barcode 8934567890123",
        "AWS account 123456789012",
        "Bitrate 128 kbps, +3 dB",
        "Delta +12 345 678 units",
        "Tip: the secret: practice daily",
        "Test summary pass: 12 fail: 0",
        "PIN: required",
        "Hash 4111111111111111 (test vector)",
        "Call 1900 1234 for help",
        "Release 2026.11.22.1",
        "Build 20261122.4",
        "Room +84 lobby",
        "user@localhost",
        "name@host",
        "token: none",
        "Bearer: see docs",
        "password=",
        "MY_SECRET_NAME_LABEL=Shown on screen",
    ]
    .iter()
    .map(|x| x.to_string())
    .collect();
    // ngày và giờ sinh tự động
    for d in 1..=12 {
        v.push(format!(
            "{:02}/{:02}/2026 {:02}:{:02}",
            d,
            d + 10,
            d + 5,
            d * 4
        ));
    }
    for d in 1..=8 {
        v.push(format!(
            "2026-{:02}-{:02} build {}.{}.{}",
            d,
            d + 3,
            d,
            d * 2,
            d * 7
        ));
    }
    v
}

/// Đếm mỗi loại: tp, fn, fp (trên mọi mẫu không mong loại đó), fp trên bộ âm tính.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Counts {
    tp: u32,
    fn_: u32,
    fp: u32,
    fp_neg: u32,
}

struct Measured {
    per: [Counts; 7],
    negatives: u32,
    negatives_flagged: u32,
    /// Chỉ số mẫu dương tính bị bỏ sót (trong `positives()`).
    missed: Vec<usize>,
    /// Chỉ số mẫu âm tính bị nhận nhầm (trong `negatives()`).
    flagged: Vec<usize>,
    /// Chỉ số mẫu dương tính có thêm loại khác.
    cross: Vec<usize>,
}

fn measure() -> Measured {
    let mut per = [Counts::default(); 7];
    let mut missed = Vec::new();
    let mut cross = Vec::new();
    for (i, (want, sample)) in positives().iter().enumerate() {
        let got = found(sample);
        let w = cat_index(*want);
        if got[w] {
            per[w].tp += 1;
        } else {
            per[w].fn_ += 1;
            missed.push(i);
        }
        let mut extra = false;
        for (j, &g) in got.iter().enumerate() {
            if g && j != w {
                per[j].fp += 1;
                extra = true;
            }
        }
        if extra {
            cross.push(i);
        }
    }
    let neg = negatives();
    let mut flagged = Vec::new();
    for (i, t) in neg.iter().enumerate() {
        let got = found(&s(t));
        if got.iter().any(|&g| g) {
            flagged.push(i);
        }
        for (j, &g) in got.iter().enumerate() {
            if g {
                per[j].fp += 1;
                per[j].fp_neg += 1;
            }
        }
    }
    Measured {
        per,
        negatives: neg.len() as u32,
        negatives_flagged: flagged.len() as u32,
        missed,
        flagged,
        cross,
    }
}

#[test]
fn corpus_is_large_enough() {
    let pos = positives();
    for c in CATS {
        let n = pos.iter().filter(|(x, _)| *x == c).count();
        assert!(n >= 20, "category {c:?} has {n} samples");
    }
    assert!(negatives().len() >= 150, "negatives {}", negatives().len());
}

/// Số đo hiện tại (Task 133). Mỗi loại: (tp tối thiểu, fp tối đa, fp trên bộ âm tính tối đa).
const FLOOR: [(u32, u32, u32); 7] = [
    (20, 0, 0), // Email: 24 mẫu
    (23, 1, 1), // Phone: 26 mẫu
    (25, 1, 1), // CreditCard: 25 mẫu
    (30, 0, 0), // ApiKey: 31 mẫu
    (22, 0, 0), // Token: 23 mẫu
    (26, 4, 4), // Password: 28 mẫu
    (17, 0, 0), // PrivateKey: 20 mẫu
];
/// Số dòng âm tính (trên 196) có ít nhất một phát hiện.
const MAX_NEGATIVES_FLAGGED: u32 = 6;

#[test]
fn quality_does_not_regress() {
    let m = measure();
    let got: Vec<(u32, u32, u32)> = m.per.iter().map(|c| (c.tp, c.fp, c.fp_neg)).collect();
    for (i, c) in CATS.iter().enumerate() {
        let (tp, fp, fp_neg) = FLOOR[i];
        assert!(
            m.per[i].tp >= tp && m.per[i].fp <= fp && m.per[i].fp_neg <= fp_neg,
            "{c:?}: measured {:?}, floor {:?}; all {got:?}; negatives {}/{}; missed {:?}; flagged {:?}; cross {:?}",
            m.per[i],
            FLOOR[i],
            m.negatives_flagged,
            m.negatives,
            m.missed,
            m.flagged,
            m.cross
        );
    }
    assert!(m.negatives_flagged <= MAX_NEGATIVES_FLAGGED);
}
