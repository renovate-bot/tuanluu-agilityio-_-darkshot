//! Khóa bí mật được mã hóa bằng DPAPI, gắn với tài khoản Windows hiện tại.

use shotme_core::config::SecretStore;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{LocalFree, HLOCAL};
use windows::Win32::Security::Cryptography::{
    CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
};

/// Mã hóa và giải mã bằng `CryptProtectData`/`CryptUnprotectData` (phạm vi người dùng hiện tại).
/// Không dùng `CRYPTPROTECT_LOCAL_MACHINE`; luôn có `CRYPTPROTECT_UI_FORBIDDEN`.
#[derive(Debug, Clone, Copy, Default)]
pub struct DpapiStore;

impl SecretStore for DpapiStore {
    fn protect(&self, plain: &[u8]) -> Result<Vec<u8>, String> {
        let input = blob_in(plain)?;
        let mut out = OutBlob::new(false);
        // SAFETY: `input` trỏ vào `plain` còn sống suốt lời gọi, hàm chỉ đọc nó; `out` nhận
        // vùng nhớ do DPAPI cấp và `OutBlob` giải phóng bằng LocalFree trên mọi đường đi.
        unsafe {
            CryptProtectData(
                &input,
                PCWSTR::null(),
                None,
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out.blob,
            )
        }
        .map_err(|e| {
            format!(
                "Không mã hóa được khóa bí mật (mã lỗi {:#010x})",
                e.code().0
            )
        })?;
        Ok(out.to_vec())
    }

    fn unprotect(&self, cipher: &[u8]) -> Result<Vec<u8>, String> {
        let input = blob_in(cipher)?;
        // Bản rõ do DPAPI cấp được xóa trắng trước khi LocalFree.
        let mut out = OutBlob::new(true);
        // SAFETY: như `protect`; `input` chỉ được đọc, `out` do `OutBlob` giải phóng.
        unsafe {
            CryptUnprotectData(
                &input,
                None,
                None,
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out.blob,
            )
        }
        .map_err(|e| {
            format!(
                "Không giải mã được khóa bí mật (mã lỗi {:#010x})",
                e.code().0
            )
        })?;
        Ok(out.to_vec())
    }
}

/// Khối vào cho DPAPI, trỏ thẳng vào dữ liệu của bên gọi (không sao chép).
fn blob_in(data: &[u8]) -> Result<CRYPT_INTEGER_BLOB, String> {
    let len = u32::try_from(data.len()).map_err(|_| "Dữ liệu quá lớn để mã hóa".to_string())?;
    Ok(CRYPT_INTEGER_BLOB {
        cbData: len,
        // DPAPI không ghi vào khối vào; ép kiểu chỉ để khớp chữ ký C.
        pbData: data.as_ptr() as *mut u8,
    })
}

/// Khối ra do DPAPI cấp phát; luôn được `LocalFree` khi drop.
struct OutBlob {
    blob: CRYPT_INTEGER_BLOB,
    wipe: bool,
}

impl OutBlob {
    fn new(wipe: bool) -> Self {
        OutBlob {
            blob: CRYPT_INTEGER_BLOB {
                cbData: 0,
                pbData: std::ptr::null_mut(),
            },
            wipe,
        }
    }

    fn to_vec(&self) -> Vec<u8> {
        if self.blob.pbData.is_null() || self.blob.cbData == 0 {
            return Vec::new();
        }
        // SAFETY: DPAPI trả về `cbData` byte hợp lệ tại `pbData`, còn sống đến khi drop.
        unsafe { std::slice::from_raw_parts(self.blob.pbData, self.blob.cbData as usize) }.to_vec()
    }
}

impl Drop for OutBlob {
    fn drop(&mut self) {
        if self.blob.pbData.is_null() {
            return;
        }
        if self.wipe {
            for i in 0..self.blob.cbData as usize {
                // SAFETY: i < cbData nằm trong vùng DPAPI cấp; ghi volatile để không bị tối ưu bỏ.
                unsafe { std::ptr::write_volatile(self.blob.pbData.add(i), 0) };
            }
        }
        // SAFETY: `pbData` do DPAPI cấp bằng LocalAlloc và chỉ được giải phóng một lần ở đây.
        unsafe {
            let _ = LocalFree(Some(HLOCAL(self.blob.pbData.cast())));
        }
        self.blob.pbData = std::ptr::null_mut();
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
