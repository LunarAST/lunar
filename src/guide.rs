use std::path::Path;

/// Detect the current project's language and framework.
pub fn detect_language() -> String {
    if Path::new("Cargo.toml").exists() {
        "Rust (Cargo)".to_string()
    } else if Path::new("package.json").exists() {
        "Node.js / TypeScript".to_string()
    } else if Path::new("requirements.txt").exists() || Path::new("pyproject.toml").exists() {
        "Python".to_string()
    } else {
        "Unknown".to_string()
    }
}

/// Detect the current Git branch name.
pub fn detect_branch() -> Option<String> {
    std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
}

/// Check if the project has been initialized with lunar.
pub fn is_initialized() -> bool {
    Path::new(".lunar/interfaces.yml").exists()
}

/// Check if scan data exists.
pub fn has_scan_data() -> bool {
    Path::new(".lunar/.interfaces-autogen.json").exists()
}

/// Check if there are pending AI suggestions.
pub fn pending_suggestions() -> Option<usize> {
    let dir = Path::new(".lunar/suggestions");
    if !dir.is_dir() { return None; }
    let count = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().map_or(false, |ext| ext == "yaml" || ext == "yml"))
                .count()
        })
        .unwrap_or(0);
    if count > 0 { Some(count) } else { None }
}

/// Check if nightly Rust toolchain is available (for rustdoc mode).
pub fn detect_nightly_available() -> bool {
    std::process::Command::new("cargo")
        .args(["+nightly", "--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Print the contextual guide for the current project.
pub fn show_guide() {
    let current_dir = std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "unknown".to_string());

    let language = detect_language();
    let branch = detect_branch();
    let initialized = is_initialized();
    let has_data = has_scan_data();
    let pending = pending_suggestions();
    let nightly_available = detect_nightly_available();

    println!("🌙 LunarAST — Ecosystem Contract Governance");
    println!();
    println!("  Current directory: {}", current_dir);
    println!("  Detected: {} {}", language, if let Some(ref b) = branch {
        format!("| Git branch: {}", b)
    } else {
        String::new()
    });
    println!();

    if !initialized {
        println!("  It looks like this project hasn't been initialized yet.");
        println!();
        println!("  Start here:");
        println!("    lunar init         Initialize contract workspace");
        if language.starts_with("Rust") {
            println!("    lunar scan         Scan for Axum routes");
            if nightly_available {
                println!();
                println!("  Tip: For maximum accuracy, use rustdoc mode (requires nightly):");
                println!("    cargo +nightly rustdoc -- -Z unstable-options --output-format json");
                println!("    lunar scan --rustdoc");
            }
        } else {
            println!("    lunar scan         Scan for routes (adapter required)");
            println!();
            println!("  Tip: If no adapter is available for your framework,");
            println!("  you can let AI help generate contract patches.");
            println!("  Just provide your source code context to an AI assistant");
            println!("  and ask it to output a YAML patch for exposed/consumed endpoints.");
            println!("  Then merge it with:  cat patch.yaml | lunar patch");
        }
    } else if !has_data {
        println!("  Contract workspace exists, but no scan data found.");
        println!();
        println!("  Run:");
        println!("    lunar scan         Extract route contracts");
        println!("    lunar doctor       Check ecosystem health");
        if nightly_available {
            println!();
            println!("  Tip: For maximum accuracy with Rust projects:");
            println!("    cargo +nightly rustdoc -- -Z unstable-options --output-format json");
            println!("    lunar scan --rustdoc");
        }
    } else {
        println!("  Contract data is ready.");
        println!();
        println!("  Available commands:");
        println!("    lunar diff         Compare current routes with last scan");
        println!("    lunar doctor       Check ecosystem health");
        println!("    lunar sync --apply Merge scan results and AI suggestions");
        println!("    lunar map          Generate ecosystem topology");
        println!();
        if let Some(count) = pending {
            println!("  ⚠️  You have {} pending AI suggestion(s). Run `lunar sync --apply` to merge them.", count);
        } else {
            println!("  Tip: If you discover missing contracts, copy the output of");
            println!("  `lunar diff` to your AI assistant to generate a patch.");
        }
    }

    println!();
    println!("  For all commands:  lunar help");
}

pub struct ProjectState {
    pub project_name: String,
    pub language: String,
    pub branch: Option<String>,
    pub initialized: bool,
    pub has_data: bool,
    pub pending_count: usize,
}

impl ProjectState {
    pub fn status_summary(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if self.initialized { parts.push("Initialized".to_string()); }
        if self.has_data { parts.push("scan data ready".to_string()); }
        if self.pending_count > 0 { parts.push(format!("{} pending suggestion(s)", self.pending_count)); }
        if parts.is_empty() { "Not initialized".to_string() } else { parts.join(", ") }
    }
}

pub fn analyze() -> ProjectState {
    let project_name = std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "unknown".to_string());
    ProjectState {
        project_name,
        language: detect_language(),
        branch: detect_branch(),
        initialized: is_initialized(),
        has_data: has_scan_data(),
        pending_count: pending_suggestions().unwrap_or(0),
    }
}
