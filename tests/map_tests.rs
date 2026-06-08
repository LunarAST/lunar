use lunar::{ActualJson, generate_lunar_map};
use std::collections::HashMap;
use std::fs;

#[test]
fn test_generate_lunar_map_basic() {
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

    // Check projects
    assert_eq!(lunar_map.projects.len(), 2);

    // Check alignments
    assert_eq!(lunar_map.alignments.len(), 2);
    let auth_alignment = lunar_map.alignments.iter()
        .find(|a| a.client_project == "auth-service")
        .unwrap();
    assert_eq!(auth_alignment.status, "Aligned");

    // Check aggregated edges
    assert_eq!(lunar_map.aggregated_edges.len(), 2);
    let edge = lunar_map.aggregated_edges.iter()
        .find(|e| e.client_project == "auth-service")
        .unwrap();
    assert_eq!(edge.call_count, 1);
    assert_eq!(edge.status, "Aligned");

    // Check anomalies
    assert_eq!(lunar_map.anomalies.unused_endpoints.len(), 0);
    assert_eq!(lunar_map.anomalies.orphaned_consumers.len(), 0);
}
