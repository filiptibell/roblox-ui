use std::path::Path;

use anyhow::Result;

use roblox_ui_util::zip::extract_files_from_zip;

use super::*;

mod constants;

use constants::*;

pub struct Classic;

impl IconPackProvider for Classic {
    async fn get(&self) -> Result<IconPackContents> {
        let files = extract_files_from_zip(
            PACK_FILE_ZIPPED,
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
