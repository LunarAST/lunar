use std::path::Path;

pub struct AnalyzeState {
    pub project_name: String,
    pub language: String,
    pub branch: Option<String>,
    pub initialized: bool,
    pub has_data: bool,
}

impl AnalyzeState {
    pub fn status_summary(&self) -> String {
        if self.initialized && self.has_data {
            "Initialized, scan data ready".into()
        } else if self.initialized {
            "Initialized, no scan data".into()
        } else {
            "Not initialized".into()
        }
    }
}

pub fn analyze() -> AnalyzeState {
    let project_name = std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|f| f.to_string_lossy().to_string()))
        .unwrap_or_else(|| "unknown".to_string());

    AnalyzeState {
        project_name,
        language: detect_language(),
        branch: None,
        initialized: is_initialized(),
        has_data: has_scan_data(),
    }
}

pub fn detect_language() -> String {
    if Path::new("Cargo.toml").exists() {
        "Rust (Cargo)".to_string()
    } else if Path::new("package.json").exists() {
        "JavaScript/TypeScript".to_string()
    } else {
        "Unknown".to_string()
    }
}

pub fn is_initialized() -> bool {
    Path::new(".lunar/interfaces.yml").exists()
}

pub fn has_scan_data() -> bool {
    Path::new("lunar-map.json").exists()
}

pub fn pending_suggestions() -> Option<String> {
    // simplified for now
    None
}
