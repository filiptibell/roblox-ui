use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use tracing::info;

use crate::data::*;

#[derive(Debug, Clone, Parser)]
pub struct GenerateDataCommand {
    #[arg(short, long)]
    output: PathBuf,
}

impl GenerateDataCommand {
    pub async fn run(self) -> Result<()> {
        info!("Downloading api docs...");
        let tree = ApiDocTree::download().await?;

        let path = self.output.join("api_docs.json");
        let json = serde_json::to_vec_pretty(&tree)?;
        tokio::fs::write(path, json).await?;

        Ok(())
    }
}
