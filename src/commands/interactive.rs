use std::io::{self, Write};
use std::process::ExitCode;
use crate::guide;
use crate::commands::{scan, diff, sync, pull, serve, setup_totp, visibility, sync_repos};
use crate::map::map;
use crate::doctor::doctor_check;

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
        println!("\n🌙 LunarAST — Ecosystem Contract Governance\n");
        println!("  🔔 Detected pending AI patch for project '{}'", project_name);
        print!("     Auto-merge and refresh map? [Y/n]: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if input.trim().to_lowercase().starts_with('y') || input.trim().is_empty() {
            println!("📡 Merging contract patch...");
            crate::patch::apply_patch_yaml_at(&base_path, &patch_str, true)?;
            println!("📡 Re-compiling topography map...");
            map(None, None, false, None, true).await?;
            println!("✓ All contract changes aligned and map refreshed.\n");
        }
    }
    Ok(())
}

fn print_header(state: &guide::AnalyzeState) {
    let domain = std::env::var("LUNAR_SERVE_DOMAIN").unwrap_or_else(|_| "https://lunar.aifify.com".to_string());
    let totp = if std::path::Path::new(".lunar/totp.secret").exists() { "✅" } else { "⚠️" };
    let scan = if state.has_data { "🟢 Scanned" } else { "🟡 No data" };

    println!("\n🌙 LunarAST — Ecosystem Contract Governance");
    println!("{}", "─".repeat(60));
    println!("📋 {}  |  🌿 {}  |  {}  |  🔐 TOTP {}  |  🌐 {}",
        state.project_name,
        state.language,
        scan,
        totp,
        domain
    );
    println!("{}", "─".repeat(60));
}

fn print_main_menu(state: &guide::AnalyzeState) {
    print_header(state);
    if !state.has_data {
        println!(" 1) Core Operations (scan, health...)");
        println!(" 2) Security (TOTP setup)");
        println!(" 0) Quit");
    } else {
        println!(" 1) Core Operations");
        println!(" 2) Security");
        println!(" 3) Danger (clean all data)");
        println!(" 0) Quit");
    }
    println!("{}", "─".repeat(60));
}

fn print_core_menu(state: &guide::AnalyzeState) {
    print_header(state);
    if !state.has_data {
        println!(" 1) Scan project");
        println!(" 7) Health check");
        println!(" R) Sync repo info from Git");
        println!(" 0) Back");
    } else {
        println!(" 1) Scan project         2) Show changes");
        println!(" 3) Sync contracts       4) Pull AI patch");
        println!(" 5) Launch server        6) Generate map");
        println!(" 7) Health check         8) Stop server");
        println!(" 9) Restart server       R) Sync repo info");
        println!(" 0) Back");
    }
    println!("{}", "─".repeat(60));
}

fn print_security_menu(state: &guide::AnalyzeState) {
    print_header(state);
    println!(" 1) TOTP Setup");
    if state.has_data {
        println!(" 2) Visibility Manager");
        println!(" 3) Generate keypair");
    }
    println!(" 0) Back");
    println!("{}", "─".repeat(60));
}

// 进程管理辅助函数
fn stop_server() -> anyhow::Result<()> {
    let pid_path = ".lunar/lunar-serve.pid";
    if !std::path::Path::new(pid_path).exists() {
        println!("Server is not running (PID file not found).");
        return Ok(());
    }
    let pid_str = std::fs::read_to_string(pid_path)?;
    let pid: u32 = pid_str.trim().parse()?;
    // 发送 SIGTERM
    unsafe { libc::kill(pid as i32, libc::SIGTERM); }
    // 等待进程结束（可选）
    std::thread::sleep(std::time::Duration::from_secs(1));
    let _ = std::fs::remove_file(pid_path);
    println!("Server (PID {}) has been stopped.", pid);
    Ok(())
}

fn restart_server() -> anyhow::Result<()> {
    stop_server().ok();
    serve::execute()
}

pub async fn run() -> ExitCode {
    if let Err(e) = auto_probe_and_merge().await {
        eprintln!("Warning: {}", e);
    }

    let state = guide::analyze();
    print_main_menu(&state);

    loop {
        print!("→ ");
        io::stdout().flush().ok();
        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        let input = input.trim().to_lowercase();

        match input.as_str() {
            "0" | "q" => return ExitCode::from(0),
            "1" => core_submenu().await,
            "2" => security_submenu().await,
            "3" => {
                let state = guide::analyze();
                if state.has_data {
                    println!("Run 'lunar ci144 --yes' manually if you're sure.");
                } else {
                    println!("Invalid option.");
                }
                print_main_menu(&state);
            }
            "h" => {
                let state = guide::analyze();
                print_main_menu(&state);
            }
            _ => {
                println!("Invalid option. Enter 0-3, or h for help.");
            }
        }
    }
}

async fn core_submenu() {
    loop {
        let state = guide::analyze();
        print_core_menu(&state);
        
        print!("→ ");
        io::stdout().flush().ok();
        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        let input = input.trim().to_lowercase();

        if input == "0" {
            let state = guide::analyze();
            print_main_menu(&state);
            return;
        }
        if input == "h" {
            continue;
        }

        // 处理字母命令
        if input == "r" {
            if let Err(e) = sync_repos::run().await {
                eprintln!("Error: {}", e);
            }
            continue;
        }

        let choice: u32 = match input.parse() {
            Ok(n) => n,
            Err(_) => {
                println!("Invalid option. Enter a number, R, h, or 0 to go back.");
                continue;
            }
        };

        let result = if !state.has_data {
            match choice {
                1 => scan::execute(),
                7 => { doctor_check(); Ok(()) },
                _ => { println!("Invalid option."); Ok(()) }
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
                8 => { stop_server().map_err(|e| anyhow::anyhow!(e)) },
                9 => { restart_server().map_err(|e| anyhow::anyhow!(e)) },
                _ => { println!("Invalid option."); Ok(()) }
            }
        };

        if let Err(e) = result {
            eprintln!("Error: {}", e);
        }
    }
}

async fn security_submenu() {
    loop {
        let state = guide::analyze();
        print_security_menu(&state);
        
        print!("→ ");
        io::stdout().flush().ok();
        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        let input = input.trim().to_lowercase();

        if input == "0" {
            let state = guide::analyze();
            print_main_menu(&state);
            return;
        }
        if input == "h" {
            continue;
        }

        let choice: u32 = match input.parse() {
            Ok(n) => n,
            Err(_) => {
                println!("Invalid option. Enter a number, h for help, 0 to go back.");
                continue;
            }
        };

        let result = match choice {
            1 => { setup_totp::run().await.map(|_| ()) },
            2 => {
                if state.has_data {
                    visibility::run_interactive().await.map(|_| ())
                } else {
                    println!("No project data yet."); Ok(())
                }
            },
            3 => {
                if state.has_data {
                    crate::keygen::generate_keypair(&state.project_name)
                } else {
                    println!("No project data yet."); Ok(())
                }
            },
            _ => { println!("Invalid option."); Ok(()) }
        };

        if let Err(e) = result {
            eprintln!("Error: {}", e);
        }
    }
}
