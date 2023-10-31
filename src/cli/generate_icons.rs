use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::Parser;
use futures::future::join_all;
use tokio::fs;

use crate::icons::*;

#[derive(Debug, Clone, Parser)]
pub struct GenerateIconsCommand {
    #[arg(short, long, group = "exclusive")]
    all: bool,
    #[arg(short, long, group = "exclusive")]
    pack: Option<IconPack>,
    #[arg(short, long)]
    output: PathBuf,
}

impl GenerateIconsCommand {
    pub async fn run(self) -> Result<()> {
        if self.all {
            let packs = IconPack::all();

            fs::remove_dir_all(&self.output).await.ok();

            println!("Downloading icon packs...");
            let mut all_contents_futs = Vec::new();
            for pack in packs {
                all_contents_futs.push(pack.provider().download());
            }
            let mut all_contents = Vec::new();
            for result in join_all(all_contents_futs).await {
                all_contents.push(result.context("failed to download icon pack contents")?);
            }

            println!("Writing icon packs...");
            let mut all_files_futs = Vec::new();
            for (index, contents) in all_contents.iter().enumerate() {
                let pack_name = packs[index].to_string();
                let pack_path = self.output.join(pack_name);
                all_files_futs.push(contents.write_to(pack_path));
            }
            for result in join_all(all_files_futs).await {
                result.context("failed to write icon pack contents")?;
            }

            let total_len = all_contents.iter().fold(0, |acc, c| acc + c.len());
            println!(
                "Generated {} icon packs with {} files total to '{}'",
                packs.len(),
                total_len,
                self.output.display()
            );

            Ok(())
        } else if let Some(pack) = self.pack {
            fs::remove_dir_all(&self.output).await.ok();

            println!("Downloading icon pack '{pack}'...");

            let contents = pack
                .provider()
                .download()
                .await
                .context("failed to download icon pack contents")?;

            println!("Writing icon pack to '{}'...", self.output.display());

            contents
                .write_to(&self.output)
                .await
                .context("failed to write icon pack contents")?;

            println!("Generated {} files total", contents.len());

            Ok(())
        } else {
            bail!("missing icon pack arg")
        }
    }
}
