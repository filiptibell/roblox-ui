/*!
    The in-process [`Project`] handle: it owns the [`Dom`] store and the sync
    engine, watches the filesystem transparently in the background, and supports
    live-swapping the root project file. The stdio server is just one optional
    consumer built on top of it.

    [`Dom`]: crate::Dom
*/

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex as StdMutex};

use anyhow::Result;
use async_channel::Receiver;
use async_fs::{metadata, read_to_string};
use async_global_executor::Task;
use async_lock::{RwLock, RwLockReadGuard};
use rbx_dom_weak::{types::Ref, Ustr};
use tracing::error;

use roblox_ui_util::path::make_absolute_and_clean;

use crate::config::Config;
use crate::dom::Dom;
use crate::model::{Delta, SourceKey};
use crate::sync::{build_node, build_project, EngineOptions, ProjectFile};
use crate::watch::AsyncFileWatcher;

/**
    The in-process handle to a live, property-complete instance backend.

    A `Project` owns the [`Dom`] and the sync engine, and — crucially —
    **transparently watches the filesystem in the background**. After
    [`Project::open`], any change to a source file (made by *any* process: an
    editor, a build tool, `git`) is picked up automatically and reflected as
    self-contained [`Delta`]s to every [`subscribe`](Project::subscribe)r, with
    no further calls from the consumer. The stdio server is just one optional
    consumer built on top of this.

    The root project file can be **live-swapped** at runtime via
    [`set_project_file`](Project::set_project_file) — e.g. from a lightweight
    `default.project.json` to a heavier `build.project.json`. Because the
    reconciler keys instances by provenance, the shared (typically identical)
    portion of the two files reconciles in place with **stable `Ref`s**, and only
    the additive parts produce `Added`/`Removed` deltas.

    The background watch task is owned by the handle and cancelled when the
    handle is dropped (or restarted on a swap). Use [`Project::open_unwatched`]
    for deterministic tests that drive [`sync_path`](Project::sync_path) manually.
*/
pub struct Project {
    inner: Arc<ProjectInner>,
    // Holds the background watch loop. Cancelled (and respawned on a swap) by
    // replacing the `Task`. Holds no reference back to `Project`, so there is no
    // ownership cycle. `None` when opened unwatched.
    watch: StdMutex<Option<Task<()>>>,
}

struct ProjectInner {
    // The root project file currently in effect. Swappable at runtime.
    project_file: RwLock<PathBuf>,
    config_globs: Vec<glob::Pattern>,
    opts: RwLock<EngineOptions>,
    dom: Arc<RwLock<Dom>>,
}

/**
    A filesystem-backed mutation request.
*/
#[derive(Debug, Clone)]
pub enum Command {
    Insert {
        parent: Ref,
        class: Ustr,
        name: String,
    },
    Rename {
        id: Ref,
        name: String,
    },
    Delete {
        id: Ref,
    },
}

impl Project {
    /**
        Open a project, perform the initial sync, and start watching in the
        background. External file changes flow to subscribers automatically.
    */
    pub async fn open(config: Config) -> Result<Self> {
        let inner = Arc::new(ProjectInner::new(config));
        inner.resync().await;
        let watch = StdMutex::new(Some(spawn_watch(&inner)));
        Ok(Self { inner, watch })
    }

    /**
        Open a project and perform the initial sync, but do **not** start the
        background watcher. For deterministic tests / embedders that drive
        re-syncs themselves.
    */
    pub async fn open_unwatched(config: Config) -> Result<Self> {
        let inner = Arc::new(ProjectInner::new(config));
        inner.resync().await;
        Ok(Self {
            inner,
            watch: StdMutex::new(None),
        })
    }

    /**
        The root project file currently in effect.
    */
    pub async fn project_file(&self) -> PathBuf {
        self.inner.project_file.read().await.clone()
    }

    /**
        Live-swap the root project file (e.g. `default.project.json` →
        `build.project.json`) and reconcile against it. A relative `path` is
        resolved against the current project file's directory.

        The reconcile is **incremental and key-stable**: instances common to both
        files keep their `Ref`s, and only the differing (typically additive) parts
        emit `Added`/`Removed` deltas — which are returned and also broadcast to
        subscribers. If the project was opened watched, the background watcher is
        restarted so any new `$path` roots are observed.
    */
    pub async fn set_project_file(&self, path: impl Into<PathBuf>) -> Vec<Delta> {
        self.inner.set_project_file(path.into()).await;
        let deltas = self.inner.resync().await;

        // Restart the watcher (only if one was running) — the new file may pull
        // in `$path` roots that the previous watch set did not cover.
        let watching = self.watch.lock().unwrap().is_some();
        if watching {
            let task = spawn_watch(&self.inner);
            *self.watch.lock().unwrap() = Some(task);
        }

        deltas
    }

    /**
        Shared handle to the underlying store (for consumers that hold it).
    */
    pub fn dom(&self) -> Arc<RwLock<Dom>> {
        Arc::clone(&self.inner.dom)
    }

    /**
        Acquire a read guard over the store.
    */
    pub async fn read(&self) -> RwLockReadGuard<'_, Dom> {
        self.inner.dom.read().await
    }

    /**
        Subscribe to the self-contained delta stream.
    */
    pub async fn subscribe(&self) -> Receiver<Vec<Delta>> {
        self.inner.dom.write().await.subscribe()
    }

    /**
        Rebuild the whole project and reconcile it. Returns the emitted deltas.
    */
    pub async fn resync(&self) -> Vec<Delta> {
        self.inner.resync().await
    }

    /**
        Reconcile the change at a single path (the incremental entrypoint the
        watcher drives). Exposed so embedders/tests can trigger a re-sync
        deterministically without waiting on filesystem-event timing.
    */
    pub async fn sync_path(&self, path: &Path) {
        self.inner.handle_change(path).await;
    }

    /**
        Apply a filesystem mutation, then reconcile. Returns the emitted deltas.
    */
    pub async fn apply(&self, command: Command) -> Result<Vec<Delta>> {
        self.inner.apply(command).await
    }
}

/**
    Spawn a detached background watch loop bound to `inner`. The returned `Task`
    cancels the loop when dropped (on handle drop or on a project-file swap).
*/
fn spawn_watch(inner: &Arc<ProjectInner>) -> Task<()> {
    let inner = Arc::clone(inner);
    async_global_executor::spawn(async move {
        inner.watch_loop().await;
    })
}

impl ProjectInner {
    fn new(config: Config) -> Self {
        let config_globs = config.ignore_patterns();
        let opts = EngineOptions {
            ignore_globs: config_globs.clone(),
        };
        Self {
            project_file: RwLock::new(make_absolute_and_clean(&config.project_file)),
            config_globs,
            opts: RwLock::new(opts),
            dom: Arc::new(RwLock::new(Dom::new())),
        }
    }

    /**
        Swap the root project file, resolving a relative `path` against the
        current project file's directory.
    */
    async fn set_project_file(&self, path: PathBuf) {
        let resolved = if path.is_absolute() {
            make_absolute_and_clean(path)
        } else {
            let base = self
                .project_file
                .read()
                .await
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."));
            make_absolute_and_clean(base.join(path))
        };
        *self.project_file.write().await = resolved;
    }

    async fn opts(&self) -> EngineOptions {
        self.opts.read().await.clone()
    }

    /**
        Parse the project file, returning it if present + valid.
    */
    async fn parse_project(&self) -> Option<ProjectFile> {
        let path = self.project_file.read().await.clone();
        let text = read_to_string(&path).await.ok()?;
        match ProjectFile::parse(&path, &text) {
            Ok(project) => Some(project),
            Err(e) => {
                error!("failed to parse project file: {e}");
                None
            }
        }
    }

    /**
        The set of directories the watcher must observe recursively: the project
        file's own directory plus every existing `$path` root (which may live
        outside it). Nested roots are dropped.
    */
    async fn watch_roots(&self) -> Vec<PathBuf> {
        let project_file = self.project_file.read().await.clone();
        let mut roots = vec![project_file
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))];

        if let Some(project) = self.parse_project().await {
            for root in project.path_roots() {
                if metadata(&root).await.map(|m| m.is_dir()).unwrap_or(false) {
                    roots.push(root);
                }
            }
        }

        // Dedupe and drop any root nested under another.
        roots.sort();
        roots.dedup();
        let mut result: Vec<PathBuf> = Vec::new();
        for root in roots {
            if result.iter().any(|kept| root.starts_with(kept)) {
                continue;
            }
            result.push(root);
        }
        result
    }

    async fn watch_loop(&self) {
        let roots = self.watch_roots().await;
        let mut watcher = match AsyncFileWatcher::new(roots) {
            Ok(watcher) => watcher,
            Err(e) => {
                error!("failed to start file watcher: {e}");
                return;
            }
        };
        while let Some(path) = watcher.recv().await {
            self.handle_change(&path).await;
        }
    }

    /**
        Rebuild the whole project snapshot and reconcile it into the store.
    */
    async fn resync(&self) -> Vec<Delta> {
        let snapshot = match self.parse_project().await {
            Some(project) => {
                // Fold the project's globIgnorePaths into the active options.
                let mut globs = self.config_globs.clone();
                globs.extend(
                    project
                        .glob_ignore_paths
                        .iter()
                        .filter_map(|g| glob::Pattern::new(g).ok()),
                );
                let opts = EngineOptions {
                    ignore_globs: globs,
                };
                *self.opts.write().await = opts.clone();
                build_project(&project, &opts).await
            }
            None => None,
        };
        self.dom.write().await.apply_snapshot(snapshot)
    }

    /**
        Reconcile a single filesystem change, Rojo-style: find the nearest owning
        instance for the changed path and recompute just that subtree.
    */
    async fn handle_change(&self, path: &Path) {
        let path = make_absolute_and_clean(path);

        // A project-file change can restructure anything → full resync.
        let project_file = self.project_file.read().await.clone();
        if path == project_file
            || path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.ends_with(".project.json") || n.ends_with(".project.jsonc"))
                .unwrap_or(false)
        {
            self.resync().await;
            return;
        }

        // Recompute the owner of the changed path. For a delete, the path itself
        // is gone, so start the ancestor search at its parent directory.
        let exists = metadata(&path).await.is_ok();
        let target = if exists {
            path.as_path()
        } else {
            match path.parent() {
                Some(parent) => parent,
                None => return,
            }
        };

        // Ancestor search over the reverse index for the nearest owning ref.
        let owner = {
            let dom = self.dom.read().await;
            let mut current = Some(target);
            let mut found = None;
            while let Some(dir) = current {
                if let Some(owner) = dom.owner_of_file(dir) {
                    found = Some(owner);
                    break;
                }
                current = dir.parent();
            }
            found
        };

        let Some(owner) = owner else {
            // Nothing owns this path (e.g. a brand-new top-level area) → resync.
            self.resync().await;
            return;
        };

        if self.resync_instance(owner).await.is_none() {
            // Could not recompute incrementally (e.g. a project-node owner, or a
            // recompute that returned nothing) → fall back to a full resync.
            self.resync().await;
        }
    }

    /**
        Recompute and reconcile just the subtree owned by `owner`. Returns
        `Some(())` if it was handled incrementally, `None` if the caller should
        fall back to a full resync.
    */
    async fn resync_instance(&self, owner: Ref) -> Option<()> {
        let key = { self.dom.read().await.source_key(owner).cloned() };
        match key {
            Some(SourceKey::Path(path)) => {
                let opts = self.opts().await;
                match build_node(&opts, &path).await {
                    Some(snapshot) => {
                        self.dom.write().await.apply_node(snapshot);
                        Some(())
                    }
                    // The path vanished (e.g. directory removed). Recomputing the
                    // parent owner handles the removal; signal a fallback.
                    None => None,
                }
            }
            // Project-node / root owners are only reached when a file directly
            // under a service's `$path` root is added/removed — rare. A full
            // resync keeps the project structure correct.
            _ => None,
        }
    }

    async fn apply(&self, command: Command) -> Result<Vec<Delta>> {
        match command {
            Command::Insert {
                parent,
                class,
                name,
            } => {
                let parent_paths = {
                    let dom = self.dom.read().await;
                    dom.get_metadata(parent).and_then(|m| m.paths.clone())
                };
                let Some(parent_paths) = parent_paths else {
                    return Ok(Vec::new());
                };
                crate::dom::fs_create_instance(&parent_paths, &class, &name).await?;
            }
            Command::Rename { id, name } => {
                let (paths, current) = {
                    let dom = self.dom.read().await;
                    let current = dom.get_instance(id).map(|i| i.name.clone());
                    let paths = dom.get_metadata(id).and_then(|m| m.paths.clone());
                    (paths, current)
                };
                let (Some(paths), Some(current)) = (paths, current) else {
                    return Ok(Vec::new());
                };
                crate::dom::fs_rename_instance(&paths, &current, &name).await?;
            }
            Command::Delete { id } => {
                let paths = {
                    let dom = self.dom.read().await;
                    dom.get_metadata(id).and_then(|m| m.paths.clone())
                };
                let Some(paths) = paths else {
                    return Ok(Vec::new());
                };
                crate::dom::fs_delete_instance(&paths).await?;
            }
        }
        Ok(self.resync().await)
    }
}
