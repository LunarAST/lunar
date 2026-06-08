use anyhow::Result;
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

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
    /// Convert segments to standard path string: /literal/{param}/*
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

    /// Unique key for comparison: method + path
    fn key(&self) -> String {
        format!("{} {}", self.method, self.to_path())
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ActualJson {
    exposed: Vec<RouteEntry>,
    consumed: Vec<RouteEntry>,
}

// ---------- Mock adapter outputs ----------

fn mock_adapter_output() -> String {
    // Simulates scanning the original project
    r#"{"method":"GET","segments":[{"type":"literal","value":"healthz"}],"source_file":"src/main.rs","line_number":10,"extraction_method":"ast"}
{"method":"POST","segments":[{"type":"literal","value":"api"},{"type":"literal","value":"v1"},{"type":"parameter","name":"userId","raw_constraint":"\\d+"}],"source_file":"src/main.rs","line_number":20,"extraction_method":"ast"}
{"_lunar":{"status":"success","count":2}}
"#
    .to_string()
}

fn mock_modified_output() -> String {
    // Simulates the project after some code changes:
    // - removed GET /healthz
    // - added GET /api/v2/orders
    // - changed POST /api/v1/{userId} to PUT /api/v1/{userId}
    r#"{"method":"PUT","segments":[{"type":"literal","value":"api"},{"type":"literal","value":"v1"},{"type":"parameter","name":"userId","raw_constraint":"\\d+"}],"source_file":"src/main.rs","line_number":20,"extraction_method":"ast"}
{"method":"GET","segments":[{"type":"literal","value":"api"},{"type":"literal","value":"v2"},{"type":"literal","value":"orders"}],"source_file":"src/main.rs","line_number":30,"extraction_method":"ast"}
{"_lunar":{"status":"success","count":2}}
"#
    .to_string()
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
    let ldjson_output = mock_adapter_output();
    let routes = parse_routes(&ldjson_output)?;

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

    // Load old routes
    let old_content = fs::read_to_string(&old_path)?;
    let old_actual: ActualJson = serde_json::from_str(&old_content)?;
    let old_routes = old_actual.exposed;

    // Simulate new scan (after code changes)
    let new_ldjson = mock_modified_output();
    let new_routes = parse_routes(&new_ldjson)?;

    // Build maps by key (method + path)
    let old_map: HashMap<String, RouteEntry> = old_routes
        .into_iter()
        .map(|r| (r.key(), r))
        .collect();
    let new_map: HashMap<String, RouteEntry> = new_routes
        .into_iter()
        .map(|r| (r.key(), r))
        .collect();

    let mut has_changes = false;

    // Find removed (in old but not in new)
    for key in old_map.keys() {
        if !new_map.contains_key(key) {
            if !has_changes {
                println!("Changes detected:");
                has_changes = true;
            }
            println!("  - {}  (removed)", key);
        }
    }

    // Find added (in new but not in old)
    for key in new_map.keys() {
        if !old_map.contains_key(key) {
            if !has_changes {
                println!("Changes detected:");
                has_changes = true;
            }
            println!("  + {}  (added)", key);
        }
    }

    // Find modified (same key, different segments)
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
