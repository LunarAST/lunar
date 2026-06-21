use anyhow::anyhow;
use std::path::Path;
use lunar_interface::InterfacesYml;

pub fn apply_patch_yaml_at(base_path: &Path, patch_content: &str, create_backup: bool) -> anyhow::Result<()> {
    let interfaces_path = base_path.join(".lunar/interfaces.yml");
    let mut current: InterfacesYml = if interfaces_path.exists() {
        let raw = std::fs::read_to_string(&interfaces_path)?;
        serde_yaml::from_str(&raw)?
    } else {
        InterfacesYml::default()
    };

    // Parse patch content (must be a YAML fragment)
    let patch: serde_yaml::Value = serde_yaml::from_str(patch_content)
        .map_err(|e| anyhow!("Patch YAML parse error: {}. Ensure the patch is valid YAML.", e))?;

    // Apply patch: simplified merge strategy, overwrite existing exposed/consumed entries
    if let Some(exposed) = patch.get("exposed") {
        let new_exposed: Vec<lunar_interface::InterfaceItem> = serde_yaml::from_value(exposed.clone())
            .map_err(|e| anyhow!("Invalid 'exposed' format: {}. Each entry must have 'method' and 'path'.", e))?;
        current.exposed = Some(new_exposed);
    }
    if let Some(consumed) = patch.get("consumed") {
        let new_consumed: Vec<lunar_interface::InterfaceItem> = serde_yaml::from_value(consumed.clone())
            .map_err(|e| anyhow!("Invalid 'consumed' format: {}. Each entry must have 'method', 'path', and 'targetProject'.", e))?;
        current.consumed = Some(new_consumed);
    }

    // Create backup file
    if create_backup {
        let backup_dir = base_path.join(".lunar/.backup");
        std::fs::create_dir_all(&backup_dir)?;
        let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
        let backup_path = backup_dir.join(format!("interfaces.yml.bak.{}", ts));
        if interfaces_path.exists() {
            std::fs::copy(&interfaces_path, &backup_path)?;
        }
    }

    // Write updated config to file
    let new_content = serde_yaml::to_string(&current)?;
    std::fs::write(&interfaces_path, new_content)?;
    Ok(())
}

pub fn patch_cmd(_file: Option<String>) -> anyhow::Result<()> {
    // ... original logic ...
    Ok(())
}
