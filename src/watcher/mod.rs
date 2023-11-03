use std::path::Path;

use anyhow::Result;
use tracing::info;

mod async_cache;
mod async_watcher;
mod settings;

use async_cache::*;
use async_watcher::*;
pub use settings::*;

#[derive(Debug, Clone)]
pub struct WatcherArguments {
    pub settings: Settings,
}

pub struct Watcher {
    args: WatcherArguments,
}

impl Watcher {
    pub fn new(args: WatcherArguments) -> Self {
        Self { args }
    }

    fn handle_event(&mut self, event: AsyncFileEvent, path: &Path, _contents: Option<&str>) {
        info!("{:?} -> {}", event, path.display());
        // TODO: Check if the path is a sourcemap.json or rojo project file, then:
        // TODO: - Handle changes to sourcemap
        // TODO: - Handle changes to rojo project
    }

    pub async fn watch(mut self) -> Result<()> {
        let paths = self.args.settings.relevant_paths();

        // Update all paths once initially
        let mut cache = AsyncFileCache::new();
        for path in &paths {
            if let Some(event) = cache.update(path).await? {
                self.handle_event(event, path, cache.get(path));
            }
        }

        // Watch for further changes to the paths
        let mut watcher = AsyncFileWatcher::new(paths)?;
        while let Some(path) = watcher.recv().await {
            if let Some(event) = cache.update(&path).await? {
                self.handle_event(event, &path, cache.get(&path));
            }
        }

        Ok(())
    }
}
