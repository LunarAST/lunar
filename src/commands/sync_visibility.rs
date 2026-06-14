use std::fs;
use serde_json::Value;

pub async fn run() -> anyhow::Result<()> {
    let repos_path = "repos.json";
    if !std::path::Path::new(repos_path).exists() {
        println!("No repos.json found. Nothing to sync.");
        return Ok(());
    }

    let content = fs::read_to_string(repos_path)?;
    let mut config: Value = serde_json::from_str(&content)?;
    let projects = config
        .get_mut("projects")
        .and_then(|p| p.as_array_mut())
        .ok_or_else(|| anyhow::anyhow!("Invalid repos.json structure"))?;

    let token = std::env::var("GITHUB_TOKEN").ok();
    if token.is_none() {
        println!("Note: GITHUB_TOKEN not set. Will skip GitHub API checks.");
    }

    let client = reqwest::Client::new();

    // Collect the pairs of project indices and github coordinates first, then update later
    let mut updates: Vec<(usize, String, String)> = Vec::new();

    for (idx, proj) in projects.iter().enumerate() {
        let source = proj.get("source");
        let github = source.and_then(|s| s.get("github"));
        if let Some(github) = github {
            let owner = github.get("owner").and_then(|v| v.as_str()).unwrap_or("");
            let repo = github.get("repo").and_then(|v| v.as_str()).unwrap_or("");
            if !owner.is_empty() && !repo.is_empty() {
                updates.push((idx, owner.to_string(), repo.to_string()));
            }
        }
    }

    for (idx, owner, repo) in updates {
        if let Some(ref t) = token {
            let url = format!("https://api.github.com/repos/{}/{}", owner, repo);
            match client.get(&url)
                .header("Authorization", format!("Bearer {}", t))
                .header("User-Agent", "LunarAST")
                .send()
                .await
            {
                Ok(resp) => {
                    if resp.status().is_success() {
                        if let Ok(json) = resp.json::<Value>().await {
                            let is_private = json.get("private").and_then(|v| v.as_bool()).unwrap_or(false);
                            let visibility = if is_private { "private" } else { "public" };
                            projects[idx]["visibility"] = Value::String(visibility.to_string());
                            println!("  ✓ {}/{} → {}", owner, repo, visibility);
                        }
                    } else {
                        eprintln!("  ⚠ Could not fetch {}/{}, status: {}", owner, repo, resp.status());
                    }
                }
                Err(e) => {
                    eprintln!("  ✗ Request failed for {}/{}: {}", owner, repo, e);
                }
            }
        } else {
            println!("  ? {}/{} – skipped (no GITHUB_TOKEN)", owner, repo);
        }
    }

    fs::write(repos_path, serde_json::to_string_pretty(&config)?)?;
    println!("\nVisibility synced from GitHub. Restart lunar-serve to apply changes.");
    Ok(())
}
