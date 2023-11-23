use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use rbx_dom_weak::types::Ref;

use super::util::*;
use super::Dom;

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceMetadata {
    pub actions: InstanceMetadataActions,
    pub paths: InstanceMetadataPaths,
}

impl InstanceMetadata {
    /**
        Creates a new metadata for the given instance `id` in the given `dom`.

        Both the instance and its parent (if any) must be inserted into the dom
        prior to creating this metadata, otherwise this function may panic.

        Returns `None` if all metadata actions and paths are either `false` or `None`,
        which would have also meant that no useful props would have been serializable.
    */
    pub fn new(id: Ref, dom: &Dom, file_paths: &[PathBuf]) -> Option<Self> {
        // OPTIMIZATION: if we got no file paths, we already know that metadata
        // will be empty, since actions all also depend on file paths existing
        if file_paths.is_empty() {
            return None;
        }

        let mut actions = InstanceMetadataActions::default();
        let mut paths = InstanceMetadataPaths::default();

        for file_path in file_paths {
            let file_name = match path_file_name(file_path) {
                Some(s) => s,
                None => continue,
            };
            // NOTE: We don't actually check the filesystem for if this is correct, but that's
            // completely fine, since we _should_ be getting valid paths from rojo already
            if file_name.ends_with(".meta.json") {
                paths.file_meta = Some(file_path.to_owned());
            } else if is_known_file(file_name) {
                paths.file = Some(file_path.to_owned())
            } else {
                paths.folder = Some(file_path.to_owned())
            }
        }

        // For actions metadata we need to check this instance and its parents metadata
        let instance = dom
            .get_instance(id)
            .expect("tried to create metadata for nonexistent instance");
        let parent = dom.get_instance(instance.parent());
        let parent_meta = parent.and_then(|inst| dom.get_metadata(inst.referent()));

        let is_root = matches!(dom.get_root_id(), Some(i) if i == id);
        let is_datamodel = instance.class == "DataModel";

        /*
            A file can be opened by clicking it if it is an "openable" file, this
            means scripts (.lua or .luau) and some others (LocalizationTable .csv)
        */
        actions.can_open = paths
            .file
            .as_deref()
            .and_then(path_file_name)
            .map(is_openable_file)
            .unwrap_or_default();

        /*
            - Objects can be inserted into anything that has a folder path
            - Services can be inserted into datamodel roots with folder paths
        */
        actions.can_insert_object = paths.folder.is_some();
        actions.can_insert_service = actions.can_insert_object && is_datamodel && is_root;

        /*
            - Instances can be moved if both the instance and its parent has either a file or folder
            - Instances can be pasted next to another if that other instance has a parent with a folder
            - Instances can be pasted into another if that other instance has a folder
        */
        actions.can_move = parent.is_some() && (paths.file.is_some() || paths.folder.is_some());
        actions.can_paste_sibling = parent_meta
            .map(|meta| meta.paths.folder.is_some())
            .unwrap_or_default();
        actions.can_paste_into = actions.can_insert_object;

        if actions.contains_serializable_props() || paths.contains_serializable_props() {
            Some(Self { actions, paths })
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceMetadataActions {
    /// If the instance can be "opened" by directly clicking on it or not.
    #[serde(skip_serializing_if = "is_false")]
    pub can_open: bool,
    /// If the instance can be moved or not.
    /// This includes actions such as drag & drop, copy, and cut.
    #[serde(skip_serializing_if = "is_false")]
    pub can_move: bool,
    /// If an instance can be pasted as a sibling (next to this instance) or not.
    #[serde(skip_serializing_if = "is_false")]
    pub can_paste_sibling: bool,
    /// If an instance can be pasted as a child of this instance or not.
    #[serde(skip_serializing_if = "is_false")]
    pub can_paste_into: bool,
    /// If a service can be inserted as a child of this instance or not.
    #[serde(skip_serializing_if = "is_false")]
    pub can_insert_service: bool,
    /// If a non-service / normal instance can be inserted as a child of this instance or not.
    #[serde(skip_serializing_if = "is_false")]
    pub can_insert_object: bool,
}

impl InstanceMetadataActions {
    fn contains_serializable_props(&self) -> bool {
        self.can_open
            || self.can_move
            || self.can_paste_sibling
            || self.can_paste_into
            || self.can_insert_service
            || self.can_insert_object
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceMetadataPaths {
    /// Source directory of the instance. Should only be present if the
    /// instance was created by a file, or a directory with only a meta file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder: Option<PathBuf>,
    /// Main file that created this instance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<PathBuf>,
    /// Metadata file (`.meta.json`) for this instance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_meta: Option<PathBuf>,
    /// Path to the Rojo manifest, if the instance came from Rojo.
    /// Only present for the root instance / dom root metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rojo: Option<PathBuf>,
    /// Path to the Wally manifest, if one was found.
    /// Only present for the root instance / dom root metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wally: Option<PathBuf>,
    /// Path to the Wally lockfile, if one was found.
    /// Only present for the root instance / dom root metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wally_lock: Option<PathBuf>,
}

impl InstanceMetadataPaths {
    fn contains_serializable_props(&self) -> bool {
        self.folder.is_some()
            || self.file.is_some()
            || self.file_meta.is_some()
            || self.rojo.is_some()
            || self.wally.is_some()
            || self.wally_lock.is_some()
    }
}
