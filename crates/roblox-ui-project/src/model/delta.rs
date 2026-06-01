/*!
    The self-contained [`Delta`] change stream the store broadcasts to consumers
    after every reconcile transaction.
*/

use rbx_dom_weak::{
    types::{Ref, Variant},
    Ustr,
};

/**
    A single self-contained change to the instance DOM.

    Deltas are emitted in batches (`Vec<Delta>`) on a broadcast channel after
    every sync transaction. They are **self-contained**: each carries the values
    a consumer needs to update its own view incrementally, without locking or
    re-reading the writer's tree. In particular [`Delta::PropertiesChanged`]
    carries the new (or reset-to-default) values, and [`Delta::Added`] carries
    enough to create a placeholder node before any further reads.

    Identity (`id`) is a **stable `Ref`**: a property edit on an existing
    instance produces a `PropertiesChanged` on the same `Ref` it had before, not
    a `Removed` + `Added` pair.
*/
#[derive(Debug, Clone, PartialEq)]
pub enum Delta {
    Added {
        id: Ref,
        parent: Ref,
        index: usize,
        class: Ustr,
        name: String,
    },
    Removed {
        id: Ref,
        parent: Ref,
    },
    Renamed {
        id: Ref,
        name: String,
    },
    Reclassed {
        id: Ref,
        class: Ustr,
    },
    /**
        One or more properties changed. `None` means the property was reset to
        its class default (i.e. removed from the override set).
    */
    PropertiesChanged {
        id: Ref,
        changed: Vec<(Ustr, Option<Variant>)>,
    },
    /**
        Non-structural metadata (source paths, package info, available actions)
        changed for this instance.
    */
    MetadataChanged {
        id: Ref,
    },
}

impl Delta {
    /**
        The instance this delta is about.
    */
    pub fn id(&self) -> Ref {
        match self {
            Delta::Added { id, .. }
            | Delta::Removed { id, .. }
            | Delta::Renamed { id, .. }
            | Delta::Reclassed { id, .. }
            | Delta::PropertiesChanged { id, .. }
            | Delta::MetadataChanged { id } => *id,
        }
    }
}
