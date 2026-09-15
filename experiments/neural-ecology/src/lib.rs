pub mod backend;
pub mod model;
pub mod runner;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub fn digest(bytes: &[u8]) -> String {
    #[cfg(target_os = "macos")]
    if let Ok(len) = u32::try_from(bytes.len()) {
        #[link(name = "System")]
        extern "C" {
            fn CC_SHA256(data: *const std::ffi::c_void, len: u32, md: *mut u8) -> *mut u8;
        }
        let mut out = [0u8; 32];
        // SAFETY: CommonCrypto's one-shot SHA-256 ABI is declared in the local
        // SDK's CommonDigest.h; input lifetime and 32-byte output are valid.
        let result = unsafe { CC_SHA256(bytes.as_ptr().cast(), len, out.as_mut_ptr()) };
        assert!(!result.is_null());
        return out.iter().map(|b| format!("{b:02x}")).collect();
    }
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}
