#![allow(clippy::unnecessary_to_owned)]

use std::collections::HashMap;

use rbx_dom_weak::{types::Ref, Instance, InstanceBuilder, WeakDom};
use serde::{Deserialize, Serialize};

mod meta;
mod node;

pub use meta::*;
pub use node::*;

use super::Config;

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
    metas: HashMap<Ref, InstanceMetadata>,
    root_meta: InstanceMetadata,
}

impl Dom {
    pub fn new(config: Config) -> Self {
        Self {
            _config: config,
            inner: WeakDom::new(InstanceBuilder::new(DOM_ROOT_NAME_NONE)),
            metas: HashMap::new(),
            root_meta: InstanceMetadata::default(),
        }
    }

    fn insert_instance(&mut self, parent_id: Ref, node: InstanceNode) -> Ref {
        let inst = InstanceBuilder::new(node.class_name).with_name(node.name);
        let id = self.inner.insert(parent_id, inst);
        if let Some(meta) = InstanceMetadata::from_paths(&node.file_paths) {
            self.metas.insert(id, meta);
        }
        for child_node in node.children {
            self.insert_instance(id, child_node);
        }
        id
    }

    fn remove_instance(&mut self, id: Ref) {
        self.metas.remove(&id);
        if let Some(inst) = self.inner.get_by_ref(id) {
            for child_id in inst.children().to_vec() {
                self.remove_instance(child_id);
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
        fn match_instance_with_level(
            inst: &Instance,
            node: &InstanceNode,
            filter: MatchFilter,
        ) -> bool {
            match filter {
                MatchFilter::VeryStrict => {
                    inst.name == node.name
                        && inst.class == node.class_name
                        && inst.children().len() == node.children.len()
                }
                MatchFilter::Strict => inst.name == node.name && inst.class == node.class_name,
                MatchFilter::Any => inst.name == node.name || inst.class == node.class_name,
            }
        }
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

    fn apply_changes(&mut self, id: Ref, node: &InstanceNode) -> Option<DomNotification> {
        let inst = self
            .inner
            .get_by_ref_mut(id)
            .expect("tried to diff/apply changes for nonexistent node");

        let changed_class_name = if inst.class != node.class_name {
            Some(node.class_name.to_owned())
        } else {
            None
        };
        let changed_name = if inst.name != node.name {
            Some(node.name.to_owned())
        } else {
            None
        };

        if changed_class_name.is_some() || changed_name.is_some() {
            if let Some(new_class_name) = &changed_class_name {
                inst.class = new_class_name.to_owned();
            }
            if let Some(new_name) = &changed_name {
                inst.name = new_name.to_owned();
            }
            Some(DomNotification::Changed {
                id,
                class_name: changed_class_name,
                name: changed_name,
            })
        } else if id != self.inner.root_ref() {
            let new_meta = InstanceMetadata::from_paths(&node.file_paths);
            if self.get_metadata(id) != new_meta.as_ref() {
                match new_meta {
                    Some(meta) => self.metas.insert(id, meta),
                    None => self.metas.remove(&id),
                };
                Some(DomNotification::Changed {
                    id,
                    class_name: None,
                    name: None,
                })
            } else {
                None
            }
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
                    let child_id = self.insert_instance(pid, child_node);
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
                root.name = node.name.to_owned();
                root.class = node.class_name.to_owned();

                // Use our insert_instance method to create the entire instance tree
                let temp_id = self.insert_instance(Ref::none(), node);
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
                    self.remove_instance(child_id);
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
            let mut map = HashMap::new();
            self.match_ids_to_nodes(&mut map, &mut ids, &mut nodes, MatchFilter::VeryStrict);
            self.match_ids_to_nodes(&mut map, &mut ids, &mut nodes, MatchFilter::Strict);
            self.match_ids_to_nodes(&mut map, &mut ids, &mut nodes, MatchFilter::Any);

            // Any old instance that was not matched must have been removed, and in that
            // case we should also have a parent, since any handling of root nodes
            // being added / removed should have been handled in cases #1 -> #3
            if !ids.is_empty() {
                let pid = parent_id.expect("what the heck");
                for child_id in ids {
                    self.remove_instance(child_id);
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
                    let child_id = self.insert_instance(pid, child_node);
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

    pub fn get_instance(&self, id: Ref) -> Option<&Instance> {
        self.inner.get_by_ref(id)
    }

    pub fn get_metadata(&self, id: Ref) -> Option<&InstanceMetadata> {
        if id == self.inner.root_ref() {
            Some(&self.root_meta)
        } else {
            self.metas.get(&id)
        }
    }

    pub fn get_root_instance(&self) -> Option<&Instance> {
        if self.inner.root().name != DOM_ROOT_NAME_NONE {
            Some(self.inner.root())
        } else {
            None
        }
    }

    pub fn _get_root_metadata(&self) -> Option<&InstanceMetadata> {
        self.get_metadata(self.inner.root_ref())
    }

    pub fn apply_new_root(&mut self, node: Option<InstanceNode>) -> Vec<DomNotification> {
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
        self.apply_children(None, root_ids, root_nodes)
    }
}
