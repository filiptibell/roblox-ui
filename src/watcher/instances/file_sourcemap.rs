use anyhow::Result;
use tracing::trace;

use crate::watcher::Settings;

use super::*;

#[derive(Debug, Default)]
pub struct FileSourcemapProvider {
    settings: Settings,
    current: Option<SourcemapNode>,
}

impl FileSourcemapProvider {
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

    pub async fn update(&mut self, smap: Option<&SourcemapNode>) -> Result<()> {
        trace!("updating file provider");
        // TODO: Diff new sourcemap arg against the last known one here, emit changes, etc
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        trace!("stopping file provider");
        Ok(())
    }
}
