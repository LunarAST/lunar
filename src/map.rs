use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use lunar_interface::{LunarMapConfig, ActualJson, InterfacesYml, generate_lunar_map, merge_intent_into_actual};
use crate::uploader;

pub fn auto_detect_projects(base_dir: &Path) -> Result<HashMap<String, String>> {
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

pub async fn map(config_path: Option<&str>, output: Option<&str>, upload: bool, bucket: Option<&str>, yes: bool) -> Result<()> {
    let config: LunarMapConfig = if let Some(cfg_path) = config_path {
        let config_content = fs::read_to_string(cfg_path)?;
        serde_json::from_str(&config_content)?
    } else {
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
    let mut project_paths = HashMap::new();
    let mut project_hashes = HashMap::new(); // [ADDED v3.0] Track project hashes
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

        let workspace_path = Path::new(path_str)
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        project_paths.insert(name.clone(), workspace_path);
        
        // [ADDED v3.0] Placeholder hashes mapping to satisfy 3.0 interface signature
        project_hashes.insert(name.clone(), HashMap::new());
    }
    // [MODIFIED v3.0] Ingest 4 arguments to align with the upgraded core topography engine signature
    let lunar_map = generate_lunar_map(&project_actuals, &HashMap::new(), &project_paths, &project_hashes);
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
