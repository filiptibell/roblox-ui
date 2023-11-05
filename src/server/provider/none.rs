use anyhow::Result;
use tracing::trace;

use crate::server::Config;

/**
    An instance provider that emits `null` once at startup.

    Used when no other instance providers are available.
*/
#[derive(Debug, Default)]
pub struct NoneProvider {
    _config: Config,
}

impl NoneProvider {
    pub fn new(config: Config) -> Self {
        Self { _config: config }
    }

    pub async fn start(&mut self) -> Result<()> {
        trace!("starting none provider");

        println!("null");

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
