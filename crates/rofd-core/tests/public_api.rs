use rofd_core::VERSION;

#[test]
fn exposes_crate_version() {
    assert_eq!(VERSION, "0.2.1");
}
