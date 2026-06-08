use anyhow::Result;
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

    fn key(&self) -> String {
        format!("{} {}", self.method, self.to_path())
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ActualJson {
    exposed: Vec<RouteEntry>,
    consumed: Vec<RouteEntry>,
}

// ---------- Adapter process management ----------

/// Find an adapter binary by name in PATH.
fn find_adapter(name: &str) -> Option<String> {
    // First, check next to the current executable (development convenience)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let local_path = exe_dir.join(name);
            if local_path.exists() {
                return Some(local_path.to_string_lossy().to_string());
            }
        }
    }
    // Then, check PATH
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

/// Run the adapter for the current project and return the parsed routes.
fn run_adapter() -> Result<Vec<RouteEntry>> {
    let project_dir = std::env::current_dir()?;
    let project_dir_str = project_dir.to_string_lossy().to_string();

    // Detect project language and find appropriate adapter.
    // For now, we check for Cargo.toml to detect Rust projects.
    // Future: support other languages via config or auto-detection.
    let adapter_name = if project_dir.join("Cargo.toml").exists() {
        "lunar-extract-rust"
    } else {
        anyhow::bail!(
            "Could not detect project language. Currently supported: Rust (Cargo.toml found)."
        );
    };

    let adapter_path = find_adapter(adapter_name).ok_or_else(|| {
        anyhow::anyhow!(
            "Adapter '{}' not found in PATH. Install it first.\n\
             For Rust/Axum: cargo install lunar-extract-rust\n\
             Or build from source: https://github.com/LunarAST/lunar-extract-rust",
            adapter_name
        )
    })?;

    // Spawn adapter subprocess
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

// ---------- Core: parse LDJSON output into routes ----------

fn parse_routes(ldjson: &str) -> Result<Vec<RouteEntry>> {
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

// ---------- Scan command ----------

fn scan() -> Result<()> {
    println!("Scanning project...");
    let routes = run_adapter()?;
    println!("✓ Count verified: {} routes extracted", routes.len());

    let actual = ActualJson {
        exposed: routes,
        consumed: vec![],
    };

    let output_path = Path::new(".lunar").join("route-ast-actual.json");
    fs::create_dir_all(".lunar")?;
    let json_string = serde_json::to_string_pretty(&actual)?;
    fs::write(&output_path, json_string)?;

    println!("✓ Wrote actual.json to {}", output_path.display());
    Ok(())
}

// ---------- Diff command ----------

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

    let old_map: HashMap<String, RouteEntry> = old_routes
        .into_iter()
        .map(|r| (r.key(), r))
        .collect();
    let new_map: HashMap<String, RouteEntry> = new_routes
        .into_iter()
        .map(|r| (r.key(), r))
        .collect();

    let mut has_changes = false;

    for key in old_map.keys() {
        if !new_map.contains_key(key) {
            if !has_changes {
                println!("Changes detected:");
                has_changes = true;
            }
            println!("  - {}  (removed)", key);
        }
    }

    for key in new_map.keys() {
        if !old_map.contains_key(key) {
            if !has_changes {
                println!("Changes detected:");
                has_changes = true;
            }
            println!("  + {}  (added)", key);
        }
    }

    for key in old_map.keys() {
        if let (Some(old), Some(new)) = (old_map.get(key), new_map.get(key)) {
            if old.segments != new.segments {
                if !has_changes {
                    println!("Changes detected:");
                    has_changes = true;
                }
                println!("  ~ {}  (modified segments)", key);
            }
        }
    }

    if !has_changes {
        println!("No changes detected.");
    }

    Ok(())
}

// ---------- Main ----------

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Scan => scan()?,
        Commands::Diff => diff()?,
    }
    Ok(())
}
