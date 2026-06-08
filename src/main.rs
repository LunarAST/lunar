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

    fn key(&self) -> String {
        format!("{} {}", self.method, self.to_path())
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

    if !has_changes {
        println!("No changes detected.");
    }

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

    // Load current scan data
    let actual_content = fs::read_to_string(&actual_path)?;
    let actual: ActualJson = serde_json::from_str(&actual_content)?;
    let scanned_routes = actual.exposed;

    // Load or create interfaces.yml
    let mut interfaces: InterfacesYml = if interfaces_path.exists() {
        let yaml_content = fs::read_to_string(&interfaces_path)?;
        serde_yaml::from_str(&yaml_content)?
    } else {
        InterfacesYml {
            project: None,
            project_type: None,
            environment: None,
            exposed: Some(Vec::new()),
            consumed: None,
        }
    };

    // Merge scanned routes into exposed list
    let new_exposed: Vec<InterfaceItem> = scanned_routes
        .iter()
        .map(|r| InterfaceItem {
            path: r.to_path(),
            method: r.method.clone(),
            reason: None,
            target_project: None,
        })
        .collect();

    if dry_run {
        println!("--- Dry run preview ---");
        if let Some(ref existing) = interfaces.exposed {
            let old_map: HashMap<String, &InterfaceItem> = existing
                .iter()
                .map(|i| (format!("{} {}", i.method, i.path), i))
                .collect();
            let new_map: HashMap<String, &InterfaceItem> = new_exposed
                .iter()
                .map(|i| (format!("{} {}", i.method, i.path), i))
                .collect();

            for key in old_map.keys() {
                if !new_map.contains_key(key) {
                    println!("  - {} (would be removed from interfaces.yml)", key);
                }
            }
            for key in new_map.keys() {
                if !old_map.contains_key(key) {
                    println!("  + {} (would be added to interfaces.yml)", key);
                }
            }
        } else {
            println!("  All {} routes would be added to interfaces.yml", new_exposed.len());
        }
        println!("--- End of preview ---");
        return Ok(());
    }

    if !apply {
        println!("No action taken. Use --apply to write changes, or --dry-run to preview.");
        return Ok(());
    }

    // Backup old interfaces.yml if it exists
    if interfaces_path.exists() {
        fs::create_dir_all(&backup_dir)?;
        let timestamp = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
        let backup_path = backup_dir.join(format!("interfaces.yml.bak.{}", timestamp));
        fs::copy(&interfaces_path, &backup_path)?;
        println!("✓ Backup saved to {}", backup_path.display());
    }

    // Apply changes
    interfaces.exposed = Some(new_exposed);

    let new_yaml = serde_yaml::to_string(&interfaces)?;
    fs::write(&interfaces_path, new_yaml)?;

    println!("✓ interfaces.yml updated with {} exposed routes", scanned_routes.len());
    Ok(())
}

// ---------- Main ----------

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Scan => scan()?,
        Commands::Diff => diff()?,
        Commands::Sync { apply, dry_run } => sync(apply, dry_run)?,
    }
    Ok(())
}
