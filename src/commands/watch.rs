use std::fs;
use std::path::Path;
use std::time::Duration;
use std::io::{self, Write};
use serde::Deserialize;

#[derive(Deserialize, Default)]
struct TopographyMap {
    #[serde(default)]
    projects: Vec<ProjectMeta>,
}

#[derive(Deserialize, Default)]
struct ProjectMeta {
    #[serde(default)]
    name: String,
    #[serde(default)]
    path: String,
}

pub async fn run() -> anyhow::Result<()> {
    let map_path = "lunar-map.json";
    if !Path::new(map_path).exists() {
        anyhow::bail!("lunar-map.json not found. Run 'lunar map' first.");
    }
    let map_content = fs::read_to_string(map_path)?;
    let map_val: TopographyMap = serde_json::from_str(&map_content)?;

    println!("👀 Watching for new AI patches... (press Ctrl+C to stop)\n");

    // Track known filenames per project to avoid re-notifying
    let mut known_files: std::collections::HashMap<String, Vec<String>> = Default::default();

    loop {
        for proj in &map_val.projects {
            if proj.name.is_empty() || proj.path.is_empty() { continue; }
            let suggest_dir = Path::new(&proj.path).join(".lunar/suggestions");
            if !suggest_dir.is_dir() { continue; }
            let entries = match fs::read_dir(&suggest_dir) {
                Ok(e) => e,
                Err(_) => continue,
            };
            let mut current_files = Vec::new();
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("yaml") { continue; }
                let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
                if filename.ends_with(".applied") || filename.ends_with(".failed") || filename.ends_with(".diff") {
                    continue;
                }
                current_files.push(filename.to_string());

                let known = known_files.entry(proj.name.clone()).or_default();
                if !known.contains(&filename.to_string()) {
                    // New patch found! Notify and generate diff.
                    println!("🔔 New AI patch detected: {}", filename);
                    println!("   Project: {}", proj.name);
                    println!("   Path:    {:?}", path);
                    // Generate diff by reading patch content and comparing with current interfaces
                    if let Ok(content) = fs::read_to_string(&path) {
                        // Extract patch YAML (in case of LUNAR_PATCH format)
                        let patch_yaml = if let Some(start) = content.find("---CONTENT---") {
                            let after = &content[start + "---CONTENT---".len()..];
                            if let Some(end) = after.find("---LUNAR_PATCH_END---") {
                                after[..end].trim().to_string()
                            } else {
                                content.clone()
                            }
                        } else {
                            content.clone()
                        };
                        // Write diff summary to .diff file for human/peer AI review
                        let diff_path = suggest_dir.join(format!("{}.diff", filename));
                        let interfaces_path = Path::new(&proj.path).join(".lunar/interfaces.yml");
                        let current_yml = if interfaces_path.exists() {
                            fs::read_to_string(&interfaces_path).unwrap_or_default()
                        } else {
                            "(no existing interfaces.yml)".to_string()
                        };
                        let diff_content = format!(
                            "# AI Patch Review\n\n## Current interfaces.yml\n```yaml\n{}\n```\n\n## Proposed Patch\n```yaml\n{}\n```\n",
                            current_yml, patch_yaml
                        );
                        fs::write(&diff_path, diff_content)?;
                        println!("   Diff saved: {:?}", diff_path);
                        println!("   Share the diff file with another AI for peer review.");
                        println!();
                    }
                    known.push(filename.to_string());
                }
            }
            // Update known files list (prune deleted files)
            *known_files.entry(proj.name.clone()).or_default() = current_files;
        }
        io::stdout().flush().ok();
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}
