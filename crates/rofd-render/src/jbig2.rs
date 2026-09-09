//! Minimal safe wrapper over the system jbig2dec library (Apache-2.0).
//!
//! Only standalone JBIG2 files (`\x97JB2` magic) are decoded, which is how
//! OFD packages embed JBIG2 images; PDF-style embedded symbol streams are
//! out of scope. The first decoded page becomes the image.

use std::ffi::{c_char, c_int, c_uint, c_void};

/// Decoded bi-level image expanded to opaque RGBA8 pixels.
pub(crate) struct DecodedJbig2 {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) rgba: Vec<u8>,
}

#[repr(C)]
struct Jbig2Image {
    width: c_uint,
    height: c_uint,
    stride: c_uint,
    data: *mut u8,
    refcount: c_int,
}

#[repr(C)]
struct Jbig2Ctx {
    _private: [u8; 0],
}

extern "C" {
    fn jbig2_ctx_new_imp(
        allocator: *mut c_void,
        options: c_int,
        global_ctx: *mut c_void,
        error_callback: Option<unsafe extern "C" fn(*mut c_void, *const c_char, c_int, c_uint)>,
        error_callback_data: *mut c_void,
        major: c_int,
        minor: c_int,
    ) -> *mut Jbig2Ctx;
    fn jbig2_ctx_free(ctx: *mut Jbig2Ctx) -> *mut c_void;
    fn jbig2_data_in(ctx: *mut Jbig2Ctx, data: *const u8, size: usize) -> c_int;
    fn jbig2_page_out(ctx: *mut Jbig2Ctx) -> *mut Jbig2Image;
    fn jbig2_release_page(ctx: *mut Jbig2Ctx, image: *mut Jbig2Image);
}

/// jbig2dec's default error handler prints to stderr; keep decoding quiet
/// and surface failures through the return value instead.
unsafe extern "C" fn silent_error_callback(
    _data: *mut c_void,
    _message: *const c_char,
    _severity: c_int,
    _segment: c_uint,
) {
}

/// RAII guard releasing the decoder context exactly once.
struct Context(*mut Jbig2Ctx);

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            jbig2_ctx_free(self.0);
        }
    }
}

/// Decodes one standalone JBIG2 file into opaque RGBA8 pixels.
///
/// Set bits in the packed 1bpp page buffer are black ink, matching
/// jbig2dec's representation.
pub(crate) fn decode_standalone(bytes: &[u8]) -> Result<DecodedJbig2, String> {
    let (major, minor) = jbig2dec_version();
    unsafe {
        let raw = jbig2_ctx_new_imp(
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            Some(silent_error_callback),
            std::ptr::null_mut(),
            major,
            minor,
        );
        if raw.is_null() {
            return Err("jbig2dec context allocation failed".to_owned());
        }
        let ctx = Context(raw);
        if jbig2_data_in(ctx.0, bytes.as_ptr(), bytes.len()) != 0 {
            return Err("jbig2dec rejected the encoded stream".to_owned());
        }
        let page = jbig2_page_out(ctx.0);
        if page.is_null() {
            return Err("jbig2dec produced no decoded page".to_owned());
        }
        let image = &*page;
        let width = image.width;
        let height = image.height;
        let stride = image.stride;
        if width == 0 || height == 0 || stride < width.div_ceil(8) {
            jbig2_release_page(ctx.0, page);
            return Err(format!(
                "jbig2dec produced a degenerate page ({width}x{height}, stride {stride})"
            ));
        }
        let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
        for y in 0..height as usize {
            let row =
                std::slice::from_raw_parts(image.data.add(y * stride as usize), stride as usize);
            for x in 0..width as usize {
                let inked = (row[x / 8] >> (7 - (x % 8))) & 1;
                rgba.extend_from_slice(if inked != 0 {
                    &[0, 0, 0, 255]
                } else {
                    &[255, 255, 255, 255]
                });
            }
        }
        jbig2_release_page(ctx.0, page);
        Ok(DecodedJbig2 {
            width,
            height,
            rgba,
        })
    }
}

/// `jbig2_ctx_new_imp` rejects mismatched version arguments, so the build
/// script exports the version pkg-config resolved.
fn jbig2dec_version() -> (c_int, c_int) {
    let parse = |value: &str| -> c_int {
        value
            .parse()
            .expect("build script exported a numeric jbig2dec version")
    };
    (
        parse(env!("ROFD_JBIG2_VERSION_MAJOR")),
        parse(env!("ROFD_JBIG2_VERSION_MINOR")),
    )
}

#[cfg(test)]
mod tests {
    use super::decode_standalone;

    #[test]
    fn rejects_truncated_streams() {
        assert!(decode_standalone(&[]).is_err());
        assert!(decode_standalone(b"\x97JB2\r\n\x1a\n").is_err());
        assert!(decode_standalone(b"not a jbig2 file at all").is_err());
    }
}
