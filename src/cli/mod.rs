use anyhow::Result;
use clap::{Parser, Subcommand};

mod generate_classes;
mod generate_icons;
mod generate_reflection;
mod serve;
mod tracing;

use generate_classes::*;
use generate_icons::*;
use generate_reflection::*;
use serve::*;
use tracing::*;

#[derive(Debug, Clone, Subcommand)]
pub enum CliSubcommand {
    GenerateClasses(GenerateClassesCommand),
    GenerateIcons(GenerateIconsCommand),
    GenerateReflection(GenerateReflectionCommand),
    Serve(ServeCommand),
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
        setup_tracing();

        match self.subcommand {
            CliSubcommand::GenerateClasses(cmd) => cmd.run().await,
            CliSubcommand::GenerateIcons(cmd) => cmd.run().await,
            CliSubcommand::GenerateReflection(cmd) => cmd.run().await,
            CliSubcommand::Serve(cmd) => cmd.run().await,
        }
    }
}
