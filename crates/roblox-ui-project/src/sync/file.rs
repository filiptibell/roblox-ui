/*!
    Single-file rules: dispatch a classified file to scripts, models, nested
    projects, or simple value instances.
*/

use std::path::Path;

use async_fs::read_to_string;
use rbx_dom_weak::{types::Variant, Ustr};
use rbx_reflection::ReflectionDatabase;

use roblox_ui_util::{path::make_absolute_and_clean, rojo::file_name_str};

use crate::model::{Snapshot, SourceKey};
use crate::reflect::default_diff;

use super::model::{build_model_json, build_rbxm, build_rbxmx};
use super::project::build_project_file_node;
use super::rules::{classify_file, FileRule};
use super::EngineOptions;

pub(crate) async fn build_file(
    db: &ReflectionDatabase<'_>,
    opts: &EngineOptions,
    path: &Path,
) -> Option<Snapshot> {
    let path = make_absolute_and_clean(path);
    let fname = file_name_str(&path)?;
    let (name, rule) = classify_file(fname)?;

    match rule {
        // Consumed by their directory / not an instance on their own.
        FileRule::Init | FileRule::Meta => None,

        FileRule::Rbxmx => build_rbxmx(db, &path, name).await,
        FileRule::Rbxm => build_rbxm(db, &path, name).await,
        FileRule::ModelJson => build_model_json(db, &path, name).await,
        FileRule::Project => {
            build_project_file_node(db, opts, &path, SourceKey::Path(path.clone()), Some(name))
                .await
        }

        FileRule::Script(class) => Some(value_instance(db, &path, name, class, "Source").await),
        FileRule::StringValue => {
            Some(value_instance(db, &path, name, "StringValue", "Value").await)
        }
        FileRule::LocalizationTable => Some(leaf(&path, name, "LocalizationTable")),
        // .json / .jsonc / .toml / .yaml → a ModuleScript that returns the
        // decoded table. We model the class; the generated source body is not
        // needed by an explorer/property consumer.
        FileRule::Module => Some(leaf(&path, name, "ModuleScript")),
    }
}

/**
    A leaf instance backed by a single file, with no read properties.
*/
fn leaf(path: &Path, name: &str, class: &str) -> Snapshot {
    let mut snapshot = Snapshot::new(SourceKey::Path(path.to_path_buf()), Ustr::from(class), name);
    snapshot.file_paths.push(path.to_path_buf());
    snapshot
}

/**
    A leaf instance whose file text becomes the `prop` property (e.g. a script's
    `Source`, or a StringValue's `Value`).
*/
async fn value_instance(
    db: &ReflectionDatabase<'_>,
    path: &Path,
    name: &str,
    class: &str,
    prop: &str,
) -> Snapshot {
    let mut snapshot = leaf(path, name, class);
    if let Ok(text) = read_to_string(path).await {
        snapshot
            .properties
            .insert(Ustr::from(prop), Variant::String(text));
    }
    default_diff(db, class, &mut snapshot.properties);
    snapshot
}
