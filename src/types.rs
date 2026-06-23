use serde::Deserialize;

/// Represents the high-level topography map containing all known projects.
#[derive(Deserialize, Default)]
pub struct TopographyMap {
    #[serde(default)]
    pub projects: Vec<ProjectMeta>,
}

/// Metadata for a single project discovered in the ecosystem.
#[derive(Deserialize, Default)]
pub struct ProjectMeta {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub path: String,
}
