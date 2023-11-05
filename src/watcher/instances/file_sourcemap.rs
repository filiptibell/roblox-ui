use anyhow::Result;
use tracing::trace;

use crate::watcher::Settings;

use super::*;

#[derive(Debug, Default)]
pub struct FileSourcemapProvider {
    _settings: Settings,
    sourcemap: Option<InstanceNode>,
}

impl FileSourcemapProvider {
    pub fn new(settings: Settings) -> Self {
        Self {
            _settings: settings,
            sourcemap: None,
        }
    }

    pub async fn start(&mut self, smap: Option<&InstanceNode>) -> Result<()> {
        trace!("starting file provider");

        match smap {
            None => println!("null"),
            Some(init) => {
                self.sourcemap.replace(init.clone());
                println!("{}", init.diff_full());
            }
        }

        Ok(())
    }

    pub async fn update(&mut self, smap: Option<&InstanceNode>) -> Result<()> {
        trace!("updating file provider");

        match (self.sourcemap.take(), smap) {
            (None, None) => {}
            (Some(_), None) => {
                println!("null")
            }
            (None, Some(new)) => {
                self.sourcemap.replace(new.clone());
                println!("{}", new.diff_full())
            }
            (Some(old), Some(new)) => {
                self.sourcemap.replace(new.clone());
                println!("{}", old.diff_with(new));
            }
        }

        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        trace!("stopping file provider");

        Ok(())
    }
}
