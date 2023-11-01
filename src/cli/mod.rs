#![allow(clippy::enum_variant_names)]

use anyhow::Result;
use clap::{Parser, Subcommand};

mod generate_classes;
mod generate_icons;
mod generate_reflection;

use generate_classes::*;
use generate_icons::*;
use generate_reflection::*;

#[derive(Debug, Clone, Subcommand)]
pub enum CliSubcommand {
    GenerateClasses(GenerateClassesCommand),
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
            CliSubcommand::GenerateClasses(cmd) => cmd.run().await,
            CliSubcommand::GenerateIcons(cmd) => cmd.run().await,
            CliSubcommand::GenerateReflection(cmd) => cmd.run().await,
        }
    }
}
