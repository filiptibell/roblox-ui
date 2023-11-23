use std::{cmp::Ordering, path::PathBuf};

use serde::{Deserialize, Serialize};

/**
    A node representing an instance and its children.

    Analogous and currently identical in structure to a Rojo sourcemap node.
*/
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceNode {
    // TODO: Do some benchmarking and see if it would be better to
    // use one of Arc<str>, Rc<str>, Cow<str> for all this string data
    pub class_name: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub file_paths: Vec<PathBuf>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<InstanceNode>,
}

impl InstanceNode {
    fn sort_inner(&mut self) {
        for child in &mut self.children {
            child.sort_inner();
        }
        self.children.sort();
    }

    pub fn from_json(json: impl AsRef<str>) -> Result<Self, serde_json::Error> {
        let mut node = serde_json::from_str::<Self>(json.as_ref())?;
        node.sort_inner();
        Ok(node)
    }

    /**
        Merges a different stub instance node into this one.

        This will add file paths from the other instance node to
        this one and the children of this node, recursively, if
        this node does not have any file paths present.

        Used for Rojo sourcemaps where some paths may be missing in
        the sourcemap but can be found in the project manifest stub.
    */
    pub(crate) fn merge_stub(&mut self, other: &Self) {
        if self.file_paths.is_empty() && !other.file_paths.is_empty() {
            self.file_paths.extend_from_slice(&other.file_paths);
        }
        for child in self.children.iter_mut() {
            let other_child = other
                .children
                .iter()
                .find(|c| c.class_name == child.class_name && c.name == child.name);
            if let Some(other_child) = other_child {
                child.merge_stub(other_child);
            }
        }
    }
}

impl Ord for InstanceNode {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialOrd for InstanceNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
