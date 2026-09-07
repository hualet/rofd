fn main() {
    println!("cargo:rustc-cdylib-link-arg=-Wl,-soname,librofd_ffi.so.0");
}
