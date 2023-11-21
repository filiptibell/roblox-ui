use std::path::PathBuf;

use serde::{Deserialize, Serialize};

const FILE_PATH_SUFFIXES: &[&str] = &[".luau", ".lua", ".rbxmx", ".rbxm", ".txt", ".csv", ".json"];

fn is_file(file_path: &str) -> bool {
    FILE_PATH_SUFFIXES
        .iter()
        .any(|suffix| file_path.ends_with(suffix))
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceMetadata {
    pub paths: InstanceMetadataPaths,
}

impl InstanceMetadata {
    pub fn from_paths(file_paths: &[PathBuf]) -> Option<Self> {
        let mut paths = InstanceMetadataPaths::default();

        for file_path in file_paths {
            let file_name = match file_path.file_name().and_then(|e| e.to_str()) {
                Some(s) => s,
                None => continue,
            };
            // NOTE: We don't actually check the filesystem for if this is correct, but that's
            // completely fine, since we _should_ be getting valid paths from rojo already
            if file_name.ends_with(".meta.json") {
                paths.file_meta = Some(file_path.to_owned());
            } else if is_file(file_name) {
                paths.file = Some(file_path.to_owned())
            } else {
                paths.folder = Some(file_path.to_owned())
            }
        }

        // NOTE: The metadata with the paths can get quite
        // large, which is why we return option at all here
        if paths.is_empty() {
            None
        } else {
            Some(Self { paths })
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceMetadataPaths {
    pub folder: Option<PathBuf>,     // Directory
    pub file: Option<PathBuf>,       // Main file
    pub file_meta: Option<PathBuf>,  // Any .meta file
    pub rojo: Option<PathBuf>,       // Rojo manifest
    pub wally: Option<PathBuf>,      // Wally manifest
    pub wally_lock: Option<PathBuf>, // Wally lockfile
}

impl InstanceMetadataPaths {
    pub fn is_empty(&self) -> bool {
        self.folder.is_none()
            && self.file.is_none()
            && self.file_meta.is_none()
            && self.rojo.is_none()
            && self.wally.is_none()
            && self.wally_lock.is_none()
    }
}
