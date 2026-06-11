use anyhow::Result;
use lunar_interface::RouteEntry;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;
use wait_timeout::ChildExt;

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

pub fn run_adapter() -> Result<Vec<RouteEntry>> {
    let project_dir = std::env::current_dir()?;
    let project_dir_str = project_dir.to_string_lossy().to_string();
    let adapter_name = if project_dir.join("Cargo.toml").exists() { "lunar-extract-rust" }
    else { anyhow::bail!("Could not detect project language. Currently supported: Rust (Cargo.toml found)."); };
    let adapter_path = find_adapter(adapter_name).ok_or_else(|| anyhow::anyhow!("Adapter '{}' not found in PATH.", adapter_name))?;
    let mut child = Command::new(&adapter_path).arg(&project_dir_str).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
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
        None => { child.kill()?; child.wait()?; anyhow::bail!("Adapter '{}' timed out after 30 seconds", adapter_name); }
    }
}

pub fn parse_routes(ldjson: &str) -> Result<Vec<RouteEntry>> {
    let mut routes = Vec::new();
    let mut count_from_marker: Option<usize> = None;
    for line in ldjson.lines() {
        if line.is_empty() { continue; }
        if line.contains("\"_lunar\"") {
            let marker: serde_json::Value = serde_json::from_str(line)?;
            if marker["_lunar"]["status"] == "success" { count_from_marker = Some(marker["_lunar"]["count"].as_u64().unwrap() as usize); }
            else if marker["_lunar"]["status"] == "error" { let msg = marker["_lunar"]["message"].as_str().unwrap_or("unknown error"); anyhow::bail!("Adapter reported error: {}", msg); }
            continue;
        }
        routes.push(serde_json::from_str(line)?);
    }
    if let Some(expected) = count_from_marker { if routes.len() != expected { anyhow::bail!("Count mismatch: expected {}, got {}", expected, routes.len()); } }
    else { anyhow::bail!("No end marker found"); }
    Ok(routes)
}
