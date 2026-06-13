use anyhow::Result;
use chrono::Utc;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use crate::patch::apply_patch_yaml_at;
use crate::map::map;

pub async fn execute(project: Option<String>, yes: bool) -> Result<()> {
    let (project_name, base_path) = if let Some(ref p_name) = project {
        let map_path = "lunar-map.json";
        if !Path::new(map_path).exists() {
            anyhow::bail!("Global lunar-map.json missing. Run 'lunar map' first to compile metadata.");
        }
        let map_content = fs::read_to_string(map_path)?;
        let map: serde_json::Value = serde_json::from_str(&map_content)?;
        
        let path_str = map["projects"].as_array()
            .and_then(|arr| arr.iter().find(|p| p["name"].as_str().map_or(false, |n| n.eq_ignore_ascii_case(p_name))))
            .and_then(|proj| proj["path"].as_str())
            .ok_or_else(|| anyhow::anyhow!("Project '{}' not found in lunar-map.json.", p_name))?;
            
        (p_name.clone(), std::path::PathBuf::from(path_str))
    } else {
        let interfaces_path = Path::new(".lunar").join("interfaces.yml");
        if !interfaces_path.exists() {
            anyhow::bail!("interfaces.yml missing. Navigate to project root or use 'lunar pull -p <project> -y' to target anywhere.");
        }
        let content = fs::read_to_string(&interfaces_path)?;
        let yaml_val: serde_yaml::Value = serde_yaml::from_str(&content)?;
        let project_name = yaml_val.get("project")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow::anyhow!("interfaces.yml 'project' field is empty or missing."))?;
            
        (project_name.to_string(), std::path::PathBuf::from("."))
    };

    let port: u16 = std::env::var("LUNAR_SERVE_PORT").unwrap_or_else(|_| "8787".to_string()).parse().unwrap_or(8787);
    println!("📡 Fetching proposed AI patch from local serve (127.0.0.1:{})...", port);

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port)).await?;
    let request = format!(
        "GET /api/v1/projects/{}/todo HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAccept: application/json\r\nConnection: close\r\n\r\n",
        project_name, port
    );
    stream.write_all(request.as_bytes()).await?;

    let mut response = String::new();
    stream.read_to_string(&mut response).await?;

    let body = response.split("\r\n\r\n").nth(1)
        .ok_or_else(|| anyhow::anyhow!("Invalid response from lunar-serve."));

    let json_val: serde_json::Value = serde_json::from_str(body?)?;
    let patch_str = json_val.get("tasks")
        .and_then(|t| t.as_array())
        .and_then(|arr| arr.get(0))
        .and_then(|first| first.get("patch"))
        .and_then(|p| p.as_str())
        .ok_or_else(|| anyhow::anyhow!("No pending AI patch found in the active Todo list."));

    println!("✓ AI patch retrieved successfully!");
    
    // Apply patch directly to the resolved target base path on disk
    apply_patch_yaml_at(&base_path, patch_str?, yes)?;

    let todo_path = base_path.join(".lunar/ai-todo.json");
    let archive_dir = base_path.join(".lunar/access-logs");
    if let Ok(todo_content) = fs::read_to_string(&todo_path) {
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
                let _ = fs::write(&todo_path, formatted);
            }

            if !archived_tasks.is_empty() {
                let _ = fs::create_dir_all(&archive_dir);
                let archive_path = archive_dir.join("ai-todo-archive.jsonl");
                if let Ok(mut archive_file) = fs::OpenOptions::new().create(true).append(true).open(archive_path) {
                    for task in archived_tasks {
                        let archive_entry = serde_json::json!({
                            "task": task,
                            "archivedAt": Utc::now().to_rfc3339(),
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

    if yes {
        println!("📡 Automated GitOps: Re-compiling the global topography map...");
        if let Err(e) = map(None, None, false, None, true).await {
            eprintln!("Error regenerating map: {}", e);
        }
    } else {
        print!("\n✓ interfaces.yml updated. A contract change was merged.\nDo you want to re-compile the global topography map? [Y/n]: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let trimmed = input.trim().to_lowercase();
        if trimmed.is_empty() || trimmed == "y" || trimmed == "yes" {
            println!("📡 Compiling and regenerating the global topography map...");
            if let Err(e) = map(None, None, false, None, true).await {
                eprintln!("Error regenerating map: {}", e);
            }
        }
    }
    Ok(())
}
