use std::path::{Path, PathBuf};

use futures::future::join_all;
use tokio::fs::{metadata, read_dir};

use super::{InstanceNode, RojoProjectFile, RojoProjectFileNode};

/**
    Max generation depth is limited here since the user will probably not navigate
    more than this amount of levels deep before we get some real info back from Rojo,
    and limiting the depth also limits any filesystem traversal that we have to do
*/
const MAX_DEPTH: usize = 4;

/**
    File extension -> class name conversions as defined by the Rojo spec:

    https://rojo.space/docs/v7/sync-details/

    Note that we intentionally mark any model / meta files as plain instances
    here, since we would have to read their contents to get proper class name
*/
const CLASS_NAME_SUFFIXES: &[(&str, &str)] = &[
    (".server.luau", "Script"),
    (".server.lua", "Script"),
    (".client.luau", "LocalScript"),
    (".client.lua", "LocalScript"),
    (".luau", "ModuleScript"),
    (".lua", "ModuleScript"),
    (".rbxmx", "Instance"),
    (".rbxm", "Instance"),
    (".txt", "StringValue"),
    (".csv", "LocalizationTable"),
    (".model.json", "Instance"),
    (".project.json", "Instance"),
    (".meta.json", "Instance"),
    (".json", "ModuleScript"),
];

/**
    Generates a small portion of the complete instance tree that a Rojo project file may represent.

    Note that this is ***not*** a full reimplementation of Rojo logic, nor is it meant to be.

    This is meant exclusively as a way for Roblox UI to very quickly
    create a basic instance tree that the user can start interacting
    with, while the real Rojo subprocess runs in the background.
*/
pub async fn generate_project_file_instance_tree(
    project_file: &RojoProjectFile,
) -> Option<InstanceNode> {
    generate_project_node_instance(
        project_file.name.to_string(),
        project_file.tree.clone(),
        1,
        false,
    )
    .await
}

#[async_recursion::async_recursion]
async fn generate_project_node_instance(
    name: String,
    node: RojoProjectFileNode,
    current_depth: usize,
    parent_is_datamodel: bool,
) -> Option<InstanceNode> {
    let mut class_name = node.class_name.clone().or_else(|| {
        // HACK: We assume that all children of a DataModel are services which have
        // class names that are the same as their names, this is not necessarily
        // accurate, but to verify this we would have to deserialize and parse
        // the entire rbx-dom database which unnecessarily adds ~10ms to startup
        if parent_is_datamodel {
            Some(name.clone())
        } else {
            None
        }
    });

    // NOTE: If we can't figure out the class name from the project node,
    // we will try to use any file path it has to figure it out instead
    if class_name.is_none() {
        if let Some(path) = node.path.as_deref() {
            if let Some(class) = class_name_from_path(path).await {
                class_name.replace(class.to_owned());
            }
        }
    }

    let class_name = class_name?;
    let mut children = Vec::new();
    let mut file_paths = Vec::new();
    if let Some(path) = node.path.as_deref() {
        file_paths.push(path.to_path_buf())
    }

    if current_depth > MAX_DEPTH {
        Some(InstanceNode {
            class_name,
            name,
            children,
            file_paths,
        })
    } else {
        // Add children from direct project nodes
        let child_futs = node
            .other_fields
            .into_iter()
            .filter(|(key, _)| !matches!(key, s if s.starts_with('$')))
            .filter_map(|(key, value)| {
                match serde_json::from_value::<RojoProjectFileNode>(value.clone()) {
                    Ok(val) => Some((key, val)),
                    Err(_) => None,
                }
            })
            .map(|(key, child_node)| {
                generate_project_node_instance(
                    key,
                    child_node,
                    current_depth + 1,
                    class_name == "DataModel",
                )
            })
            .collect::<Vec<_>>();
        children.extend(join_all(child_futs).await.into_iter().flatten());

        // Add instances from filesystem if we got a path,
        // replicating some very basic behavior from Rojo
        if let Some(path) = node.path.as_deref() {
            children.extend(instance_nodes_at_path(path.to_path_buf(), current_depth).await);
        }

        // Return the new full node with children
        Some(InstanceNode {
            class_name,
            name,
            children,
            file_paths,
        })
    }
}

// Please don't look at anything below, it is mostly hacked together ... thank you

fn file_name_str(path: &Path) -> Option<&str> {
    path.file_name().and_then(|f| f.to_str())
}

async fn read_dir_all(path: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut entries = match read_dir(path).await {
        Err(_) => return paths,
        Ok(e) => e,
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        paths.push(entry.path())
    }
    paths
}

fn parse_name_and_class_name(path: &Path) -> Option<(&str, &'static str)> {
    let file_name = file_name_str(path)?;
    for (suffix, class_name) in CLASS_NAME_SUFFIXES {
        if file_name.ends_with(suffix) {
            let name = file_name.strip_suffix(suffix).unwrap();
            return Some((name, class_name));
        }
    }
    None
}

async fn class_name_from_path(path: impl AsRef<Path>) -> Option<&'static str> {
    let path = path.as_ref();
    let meta = match metadata(path).await {
        Err(_) => return None,
        Ok(m) => m,
    };
    if meta.is_dir() {
        Some(
            read_dir_all(path)
                .await
                .iter()
                .find_map(|p| {
                    if let Some(("init", class_name)) = parse_name_and_class_name(p) {
                        Some(class_name)
                    } else {
                        None
                    }
                })
                .unwrap_or("Folder"),
        )
    } else if meta.is_file() {
        if let Some((_, class_name)) = parse_name_and_class_name(path) {
            Some(class_name)
        } else {
            None
        }
    } else {
        None
    }
}

#[async_recursion::async_recursion]
async fn instance_nodes_at_path(path: PathBuf, current_depth: usize) -> Vec<InstanceNode> {
    let mut children = Vec::new();

    let meta = match metadata(&path).await {
        Err(_) => return children,
        Ok(m) => m,
    };

    if meta.is_dir() && current_depth <= MAX_DEPTH {
        let child_futs = read_dir_all(&path)
            .await
            .into_iter()
            .map(|child_path| instance_nodes_at_path(child_path, current_depth + 1))
            .collect::<Vec<_>>();
        children.extend(join_all(child_futs).await.into_iter().flatten());
    } else if meta.is_file() {
        if let Some((name, class_name)) = parse_name_and_class_name(&path) {
            if name != "init" {
                children.push(InstanceNode {
                    class_name: class_name.to_string(),
                    name: name.to_string(),
                    file_paths: vec![path],
                    children: vec![],
                })
            }
        }
    }

    children
}
