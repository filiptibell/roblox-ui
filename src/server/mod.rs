use std::path::Path;

use anyhow::Result;
use tracing::{debug, error};

mod config;
mod notify;
mod provider;

use notify::*;
use provider::*;

pub use config::*;

pub struct Server {
    config: Config,
    instances: InstanceProvider,
}

impl Server {
    pub fn new(config: Config) -> Self {
        let instances = InstanceProvider::new(config.clone());
        Self { config, instances }
    }

    async fn handle_event(
        &mut self,
        event: AsyncFileEvent,
        file_path: &Path,
        file_contents: Option<&str>,
    ) {
        let res = if self.config.is_sourcemap_path(file_path) {
            self.instances.update_file(file_contents).await
        } else if self.config.is_rojo_project_path(file_path) {
            self.instances.update_rojo(file_contents).await
        } else {
            Ok(())
        };
        match res {
            Err(e) => error!("{:?} -> {} -> {e:?}", event, file_path.display()),
            Ok(_) => debug!("{:?} -> {}", event, file_path.display()),
        }
    }

    pub async fn serve(mut self) -> Result<()> {
        let paths = self.config.paths_to_watch();
        let paths = paths.iter().map(|p| p.to_path_buf()).collect::<Vec<_>>();

        // Emit an initial 'null' (meaning no instance data) to
        // let the consumer know instance watching has started
        println!("null");

        // Update all paths once initially
        let mut cache = AsyncFileCache::new();
        for path in &paths {
            if let Some(event) = cache.read_file_at(path).await? {
                self.handle_event(event, path, cache.get_file(path)).await;
            }
        }

        // Watch for further changes to the paths
        let mut watcher = AsyncFileWatcher::new(paths)?;
        while let Some(path) = watcher.recv().await {
            if let Some(event) = cache.read_file_at(&path).await? {
                self.handle_event(event, &path, cache.get_file(&path)).await;
            }
        }

        Ok(())
    }
}
