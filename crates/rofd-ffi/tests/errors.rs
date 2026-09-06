use rofd_ffi::{
    rofd_error_free, rofd_error_get_message, rofd_error_get_status, ROFD_STATUS_INVALID_ARGUMENT,
};

#[test]
fn null_error_access_is_defined() {
    assert_eq!(
        unsafe { rofd_error_get_status(std::ptr::null()) },
        ROFD_STATUS_INVALID_ARGUMENT
    );
    assert!(unsafe { rofd_error_get_message(std::ptr::null()) }.is_null());
    unsafe { rofd_error_free(std::ptr::null_mut()) };
}
