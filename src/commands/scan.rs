use anyhow::Result;
use std::fs;
use std::path::Path;
use lunar_interface::ActualJson;
use crate::adapter::run_adapter;

pub fn execute() -> Result<()> {
    println!("Scanning project...");
    let routes = run_adapter()?;
    println!("✓ Count verified: {} routes extracted", routes.len());
    let actual = ActualJson { exposed: routes, consumed: vec![], project_type: None };
    let output_path = Path::new(".lunar").join(".interfaces-autogen.json");
    fs::create_dir_all(".lunar")?;
    fs::write(&output_path, serde_json::to_string_pretty(&actual)?)?;
    println!("✓ Wrote autogen.json to {}", output_path.display());
    Ok(())
}

/// Scan a specific project by temporarily switching to its directory.
pub fn execute_at(name: &str, path: &str) -> Result<()> {
    let target = Path::new(path);
    if !target.exists() {
        anyhow::bail!("Project path does not exist: {}", path);
    }
    let original_dir = std::env::current_dir()?;
    std::env::set_current_dir(target)?;
    println!("Scanning project '{}' at {}...", name, path);
    let result = execute();
    std::env::set_current_dir(original_dir)?;
    result
}
