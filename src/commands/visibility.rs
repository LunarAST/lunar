use std::fs;
use std::io::{self, Write};
use serde_json::Value;

fn load_repos() -> anyhow::Result<Value> {
    let repos_path = "repos.json";
    if !std::path::Path::new(repos_path).exists() {
        let default = serde_json::json!({
            "version": "0.5.0",
            "projects": []
        });
        fs::write(repos_path, serde_json::to_string_pretty(&default)?)?;
    }
    let content = fs::read_to_string(repos_path)?;
    Ok(serde_json::from_str(&content)?)
}

fn save_repos(config: &Value) -> anyhow::Result<()> {
    fs::write("repos.json", serde_json::to_string_pretty(config)?)?;
    Ok(())
}

pub fn set_all(visibility: &str) -> anyhow::Result<()> {
    let mut config = load_repos()?;
    let projects = config
        .get_mut("projects")
        .and_then(|p| p.as_array_mut())
        .ok_or_else(|| anyhow::anyhow!("Invalid repos.json structure"))?;

    for proj in projects.iter_mut() {
        proj["visibility"] = Value::String(visibility.to_string());
    }
    save_repos(&config)?;
    println!("✓ All projects set to {}.", visibility);
    println!("Restart lunar-serve to apply changes.");
    Ok(())
}

pub fn toggle_one(project_name: &str) -> anyhow::Result<()> {
    let mut config = load_repos()?;
    let projects = config
        .get_mut("projects")
        .and_then(|p| p.as_array_mut())
        .ok_or_else(|| anyhow::anyhow!("Invalid repos.json structure"))?;

    if let Some(proj) = projects.iter_mut().find(|p| p.get("name").and_then(|n| n.as_str()) == Some(project_name)) {
        let current = proj.get("visibility").and_then(|v| v.as_str()).unwrap_or("public");
        let new = if current == "public" { "private" } else { "public" };
        proj["visibility"] = Value::String(new.to_string());
        save_repos(&config)?;
        println!("✓ {} is now {}.", project_name, new);
    } else {
        anyhow::bail!("Project '{}' not found in repos.json", project_name);
    }
    Ok(())
}

pub async fn run_interactive() -> anyhow::Result<()> {
    loop {
        let config = load_repos()?;
        let projects = config
            .get("projects")
            .and_then(|p| p.as_array())
            .cloned()
            .unwrap_or_default();

        println!("\n🌐 Project Visibility Manager");
        println!("─────────────────────────────────");
        if projects.is_empty() {
            println!("No projects configured in repos.json.");
        } else {
            for (i, proj) in projects.iter().enumerate() {
                let name = proj.get("name").and_then(|n| n.as_str()).unwrap_or("unnamed");
                let visibility = proj.get("visibility").and_then(|v| v.as_str()).unwrap_or("public");
                let icon = if visibility == "private" { "🔒" } else { "🌍" };
                println!("  [{}] {} {} - {}", i + 1, icon, name, visibility);
            }
        }
        println!("\n  [A] Lock all (set all to private)");
        println!("  [B] Unlock all (set all to public)");
        println!("  [C] Toggle one (enter project name)");
        println!("  [q] Back to main menu");
        print!("  Your choice: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        match input.as_str() {
            "a" => set_all("private")?,
            "b" => set_all("public")?,
            "c" => {
                print!("Enter project name to toggle: ");
                io::stdout().flush()?;
                let mut name = String::new();
                io::stdin().read_line(&mut name)?;
                let name = name.trim();
                if let Err(e) = toggle_one(name) {
                    eprintln!("Error: {}", e);
                }
            }
            "q" => break,
            _ => {
                if let Ok(n) = input.parse::<usize>() {
                    if n >= 1 && n <= projects.len() {
                        let name = projects[n-1].get("name").and_then(|s| s.as_str()).unwrap_or("").to_string();
                        if let Err(e) = toggle_one(&name) {
                            eprintln!("Error: {}", e);
                        }
                    } else {
                        println!("Invalid selection.");
                    }
                } else {
                    println!("Invalid choice.");
                }
            }
        }
    }
    Ok(())
}

pub async fn lock_all_quick() -> anyhow::Result<()> {
    set_all("private")
}
