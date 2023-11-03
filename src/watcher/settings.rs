use std::{path::PathBuf, str::FromStr};

use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub autogenerate: bool,
    pub ignore_globs: Vec<String>,
    pub include_non_scripts: bool,
    pub rojo_project_file: Option<PathBuf>,
}

impl FromStr for Settings {
    type Err = serde_json::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}
