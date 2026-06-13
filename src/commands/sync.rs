use anyhow::Result;
use chrono::Utc;
use std::fs;
use std::path::Path;
use lunar_interface::{ActualJson, InterfacesYml, InterfaceItem};

pub fn execute(apply: bool, dry_run: bool) -> Result<()> {
    let interfaces_path = Path::new(".lunar").join("interfaces.yml");
    let backup_dir = Path::new(".lunar").join(".backup");
    let suggestions_dir = Path::new(".lunar").join("suggestions");
    let actual_path = Path::new(".lunar").join(".interfaces-autogen.json");
    if !actual_path.exists() { println!("No scan data found. Run 'lunar scan' first."); return Ok(()); }
    let actual: ActualJson = serde_json::from_str(&fs::read_to_string(&actual_path)?)?;
    let new_exposed: Vec<InterfaceItem> = actual.exposed.iter().map(|r| InterfaceItem {
        path: r.to_path(), method: r.method.clone(), reason: None, target_project: None,
    }).collect();
    let mut interfaces: InterfacesYml = if interfaces_path.exists() {
        serde_yaml::from_str(&fs::read_to_string(&interfaces_path)?)?
    } else {
        InterfacesYml { project: None, project_type: None, environment: None, exposed: Some(Vec::new()), consumed: None }
    };
    if let Some(ref mut existing) = interfaces.exposed {
        for item in &new_exposed { if !existing.iter().any(|e| e.path == item.path && e.method == item.method) { existing.push(item.clone()); } }
    } else { interfaces.exposed = Some(new_exposed.clone()); }
    if suggestions_dir.is_dir() {
        let mut entries: Vec<_> = fs::read_dir(&suggestions_dir)?.filter_map(|e| e.ok()).filter(|e| e.path().extension().map_or(false, |ext| ext == "yaml" || ext == "yml")).collect();
        entries.sort_by_key(|e| e.file_name());
        if !entries.is_empty() {
            println!("Found {} AI/human suggestion(s) to merge.", entries.len());
            for entry in &entries {
                let path = entry.path();
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(suggestion) = serde_yaml::from_str::<InterfacesYml>(&content) {
                        if let Some(sug_exposed) = suggestion.exposed {
                            let existing = interfaces.exposed.get_or_insert_with(Vec::new);
                            for item in &sug_exposed {
                                if let Some(ei) = existing.iter_mut().find(|e| e.path == item.path && e.method == item.method) {
                                    if item.reason.is_some() { ei.reason = item.reason.clone(); }
                                    if item.target_project.is_some() { ei.target_project = item.target_project.clone(); }
                                } else { existing.push(item.clone()); }
                            }
                        }
                        if let Some(sug_consumed) = suggestion.consumed {
                            let existing = interfaces.consumed.get_or_insert_with(Vec::new);
                            for item in &sug_consumed {
                                if let Some(ei) = existing.iter_mut().find(|e| e.path == item.path && e.method == item.method) {
                                    if item.reason.is_some() { ei.reason = item.reason.clone(); }
                                    if item.target_project.is_some() { ei.target_project = item.target_project.clone(); }
                                } else { existing.push(item.clone()); }
                            }
                        }
                        let new_path = path.with_extension("yaml.applied");
                        fs::rename(&path, &new_path)?;
                    }
                }
            }
            println!("Suggestions processed.");
        }
    }
    if dry_run {
        println!("--- Dry run preview ---");
        if let Some(ref existing) = interfaces.exposed { for item in existing { println!("  E: {} {}", item.method, item.path); } }
        if let Some(ref existing) = interfaces.consumed { for item in existing { println!("  C: {} {} -> {}", item.method, item.path, item.target_project.as_deref().unwrap_or("?")); } }
        println!("--- End of preview ---");
        return Ok(());
    }
    if !apply { println!("No action taken."); return Ok(()); }
    if interfaces_path.exists() {
        fs::create_dir_all(&backup_dir)?;
        let ts = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
        fs::copy(&interfaces_path, backup_dir.join(format!("interfaces.yml.bak.{}", ts)))?;
        println!("✓ Backup saved");
    }
    fs::write(&interfaces_path, serde_yaml::to_string(&interfaces)?)?;
    println!("✓ interfaces.yml updated");
    Ok(())
}
