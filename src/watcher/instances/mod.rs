use anyhow::Result;
use tracing::error;

mod file_sourcemap;
mod provider;
mod rojo_sourcemap;
mod shared;

use file_sourcemap::*;
use rojo_sourcemap::*;

pub use provider::*;
pub use shared::*;

use super::*;

/**
    A fault-tolerant instance watcher.

    Prioritizes instance watching in the following order:

    1. Using `rojo sourcemap --watch`, if a project is available and valid
    2. Using a `sourcemap.json` file, if available and valid
    3. Using empty data, which should display as blank in an explorer
*/
#[derive(Debug, Default)]
pub struct InstanceWatcher {
    settings: Settings,
    last_sourcemap: Option<SourcemapNode>,
    last_project: Option<RojoProjectFile>,
    provider: Option<InstanceProvider>,
}

impl InstanceWatcher {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings,
            last_sourcemap: None,
            last_project: None,
            provider: None,
        }
    }

    async fn update_provider(&mut self, desired_kind: InstanceProviderKind) -> Result<()> {
        if !matches!(self.provider.as_ref().map(|p| p.kind()), Some(k) if k == desired_kind) {
            // Create, start, and store a new provider, stop the old one if one existed
            let mut this = InstanceProvider::from_kind(desired_kind, self.settings.clone());
            match this.start().await {
                Err(e) => {
                    error!("failed to start provider - {e}");
                }
                Ok(_) => {
                    if let Some(mut last) = self.provider.replace(this) {
                        if let Err(e) = last.stop().await {
                            error!("failed to stop provider - {e}");
                        }
                    }
                }
            }
        } else if let Some(this) = self.provider.as_mut() {
            // Desired provider kind did not change, update the current one
            this.update(self.last_sourcemap.as_ref()).await?;
        }
        Ok(())
    }

    pub async fn update_file(&mut self, contents: Option<&str>) -> Result<()> {
        let smap: Option<SourcemapNode> = contents
            .and_then(|c| if c.is_empty() { None } else { Some(c) })
            .and_then(|s| match serde_json::from_str(s) {
                Ok(v) => Some(v),
                Err(e) => {
                    error!("failed to deserialize sourcemap file - {e}");
                    None
                }
            });

        // Replace stored sourcemap, we don't actually care
        // if it has changed here, we do diffing later
        let is_some = smap.is_some();
        self.last_sourcemap = smap;

        // We should not update the current provider if it is rojo,
        // we let that take precendence since it is more efficient
        let provider_kind = self.provider.as_ref().map(|p| p.kind());
        if !matches!(provider_kind, Some(InstanceProviderKind::RojoSourcemap)) {
            if is_some {
                self.update_provider(InstanceProviderKind::FileSourcemap)
                    .await?;
            } else {
                self.update_provider(InstanceProviderKind::None).await?;
            }
        }

        Ok(())
    }

    pub async fn update_rojo(&mut self, contents: Option<&str>) -> Result<()> {
        let proj: Option<RojoProjectFile> = contents
            .and_then(|c| if c.is_empty() { None } else { Some(c) })
            .and_then(|s| match serde_json::from_str(s) {
                Ok(v) => Some(v),
                Err(e) => {
                    error!("failed to deserialize rojo project file - {e}");
                    None
                }
            });

        // Replace stored project manifest, and check if it changed.
        // If the rojo project did not change substantially or in
        // a way that we care about, we can skip the rest below
        let is_some = proj.is_some();
        if self.last_project != proj {
            self.last_project = proj;
        } else {
            return Ok(());
        }

        // Stop / despawn any spawned process
        self.update_provider(InstanceProviderKind::None).await?;

        if is_some {
            // We have a project, try to spawn a new process
            self.update_provider(InstanceProviderKind::RojoSourcemap)
                .await
        } else {
            // No project, go back to either using manual
            // sourcemaps if we can or simply doing nothing
            if self.last_sourcemap.is_some() {
                self.update_provider(InstanceProviderKind::FileSourcemap)
                    .await
            } else {
                self.update_provider(InstanceProviderKind::None).await
            }
        }
    }
}
