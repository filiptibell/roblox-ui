#![allow(clippy::unnecessary_to_owned)]

use std::path::{Path, PathBuf};

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rbx_dom_weak::{types::Ref, Instance, InstanceBuilder, WeakDom};
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};
use serde::{Deserialize, Serialize};

use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

mod fs;
mod meta;
mod node;
mod query;
mod util;

pub use meta::*;
pub use node::*;
pub use query::*;

use super::Config;
use crate::util::path::make_absolute_and_clean;

// NOTE: If anyone ever names their root instance this, things may break... let's hope they don't
const DOM_ROOT_NAME_NONE: &str = "<|<|<|ROOT|>|>|>";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MatchFilter {
    VeryStrict,
    Strict,
    Any,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", content = "data")]
pub enum DomNotification {
    Changed {
        id: Ref,
        #[serde(skip_serializing_if = "Option::is_none", rename = "className")]
        class_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    Added {
        #[serde(skip_serializing_if = "Option::is_none", rename = "parentId")]
        parent_id: Option<Ref>,
        #[serde(rename = "childId")]
        child_id: Ref,
    },
    Removed {
        #[serde(skip_serializing_if = "Option::is_none", rename = "parentId")]
        parent_id: Option<Ref>,
        #[serde(rename = "childId")]
        child_id: Ref,
    },
}

#[derive(Debug)]
pub struct Dom {
    _config: Config,
    inner: WeakDom,
    ids: HashSet<Ref>,
    metas: HashMap<Ref, InstanceMetadata>,
    path_map: HashMap<PathBuf, Ref>,
    root_meta: InstanceMetadata,
    notification_tx: UnboundedSender<DomNotification>,
    notification_rx: Option<UnboundedReceiver<DomNotification>>,
}

impl Dom {
    pub fn new(config: Config) -> Self {
        let (notification_tx, notification_rx) = unbounded_channel();
        Self {
            _config: config,
            inner: WeakDom::new(InstanceBuilder::new(DOM_ROOT_NAME_NONE)),
            ids: HashSet::default(),
            metas: HashMap::default(),
            path_map: HashMap::default(),
            root_meta: InstanceMetadata::default(),
            notification_tx,
            notification_rx: Some(notification_rx),
        }
    }

    pub fn take_notification_receiver(&mut self) -> Option<UnboundedReceiver<DomNotification>> {
        self.notification_rx.take()
    }

    fn notify(&self, notification: DomNotification) {
        // NOTE: Not having any listeners is fine and is the only error case
        self.notification_tx.send(notification).ok();
    }

    fn insert_instance_into_dom(&mut self, parent_id: Ref, node: InstanceNode) -> Ref {
        let inst = InstanceBuilder::new(node.class_name).with_name(node.name);
        let id = self.inner.insert(parent_id, inst);

        if let Some(meta) = InstanceMetadata::new(id, self, &node.file_paths) {
            if let Some(paths) = &meta.paths {
                for path in paths {
                    self.path_map.insert(path.to_path_buf(), id);
                }
            }
            self.metas.insert(id, meta);
        }

        // NOTE: Children must be inserted *after* this new instance, since proper
        // metadata creation may depend on the already existing metadata of a parent
        for child_node in node.children {
            self.insert_instance_into_dom(id, child_node);
        }

        self.ids.insert(id);
        id
    }

    fn remove_instance_from_dom(&mut self, id: Ref) {
        self.ids.remove(&id);
        self.metas.remove(&id);
        if let Some(inst) = self.inner.get_by_ref(id) {
            if let Some(meta) = self.metas.get(&id) {
                if let Some(paths) = &meta.paths {
                    for path in paths {
                        self.path_map.remove(path);
                    }
                }
            }
            for child_id in inst.children().to_vec() {
                self.remove_instance_from_dom(child_id);
            }
            self.inner.destroy(id);
        }
    }

    fn match_ids_to_nodes(
        &self,
        result_map: &mut HashMap<Ref, InstanceNode>,
        known_ids: &mut Vec<Ref>,
        new_nodes: &mut Vec<InstanceNode>,
        filter: MatchFilter,
    ) {
        let match_instance_with_level =
            |inst: &Instance, node: &InstanceNode, filter: MatchFilter| match filter {
                MatchFilter::VeryStrict => {
                    inst.name == node.name
                        && inst.class == node.class_name
                        && inst.children().len() == node.children.len()
                }
                MatchFilter::Strict => inst.name == node.name && inst.class == node.class_name,
                MatchFilter::Any => inst.name == node.name || inst.class == node.class_name,
            };
        // NOTE: We iterate in reverse order since we may remove items during iteration
        let cloned = known_ids.to_vec();
        for (id_idx, id) in cloned.iter().enumerate().rev() {
            let inst = self
                .inner
                .get_by_ref(*id)
                .expect("unexpectedly missing instance");
            let found = new_nodes.iter().enumerate().find_map(|(idx, node)| {
                if match_instance_with_level(inst, node, filter) {
                    Some(idx)
                } else {
                    None
                }
            });
            // Insert the match into results if we got one, and remove from ids + nodes
            if let Some(node_idx) = found {
                let node = new_nodes.remove(node_idx);
                result_map.insert(*id, node);
                known_ids.remove(id_idx);
            }
        }
    }

    fn apply_metadata(&mut self, id: Ref, file_paths: &[PathBuf]) -> bool {
        if id != self.inner.root_ref() {
            let new_meta = InstanceMetadata::new(id, self, file_paths);
            if self.get_metadata(id) != new_meta.as_ref() {
                match new_meta {
                    Some(meta) => self.metas.insert(id, meta),
                    None => self.metas.remove(&id),
                };
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn apply_changes(&mut self, id: Ref, node: &InstanceNode) -> Option<DomNotification> {
        let inst = self.inner.get_by_ref(id).unwrap();

        let changed_class_name = if inst.class != node.class_name {
            Some(node.class_name.as_str())
        } else {
            None
        };

        let changed_name = if inst.name != node.name {
            Some(node.name.as_str())
        } else {
            None
        };

        let changed_meta = self.apply_metadata(id, &node.file_paths);

        if changed_class_name.is_some() || changed_name.is_some() || changed_meta {
            let inst_mut = self.inner.get_by_ref_mut(id).unwrap();

            if let Some(new_class_name) = changed_class_name {
                new_class_name.clone_into(&mut inst_mut.class);
            }
            if let Some(new_name) = changed_name {
                new_name.clone_into(&mut inst_mut.name);
            }

            Some(DomNotification::Changed {
                id,
                class_name: changed_class_name.map(ToOwned::to_owned),
                name: changed_name.map(ToOwned::to_owned),
            })
        } else {
            None
        }
    }

    fn apply_children(
        &mut self,
        parent_id: Option<Ref>,
        mut ids: Vec<Ref>,
        mut nodes: Vec<InstanceNode>,
    ) -> Vec<DomNotification> {
        let mut notifications = Vec::new();

        if ids.is_empty() && nodes.is_empty() {
            // Case #1 - no old ids or new nodes, this should never happen,
            // but just in case it does happen, we can safely ignore it
        } else if ids.is_empty() {
            // Case #2 - all children were added
            if let Some(pid) = parent_id {
                // Non-root children were added
                for child_node in nodes {
                    let child_id = self.insert_instance_into_dom(pid, child_node);
                    notifications.push(DomNotification::Added {
                        parent_id: Some(pid),
                        child_id,
                    })
                }
            } else {
                // Root was added
                let node = nodes.pop().unwrap();

                let root_id = self.inner.root_ref();
                let root = self.inner.root_mut();

                // Overwrite root instance properties with the ones from root node
                node.name.clone_into(&mut root.name);
                node.class_name.clone_into(&mut root.class);

                // Use our insert_instance method to create the entire instance tree
                let temp_id = self.insert_instance_into_dom(Ref::none(), node);
                let temp_child_ids = self.inner.get_by_ref(temp_id).unwrap().children().to_vec();

                // Transfer over children from our temp instance to the real root
                for child_id in temp_child_ids {
                    self.inner.transfer_within(child_id, root_id)
                }

                notifications.push(DomNotification::Added {
                    parent_id: None,
                    child_id: root_id,
                })
            }
        } else if nodes.is_empty() {
            // Case #3 - all children were removed
            if let Some(pid) = parent_id {
                // Non-root children were removed
                for child_id in ids {
                    self.remove_instance_from_dom(child_id);
                    notifications.push(DomNotification::Removed {
                        parent_id: Some(pid),
                        child_id,
                    })
                }
            } else {
                // Root was removed
                let root_id = self.inner.root_ref();
                let root = self.inner.root_mut();

                // We cant remove the root of a weak dom completely, so we have to replace
                // name & class with dummy properties and manually clear out its children
                root.name = String::from(DOM_ROOT_NAME_NONE);
                root.class = String::from(DOM_ROOT_NAME_NONE);
                for child_id in root.children().to_vec() {
                    self.inner.destroy(child_id);
                }
                self.metas.clear();

                notifications.push(DomNotification::Removed {
                    parent_id: None,
                    child_id: root_id,
                })
            }
        } else {
            // Case #4 - some children may have been added, changed, or removed

            // Match new nodes to existing instances in the dom as best we can,
            // multiple stages where we prefer more strict (exact) matches first
            let mut map = HashMap::default();
            self.match_ids_to_nodes(&mut map, &mut ids, &mut nodes, MatchFilter::VeryStrict);
            self.match_ids_to_nodes(&mut map, &mut ids, &mut nodes, MatchFilter::Strict);
            self.match_ids_to_nodes(&mut map, &mut ids, &mut nodes, MatchFilter::Any);

            // Any old instance that was not matched must have been removed, and in that
            // case we should also have a parent, since any handling of root nodes
            // being added / removed should have been handled in cases #1 -> #3
            if !ids.is_empty() {
                let pid = parent_id.expect("what the heck");
                for child_id in ids {
                    self.remove_instance_from_dom(child_id);
                    notifications.push(DomNotification::Removed {
                        parent_id: Some(pid),
                        child_id,
                    })
                }
            }

            // Any new node that was not matched was probably added - same parent semantics as above
            if !nodes.is_empty() {
                let pid = parent_id.expect("what the heck");
                for child_node in nodes {
                    let child_id = self.insert_instance_into_dom(pid, child_node);
                    notifications.push(DomNotification::Added {
                        parent_id: Some(pid),
                        child_id,
                    })
                }
            }

            // Everything else needs to be checked for changes
            for (id, node) in map {
                notifications.extend(self.apply_changes(id, &node));
                let inst = self.inner.get_by_ref(id).expect("missing child");
                notifications.extend(self.apply_children(
                    Some(id),
                    inst.children().to_vec(),
                    node.children,
                ));
            }
        }

        notifications
    }

    #[inline]
    pub fn get_instance(&self, id: Ref) -> Option<&Instance> {
        self.inner.get_by_ref(id)
    }

    #[inline]
    pub fn get_metadata(&self, id: Ref) -> Option<&InstanceMetadata> {
        if id == self.inner.root_ref() {
            Some(&self.root_meta)
        } else {
            self.metas.get(&id)
        }
    }

    #[inline]
    pub fn get_root_id(&self) -> Option<Ref> {
        if self.inner.root().name != DOM_ROOT_NAME_NONE {
            Some(self.inner.root_ref())
        } else {
            None
        }
    }

    pub fn find_by_path(&self, path: impl AsRef<Path>) -> Option<Ref> {
        self.path_map.get(&make_absolute_and_clean(path)).cloned()
    }

    pub fn find_by_query(&self, params: DomQueryParams) -> Vec<Ref> {
        // FUTURE: Use some kind of precompiled search engine ?? but
        // this seems to be good enough for now and is not too slow
        let mut results = self
            .ids
            .par_iter()
            .filter_map(|id| {
                self.inner
                    .get_by_ref(*id)
                    .map(|inst| (id, inst, self.metas.get(id)))
            })
            .filter_map(|(id, inst, meta)| {
                params
                    .score(inst, meta)
                    .map(|s| DomQueryResult::new(s, *id))
            })
            .collect::<Vec<_>>();

        results.sort_unstable();
        results.reverse();

        results
            .into_iter()
            .take(params.limit())
            .map(|result| result.id)
            .collect::<Vec<_>>()
    }

    pub fn apply_new_root(&mut self, node: Option<InstanceNode>) {
        let root_ids = if self.inner.root().name != DOM_ROOT_NAME_NONE {
            vec![self.inner.root_ref()]
        } else {
            Vec::new()
        };

        let root_nodes = if let Some(node) = node {
            vec![node]
        } else {
            Vec::new()
        };

        let notifications = self.apply_children(None, root_ids, root_nodes);
        for notification in notifications {
            self.notify(notification);
        }
    }

    pub async fn insert_instance(
        &mut self,
        parent: Ref,
        class_name: String,
        name: String,
    ) -> Option<Ref> {
        let parent_paths = match (
            self.get_instance(parent),
            self.get_metadata(parent)
                .and_then(|meta| meta.paths.as_ref()),
        ) {
            (i, Some(paths)) if i.is_some() => paths,
            _ => return None,
        };

        let (new_child_paths, changed_parent_paths) =
            match fs::create_instance(parent_paths, &class_name, &name).await {
                Ok((child_paths, parent_paths)) if !child_paths.is_empty() => {
                    (child_paths, parent_paths)
                }
                Ok(_) => return None,
                Err(e) => {
                    tracing::error!("{}", e);
                    return None;
                }
            };

        let child_id = self.insert_instance_into_dom(
            parent,
            InstanceNode {
                class_name,
                name,
                file_paths: new_child_paths,
                children: vec![],
            },
        );

        if let Some(changed_paths) = changed_parent_paths {
            let changed_metadata = self.apply_metadata(parent, &changed_paths);
            if changed_metadata {
                self.notify(DomNotification::Changed {
                    id: parent,
                    class_name: None,
                    name: None,
                });
            }
        }

        self.notify(DomNotification::Added {
            parent_id: Some(parent),
            child_id,
        });

        Some(child_id)
    }

    pub async fn rename_instance(&mut self, id: Ref, name: String) -> bool {
        let instance_paths = match (
            self.get_instance(id),
            self.get_metadata(id).and_then(|meta| meta.paths.as_ref()),
        ) {
            (i, Some(paths)) if i.is_some() => paths.clone(),
            _ => return false,
        };

        let instance = self.inner.get_by_ref_mut(id).unwrap();
        let instance_name = instance.name.as_str();

        let changed_paths = match fs::rename_instance(&instance_paths, instance_name, &name).await {
            Ok(paths) => paths,
            Err(e) => {
                tracing::error!("{}", e);
                return false;
            }
        };

        instance.name.clone_from(&name);

        let changed_metadata = self.apply_metadata(id, &changed_paths);
        if changed_metadata {
            self.notify(DomNotification::Changed {
                id,
                class_name: None,
                name: Some(name),
            });
        }

        true
    }

    pub async fn delete_instance(&mut self, id: Ref) -> bool {
        let parent = match self.get_instance(id).map(|inst| inst.parent()) {
            Some(parent) => parent,
            None => return false,
        };

        let instance_paths = match (
            self.get_instance(id),
            self.get_metadata(id).and_then(|meta| meta.paths.as_ref()),
        ) {
            (i, Some(paths)) if i.is_some() => paths,
            _ => return false,
        };

        match fs::delete_instance(instance_paths).await {
            Ok(_) => {}
            Err(e) => {
                tracing::error!("{}", e);
                return false;
            }
        }

        self.remove_instance_from_dom(id);

        self.notify(DomNotification::Removed {
            parent_id: Some(parent),
            child_id: id,
        });

        false
    }

    pub async fn move_instance(&mut self, _id: Ref, _new_parent_id: Ref) -> bool {
        // TODO: Implement this
        false
    }
}
