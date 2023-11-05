use std::{
    num::NonZeroU64,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use serde::{Deserialize, Serialize};

static ID_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourcemapNode {
    pub name: String,
    pub class_name: String,
    #[serde(default)]
    pub file_paths: Vec<PathBuf>,
    #[serde(default)]
    pub children: Vec<SourcemapNode>,
}

// NOTE: Project file structs should only contain the information we
// care about and determine would need to cause a restart of any rojo
// executable command(s), they will be compared in providers using Eq
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RojoProjectFile {
    pub name: String,
    pub tree: RojoProjectFileTree,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RojoProjectFileTree {
    #[serde(rename = "$path")]
    pub path: Option<String>,
    #[serde(rename = "$className")]
    pub class_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstanceNode {
    pub id: Option<NonZeroU64>,
    pub name: String,
    pub class_name: String,
    pub file_paths: Vec<PathBuf>,
    pub children: Vec<InstanceNode>,
}

impl InstanceNode {
    pub fn get_or_generate_id(&mut self) -> NonZeroU64 {
        match self.id {
            Some(i) => i,
            None => {
                let i = ID_COUNTER.fetch_add(1, Ordering::SeqCst);
                let i = NonZeroU64::try_from(i).expect("overflow");
                self.id = Some(i);
                i
            }
        }
    }
}

impl From<SourcemapNode> for InstanceNode {
    fn from(value: SourcemapNode) -> Self {
        Self {
            id: None,
            name: value.name,
            class_name: value.class_name,
            file_paths: value.file_paths,
            children: value.children.into_iter().map(InstanceNode::from).collect(),
        }
    }
}
