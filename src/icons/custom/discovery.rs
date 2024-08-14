use std::path::PathBuf;

use tokio::fs::read_dir;

pub async fn discover_roblox_custom_dirs(
    path: impl Into<PathBuf>,
) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut found = Vec::new();
    let mut stack = vec![path.into()];
    while let Some(dir) = stack.pop() {
        let mut reader = read_dir(dir).await?;
        let mut inner = Vec::new();
        while let Some(entry) = reader.next_entry().await? {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                if entry_path
                    .file_name()
                    .is_some_and(|d| d.eq_ignore_ascii_case("RobloxCustom"))
                {
                    found.push(entry_path);
                    inner.clear();
                    break;
                } else {
                    inner.push(entry_path);
                }
            }
        }
        stack.extend(inner);
    }
    Ok(found)
}
