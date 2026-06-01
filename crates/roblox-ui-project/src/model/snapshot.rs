/*!
    The intermediate [`Snapshot`] tree the sync engine produces from the
    filesystem, ready to be reconciled into the live store.
*/

use std::path::PathBuf;

use rbx_dom_weak::{
    types::Variant,
    {Ustr, UstrMap},
};

use crate::model::SourceKey;

/**
    The intermediate representation produced by the sync engine: a fully-resolved
    instance plus its children, ready to be reconciled into the live [`WeakDom`].

    A `Snapshot` is *value-typed and self-describing*: it carries the typed
    property overrides ([`Variant`]s, default-diffed against the class defaults),
    the set of files that contributed to it (for the reverse index + metadata),
    and a stable [`SourceKey`] identity.

    [`WeakDom`]: rbx_dom_weak::WeakDom
*/
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub key: SourceKey,
    pub class: Ustr,
    pub name: String,
    /**
        Non-default property overrides as typed variants. `Name` is never stored
        here — it lives in [`Snapshot::name`].
    */
    pub properties: UstrMap<Variant>,
    /**
        Every file that contributed to this instance (primary source, meta
        overlay, owning model file, …). Drives the reverse index and metadata.
    */
    pub file_paths: Vec<PathBuf>,
    pub children: Vec<Snapshot>,
}

impl Snapshot {
    pub fn new(key: SourceKey, class: Ustr, name: impl Into<String>) -> Self {
        Self {
            key,
            class,
            name: name.into(),
            properties: UstrMap::default(),
            file_paths: Vec::new(),
            children: Vec::new(),
        }
    }

    /**
        Sort children by name, recursively, for stable explorer ordering.
    */
    pub fn sort(&mut self) {
        for child in &mut self.children {
            child.sort();
        }
        self.children.sort_by(|a, b| a.name.cmp(&b.name));
    }
}
