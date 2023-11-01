use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use tokio::fs;

use crate::reflection::*;
use crate::util::zip::*;

#[derive(Debug, Clone, Parser)]
pub struct GenerateReflectionCommand {
    #[arg(short, long)]
    output: PathBuf,
}

impl GenerateReflectionCommand {
    pub async fn run(self) -> Result<()> {
        println!("Downloading latest Roblox Studio...");
        let studio = download_latest_studio().await?;

        println!("Parsing reflection metadata...");
        let reflection_bytes = extract_file_from_zip(&studio, "ReflectionMetadata.xml")?;
        let reflection_metadata = parse_reflection_metadata(&reflection_bytes)?;

        println!("Writing reflection file...");
        let reflection_json = serde_json::to_string(&reflection_metadata)?;
        fs::write(&self.output, reflection_json).await?;

        println!("Generated reflection at '{}'", self.output.display());

        Ok(())
    }
}
