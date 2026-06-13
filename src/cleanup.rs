use anyhow::Result;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

pub fn cleanup_local(force: bool) -> Result<Vec<String>> {
    let lunar_dir = Path::new(".lunar");
    if !lunar_dir.exists() { println!("No .lunar/ directory found. Nothing to clean up."); return Ok(vec![]); }
    let candidates = vec![lunar_dir.join("route-ast-actual.json"), lunar_dir.join(".interfaces-autogen.json")];
    let to_remove: Vec<_> = candidates.into_iter().filter(|p| p.exists()).collect();
    if to_remove.is_empty() { println!("No cache files found. Nothing to clean up."); return Ok(vec![]); }
    println!("The following files will be removed:");
    for f in &to_remove { println!("  - {}", f.display()); }
    println!();
    if !force {
        println!("This action cannot be undone.");
        print!("Are you sure you want to continue? [y/N] ");
        io::stdout().flush()?;
        let mut input = String::new();
        if let Ok(mut tty) = std::fs::File::open("/dev/tty") {
            use std::io::BufRead;
            let mut reader = std::io::BufReader::new(&mut tty);
            reader.read_line(&mut input)?;
        } else {
            io::stdin().read_line(&mut input)?;
        }
        if input.trim().to_lowercase() != "y" && input.trim().to_lowercase() != "yes" { println!("Cleanup cancelled."); return Ok(vec![]); }
    }
    let mut removed = Vec::new();
    for f in &to_remove { fs::remove_file(f)?; removed.push(f.display().to_string()); }
    for r in &removed { println!("✓ Removed {}", r); }
    println!("Cleanup complete.");
    Ok(removed)
}

/// [ADDED] Task E Supplement: Cleans up historical audit archives older than N days.
pub fn cleanup_archives(base_path: &Path, days: i64, force: bool) -> Result<()> {
    let log_dir = base_path.join(".lunar/access-logs");
    if !log_dir.is_dir() {
        println!("No historical archives found in {}", log_dir.display());
        return Ok(());
    }

    if !force {
        print!("Do you want to clean up historical audit archives in {} older than {} days? [y/N]: ", log_dir.display(), days);
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let trimmed = input.trim().to_lowercase();
        if trimmed != "y" && trimmed != "yes" {
            println!("Archive cleanup cancelled.");
            return Ok(());
        }
    }

    println!("🧹 Purging historical archives older than {} days...", days);
    for entry in fs::read_dir(&log_dir)?.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                // Exclude the compiled task archive from daily purge to retain project history
                if file_name.ends_with(".jsonl") && file_name != "ai-todo-archive.jsonl" {
                    let date_str = file_name.trim_end_matches(".jsonl");
                    if let Ok(file_date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                        let today = chrono::Utc::now().date_naive();
                        let age = today.signed_duration_since(file_date).num_days();
                        if age > days {
                            if let Ok(_) = fs::remove_file(&path) {
                                println!("  ✓ Purged expired log file: {}", file_name);
                            }
                        }
                    }
                }
            }
        }
    }
    println!("✓ Archive cleanup complete.");
    Ok(())
}
