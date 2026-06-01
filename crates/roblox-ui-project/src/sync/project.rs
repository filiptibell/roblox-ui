// Parsing of `*.project.json` files and building of the project tree (services,
// `$path` nodes, `$properties`/`$attributes`, nested projects).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use async_fs::read_to_string;
use rbx_dom_weak::Ustr;
use rbx_reflection::ReflectionDatabase;
use serde::Deserialize;
use serde_json::{Map as JsonMap, Value as Json};

use roblox_ui_util::path::make_absolute_and_clean;

use crate::model::{Snapshot, SourceKey};
use crate::reflect::{database, default_diff, is_service};

use super::value::{apply_json_properties, resolve_attributes};
use super::{build_path, BoxFut, EngineOptions};

/**
    A node in a `*.project.json` tree.

    Mirrors the Rojo project format: the `$`-prefixed keys are meta-fields and
    every other key is a named child node. We parse but never *execute* Rojo —
    this is our own, Rojo-compatible reader.
*/
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub(crate) struct ProjectNode {
    #[serde(rename = "$path")]
    pub path: Option<PathBuf>,
    #[serde(rename = "$className")]
    pub class_name: Option<Ustr>,
    #[serde(rename = "$properties")]
    pub properties: JsonMap<String, Json>,
    #[serde(rename = "$attributes")]
    pub attributes: JsonMap<String, Json>,
    #[serde(rename = "$ignoreUnknownInstances")]
    pub ignore_unknown_instances: Option<bool>,
    /// Named child nodes (everything not starting with `$`).
    #[serde(flatten)]
    pub other: JsonMap<String, Json>,
}

impl ProjectNode {
    /**
        The named children of this node, parsed into `(name, node)` pairs.
    */
    pub(crate) fn children(&self) -> Vec<(String, ProjectNode)> {
        self.other
            .iter()
            .filter(|(key, _)| !key.starts_with('$'))
            .filter_map(|(key, value)| {
                serde_json::from_value::<ProjectNode>(value.clone())
                    .ok()
                    .map(|node| (key.clone(), node))
            })
            .collect()
    }

    /**
        Resolve this node's `$path` against the project's base directory.
    */
    pub(crate) fn resolved_path(&self, base_dir: &Path) -> Option<PathBuf> {
        self.path.as_ref().map(|p| {
            if p.is_absolute() {
                make_absolute_and_clean(p)
            } else {
                make_absolute_and_clean(base_dir.join(p))
            }
        })
    }
}

/**
    A parsed `*.project.json` file plus the context needed to resolve it.
*/
#[derive(Debug, Clone)]
pub(crate) struct ProjectFile {
    pub name: String,
    pub tree: ProjectNode,
    /// Directory the project file lives in; relative `$path`s resolve against it.
    pub base_dir: PathBuf,
    pub path: PathBuf,
    /// Glob patterns the project asks the engine/watcher to ignore.
    pub glob_ignore_paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ProjectFileRaw {
    #[serde(default)]
    name: String,
    #[serde(default)]
    tree: ProjectNode,
    #[serde(default, rename = "globIgnorePaths")]
    glob_ignore_paths: Vec<String>,
}

impl ProjectFile {
    pub(crate) fn parse(path: impl AsRef<Path>, json: impl AsRef<str>) -> Result<Self> {
        let path = make_absolute_and_clean(path.as_ref());
        let raw: ProjectFileRaw = serde_json::from_str(&super::jsonc::strip(json.as_ref()))
            .context("failed to parse project file json")?;
        let base_dir = path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        Ok(Self {
            name: raw.name,
            tree: raw.tree,
            base_dir,
            path,
            glob_ignore_paths: raw.glob_ignore_paths,
        })
    }

    /**
        Every existing directory referenced by a `$path` anywhere in the tree.
        Used to decide which roots the watcher must observe (a `$path` may point
        outside the project file's own directory).
    */
    pub(crate) fn path_roots(&self) -> Vec<PathBuf> {
        let mut roots = Vec::new();
        collect_path_roots(&self.tree, &self.base_dir, &mut roots);
        roots
    }
}

fn collect_path_roots(node: &ProjectNode, base_dir: &Path, out: &mut Vec<PathBuf>) {
    if let Some(path) = node.resolved_path(base_dir) {
        out.push(path);
    }
    for (_, child) in node.children() {
        collect_path_roots(&child, base_dir, out);
    }
}

/**
    Build the full instance tree for a project, returning the root snapshot.
*/
pub async fn build_project(project: &ProjectFile, opts: &EngineOptions) -> Option<Snapshot> {
    let db = database();
    let mut root = build_project_node(
        db,
        opts,
        SourceKey::Root,
        &project.name,
        &project.tree,
        &project.base_dir,
        false,
    )
    .await?;
    root.file_paths.push(project.path.clone());
    root.sort();
    Some(root)
}

/**
    Build a snapshot for a node declared in the project tree.
*/
pub(crate) fn build_project_node<'a>(
    db: &'a ReflectionDatabase,
    opts: &'a EngineOptions,
    key: SourceKey,
    name: &'a str,
    node: &'a ProjectNode,
    base_dir: &'a Path,
    parent_is_datamodel: bool,
) -> BoxFut<'a, Option<Snapshot>> {
    Box::pin(async move {
        // If the node points at a path, start from the filesystem snapshot and
        // then re-key it to follow the project position (stable identity).
        let mut snapshot = if let Some(path) = node.resolved_path(base_dir) {
            build_path(db, opts, &path).await.map(|mut s| {
                s.key = key.clone();
                s
            })
        } else {
            None
        };

        // Resolve the class name (Rojo order): explicit `$className` wins; else
        // the path-derived class unless it is a plain `Folder` under a DataModel
        // and the name is a known Service; else a Service shorthand; else Folder.
        let path_class = snapshot.as_ref().map(|s| s.class);
        let class = node.class_name.unwrap_or_else(|| match path_class {
            Some(c) if c == "Folder" && parent_is_datamodel && is_service(db, name) => {
                Ustr::from(name)
            }
            Some(c) => c,
            None if parent_is_datamodel && is_service(db, name) => Ustr::from(name),
            None => Ustr::from("Folder"),
        });

        let mut snapshot = snapshot.take().unwrap_or_else(|| {
            let mut s = Snapshot::new(key.clone(), class, name);
            if let Some(path) = node.resolved_path(base_dir) {
                s.file_paths.push(path);
            }
            s
        });

        snapshot.key = key.clone();
        snapshot.class = class;
        snapshot.name = name.to_string();

        // Overlay project-declared properties + attributes on top.
        apply_json_properties(db, &class, &node.properties, &mut snapshot.properties);
        if let Some(attrs) = resolve_attributes(db, &node.attributes) {
            snapshot.properties.insert(Ustr::from("Attributes"), attrs);
        }
        default_diff(db, &class, &mut snapshot.properties);

        // Append inline project children.
        let is_datamodel = class == "DataModel";
        for (child_name, child_node) in node.children() {
            let child_key = SourceKey::project_child(&key, Ustr::from(child_name.as_str()));
            if let Some(child) = build_project_node(
                db,
                opts,
                child_key,
                &child_name,
                &child_node,
                base_dir,
                is_datamodel,
            )
            .await
            {
                snapshot.children.push(child);
            }
        }

        Some(snapshot)
    })
}

/**
    Build a snapshot from a nested `*.project.json` file, keyed by `key` and
    (optionally) renamed to `name_override` (e.g. the directory/child name).
*/
pub(crate) fn build_project_file_node<'a>(
    db: &'a ReflectionDatabase,
    opts: &'a EngineOptions,
    file: &'a Path,
    key: SourceKey,
    name_override: Option<&'a str>,
) -> BoxFut<'a, Option<Snapshot>> {
    Box::pin(async move {
        let text = read_to_string(file).await.ok()?;
        let project = ProjectFile::parse(file, &text).ok()?;
        let name = name_override.unwrap_or(&project.name);
        let mut snapshot =
            build_project_node(db, opts, key, name, &project.tree, &project.base_dir, false)
                .await?;
        // Editing the project file should recompute this node.
        snapshot.file_paths.push(file.to_path_buf());
        Some(snapshot)
    })
}
