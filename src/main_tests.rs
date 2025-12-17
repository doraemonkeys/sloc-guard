use sloc_guard::{EXIT_CONFIG_ERROR, EXIT_SUCCESS, EXIT_THRESHOLD_EXCEEDED};

#[test]
fn exit_codes_documented() {
    assert_eq!(EXIT_SUCCESS, 0);
    assert_eq!(EXIT_THRESHOLD_EXCEEDED, 1);
    assert_eq!(EXIT_CONFIG_ERROR, 2);
}
