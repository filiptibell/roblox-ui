use anyhow::Result;
use clap::{Parser, Subcommand};

mod generate_icons;
use generate_icons::*;

#[derive(Debug, Clone, Subcommand)]
pub enum CliSubcommand {
    GenerateIcons(GenerateIconsCommand),
}

#[derive(Debug, Clone, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[clap(subcommand)]
    subcommand: CliSubcommand,
}

impl Cli {
    pub fn new() -> Self {
        Self::parse()
    }

    pub async fn run(self) -> Result<()> {
        match self.subcommand {
            CliSubcommand::GenerateIcons(cmd) => cmd.run().await,
        }
    }
}
