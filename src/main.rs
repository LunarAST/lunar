use anyhow::Result;
use chrono::Utc;
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

/// LunarAST CLI - Static contract extraction and comparison
#[derive(Parser)]
#[command(name = "lunar")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan the current project and output actual.json
    Scan,
    /// Compare current project routes with last saved actual.json
    Diff,
    /// Sync changes from scan into the intent overlay (with backup)
    Sync {
        /// Actually apply changes (writes to interfaces.yml)
        #[arg(long)]
        apply: bool,
        /// Preview changes without writing
        #[arg(long)]
        dry_run: bool,
    },
}

// ---------- Data structures ----------

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct RouteSegment {
    #[serde(rename = "type")]
    segment_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    raw_constraint: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct RouteEntry {
    method: String,
    segments: Vec<RouteSegment>,
    source_file: String,
    line_number: u32,
    extraction_method: String,
}

impl RouteEntry {
    fn to_path(&self) -> String {
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

    /// Display path (uses actual segment values, not normalized parameter names)
    fn display_path(&self) -> String {
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

    /// Structural identity: segment count and type sequence
    fn structural_id(&self) -> String {
        let types: Vec<&str> = self.segments
            .iter()
            .map(|s| s.segment_type.as_str())
            .collect();
        types.join(":")
    }

    /// Aligned parameter names: only positions where both have parameter type
    fn get_aligned_parameter_names(&self, other: &RouteEntry) -> Vec<String> {
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
struct ActualJson {
    exposed: Vec<RouteEntry>,
    consumed: Vec<RouteEntry>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct InterfacesYml {
    #[serde(skip_serializing_if = "Option::is_none")]
    project: Option<String>,
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    project_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    environment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exposed: Option<Vec<InterfaceItem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    consumed: Option<Vec<InterfaceItem>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct InterfaceItem {
    path: String,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "targetProject")]
    target_project: Option<String>,
}

// ---------- Alignment statuses ----------

#[derive(Debug, PartialEq)]
enum DiffResult {
    Added,
    Removed,
    MethodChanged { old_method: String, new_method: String },
    ParamNamesChanged { old_names: Vec<String>, new_names: Vec<String> },
    Unchanged,
}

// ---------- Adapter process management ----------

fn find_adapter(name: &str) -> Option<String> {
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

fn run_adapter() -> Result<Vec<RouteEntry>> {
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

fn parse_routes(ldjson: &str) -> Result<Vec<RouteEntry>> {
    let mut routes: Vec<RouteEntry> = Vec::new();
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

// ---------- Ordinal alignment ----------

/// Compare two routes at the same structural position.
fn compare_routes(old: &RouteEntry, new: &RouteEntry) -> DiffResult {
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

/// Build a "structural index" that maps structural_id to the routes sharing that structure.
fn build_structural_index(routes: &[RouteEntry]) -> HashMap<String, Vec<RouteEntry>> {
    let mut index: HashMap<String, Vec<RouteEntry>> = HashMap::new();
    for r in routes {
        index.entry(r.structural_id()).or_default().push(r.clone());
    }
    index
}

fn diff() -> Result<()> {
    let old_path = Path::new(".lunar").join("route-ast-actual.json");
    if !old_path.exists() {
        println!("No previous scan found. Run 'lunar scan' first.");
        return Ok(());
    }

    let old_content = fs::read_to_string(&old_path)?;
    let old_actual: ActualJson = serde_json::from_str(&old_content)?;
    let old_routes = old_actual.exposed;

    let new_routes = run_adapter()?;

    let old_index = build_structural_index(&old_routes);
    let new_index = build_structural_index(&new_routes);

    let mut all_structs: Vec<String> = old_index.keys().cloned().collect();
    for k in new_index.keys() {
        if !all_structs.contains(k) {
            all_structs.push(k.clone());
        }
    }
    all_structs.sort();

    let mut changes = Vec::new();

    for struct_id in &all_structs {
        let old_group = old_index.get(struct_id).cloned().unwrap_or_default();
        let new_group = new_index.get(struct_id).cloned().unwrap_or_default();

        if old_group.is_empty() {
            for nr in &new_group {
                changes.push(format!("  + {} {} (added)", nr.method, nr.display_path()));
            }
            continue;
        }
        if new_group.is_empty() {
            for or in &old_group {
                changes.push(format!("  - {} {} (removed)", or.method, or.display_path()));
            }
            continue;
        }

        // Both groups exist — perform pairwise comparison
        let mut new_matched: Vec<bool> = vec![false; new_group.len()];
        let mut old_matched: Vec<bool> = vec![false; old_group.len()];

        for (oi, or) in old_group.iter().enumerate() {
            for (ni, nr) in new_group.iter().enumerate() {
                if new_matched[ni] { continue; }
                let r = compare_routes(or, nr);
                match r {
                    DiffResult::Unchanged => {
                        old_matched[oi] = true;
                        new_matched[ni] = true;
                        break;
                    }
                    DiffResult::ParamNamesChanged { old_names, new_names } => {
                        changes.push(format!(
                            "  ~ {} {} (param names: {:?} → {:?})",
                            or.method, or.display_path(), old_names, new_names
                        ));
                        old_matched[oi] = true;
                        new_matched[ni] = true;
                        break;
                    }
                    DiffResult::MethodChanged { old_method, new_method } => {
                        changes.push(format!(
                            "  ~ {} {} → {} (method changed)",
                            old_method, or.display_path(), new_method
                        ));
                        old_matched[oi] = true;
                        new_matched[ni] = true;
                        break;
                    }
                    _ => {}
                }
            }
        }

        for (oi, or) in old_group.iter().enumerate() {
            if !old_matched[oi] {
                changes.push(format!("  - {} {} (removed)", or.method, or.display_path()));
            }
        }
        for (ni, nr) in new_group.iter().enumerate() {
            if !new_matched[ni] {
                changes.push(format!("  + {} {} (added)", nr.method, nr.display_path()));
            }
        }
    }

    if changes.is_empty() {
        println!("No changes detected.");
    } else {
        println!("Changes detected:");
        for line in &changes {
            println!("{}", line);
        }
    }
    Ok(())
}

// ---------- Scan command ----------

fn scan() -> Result<()> {
    println!("Scanning project...");
    let routes = run_adapter()?;
    println!("✓ Count verified: {} routes extracted", routes.len());

    let actual = ActualJson { exposed: routes, consumed: vec![] };
    let output_path = Path::new(".lunar").join("route-ast-actual.json");
    fs::create_dir_all(".lunar")?;
    fs::write(&output_path, serde_json::to_string_pretty(&actual)?)?;
    println!("✓ Wrote actual.json to {}", output_path.display());
    Ok(())
}

// ---------- Sync command ----------

fn sync(apply: bool, dry_run: bool) -> Result<()> {
    let interfaces_path = Path::new(".lunar").join("interfaces.yml");
    let backup_dir = Path::new(".lunar").join(".backup");
    let actual_path = Path::new(".lunar").join("route-ast-actual.json");

    if !actual_path.exists() {
        println!("No scan data found. Run 'lunar scan' first.");
        return Ok(());
    }

    let actual: ActualJson = serde_json::from_str(&fs::read_to_string(&actual_path)?)?;
    let new_exposed: Vec<InterfaceItem> = actual.exposed.iter().map(|r| InterfaceItem {
        path: r.to_path(), method: r.method.clone(), reason: None, target_project: None,
    }).collect();

    let mut interfaces: InterfacesYml = if interfaces_path.exists() {
        serde_yaml::from_str(&fs::read_to_string(&interfaces_path)?)?
    } else {
        InterfacesYml { project: None, project_type: None, environment: None, exposed: Some(Vec::new()), consumed: None }
    };

    if dry_run {
        println!("--- Dry run preview ---");
        if let Some(ref existing) = interfaces.exposed {
            let old_set: HashMap<String, &InterfaceItem> = existing.iter().map(|i| (format!("{} {}", i.method, i.path), i)).collect();
            let new_set: HashMap<String, &InterfaceItem> = new_exposed.iter().map(|i| (format!("{} {}", i.method, i.path), i)).collect();
            for k in old_set.keys() { if !new_set.contains_key(k) { println!("  - {} (would be removed)", k); } }
            for k in new_set.keys() { if !old_set.contains_key(k) { println!("  + {} (would be added)", k); } }
        }
        println!("--- End of preview ---");
        return Ok(());
    }

    if !apply {
        println!("No action taken. Use --apply to write changes, or --dry-run to preview.");
        return Ok(());
    }

    if interfaces_path.exists() {
        fs::create_dir_all(&backup_dir)?;
        let ts = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
        fs::copy(&interfaces_path, backup_dir.join(format!("interfaces.yml.bak.{}", ts)))?;
        println!("✓ Backup saved");
    }

    interfaces.exposed = Some(new_exposed);
    fs::write(&interfaces_path, serde_yaml::to_string(&interfaces)?)?;
    println!("✓ interfaces.yml updated with {} routes", actual.exposed.len());
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Scan => scan(),
        Commands::Diff => diff(),
        Commands::Sync { apply, dry_run } => sync(apply, dry_run),
    }
}
