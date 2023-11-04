use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourcemapNode {
    name: String,
    class_name: String,
    #[serde(default)]
    file_paths: Vec<PathBuf>,
    #[serde(default)]
    children: Vec<SourcemapNode>,
}
