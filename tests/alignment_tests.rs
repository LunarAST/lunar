use lunar_interface::{RouteEntry, compare_routes, DiffResult};
use serde_json;
use std::fs;

fn load_pair(path: &str) -> (RouteEntry, RouteEntry) {
    let data = fs::read_to_string(path).expect("failed to read test fixture");
    let routes: Vec<RouteEntry> = serde_json::from_str(&data).expect("failed to parse fixture");
    assert_eq!(routes.len(), 2, "fixture must contain exactly two routes");
    (routes[0].clone(), routes[1].clone())
}

#[test]
fn test_identical_routes() {
    let (a, b) = load_pair("tests/fixtures/identical.json");
    assert_eq!(compare_routes(&a, &b), DiffResult::Unchanged);
}

#[test]
fn test_method_changed() {
    let (a, b) = load_pair("tests/fixtures/method_changed.json");
    assert_eq!(
        compare_routes(&a, &b),
        DiffResult::MethodChanged {
            old_method: "GET".into(),
            new_method: "POST".into()
        }
    );
}

#[test]
fn test_param_names_changed() {
    let (a, b) = load_pair("tests/fixtures/param_changed.json");
    assert_eq!(
        compare_routes(&a, &b),
        DiffResult::ParamNamesChanged {
            old_names: vec!["id".into()],
            new_names: vec!["user_id".into()]
        }
    );
}

#[test]
fn test_heuristic_match_ignored() {
    let (a, b) = load_pair("tests/fixtures/heuristic.json");
    assert_eq!(compare_routes(&a, &b), DiffResult::Unchanged);
}
