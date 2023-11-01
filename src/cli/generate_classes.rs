use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use tokio::fs;

use crate::classes::*;

#[derive(Debug, Clone, Parser)]
pub struct GenerateClassesCommand {
    #[arg(short, long)]
    output: PathBuf,
}

impl GenerateClassesCommand {
    pub async fn run(self) -> Result<()> {
        println!("Generating class datas...");
        let classes = Classes::from_database()?;
        let classes_json = serde_json::to_string(&classes)
            .context("failed to serialize class datas into json file")?;

        println!("Writing classes file...");
        fs::write(&self.output, classes_json).await?;

        println!("Generated classes at '{}'", self.output.display());

        Ok(())
    }
}
