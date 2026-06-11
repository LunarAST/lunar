use anyhow::Result;
use chrono::Utc;
use clap::{Parser, Subcommand};
use lunar::{
    ActualJson, InterfacesYml, InterfaceItem, LunarMapConfig,
    generate_lunar_map, compare_routes, build_structural_index,
    run_adapter, DiffResult, doctor_check, cleanup_local, apply_patch_yaml,
    merge_intent_into_actual, uploader, guide,
};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "lunar")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Scan,
    Diff,
    Sync {
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        dry_run: bool,
    },
    Map {
        /// Path to config file (optional; auto-detects projects if omitted)
        #[arg(short = 'c', long)]
        config: Option<String>,
        #[arg(short = 'o', long)]
        output: Option<String>,
        #[arg(long)]
        upload: bool,
        #[arg(long, requires = "upload")]
        bucket: Option<String>,
        #[arg(long)]
        yes: bool,
    },
    Doctor,
    Cleanup {
        #[arg(long)]
        all: bool,
        #[arg(long)]
        yes: bool,
    },
    Patch {
        file: Option<String>,
    },
    Keygen {
        #[arg(default_value_t = current_dir_project_name())]
        project: String,
    },
    Share,
}

fn current_dir_project_name() -> String {
    std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "unknown".to_string())
}

fn scan() -> Result<()> {
    println!("Scanning project...");
    let routes = run_adapter()?;
    println!("✓ Count verified: {} routes extracted", routes.len());
    let actual = ActualJson { exposed: routes, consumed: vec![], project_type: None };
    let output_path = Path::new(".lunar").join(".interfaces-autogen.json");
    fs::create_dir_all(".lunar")?;
    fs::write(&output_path, serde_json::to_string_pretty(&actual)?)?;
    println!("✓ Wrote autogen.json to {}", output_path.display());
    Ok(())
}

fn diff() -> Result<()> {
    let old_path = Path::new(".lunar").join(".interfaces-autogen.json");
    if !old_path.exists() { println!("No previous scan found. Run 'lunar scan' first."); return Ok(()); }
    let old_content = fs::read_to_string(&old_path)?;
    let old_actual: ActualJson = serde_json::from_str(&old_content)?;
    let old_routes = old_actual.exposed;
    let new_routes = run_adapter()?;
    let old_index = build_structural_index(&old_routes);
    let new_index = build_structural_index(&new_routes);
    let mut all_structs: Vec<String> = old_index.keys().cloned().collect();
    for k in new_index.keys() { if !all_structs.contains(k) { all_structs.push(k.clone()); } }
    all_structs.sort();
    let mut changes = Vec::new();
    for struct_id in &all_structs {
        let old_group = old_index.get(struct_id).cloned().unwrap_or_default();
        let new_group = new_index.get(struct_id).cloned().unwrap_or_default();
        if old_group.is_empty() { for nr in &new_group { changes.push(format!("  + {} {} (added)", nr.method, nr.display_path())); } continue; }
        if new_group.is_empty() { for or in &old_group { changes.push(format!("  - {} {} (removed)", or.method, or.display_path())); } continue; }
        let mut new_matched = vec![false; new_group.len()];
        let mut old_matched = vec![false; old_group.len()];
        for (oi, or) in old_group.iter().enumerate() {
            for (ni, nr) in new_group.iter().enumerate() {
                if new_matched[ni] { continue; }
                match compare_routes(or, nr) {
                    DiffResult::Unchanged => { old_matched[oi] = true; new_matched[ni] = true; break; }
                    DiffResult::ParamNamesChanged { old_names, new_names } => {
                        changes.push(format!("  ~ {} {} (param names: {:?} → {:?})", or.method, or.display_path(), old_names, new_names));
                        old_matched[oi] = true; new_matched[ni] = true; break;
                    }
                    DiffResult::MethodChanged { old_method, new_method } => {
                        changes.push(format!("  ~ {} {} → {} (method changed)", old_method, or.display_path(), new_method));
                        old_matched[oi] = true; new_matched[ni] = true; break;
                    }
                    _ => {}
                }
            }
        }
        for (oi, or) in old_group.iter().enumerate() { if !old_matched[oi] { changes.push(format!("  - {} {} (removed)", or.method, or.display_path())); } }
        for (ni, nr) in new_group.iter().enumerate() { if !new_matched[ni] { changes.push(format!("  + {} {} (added)", nr.method, nr.display_path())); } }
    }
    if changes.is_empty() { println!("No changes detected."); }
    else {
        println!("Changes detected:"); for line in &changes { println!("{}", line); }
        println!();
        println!("Hint: Copy the above output to your AI assistant to generate a contract patch.");
        println!("Then run `cat patch.yaml | lunar patch` to apply it.");
    }
    Ok(())
}

fn sync(apply: bool, dry_run: bool) -> Result<()> {
    let interfaces_path = Path::new(".lunar").join("interfaces.yml");
    let backup_dir = Path::new(".lunar").join(".backup");
    let suggestions_dir = Path::new(".lunar").join("suggestions");
    let actual_path = Path::new(".lunar").join(".interfaces-autogen.json");
    if !actual_path.exists() { println!("No scan data found. Run 'lunar scan' first."); return Ok(()); }
    let actual: ActualJson = serde_json::from_str(&fs::read_to_string(&actual_path)?)?;
    let new_exposed: Vec<InterfaceItem> = actual.exposed.iter().map(|r| InterfaceItem {
        path: r.to_path(), method: r.method.clone(), reason: None, target_project: None,
    }).collect();
    let mut interfaces: InterfacesYml = if interfaces_path.exists() {
        serde_yaml::from_str(&fs::read_to_string(&interfaces_path)?)?
    } else {
        InterfacesYml { project: None, project_type: None, environment: None, exposed: Some(Vec::new()), consumed: None }
    };
    if let Some(ref mut existing) = interfaces.exposed {
        for item in &new_exposed { if !existing.iter().any(|e| e.path == item.path && e.method == item.method) { existing.push(item.clone()); } }
    } else { interfaces.exposed = Some(new_exposed.clone()); }
    if suggestions_dir.is_dir() {
        let mut entries: Vec<_> = fs::read_dir(&suggestions_dir)?.filter_map(|e| e.ok()).filter(|e| e.path().extension().map_or(false, |ext| ext == "yaml" || ext == "yml")).collect();
        entries.sort_by_key(|e| e.file_name());
        if !entries.is_empty() {
            println!("Found {} AI/human suggestion(s) to merge.", entries.len());
            for entry in &entries {
                let path = entry.path();
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(suggestion) = serde_yaml::from_str::<InterfacesYml>(&content) {
                        if let Some(sug_exposed) = suggestion.exposed {
                            let existing = interfaces.exposed.get_or_insert_with(Vec::new);
                            for item in &sug_exposed {
                                if let Some(ei) = existing.iter_mut().find(|e| e.path == item.path && e.method == item.method) {
                                    if item.reason.is_some() { ei.reason = item.reason.clone(); }
                                    if item.target_project.is_some() { ei.target_project = item.target_project.clone(); }
                                } else { existing.push(item.clone()); }
                            }
                        }
                        if let Some(sug_consumed) = suggestion.consumed {
                            let existing = interfaces.consumed.get_or_insert_with(Vec::new);
                            for item in &sug_consumed {
                                if let Some(ei) = existing.iter_mut().find(|e| e.path == item.path && e.method == item.method) {
                                    if item.reason.is_some() { ei.reason = item.reason.clone(); }
                                    if item.target_project.is_some() { ei.target_project = item.target_project.clone(); }
                                } else { existing.push(item.clone()); }
                            }
                        }
                        let new_path = path.with_extension("yaml.applied");
                        fs::rename(&path, &new_path)?;
                    }
                }
            }
            println!("Suggestions processed.");
        }
    }
    if dry_run {
        println!("--- Dry run preview ---");
        if let Some(ref existing) = interfaces.exposed { for item in existing { println!("  E: {} {}", item.method, item.path); } }
        if let Some(ref existing) = interfaces.consumed { for item in existing { println!("  C: {} {} -> {}", item.method, item.path, item.target_project.as_deref().unwrap_or("?")); } }
        println!("--- End of preview ---");
        return Ok(());
    }
    if !apply { println!("No action taken."); return Ok(()); }
    if interfaces_path.exists() {
        fs::create_dir_all(&backup_dir)?;
        let ts = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
        fs::copy(&interfaces_path, backup_dir.join(format!("interfaces.yml.bak.{}", ts)))?;
        println!("✓ Backup saved");
    }
    fs::write(&interfaces_path, serde_yaml::to_string(&interfaces)?)?;
    println!("✓ interfaces.yml updated");
    Ok(())
}

/// Auto-detect projects in a given directory by finding .lunar/.interfaces-autogen.json files.
fn auto_detect_projects(base_dir: &Path) -> Result<HashMap<String, String>> {
    let mut projects = HashMap::new();
    if !base_dir.is_dir() {
        return Ok(projects);
    }
    for entry in fs::read_dir(base_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let autogen = path.join(".lunar/.interfaces-autogen.json");
            if autogen.exists() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    projects.insert(name.to_string(), autogen.to_string_lossy().to_string());
                }
            }
        }
    }
    Ok(projects)
}

async fn map(config_path: Option<&str>, output: Option<&str>, upload: bool, bucket: Option<&str>, yes: bool) -> Result<()> {
    let config: LunarMapConfig = if let Some(cfg_path) = config_path {
        let config_content = fs::read_to_string(cfg_path)?;
        serde_json::from_str(&config_content)?
    } else {
        // Auto-detect projects
        let scan_dir = std::env::var("LUNAR_PROJECTS_DIR").unwrap_or_else(|_| "/opt".to_string());
        let base = Path::new(&scan_dir);
        println!("No config file specified. Auto-detecting projects in {}...", scan_dir);
        let detected = auto_detect_projects(base)?;
        if detected.is_empty() {
            anyhow::bail!("No projects found in {}. Run 'lunar scan' in each project first, or specify a config file with --config.", scan_dir);
        }
        println!("Found {} project(s):", detected.len());
        for (name, path) in &detected {
            println!("  - {} ({})", name, path);
        }
        LunarMapConfig { projects: detected }
    };

    let mut project_actuals = HashMap::new();
    for (name, path_str) in &config.projects {
        let actual_content = fs::read_to_string(path_str)?;
        let mut actual: ActualJson = serde_json::from_str(&actual_content)?;
        let intent_path = Path::new(path_str).parent().unwrap().join("interfaces.yml");
        if intent_path.exists() {
            if let Ok(intent_content) = fs::read_to_string(&intent_path) {
                if let Ok(intent) = serde_yaml::from_str::<InterfacesYml>(&intent_content) {
                    merge_intent_into_actual(&mut actual, &intent);
                }
            }
        }
        project_actuals.insert(name.clone(), actual);
    }
    let lunar_map = generate_lunar_map(&project_actuals, &HashMap::new());
    let output_json = serde_json::to_string_pretty(&lunar_map)?;

    let output_path = if let Some(out_path) = output {
        fs::write(out_path, &output_json)?;
        println!("✓ lunar-map.json written to {}", out_path);
        out_path.to_string()
    } else {
        let default_path = "lunar-map.json";
        fs::write(default_path, &output_json)?;
        println!("{}", output_json);
        default_path.to_string()
    };

    if upload {
        let bucket_name = bucket
            .map(|b| b.to_string())
            .or_else(|| std::env::var("LUNAR_S3_BUCKET").ok())
            .ok_or_else(|| anyhow::anyhow!("No bucket specified. Use --bucket or set LUNAR_S3_BUCKET env."))?;

        let metadata = fs::metadata(&output_path)?;
        let file_size_kb = metadata.len() as f64 / 1024.0;
        println!("  File: {} ({:.1} KB)", output_path, file_size_kb);
        println!("  Target: {}/lunar-map.json", bucket_name);

        if !yes {
            print!("Proceed with upload? [y/N] ");
            io::stdout().flush()?;
            let mut input = String::new();
            if let Ok(mut tty) = std::fs::File::open("/dev/tty") {
                use std::io::BufRead;
                let mut reader = std::io::BufReader::new(&mut tty);
                reader.read_line(&mut input)?;
            } else {
                io::stdin().read_line(&mut input)?;
            }
            if input.trim().to_lowercase() != "y" && input.trim().to_lowercase() != "yes" {
                println!("Upload cancelled.");
                return Ok(());
            }
        }

        uploader::upload_to_s3(Path::new(&output_path), "lunar-map.json", &bucket_name).await?;
    }

    Ok(())
}

fn patch_cmd(file: Option<String>) -> Result<()> {
    let yaml_str = if let Some(path_str) = file {
        fs::read_to_string(&path_str)?
    } else {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        if buf.trim().is_empty() {
            println!("No input provided. Usage:");
            println!("  lunar patch path/to/file.yaml");
            println!("  cat patch.yaml | lunar patch");
            return Ok(());
        }
        buf
    };
    apply_patch_yaml(&yaml_str)
}

// ---------- Interactive mode ----------

async fn interactive_mode() -> ExitCode {
    loop {
        let state = guide::analyze();
        println!();
        println!("🌙 LunarAST — Ecosystem Contract Governance");
        println!();
        println!("  Project: {}", state.project_name);
        println!("  Detected: {} {}", state.language, if let Some(ref b) = state.branch {
            format!("| Git branch: {}", b)
        } else {
            String::new()
        });
        println!("  Status: {}", state.status_summary());
        println!();

        let mut options: Vec<(&str, &str)> = Vec::new();
        let mut index = 1usize;

        if !state.initialized {
            println!("  This project hasn't been initialized yet.");
            println!();
            println!("  [1] Initialize project (lunar init)");
            println!("  [q] Quit");
            print!("\n  Your choice: ");
            io::stdout().flush().ok();
            let mut input = String::new();
            io::stdin().read_line(&mut input).ok();
            let input = input.trim().to_lowercase();
            match input.as_str() {
                "1" => {
                    println!("\nRunning lunar init...\n");
                    if let Err(e) = lunar_init() {
                        eprintln!("Error: {}", e);
                    }
                }
                "q" => return ExitCode::from(0),
                _ => println!("Invalid choice."),
            }
            continue;
        }

        if !state.has_data {
            options.push(("Scan project", "lunar scan"));
            options.push(("Health check", "lunar doctor"));
        } else {
            options.push(("Scan project (re-extract)", "lunar scan"));
            options.push(("Show changes", "lunar diff"));
            options.push(("Sync contracts", "lunar sync --apply"));
            options.push(("Generate topology", "lunar map"));
            options.push(("Health check", "lunar doctor"));
        }

        for (desc, _cmd) in &options {
            println!("  [{}] {}", index, desc);
            index += 1;
        }
        println!("  [q] Quit");

        print!("\n  Your choice: ");
        io::stdout().flush().ok();
        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        let input = input.trim().to_lowercase();

        if input == "q" {
            return ExitCode::from(0);
        }

        let choice: usize = match input.parse() {
            Ok(n) if n >= 1 && n <= options.len() => n,
            _ => {
                println!("Invalid choice.");
                continue;
            }
        };

        let (_desc, cmd_str) = options[choice - 1];

        println!("\nRunning {}...\n", cmd_str);

        let result = match cmd_str {
            "lunar scan" => scan(),
            "lunar diff" => diff(),
            "lunar sync --apply" => sync(true, false),
            "lunar map" => {
                
                map(None, None, false, None, false).await
            }
            "lunar doctor" => { doctor_check(); Ok(()) }
            _ => Ok(()),
        };

        if let Err(e) = result {
            eprintln!("Error: {}", e);
        }
    }
}

fn lunar_init() -> Result<()> {
    let interfaces_path = Path::new(".lunar").join("interfaces.yml");
    if interfaces_path.exists() {
        println!("interfaces.yml already exists.");
        return Ok(());
    }
    fs::create_dir_all(".lunar")?;
    let initial_yaml = r#"# LunarAST Project Interface Contract
# This file is owned and maintained by humans.
project: ""
type: mixed
environment: production
"#;
    fs::write(&interfaces_path, initial_yaml)?;
    println!("✓ Created .lunar/interfaces.yml");
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    if std::env::args().len() == 1 {
        return interactive_mode().await;
    }

    let cli = Cli::parse();
    let command = match cli.command {
        Some(cmd) => cmd,
        None => return interactive_mode().await,
    };

    let result = match command {
        Commands::Scan => scan(),
        Commands::Diff => diff(),
        Commands::Sync { apply, dry_run } => sync(apply, dry_run),
        Commands::Map { config, output, upload, bucket, yes } => {
            map(config.as_deref(), output.as_deref(), upload, bucket.as_deref(), yes).await
        }
        Commands::Doctor => { return doctor_check(); }
        Commands::Cleanup { all: _, yes } => cleanup_local(yes).map(|_| ()),
        Commands::Patch { file } => patch_cmd(file),
        Commands::Keygen { project } => {
            match lunar::keygen::generate_keypair(&project) {
                Ok(()) => Ok(()),
                Err(e) => { eprintln!("Error: {}", e); Ok(()) }
            }
        }
        Commands::Share => {
            match lunar::share::run_share().await {
                Ok(()) => Ok(()),
                Err(e) => { eprintln!("Error: {}", e); Ok(()) }
            }
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        ExitCode::from(1)
    } else {
        ExitCode::from(0)
    }
}
