use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use tokio::fs;
use tracing::info;

use crate::reflection::*;
use crate::util::zip::*;

#[derive(Debug, Clone, Parser)]
pub struct GenerateReflectionCommand {
    #[arg(short, long)]
    output: PathBuf,
}

impl GenerateReflectionCommand {
    pub async fn run(self) -> Result<()> {
        info!("Downloading latest Roblox Studio...");
        let studio = download_latest_studio().await?;

        info!("Parsing reflection metadata...");
        let reflection_bytes = extract_file_from_zip(&studio, "ReflectionMetadata.xml")?;
        let reflection_metadata = parse_reflection_metadata(&reflection_bytes)?;

        info!("Writing reflection file...");
        let reflection_json = serde_json::to_string(&reflection_metadata)?;
        fs::write(&self.output, reflection_json).await?;

        info!("Generated reflection at '{}'", self.output.display());

        Ok(())
    }
}
