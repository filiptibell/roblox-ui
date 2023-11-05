use anyhow::Result;
use clap::Parser;
use tracing::debug;

use crate::server::{Config, Server};

#[derive(Debug, Clone, Parser)]
pub struct ServeCommand {
    #[arg(long)]
    pub settings: Option<Config>,
}

impl ServeCommand {
    pub async fn run(self) -> Result<()> {
        let config = self.settings.unwrap_or_default();

        debug!("Parsed arguments\nconfig: {config:#?}");

        Server::new(config).serve().await
    }
}
