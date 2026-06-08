use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

// ---------- Data structures ----------

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct RouteSegment {
    #[serde(rename = "type")]
    pub segment_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_constraint: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RouteEntry {
    pub method: String,
    pub segments: Vec<RouteSegment>,
    pub source_file: String,
    pub line_number: u32,
    pub extraction_method: String,
    #[serde(default)]
    pub target_project: Option<String>,
}

impl RouteEntry {
    pub fn to_path(&self) -> String {
        let mut path = String::new();
        for seg in &self.segments {
            match seg.segment_type.as_str() {
                "literal" => {
                    path.push('/');
                    path.push_str(seg.value.as_ref().unwrap());
                }
                "parameter" => {
                    path.push_str("/{");
                    path.push_str(seg.name.as_ref().unwrap());
                    path.push('}');
                }
                "wildcard" => {
                    path.push_str("/*");
                }
                _ => {}
            }
        }
        path
    }

    pub fn display_path(&self) -> String {
        let mut path = String::new();
        for seg in &self.segments {
            match seg.segment_type.as_str() {
                "literal" => {
                    path.push('/');
                    path.push_str(seg.value.as_ref().unwrap());
                }
                "parameter" => {
                    path.push_str("/:");
                    path.push_str(seg.name.as_ref().unwrap());
                }
                "wildcard" => path.push_str("/*"),
                _ => {}
            }
        }
        path
    }

    pub fn structural_id(&self) -> String {
        let types: Vec<&str> = self
            .segments
            .iter()
            .map(|s| s.segment_type.as_str())
            .collect();
        types.join(":")
    }

    pub fn get_aligned_parameter_names(&self, other: &RouteEntry) -> Vec<String> {
        let len = self.segments.len().min(other.segments.len());
        let mut names = Vec::new();
        for i in 0..len {
            let a = &self.segments[i];
            let b = &other.segments[i];
            if a.segment_type == "parameter" && b.segment_type == "parameter" {
                names.push(a.name.clone().unwrap_or_default());
            }
        }
        names
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActualJson {
    pub exposed: Vec<RouteEntry>,
    pub consumed: Vec<RouteEntry>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct InterfacesYml {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exposed: Option<Vec<InterfaceItem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consumed: Option<Vec<InterfaceItem>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InterfaceItem {
    pub path: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "targetProject")]
    pub target_project: Option<String>,
}

// ---------- Alignment ----------

#[derive(Debug, PartialEq)]
pub enum DiffResult {
    Added,
    Removed,
    MethodChanged { old_method: String, new_method: String },
    ParamNamesChanged { old_names: Vec<String>, new_names: Vec<String> },
    Unchanged,
}

pub fn compare_routes(old: &RouteEntry, new: &RouteEntry) -> DiffResult {
    if old.method != new.method {
        return DiffResult::MethodChanged {
            old_method: old.method.clone(),
            new_method: new.method.clone(),
        };
    }
    let old_params = old.get_aligned_parameter_names(new);
    let new_params = new.get_aligned_parameter_names(old);
    if old_params != new_params {
        return DiffResult::ParamNamesChanged {
            old_names: old_params,
            new_names: new_params,
        };
    }
    DiffResult::Unchanged
}

pub fn build_structural_index(routes: &[RouteEntry]) -> HashMap<String, Vec<RouteEntry>> {
    let mut index: HashMap<String, Vec<RouteEntry>> = HashMap::new();
    for r in routes {
        index.entry(r.structural_id()).or_default().push(r.clone());
    }
    index
}

// ---------- Lunar Map Generation ----------

#[derive(Debug, Serialize, Deserialize)]
pub struct LunarMapConfig {
    pub projects: HashMap<String, String>, // project_name -> path to actual.json
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub project_type: String,
    pub sha: String,
    #[serde(rename = "scanStatus")]
    pub scan_status: String,
    pub interfaces: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlignmentEntry {
    #[serde(rename = "clientProject")]
    pub client_project: String,
    #[serde(rename = "serverProject")]
    pub server_project: String,
    pub path: String,
    pub method: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AggregatedEdge {
    #[serde(rename = "clientProject")]
    pub client_project: String,
    #[serde(rename = "serverProject")]
    pub server_project: String,
    #[serde(rename = "callCount")]
    pub call_count: usize,
    pub status: String,
    pub paths: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Anomalies {
    #[serde(rename = "unusedEndpoints")]
    pub unused_endpoints: Vec<AnomalyEndpoint>,
    #[serde(rename = "orphanedConsumers")]
    pub orphaned_consumers: Vec<AnomalyEndpoint>,
    #[serde(rename = "crossLayerViolations")]
    pub cross_layer_violations: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnomalyEndpoint {
    pub project: String,
    pub path: String,
    pub method: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LunarMap {
    pub version: String,
    pub projects: Vec<ProjectInfo>,
    pub alignments: Vec<AlignmentEntry>,
    #[serde(rename = "aggregatedEdges")]
    pub aggregated_edges: Vec<AggregatedEdge>,
    pub anomalies: Anomalies,
}

/// Align one consumer route against the exposed routes of all other projects.
fn align_consumed_route(
    consumed: &RouteEntry,
    client_name: &str,
    project_map: &HashMap<String, &ActualJson>,
) -> Option<AlignmentEntry> {
    let target = consumed.target_project.as_deref().unwrap_or("");

    for (server_name, server_actual) in project_map {
        if server_name == client_name {
            continue;
        }
        if !target.is_empty() && target != *server_name {
            continue;
        }

        if let Some(server_route) = server_actual
            .exposed
            .iter()
            .find(|r| r.structural_id() == consumed.structural_id())
        {
            let result = compare_routes(consumed, server_route);
            let (status, warning) = match result {
                DiffResult::Unchanged => ("Aligned".to_string(), None),
                DiffResult::ParamNamesChanged { .. } => {
                    ("ParamNameMismatch".to_string(), None)
                }
                DiffResult::MethodChanged { .. } => {
                    ("MethodMismatch".to_string(), None)
                }
                _ => unreachable!(),
            };
            return Some(AlignmentEntry {
                client_project: client_name.to_string(),
                server_project: server_name.clone(),
                path: consumed.to_path(),
                method: consumed.method.clone(),
                status,
                warning,
            });
        }
    }

    if !target.is_empty() {
        Some(AlignmentEntry {
            client_project: client_name.to_string(),
            server_project: target.to_string(),
            path: consumed.to_path(),
            method: consumed.method.clone(),
            status: "Orphaned".to_string(),
            warning: None,
        })
    } else {
        None
    }
}

/// Compute the most severe status from a list of status strings.
fn aggregate_status(statuses: &[String]) -> String {
    let priority = |s: &str| -> usize {
        match s {
            "MethodMismatch" => 0,
            "Orphaned" => 1,
            "ParamNameMismatch" => 2,
            "Aligned" => 3,
            _ => 4,
        }
    };
    statuses
        .iter()
        .min_by_key(|s| priority(s))
        .cloned()
        .unwrap_or_else(|| "Aligned".to_string())
}

/// Detect unused endpoints: exposed routes of a project that no other project consumes.
fn detect_unused_endpoints(
    project_actuals: &HashMap<String, ActualJson>,
    alignments: &[AlignmentEntry],
) -> Vec<AnomalyEndpoint> {
    let mut unused = Vec::new();
    for (name, actual) in project_actuals {
        for exposed in &actual.exposed {
            let consumed_by_any = alignments.iter().any(|a| {
                a.server_project == *name
                    && a.path == exposed.to_path()
                    && a.method == exposed.method
                    && a.status != "Orphaned"
            });
            if !consumed_by_any {
                unused.push(AnomalyEndpoint {
                    project: name.clone(),
                    path: exposed.to_path(),
                    method: exposed.method.clone(),
                });
            }
        }
    }
    unused
}

pub fn generate_lunar_map(
    project_actuals: &HashMap<String, ActualJson>,
) -> LunarMap {
    let mut projects = Vec::new();
    let mut alignments = Vec::new();

    // Build project infos
    for (name, actual) in project_actuals {
        let interfaces = serde_json::json!({
            "exposed": actual.exposed.iter().map(|r| {
                serde_json::json!({
                    "path": r.to_path(),
                    "method": r.method
                })
            }).collect::<Vec<_>>(),
            "consumed": actual.consumed.iter().map(|r| {
                serde_json::json!({
                    "path": r.to_path(),
                    "method": r.method,
                    "targetProject": r.target_project.as_deref().unwrap_or("unknown")
                })
            }).collect::<Vec<_>>(),
        });
        projects.push(ProjectInfo {
            name: name.clone(),
            project_type: "mixed".to_string(),
            sha: "unknown".to_string(),
            scan_status: "success".to_string(),
            interfaces,
        });
    }

    // Build alignments
    let project_map: HashMap<String, &ActualJson> = project_actuals
        .iter()
        .map(|(k, v)| (k.clone(), v))
        .collect();

    for (client_name, actual) in project_actuals {
        for consumed in &actual.consumed {
            if let Some(entry) = align_consumed_route(consumed, client_name, &project_map) {
                alignments.push(entry);
            }
        }
    }

    // Build aggregated edges
    let mut edge_groups: HashMap<(String, String), Vec<&AlignmentEntry>> = HashMap::new();
    for entry in &alignments {
        let key = (entry.client_project.clone(), entry.server_project.clone());
        edge_groups.entry(key).or_default().push(entry);
    }
    let aggregated_edges: Vec<AggregatedEdge> = edge_groups
        .into_iter()
        .map(|((client, server), entries)| {
            let statuses: Vec<String> = entries.iter().map(|e| e.status.clone()).collect();
            let paths: Vec<String> = entries.iter().map(|e| format!("{} {}", e.method, e.path)).collect();
            AggregatedEdge {
                client_project: client,
                server_project: server,
                call_count: entries.len(),
                status: aggregate_status(&statuses),
                paths,
            }
        })
        .collect();

    // Detect anomalies
    let unused_endpoints = detect_unused_endpoints(project_actuals, &alignments);
    let orphaned_consumers: Vec<AnomalyEndpoint> = alignments
        .iter()
        .filter(|a| a.status == "Orphaned")
        .map(|a| AnomalyEndpoint {
            project: a.client_project.clone(),
            path: a.path.clone(),
            method: a.method.clone(),
        })
        .collect();

    let anomalies = Anomalies {
        unused_endpoints,
        orphaned_consumers,
        cross_layer_violations: vec![],
    };

    LunarMap {
        version: "0.5.0".to_string(),
        projects,
        alignments,
        aggregated_edges,
        anomalies,
    }
}

// ---------- Adapter ----------

pub fn find_adapter(name: &str) -> Option<String> {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let local_path = exe_dir.join(name);
            if local_path.exists() {
                return Some(local_path.to_string_lossy().to_string());
            }
        }
    }
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in path_var.split(':') {
            let candidate = Path::new(dir).join(name);
            if candidate.exists() {
                return Some(candidate.to_string_lossy().to_string());
            }
        }
    }
    None
}

pub fn run_adapter() -> anyhow::Result<Vec<RouteEntry>> {
    let project_dir = std::env::current_dir()?;
    let project_dir_str = project_dir.to_string_lossy().to_string();

    let adapter_name = if project_dir.join("Cargo.toml").exists() {
        "lunar-extract-rust"
    } else {
        anyhow::bail!("Could not detect project language. Currently supported: Rust (Cargo.toml found).");
    };

    let adapter_path = find_adapter(adapter_name).ok_or_else(|| {
        anyhow::anyhow!(
            "Adapter '{}' not found in PATH. Install it first.\n\
             For Rust/Axum: cargo install lunar-extract-rust\n\
             Or build from source: https://github.com/LunarAST/lunar-extract-rust",
            adapter_name
        )
    })?;

    let output = Command::new(&adapter_path)
        .arg(&project_dir_str)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Adapter '{}' failed: {}", adapter_name, stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    parse_routes(&stdout)
}

pub fn parse_routes(ldjson: &str) -> anyhow::Result<Vec<RouteEntry>> {
    let mut routes: Vec<RouteEntry> = Vec::new();
    let mut count_from_marker: Option<usize> = None;

    for line in ldjson.lines() {
        if line.is_empty() {
            continue;
        }
        if line.contains("\"_lunar\"") {
            let marker: serde_json::Value = serde_json::from_str(line)?;
            if marker["_lunar"]["status"] == "success" {
                count_from_marker = Some(marker["_lunar"]["count"].as_u64().unwrap() as usize);
            } else if marker["_lunar"]["status"] == "error" {
                let msg = marker["_lunar"]["message"].as_str().unwrap_or("unknown error");
                anyhow::bail!("Adapter reported error: {}", msg);
            }
            continue;
        }
        let route: RouteEntry = serde_json::from_str(line)?;
        routes.push(route);
    }

    if let Some(expected) = count_from_marker {
        if routes.len() != expected {
            anyhow::bail!("Count mismatch: expected {}, got {}", expected, routes.len());
        }
    } else {
        anyhow::bail!("No end marker found in adapter output");
    }
    Ok(routes)
}
