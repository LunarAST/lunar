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

/// [MODIFIED] Multi-Language Base Sniffer.
/// If no Rust compiler is found, gracefully downgrade to write a dummy facts file instead of crashing.
pub fn run_adapter() -> Result<Vec<RouteEntry>> {
    let project_dir = std::env::current_dir()?;
    let project_dir_str = project_dir.to_string_lossy().to_string();
    
    // Sniff main files on disk to determine workspace language
    let has_cargo = project_dir.join("Cargo.toml").exists();
    let has_python = project_dir.join("requirements.txt").exists() 
        || project_dir.join("pyproject.toml").exists() 
        || project_dir.join("Pipfile").exists();
    let has_go = project_dir.join("go.mod").exists();
    let has_node = project_dir.join("package.json").exists();
    let has_nginx = project_dir.join("nginx.conf").exists();

    if has_cargo {
        let adapter_name = "lunar-extract-rust";
        let adapter_path = find_adapter(adapter_name)
            .ok_or_else(|| anyhow::anyhow!("Adapter '{}' not found in PATH.", adapter_name))?;
        
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
    } else if has_python || has_go || has_node || has_nginx {
        let lang = if has_python { "Python" } else if has_go { "Go" } else if has_node { "Node.js" } else { "Nginx" };
        println!("⚠️  Main language detected: {}. AST adapter is not yet compiled or installed in PATH.", lang);
        println!("👉 LunarAST will automatically downgrade to 'Self-Growing Intent-Only mode' for this workspace.");
        
        // Write a dummy facts file so scan succeeds and map can register this directory!
        let actual = serde_json::json!({
            "exposed": [],
            "consumed": [],
            "projectType": lang.to_lowercase()
        });
        let output_path = Path::new(".lunar").join(".interfaces-autogen.json");
        std::fs::create_dir_all(".lunar")?;
        std::fs::write(&output_path, serde_json::to_string_pretty(&actual)?)?;
        
        // Return empty list so CLI scan doesn't crash
        Ok(vec![])
    } else {
        anyhow::bail!("Could not detect project language. Expected standard project files (Cargo.toml, requirements.txt, go.mod, etc.)");
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
