use std::fs;
use std::path::Path;
use std::process::Command;

pub async fn run() -> anyhow::Result<()> {
    println!("🔄 Syncing repository metadata from local .git directories...\n");

    let map_path = "lunar-map.json";
    if !Path::new(map_path).exists() {
        anyhow::bail!("lunar-map.json not found. Run 'lunar scan' first.");
    }
    let map_content = fs::read_to_string(map_path)?;
    let map: serde_json::Value = serde_json::from_str(&map_content)?;
    let projects = map["projects"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("lunar-map.json projects array missing"))?;

    let repos_path = "repos.json";
    let mut repos: serde_json::Value = if Path::new(repos_path).exists() {
        let content = fs::read_to_string(repos_path)?;
        serde_json::from_str(&content)?
    } else {
        serde_json::json!({
            "version": "0.5.0",
            "projects": []
        })
    };
    let repos_projects = repos["projects"]
        .as_array_mut()
        .ok_or_else(|| anyhow::anyhow!("repos.json projects array missing"))?;

    for proj in projects {
        let name = proj["name"].as_str().unwrap_or("");
        let path_str = proj["path"].as_str().unwrap_or("");
        if name.is_empty() || path_str.is_empty() {
            continue;
        }
        let proj_dir = Path::new(path_str);
        if !proj_dir.join(".git").exists() {
            println!("  ⚠ {} : not a Git repository, skipping", name);
            continue;
        }

        let remote_url = match Command::new("git")
            .args(["-C", path_str, "config", "--get", "remote.origin.url"])
            .output()
        {
            Ok(output) if output.status.success() => {
                String::from_utf8_lossy(&output.stdout).trim().to_string()
            }
            _ => {
                println!("  ⚠ {} : unable to read remote URL, skipping", name);
                continue;
            }
        };

        let (owner, repo) = if remote_url.contains("github.com") {
            let parts: Vec<&str> = remote_url.trim_end_matches('/').split('/').collect();
            if parts.len() >= 2 {
                let repo_part = if parts.last().unwrap().ends_with(".git") {
                    &parts.last().unwrap()[..parts.last().unwrap().len()-4]
                } else {
                    parts.last().unwrap()
                };
                (parts[parts.len()-2].to_string(), repo_part.to_string())
            } else {
                continue;
            }
        } else {
            continue;
        };

        let branch = match Command::new("git")
            .args(["-C", path_str, "rev-parse", "--abbrev-ref", "HEAD"])
            .output()
        {
            Ok(output) if output.status.success() => {
                String::from_utf8_lossy(&output.stdout).trim().to_string()
            }
            _ => "main".to_string(),
        };

        let entry = repos_projects.iter_mut().find(|e| {
            e.get("name").and_then(|n| n.as_str()) == Some(name)
        });
        if let Some(entry) = entry {
            entry["source"] = serde_json::json!({
                "type": "github",
                "github": {
                    "owner": owner,
                    "repo": repo,
                    "branch": branch
                }
            });
            println!("  ✓ {} : {}/{} ({})", name, owner, repo, branch);
        } else {
            repos_projects.push(serde_json::json!({
                "name": name,
                "path": path_str,
                "visibility": "public",
                "source": {
                    "type": "github",
                    "github": {
                        "owner": owner,
                        "repo": repo,
                        "branch": branch
                    }
                }
            }));
            println!("  + {} : {}/{} ({})", name, owner, repo, branch);
        }
    }

    fs::write(repos_path, serde_json::to_string_pretty(&repos)?)?;
    println!("\n✅ repos.json updated from local Git data.");
    Ok(())
}
