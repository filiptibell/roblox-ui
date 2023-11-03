use anyhow::Result;
use clap::Parser;
use tracing::debug;

use crate::watcher::{Settings, Watcher, WatcherArguments};

#[derive(Debug, Clone, Parser)]
pub struct WatchCommand {
    #[arg(long)]
    pub settings: Option<Settings>,
}

impl WatchCommand {
    pub async fn run(self) -> Result<()> {
        let args = WatcherArguments {
            settings: self.settings.unwrap_or_default(),
        };

        debug!("Parsed arguments\nsettings: {:#?}", args.settings);

        Watcher::new(args).watch().await
    }
}
