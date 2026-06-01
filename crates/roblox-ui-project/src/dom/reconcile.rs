/*!
    The reconciler: diffs a freshly-built `Snapshot` against the live store and
    applies the minimal set of in-place mutations, reusing `Ref`s by `SourceKey`
    so identity is stable across re-syncs. Every public entry point emits a
    self-contained `Vec<Delta>`.
*/

use rbx_dom_weak::{
    types::{Ref, Variant},
    InstanceBuilder, Ustr, UstrMap,
};

use roblox_ui_util::path::make_absolute_and_clean;

use crate::model::{Delta, Snapshot};

use super::{Dom, InstanceMetadata, DOM_ROOT_NAME_NONE};

impl Dom {
    /**
        Reconcile the entire tree against a freshly-built project snapshot
        (`None` clears it). Returns the emitted, self-contained deltas.
    */
    pub fn apply_snapshot(&mut self, snapshot: Option<Snapshot>) -> Vec<Delta> {
        let mut deltas = Vec::new();
        match snapshot {
            Some(root) => self.reconcile_root(root, &mut deltas),
            None => self.reset_root(&mut deltas),
        }
        self.emit(deltas.clone());
        deltas
    }

    /**
        Reconcile a single already-present subtree (identified by its key)
        against a freshly-rebuilt snapshot. Used by the incremental fast path
        for content edits. Returns `None` if the node isn't present (caller
        should fall back to a full reconcile).
    */
    pub fn apply_node(&mut self, snapshot: Snapshot) -> Option<Vec<Delta>> {
        let id = self.by_key.get(&snapshot.key).copied()?;
        let mut deltas = Vec::new();
        self.reconcile_existing(id, &snapshot, &mut deltas);
        self.emit(deltas.clone());
        Some(deltas)
    }

    fn reset_root(&mut self, deltas: &mut Vec<Delta>) {
        let root_ref = self.inner.root_ref();
        if self.inner.root().name == DOM_ROOT_NAME_NONE {
            return;
        }
        for child in self.inner.root().children().to_vec() {
            self.remove_subtree(child, deltas);
        }
        let root = self.inner.root_mut();
        root.name = String::from(DOM_ROOT_NAME_NONE);
        root.class = Ustr::from(DOM_ROOT_NAME_NONE);
        root.properties.clear();
        deltas.push(Delta::Removed {
            id: root_ref,
            parent: Ref::none(),
        });
    }

    fn reconcile_root(&mut self, snapshot: Snapshot, deltas: &mut Vec<Delta>) {
        let root_ref = self.inner.root_ref();
        let was_present = self.get_root_id().is_some();

        // Update the root instance fields in place.
        let root = self.inner.root_mut();
        let class_changed = root.class != snapshot.class;
        let name_changed = root.name != snapshot.name;
        root.class = snapshot.class;
        root.name = snapshot.name.clone();
        self.set_properties(root_ref, &snapshot.properties, deltas, !was_present);
        self.register_meta(root_ref, &snapshot, true, deltas, false);

        if !was_present {
            deltas.push(Delta::Added {
                id: root_ref,
                parent: Ref::none(),
                index: 0,
                class: snapshot.class,
                name: snapshot.name.clone(),
            });
        } else {
            if class_changed {
                deltas.push(Delta::Reclassed {
                    id: root_ref,
                    class: snapshot.class,
                });
            }
            if name_changed {
                deltas.push(Delta::Renamed {
                    id: root_ref,
                    name: snapshot.name.clone(),
                });
            }
        }

        self.reconcile_children(root_ref, &snapshot.children, deltas);
    }

    fn reconcile_existing(&mut self, id: Ref, snapshot: &Snapshot, deltas: &mut Vec<Delta>) {
        let inst = match self.inner.get_by_ref(id) {
            Some(inst) => inst,
            None => return,
        };
        let class_changed = inst.class != snapshot.class;
        let name_changed = inst.name != snapshot.name;

        if class_changed || name_changed {
            let inst = self.inner.get_by_ref_mut(id).unwrap();
            if class_changed {
                inst.class = snapshot.class;
            }
            if name_changed {
                snapshot.name.clone_into(&mut inst.name);
            }
        }
        if class_changed {
            deltas.push(Delta::Reclassed {
                id,
                class: snapshot.class,
            });
        }
        if name_changed {
            deltas.push(Delta::Renamed {
                id,
                name: snapshot.name.clone(),
            });
        }

        self.set_properties(id, &snapshot.properties, deltas, false);
        self.register_meta(id, snapshot, false, deltas, true);
        self.reconcile_children(id, &snapshot.children, deltas);
    }

    fn reconcile_children(
        &mut self,
        parent: Ref,
        new_children: &[Snapshot],
        deltas: &mut Vec<Delta>,
    ) {
        // Remove existing children whose key is gone from the new set. A hash
        // set of the new keys keeps this linear even for very wide nodes.
        let new_keys: ahash::AHashSet<&_> = new_children.iter().map(|c| &c.key).collect();
        let existing = self.inner.get_by_ref(parent).unwrap().children().to_vec();
        for child in existing {
            let still_present = self
                .keys
                .get(&child)
                .map(|key| new_keys.contains(key))
                .unwrap_or(false);
            if !still_present {
                self.remove_subtree(child, deltas);
            }
        }

        // Reconcile / insert each new child.
        for (idx, child_snapshot) in new_children.iter().enumerate() {
            match self.by_key.get(&child_snapshot.key).copied() {
                Some(existing_ref)
                    if self
                        .inner
                        .get_by_ref(existing_ref)
                        .map(|i| i.parent() == parent)
                        .unwrap_or(false) =>
                {
                    self.reconcile_existing(existing_ref, child_snapshot, deltas);
                }
                _ => {
                    self.insert_subtree(parent, child_snapshot, idx, deltas);
                }
            }
        }
    }

    fn insert_subtree(
        &mut self,
        parent: Ref,
        snapshot: &Snapshot,
        index: usize,
        deltas: &mut Vec<Delta>,
    ) -> Ref {
        let mut builder = InstanceBuilder::new(snapshot.class).with_name(snapshot.name.as_str());
        for (key, value) in &snapshot.properties {
            builder = builder.with_property(*key, value.clone());
        }
        let id = self.inner.insert(parent, builder);

        self.by_key.insert(snapshot.key.clone(), id);
        self.keys.insert(id, snapshot.key.clone());
        self.register_meta(id, snapshot, false, deltas, false);

        deltas.push(Delta::Added {
            id,
            parent,
            index,
            class: snapshot.class,
            name: snapshot.name.clone(),
        });

        for (idx, child) in snapshot.children.iter().enumerate() {
            self.insert_subtree(id, child, idx, deltas);
        }

        id
    }

    fn remove_subtree(&mut self, id: Ref, deltas: &mut Vec<Delta>) {
        let parent = self
            .inner
            .get_by_ref(id)
            .map(|i| i.parent())
            .unwrap_or_else(Ref::none);
        self.unregister_recursive(id);
        self.inner.destroy(id);
        deltas.push(Delta::Removed { id, parent });
    }

    fn unregister_recursive(&mut self, id: Ref) {
        let children = self
            .inner
            .get_by_ref(id)
            .map(|i| i.children().to_vec())
            .unwrap_or_default();
        for child in children {
            self.unregister_recursive(child);
        }
        if let Some(key) = self.keys.remove(&id) {
            self.by_key.remove(&key);
        }
        self.metas.remove(&id);
        self.file_index.retain(|_, v| *v != id);
    }

    /**
        Diff and apply the property override set, emitting a single
        `PropertiesChanged` delta if anything moved (unless suppressed, e.g. when
        folded into a fresh `Added`).
    */
    fn set_properties(
        &mut self,
        id: Ref,
        new_props: &UstrMap<Variant>,
        deltas: &mut Vec<Delta>,
        suppress_delta: bool,
    ) {
        let inst = match self.inner.get_by_ref(id) {
            Some(inst) => inst,
            None => return,
        };
        let mut changed: Vec<(Ustr, Option<Variant>)> = Vec::new();
        for (key, value) in new_props {
            if inst.properties.get(key) != Some(value) {
                changed.push((*key, Some(value.clone())));
            }
        }
        for key in inst.properties.keys() {
            if !new_props.contains_key(key) {
                changed.push((*key, None));
            }
        }
        if changed.is_empty() {
            return;
        }

        let inst = self.inner.get_by_ref_mut(id).unwrap();
        for (key, value) in &changed {
            match value {
                Some(v) => {
                    inst.properties.insert(*key, v.clone());
                }
                None => {
                    inst.properties.remove(key);
                }
            }
        }

        if !suppress_delta {
            deltas.push(Delta::PropertiesChanged { id, changed });
        }
    }

    /**
        (Re)derive metadata + the reverse file index for an instance, emitting a
        `MetadataChanged` delta if it changed (when `allow_delta`, non-root only).
    */
    fn register_meta(
        &mut self,
        id: Ref,
        snapshot: &Snapshot,
        is_root: bool,
        deltas: &mut Vec<Delta>,
        allow_delta: bool,
    ) {
        // Update the reverse file index unconditionally.
        for path in &snapshot.file_paths {
            self.file_index.insert(make_absolute_and_clean(path), id);
        }

        let new_meta = InstanceMetadata::new(id, self, &snapshot.file_paths);
        if is_root {
            self.root_meta = new_meta.unwrap_or_default();
            return;
        }
        let changed = self.metas.get(&id) != new_meta.as_ref();
        match new_meta {
            Some(meta) => {
                self.metas.insert(id, meta);
            }
            None => {
                self.metas.remove(&id);
            }
        }
        if changed && allow_delta {
            deltas.push(Delta::MetadataChanged { id });
        }
    }
}
