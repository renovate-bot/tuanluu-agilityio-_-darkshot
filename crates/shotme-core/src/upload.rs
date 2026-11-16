//! Tải ảnh lên R2 (giao thức S3, chữ ký SigV4).

use std::time::Duration;

use crate::config::{Expiry, R2Config};

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
        todo!()
    }
}

pub trait Uploader: Send + Sync {
    /// Trả link công khai.
    fn upload(&self, key: &str, png: &[u8]) -> Result<String, UploadError>;
}

pub fn object_key(_expiry: Expiry, _name: &str) -> String {
    todo!()
}

pub fn public_url(_base: &str, _key: &str) -> String {
    todo!()
}

pub struct R2Uploader {
    _unused: (),
}

impl R2Uploader {
    pub fn new(_cfg: &R2Config) -> Self {
        todo!()
    }

    pub fn with_endpoints(_cfg: &R2Config, _s3_endpoint: &str, _public_base: &str) -> Self {
        todo!()
    }

    #[cfg(test)]
    fn with_global_timeout(
        _cfg: &R2Config,
        _s3_endpoint: &str,
        _public_base: &str,
        _global: Duration,
    ) -> Self {
        todo!()
    }

    pub fn test_connection(&self) -> Result<(), UploadError> {
        todo!()
    }
}

impl Uploader for R2Uploader {
    fn upload(&self, _key: &str, _png: &[u8]) -> Result<String, UploadError> {
        todo!()
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
