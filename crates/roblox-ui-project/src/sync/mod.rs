/*!
    The in-house file -> instance-DOM sync engine: our own, Rojo-compatible
    implementation of the file -> instance "sync rules". It never invokes the
    `rojo` binary and never reimplements any rbx-* crate — model files are parsed
    with `rbx_xml` / `rbx_binary` and property types are disambiguated against
    `rbx_reflection_database`. The result is a `Snapshot` tree of typed `Variant`
    properties. Rule coverage mirrors Rojo 7's `snapshot_middleware`.
*/

use std::future::Future;
use std::path::Path;
use std::pin::Pin;

use async_fs::metadata;
use rbx_reflection::ReflectionDatabase;

use roblox_ui_util::{path::make_absolute_and_clean, rojo::file_name_str};

use crate::model::Snapshot;
use crate::reflect::{database, default_diff};

mod dir;
mod file;
mod jsonc;
mod meta;
mod model;
mod project;
mod rules;
mod value;

pub(crate) use project::ProjectFile;

pub use project::build_project;

/**
    A boxed future, used to break the recursion between the directory / file /
    project builders (each of which can re-enter the others).
*/
type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/**
    Options controlling how the engine walks the filesystem.
*/
#[derive(Debug, Clone, Default)]
pub struct EngineOptions {
    /// Glob patterns (relative or absolute) whose matches are skipped.
    pub ignore_globs: Vec<glob::Pattern>,
}

impl EngineOptions {
    fn is_ignored(&self, path: &Path) -> bool {
        if path.file_name().and_then(|n| n.to_str()) == Some(".git") {
            return true;
        }
        self.ignore_globs.iter().any(|g| g.matches_path(path))
    }
}

/**
    Build a single standalone node for `path` (file or directory), applying any
    sibling `*.meta.json` overlay just as the owning directory would. Used by the
    incremental fast path for content edits.
*/
pub async fn build_node(opts: &EngineOptions, path: &Path) -> Option<Snapshot> {
    let db = database();
    let path = make_absolute_and_clean(path);
    let mut snapshot = build_path(db, opts, &path).await?;

    if let Some(parent) = path.parent() {
        let meta_path = parent.join(format!("{}.meta.json", snapshot.name));
        if metadata(&meta_path).await.is_ok() {
            let allow_props = !file_name_str(&path)
                .map(rules::is_model_path)
                .unwrap_or(false);
            meta::apply_meta_file(db, &meta_path, &mut snapshot, allow_props).await;
            let class = snapshot.class;
            default_diff(db, &class, &mut snapshot.properties);
        }
    }

    snapshot.sort();
    Some(snapshot)
}

/**
    Build a snapshot from a filesystem path, dispatching on file vs directory.
*/
fn build_path<'a>(
    db: &'a ReflectionDatabase,
    opts: &'a EngineOptions,
    path: &'a Path,
) -> BoxFut<'a, Option<Snapshot>> {
    Box::pin(async move {
        let meta = metadata(path).await.ok()?;
        if meta.is_dir() {
            dir::build_dir(db, opts, path).await
        } else if meta.is_file() {
            file::build_file(db, opts, path).await
        } else {
            None
        }
    })
}
