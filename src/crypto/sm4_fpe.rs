use super::sm4_fpe_sys::*;
use std::ffi::{CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::sync::Mutex;

const SM4_KEY_SIZE: usize = 16;
const MIN_PLAINTEXT_LENGTH: usize = 6;

struct Sm4ContextHandle(*mut Sm4Context);

impl Drop for Sm4ContextHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                WBCRYPTO_sm4_context_free(self.0);
            }
        }
    }
}

unsafe impl Send for Sm4ContextHandle {}
unsafe impl Sync for Sm4ContextHandle {}

struct FpeContextHandle(*mut FpeContext);

impl Drop for FpeContextHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                WBCRYPTO_fpe_free(self.0);
            }
        }
    }
}

unsafe impl Send for FpeContextHandle {}
unsafe impl Sync for FpeContextHandle {}

pub struct Sm4FpeCipher {
    sm4: Mutex<Sm4ContextHandle>,
    radix: u32,
}

impl Sm4FpeCipher {
    pub fn new(key: &[u8; SM4_KEY_SIZE], radix: u32) -> Result<Self, String> {
        if radix < 2 || radix > 36 {
            return Err(format!(
                "Invalid radix: {}. Must be between 2 and 36",
                radix
            ));
        }

        unsafe {
            let sm4_ctx = WBCRYPTO_sm4_context_init();
            if sm4_ctx.is_null() {
                return Err("Failed to initialize SM4 context".to_string());
            }

            let sm4_handle = Sm4ContextHandle(sm4_ctx);

            let ret = WBCRYPTO_sm4_init_key(sm4_ctx, key.as_ptr(), SM4_KEY_SIZE as libc::size_t);
            if ret != 1 {
                return Err(format!("Failed to initialize SM4 key, error code: {}", ret));
            }

            Ok(Sm4FpeCipher {
                sm4: Mutex::new(sm4_handle),
                radix,
            })
        }
    }

    pub fn encrypt(&self, plaintext: &str, tweak: &[u8]) -> Result<String, String> {
        if plaintext.len() < MIN_PLAINTEXT_LENGTH {
            return Err(format!(
                "Plaintext too short: {} chars (minimum {})",
                plaintext.len(),
                MIN_PLAINTEXT_LENGTH
            ));
        }

        if !plaintext.is_ascii() {
            return Err("Plaintext must contain only ASCII characters".to_string());
        }

        if !self.validate_alphabet(plaintext) {
            return Err(format!(
                "Plaintext contains characters outside radix {} alphabet",
                self.radix
            ));
        }

        let result = catch_unwind(AssertUnwindSafe(|| self.encrypt_internal(plaintext, tweak)));

        match result {
            Ok(Ok(ciphertext)) => Ok(ciphertext),
            Ok(Err(e)) => Err(e),
            Err(_) => Err("FFI call panicked during encryption".to_string()),
        }
    }

    pub fn decrypt(&self, ciphertext: &str, tweak: &[u8]) -> Result<String, String> {
        if ciphertext.len() < MIN_PLAINTEXT_LENGTH {
            return Err(format!(
                "Ciphertext too short: {} chars (minimum {})",
                ciphertext.len(),
                MIN_PLAINTEXT_LENGTH
            ));
        }

        if !ciphertext.is_ascii() {
            return Err("Ciphertext must contain only ASCII characters".to_string());
        }

        if !self.validate_alphabet(ciphertext) {
            return Err(format!(
                "Ciphertext contains characters outside radix {} alphabet",
                self.radix
            ));
        }

        let result = catch_unwind(AssertUnwindSafe(|| {
            self.decrypt_internal(ciphertext, tweak)
        }));

        match result {
            Ok(Ok(plaintext)) => Ok(plaintext),
            Ok(Err(e)) => Err(e),
            Err(_) => Err("FFI call panicked during decryption".to_string()),
        }
    }

    fn encrypt_internal(&self, plaintext: &str, tweak: &[u8]) -> Result<String, String> {
        let input =
            CString::new(plaintext).map_err(|e| format!("Invalid plaintext string: {}", e))?;

        let output_len = plaintext.len() + 1;
        let mut output = vec![0u8; output_len];

        let sm4_handle = self.sm4.lock().unwrap();

        unsafe {
            let tweak_cstr = if tweak.is_empty() {
                ptr::null()
            } else {
                tweak.as_ptr() as *const libc::c_char
            };

            let fpe_ctx = WBCRYPTO_sm4_fpe_init(
                sm4_handle.0,
                tweak_cstr,
                tweak.len() as libc::size_t,
                self.radix as libc::c_uint,
            );

            if fpe_ctx.is_null() {
                return Err("Failed to initialize FPE context".to_string());
            }

            let fpe_handle = FpeContextHandle(fpe_ctx);

            let ret = WBCRYPTO_ff1_encrypt(
                fpe_handle.0,
                input.as_ptr(),
                output.as_mut_ptr() as *mut libc::c_char,
            );

            if ret != 1 {
                return Err(format!("SM4-FF1 encryption failed, error code: {}", ret));
            }

            let result_cstr = CStr::from_ptr(output.as_ptr() as *const libc::c_char);
            result_cstr
                .to_str()
                .map(|s| s.to_string())
                .map_err(|e| format!("Invalid UTF-8 in encrypted output: {}", e))
        }
    }

    fn decrypt_internal(&self, ciphertext: &str, tweak: &[u8]) -> Result<String, String> {
        let input =
            CString::new(ciphertext).map_err(|e| format!("Invalid ciphertext string: {}", e))?;

        let output_len = ciphertext.len() + 1;
        let mut output = vec![0u8; output_len];

        let sm4_handle = self.sm4.lock().unwrap();

        unsafe {
            let tweak_cstr = if tweak.is_empty() {
                ptr::null()
            } else {
                tweak.as_ptr() as *const libc::c_char
            };

            let fpe_ctx = WBCRYPTO_sm4_fpe_init(
                sm4_handle.0,
                tweak_cstr,
                tweak.len() as libc::size_t,
                self.radix as libc::c_uint,
            );

            if fpe_ctx.is_null() {
                return Err("Failed to initialize FPE context".to_string());
            }

            let fpe_handle = FpeContextHandle(fpe_ctx);

            let ret = WBCRYPTO_ff1_decrypt(
                fpe_handle.0,
                input.as_ptr(),
                output.as_mut_ptr() as *mut libc::c_char,
            );

            if ret != 1 {
                return Err(format!("SM4-FF1 decryption failed, error code: {}", ret));
            }

            let result_cstr = CStr::from_ptr(output.as_ptr() as *const libc::c_char);
            result_cstr
                .to_str()
                .map(|s| s.to_string())
                .map_err(|e| format!("Invalid UTF-8 in decrypted output: {}", e))
        }
    }

    fn validate_alphabet(&self, text: &str) -> bool {
        let valid_chars = self.get_alphabet_chars();
        text.chars().all(|c| valid_chars.contains(&c))
    }

    fn get_alphabet_chars(&self) -> Vec<char> {
        match self.radix {
            10 => ('0'..='9').collect(),
            16 => {
                let mut chars: Vec<char> = ('0'..='9').collect();
                chars.extend(('a'..='f').chain('A'..='F'));
                chars
            }
            36 => {
                let mut chars: Vec<char> = ('0'..='9').collect();
                chars.extend(('a'..='z').chain('A'..='Z'));
                chars
            }
            r if r <= 10 => (0..r).map(|i| char::from_digit(i, r).unwrap()).collect(),
            _ => (0..self.radix)
                .filter_map(|i| char::from_digit(i, self.radix))
                .collect(),
        }
    }
}

unsafe impl Send for Sm4FpeCipher {}
unsafe impl Sync for Sm4FpeCipher {}

impl super::fpe_trait::FpeBackend for Sm4FpeCipher {
    fn encrypt(&self, plaintext: &str, tweak: &[u8]) -> Result<String, String> {
        self.encrypt(plaintext, tweak)
    }

    fn decrypt(&self, ciphertext: &str, tweak: &[u8]) -> Result<String, String> {
        self.decrypt(ciphertext, tweak)
    }

    fn backend_name(&self) -> &'static str {
        "SM4-FF1"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sm4_fpe_cipher_creation() {
        let key = [0u8; 16];
        let cipher = Sm4FpeCipher::new(&key, 10);
        assert!(
            cipher.is_ok(),
            "Failed to create SM4 FPE cipher: {:?}",
            cipher.err()
        );
    }

    #[test]
    fn test_invalid_radix() {
        let key = [0u8; 16];

        let cipher = Sm4FpeCipher::new(&key, 1);
        assert!(cipher.is_err());

        let cipher = Sm4FpeCipher::new(&key, 37);
        assert!(cipher.is_err());
    }
}
