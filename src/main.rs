use anyhow::Result;
use chrono::Utc;
use clap::{Parser, Subcommand};
use lunar::{ActualJson, InterfacesYml, InterfaceItem, RouteEntry, LunarMapConfig, generate_lunar_map, compare_routes, build_structural_index, run_adapter, DiffResult};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

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
    /// Generate a global topology map from multiple project actual.json files
    Map {
        /// Path to lunar-map-config.json
        #[arg(default_value = "lunar-map-config.json")]
        config: String,
    },
}

fn scan() -> Result<()> {
    println!("Scanning project...");
    let routes = run_adapter()?;
    println!("✓ Count verified: {} routes extracted", routes.len());
    let actual = ActualJson { exposed: routes, consumed: vec![] };
    let output_path = Path::new(".lunar").join("route-ast-actual.json");
    fs::create_dir_all(".lunar")?;
    fs::write(&output_path, serde_json::to_string_pretty(&actual)?)?;
    println!("✓ Wrote actual.json to {}", output_path.display());
    Ok(())
}

fn diff() -> Result<()> {
    let old_path = Path::new(".lunar").join("route-ast-actual.json");
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

        if old_group.is_empty() {
            for nr in &new_group { changes.push(format!("  + {} {} (added)", nr.method, nr.display_path())); }
            continue;
        }
        if new_group.is_empty() {
            for or in &old_group { changes.push(format!("  - {} {} (removed)", or.method, or.display_path())); }
            continue;
        }

        let mut new_matched = vec![false; new_group.len()];
        let mut old_matched = vec![false; old_group.len()];
        for (oi, or) in old_group.iter().enumerate() {
            for (ni, nr) in new_group.iter().enumerate() {
                if new_matched[ni] { continue; }
                match compare_routes(or, nr) {
                    DiffResult::Unchanged => {
                        old_matched[oi] = true; new_matched[ni] = true; break;
                    }
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
    let actual_path = Path::new(".lunar").join("route-ast-actual.json");
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

    if dry_run {
        println!("--- Dry run preview ---");
        if let Some(ref existing) = interfaces.exposed {
            let old_set: HashMap<String, &InterfaceItem> = existing.iter().map(|i| (format!("{} {}", i.method, i.path), i)).collect();
            let new_set: HashMap<String, &InterfaceItem> = new_exposed.iter().map(|i| (format!("{} {}", i.method, i.path), i)).collect();
            for k in old_set.keys() { if !new_set.contains_key(k) { println!("  - {} (would be removed)", k); } }
            for k in new_set.keys() { if !old_set.contains_key(k) { println!("  + {} (would be added)", k); } }
        }
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
    interfaces.exposed = Some(new_exposed);
    fs::write(&interfaces_path, serde_yaml::to_string(&interfaces)?)?;
    println!("✓ interfaces.yml updated with {} routes", actual.exposed.len());
    Ok(())
}

fn map(config_path: &str) -> Result<()> {
    let config_content = fs::read_to_string(config_path)?;
    let config: LunarMapConfig = serde_json::from_str(&config_content)?;

    let mut project_actuals = HashMap::new();
    for (name, path_str) in &config.projects {
        let actual_content = fs::read_to_string(path_str)?;
        let actual: ActualJson = serde_json::from_str(&actual_content)?;
        project_actuals.insert(name.clone(), actual);
    }

    let lunar_map = generate_lunar_map(&project_actuals);
    let output = serde_json::to_string_pretty(&lunar_map)?;
    println!("{}", output);
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Scan => scan(),
        Commands::Diff => diff(),
        Commands::Sync { apply, dry_run } => sync(apply, dry_run),
        Commands::Map { config } => map(&config),
    }
}
