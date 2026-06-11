use lunar_interface::ActualJson;
use crate::adapter::find_adapter;
use crate::adapter::run_adapter;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

pub fn doctor_check() -> ExitCode {
    let mut issues = 0u8;
    let mut env_issues = 0u8;
    println!("🔍 LunarAST Doctor — Ecosystem Health Check\n");
    if Path::new("Cargo.toml").exists() { println!("✅ Project: Rust project detected (Cargo.toml)"); }
    else { println!("❌ Project: No Cargo.toml found"); env_issues += 1; issues += 1; }
    let config_path = Path::new(".lunar").join("config.yml");
    let adapter_name = "lunar-extract-rust";
    let _adapter_source = if config_path.exists() {
        if let Ok(config_content) = fs::read_to_string(&config_path) {
            if let Ok(config) = serde_yaml::from_str::<serde_yaml::Value>(&config_content) {
                if let Some(adapters) = config.get("adapters") {
                    if let Some(path) = adapters.get(adapter_name) {
                        if let Some(path_str) = path.as_str() {
                            if Path::new(path_str).exists() { println!("✅ Adapter: {} found at {} [Config Overridden]", adapter_name, path_str); None }
                            else { println!("❌ Adapter: Config override points to non-existent path: {}", path_str); env_issues += 1; issues += 1; Some("config override invalid".to_string()) }
                        } else { None }
                    } else { None }
                } else { None }
            } else { None }
        } else { None }
    } else { None };
    if _adapter_source.is_none() && !config_path.exists() {
        match find_adapter(adapter_name) {
            Some(path) => println!("✅ Adapter: {} found at {} [PATH]", adapter_name, path),
            None => { println!("❌ Adapter: {} not found in PATH", adapter_name); env_issues += 1; issues += 1; }
        }
    }
    if issues == 0 || env_issues == 0 {
        match run_adapter() {
            Ok(routes) => println!("✅ Adapter test: successfully extracted {} routes", routes.len()),
            Err(e) => { println!("❌ Adapter test: handshake failed — {}", e); env_issues += 1; issues += 1; }
        }
    }
    let autogen_path = Path::new(".lunar").join(".interfaces-autogen.json");
    if autogen_path.exists() { println!("✅ Scan data: .lunar/.interfaces-autogen.json exists"); }
    else { println!("❌ Scan data: .lunar/.interfaces-autogen.json missing"); env_issues += 1; issues += 1; }
    if autogen_path.exists() {
        if let Ok(content) = fs::read_to_string(&autogen_path) {
            if serde_json::from_str::<ActualJson>(&content).is_ok() { println!("✅ Data format: valid JSON with exposed/consumed fields"); }
            else { println!("❌ Data format: JSON corrupted or schema mismatch"); issues += 1; }
        }
    }
    let interfaces_path = Path::new(".lunar").join("interfaces.yml");
    if interfaces_path.exists() { println!("✅ Interfaces: .lunar/interfaces.yml exists"); }
    else { println!("⚠️  Interfaces: .lunar/interfaces.yml not found"); }
    println!();
    if issues == 0 { println!("🟢 All checks passed. Ecosystem is healthy."); ExitCode::from(0) }
    else if env_issues > 0 { println!("🔴 {} environment issue(s) found.", env_issues); ExitCode::from(1) }
    else { println!("🔴 {} data issue(s) found.", issues); ExitCode::from(2) }
}
