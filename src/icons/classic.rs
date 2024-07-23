use std::path::Path;

use anyhow::{Context, Result};

use crate::util::zip::extract_files_from_zip;

use super::*;

const PACK_FILE_PREFIX: &str = "RobloxCustom/standard/";
const PACK_ZIP_URL: &str = concat!(
    "https://devforum.roblox.com/uploads/short-url/",
    "vHjNEH4jUjBlz9tl8Yetp6T1b99.zip"
);

pub struct Classic;

impl IconPackProvider for Classic {
    async fn download(&self) -> Result<IconPackContents> {
        let bytes = reqwest::get(PACK_ZIP_URL)
            .await
            .context("failed to fetch classic icon pack (1)")?
            .bytes()
            .await
            .context("failed to fetch classic icon pack (2)")?
            .to_vec();

        let files = extract_files_from_zip(
            bytes,
            Some(|path: &Path| path.starts_with(PACK_FILE_PREFIX)),
        )?;

        let mut contents = IconPackContents::new();

        for (file_path, file_bytes) in files {
            contents.insert_icon(
                file_path
                    .strip_prefix(PACK_FILE_PREFIX)
                    .expect("file paths should have been stripped"),
                file_bytes,
            );
        }

        Ok(contents)
    }
}
