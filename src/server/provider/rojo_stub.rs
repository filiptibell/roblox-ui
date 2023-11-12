use std::path::{Path, PathBuf};

use futures::future::{join_all, BoxFuture, FutureExt};
use once_cell::sync::Lazy;
use rbx_reflection::{ClassTag, ReflectionDatabase};
use tokio::fs;

use super::{InstanceNode, RojoProjectFile, RojoProjectFileNode};

static CLASS_DATABASE: Lazy<&ReflectionDatabase> = Lazy::new(rbx_reflection_database::get);

// Max generation depth is limited here since the user will probably not navigate
// more than this amount of levels deep before we get some real info back from Rojo,
// and limiting the depth also limits any filesystem traversal that we have to do
const MAX_DEPTH: usize = 3;

const CLASS_NAME_SUFFIXES: &[(&str, &str)] = &[
    (".server.luau", "Script"),
    (".server.lua", "Script"),
    (".client.luau", "LocalScript"),
    (".client.lua", "LocalScript"),
    (".luau", "ModuleScript"),
    (".lua", "ModuleScript"),
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
    generate_project_node_instance(&project_file.name, project_file.tree.clone(), 1, false).await
}

fn generate_project_node_instance(
    name: impl Into<String>,
    node: RojoProjectFileNode,
    current_depth: usize,
    parent_is_datamodel: bool,
) -> BoxFuture<'static, Option<InstanceNode>> {
    let name = name.into();
    let mut class_name = node.class_name.clone().or_else(|| {
        if parent_is_datamodel && is_service_class_name(&name) {
            Some(name.clone())
        } else {
            None
        }
    });

    let fut = async move {
        // NOTE: If we can't figure out the class name from the project node,
        // we will try to use any file path it has to figure it out instead
        if class_name.is_none() {
            if let Some(path) = node.path.as_deref() {
                if let Some(class) = class_name_from_path(path).await {
                    class_name.replace(class.to_string());
                }
            }
        }

        let class_name = class_name?;
        let is_data_model = class_name == "DataModel";

        let mut file_paths = Vec::new();
        if let Some(path) = node.path.as_deref() {
            file_paths.push(path.to_path_buf())
        }

        let mut children = Vec::new();

        if current_depth > MAX_DEPTH {
            Some(InstanceNode {
                class_name,
                name,
                children,
                file_paths,
            })
        } else {
            // Add children from direct project nodes
            let mut child_futs = Vec::new();
            for (key, value) in &node.other_fields {
                // Misc metadata keys that we can skip are prefixed with $
                if matches!(key, s if s.starts_with('$')) {
                    continue;
                }
                let child_node: RojoProjectFileNode = match serde_json::from_value(value.clone()) {
                    Err(_) => continue,
                    Ok(node) => node,
                };
                child_futs.push(generate_project_node_instance(
                    key,
                    child_node,
                    current_depth + 1,
                    is_data_model,
                ));
            }
            children.extend(join_all(child_futs).await.into_iter().flatten());

            // Add children by simple file name matching if we got a dir path
            if let Some(path) = node.path.as_deref() {
                children.extend(child_nodes_for_path(path).await);
            }

            // Return the new full node with children
            Some(InstanceNode {
                class_name,
                name,
                children,
                file_paths,
            })
        }
    };

    fut.boxed()
}

// Please don't look at anything below, it is mostly hacked together ... thank you

fn file_name_str(path: &Path) -> Option<&str> {
    path.file_name().and_then(|f| f.to_str())
}

async fn read_dir_all(path: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut entries = match fs::read_dir(path).await {
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

fn is_service_class_name(name: impl AsRef<str>) -> bool {
    if let Some(desc) = CLASS_DATABASE.classes.get(name.as_ref()) {
        if desc.tags.contains(&ClassTag::Service) {
            return true;
        }
    }
    false
}

fn class_name_from_file_name(name: impl AsRef<str>) -> Option<&'static str> {
    let parts = name.as_ref().split('.').collect::<Vec<_>>();
    let last = parts.iter().nth_back(0).copied()?;
    let second_last = parts.iter().nth_back(1).copied();
    match last {
        "luau" | "lua" => Some(match second_last {
            Some("server") => "Script",
            Some("client") => "LocalScript",
            _ => "ModuleScript",
        }),
        _ => None,
    }
}

async fn class_name_from_path(path: impl AsRef<Path>) -> Option<&'static str> {
    let path = path.as_ref();
    let meta = match fs::metadata(path).await {
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
        let file_name = file_name_str(path)?;
        class_name_from_file_name(file_name)
    } else {
        None
    }
}

async fn child_nodes_for_path(path: impl AsRef<Path>) -> Vec<InstanceNode> {
    let mut children = Vec::new();

    let path = path.as_ref();
    let meta = match fs::metadata(path).await {
        Err(_) => return children,
        Ok(m) => m,
    };

    if meta.is_dir() {
        for child_path in read_dir_all(path).await {
            if let Some((name, class_name)) = parse_name_and_class_name(&child_path) {
                children.push(InstanceNode {
                    class_name: class_name.to_string(),
                    name: name.to_string(),
                    file_paths: vec![child_path],
                    children: vec![],
                })
            }
        }
    }

    children
}
