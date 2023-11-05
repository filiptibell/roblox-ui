use std::{cmp::Ordering, net::SocketAddr};

use serde::{Deserialize, Serialize, Serializer};

/**
    Stub representing a rojo project configuration file tree.

    Intentionally omits instance / child definitions.
*/
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RojoProjectFileTree {
    #[serde(rename = "$path")]
    pub path: Option<String>,
    #[serde(rename = "$className")]
    pub class_name: Option<String>,
}

/**
    Stub representing a rojo project configuration file.

    NOTE: Project file structs should only contain the information we
    care about and determine would need to cause a restart of any rojo
    executable command(s), they will be compared in providers using Eq
*/
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RojoProjectFile {
    pub name: String,
    pub tree: RojoProjectFileTree,
    pub serve_address: Option<String>,
    pub serve_port: Option<u16>,
}

impl RojoProjectFile {
    pub fn from_json(json: impl AsRef<str>) -> Result<Self, serde_json::Error> {
        serde_json::from_str::<Self>(json.as_ref())
    }

    pub fn _serve_address(&self) -> Option<SocketAddr> {
        if let Some(addr) = &self.serve_address {
            if let Ok(parsed) = addr.as_str().parse() {
                return Some(parsed);
            }
        }

        self.serve_port
            .as_ref()
            .map(|port| SocketAddr::from(([127, 0, 0, 1], *port)))
    }
}

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
    pub file_paths: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<InstanceNode>,
}

impl InstanceNode {
    pub fn from_json(json: impl AsRef<str>) -> Result<Self, serde_json::Error> {
        let mut node = serde_json::from_str::<Self>(json.as_ref())?;
        node.children.sort();
        Ok(node)
    }

    pub fn diff_full(&self) -> String {
        serde_json::to_string(&InstanceNodeDiffVariant::Full(self))
            .expect("instance node diff should always be serializable")
    }

    pub fn diff_with(&self, other: &Self) -> String {
        let changes = InstanceNodeDiff::new_diff(self, other);
        serde_json::to_string(&InstanceNodeDiffVariant::Diff(changes))
            .expect("instance node diff should always be serializable")
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

/**
    A variant containing the different kinds of node diffs currently implemented.

    - `InstanceNodeDiffVariant::Full` for a full (new) instance tree
    - `InstanceNodeDiffVariant::Diff` for a diff between an old and new instance tree
*/
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "data")]
enum InstanceNodeDiffVariant<'a> {
    Full(&'a InstanceNode),
    Diff(InstanceNodeDiff),
}

/**
    An instance diff node, which can be one of three variants:

    - `InstanceNodeDiff:Unchanged` which serializes as `"U"`
    - `InstanceNodeDiff:Removed` which serializes as `"R"`
    - `InstanceNodeDiff:AddedOrChanged` which serializes as plain data similar to an `InstanceNode`
*/
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
enum InstanceNodeDiff {
    #[serde(serialize_with = "ser_diff_unchanged")]
    Unchanged,
    #[serde(serialize_with = "ser_diff_removed")]
    Removed,
    AddedOrChanged {
        #[serde(skip_serializing_if = "Option::is_none")]
        class_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        file_paths: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        children: Vec<InstanceNodeDiff>,
    },
}

impl InstanceNodeDiff {
    fn new(node: &InstanceNode) -> Self {
        Self::AddedOrChanged {
            class_name: Some(node.class_name.clone()),
            name: Some(node.name.clone()),
            file_paths: node.file_paths.clone(),
            children: Self::child_diff(&[], &node.children),
        }
    }

    fn new_diff(old: &InstanceNode, new: &InstanceNode) -> Self {
        let class_name = if old.class_name != new.class_name {
            Some(new.class_name.clone())
        } else {
            None
        };
        let name = if old.name != new.name {
            Some(new.name.clone())
        } else {
            None
        };

        // NOTE: File paths generally don't change much and are ordered well,
        // so we don't care about doing any special diffing for them here
        let file_paths = if old.file_paths != new.file_paths {
            new.file_paths.clone()
        } else {
            Vec::new()
        };

        let children = Self::child_diff(&old.children, &new.children);
        if class_name.is_some() || name.is_some() || !file_paths.is_empty() || !children.is_empty()
        {
            Self::AddedOrChanged {
                class_name,
                name,
                file_paths,
                children,
            }
        } else {
            Self::Unchanged
        }
    }

    fn child_diff(vec_old: &[InstanceNode], vec_new: &[InstanceNode]) -> Vec<Self> {
        // FUTURE: Improved diffing algorithm for children that
        // might have stayed the same but their index has changed,
        // for now we sort in the from_json constructor which helps
        let len_old = vec_old.len();
        let len_new = vec_new.len();

        let mut children = Vec::new();
        for index in 0..len_old.max(len_new) {
            let item_old = vec_old.get(index);
            let item_new = vec_new.get(index);
            if len_old > index && len_new > index {
                children.push(Self::new_diff(item_old.unwrap(), item_new.unwrap()));
            } else if len_old > index {
                children.push(Self::Removed);
            } else if len_new > index {
                children.push(Self::new(item_new.unwrap()));
            } else {
                unreachable!()
            }
        }

        if children
            .iter()
            .all(|child| matches!(child, Self::Unchanged))
        {
            Vec::new()
        } else {
            children
        }
    }
}

fn ser_diff_unchanged<S: Serializer>(s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str("U")
}

fn ser_diff_removed<S: Serializer>(s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str("R")
}
