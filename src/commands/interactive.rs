use std::io::{self, Write};
use std::process::ExitCode;
use std::path::Path;
use serde::Deserialize;
use crate::guide;
use crate::commands::{scan, diff, sync, pull, serve, setup_totp, visibility, sync_repos};
use crate::map::map;
use crate::doctor::doctor_check;

// ── Strongly-typed JSON structures replacing chained Value calls ──
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

#[derive(Deserialize, Default)]
struct AiTodo {
    #[serde(default)]
    tasks: Vec<AiTask>,
}

#[derive(Deserialize, Default)]
struct AiTask {
    #[serde(default)]
    status: String,
    #[serde(default)]
    patch: Option<String>,
}

// ── Auto-probe & merge ──
async fn auto_probe_and_merge() -> anyhow::Result<()> {
    let map_path = "lunar-map.json";
    if !Path::new(map_path).exists() {
        return Ok(());
    }
    let map_content = std::fs::read_to_string(map_path)?;
    let map_val: TopographyMap = serde_json::from_str(&map_content)?;

    for proj in map_val.projects {
        if proj.name.is_empty() || proj.path.is_empty() { continue; }
        let base_path = Path::new(&proj.path);
        let todo_path = base_path.join(".lunar/ai-todo.json");
        if !todo_path.exists() { continue; }

        let todo_content = match std::fs::read_to_string(&todo_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let todo_json: AiTodo = match serde_json::from_str(&todo_content) {
            Ok(j) => j,
            Err(_) => continue,
        };

        for task in todo_json.tasks {
            let is_pending = task.status == "pending_alignment" || task.status == "pending";
            if is_pending && task.patch.as_deref().map_or(false, |p| !p.is_empty()) {
                let patch_str = task.patch.unwrap();
                println!("\n🌙 LunarAST — Ecosystem Contract Governance\n");
                println!("  🔔 Detected pending AI patch for project '{}'", proj.name);
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
                return Ok(());
            }
        }
    }
    Ok(())
}

// ── UI helpers ──
fn print_header(state: &guide::AnalyzeState) {
    let domain = std::env::var("LUNAR_SERVE_DOMAIN").unwrap_or_else(|_| String::from("(not set)"));
    let totp = if Path::new(".lunar/totp.secret").exists() { "✅" } else { "⚠️" };
    let scan = if state.has_data { "🟢 Scanned" } else { "🟡 No data" };

    println!("\n🌙 LunarAST — Ecosystem Contract Governance");
    println!("{}", "─".repeat(60));
    println!("📋 {}  |  🌿 {}  |  {}  |  🔐 TOTP {}  |  🌐 {}",
        state.project_name, state.language, scan, totp, domain);
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

// ── Platform-safe process termination ──
#[cfg(unix)]
fn stop_server() -> anyhow::Result<()> {
    let pid_path = ".lunar/lunar-serve.pid";
    if !Path::new(pid_path).exists() {
        println!("Server is not running (PID file not found).");
        return Ok(());
    }
    let pid_str = std::fs::read_to_string(pid_path)?;
    let pid: u32 = pid_str.trim().parse()?;
    unsafe {
        if libc::kill(pid as i32, libc::SIGTERM) != 0 {
            return Err(anyhow::anyhow!("Failed to send SIGTERM to process {}", pid));
        }
    }
    std::thread::sleep(std::time::Duration::from_secs(1));
    let _ = std::fs::remove_file(pid_path);
    println!("Server (PID {}) has been stopped.", pid);
    Ok(())
}

#[cfg(not(unix))]
fn stop_server() -> anyhow::Result<()> {
    let pid_path = ".lunar/lunar-serve.pid";
    if !Path::new(pid_path).exists() {
        println!("Server is not running (PID file not found).");
        return Ok(());
    }
    let pid_str = std::fs::read_to_string(pid_path)?;
    let pid: u32 = pid_str.trim().parse()?;
    let status = std::process::Command::new("taskkill")
        .args(&["/F", "/PID", &pid.to_string()])
        .status()?;
    if status.success() {
        println!("Server (PID {}) stopped.", pid);
        let _ = std::fs::remove_file(pid_path);
        Ok(())
    } else {
        Err(anyhow::anyhow!("Failed to stop process {} on non-Unix platform", pid))
    }
}

fn restart_server() -> anyhow::Result<()> {
    stop_server().ok();
    serve::execute()
}

// ── Main entry ──
pub async fn run() -> ExitCode {
    if let Err(e) = auto_probe_and_merge().await {
        eprintln!("Warning: {}", e);
    }

    let mut state = guide::analyze();
    let mut show_menu = true;

    loop {
        if show_menu {
            print_main_menu(&state);
            show_menu = false;
        }

        print!("→ ");
        io::stdout().flush().ok();
        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        let input = input.trim().to_lowercase();

        match input.as_str() {
            "0" | "q" => return ExitCode::from(0),
            "1" => {
                core_submenu().await;
                state = guide::analyze();
                show_menu = true;
            }
            "2" => {
                security_submenu().await;
                state = guide::analyze();
                show_menu = true;
            }
            "3" => {
                if state.has_data {
                    println!("Run 'lunar ci144 --yes' manually if you're sure.");
                    println!("Press Enter to continue...");
                    let mut _wait = String::new();
                    io::stdin().read_line(&mut _wait).ok();
                    show_menu = true;
                } else {
                    println!("Invalid option. Enter 0-3, or h for help.");
                }
            }
            "h" => {
                state = guide::analyze();
                show_menu = true;
            }
            _ => println!("Invalid option. Enter 0-3, or h for help."),
        }
    }
}

async fn core_submenu() {
    let mut show_menu = true;
    loop {
        let state = guide::analyze();
        if show_menu {
            print_core_menu(&state);
            show_menu = false;
        }

        print!("→ ");
        io::stdout().flush().ok();
        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        let input = input.trim().to_lowercase();

        if input == "0" { return; }
        if input == "h" { show_menu = true; continue; }

        let executed = match (state.has_data, input.as_str()) {
            (false, "1") => Some(scan::execute()),
            (false, "7") => { doctor_check(); Some(Ok(())) },
            (false, "r") => Some(sync_repos::run().await.map(|_| ())),

            (true, "1") => Some(scan::execute()),
            (true, "2") => Some(diff::execute()),
            (true, "3") => Some(sync::execute(true, false)),
            (true, "4") => Some(pull::execute(None, false).await),
            (true, "5") => Some(serve::execute()),
            (true, "6") => Some(map(None, None, false, None, false).await),
            (true, "7") => { doctor_check(); Some(Ok(())) },
            (true, "8") => Some(stop_server()),
            (true, "9") => Some(restart_server()),
            (true, "r") => Some(sync_repos::run().await.map(|_| ())),

            _ => {
                println!("Invalid option. Enter a valid menu option.");
                None
            }
        };

        if let Some(result) = executed {
            if let Err(e) = result {
                eprintln!("Error: {}", e);
            }
            println!("Press Enter to continue...");
            let mut _wait = String::new();
            io::stdin().read_line(&mut _wait).ok();
            show_menu = true;
        }
    }
}

async fn security_submenu() {
    let mut show_menu = true;
    loop {
        let state = guide::analyze();
        if show_menu {
            print_security_menu(&state);
            show_menu = false;
        }

        print!("→ ");
        io::stdout().flush().ok();
        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        let input = input.trim().to_lowercase();

        if input == "0" { return; }
        if input == "h" { show_menu = true; continue; }

        let executed = match (state.has_data, input.as_str()) {
            (_, "1") => Some(setup_totp::run().await.map(|_| ())),
            (true, "2") => Some(visibility::run_interactive().await.map(|_| ())),
            (true, "3") => Some(crate::keygen::generate_keypair(&state.project_name)),
            _ => {
                println!("Invalid option. Enter a valid menu option.");
                None
            }
        };

        if let Some(result) = executed {
            if let Err(e) = result {
                eprintln!("Error: {}", e);
            }
            println!("Press Enter to continue...");
            let mut _wait = String::new();
            io::stdin().read_line(&mut _wait).ok();
            show_menu = true;
        }
    }
}
