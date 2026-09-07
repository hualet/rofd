const HEADER: &str = include_str!("../include/rofd.h");
const DESIGN: &str =
    include_str!("../../../docs/superpowers/specs/2026-09-06-rofd-c-abi-v1-design.md");

fn normalized(document: &str) -> String {
    document
        .replace(['*', '`'], "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn header_and_design_state_complete_input_pointer_contract() {
    for document in [HEADER, DESIGN] {
        let document = normalized(document);
        assert!(document.contains("non-NULL input C string"));
        assert!(document.contains("valid readable character array object"));
        assert!(document.contains("containing UTF-8 bytes"));
        assert!(document.contains("terminating NUL byte"));
        assert!(document.contains("options pointer must be correctly aligned"));
        assert!(document.contains("live handle storage must not overlap any output slot"));
    }
}

#[test]
fn document_open_contract_defines_null_and_empty_failures() {
    for document in [HEADER, DESIGN] {
        let document = normalized(document);
        assert!(document.contains("NULL path is a defined input"));
        assert!(document.contains("empty path"));
        assert!(document.contains("ROFD_STATUS_INVALID_ARGUMENT"));
        assert!(document.contains("non-NULL path"));
    }
}

#[test]
fn header_and_design_bound_all_ffi_input_regions() {
    for document in [HEADER, DESIGN] {
        let document = normalized(document);
        assert!(document.contains("first byte of a valid readable character array object"));
        assert!(document.contains("terminating NUL byte must occur within that same object"));
        assert!(document.contains("byte length must be representable by ptrdiff_t"));
        assert!(document.contains("correctly aligned for const char"));
        assert!(document.contains("single valid, initialized, readable array object"));
        assert!(document.contains("complete v1 prefix must be initialized and readable"));
    }
}
