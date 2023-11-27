use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;

use rbx_dom_weak::types::Ref;

use crate::util::path::make_absolute_and_clean;

use super::util::*;
use super::Dom;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package: Option<InstanceMetadataPackage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<InstanceMetadataActions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paths: Option<InstanceMetadataPaths>,
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
        let mut package = None;
        let mut actions = InstanceMetadataActions::default();
        let mut paths = InstanceMetadataPaths::default();

        for file_path in file_paths {
            let file_name = match path_file_name(file_path) {
                Some(s) => s,
                None => continue,
            };
            // NOTE: We don't actually check the filesystem for if this is correct, but that's
            // completely fine, since we _should_ be getting valid paths from others already
            if file_name == "wally.toml" {
                paths.wally = Some(file_path.to_owned());
            } else if file_name == "wally.lock" {
                paths.wally_lock = Some(file_path.to_owned());
            } else if file_name.ends_with(".project.json") {
                paths.rojo = Some(file_path.to_owned());
            } else if file_name.ends_with(".meta.json") {
                paths.file_meta = Some(file_path.to_owned());
            } else if is_known_file(file_name) {
                paths.file = Some(file_path.to_owned())
            } else {
                paths.folder = Some(file_path.to_owned())
            }
        }

        // If we got a file path then that means we should also
        // know the parent folder path, so use that as a backup
        if paths.folder.is_none() {
            let any_file_path = paths
                .file
                .as_ref()
                .or(paths.file_meta.as_ref())
                .or(paths.rojo.as_ref())
                .or(paths.wally.as_ref());
            if let Some(file_parent) = any_file_path.and_then(|p| p.parent()) {
                // NOTE: Possible source of TOCTOU bugs, but not
                // adding in any invalid paths is more important
                if file_parent.exists() {
                    paths.folder = Some(file_parent.to_path_buf());
                }
            }
        }

        // For some more metadata we need to check this instance and its parents metadata
        let instance = dom
            .get_instance(id)
            .expect("tried to create metadata for nonexistent instance");
        let parent = dom.get_instance(instance.parent());
        let parent_meta = parent.and_then(|inst| dom.get_metadata(inst.referent()));

        let is_root = matches!(dom.get_root_id(), Some(i) if i == id);
        let is_datamodel = instance.class == "DataModel";

        if let Some(parent_package) = parent_meta.and_then(|meta| meta.package.as_ref()) {
            // If the parent is part of a package, this instance must
            // be too, so there's no need to do more complicated checks
            package = Some(InstanceMetadataPackage::from_parent(parent_package));
        } else if let Some(file) = paths.file.as_deref() {
            // If we got a package file path, we may be able
            // to parse useful Wally package metadata out of it
            package = InstanceMetadataPackage::from_path(file);
        }

        // If we *still* don't have a file or folder path, but we know that this instance
        // is a folder, and the parent has a folder path, we can derive a folder path
        if paths.folder.is_none() && instance.class == "Folder" {
            if let Some(parent_folder) = parent_meta
                .and_then(|meta| meta.paths.as_ref())
                .and_then(|paths| paths.folder.as_deref())
            {
                // NOTE: Possible source of TOCTOU bugs, but not
                // adding in any invalid paths is more important
                let child_folder = parent_folder.join(instance.name.clone());
                if child_folder.exists() {
                    paths.folder = Some(child_folder)
                }
            }
        }

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
            .and_then(|meta| meta.paths.as_ref())
            .map(|paths| paths.folder.is_some())
            .unwrap_or_default();
        actions.can_paste_into = actions.can_insert_object;

        // Only return metadata if it actually has useful data inside of it
        let this = Self {
            package,
            actions: actions.data_or_none(),
            paths: paths.data_or_none().map(|p| p.make_absolute_and_clean()),
        };

        this.data_or_none()
    }

    fn contains_data(&self) -> bool {
        self.package.is_some() || self.actions.is_some() || self.paths.is_some()
    }

    fn data_or_none(self) -> Option<Self> {
        if self.contains_data() {
            Some(self)
        } else {
            None
        }
    }
}

// NOTE: We use Arc<String> here to make cloning much cheaper with package
// metadata cloning for children, all children of a package should have the
// same metadata and we do not need to mutate that package metadata either
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceMetadataPackage {
    /// Scope of the Wally package.
    pub scope: Arc<String>,
    /// Name of the Wally package.
    pub name: Arc<String>,
    /// Version of the Wally package.
    pub version: Arc<String>,
    /// If this instance is the root instance of the Wally package or not.
    #[serde(skip_serializing_if = "is_false")]
    pub is_root: bool,
}

impl InstanceMetadataPackage {
    fn from_parent(parent_meta: &Self) -> Self {
        if parent_meta.is_root {
            Self {
                is_root: false,
                ..parent_meta.clone()
            }
        } else {
            parent_meta.clone()
        }
    }

    fn from_path(path: impl AsRef<Path>) -> Option<Self> {
        let path = path.as_ref();

        // Look for a folder inside of another named '_Index'
        let mut found_index = false;
        let mut found_folder = None;
        let mut found_inner = None;
        for component in path.components() {
            if let Some(component_str) = component.as_os_str().to_str() {
                if !found_index && component_str == "_Index" {
                    found_index = true;
                } else if found_index && found_folder.is_none() {
                    found_folder = Some(component_str);
                } else if found_index && found_folder.is_some() {
                    found_inner = Some(component_str);
                    break;
                }
            } else {
                break;
            }
        }

        // Make sure we got both components
        let package_folder = found_folder?;
        let package_inner = found_inner?;

        // If we found a matching folder, split it by '_' and '@'
        // Example wally package folder name: 'scope_package@1.0.0'
        // Any instance with the exact name of the package inside of
        // that folder is also guaranteed to be the "root" of the package
        if let Some((scope, rest)) = package_folder.split_once('_') {
            if let Some((name, version)) = rest.split_once('@') {
                return Some(Self {
                    scope: Arc::new(scope.to_owned()),
                    name: Arc::new(name.to_owned()),
                    version: Arc::new(version.to_owned()),
                    is_root: package_inner == name,
                });
            }
        }

        None
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
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
    fn contains_data(&self) -> bool {
        self.can_open
            || self.can_move
            || self.can_paste_sibling
            || self.can_paste_into
            || self.can_insert_service
            || self.can_insert_object
    }

    fn data_or_none(self) -> Option<Self> {
        if self.contains_data() {
            Some(self)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
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
    fn contains_data(&self) -> bool {
        self.folder.is_some()
            || self.file.is_some()
            || self.file_meta.is_some()
            || self.rojo.is_some()
            || self.wally.is_some()
            || self.wally_lock.is_some()
    }

    fn data_or_none(self) -> Option<Self> {
        if self.contains_data() {
            Some(self)
        } else {
            None
        }
    }

    fn existing_paths(&self) -> Vec<&Path> {
        let paths_opt = &[
            self.folder.as_deref(),
            self.file.as_deref(),
            self.file_meta.as_deref(),
            self.rojo.as_deref(),
            self.wally.as_deref(),
            self.wally_lock.as_deref(),
        ];
        paths_opt
            .iter()
            .filter_map(|path| *path)
            .collect::<Vec<_>>()
    }

    fn make_absolute_and_clean(mut self) -> Self {
        self.folder = self.folder.map(make_absolute_and_clean);
        self.file = self.file.map(make_absolute_and_clean);
        self.file_meta = self.file_meta.map(make_absolute_and_clean);
        self.rojo = self.rojo.map(make_absolute_and_clean);
        self.wally = self.wally.map(make_absolute_and_clean);
        self.wally_lock = self.wally_lock.map(make_absolute_and_clean);
        self
    }
}

impl<'a> IntoIterator for &'a InstanceMetadataPaths {
    type Item = &'a Path;
    type IntoIter = std::vec::IntoIter<&'a Path>;

    fn into_iter(self) -> Self::IntoIter {
        self.existing_paths().into_iter()
    }
}
