use anyhow::Result;
use clap::Parser;
use tracing::debug;

use crate::watcher::{Transport, Watcher, WatcherArguments};

#[derive(Debug, Clone, Parser)]
pub struct WatchCommand {
    #[arg(long, alias = "port")]
    pub socket: Option<u16>,
    #[arg(long)]
    pub stdio: bool,
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
        };

        debug!("Parsed arguments\n\ttransport: {}", args.transport);

        Watcher::new(args).watch().await
    }
}
