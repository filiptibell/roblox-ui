use anyhow::Result;
use async_channel::Sender;
use tracing::trace;

use super::{super::config::Config, InstanceNode};

/**
    An instance provider that uses a `sourcemap.json` file to emit diffs.
*/
#[derive(Debug)]
pub struct FileSourcemapProvider {
    _config: Config,
    sender: Sender<Option<InstanceNode>>,
}

impl FileSourcemapProvider {
    pub fn new(config: Config, sender: Sender<Option<InstanceNode>>) -> Self {
        Self {
            _config: config,
            sender,
        }
    }

    pub async fn start(&mut self, smap: Option<&InstanceNode>) -> Result<()> {
        trace!("starting file provider");

        self.sender.try_send(smap.cloned()).ok();

        Ok(())
    }

    pub async fn update(&mut self, smap: Option<&InstanceNode>) -> Result<()> {
        trace!("updating file provider");

        self.sender.try_send(smap.cloned()).ok();

        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        trace!("stopping file provider");

        Ok(())
    }
}
