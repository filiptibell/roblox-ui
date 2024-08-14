use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{bail, Error, Result};
use bytes::Bytes;
use clap::Parser;
use futures::{stream::FuturesUnordered, TryStreamExt};
use tokio::{
    fs::{read, read_to_string},
    try_join,
};

use crate::icons::{discover_roblox_custom_dirs, CustomThemeFile, IconPackContents};

#[derive(Debug, Clone, Parser)]
pub struct GenerateCustomIconsCommand {
    #[arg(short, long)]
    input: PathBuf,
    #[arg(short, long)]
    output: PathBuf,
}

impl GenerateCustomIconsCommand {
    pub async fn run(self) -> Result<()> {
        if !self.input.exists() {
            bail!("Input directory '{}' does not exist", self.input.display());
        } else if !self.input.is_dir() {
            bail!("Input path '{}' is not a directory", self.input.display());
        }

        // Try to discover all themes in the input directory
        let custom_dirs = discover_roblox_custom_dirs(&self.input).await?;
        let custom_themes = custom_dirs
            .into_iter()
            .map(|dir| async move {
                let theme_str = read_to_string(dir.join("index.theme")).await?;
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
        } else if let (Some((theme_light_dir, theme_light)), Some((theme_dark_dir, theme_dark))) =
            (theme_light, theme_dark)
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
            bail!("No custom themes were found in '{}'", self.input.display());
        }

        // Finally, write the contents to the output dir
        println!("Writing all images to '{}'...", self.output.display());
        contents.write_to(&self.output).await?;

        Ok(())
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
        .map(|path| async move { read(&path).await.map(|bytes| (path, Bytes::from(bytes))) })
        .collect::<FuturesUnordered<_>>()
        .try_collect::<Vec<_>>()
        .await
        .map_err(Error::from)
}
