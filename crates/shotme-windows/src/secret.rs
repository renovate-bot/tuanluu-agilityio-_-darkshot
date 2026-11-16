//! Khóa bí mật được mã hóa bằng DPAPI, gắn với tài khoản Windows hiện tại.

use shotme_core::config::SecretStore;

/// Mã hóa và giải mã bằng `CryptProtectData`/`CryptUnprotectData` (phạm vi người dùng hiện tại).
#[derive(Debug, Clone, Copy, Default)]
pub struct DpapiStore;

impl SecretStore for DpapiStore {
    fn protect(&self, _plain: &[u8]) -> Result<Vec<u8>, String> {
        todo!()
    }

    fn unprotect(&self, _cipher: &[u8]) -> Result<Vec<u8>, String> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Chỉ dùng chuỗi giả; không bao giờ đọc khóa thật hay file cấu hình của người dùng.
    #[test]
    fn dpapi_round_trip() {
        let c = DpapiStore.protect(b"TOP-SECRET-VALUE").unwrap();
        assert!(!c.windows(16).any(|w| w == b"TOP-SECRET-VALUE"));
        assert_eq!(DpapiStore.unprotect(&c).unwrap(), b"TOP-SECRET-VALUE");
    }

    #[test]
    fn dpapi_rejects_garbage() {
        assert!(DpapiStore.unprotect(b"not a dpapi blob").is_err());
    }
}
