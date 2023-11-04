use anyhow::Result;
use tracing::trace;

use crate::watcher::Settings;

use super::SourcemapNode;

#[derive(Debug, Default)]
pub struct FileProvider {
    settings: Settings,
    current: Option<SourcemapNode>,
}

impl FileProvider {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings,
            current: None,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        trace!("starting file provider");
        Ok(())
    }

    pub async fn update(&mut self) -> Result<()> {
        trace!("updating file provider");
        // TODO: Grab the stored sourcemap, diff it against
        // the last known one here, emit changes, etc
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        trace!("stopping file provider");
        Ok(())
    }
}
