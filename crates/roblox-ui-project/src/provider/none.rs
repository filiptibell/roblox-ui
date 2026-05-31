use anyhow::Result;
use async_channel::Sender;
use tracing::trace;

use super::{super::config::Config, InstanceNode};

/**
    An instance provider that emits `null` once at startup.

    Used when no other instance providers are available.
*/
#[derive(Debug)]
pub struct NoneProvider {
    _config: Config,
    sender: Sender<Option<InstanceNode>>,
}

impl NoneProvider {
    pub fn new(config: Config, sender: Sender<Option<InstanceNode>>) -> Self {
        Self {
            _config: config,
            sender,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        trace!("starting none provider");

        self.sender.try_send(None).ok();

        Ok(())
    }

    pub async fn update(&mut self) -> Result<()> {
        trace!("updating none provider");

        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        trace!("stopping none provider");

        Ok(())
    }
}
