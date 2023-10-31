use std::io::{Cursor, Read};

use anyhow::{Context, Result};

use super::*;

pub struct ClassicIconPack;

const PACK_ZIP_URL: &str = concat!(
    "https://devforum.roblox.com/uploads/short-url/",
    "vHjNEH4jUjBlz9tl8Yetp6T1b99.zip"
);

#[async_trait::async_trait]
impl IconPackProvider for ClassicIconPack {
    async fn download(&self) -> Result<IconPackContents> {
        let bytes = reqwest::get(PACK_ZIP_URL)
            .await
            .context("failed to fetch classic icon pack (1)")?
            .bytes()
            .await
            .context("failed to fetch classic icon pack (2)")?
            .to_vec();

        let mut reader = Cursor::new(bytes);
        let mut contents = IconPackContents::new();

        let mut archive = zip::ZipArchive::new(&mut reader)
            .context("failed to read classic icon pack zip file")?;
        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .context("failed to read classic icon pack zip file")?;

            if !file.is_file() {
                continue;
            }

            if let Some(Ok(icon_path)) = file.enclosed_name().map(|p| {
                p.strip_prefix("RobloxCustom/standard/")
                    .map(|p| p.to_path_buf())
            }) {
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer)?;
                contents.insert_icon(icon_path, buffer);
            }
        }

        Ok(contents)
    }
}
