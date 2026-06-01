/*!
    Model-file rules: `.rbxmx` / `.rbxm` (parsed via rbx_xml / rbx_binary) and
    `.model.json(c)`. Each yields a subtree carrying full typed properties; the
    root is named after the file (Rojo behaviour) and owns it for the reverse
    index, while descendants are keyed by their index path within the file.
*/

use std::path::Path;

use async_fs::{read, read_to_string};
use rbx_dom_weak::{types::Ref, Ustr, WeakDom};
use rbx_reflection::ReflectionDatabase;
use serde::Deserialize;
use serde_json::{Map as JsonMap, Value as Json};

use crate::model::{Snapshot, SourceKey};
use crate::reflect::default_diff;

use super::value::{apply_json_properties, resolve_attributes};

/**
    Build a snapshot from an `.rbxmx` (XML) model file named `name`.
*/
pub(crate) async fn build_rbxmx(
    db: &ReflectionDatabase<'_>,
    file: &Path,
    name: &str,
) -> Option<Snapshot> {
    let bytes = read(file).await.ok()?;
    let dom = rbx_xml::from_reader_default(bytes.as_slice()).ok()?;
    graft_model(db, file, name, &dom)
}

/**
    Build a snapshot from an `.rbxm` (binary) model file named `name`.
*/
pub(crate) async fn build_rbxm(
    db: &ReflectionDatabase<'_>,
    file: &Path,
    name: &str,
) -> Option<Snapshot> {
    let bytes = read(file).await.ok()?;
    let dom = rbx_binary::from_reader(bytes.as_slice()).ok()?;
    graft_model(db, file, name, &dom)
}

/**
    Build a snapshot from a `.model.json(c)` file named `name`.
*/
pub(crate) async fn build_model_json(
    db: &ReflectionDatabase<'_>,
    file: &Path,
    name: &str,
) -> Option<Snapshot> {
    let text = read_to_string(file).await.ok()?;
    let model: ModelJson = serde_json::from_str(&super::jsonc::strip(&text)).ok()?;
    let mut snapshot = build_model_json_node(db, file, &model, &[]);
    snapshot.key = SourceKey::Path(file.to_path_buf());
    // The filename names the root (a top-level `name` is honoured but deprecated).
    snapshot.name = model.name.clone().unwrap_or_else(|| name.to_string());
    snapshot.file_paths = vec![file.to_path_buf()];
    Some(snapshot)
}

/**
    Graft a parsed model `WeakDom`'s first top-level instance into a snapshot.
*/
fn graft_model(
    db: &ReflectionDatabase<'_>,
    file: &Path,
    name: &str,
    dom: &WeakDom,
) -> Option<Snapshot> {
    let tops = dom.root().children();
    let top_ref = *tops.first()?;
    let mut snapshot = build_nested(db, file, dom, top_ref, &[]);
    snapshot.key = SourceKey::Path(file.to_path_buf());
    snapshot.name = name.to_string();
    snapshot.file_paths = vec![file.to_path_buf()];
    Some(snapshot)
}

fn build_nested(
    db: &ReflectionDatabase<'_>,
    file: &Path,
    dom: &WeakDom,
    referent: Ref,
    index_path: &[u32],
) -> Snapshot {
    let inst = dom
        .get_by_ref(referent)
        .expect("dangling ref in parsed model");
    let key = if index_path.is_empty() {
        SourceKey::Path(file.to_path_buf())
    } else {
        SourceKey::nested(file.to_path_buf(), index_path)
    };

    let mut properties = inst.properties.clone();
    properties.remove(&Ustr::from("Name"));
    default_diff(db, &inst.class, &mut properties);

    let mut snapshot = Snapshot::new(key, inst.class, inst.name.clone());
    snapshot.properties = properties;

    for (i, child_ref) in inst.children().iter().enumerate() {
        let mut child_path = index_path.to_vec();
        child_path.push(i as u32);
        snapshot
            .children
            .push(build_nested(db, file, dom, *child_ref, &child_path));
    }

    snapshot
}

#[derive(Debug, Deserialize)]
struct ModelJson {
    #[serde(rename = "className", alias = "ClassName")]
    class_name: Option<Ustr>,
    #[serde(default, rename = "name", alias = "Name")]
    name: Option<String>,
    #[serde(default, rename = "properties", alias = "Properties")]
    properties: JsonMap<String, Json>,
    #[serde(default, rename = "attributes", alias = "Attributes")]
    attributes: JsonMap<String, Json>,
    #[serde(default, rename = "children", alias = "Children")]
    children: Vec<ModelJson>,
}

fn build_model_json_node(
    db: &ReflectionDatabase<'_>,
    file: &Path,
    model: &ModelJson,
    index_path: &[u32],
) -> Snapshot {
    let class = model.class_name.unwrap_or_else(|| Ustr::from("Folder"));
    let key = if index_path.is_empty() {
        SourceKey::Path(file.to_path_buf())
    } else {
        SourceKey::nested(file.to_path_buf(), index_path)
    };
    let name = model.name.clone().unwrap_or_else(|| class.to_string());

    let mut snapshot = Snapshot::new(key, class, name);
    apply_json_properties(db, &class, &model.properties, &mut snapshot.properties);
    if let Some(attrs) = resolve_attributes(db, &model.attributes) {
        snapshot.properties.insert(Ustr::from("Attributes"), attrs);
    }
    default_diff(db, &class, &mut snapshot.properties);

    for (i, child) in model.children.iter().enumerate() {
        let mut child_path = index_path.to_vec();
        child_path.push(i as u32);
        snapshot
            .children
            .push(build_model_json_node(db, file, child, &child_path));
    }

    snapshot
}
