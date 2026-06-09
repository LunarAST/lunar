use anyhow::Result;
use chrono::Utc;
use clap::{Parser, Subcommand};
use lunar::{
    ActualJson, InterfacesYml, InterfaceItem, LunarMapConfig,
    generate_lunar_map, compare_routes, build_structural_index,
    run_adapter, DiffResult, doctor_check, cleanup_local,
};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "lunar")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
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
        #[arg(default_value = "lunar-map-config.json")]
        config: String,
        #[arg(short = 'o', long)]
        output: Option<String>,
    },
    Doctor,
    Cleanup {
        #[arg(long)]
        all: bool,
        #[arg(long)]
        yes: bool,
    },
}

fn scan() -> Result<()> {
    println!("Scanning project...");
    let routes = run_adapter()?;
    println!("✓ Count verified: {} routes extracted", routes.len());
    let actual = ActualJson { exposed: routes, consumed: vec![] };
    let output_path = Path::new(".lunar").join(".interfaces-autogen.json");
    fs::create_dir_all(".lunar")?;
    fs::write(&output_path, serde_json::to_string_pretty(&actual)?)?;
    println!("✓ Wrote autogen.json to {}", output_path.display());
    Ok(())
}

fn diff() -> Result<()> {
    let old_path = Path::new(".lunar").join(".interfaces-autogen.json");
    if !old_path.exists() {
        println!("No previous scan found. Run 'lunar scan' first.");
        return Ok(());
    }
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
    else { println!("Changes detected:"); for line in &changes { println!("{}", line); } }
    Ok(())
}

fn sync(apply: bool, dry_run: bool) -> Result<()> {
    let interfaces_path = Path::new(".lunar").join("interfaces.yml");
    let backup_dir = Path::new(".lunar").join(".backup");
    let suggestions_dir = Path::new(".lunar").join("suggestions");
    let actual_path = Path::new(".lunar").join(".interfaces-autogen.json");

    if !actual_path.exists() {
        println!("No scan data found. Run 'lunar scan' first.");
        return Ok(());
    }

    let actual: ActualJson = serde_json::from_str(&fs::read_to_string(&actual_path)?)?;
    let new_exposed: Vec<InterfaceItem> = actual.exposed.iter().map(|r| InterfaceItem {
        path: r.to_path(), method: r.method.clone(), reason: None, target_project: None,
    }).collect();

    let mut interfaces: InterfacesYml = if interfaces_path.exists() {
        serde_yaml::from_str(&fs::read_to_string(&interfaces_path)?)?
    } else {
        InterfacesYml {
            project: None, project_type: None, environment: None,
            exposed: Some(Vec::new()), consumed: None,
        }
    };

    // Merge scan results into interfaces
    if let Some(ref mut existing) = interfaces.exposed {
        for item in &new_exposed {
            if !existing.iter().any(|e| e.path == item.path && e.method == item.method) {
                existing.push(item.clone());
            }
        }
    } else {
        interfaces.exposed = Some(new_exposed.clone());
    }

    // Process suggestion patches if present
    let mut suggestions_applied = false;
    let mut suggestion_files = Vec::new();
    if suggestions_dir.is_dir() {
        let mut entries: Vec<_> = fs::read_dir(&suggestions_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "yaml" || ext == "yml"))
            .collect();
        entries.sort_by_key(|e| e.file_name());
        if !entries.is_empty() {
            println!("Found {} AI/human suggestion(s) to merge.", entries.len());
            for entry in &entries {
                let path = entry.path();
                match fs::read_to_string(&path) {
                    Ok(content) => {
                        match serde_yaml::from_str::<InterfacesYml>(&content) {
                            Ok(suggestion) => {
                                if let Some(sug_exposed) = suggestion.exposed {
                                    let existing = interfaces.exposed.get_or_insert_with(Vec::new);
                                    for item in &sug_exposed {
                                        if let Some(existing_item) = existing.iter_mut().find(|e| e.path == item.path && e.method == item.method) {
                                            if item.reason.is_some() { existing_item.reason = item.reason.clone(); }
                                            if item.target_project.is_some() { existing_item.target_project = item.target_project.clone(); }
                                        } else {
                                            existing.push(item.clone());
                                        }
                                    }
                                }
                                if let Some(sug_consumed) = suggestion.consumed {
                                    let existing = interfaces.consumed.get_or_insert_with(Vec::new);
                                    for item in &sug_consumed {
                                        if let Some(existing_item) = existing.iter_mut().find(|e| e.path == item.path && e.method == item.method) {
                                            if item.reason.is_some() { existing_item.reason = item.reason.clone(); }
                                            if item.target_project.is_some() { existing_item.target_project = item.target_project.clone(); }
                                        } else {
                                            existing.push(item.clone());
                                        }
                                    }
                                }
                                suggestions_applied = true;
                                suggestion_files.push(path);
                            }
                            Err(e) => {
                                eprintln!("Warning: failed to parse suggestion file {}: {}", path.display(), e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Warning: could not read suggestion file {}: {}", path.display(), e);
                    }
                }
            }
        }
    }

    let suggestion_count = suggestion_files.len();

    if dry_run {
        println!("--- Dry run preview ---");
        if let Some(ref existing) = interfaces.exposed {
            println!("Exposed endpoints ({}):", existing.len());
            for item in existing {
                println!("  {} {}", item.method, item.path);
            }
        }
        if let Some(ref existing) = interfaces.consumed {
            println!("Consumed endpoints ({}):", existing.len());
            for item in existing {
                println!("  {} {} -> {}", item.method, item.path, item.target_project.as_deref().unwrap_or("?"));
            }
        }
        if suggestion_count > 0 {
            println!("{} suggestion(s) would be applied.", suggestion_count);
        }
        println!("--- End of preview ---");
        return Ok(());
    }

    if !apply {
        println!("No action taken. Use --apply to write changes, or --dry-run to preview.");
        return Ok(());
    }

    // Confirmation prompt if suggestions are present
    if suggestions_applied {
        println!("The following suggestion changes will be applied:");
        if let Some(ref exposed) = interfaces.exposed {
            for item in exposed {
                println!("  E: {} {}", item.method, item.path);
            }
        }
        if let Some(ref consumed) = interfaces.consumed {
            for item in consumed {
                println!("  C: {} {} -> {}", item.method, item.path, item.target_project.as_deref().unwrap_or("?"));
            }
        }
        print!("Proceed with merge? [y/N] ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if input.trim().to_lowercase() != "y" && input.trim().to_lowercase() != "yes" {
            println!("Merge cancelled. Suggestions remain in {}.", suggestions_dir.display());
            return Ok(());
        }
    }

    // Backup and write
    if interfaces_path.exists() {
        fs::create_dir_all(&backup_dir)?;
        let ts = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
        fs::copy(&interfaces_path, backup_dir.join(format!("interfaces.yml.bak.{}", ts)))?;
        println!("✓ Backup saved");
    }

    fs::write(&interfaces_path, serde_yaml::to_string(&interfaces)?)?;
    println!("✓ interfaces.yml updated with {} exposed routes", interfaces.exposed.as_ref().map_or(0, |v| v.len()));

    // Rename processed suggestion files to .yaml.applied
    if !suggestion_files.is_empty() {
        for path in &suggestion_files {
            let new_path = path.with_extension("yaml.applied");
            fs::rename(path, &new_path)?;
        }
        println!("✓ {} suggestion(s) applied and renamed to .yaml.applied", suggestion_files.len());
    }

    Ok(())
}

fn map(config_path: &str, output: Option<&str>) -> Result<()> {
    let config_content = fs::read_to_string(config_path)?;
    let config: LunarMapConfig = serde_json::from_str(&config_content)?;
    let mut project_actuals = HashMap::new();
    for (name, path_str) in &config.projects {
        let actual_content = fs::read_to_string(path_str)?;
        let actual: ActualJson = serde_json::from_str(&actual_content)?;
        project_actuals.insert(name.clone(), actual);
    }
    let lunar_map = generate_lunar_map(&project_actuals, &HashMap::new());
    let output_json = serde_json::to_string_pretty(&lunar_map)?;
    if let Some(out_path) = output {
        fs::write(out_path, output_json)?;
        println!("✓ lunar-map.json written to {}", out_path);
    } else {
        println!("{}", output_json);
    }
    Ok(())
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Scan => scan(),
        Commands::Diff => diff(),
        Commands::Sync { apply, dry_run } => sync(apply, dry_run),
        Commands::Map { config, output } => map(&config, output.as_deref()),
        Commands::Doctor => { return doctor_check(); }
        Commands::Cleanup { all: _, yes } => cleanup_local(yes).map(|_| ()),
    };
    if let Err(e) = result {
        eprintln!("Error: {}", e);
        ExitCode::from(1)
    } else {
        ExitCode::from(0)
    }
}
