use anyhow::Result;
use lunar_interface::Ci144Config;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

/// High-performance, zero-dependency 64-bit FNV-1a hash algorithm.
/// Conforms to "Occam's Razor" to calculate file changes instantly without neural-network DBs.
fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Reads the local multi-target hash map from .lunar/lunarast.hash.
pub fn read_hashes(base_path: &Path) -> HashMap<String, String> {
    let hash_path = base_path.join(".lunar/lunarast.hash");
    let mut map = HashMap::new();
    if let Ok(content) = fs::read_to_string(&hash_path) {
        for line in content.lines() {
            let parts: Vec<&str> = line.split('=').collect();
            if parts.len() == 2 {
                map.insert(parts[0].trim().to_string(), parts[1].trim().to_string());
            }
        }
    }
    map
}

/// Safely updates a specific compile target's hash line without wiping other active targets.
pub fn write_hash(base_path: &Path, target: &str, hash: &str) -> std::io::Result<()> {
    let hash_path = base_path.join(".lunar/lunarast.hash");
    let mut map = read_hashes(base_path);
    map.insert(target.to_string(), hash.to_string());
    
    let mut content = String::new();
    for (k, v) in map {
        content.push_str(&format!("{}={}\n", k, v));
    }
    
    let dir = base_path.join(".lunar");
    fs::create_dir_all(dir)?;
    fs::write(hash_path, content)
}

/// Computes a cumulative checksum of interfaces.yml, ci-144.yml, and target variables.
pub fn calculate_target_hash(base_path: &Path, target: &str) -> String {
    let mut combined_data = Vec::new();
    
    let interfaces_path = base_path.join(".lunar/interfaces.yml");
    if let Ok(data) = fs::read(interfaces_path) {
        combined_data.extend_from_slice(&data);
    }
    
    let ci144_path = base_path.join(".lunar/ci-144.yml");
    if let Ok(data) = fs::read(ci144_path) {
        combined_data.extend_from_slice(&data);
    }
    
    combined_data.extend_from_slice(target.as_bytes());
    
    let hash_val = fnv1a_hash(&combined_data);
    format!("{:016x}", hash_val)
}

/// Checks the current contract state hash against the stored hash in lunarast.hash.
/// Offers an interactive self-test, update, or ignore menu if quiet is false.
pub async fn execute_check(quiet: bool) -> Result<()> {
    let base_path = Path::new(".");
    let ci144_path = base_path.join(".lunar/ci-144.yml");
    if !ci144_path.exists() {
        if !quiet {
            println!("No ci-144.yml configuration found. Skipping check.");
        }
        return Ok(());
    }

    let content = fs::read_to_string(&ci144_path)?;
    let config: Ci144Config = match serde_yaml::from_str(&content) {
        Ok(c) => c,
        Err(_) => {
            if !quiet {
                println!("⚠️  Warning: Invalid ci-144.yml file. Skipping consistency check.");
            }
            return Ok(());
        }
    };
    
    let target = &config.target;
    let stored_hashes = read_hashes(base_path);
    let stored_hash = stored_hashes.get(target).cloned().unwrap_or_default();
    let current_hash = calculate_target_hash(base_path, target);

    if stored_hash != current_hash {
        if quiet {
            // [Section 5.3] Emit silent, non-blocking warn line post-sync/map merges
            println!("⚠️  Warning: Bridge '{}' is OUTDATED. Current contract hash differs from stored hash.", target);
            println!("   Run 'lunar' or 'lunar ci-144 check' to resolve.");
            return Ok(());
        }

        // [Section 5.4] Interactive check menu
        loop {
            println!("\nBridge status: OUTDATED (target: {})", target);
            println!("Current input hash: {} (differs from stored: {})", current_hash, stored_hash);
            println!("\nOptions:");
            println!("  [1] Re-generate bridge now (lunar gen ci-144)");
            println!("  [2] Run self-test (lunar ci-144 test)");
            println!("  [3] Ignore this change (update stored hash to current inputs)");
            println!("  [q] Quit");
            print!("\n  Your choice: ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let choice = input.trim();

            if choice == "q" {
                break;
            } else if choice == "1" {
                crate::commands::gen::execute_ci144(false, "rust", "src/bin").await?;
                let _ = write_hash(base_path, target, &current_hash);
                println!("✓ Bridge successfully re-generated and hash updated.");
                break;
            } else if choice == "2" {
                execute_test().await?;
            } else if choice == "3" {
                let _ = write_hash(base_path, target, &current_hash);
                println!("✓ Stored hash forced to match current inputs.");
                break;
            } else {
                println!("Invalid choice.");
            }
        }
    } else {
        if !quiet {
            println!("✅ Bridge status: ALIGNED (target: {})", target);
            println!("   Current hash: {}", current_hash);
        }
    }
    Ok(())
}

/// Spawns a background build of the compiled bridge binary to verify AST health.
pub async fn execute_test() -> Result<()> {
    println!("🧪 Running self-test for CI-144 bridge...");
    println!("   Compiling cellrix_bridge binary...");
    
    let mut child = std::process::Command::new("cargo")
        .args(["build", "--bin", "cellrix_bridge"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
        
    let status = child.wait()?;
    if status.success() {
        println!("✅ Compile test passed! Binary is ready at target/debug/cellrix_bridge.");
    } else {
        println!("❌ Compile test failed. Please check the code structure of your project.");
    }
    Ok(())
}

/// Clean up generated bridge files and hash records.
pub fn execute_clean() -> Result<()> {
    let base_path = Path::new(".");
    let bridge_path = base_path.join("src/bin/cellrix_bridge.rs");
    if bridge_path.exists() {
        let _ = fs::remove_file(bridge_path);
        println!("✓ Deleted src/bin/cellrix_bridge.rs");
    }
    
    let hash_path = base_path.join(".lunar/lunarast.hash");
    if hash_path.exists() {
        let _ = fs::remove_file(hash_path);
        println!("✓ Deleted .lunar/lunarast.hash");
    }
    Ok(())
}
