use super::*;

#[test]
fn exit_codes_are_distinct() {
    assert_ne!(EXIT_SUCCESS, EXIT_THRESHOLD_EXCEEDED);
    assert_ne!(EXIT_SUCCESS, EXIT_CONFIG_ERROR);
    assert_ne!(EXIT_THRESHOLD_EXCEEDED, EXIT_CONFIG_ERROR);
}
