/*!
    `.meta.json` overlays: `className` (always) and `properties`/`attributes`
    (unless the base came from a model file).
*/

use std::path::Path;

use async_fs::read_to_string;
use rbx_dom_weak::Ustr;
use rbx_reflection::ReflectionDatabase;
use serde::Deserialize;
use serde_json::{Map as JsonMap, Value as Json};

use crate::model::Snapshot;

use super::value::{apply_json_properties, resolve_attributes};

#[derive(Debug, Deserialize, Default)]
struct MetaJson {
    #[serde(default, rename = "className", alias = "ClassName")]
    class_name: Option<Ustr>,
    #[serde(default, rename = "properties", alias = "Properties")]
    properties: JsonMap<String, Json>,
    #[serde(default, rename = "attributes", alias = "Attributes")]
    attributes: JsonMap<String, Json>,
}

/**
    Apply a `.meta.json` overlay onto a snapshot in place. `className` is always
    honoured; `properties`/`attributes` only when `allow_props` (Rojo forbids
    meta properties on model files).
*/
pub(crate) async fn apply_meta_file(
    db: &ReflectionDatabase<'_>,
    meta_path: &Path,
    snapshot: &mut Snapshot,
    allow_props: bool,
) {
    let text = match read_to_string(meta_path).await {
        Ok(text) => text,
        Err(_) => return,
    };
    let meta: MetaJson = match serde_json::from_str(&super::jsonc::strip(&text)) {
        Ok(meta) => meta,
        Err(_) => return,
    };

    if let Some(class) = meta.class_name {
        snapshot.class = class;
    }
    if allow_props {
        let class = snapshot.class;
        apply_json_properties(db, &class, &meta.properties, &mut snapshot.properties);
        if let Some(attrs) = resolve_attributes(db, &meta.attributes) {
            snapshot.properties.insert(Ustr::from("Attributes"), attrs);
        }
    }
    snapshot.file_paths.push(meta_path.to_path_buf());
}
