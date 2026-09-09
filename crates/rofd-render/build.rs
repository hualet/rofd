//! Locates the system jbig2dec library for the default `jbig2` feature,
//! the same way Cairo and FreeType are found: through pkg-config.
//!
//! The runtime-version arguments `jbig2_ctx_new_imp` requires are exported
//! as compile-time environment variables so the FFI wrapper passes the
//! version pkg-config resolved.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var_os("CARGO_FEATURE_JBIG2").is_none() {
        return;
    }
    let library = pkg_config::probe_library("jbig2dec").unwrap_or_else(|error| {
        panic!(
            "the default `jbig2` feature needs the system jbig2dec development files \
             (Debian: libjbig2dec0-dev); install them or build with --no-default-features: {error}"
        )
    });
    let (major, minor) = parse_version(&library.version);
    println!("cargo:rustc-env=ROFD_JBIG2_VERSION_MAJOR={major}");
    println!("cargo:rustc-env=ROFD_JBIG2_VERSION_MINOR={minor}");
}

/// Extracts the numeric version pkg-config reports (`0.20` → `0`, `20`).
fn parse_version(version: &str) -> (String, String) {
    let mut parts = version.split('.');
    let major = parts.next().unwrap_or("0").trim().to_owned();
    let minor = parts.next().unwrap_or("0").trim().to_owned();
    let digits = |value: &str| !value.is_empty() && value.chars().all(|c| c.is_ascii_digit());
    assert!(
        digits(&major) && digits(&minor),
        "unparsable jbig2dec version {version}"
    );
    (major, minor)
}
