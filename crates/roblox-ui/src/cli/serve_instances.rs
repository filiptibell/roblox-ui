use anyhow::Result;
use clap::Parser;
use tracing::debug;

use roblox_ui_server::{Config, Server};

#[derive(Debug, Clone, Parser)]
pub struct ServeInstancesCommand {
    #[arg(long, env)]
    pub settings: Option<Config>,
}

impl ServeInstancesCommand {
    pub async fn run(self) -> Result<()> {
        let config = self.settings.unwrap_or_default();

        debug!("Parsed arguments\nconfig: {config:#?}");

        Server::new(config).serve_instances().await
    }
}
