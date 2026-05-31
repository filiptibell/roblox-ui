use anyhow::Result;
use tokio::sync::mpsc::UnboundedSender;
use tracing::trace;

use super::{super::config::Config, InstanceNode};

/**
    An instance provider that uses a `sourcemap.json` file to emit diffs.
*/
#[derive(Debug)]
pub struct FileSourcemapProvider {
    _config: Config,
    sender: UnboundedSender<Option<InstanceNode>>,
}

impl FileSourcemapProvider {
    pub fn new(config: Config, sender: UnboundedSender<Option<InstanceNode>>) -> Self {
        Self {
            _config: config,
            sender,
        }
    }

    pub async fn start(&mut self, smap: Option<&InstanceNode>) -> Result<()> {
        trace!("starting file provider");

        self.sender.send(smap.cloned()).ok();

        Ok(())
    }

    pub async fn update(&mut self, smap: Option<&InstanceNode>) -> Result<()> {
        trace!("updating file provider");

        self.sender.send(smap.cloned()).ok();

        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        trace!("stopping file provider");

        Ok(())
    }
}
