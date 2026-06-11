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
