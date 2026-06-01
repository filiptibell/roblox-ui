/*!
    Directory rules: a nested project if it contains `default.project.json`, a
    script/table if it has an `init.*`, else a plain `Folder`; recurses into
    children with sibling `.meta.json` overlays.
*/

use std::path::{Path, PathBuf};

use async_fs::{read_dir, read_to_string};
use futures_lite::StreamExt;
use rbx_dom_weak::{types::Variant, Ustr, UstrMap};
use rbx_reflection::ReflectionDatabase;

use roblox_ui_util::{path::make_absolute_and_clean, rojo::file_name_str};

use crate::model::{Snapshot, SourceKey};
use crate::reflect::default_diff;

use super::meta::apply_meta_file;
use super::project::build_project_file_node;
use super::rules::{init_dir_class, is_init_filename, is_model_path, INIT_PRIORITY};
use super::{build_path, BoxFut, EngineOptions};

/**
    The project file that makes its directory *be* that project.
*/
const DEFAULT_PROJECT_NAME: &str = "default.project.json";

pub(crate) fn build_dir<'a>(
    db: &'a ReflectionDatabase,
    opts: &'a EngineOptions,
    path: &'a Path,
) -> BoxFut<'a, Option<Snapshot>> {
    Box::pin(async move {
        let path = make_absolute_and_clean(path);
        let name = file_name_str(&path)?.to_string();
        let entries = read_dir_all(&path).await;

        // A directory containing a default project file *is* that project.
        if let Some(project_file) = entries
            .iter()
            .find(|e| file_name_str(e) == Some(DEFAULT_PROJECT_NAME))
        {
            let mut snapshot = build_project_file_node(
                db,
                opts,
                project_file,
                SourceKey::Path(path.clone()),
                Some(&name),
            )
            .await?;
            snapshot.key = SourceKey::Path(path.clone());
            snapshot.name = name;
            snapshot.file_paths.push(path.clone());
            return Some(snapshot);
        }

        // Partition entries: init scripts, init.meta.json, sibling metas, rest.
        let mut class = Ustr::from("Folder");
        let mut file_paths = vec![path.clone()];
        let mut properties = UstrMap::default();
        let mut init_meta: Option<PathBuf> = None;
        let mut sibling_metas: Vec<(String, PathBuf)> = Vec::new();
        let mut child_paths: Vec<PathBuf> = Vec::new();
        let mut init_files: Vec<(String, PathBuf)> = Vec::new();

        for entry in &entries {
            if opts.is_ignored(entry) {
                continue;
            }
            let fname = match file_name_str(entry) {
                Some(fname) => fname,
                None => continue,
            };
            if fname == "init.meta.json" {
                init_meta = Some(entry.clone());
            } else if let Some(stem) = fname.strip_suffix(".meta.json") {
                sibling_metas.push((stem.to_string(), entry.clone()));
            } else if is_init_filename(fname) {
                init_files.push((fname.to_string(), entry.clone()));
            } else {
                child_paths.push(entry.clone());
            }
        }

        // Promote the directory using the highest-priority init file present.
        if let Some(init_name) = INIT_PRIORITY
            .iter()
            .find(|n| init_files.iter().any(|(f, _)| f == *n))
        {
            let (_, init_path) = init_files
                .iter()
                .find(|(f, _)| f == *init_name)
                .expect("init present");
            if let Some((init_class, is_script)) = init_dir_class(init_name) {
                class = Ustr::from(init_class);
                file_paths.push(init_path.clone());
                if is_script {
                    if let Ok(text) = read_to_string(init_path).await {
                        properties.insert(Ustr::from("Source"), Variant::String(text));
                    }
                }
            }
        }

        let mut snapshot = Snapshot::new(SourceKey::Path(path.clone()), class, name);
        snapshot.file_paths = file_paths;
        snapshot.properties = properties;

        // init.meta.json overlays the directory instance.
        if let Some(meta_path) = init_meta {
            apply_meta_file(db, &meta_path, &mut snapshot, true).await;
        }
        let dir_class = snapshot.class;
        default_diff(db, &dir_class, &mut snapshot.properties);

        // Build children, then apply any matching sibling meta overlay.
        for child_path in child_paths {
            if let Some(mut child) = build_path(db, opts, &child_path).await {
                if let Some((_, meta_path)) =
                    sibling_metas.iter().find(|(stem, _)| *stem == child.name)
                {
                    let allow_props = !file_name_str(&child_path)
                        .map(is_model_path)
                        .unwrap_or(false);
                    apply_meta_file(db, meta_path, &mut child, allow_props).await;
                    let cls = child.class;
                    default_diff(db, &cls, &mut child.properties);
                }
                snapshot.children.push(child);
            }
        }

        Some(snapshot)
    })
}

async fn read_dir_all(path: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut entries = match read_dir(path).await {
        Err(_) => return paths,
        Ok(entries) => entries,
    };
    while let Some(Ok(entry)) = entries.next().await {
        paths.push(entry.path());
    }
    paths
}
