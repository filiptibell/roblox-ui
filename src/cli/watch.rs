use anyhow::Result;
use clap::Parser;
use tracing::debug;

use crate::watcher::{Settings, Transport, Watcher, WatcherArguments};

#[derive(Debug, Clone, Parser)]
pub struct WatchCommand {
    #[arg(long, alias = "port")]
    pub socket: Option<u16>,
    #[arg(long)]
    pub stdio: bool,
    #[arg(long)]
    pub settings: Option<Settings>,
}

impl WatchCommand {
    pub async fn run(self) -> Result<()> {
        let transport = if let Some(port) = self.socket {
            Some(Transport::Socket(port))
        } else if self.stdio {
            Some(Transport::Stdio)
        } else {
            None
        };

        let args = WatcherArguments {
            transport: transport.unwrap_or_default(),
            settings: self.settings.unwrap_or_default(),
        };

        debug!(
            "Parsed arguments\n\ttransport: {}\n\tsettings: {:?}",
            args.transport, args.settings
        );

        Watcher::new(args).watch().await
    }
}
