use anyhow::Result;
use lunar_interface::InterfacesYml;
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

fn load_known_projects(base_path: &Path) -> Vec<String> {
    let candidates: Vec<std::path::PathBuf> = vec![
        base_path.join("repos.json"),
        base_path.join(".lunar/repos.json"),
    ];
    for path in candidates {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(arr) = json.get("projects").and_then(|p| p.as_array()) {
                        return arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
                    }
                }
            }
        }
    }
    Vec::new()
}

pub fn apply_patch_yaml(yaml_str: &str) -> Result<()> {
    apply_patch_yaml_at(Path::new("."), yaml_str, false)
}

pub fn apply_patch_yaml_at(base_path: &Path, yaml_str: &str, force: bool) -> Result<()> {
    let interfaces_path = base_path.join(".lunar/interfaces.yml");
    let backup_dir = base_path.join(".lunar/.backup");

    let patch: InterfacesYml = serde_yaml::from_str(yaml_str)
        .map_err(|e| anyhow::anyhow!("Invalid YAML patch: {}", e))?;

    let known_projects = load_known_projects(base_path);
    if let Some(ref consumed) = patch.consumed {
        for item in consumed {
            if let Some(ref target) = item.target_project {
                if !known_projects.is_empty() && !known_projects.iter().any(|p| p == target) {
                    eprintln!("⚠️  Warning: targetProject '{}' is not in the known project list.", target);
                    eprintln!("   Known projects: {:?}", known_projects);
                    eprintln!("   If this is a new project, add it to repos.json first.");
                }
            }
        }
    }

    let mut interfaces: InterfacesYml = if interfaces_path.exists() {
        serde_yaml::from_str(&fs::read_to_string(&interfaces_path)?)?
    } else {
        InterfacesYml { project: None, project_type: None, environment: None, exposed: Some(Vec::new()), consumed: None }
    };

    if let Some(p_exposed) = patch.exposed {
        let existing = interfaces.exposed.get_or_insert_with(Vec::new);
        for item in &p_exposed {
            if let Some(ei) = existing.iter_mut().find(|e| e.path == item.path && e.method == item.method) {
                if item.reason.is_some() { ei.reason = item.reason.clone(); }
                if item.target_project.is_some() { ei.target_project = item.target_project.clone(); }
            } else { existing.push(item.clone()); }
        }
    }
    if let Some(p_consumed) = patch.consumed {
        let existing = interfaces.consumed.get_or_insert_with(Vec::new);
        for item in &p_consumed {
            if let Some(ei) = existing.iter_mut().find(|e| e.path == item.path && e.method == item.method) {
                if item.reason.is_some() { ei.reason = item.reason.clone(); }
                if item.target_project.is_some() { ei.target_project = item.target_project.clone(); }
            } else { existing.push(item.clone()); }
        }
    }

    if patch.project_type.is_some() {
        interfaces.project_type = patch.project_type.clone();
    }
    if patch.project.is_some() {
        interfaces.project = patch.project.clone();
    }

    println!("Changes to be applied:");
    if let Some(ref exposed) = interfaces.exposed { for item in exposed { println!("  E: {} {}", item.method, item.path); } }
    if let Some(ref consumed) = interfaces.consumed { for item in consumed { println!("  C: {} {} -> {}", item.method, item.path, item.target_project.as_deref().unwrap_or("?")); } }
    if let Some(ref pt) = interfaces.project_type { println!("  Project type: {}", pt); }

    if !force {
        print!("Proceed with merge? [y/N] ");
        io::stdout().flush()?;
        let mut input = String::new();
        if let Ok(mut tty) = std::fs::File::open("/dev/tty") {
            use std::io::BufRead;
            let mut reader = std::io::BufReader::new(&mut tty);
            reader.read_line(&mut input)?;
        } else {
            io::stdin().read_line(&mut input)?;
        }
        if input.trim().to_lowercase() != "y" && input.trim().to_lowercase() != "yes" {
            println!("Merge cancelled.");
            return Ok(());
        }
    }

    if interfaces_path.exists() {
        fs::create_dir_all(&backup_dir)?;
        let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
        fs::copy(&interfaces_path, backup_dir.join(format!("interfaces.yml.bak.{}", ts)))?;
        println!("✓ Backup saved");
    }
    fs::write(&interfaces_path, serde_yaml::to_string(&interfaces)?)?;
    println!("✓ interfaces.yml updated");
    Ok(())
}

pub fn patch_cmd(file: Option<String>) -> Result<()> {
    let yaml_str = if let Some(path_str) = file {
        fs::read_to_string(&path_str)?
    } else {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        if buf.trim().is_empty() {
            println!("No input provided. Usage:");
            println!("  lunar patch path/to/file.yaml");
            println!("  cat patch.yaml | lunar patch");
            return Ok(());
        }
        buf
    };
    apply_patch_yaml(&yaml_str)
}
