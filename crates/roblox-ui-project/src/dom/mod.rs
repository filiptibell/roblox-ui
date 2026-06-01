/*!
    The authoritative instance store, built on [`rbx_dom_weak::WeakDom`], plus the
    bookkeeping (stable identity, reverse file index, derived metadata) and the
    self-contained delta broadcast around it. Reconciliation lives in [`reconcile`].
*/

use std::path::{Path, PathBuf};

use ahash::AHashMap as HashMap;
use async_channel::{unbounded, Receiver, Sender};
use rbx_dom_weak::{
    types::{Ref, Variant},
    Instance, InstanceBuilder, UstrMap, WeakDom,
};

use roblox_ui_util::path::make_absolute_and_clean;

use crate::model::{Delta, SourceKey};

mod fs;
mod meta;
mod reconcile;
mod search;
mod util;

pub use fs::{
    create_instance as fs_create_instance, delete_instance as fs_delete_instance,
    rename_instance as fs_rename_instance,
};
pub use meta::*;

// NOTE: If anyone ever names their root instance this, things may break... let's hope they don't
const DOM_ROOT_NAME_NONE: &str = "<|<|<|ROOT|>|>|>";

/**
    The authoritative instance store.

    The tree itself lives in an [`rbx_dom_weak::WeakDom`] — the foundational
    crate already models exactly what we need (`Instance { class, name,
    properties: UstrMap<Variant>, children }` keyed by a stable [`Ref`]); we just
    *populate* `properties`. Around it the `Dom` keeps the bookkeeping needed for
    fast, stable, incremental sync:

    - `by_key`/`keys` — the `SourceKey` ↔ [`Ref`] mapping that gives every
      instance a stable identity across re-syncs.
    - `file_index` — file path → owning [`Ref`], for `find_by_path`, the
      `search` engine, and the sync engine's reverse lookups.
    - `metas` — derived [`InstanceMetadata`] (paths, package, actions).

    Reconciliation lives in the `reconcile` submodule; every transaction emits a
    self-contained `Vec<Delta>` to all subscribers.
*/
#[derive(Debug)]
pub struct Dom {
    inner: WeakDom,
    by_key: HashMap<SourceKey, Ref>,
    keys: HashMap<Ref, SourceKey>,
    file_index: HashMap<PathBuf, Ref>,
    metas: HashMap<Ref, InstanceMetadata>,
    root_meta: InstanceMetadata,
    subscribers: Vec<Sender<Vec<Delta>>>,
}

impl Default for Dom {
    fn default() -> Self {
        Self::new()
    }
}

impl Dom {
    pub fn new() -> Self {
        let inner = WeakDom::new(InstanceBuilder::new(DOM_ROOT_NAME_NONE));
        let root_ref = inner.root_ref();
        let mut by_key = HashMap::new();
        let mut keys = HashMap::new();
        by_key.insert(SourceKey::Root, root_ref);
        keys.insert(root_ref, SourceKey::Root);
        Self {
            inner,
            by_key,
            keys,
            file_index: HashMap::new(),
            metas: HashMap::new(),
            root_meta: InstanceMetadata::default(),
            subscribers: Vec::new(),
        }
    }

    /**
        Subscribe to the self-contained delta stream. Each subscriber gets its
        own queue and receives every subsequent transaction's deltas.
    */
    pub fn subscribe(&mut self) -> Receiver<Vec<Delta>> {
        let (tx, rx) = unbounded();
        self.subscribers.push(tx);
        rx
    }

    /**
        Broadcast a transaction's deltas to all live subscribers, dropping any
        whose receiver has been closed.
    */
    fn emit(&mut self, deltas: Vec<Delta>) {
        if deltas.is_empty() {
            return;
        }
        self.subscribers
            .retain(|tx| tx.try_send(deltas.clone()).is_ok());
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

    /**
        Typed property overrides for an instance (default-diffed).
    */
    #[inline]
    pub fn get_properties(&self, id: Ref) -> Option<&UstrMap<Variant>> {
        self.inner.get_by_ref(id).map(|inst| &inst.properties)
    }

    /**
        Total number of instances currently in the store (including the root).
    */
    #[inline]
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.get_root_id().is_none()
    }

    #[inline]
    pub fn children(&self, id: Ref) -> &[Ref] {
        self.inner
            .get_by_ref(id)
            .map(|inst| inst.children())
            .unwrap_or(&[])
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
        self.owner_of_file(path)
    }

    /**
        The owning instance ref for a file (reverse index). Drives both
        `find_by_path` and the sync engine's incremental ancestor search.
    */
    pub fn owner_of_file(&self, path: impl AsRef<Path>) -> Option<Ref> {
        self.file_index.get(&make_absolute_and_clean(path)).copied()
    }

    pub fn source_key(&self, id: Ref) -> Option<&SourceKey> {
        self.keys.get(&id)
    }

    pub fn ref_for_key(&self, key: &SourceKey) -> Option<Ref> {
        self.by_key.get(key).copied()
    }

    /**
        Run a Roblox Studio explorer search over the tree (name / `is:` / `tag:`
        / property comparisons / ancestry paths / boolean `and`-`or`), returning
        matches in tree order, capped at `limit`.
    */
    pub fn search(&self, query: &str, limit: Option<usize>) -> Vec<Ref> {
        search::search(self, query, limit)
    }
}
