use std::io::{self, Write};
use std::process::ExitCode;
use crate::guide;
use crate::commands::{scan, diff, sync, pull, serve, setup_totp};
use crate::map::map;
use crate::doctor::doctor_check;

async fn lunar_init() -> anyhow::Result<()> {
    let interfaces_path = std::path::Path::new(".lunar").join("interfaces.yml");
    if interfaces_path.exists() {
        println!("interfaces.yml already exists.");
        return Ok(());
    }
    std::fs::create_dir_all(".lunar")?;
    let initial_yaml = r#"# LunarAST Project Interface Contract
# This file is owned and maintained by humans.
project: ""
type: mixed
environment: production
"#;
    std::fs::write(&interfaces_path, initial_yaml)?;
    println!("✓ Created .lunar/interfaces.yml");
    Ok(())
}

async fn auto_probe_and_merge() -> anyhow::Result<()> {
    let map_path = "lunar-map.json";
    if !std::path::Path::new(map_path).exists() {
        return Ok(());
    }
    
    let map_content = std::fs::read_to_string(map_path)?;
    let map_val: serde_json::Value = serde_json::from_str(&map_content)?;
    
    let projects = match map_val.get("projects").and_then(|p| p.as_array()) {
        Some(arr) => arr,
        None => return Ok(()),
    };

    let mut pending_project = None;
    let mut pending_patch = None;
    let mut target_path = None;

    for proj in projects {
        let name = proj.get("name").and_then(|n| n.as_str()).unwrap_or("");
        let path_str = proj.get("path").and_then(|p| p.as_str()).unwrap_or("");
        if name.is_empty() || path_str.is_empty() {
            continue;
        }

        let todo_path = std::path::Path::new(path_str).join(".lunar/ai-todo.json");
        if todo_path.exists() {
            if let Ok(todo_content) = std::fs::read_to_string(&todo_path) {
                if let Ok(todo_json) = serde_json::from_str::<serde_json::Value>(&todo_content) {
                    if let Some(tasks) = todo_json.get("tasks").and_then(|t| t.as_array()) {
                        for task in tasks {
                            let status = task.get("status").and_then(|s| s.as_str()).unwrap_or("");
                            let patch = task.get("patch").and_then(|p| p.as_str()).unwrap_or("");
                            if (status == "pending_alignment" || status == "pending") && !patch.is_empty() {
                                pending_project = Some(name.to_string());
                                pending_patch = Some(patch.to_string());
                                target_path = Some(std::path::Path::new(path_str).to_path_buf());
                                break;
                            }
                        }
                    }
                }
            }
        }
        if pending_project.is_some() {
            break;
        }
    }

    if let (Some(project_name), Some(patch_str), Some(base_path)) = (pending_project, pending_patch, target_path) {
        println!("🌙 LunarAST — Ecosystem Contract Governance");
        println!();
        println!("  🔔 Detected pending AI patch for project '{}' (already reviewed via web)", project_name);
        print!("     → Auto-merge and refresh map? [Y/n]: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let trimmed = input.trim().to_lowercase();

        if trimmed.is_empty() || trimmed == "y" || trimmed == "yes" {
            println!("📡 Automated GitOps: Merging contract patch...");
            
            crate::patch::apply_patch_yaml_at(&base_path, &patch_str, true)?;
            
            let todo_path = base_path.join(".lunar/ai-todo.json");
            let archive_dir = base_path.join(".lunar/access-logs");
            if let Ok(todo_content) = std::fs::read_to_string(&todo_path) {
                if let Ok(mut todo_json) = serde_json::from_str::<serde_json::Value>(&todo_content) {
                    let mut archived_tasks = Vec::new();
                    if let Some(tasks) = todo_json.get_mut("tasks").and_then(|t| t.as_array_mut()) {
                        let mut i = 0;
                        while i < tasks.len() {
                            let status = tasks[i].get("status").and_then(|s| s.as_str()).unwrap_or("");
                            if status == "pending_alignment" || status == "pending" {
                                let mut task = tasks.remove(i);
                                task["status"] = serde_json::json!("completed");
                                archived_tasks.push(task);
                            } else {
                                i += 1;
                            }
                        }
                    }
                    
                    if let Ok(formatted) = serde_json::to_string_pretty(&todo_json) {
                        let _ = std::fs::write(&todo_path, formatted);
                    }

                    if !archived_tasks.is_empty() {
                        let _ = std::fs::create_dir_all(&archive_dir);
                        let archive_path = archive_dir.join("ai-todo-archive.jsonl");
                        if let Ok(mut archive_file) = std::fs::OpenOptions::new().create(true).append(true).open(archive_path) {
                            for task in archived_tasks {
                                let archive_entry = serde_json::json!({
                                    "task": task,
                                    "archivedAt": chrono::Utc::now().to_rfc3339(),
                                    "status": "applied"
                                });
                                if let Ok(line) = serde_json::to_string(&archive_entry) {
                                    use std::io::Write as IoWrite;
                                    let _ = writeln!(archive_file, "{}", line);
                                }
                            }
                        }
                    }
                }
            }

            println!("📡 Automated GitOps: Re-compiling the global topography map...");
            if let Err(e) = map(None, None, false, None, true).await {
                eprintln!("Error regenerating map: {}", e);
            }
            
            println!("\n✓ All contract changes cleanly aligned and map refreshed! [Press Enter to continue]");
            let mut _dummy = String::new();
            let _ = io::stdin().read_line(&mut _dummy);
        }
    }
    Ok(())
}

pub async fn run() -> ExitCode {
    if let Err(e) = auto_probe_and_merge().await {
        eprintln!("Warning in auto-probe: {}", e);
    }

    loop {
        let state = guide::analyze();
        
        let port: u16 = std::env::var("LUNAR_SERVE_PORT").unwrap_or_else(|_| "8787".to_string()).parse().unwrap_or(8787);
        let domain_str = std::env::var("LUNAR_SERVE_DOMAIN").unwrap_or_else(|_| "https://lunar.aifify.com".to_string());

        // TOTP status check
        let totp_configured = std::path::Path::new(".lunar/totp.secret").exists();
        let totp_line = if totp_configured {
            "🔐 TOTP: Configured ✅"
        } else {
            "🔐 TOTP: NOT configured ⚠️"
        };

        println!();
        println!("🌙 LunarAST — Ecosystem Contract Governance");
        println!();
        println!("────────────────────────────────────────────────────────────");
        println!("  📋 Project: {}", state.project_name);
        println!("  🌿 Language: {} {}",
            state.language,
            if let Some(ref b) = state.branch { format!("| Branch: {}", b) } else { String::new() }
        );
        println!("  🌐 Domain: {}", domain_str);
        println!("  📂 Workspace: {}", std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|_| "unknown".to_string()));
        println!("  ⚙️  Active Port: {}", port);
        println!("  {}", totp_line);
        println!("────────────────────────────────────────────────────────────");
        println!("  Status: {}", state.status_summary());
        println!();

        // Uninitialized project
        if !state.initialized {
            println!("  [1] Initialize project (lunar init)");
            println!("  [0] 🔐 Setup TOTP (bind authenticator app)");
            println!("  [q] Quit");
            print!("\n  Your choice: ");
            io::stdout().flush().ok();
            let mut input = String::new();
            io::stdin().read_line(&mut input).ok();
            let input = input.trim().to_lowercase();
            match input.as_str() {
                "1" => {
                    println!("\nRunning lunar init...\n");
                    if let Err(e) = lunar_init().await {
                        eprintln!("Error: {}", e);
                    }
                }
                "0" => {
                    println!("\nRunning TOTP setup...\n");
                    if let Err(e) = setup_totp::run().await {
                        eprintln!("Error: {}", e);
                    }
                }
                "q" => return ExitCode::from(0),
                _ => println!("Invalid choice."),
            }
            continue;
        }

        // Quick Actions
        println!("✨ Quick Actions (most common)");
        if !state.has_data {
            println!("  [1] 🔄 Scan project");
            println!("  [2] 🩺 Run health check");
        } else {
            println!("  [1] 🔄 Scan project (re-extract)");
            println!("  [2] 📊 Show changes");
            println!("  [3] 🔗 Sync contracts");
            println!("  [4] 📥 Pull AI patch");
            println!("  [5] 🚀 Launch local server");
            println!("  [6] 🌍 Generate topology map");
        }

        println!();
        println!("🔧 Advanced / Utility");
        if !state.has_data {
            println!("  [0] 🔐 Setup TOTP (bind authenticator app)");
        } else {
            println!("  [7] 🩺 Run health check");
            println!("  [8] 🔑 Generate Ed25519 keypair");
            println!("  [0] 🔐 Setup TOTP (bind authenticator app)");
        }

        println!();
        println!("⚠️  Dangerous Operations");
        println!("  [c] 🗑️  Clean all S3/R2 data (requires --yes)");
        println!();
        println!("❓ Help & Exit");
        println!("  [h] Show this help again");
        println!("  [q] Quit");

        print!("\n  Your choice: ");
        io::stdout().flush().ok();
        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        let input = input.trim().to_lowercase();

        if input == "q" {
            return ExitCode::from(0);
        }

        if input == "h" {
            continue; // Reprint menu
        }

        if input == "c" {
            println!("This will permanently delete all cloud data. Run 'lunar ci144 --yes' manually if you're sure.");
            continue;
        }

        // Parse numeric choice
        let choice: u32 = match input.parse() {
            Ok(n) => n,
            _ => {
                println!("Invalid choice.");
                continue;
            }
        };

        // Route to command based on state and choice
        let result = if !state.has_data {
            match choice {
                1 => scan::execute(),
                2 => { doctor_check(); Ok(()) },
                0 => { setup_totp::run().await.map(|_| ()) },
                _ => { println!("Invalid choice."); Ok(()) }
            }
        } else {
            match choice {
                1 => scan::execute(),
                2 => diff::execute(),
                3 => sync::execute(true, false),
                4 => pull::execute(None, false).await,
                5 => serve::execute(),
                6 => map(None, None, false, None, false).await,
                7 => { doctor_check(); Ok(()) },
                8 => crate::keygen::generate_keypair(&state.project_name),
                0 => { setup_totp::run().await.map(|_| ()) },
                _ => { println!("Invalid choice."); Ok(()) }
            }
        };

        if let Err(e) = result {
            eprintln!("Error: {}", e);
        }
    }
}
