//! AWS Signature Version 4 (header-based, payload in a single chunk).

use std::fmt;

/// Access key pair. `Debug` never prints the secret.
pub struct Credentials<'a> {
    pub access_key_id: &'a str,
    pub secret_access_key: &'a str,
}

impl fmt::Debug for Credentials<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Credentials")
            .field("access_key_id", &self.access_key_id)
            .field("secret_access_key", &"<redacted>")
            .finish()
    }
}

#[derive(Debug)]
pub struct SignInput<'a> {
    pub method: &'a str,
    /// Already URI-encoded, starts with '/'.
    pub path: &'a str,
    /// Canonical query string, "" if none.
    pub query: &'a str,
    /// Every signed header, including host, x-amz-date, x-amz-content-sha256.
    pub headers: &'a [(&'a str, &'a str)],
    pub payload_sha256_hex: &'a str,
    /// "YYYYMMDDTHHMMSSZ"
    pub amz_date: &'a str,
    pub region: &'a str,
    pub service: &'a str,
}

pub fn sha256_hex(data: &[u8]) -> String {
    let _ = data;
    todo!()
}

pub fn authorization_header(c: &Credentials, i: &SignInput) -> String {
    let _ = (c, i);
    todo!()
}

/// Current UTC time as "YYYYMMDDTHHMMSSZ", from `SystemTime`.
pub fn amz_date_now() -> String {
    todo!()
}

/// Unix seconds (UTC) to "YYYYMMDDTHHMMSSZ".
fn amz_date_from_unix(secs: u64) -> String {
    let _ = secs;
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Public AWS sample keys.
    const AKID: &str = "AKIAIOSFODNN7EXAMPLE";
    const SECRET: &str = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
    const EMPTY_SHA: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    fn creds() -> Credentials<'static> {
        Credentials {
            access_key_id: AKID,
            secret_access_key: SECRET,
        }
    }

    /// Splits "AWS4-HMAC-SHA256 Credential=.., SignedHeaders=.., Signature=.."
    /// into its three parts, tolerant of spaces after the commas.
    fn parts(h: &str) -> (String, String, String) {
        let rest = h
            .strip_prefix("AWS4-HMAC-SHA256 ")
            .expect("algorithm prefix");
        let p: Vec<&str> = rest.split(',').map(str::trim).collect();
        assert_eq!(p.len(), 3, "three comma-separated parts: {h}");
        (
            p[0].strip_prefix("Credential=").expect("Credential").to_string(),
            p[1].strip_prefix("SignedHeaders=").expect("SignedHeaders").to_string(),
            p[2].strip_prefix("Signature=").expect("Signature").to_string(),
        )
    }

    // Nguon: vi du GET Object trong tai lieu AWS S3 SigV4, da doi chieu bang mot hien thuc doc lap.
    #[test]
    fn aws_doc_example_get_object() {
        let headers = [
            ("host", "examplebucket.s3.amazonaws.com"),
            ("range", "bytes=0-9"),
            ("x-amz-content-sha256", EMPTY_SHA),
            ("x-amz-date", "20130524T000000Z"),
        ];
        let input = SignInput {
            method: "GET",
            path: "/test.txt",
            query: "",
            headers: &headers,
            payload_sha256_hex: EMPTY_SHA,
            amz_date: "20130524T000000Z",
            region: "us-east-1",
            service: "s3",
        };
        let (cred, signed, sig) = parts(&authorization_header(&creds(), &input));
        assert_eq!(cred, "AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request");
        assert_eq!(signed, "host;range;x-amz-content-sha256;x-amz-date");
        assert_eq!(
            sig,
            "f0e8bdb87c964420e857bd35b5d6ed310bd44f0170aba48dd91039c6036bdb41"
        );
    }

    // Nguon: vi du PUT Object trong tai lieu AWS S3 SigV4, da doi chieu bang mot hien thuc doc lap.
    #[test]
    fn aws_doc_example_put_object() {
        let payload_sha = sha256_hex(b"Welcome to Amazon S3.");
        assert_eq!(
            payload_sha,
            "44ce7dd67c959e0d3524ffac1771dfbba87d2b6b4b4e99e42034a8b803f8b072"
        );
        let headers = [
            ("date", "Fri, 24 May 2013 00:00:00 GMT"),
            ("host", "examplebucket.s3.amazonaws.com"),
            ("x-amz-content-sha256", payload_sha.as_str()),
            ("x-amz-date", "20130524T000000Z"),
            ("x-amz-storage-class", "REDUCED_REDUNDANCY"),
        ];
        let input = SignInput {
            method: "PUT",
            path: "/test%24file.text",
            query: "",
            headers: &headers,
            payload_sha256_hex: &payload_sha,
            amz_date: "20130524T000000Z",
            region: "us-east-1",
            service: "s3",
        };
        let (cred, signed, sig) = parts(&authorization_header(&creds(), &input));
        assert_eq!(cred, "AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request");
        assert_eq!(
            signed,
            "date;host;x-amz-content-sha256;x-amz-date;x-amz-storage-class"
        );
        assert_eq!(
            sig,
            "98ad721746da40c64f1a55b78f14c238d841ea1380cd77a1b5971af0ece108bd"
        );
    }

    #[test]
    fn sha256_of_empty() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn headers_sorted_and_lowercased() {
        let sorted = [
            ("host", "examplebucket.s3.amazonaws.com"),
            ("range", "bytes=0-9"),
            ("x-amz-content-sha256", EMPTY_SHA),
            ("x-amz-date", "20130524T000000Z"),
        ];
        let shuffled = [
            ("X-Amz-Date", "  20130524T000000Z "),
            ("Range", "bytes=0-9"),
            ("HOST", " examplebucket.s3.amazonaws.com"),
            ("X-Amz-Content-SHA256", EMPTY_SHA),
        ];
        fn mk<'a>(h: &'a [(&'a str, &'a str)]) -> SignInput<'a> {
            SignInput {
                method: "GET",
                path: "/test.txt",
                query: "",
                headers: h,
                payload_sha256_hex: EMPTY_SHA,
                amz_date: "20130524T000000Z",
                region: "us-east-1",
                service: "s3",
            }
        }
        let a = authorization_header(&creds(), &mk(&sorted));
        let b = authorization_header(&creds(), &mk(&shuffled));
        assert_eq!(a, b);
        assert_eq!(parts(&a).1, "host;range;x-amz-content-sha256;x-amz-date");
    }

    #[test]
    fn amz_date_shape() {
        let d = amz_date_now();
        assert!(d.len() == 16 && d.as_bytes()[8] == b'T' && d.ends_with('Z'));
    }

    #[test]
    fn civil_date_known_values() {
        assert_eq!(amz_date_from_unix(0), "19700101T000000Z");
        assert_eq!(amz_date_from_unix(951_782_400), "20000229T000000Z");
        assert_eq!(amz_date_from_unix(1_369_353_600), "20130524T000000Z");
    }

    #[test]
    fn debug_hides_secret() {
        let s = format!("{:?}", creds());
        assert!(!s.contains(SECRET));
        assert!(s.contains(AKID));
    }
}
