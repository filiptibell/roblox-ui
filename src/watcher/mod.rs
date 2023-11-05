use std::path::Path;

use anyhow::Result;
use tracing::{debug, error};

mod async_cache;
mod async_watcher;
mod instances;
mod settings;

use async_cache::*;
use async_watcher::*;
use instances::*;

pub use settings::*;

#[derive(Debug, Clone)]
pub struct WatcherArguments {
    pub settings: Settings,
}

pub struct Watcher {
    args: WatcherArguments,
    smap: InstanceWatcher,
}

impl Watcher {
    pub fn new(args: WatcherArguments) -> Self {
        let smap = InstanceWatcher::new(args.settings.clone());
        Self { args, smap }
    }

    async fn handle_event(&mut self, event: AsyncFileEvent, path: &Path, contents: Option<&str>) {
        let res = if self.args.settings.is_sourcemap_path(path) {
            self.smap.update_file(contents).await
        } else if self.args.settings.is_project_path(path) {
            self.smap.update_rojo(contents).await
        } else {
            Ok(())
        };
        match res {
            Err(e) => error!("{:?} -> {} -> {e:?}", event, path.display()),
            Ok(_) => debug!("{:?} -> {}", event, path.display()),
        }
    }

    pub async fn watch(mut self) -> Result<()> {
        let paths = self.args.settings.relevant_paths();

        // Update all paths once initially
        let mut cache = AsyncFileCache::new();
        for path in &paths {
            if let Some(event) = cache.update(path).await? {
                self.handle_event(event, path, cache.get(path)).await;
            }
        }

        // Watch for further changes to the paths
        let mut watcher = AsyncFileWatcher::new(paths)?;
        while let Some(path) = watcher.recv().await {
            if let Some(event) = cache.update(&path).await? {
                self.handle_event(event, &path, cache.get(&path)).await;
            }
        }

        Ok(())
    }
}
