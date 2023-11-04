use std::{
    env::current_dir,
    path::{Path, PathBuf},
    str::FromStr,
};

use path_clean::PathClean;
use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
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

impl Settings {
    pub fn is_sourcemap_path(&self, path: &Path) -> bool {
        let smap = PathBuf::from("sourcemap.json");

        let abs_smap = clean_and_make_absolute(&smap);
        let abs_path = clean_and_make_absolute(path);

        abs_smap == abs_path
    }

    pub fn is_project_path(&self, path: &Path) -> bool {
        if let Some(project_path) = &self.rojo_project_file {
            let abs_proj = clean_and_make_absolute(project_path);
            let abs_path = clean_and_make_absolute(path);

            abs_proj == abs_path
        } else {
            false
        }
    }

    pub fn relevant_paths(&self) -> Vec<PathBuf> {
        let mut paths = vec![PathBuf::from("sourcemap.json")];

        if let Some(project_path) = &self.rojo_project_file {
            paths.push(project_path.to_path_buf());
        }

        for path in paths.iter_mut() {
            *path = clean_and_make_absolute(path);
        }

        paths
    }
}

fn clean_and_make_absolute(path: &Path) -> PathBuf {
    match path.clean() {
        p if p.is_relative() => current_dir().expect("failed to get current dir").join(p),
        p => p,
    }
}
