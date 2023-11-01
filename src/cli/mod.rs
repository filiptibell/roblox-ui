use anyhow::Result;
use clap::{Parser, Subcommand};

mod generate_icons;
mod generate_reflection;

use generate_icons::*;
use generate_reflection::*;

#[derive(Debug, Clone, Subcommand)]
pub enum CliSubcommand {
    GenerateIcons(GenerateIconsCommand),
    GenerateReflection(GenerateReflectionCommand),
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
            CliSubcommand::GenerateReflection(cmd) => cmd.run().await,
        }
    }
}
