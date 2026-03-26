use libc::{c_char, c_int, c_uint, size_t, uint8_t};

#[repr(C)]
pub struct Sm4Context {
    _private: [u8; 0],
}

#[repr(C)]
pub struct FpeContext {
    _private: [u8; 0],
}

#[link(name = "wbcrypto", kind = "static")]
extern "C" {
    pub fn WBCRYPTO_sm4_context_init() -> *mut Sm4Context;

    pub fn WBCRYPTO_sm4_init_key(
        ctx: *mut Sm4Context,
        key: *const uint8_t,
        keylen: size_t,
    ) -> c_int;

    pub fn WBCRYPTO_sm4_context_free(ctx: *mut Sm4Context);

    pub fn WBCRYPTO_sm4_fpe_init(
        key: *mut Sm4Context,
        twkbuf: *const c_char,
        twklen: size_t,
        radix: c_uint,
    ) -> *mut FpeContext;

    pub fn WBCRYPTO_fpe_free(ctx: *mut FpeContext);

    pub fn WBCRYPTO_ff1_encrypt(
        ctx: *mut FpeContext,
        input: *const c_char,
        output: *mut c_char,
    ) -> c_int;

    pub fn WBCRYPTO_ff1_decrypt(
        ctx: *mut FpeContext,
        input: *const c_char,
        output: *mut c_char,
    ) -> c_int;
}
