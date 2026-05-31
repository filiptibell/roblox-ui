use std::{collections::HashMap, path::PathBuf, str::FromStr};

use thiserror::Error;
use tokio::fs::read_dir;

const THEME_HEADER_NAME: &str = "Icon Theme";

type Lowercase = String;
type Map = HashMap<Lowercase, String>;

#[derive(Debug, Error)]
pub enum CustomThemeError {
    #[error("Theme file is missing the header section")]
    Header,
    #[error("Theme file is empty (no non-header sections)")]
    Empty,
    #[error("IO error: {0}")]
    Io(#[from] tokio::io::Error),
}

#[derive(Debug, Clone)]
pub struct CustomThemeFile {
    #[allow(dead_code)]
    header: Map,
    sections: HashMap<String, Map>,
}

impl CustomThemeFile {
    pub fn best_instances_dir(&self, base_path: impl Into<PathBuf>) -> Option<PathBuf> {
        let base_path = base_path.into();
        let mut best = None;
        let mut best_res = 0;
        for (section, section_map) in self.sections.iter() {
            if section.starts_with("instance/") || section.starts_with("instances/") {
                let size = section_map.get("size").and_then(|s| s.parse::<u32>().ok());
                let scale = section_map.get("scale").and_then(|s| s.parse::<f32>().ok());
                let res = size
                    .map(|s| s as f32)
                    .zip(scale)
                    .map(|(s, sc)| (s * sc) as u32)
                    .unwrap_or(0);
                if res > best_res {
                    best = Some(base_path.join(section));
                    best_res = res;
                }
            }
        }
        best
    }

    pub async fn best_instances_paths(
        &self,
        base_path: impl Into<PathBuf>,
    ) -> Result<Vec<PathBuf>, CustomThemeError> {
        let dir = self.best_instances_dir(base_path);
        if let Some(dir) = dir {
            let mut paths = Vec::new();
            let mut reader = read_dir(dir).await?;
            while let Some(entry) = reader.next_entry().await? {
                let path = entry.path();
                if path.is_file() {
                    paths.push(path);
                }
            }
            Ok(paths)
        } else {
            Ok(Vec::new())
        }
    }
}

impl FromStr for CustomThemeFile {
    type Err = CustomThemeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut sections = HashMap::<String, Map>::new();

        // Gather all sections present in the file, which should
        // be TOML-like header format followed by key=value pairs
        let mut current_section = None;
        for line in s.lines() {
            if line.starts_with('[') {
                current_section = Some(
                    line.trim_ascii()
                        .trim_start_matches('[')
                        .trim_end_matches(']'),
                );
            } else if let Some(section) = current_section.as_ref() {
                if let Some((key, value)) = line.split_once('=') {
                    sections
                        .entry(section.to_string())
                        .or_default()
                        .insert(key.trim().to_ascii_lowercase(), value.trim().to_string());
                }
            }
        }

        // We should now have gotten the special "Icon Theme" section and something else
        let Some(header) = sections.remove(THEME_HEADER_NAME) else {
            return Err(CustomThemeError::Header);
        };
        if sections.is_empty() {
            return Err(CustomThemeError::Empty);
        }

        Ok(Self { header, sections })
    }
}
