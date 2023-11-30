use anyhow::Result;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tracing::error;

mod file_sourcemap;
mod none;
mod rojo;
mod rojo_client;
mod rojo_sourcemap;
mod rojo_stub;
mod variant;

pub use rojo::*;
pub use variant::*;

use super::config::Config;
use super::dom::InstanceNode;

/**
    A fault-tolerant instance provider & watcher.

    Prioritizes instance watching in the following order:

    1. Using `rojo sourcemap --watch`, if a project is available and valid
    2. Using a `sourcemap.json` file, if available and valid
    3. Using empty data, which should display as blank in an explorer
*/
#[derive(Debug)]
pub struct InstanceProvider {
    config: Config,
    instance_tx: UnboundedSender<Option<InstanceNode>>,
    instance_rx: Option<UnboundedReceiver<Option<InstanceNode>>>,
    last_sourcemap: Option<InstanceNode>,
    last_project: Option<RojoProjectFile>,
    provider: Option<InstanceProviderVariant>,
}

impl InstanceProvider {
    pub fn new(config: Config) -> Self {
        let (instance_tx, instance_rx) = unbounded_channel();
        Self {
            config,
            instance_tx,
            instance_rx: Some(instance_rx),
            last_sourcemap: None,
            last_project: None,
            provider: None,
        }
    }

    pub fn take_instance_receiver(&mut self) -> Option<UnboundedReceiver<Option<InstanceNode>>> {
        self.instance_rx.take()
    }

    async fn update_inner(&mut self, desired_kind: InstanceProviderKind) -> Result<()> {
        let smap = self.last_sourcemap.as_ref();
        let proj = self.last_project.as_ref();
        if !matches!(self.provider.as_ref().map(|p| p.kind()), Some(k) if k == desired_kind) {
            // Create, start, and store a new provider, stop the old one if one existed
            let mut this = InstanceProviderVariant::from_kind(
                desired_kind,
                self.config.clone(),
                self.instance_tx.clone(),
            );
            match this.start(smap, proj).await {
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
            this.update(smap, proj).await?;
        }
        Ok(())
    }

    pub async fn update_file(&mut self, contents: Option<&str>) -> Result<()> {
        let smap = contents
            .and_then(|c| if c.is_empty() { None } else { Some(c) })
            .and_then(|s| match InstanceNode::from_json(s) {
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
                self.update_inner(InstanceProviderKind::FileSourcemap)
                    .await?;
            } else {
                self.update_inner(InstanceProviderKind::None).await?;
            }
        }

        Ok(())
    }

    pub async fn update_rojo(&mut self, contents: Option<&str>) -> Result<()> {
        let proj = contents
            .and_then(|c| if c.is_empty() { None } else { Some(c) })
            .and_then(|s| match RojoProjectFile::from_json(s) {
                Ok(v) => Some(v),
                Err(e) => {
                    error!("failed to deserialize rojo project file - {e}");
                    None
                }
            });

        if let Some(proj) = &proj {
            if let Some(_session) = proj.find_serve_session().await {
                // FUTURE: Use a "rojo session provider" if we manage to find
                // info!("found rojo serve session - {session:#?}");
            }
        }

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
        self.update_inner(InstanceProviderKind::None).await?;

        if is_some {
            // We have a project, try to spawn a new process
            self.update_inner(InstanceProviderKind::RojoSourcemap).await
        } else {
            // No project, go back to either using manual
            // sourcemaps if we can or simply doing nothing
            if self.last_sourcemap.is_some() {
                self.update_inner(InstanceProviderKind::FileSourcemap).await
            } else {
                self.update_inner(InstanceProviderKind::None).await
            }
        }
    }
}
