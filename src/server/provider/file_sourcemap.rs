use anyhow::Result;
use tracing::trace;

use crate::server::Config;

use super::*;

/**
    An instance provider that uses a `sourcemap.json` file to emit diffs.
*/
#[derive(Debug, Default)]
pub struct FileSourcemapProvider {
    _config: Config,
    sourcemap: Option<InstanceNode>,
}

impl FileSourcemapProvider {
    pub fn new(config: Config) -> Self {
        Self {
            _config: config,
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
