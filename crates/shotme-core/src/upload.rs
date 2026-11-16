//! Tải ảnh lên R2 (giao thức S3, chữ ký SigV4).

use std::io::Read;
use std::time::Duration;

use crate::config::{normalize_public_base_url, Expiry, R2Config};
use crate::naming::random_object_name;
use crate::sigv4::{amz_date_now, authorization_header, sha256_hex, Credentials, SignInput};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const GLOBAL_TIMEOUT: Duration = Duration::from_secs(60);
const REGION: &str = "auto";
const SERVICE: &str = "s3";

#[derive(Debug, PartialEq, Eq)]
pub enum UploadError {
    NotConfigured,
    Network(String),
    Auth,
    BucketNotFound,
    Http(u16),
    PublicUrl(String),
}

impl UploadError {
    pub fn message_vi(&self) -> String {
        match self {
            UploadError::NotConfigured => {
                "Chưa cấu hình kho lưu trữ. Hãy nhập đủ thông tin trong cài đặt.".into()
            }
            UploadError::Network(_) => {
                "Không kết nối được tới máy chủ. Kiểm tra mạng rồi thử lại.".into()
            }
            UploadError::Auth => {
                "Khóa truy cập không hợp lệ hoặc không đủ quyền. Kiểm tra lại khóa.".into()
            }
            UploadError::BucketNotFound => {
                "Không tìm thấy kho. Kiểm tra lại tên kho và mã tài khoản.".into()
            }
            UploadError::Http(code) => format!("Máy chủ trả về lỗi {code}."),
            UploadError::PublicUrl(_) => {
                "Không đọc được ảnh qua địa chỉ công khai. Kiểm tra lại địa chỉ công khai của kho."
                    .into()
            }
        }
    }
}

pub trait Uploader: Send + Sync {
    /// Trả link công khai.
    fn upload(&self, key: &str, png: &[u8]) -> Result<String, UploadError>;
}

pub fn object_key(expiry: Expiry, name: &str) -> String {
    format!("{}/{name}.png", expiry.prefix())
}

pub fn public_url(base: &str, key: &str) -> String {
    format!("{}/{key}", base.trim_end_matches('/'))
}

/// Splits "scheme://host[:port]" into (scheme, host header value).
/// Default ports are dropped so the signed `host` equals what is sent.
fn parse_endpoint(endpoint: &str) -> Option<(String, String)> {
    let e = endpoint.trim().trim_end_matches('/');
    let (scheme, rest) = e.split_once("://")?;
    if !(scheme == "http" || scheme == "https")
        || rest.is_empty()
        || rest.contains(['/', '?', '#', '@', ' '])
    {
        return None;
    }
    let host = match (scheme, rest.rsplit_once(':')) {
        ("https", Some((h, "443"))) | ("http", Some((h, "80"))) => h,
        _ => rest,
    };
    Some((scheme.to_string(), host.to_string()))
}

/// Bucket names are placed in the URL path unencoded, so only characters that
/// need no URI encoding are accepted; anything else is `NotConfigured`.
fn bucket_is_safe(b: &str) -> bool {
    !b.is_empty()
        && b.bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'.' | b'_'))
}

/// The account id becomes part of the host name, so only `[A-Za-z0-9-]` is accepted.
fn account_id_is_safe(a: &str) -> bool {
    !a.is_empty() && a.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
}

pub struct R2Uploader {
    agent: ureq::Agent,
    access_key_id: String,
    secret_access_key: String,
    bucket: String,
    complete: bool,
    /// (scheme, host) of the S3 endpoint; `None` when unparsable.
    s3: Option<(String, String)>,
    public_base: String,
}

impl R2Uploader {
    pub fn new(cfg: &R2Config) -> Self {
        let endpoint = format!("https://{}.r2.cloudflarestorage.com", cfg.account_id);
        let public = normalize_public_base_url(&cfg.public_base_url);
        let mut up = Self::with_endpoints(cfg, &endpoint, public.as_deref().unwrap_or(""));
        if public.is_err() {
            up.complete = false;
        }
        up
    }

    pub fn with_endpoints(cfg: &R2Config, s3_endpoint: &str, public_base: &str) -> Self {
        Self::build(cfg, s3_endpoint, public_base, GLOBAL_TIMEOUT)
    }

    #[cfg(test)]
    fn with_global_timeout(
        cfg: &R2Config,
        s3_endpoint: &str,
        public_base: &str,
        global: Duration,
    ) -> Self {
        Self::build(cfg, s3_endpoint, public_base, global)
    }

    fn build(cfg: &R2Config, s3_endpoint: &str, public_base: &str, global: Duration) -> Self {
        // Roots: webpki-roots only (default TlsConfig, no platform verifier).
        // No env proxy, no redirects followed, statuses are not errors.
        let config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .max_redirects(0)
            .max_redirects_will_error(false)
            .proxy(None)
            .timeout_connect(Some(CONNECT_TIMEOUT))
            .timeout_global(Some(global))
            .build();
        R2Uploader {
            agent: ureq::Agent::new_with_config(config),
            access_key_id: cfg.access_key_id.clone(),
            secret_access_key: cfg.secret_access_key.clone(),
            bucket: cfg.bucket.clone(),
            complete: cfg.is_complete()
                && bucket_is_safe(&cfg.bucket)
                && account_id_is_safe(&cfg.account_id),
            s3: parse_endpoint(s3_endpoint),
            public_base: public_base.to_string(),
        }
    }

    /// Sends one signed S3 request; returns the HTTP status.
    fn s3_request(
        &self,
        method: &str,
        key: &str,
        content_type: Option<&str>,
        body: &[u8],
    ) -> Result<u16, UploadError> {
        let (scheme, host) = match (&self.s3, self.complete) {
            (Some(s), true) => s,
            _ => return Err(UploadError::NotConfigured),
        };
        let path = format!("/{}/{key}", self.bucket);
        let payload_hash = sha256_hex(body);
        let amz_date = amz_date_now();
        let mut headers: Vec<(&str, &str)> = vec![
            ("host", host.as_str()),
            ("x-amz-content-sha256", payload_hash.as_str()),
            ("x-amz-date", amz_date.as_str()),
        ];
        if let Some(ct) = content_type {
            headers.push(("content-type", ct));
        }
        let auth = authorization_header(
            &Credentials {
                access_key_id: &self.access_key_id,
                secret_access_key: &self.secret_access_key,
            },
            &SignInput {
                method,
                path: &path,
                query: "",
                headers: &headers,
                payload_sha256_hex: &payload_hash,
                amz_date: &amz_date,
                region: REGION,
                service: SERVICE,
            },
        );
        let url = format!("{scheme}://{host}{path}");
        let result = match method {
            "PUT" => {
                let mut req = self
                    .agent
                    .put(&url)
                    .header("x-amz-content-sha256", &payload_hash)
                    .header("x-amz-date", &amz_date)
                    .header("authorization", &auth);
                if let Some(ct) = content_type {
                    req = req.header("content-type", ct);
                }
                req.send(body)
            }
            _ => self
                .agent
                .delete(&url)
                .header("x-amz-content-sha256", &payload_hash)
                .header("x-amz-date", &amz_date)
                .header("authorization", &auth)
                .call(),
        };
        match result {
            Ok(resp) => Ok(resp.status().as_u16()),
            Err(e) => Err(UploadError::Network(e.to_string())),
        }
    }

    fn check_status(status: u16) -> Result<(), UploadError> {
        match status {
            200..=299 => Ok(()),
            401 | 403 => Err(UploadError::Auth),
            404 => Err(UploadError::BucketNotFound),
            other => Err(UploadError::Http(other)),
        }
    }

    fn fetch_public(&self, key: &str) -> Result<Vec<u8>, UploadError> {
        let url = public_url(&self.public_base, key);
        let mut resp = self
            .agent
            .get(&url)
            .call()
            .map_err(|e| UploadError::PublicUrl(e.to_string()))?;
        let status = resp.status().as_u16();
        if !(200..=299).contains(&status) {
            return Err(UploadError::PublicUrl(format!("HTTP {status}")));
        }
        let mut out = Vec::new();
        resp.body_mut()
            .as_reader()
            .take(1024)
            .read_to_end(&mut out)
            .map_err(|e| UploadError::PublicUrl(e.to_string()))?;
        Ok(out)
    }

    pub fn test_connection(&self) -> Result<(), UploadError> {
        if self.s3.is_none() || !self.complete {
            return Err(UploadError::NotConfigured);
        }
        let key = format!("d1/shotme-test-{}.txt", random_object_name());
        let status = self.s3_request("PUT", &key, Some("text/plain"), b"shotme")?;
        Self::check_status(status)?;

        let got = match self.fetch_public(&key) {
            Ok(body) if body == b"shotme" => Ok(()),
            Ok(_) => Err(UploadError::PublicUrl("nội dung không khớp".to_string())),
            Err(e) => Err(e),
        };
        // Always clean up, even when the public GET failed.
        let del = self
            .s3_request("DELETE", &key, None, b"")
            .and_then(Self::check_status);
        got?;
        del
    }
}

impl Uploader for R2Uploader {
    fn upload(&self, key: &str, png: &[u8]) -> Result<String, UploadError> {
        if !self.complete || self.s3.is_none() {
            return Err(UploadError::NotConfigured);
        }
        let status = self.s3_request("PUT", key, Some("image/png"), png)?;
        Self::check_status(status)?;
        Ok(public_url(&self.public_base, key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sigv4::sha256_hex;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    const SECRET: &str = "TOP-SECRET-VALUE";

    #[derive(Debug, Clone)]
    struct Recorded {
        request_line: String,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    }

    impl Recorded {
        fn header(&self, name: &str) -> Option<&str> {
            self.headers
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.as_str())
        }
    }

    struct Mock {
        addr: String,
        log: Arc<Mutex<Vec<Recorded>>>,
    }

    impl Mock {
        fn url(&self) -> String {
            format!("http://{}", self.addr)
        }
        fn requests(&self) -> Vec<Recorded> {
            self.log.lock().unwrap().clone()
        }
    }

    fn read_request(s: &mut TcpStream) -> Option<Recorded> {
        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];
        let head_end = loop {
            if let Some(p) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                break p;
            }
            let n = s.read(&mut chunk).ok()?;
            if n == 0 {
                return None;
            }
            buf.extend_from_slice(&chunk[..n]);
        };
        let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
        let mut lines = head.split("\r\n");
        let request_line = lines.next()?.to_string();
        let headers: Vec<(String, String)> = lines
            .filter_map(|l| l.split_once(':'))
            .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
            .collect();
        let len: usize = headers
            .iter()
            .find(|(k, _)| k == "content-length")
            .and_then(|(_, v)| v.parse().ok())
            .unwrap_or(0);
        let mut body = buf[head_end + 4..].to_vec();
        while body.len() < len {
            let n = s.read(&mut chunk).ok()?;
            if n == 0 {
                break;
            }
            body.extend_from_slice(&chunk[..n]);
        }
        Some(Recorded {
            request_line,
            headers,
            body,
        })
    }

    /// Mock server: replies with `responses` in order (200 empty when exhausted).
    /// If `stall` is set it accepts connections and never answers.
    fn spawn_mock(responses: Vec<(u16, &'static str)>, stall: bool) -> Mock {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let log = Arc::new(Mutex::new(Vec::new()));
        let log2 = log.clone();
        let queue = Arc::new(Mutex::new(std::collections::VecDeque::from(responses)));
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut s) = stream else { continue };
                if stall {
                    std::thread::spawn(move || {
                        let _s = s;
                        std::thread::sleep(Duration::from_secs(20));
                    });
                    continue;
                }
                let Some(rec) = read_request(&mut s) else {
                    continue;
                };
                log2.lock().unwrap().push(rec);
                let (code, body) = queue.lock().unwrap().pop_front().unwrap_or((200, ""));
                let resp = format!(
                    "HTTP/1.1 {code} X\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = s.write_all(resp.as_bytes());
            }
        });
        Mock { addr, log }
    }

    fn cfg() -> R2Config {
        R2Config {
            account_id: "acc".into(),
            access_key_id: "kid".into(),
            secret_access_key: SECRET.into(),
            bucket: "bkt".into(),
            public_base_url: "https://img.example.com".into(),
        }
    }

    #[test]
    fn key_and_url() {
        assert_eq!(object_key(Expiry::D30, "abc"), "d30/abc.png");
        assert_eq!(object_key(Expiry::Keep, "abc"), "keep/abc.png");
        assert_eq!(
            public_url("https://img.example.com", "d7/abc.png"),
            "https://img.example.com/d7/abc.png"
        );
    }

    #[test]
    fn upload_sends_signed_put_and_returns_public_url() {
        let m = spawn_mock(vec![(200, "")], false);
        let up = R2Uploader::with_endpoints(&cfg(), &m.url(), "https://img.example.com");
        let png = b"\x89PNG fake bytes".to_vec();
        let url = up.upload("d30/abc.png", &png).unwrap();
        assert_eq!(url, "https://img.example.com/d30/abc.png");
        let reqs = m.requests();
        assert_eq!(reqs.len(), 1);
        let r = &reqs[0];
        assert_eq!(r.request_line, "PUT /bkt/d30/abc.png HTTP/1.1");
        assert_eq!(r.body, png);
        assert_eq!(r.header("content-type"), Some("image/png"));
        assert!(r
            .header("authorization")
            .unwrap()
            .starts_with("AWS4-HMAC-SHA256 Credential=kid/"));
        assert_eq!(
            r.header("x-amz-content-sha256"),
            Some(sha256_hex(&png).as_str())
        );
        assert_eq!(r.header("host"), Some(m.addr.as_str()));
        assert!(r.header("x-amz-date").is_some());
    }

    #[test]
    fn status_mapping() {
        let cases = [
            (403, UploadError::Auth),
            (401, UploadError::Auth),
            (404, UploadError::BucketNotFound),
            (500, UploadError::Http(500)),
        ];
        for (code, expected) in cases {
            let m = spawn_mock(vec![(code, "")], false);
            let up = R2Uploader::with_endpoints(&cfg(), &m.url(), "https://img.example.com");
            assert_eq!(up.upload("d30/a.png", b"x"), Err(expected), "status {code}");
        }
    }

    #[test]
    fn incomplete_config_sends_nothing() {
        let m = spawn_mock(vec![], false);
        let mut c = cfg();
        c.bucket.clear();
        let up = R2Uploader::with_endpoints(&c, &m.url(), "https://img.example.com");
        assert_eq!(
            up.upload("d30/a.png", b"x"),
            Err(UploadError::NotConfigured)
        );
        assert_eq!(up.test_connection(), Err(UploadError::NotConfigured));
        std::thread::sleep(Duration::from_millis(100));
        assert!(m.requests().is_empty());
    }

    #[test]
    fn connection_refused_is_network_error() {
        let port = {
            let l = TcpListener::bind("127.0.0.1:0").unwrap();
            l.local_addr().unwrap().port()
        };
        let up = R2Uploader::with_endpoints(
            &cfg(),
            &format!("http://127.0.0.1:{port}"),
            "https://img.example.com",
        );
        assert!(matches!(
            up.upload("d30/a.png", b"x"),
            Err(UploadError::Network(_))
        ));
    }

    #[test]
    fn stalled_server_times_out() {
        let m = spawn_mock(vec![], true);
        let up = R2Uploader::with_global_timeout(
            &cfg(),
            &m.url(),
            "https://img.example.com",
            Duration::from_secs(1),
        );
        let t = Instant::now();
        let r = up.upload("d30/a.png", b"x");
        assert!(matches!(r, Err(UploadError::Network(_))), "{r:?}");
        assert!(t.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn test_connection_put_get_delete() {
        let m = spawn_mock(vec![(200, ""), (200, "shotme"), (204, "")], false);
        let up = R2Uploader::with_endpoints(&cfg(), &m.url(), &m.url());
        up.test_connection().unwrap();
        let reqs = m.requests();
        assert_eq!(reqs.len(), 3);
        let path = |r: &Recorded| r.request_line.split(' ').nth(1).unwrap().to_string();
        assert!(reqs[0].request_line.starts_with("PUT /bkt/d1/shotme-test-"));
        assert_eq!(reqs[0].body, b"shotme");
        assert!(reqs[1].request_line.starts_with("GET /d1/shotme-test-"));
        assert!(reqs[2]
            .request_line
            .starts_with("DELETE /bkt/d1/shotme-test-"));
        let key = path(&reqs[0]).trim_start_matches("/bkt/").to_string();
        assert_eq!(path(&reqs[1]), format!("/{key}"));
        assert_eq!(path(&reqs[2]), format!("/bkt/{key}"));
        assert!(reqs[2].header("authorization").is_some());
    }

    #[test]
    fn test_connection_reports_public_url_problem_and_still_deletes() {
        let m = spawn_mock(vec![(200, ""), (404, ""), (204, "")], false);
        let up = R2Uploader::with_endpoints(&cfg(), &m.url(), &m.url());
        assert!(matches!(
            up.test_connection(),
            Err(UploadError::PublicUrl(_))
        ));
        let reqs = m.requests();
        assert_eq!(reqs.len(), 3);
        assert!(reqs[2].request_line.starts_with("DELETE "));
    }

    #[test]
    fn bad_account_id_is_not_configured() {
        for id in ["a/b", "a@evil.com", "a:1", "a\\b", "a.b", ""] {
            let mut c = cfg();
            c.account_id = id.into();
            let up = R2Uploader::new(&c);
            assert_eq!(
                up.upload("d30/x.png", b"x"),
                Err(UploadError::NotConfigured),
                "account_id {id:?}"
            );
            assert_eq!(up.test_connection(), Err(UploadError::NotConfigured));
        }
    }

    #[test]
    fn http_public_base_is_not_configured_in_new() {
        let mut c = cfg();
        c.public_base_url = "http://img.example.com".into();
        let up = R2Uploader::new(&c);
        assert_eq!(
            up.upload("d30/x.png", b"x"),
            Err(UploadError::NotConfigured)
        );
        assert_eq!(up.test_connection(), Err(UploadError::NotConfigured));
    }

    #[test]
    fn error_text_never_contains_secret() {
        let errs = [
            UploadError::NotConfigured,
            UploadError::Network("connection refused".into()),
            UploadError::Auth,
            UploadError::BucketNotFound,
            UploadError::Http(500),
            UploadError::PublicUrl("404".into()),
        ];
        for e in errs {
            assert!(!e.message_vi().contains(SECRET));
            assert!(!format!("{e:?}").contains(SECRET));
            assert!(!e.message_vi().is_empty());
        }
    }
}
