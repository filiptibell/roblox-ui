/*!
    `roblox-ui-project` — a fast, in-house, property-complete instance backend.

    This crate owns the entire file -> instance-DOM pipeline with zero reliance on
    the Rojo tool: no `rojo serve`, no `rojo sourcemap --watch`, no Rojo HTTP. It
    reads `*.project.json` + source files directly and produces a live,
    incrementally-updated `rbx_dom_weak::WeakDom` carrying typed `Variant`
    properties, watches the filesystem transparently in the background, and emits
    self-contained `Delta`s on every change.

    It is split into the `model` (shared value types), `sync` (the
    Rojo-compatible file -> snapshot engine), `dom` (the `WeakDom`-backed store,
    reconciler, and Studio-style search), and `watch` (the recursive filesystem
    watcher) modules, tied together by the in-process [`Project`] handle.
*/

mod config;
mod dom;
mod model;
mod project;
mod reflect;
mod sync;
mod watch;

pub use config::Config;
pub use dom::{
    Dom, InstanceMetadata, InstanceMetadataActions, InstanceMetadataPackage, InstanceMetadataPaths,
};
pub use model::Delta;
pub use project::{Command, Project};
