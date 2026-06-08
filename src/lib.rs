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

/// Compare two routes at the same structural position.
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

/// Build a structural index from routes.
pub fn build_structural_index(routes: &[RouteEntry]) -> HashMap<String, Vec<RouteEntry>> {
    let mut index: HashMap<String, Vec<RouteEntry>> = HashMap::new();
    for r in routes {
        index.entry(r.structural_id()).or_default().push(r.clone());
    }
    index
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
