const HEADER: &str = include_str!("../include/rofd.h");
const DESIGN: &str =
    include_str!("../../../docs/superpowers/specs/2026-09-06-rofd-c-abi-v1-design.md");

fn normalized(document: &str) -> String {
    document
        .replace('*', "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn header_and_design_state_complete_input_pointer_contract() {
    for document in [HEADER, DESIGN] {
        let document = normalized(document);
        assert!(document.contains("non-NULL input C string"));
        assert!(document.contains("readable NUL-terminated UTF-8"));
        assert!(document.contains("options must be correctly aligned"));
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
