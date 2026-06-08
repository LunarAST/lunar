use lunar::{ActualJson, generate_lunar_map};
use std::collections::HashMap;
use std::fs;

#[test]
fn test_generate_lunar_map() {
    let auth_actual: ActualJson = serde_json::from_str(
        &fs::read_to_string("tests/fixtures/map/auth_actual.json").unwrap()
    ).unwrap();
    let user_actual: ActualJson = serde_json::from_str(
        &fs::read_to_string("tests/fixtures/map/user_actual.json").unwrap()
    ).unwrap();

    let mut map = HashMap::new();
    map.insert("auth-service".to_string(), auth_actual);
    map.insert("user-service".to_string(), user_actual);

    let lunar_map = generate_lunar_map(&map);

    assert_eq!(lunar_map.projects.len(), 2);

    // Check alignments: we expect 2 alignments (cross-matched)
    assert_eq!(lunar_map.alignments.len(), 2);
    // auth-service consumed POST /users -> user-service exposed POST /users -> Aligned
    let auth_alignment = lunar_map.alignments.iter()
        .find(|a| a.client_project == "auth-service")
        .unwrap();
    assert_eq!(auth_alignment.status, "Aligned");

    // user-service consumed GET /auth/verify -> auth-service exposed GET /auth/verify -> Aligned
    let user_alignment = lunar_map.alignments.iter()
        .find(|a| a.client_project == "user-service")
        .unwrap();
    assert_eq!(user_alignment.status, "Aligned");
}
