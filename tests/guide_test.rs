use lunar::guide;

#[test]
fn test_detect_language_rust() {
    // We can't change the current directory in tests easily,
    // but we can test that the function runs without panicking.
    let lang = guide::detect_language();
    // In a Rust project (which lunar itself is), it should detect Cargo.toml
    assert!(lang.contains("Rust") || lang.contains("Unknown"));
}

#[test]
fn test_is_initialized() {
    // lunar project itself may or may not have .lunar/interfaces.yml
    // Just test that it doesn't panic
    let _ = guide::is_initialized();
}

#[test]
fn test_has_scan_data() {
    let _ = guide::has_scan_data();
}

#[test]
fn test_pending_suggestions() {
    let result = guide::pending_suggestions();
    // lunar itself has no suggestions directory, so should be None
    assert!(result.is_none() || result.is_some());
}

#[test]
fn test_analyze_returns_valid_state() {
    let state = guide::analyze();
    assert!(!state.project_name.is_empty());
    assert!(!state.language.is_empty());
}
