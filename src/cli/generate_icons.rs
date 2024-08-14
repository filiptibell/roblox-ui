use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{bail, Context, Error, Result};
use bytes::Bytes;
use clap::Parser;
use futures::{future::join_all, stream::FuturesUnordered, TryStreamExt as _};
use tokio::{fs, try_join};
use tracing::info;

use crate::icons::*;

#[derive(Debug, Clone, Parser)]
pub struct GenerateIconsCommand {
    #[arg(short, long, group = "exclusive")]
    all: bool,
    #[arg(short, long, group = "exclusive")]
    pack: Option<IconPack>,
    #[arg(short, long, group = "exclusive")]
    input: Option<PathBuf>,
    #[arg(short, long)]
    output: PathBuf,
}

impl GenerateIconsCommand {
    pub async fn run(self) -> Result<()> {
        if self.all {
            let packs = IconPack::all();

            fs::remove_dir_all(&self.output).await.ok();

            info!("Downloading icon packs...");
            let mut all_contents_futs = Vec::new();
            for pack in packs {
                all_contents_futs.push(pack.download());
            }
            let mut all_contents = Vec::new();
            for result in join_all(all_contents_futs).await {
                all_contents.push(result.context("failed to download icon pack contents")?);
            }

            info!("Writing icon packs...");
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
            info!(
                "Generated {} icon packs with {} files total to '{}'",
                packs.len(),
                total_len,
                self.output.display()
            );

            Ok(())
        } else if let Some(input) = self.input.as_deref() {
            if !input.exists() {
                bail!("Input directory '{}' does not exist", input.display());
            } else if !input.is_dir() {
                bail!("Input path '{}' is not a directory", input.display());
            }

            // Try to discover all themes in the input directory
            let custom_dirs = discover_roblox_custom_dirs(&input).await?;
            let custom_themes = custom_dirs
                .into_iter()
                .map(|dir| async move {
                    let theme_str = fs::read_to_string(dir.join("index.theme")).await?;
                    let theme_file = CustomThemeFile::from_str(&theme_str)?;
                    Ok::<_, anyhow::Error>((dir, theme_file))
                })
                .collect::<FuturesUnordered<_>>()
                .try_collect::<Vec<_>>()
                .await?;

            // Separate into dark and light themes, or just one theme if there is only one
            let theme_dark = custom_themes
                .iter()
                .find(|(dir, _)| parent_dir_is(dir, "dark"));
            let theme_light = custom_themes
                .iter()
                .find(|(dir, _)| parent_dir_is(dir, "light"));
            let theme_dark_or_light_or_whatever =
                if theme_dark.is_none() && theme_light.is_none() && custom_themes.len() == 1 {
                    Some(custom_themes.first().unwrap())
                } else {
                    None
                };
            println!("Processing {} found themes...", custom_themes.len());

            // Create pack contents for the custom theme
            let mut contents = IconPackContents::new();
            if let Some((theme_any_dir, theme_any)) = theme_dark_or_light_or_whatever {
                // Create pack contents from the single custom theme
                let theme_any_image_paths = theme_any.best_instances_paths(theme_any_dir).await?;
                let theme_any_image_bytes = read_all_files(theme_any_image_paths).await?;
                println!(
                    "Found {} custom theme images...",
                    theme_any_image_bytes.len()
                );
                for (path, image) in theme_any_image_bytes {
                    let rel_path = PathBuf::from(path.file_name().unwrap());
                    contents.insert_icon(rel_path, image);
                }
            } else if let (
                Some((theme_light_dir, theme_light)),
                Some((theme_dark_dir, theme_dark)),
            ) = (theme_light, theme_dark)
            {
                // Create pack contents from dark + light custom theme
                let (theme_light_image_paths, theme_dark_image_paths) = try_join!(
                    theme_light.best_instances_paths(theme_light_dir),
                    theme_dark.best_instances_paths(theme_dark_dir)
                )?;
                let (theme_light_image_bytes, theme_dark_image_bytes) = try_join!(
                    read_all_files(theme_light_image_paths),
                    read_all_files(theme_dark_image_paths)
                )?;
                println!(
                    "Found {} light and {} dark custom theme images...",
                    theme_light_image_bytes.len(),
                    theme_dark_image_bytes.len()
                );
                for (path, image) in theme_light_image_bytes {
                    let rel_path = PathBuf::from(path.file_name().unwrap());
                    contents.insert_icon_light(rel_path, image);
                }
                for (path, image) in theme_dark_image_bytes {
                    let rel_path = PathBuf::from(path.file_name().unwrap());
                    contents.insert_icon_dark(rel_path, image);
                }
            } else {
                bail!("No custom themes were found in '{}'", input.display());
            }

            // Finally, write the contents to the output dir
            println!("Writing all images to '{}'...", self.output.display());
            contents.write_to(&self.output).await?;

            Ok(())
        } else if let Some(pack) = self.pack {
            fs::remove_dir_all(&self.output).await.ok();

            info!("Downloading icon pack '{pack}'...");

            let contents = pack
                .download()
                .await
                .context("failed to download icon pack contents")?;

            info!("Writing icon pack to '{}'...", self.output.display());

            contents
                .write_to(&self.output)
                .await
                .context("failed to write icon pack contents")?;

            info!("Generated {} files total", contents.len());

            Ok(())
        } else {
            bail!("missing icon pack arg")
        }
    }
}

fn parent_dir_is(roblox_custom_dir: impl AsRef<Path>, name: &'static str) -> bool {
    roblox_custom_dir.as_ref().parent().is_some_and(|p| {
        p.file_name()
            .map(|f| f.eq_ignore_ascii_case(name))
            .unwrap_or_default()
    })
}

async fn read_all_files(paths: Vec<PathBuf>) -> Result<Vec<(PathBuf, Bytes)>> {
    paths
        .into_iter()
        .map(|path| async move {
            fs::read(&path)
                .await
                .map(|bytes| (path, Bytes::from(bytes)))
        })
        .collect::<FuturesUnordered<_>>()
        .try_collect::<Vec<_>>()
        .await
        .map_err(Error::from)
}
