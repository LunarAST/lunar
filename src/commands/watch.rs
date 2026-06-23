use std::fs;
use std::path::Path;
use std::time::Duration;
use std::io::{self, Write};
use crate::types::TopographyMap;

/// Start a background watcher that polls all project suggestion directories.
/// When a new `.yaml` or `.yml` patch is discovered, a notification is printed
/// and a `.diff` file is generated for human or peer‑AI review.
///
/// This function never exits on its own; press Ctrl+C to stop.
pub async fn run() -> anyhow::Result<()> {
    let map_path = "lunar-map.json";
    if !Path::new(map_path).exists() {
        anyhow::bail!("lunar-map.json not found. Run 'lunar map' first.");
    }
    let map_content = fs::read_to_string(map_path)?;
    let map_val: TopographyMap = serde_json::from_str(&map_content)?;

    println!("👀 Watching for new AI patches... (press Ctrl+C to stop)\n");

    // Track already‑seen files to avoid duplicate notifications
    let mut known_files: std::collections::HashMap<String, Vec<String>> = Default::default();

    loop {
        for proj in &map_val.projects {
            if proj.name.is_empty() || proj.path.is_empty() {
                continue;
            }
            let suggest_dir = Path::new(&proj.path).join(".lunar/suggestions");
            if !suggest_dir.is_dir() {
                continue;
            }

            let entries = match fs::read_dir(&suggest_dir) {
                Ok(e) => e,
                Err(_) => continue,
            };

            let mut current_files = Vec::new();
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();

                // Accept both .yaml and .yml extensions
                let ext = path.extension().and_then(|e| e.to_str());
                if ext != Some("yaml") && ext != Some("yml") {
                    continue;
                }

                // Use lossy conversion to handle non‑UTF‑8 filenames gracefully
                let filename = path
                    .file_name()
                    .map(|f| f.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if filename.is_empty() {
                    continue;
                }

                // Skip already‑processed or generated files
                if filename.ends_with(".applied")
                    || filename.ends_with(".failed")
                    || filename.ends_with(".diff")
                {
                    continue;
                }

                current_files.push(filename.clone());

                let known = known_files.entry(proj.name.clone()).or_default();
                if !known.contains(&filename) {
                    // New patch detected – notify and generate diff
                    println!("🔔 New AI patch detected: {}", filename);
                    println!("   Project: {}", proj.name);
                    println!("   Path:    {:?}", path);

                    if let Ok(content) = fs::read_to_string(&path) {
                        // Extract the pure YAML portion even if the file is a LUNAR_PATCH block
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

                        let diff_path = suggest_dir.join(format!("{}.diff", filename));
                        let interfaces_path = Path::new(&proj.path).join(".lunar/interfaces.yml");
                        let current_yml = if interfaces_path.exists() {
                            fs::read_to_string(&interfaces_path).unwrap_or_default()
                        } else {
                            "(no existing interfaces.yml)".to_string()
                        };

                        let mut diff_content = String::new();
                        diff_content.push_str("# AI Patch Review\n\n");
                        diff_content.push_str("## Current interfaces.yml\n```yaml\n");
                        diff_content.push_str(&current_yml);
                        diff_content.push_str("\n```\n\n## Proposed Patch\n```yaml\n");
                        diff_content.push_str(&patch_yaml);
                        diff_content.push_str("\n```\n");

                        // Do NOT abort the watcher on write errors – just log a warning
                        match fs::write(&diff_path, diff_content) {
                            Ok(()) => {
                                println!("   Diff saved: {:?}", diff_path);
                                println!("   Share the diff file with another AI for peer review.\n");
                            }
                            Err(e) => {
                                eprintln!("   ⚠️  Failed to write diff file for {}: {}\n", filename, e);
                            }
                        }
                    }
                    known.push(filename);
                }
            }
            // Refresh known file list for this project (prune deleted files)
            *known_files.entry(proj.name.clone()).or_default() = current_files;
        }
        io::stdout().flush().ok();
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}
