/*!
    The provenance-based [`SourceKey`] that gives every instance a stable
    identity across re-syncs, replacing heuristic name/class matching.
*/

use std::path::PathBuf;

use ustr::Ustr;

/**
    A deterministic, provenance-based identity for an instance in the DOM.

    This replaces the old `(name, class, child-count)` heuristic matching with a
    stable key derived purely from *where an instance comes from*. Re-syncing a
    project reuses the existing [`rbx_dom_weak::types::Ref`] for any instance
    whose `SourceKey` matches, so edits become in-place updates (`Renamed`,
    `PropertiesChanged`, …) on a **stable `Ref`** rather than remove + re-add.

    The variants are mutually exclusive and cover every way an instance can be
    produced by the sync engine:

    - [`SourceKey::Root`] — the single project-tree root instance.
    - [`SourceKey::Project`] — a node declared inline in the `*.project.json`
      tree, identified by its name-path from the root (e.g.
      `ReplicatedStorage/Shared`). Stable even if its `$path` changes.
    - [`SourceKey::Path`] — an instance sourced from a filesystem path (a script
      file, a model file, or a directory). Keyed by the canonical absolute path.
    - [`SourceKey::Nested`] — an instance living *inside* a parsed model file
      (`.rbxmx`/`.rbxm`), keyed by the owning file plus its child-index path
      within that file's own tree.
*/
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SourceKey {
    Root,
    Project(Box<[Ustr]>),
    Path(PathBuf),
    Nested(PathBuf, Box<[u32]>),
}

impl SourceKey {
    /**
        Key for a child node declared inline under a project node.
    */
    pub fn project_child(parent: &SourceKey, name: Ustr) -> SourceKey {
        match parent {
            SourceKey::Root => SourceKey::Project(Box::new([name])),
            SourceKey::Project(path) => {
                let mut path = path.to_vec();
                path.push(name);
                SourceKey::Project(path.into_boxed_slice())
            }
            // Children of a path/nested node are themselves path/nested nodes,
            // never project nodes; this branch is unreachable in practice but we
            // fall back to a project key rooted at the name to stay total.
            _ => SourceKey::Project(Box::new([name])),
        }
    }

    /**
        Key for an instance nested inside a model file at the given index path.
    */
    pub fn nested(file: PathBuf, index_path: &[u32]) -> SourceKey {
        SourceKey::Nested(file, index_path.to_vec().into_boxed_slice())
    }
}
