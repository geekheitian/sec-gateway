use std::sync::Arc;

pub trait FpeBackend: Send + Sync {
    fn encrypt(&self, plaintext: &str, tweak: &[u8]) -> Result<String, String>;
    fn decrypt(&self, ciphertext: &str, tweak: &[u8]) -> Result<String, String>;
    fn backend_name(&self) -> &'static str;
}

pub type DynFpeBackend = Arc<dyn FpeBackend>;

pub fn create_aes_backend(key: &[u8; 32], radix: u32) -> Result<DynFpeBackend, String> {
    use super::fpe::FPECipher;
    let cipher = FPECipher::new(key, radix)?;
    Ok(Arc::new(cipher))
}

pub fn create_sm4_backend(key: &[u8; 16], radix: u32) -> Result<DynFpeBackend, String> {
    use super::sm4_fpe::Sm4FpeCipher;
    let cipher = Sm4FpeCipher::new(key, radix)?;
    Ok(Arc::new(cipher))
}
