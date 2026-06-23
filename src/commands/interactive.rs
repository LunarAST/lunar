use std::io::{self, Write};
use std::process::ExitCode;
use std::path::Path;
use std::fs;
use serde::Deserialize;
use crate::guide;
use crate::commands::{scan, diff, sync, pull, serve, setup_totp, visibility, sync_repos, watch};
use crate::map::map;
use crate::doctor::doctor_check;
use crate::types::TopographyMap;

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

fn extract_patch_content(raw: &str) -> String {
    if let Some(start) = raw.find("---CONTENT---") {
        let after = &raw[start + "---CONTENT---".len()..];
        if let Some(end) = after.find("---LUNAR_PATCH_END---") {
            return after[..end].trim().to_string();
        }
        return after.trim().to_string();
    }
    raw.trim().to_string()
}

fn extract_project_name_from_yaml(yaml: &str) -> Option<String> {
    for line in yaml.lines() {
        if let Some(rest) = line.trim().strip_prefix("project:") {
            let name = rest.trim();
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
    }
    None
}

/// Read the configured domain from environment or from the .lunar/domain file.
fn get_domain() -> String {
    if let Ok(domain) = std::env::var("LUNAR_SERVE_DOMAIN") {
        return domain;
    }
    if let Ok(content) = fs::read_to_string(".lunar/domain") {
        let trimmed = content.trim().to_string();
        if !trimmed.is_empty() {
            return trimmed;
        }
    }
    String::new()
}

/// Interactively ask the user for a domain, auto-prepending https:// if needed,
/// and save it to .lunar/domain for future sessions.
fn set_domain() {
    print!("Enter your domain (e.g., example.com): ");
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    let raw = input.trim().to_string();
    if raw.is_empty() {
        println!("Domain not changed.");
        return;
    }
    // Auto-prepend https:// if no protocol is provided
    let domain = if raw.starts_with("http://") || raw.starts_with("https://") {
        raw
    } else {
        format!("https://{}", raw.trim_start_matches('/'))
    };
    if let Some(parent) = Path::new(".lunar/domain").parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Err(e) = fs::write(".lunar/domain", &domain) {
        eprintln!("Failed to save domain: {}", e);
    } else {
        println!("Domain saved as: {}", domain);
        println!("To use it immediately, run: export LUNAR_SERVE_DOMAIN=\"{}\"", domain);
        // set_var is unsafe in Rust 2024; allow it for backward compatibility
        #[allow(unused_unsafe)]
        unsafe {
            std::env::set_var("LUNAR_SERVE_DOMAIN", &domain);
        }
    }
}

/// Count the number of pending patches across all projects.
fn count_pending_patches(map: &TopographyMap) -> usize {
    let mut count = 0;
    for proj in &map.projects {
        if proj.name.is_empty() || proj.path.is_empty() {
            continue;
        }
        let suggest_dir = Path::new(&proj.path).join(".lunar/suggestions");
        if !suggest_dir.is_dir() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(&suggest_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                let ext = path.extension().and_then(|e| e.to_str());
                if ext != Some("yaml") && ext != Some("yml") {
                    continue;
                }
                let filename = path.file_name()
                    .map(|f| f.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if filename.is_empty()
                    || filename.ends_with(".applied")
                    || filename.ends_with(".failed")
                    || filename.ends_with(".diff")
                {
                    continue;
                }
                count += 1;
            }
        }
    }
    count
}

/// Detect pending patches on startup and offer to process them interactively.
async fn auto_probe_and_merge() -> anyhow::Result<()> {
    let map_path = "lunar-map.json";
    if !Path::new(map_path).exists() {
        return Ok(());
    }
    let map_content = fs::read_to_string(map_path)?;
    let map_val: TopographyMap = serde_json::from_str(&map_content)?;

    // Phase 1: ai-todo.json entries
    for proj in &map_val.projects {
        if proj.name.is_empty() || proj.path.is_empty() { continue; }
        let base_path = Path::new(&proj.path);
        let todo_path = base_path.join(".lunar/ai-todo.json");
        if !todo_path.exists() { continue; }

        let todo_content = match fs::read_to_string(&todo_path) {
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
                println!("  🔔 Detected pending AI patch for project '{}' (from handover board)", proj.name);
                print!("     Auto-merge and refresh map? [Y/n]: ");
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                if input.trim().to_lowercase().starts_with('y') || input.trim().is_empty() {
                    println!("📡 Merging contract patch...");
                    let cleaned = extract_patch_content(&patch_str);
                    if let Err(e) = crate::patch::apply_patch_yaml_at(&base_path, &cleaned, true) {
                        eprintln!("Merge failed: {}", e);
                    } else {
                        println!("📡 Re-compiling topography map...");
                        map(None, None, false, None, true).await?;
                        println!("✓ All contract changes aligned and map refreshed.\n");
                    }
                }
                continue; // keep processing remaining tasks/projects
            }
        }
    }

    // Phase 2: raw suggestions/ directory patches
    for proj in &map_val.projects {
        if proj.name.is_empty() || proj.path.is_empty() { continue; }
        let suggest_dir = Path::new(&proj.path).join(".lunar/suggestions");
        if !suggest_dir.is_dir() { continue; }
        let entries = match fs::read_dir(&suggest_dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str());
            if ext != Some("yaml") && ext != Some("yml") { continue; }
            let filename = path.file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_default();
            if filename.is_empty()
                || filename.ends_with(".applied")
                || filename.ends_with(".failed")
                || filename.ends_with(".diff")
            {
                continue;
            }

            let content = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            if content.trim().is_empty() { continue; }
            let cleaned = extract_patch_content(&content);
            if cleaned.is_empty() { continue; }

            let target_name = extract_project_name_from_yaml(&cleaned)
                .unwrap_or_else(|| proj.name.clone());
            let target_path = if target_name == proj.name {
                proj.path.clone()
            } else {
                map_val.projects.iter()
                    .find(|p| p.name == target_name)
                    .map(|p| p.path.clone())
                    .unwrap_or_else(|| proj.path.clone())
            };

            // Generate diff for peer review
            let diff_path = suggest_dir.join(format!("{}.diff", filename));
            let interfaces_path = Path::new(&target_path).join(".lunar/interfaces.yml");
            let current_yml = if interfaces_path.exists() {
                fs::read_to_string(&interfaces_path).unwrap_or_default()
            } else {
                "(no existing interfaces.yml)".to_string()
            };
            let mut diff_content = String::new();
            diff_content.push_str("# AI Patch Review\n\n## Current interfaces.yml\n```yaml\n");
            diff_content.push_str(&current_yml);
            diff_content.push_str("\n```\n\n## Proposed Patch\n```yaml\n");
            diff_content.push_str(&cleaned);
            diff_content.push_str("\n```\n");
            let _ = fs::write(&diff_path, diff_content);

            println!("\n🌙 LunarAST — Ecosystem Contract Governance\n");
            println!("  🔔 Detected pending AI patch: {}", filename);
            println!("     Target project: {}", target_name);
            println!("     Diff saved: {:?}", diff_path);
            println!("     Share this diff with another AI for peer review.");
            print!("     Auto-merge and refresh map? [Y/n]: ");
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            if input.trim().to_lowercase().starts_with('y') || input.trim().is_empty() {
                println!("📡 Merging patch from suggestions...");
                match crate::patch::apply_patch_yaml_at(&Path::new(&target_path), &cleaned, true) {
                    Ok(()) => {
                        let applied_name = format!("{}.applied", filename);
                        let _ = fs::rename(&path, suggest_dir.join(applied_name));
                        println!("📡 Re-compiling topography map...");
                        map(None, None, false, None, true).await?;
                        println!("✓ Patch applied and map refreshed.\n");
                    }
                    Err(e) => {
                        eprintln!("Merge failed: {}", e);
                        eprintln!("The patch has been marked as failed and will be skipped on next run.");
                        let failed_name = format!("{}.failed", filename);
                        let _ = fs::rename(&path, suggest_dir.join(failed_name));
                    }
                }
            } else {
                println!("Skipped. You can merge later from the main menu.");
            }
            continue; // Continue to next patch
        }
    }

    Ok(())
}

// ── UI helpers ──
fn print_header(state: &guide::AnalyzeState) {
    let domain = get_domain();
    let display_domain = if domain.is_empty() { "(not set)".to_string() } else { domain };
    let totp = if Path::new(".lunar/totp.secret").exists() { "✅" } else { "⚠️" };
    let scan = if state.has_data { "🟢 Scanned" } else { "🟡 No data" };

    println!("\n🌙 LunarAST — Ecosystem Contract Governance");
    println!("{}", "─".repeat(60));
    println!("📋 {}  |  🌿 {}  |  {}  |  🔐 TOTP {}  |  🌐 {}",
        state.project_name, state.language, scan, totp, display_domain);
    println!("{}", "─".repeat(60));
}

fn print_main_menu(state: &guide::AnalyzeState) {
    // Notify about pending patches
    let map_path = "lunar-map.json";
    if Path::new(map_path).exists() {
        if let Ok(content) = fs::read_to_string(map_path) {
            if let Ok(map_val) = serde_json::from_str::<TopographyMap>(&content) {
                let pending = count_pending_patches(&map_val);
                if pending > 0 {
                    println!("  🔔 {} pending AI patch(es) detected. Enter Core → W to review.", pending);
                }
            }
        }
    }

    print_header(state);
    if !state.has_data {
        println!(" 1) Core Operations (scan, health...)");
        println!(" 2) Security (TOTP setup)");
        println!(" D) Set Domain");
        println!(" 0) Quit");
    } else {
        println!(" 1) Core Operations");
        println!(" 2) Security");
        println!(" 3) Danger (clean all data)");
        println!(" D) Set Domain");
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
        println!(" W) Watch for patches");
        println!(" D) Set Domain");
        println!(" 0) Back");
    } else {
        println!(" 1) Scan project         2) Show changes");
        println!(" 3) Sync contracts       4) Pull AI patch");
        println!(" 5) Launch server        6) Generate map");
        println!(" 7) Health check         8) Stop server");
        println!(" 9) Restart server       R) Sync repo info");
        println!(" W) Watch for patches    D) Set Domain");
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
    println!(" D) Set Domain");
    println!(" 0) Back");
    println!("{}", "─".repeat(60));
}

#[cfg(unix)]
fn stop_server() -> anyhow::Result<()> {
    let pid_path = ".lunar/lunar-serve.pid";
    let mut found = false;
    if Path::new(pid_path).exists() {
        let pid_str = std::fs::read_to_string(pid_path)?;
        if let Ok(pid) = pid_str.trim().parse::<i32>() {
            unsafe {
                if libc::kill(pid, libc::SIGTERM) == 0 {
                    found = true;
                    println!("Sent SIGTERM to PID {}", pid);
                }
            }
        }
    }
    if !found {
        let status = std::process::Command::new("pkill")
            .args(["-f", "lunar-serve"])
            .status();
        match status {
            Ok(s) if s.success() => {
                println!("Stopped lunar-serve via pkill.");
                found = true;
            }
            Ok(s) => anyhow::bail!("pkill returned non-zero status: {}", s),
            Err(e) => anyhow::bail!("pkill command failed: {}.", e),
        }
    }
    if found {
        let _ = std::fs::remove_file(pid_path);
        Ok(())
    } else {
        anyhow::bail!("Could not stop lunar-serve.")
    }
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
        .args(["/F", "/PID", &pid.to_string()])
        .status()?;
    if status.success() {
        println!("Stopped lunar-serve via taskkill.");
        let _ = std::fs::remove_file(pid_path);
        Ok(())
    } else {
        anyhow::bail!("taskkill failed. Ensure lunar-serve is running.")
    }
}

fn restart_server() -> anyhow::Result<()> {
    stop_server().ok();
    std::thread::sleep(std::time::Duration::from_secs(2));
    serve::execute()
}

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
            "d" => {
                set_domain();
                println!("Press Enter to continue...");
                let mut _wait = String::new();
                io::stdin().read_line(&mut _wait).ok();
                show_menu = true;
            }
            "h" => {
                state = guide::analyze();
                show_menu = true;
            }
            _ => println!("Invalid option. Enter 0-3, D, or h for help."),
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
        if input == "d" {
            set_domain();
            println!("Press Enter to continue...");
            let mut _wait = String::new();
            io::stdin().read_line(&mut _wait).ok();
            show_menu = true;
            continue;
        }

        let executed = match (state.has_data, input.as_str()) {
            (false, "1") => Some(scan::execute()),
            (false, "7") => { doctor_check(); Some(Ok(())) },
            (false, "r") => Some(sync_repos::run().await.map(|_| ())),
            (false, "w") => Some(watch::run().await.map(|_| ())),

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
            (true, "w") => Some(watch::run().await.map(|_| ())),

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
        if input == "d" {
            set_domain();
            println!("Press Enter to continue...");
            let mut _wait = String::new();
            io::stdin().read_line(&mut _wait).ok();
            show_menu = true;
            continue;
        }

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
