use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio, ExitCode};
use std::time::Duration;
use wait_timeout::ChildExt;

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
    #[serde(rename = "targetProject")]
    pub target_project: Option<String>,
}

impl RouteEntry {
    pub fn to_path(&self) -> String {
        let mut path = String::new();
        for seg in &self.segments {
            match seg.segment_type.as_str() {
                "literal" => { path.push('/'); path.push_str(seg.value.as_ref().unwrap()); }
                "parameter" => { path.push_str("/{"); path.push_str(seg.name.as_ref().unwrap()); path.push('}'); }
                "wildcard" => path.push_str("/*"),
                _ => {}
            }
        }
        path
    }

    pub fn display_path(&self) -> String {
        let mut path = String::new();
        for seg in &self.segments {
            match seg.segment_type.as_str() {
                "literal" => { path.push('/'); path.push_str(seg.value.as_ref().unwrap()); }
                "parameter" => { path.push_str("/:"); path.push_str(seg.name.as_ref().unwrap()); }
                "wildcard" => path.push_str("/*"),
                _ => {}
            }
        }
        path
    }

    pub fn structural_id(&self) -> String {
        let types: Vec<&str> = self.segments.iter().map(|s| s.segment_type.as_str()).collect();
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
        return DiffResult::MethodChanged { old_method: old.method.clone(), new_method: new.method.clone() };
    }
    let old_params = old.get_aligned_parameter_names(new);
    let new_params = new.get_aligned_parameter_names(old);
    if old_params != new_params {
        return DiffResult::ParamNamesChanged { old_names: old_params, new_names: new_params };
    }
    DiffResult::Unchanged
}

pub fn build_structural_index(routes: &[RouteEntry]) -> HashMap<String, Vec<RouteEntry>> {
    let mut index: HashMap<String, Vec<RouteEntry>> = HashMap::new();
    for r in routes { index.entry(r.structural_id()).or_default().push(r.clone()); }
    index
}

// ---------- Lunar Map Generation ----------

#[derive(Debug, Serialize, Deserialize)]
pub struct LunarMapConfig {
    pub projects: HashMap<String, String>,
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
pub struct PortConnection {
    pub path: String,
    pub method: String,
    pub status: String,
    #[serde(rename = "sourcePortIndex")]
    pub source_port_index: usize,
    #[serde(rename = "targetPortIndex")]
    pub target_port_index: usize,
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
    pub ports: Vec<PortConnection>,
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

fn align_consumed_route(
    consumed: &RouteEntry,
    client_name: &str,
    project_map: &HashMap<String, &ActualJson>,
    scan_status_map: &HashMap<String, String>,
) -> (String, Option<String>) {
    let target = consumed.target_project.as_deref().unwrap_or("");
    let target_scan_failed = !target.is_empty()
        && scan_status_map.get(target).map_or(false, |s| s == "failed" || s == "stale");
    if target_scan_failed {
        return ("Unverified".to_string(), Some("Server project scan failed or stale".to_string()));
    }
    for (server_name, server_actual) in project_map {
        if server_name == client_name { continue; }
        if !target.is_empty() && target != *server_name { continue; }
        if let Some(server_route) = server_actual.exposed.iter().find(|r| r.structural_id() == consumed.structural_id()) {
            let result = compare_routes(consumed, server_route);
            let (status, warning) = match result {
                DiffResult::Unchanged => ("Aligned".to_string(), None),
                DiffResult::ParamNamesChanged { .. } => ("ParamNameMismatch".to_string(), None),
                DiffResult::MethodChanged { .. } => ("MethodMismatch".to_string(), None),
                _ => unreachable!(),
            };
            if scan_status_map.get(server_name).map_or(false, |s| s == "failed" || s == "stale") {
                return ("Unverified".to_string(), Some("Server project scan failed or stale".to_string()));
            }
            return (status, warning);
        }
    }
    if target.is_empty() { (String::new(), None) }
    else { ("Orphaned".to_string(), None) }
}

fn aggregate_status(statuses: &[String]) -> String {
    let priority = |s: &str| -> usize {
        match s { "MethodMismatch" => 0, "Orphaned" => 1, "ParamNameMismatch" => 2, "Aligned" => 3, _ => 4 }
    };
    statuses.iter().min_by_key(|s| priority(s)).cloned().unwrap_or_else(|| "Aligned".to_string())
}

fn detect_unused_endpoints(project_actuals: &HashMap<String, ActualJson>, alignments: &[AlignmentEntry]) -> Vec<AnomalyEndpoint> {
    let mut unused = Vec::new();
    for (name, actual) in project_actuals {
        for exposed in &actual.exposed {
            let consumed_by_any = alignments.iter().any(|a| {
                a.server_project == *name && a.path == exposed.to_path() && a.method == exposed.method && a.status != "Orphaned"
            });
            if !consumed_by_any {
                unused.push(AnomalyEndpoint { project: name.clone(), path: exposed.to_path(), method: exposed.method.clone() });
            }
        }
    }
    unused
}

pub fn generate_lunar_map(
    project_actuals: &HashMap<String, ActualJson>,
    scan_statuses: &HashMap<String, String>,
) -> LunarMap {
    let mut projects = Vec::new();
    let mut alignments = Vec::new();
    for (name, actual) in project_actuals {
        let scan_status = scan_statuses.get(name).cloned().unwrap_or_else(|| "success".to_string());
        let interfaces = serde_json::json!({
            "exposed": actual.exposed.iter().map(|r| serde_json::json!({"path": r.to_path(), "method": r.method})).collect::<Vec<_>>(),
            "consumed": actual.consumed.iter().map(|r| serde_json::json!({"path": r.to_path(), "method": r.method, "targetProject": r.target_project.as_deref().unwrap_or("unknown")})).collect::<Vec<_>>(),
        });
        projects.push(ProjectInfo { name: name.clone(), project_type: "mixed".to_string(), sha: "unknown".to_string(), scan_status, interfaces });
    }
    let project_map: HashMap<String, &ActualJson> = project_actuals.iter().map(|(k, v)| (k.clone(), v)).collect();
    let scan_status_map = scan_statuses.clone();
    for (client_name, actual) in project_actuals {
        for consumed in &actual.consumed {
            let (status, warning) = align_consumed_route(consumed, client_name, &project_map, &scan_status_map);
            if status.is_empty() { continue; }
            alignments.push(AlignmentEntry { client_project: client_name.clone(), server_project: consumed.target_project.as_deref().unwrap_or("unknown").to_string(), path: consumed.to_path(), method: consumed.method.clone(), status, warning });
        }
    }
    let mut edge_groups: HashMap<(String, String), Vec<&AlignmentEntry>> = HashMap::new();
    for entry in &alignments { edge_groups.entry((entry.client_project.clone(), entry.server_project.clone())).or_default().push(entry); }
    let aggregated_edges: Vec<AggregatedEdge> = edge_groups.into_iter().map(|((client, server), entries)| {
        let statuses: Vec<String> = entries.iter().map(|e| e.status.clone()).collect();
        let paths: Vec<String> = entries.iter().map(|e| format!("{} {}", e.method, e.path)).collect();
        let mut ports = Vec::new();
        if let (Some(ca), Some(sa)) = (project_actuals.get(&client), project_actuals.get(&server)) {
            for entry in &entries {
                let si = ca.consumed.iter().position(|c| c.to_path() == entry.path && c.method == entry.method);
                let ti = sa.exposed.iter().position(|e| e.to_path() == entry.path && e.method == entry.method);
                if let (Some(s), Some(t)) = (si, ti) { ports.push(PortConnection { path: entry.path.clone(), method: entry.method.clone(), status: entry.status.clone(), source_port_index: s, target_port_index: t }); }
            }
        }
        AggregatedEdge { client_project: client, server_project: server, call_count: entries.len(), status: aggregate_status(&statuses), paths, ports }
    }).collect();
    let unused_endpoints = detect_unused_endpoints(project_actuals, &alignments);
    let orphaned_consumers: Vec<AnomalyEndpoint> = alignments.iter().filter(|a| a.status == "Orphaned").map(|a| AnomalyEndpoint { project: a.client_project.clone(), path: a.path.clone(), method: a.method.clone() }).collect();
    LunarMap { version: "0.5.0".to_string(), projects, alignments, aggregated_edges, anomalies: Anomalies { unused_endpoints, orphaned_consumers, cross_layer_violations: vec![] } }
}

// ---------- Adapter ----------

pub fn find_adapter(name: &str) -> Option<String> {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let local_path = exe_dir.join(name);
            if local_path.exists() { return Some(local_path.to_string_lossy().to_string()); }
        }
    }
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in path_var.split(':') {
            let candidate = Path::new(dir).join(name);
            if candidate.exists() { return Some(candidate.to_string_lossy().to_string()); }
        }
    }
    None
}

pub fn run_adapter() -> anyhow::Result<Vec<RouteEntry>> {
    let project_dir = std::env::current_dir()?;
    let project_dir_str = project_dir.to_string_lossy().to_string();
    let adapter_name = if project_dir.join("Cargo.toml").exists() { "lunar-extract-rust" }
    else { anyhow::bail!("Could not detect project language. Currently supported: Rust (Cargo.toml found)."); };
    let adapter_path = find_adapter(adapter_name).ok_or_else(|| anyhow::anyhow!("Adapter '{}' not found in PATH.", adapter_name))?;

    let mut child = Command::new(&adapter_path)
        .arg(&project_dir_str)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let timeout = Duration::from_secs(30);

    match child.wait_timeout(timeout)? {
        Some(status) if status.success() => {
            let mut stdout = String::new();
            child.stdout.take().unwrap().read_to_string(&mut stdout)?;
            parse_routes(&stdout)
        }
        Some(_) => {
            let mut stderr = String::new();
            child.stderr.take().unwrap().read_to_string(&mut stderr)?;
            anyhow::bail!("Adapter '{}' failed: {}", adapter_name, stderr);
        }
        None => {
            child.kill()?;
            child.wait()?;
            anyhow::bail!("Adapter '{}' timed out after 30 seconds", adapter_name);
        }
    }
}

pub fn parse_routes(ldjson: &str) -> anyhow::Result<Vec<RouteEntry>> {
    let mut routes = Vec::new();
    let mut count_from_marker: Option<usize> = None;
    for line in ldjson.lines() {
        if line.is_empty() { continue; }
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
        routes.push(serde_json::from_str(line)?);
    }
    if let Some(expected) = count_from_marker {
        if routes.len() != expected { anyhow::bail!("Count mismatch: expected {}, got {}", expected, routes.len()); }
    } else { anyhow::bail!("No end marker found"); }
    Ok(routes)
}

// ---------- Doctor ----------

/// Run ecosystem health checks. Returns an exit code (0 = healthy, 1 = env error, 2 = data error).
pub fn doctor_check() -> ExitCode {
    let mut issues = 0u8;
    let mut env_issues = 0u8;

    println!("🔍 LunarAST Doctor — Ecosystem Health Check\n");

    // Check 1: Project root
    let cargo_path = Path::new("Cargo.toml");
    if cargo_path.exists() {
        println!("✅ Project: Rust project detected (Cargo.toml)");
    } else {
        println!("❌ Project: No Cargo.toml found");
        println!("   → Ensure you run this command in a Rust project root.");
        env_issues += 1;
        issues += 1;
    }

    // Check 2: Adapter path detection (config override first, then PATH)
    let config_path = Path::new(".lunar").join("config.yml");
    let adapter_name = "lunar-extract-rust";
    let adapter_source = if config_path.exists() {
        if let Ok(config_content) = std::fs::read_to_string(&config_path) {
            if let Ok(config) = serde_yaml::from_str::<serde_yaml::Value>(&config_content) {
                if let Some(adapters) = config.get("adapters") {
                    if let Some(path) = adapters.get(adapter_name) {
                        if let Some(path_str) = path.as_str() {
                            let p = Path::new(path_str);
                            if p.exists() {
                                println!("✅ Adapter: {} found at {} [Config Overridden]", adapter_name, p.display());
                                None
                            } else {
                                println!("❌ Adapter: Config override points to non-existent path: {}", path_str);
                                env_issues += 1;
                                issues += 1;
                                Some("config override invalid".to_string())
                            }
                        } else { None }
                    } else { None }
                } else { None }
            } else { None }
        } else { None }
    } else {
        None
    };

    // Fallback to PATH if no config override
    if adapter_source.is_none() && !config_path.exists() {
        match find_adapter(adapter_name) {
            Some(path) => println!("✅ Adapter: {} found at {} [PATH]", adapter_name, path),
            None => {
                println!("❌ Adapter: {} not found in PATH", adapter_name);
                println!("   → Install: cargo install lunar-extract-rust");
                env_issues += 1;
                issues += 1;
            }
        }
    }

    // Check 3: Adapter handshake test (skip if adapter not found)
    if issues == 0 || env_issues == 0 {
        match run_adapter() {
            Ok(routes) => println!("✅ Adapter test: successfully extracted {} routes", routes.len()),
            Err(e) => {
                println!("❌ Adapter test: handshake failed — {}", e);
                println!("   → Check adapter version and project structure.");
                env_issues += 1;
                issues += 1;
            }
        }
    }

    // Check 4: Physical facts cache (.interfaces-autogen.json)
    let autogen_path = Path::new(".lunar").join(".interfaces-autogen.json");
    if autogen_path.exists() {
        println!("✅ Scan data: .lunar/.interfaces-autogen.json exists");
    } else {
        println!("❌ Scan data: .lunar/.interfaces-autogen.json missing");
        println!("   → Run `lunar scan` to generate physical facts.");
        env_issues += 1;
        issues += 1;
    }

    // Check 5: Cache format validation
    if autogen_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&autogen_path) {
            if serde_json::from_str::<ActualJson>(&content).is_ok() {
                println!("✅ Data format: valid JSON with exposed/consumed fields");
            } else {
                println!("❌ Data format: JSON corrupted or schema mismatch");
                println!("   → Run `lunar scan` to regenerate.");
                issues += 1;
            }
        }
    }

    // Check 6: Interfaces consistency
    let interfaces_path = Path::new(".lunar").join("interfaces.yml");
    if interfaces_path.exists() {
        println!("✅ Interfaces: .lunar/interfaces.yml exists");
        // Full merge conflict detection not yet implemented
        println!("   ⚠️  Merge conflict detection not yet implemented — coming in a future version.");
    } else {
        println!("⚠️  Interfaces: .lunar/interfaces.yml not found");
        println!("   → Run `lunar sync --apply` to create it from scan data.");
    }

    println!();
    if issues == 0 {
        println!("🟢 All checks passed. Ecosystem is healthy.");
        ExitCode::from(0)
    } else if env_issues > 0 {
        println!("🔴 {} environment issue(s) found.", env_issues);
        ExitCode::from(1)
    } else {
        println!("🔴 {} data issue(s) found.", issues);
        ExitCode::from(2)
    }
}

// ---------- Cleanup ----------

/// Remove local scan cache files. Returns list of removed files.
/// Does NOT delete .lunar/.backup/ or .lunar/interfaces.yml.
pub fn cleanup_local(force: bool) -> anyhow::Result<Vec<String>> {
    let lunar_dir = Path::new(".lunar");
    if !lunar_dir.exists() {
        println!("No .lunar/ directory found. Nothing to clean up.");
        return Ok(vec![]);
    }

    let candidates = vec![
        lunar_dir.join("route-ast-actual.json"),
        lunar_dir.join(".interfaces-autogen.json"),
    ];

    let to_remove: Vec<_> = candidates.into_iter().filter(|p| p.exists()).collect();

    if to_remove.is_empty() {
        println!("No cache files found. Nothing to clean up.");
        return Ok(vec![]);
    }

    println!("The following files will be removed:");
    for f in &to_remove {
        println!("  - {}", f.display());
    }
    println!();

    if !force {
        println!("This action cannot be undone.");
        print!("Are you sure you want to continue? [y/N] ");
        use std::io::Write;
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();
        if input != "y" && input != "yes" {
            println!("Cleanup cancelled.");
            return Ok(vec![]);
        }
    }

    let mut removed = Vec::new();
    for f in &to_remove {
        std::fs::remove_file(f)?;
        removed.push(f.display().to_string());
    }

    for r in &removed {
        println!("✓ Removed {}", r);
    }
    println!("Cleanup complete.");
    Ok(removed)
}
