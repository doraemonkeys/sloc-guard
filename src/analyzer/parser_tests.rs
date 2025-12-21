use super::*;

#[test]
fn test_get_parser_language_mapping() {
    assert!(get_parser("rust").is_some());
    assert!(get_parser("RUST").is_some());
    assert!(get_parser("Rust").is_some());

    assert!(get_parser("go").is_some());
    assert!(get_parser("GO").is_some());

    assert!(get_parser("python").is_some());
    assert!(get_parser("Python").is_some());

    assert!(get_parser("javascript").is_some());
    assert!(get_parser("typescript").is_some());
    assert!(get_parser("jsx").is_some());
    assert!(get_parser("tsx").is_some());

    assert!(get_parser("c").is_some());
    assert!(get_parser("c++").is_some());
    assert!(get_parser("cpp").is_some());

    assert!(get_parser("unknown").is_none());
    assert!(get_parser("java").is_none());
    assert!(get_parser("ruby").is_none());
}
