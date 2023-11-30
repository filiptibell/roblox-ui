use std::path::{Path, PathBuf};

use tokio::fs::{create_dir, read, remove_dir_all, remove_file, write};
use tokio::io;

use crate::util::rojo::{parse_name_and_class_name, CLASS_NAME_SUFFIXES};

use super::InstanceMetadataPaths;

enum InstancePathVariant<'a> {
    Dir(&'a Path),
    File(&'a Path),
    None,
}

fn is_init_path(path: &Path) -> bool {
    if let Some((name, _)) = parse_name_and_class_name(path) {
        name == "init"
    } else {
        false
    }
}

fn get_instance_path_variant(paths: &InstanceMetadataPaths) -> InstancePathVariant {
    let dir_path_opt = paths.folder.as_deref();
    let file_path_opt = paths.file.as_deref().or(paths.file_meta.as_deref());
    if matches!(file_path_opt.map(is_init_path), Some(true)) {
        InstancePathVariant::Dir(dir_path_opt.expect("paths with init file is missing dir"))
    } else if let Some(file_path) = file_path_opt {
        InstancePathVariant::File(file_path)
    } else if let Some(dir_path) = dir_path_opt {
        InstancePathVariant::Dir(dir_path)
    } else {
        InstancePathVariant::None
    }
}

async fn transform_file_to_dir_with_init(file_path: &Path) -> io::Result<(PathBuf, PathBuf)> {
    let parent_dir = file_path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Unsupported, "No parent dir"))?;

    let file_name = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::Unsupported, "No file name"))?;

    let (ext, _) = CLASS_NAME_SUFFIXES
        .iter()
        .find(|(ext, _)| file_name.ends_with(ext))
        .ok_or_else(|| io::Error::new(io::ErrorKind::Unsupported, "No matching extension"))?;

    let contents = read(&file_path).await?;
    remove_file(&file_path).await?;

    let new_name = file_name.trim_end_matches(ext);
    let new_dir = parent_dir.join(new_name);
    let new_init = new_dir.join(format!("init{ext}"));

    create_dir(&new_dir).await?;
    write(&new_init, contents).await?;

    Ok((new_dir, new_init))
}

async fn create_instance_in_dir(
    parent_path: &Path,
    class_name: &str,
    name: &str,
) -> io::Result<Vec<PathBuf>> {
    if class_name == "Folder" {
        let child_path = parent_path.join(name);
        create_dir(&child_path).await?;

        Ok(vec![child_path])
    } else {
        let (ext, _) = CLASS_NAME_SUFFIXES
            .iter()
            .find(|(_, ext_class)| class_name == *ext_class)
            .unwrap_or(&(".model.json", "Instance"));

        let file_name = format!("{name}{ext}");
        let child_path = parent_path.join(file_name);
        let child_contents = if *ext == ".model.json" {
            let json = serde_json::json!({
                "ClassName": class_name,
                "Properties": {},
            });
            serde_json::to_string_pretty(&json).unwrap()
        } else {
            String::new()
        };

        write(&child_path, child_contents).await?;
        Ok(vec![child_path, parent_path.to_path_buf()])
    }
}

pub async fn create_instance(
    parent_paths: &InstanceMetadataPaths,
    class_name: &str,
    name: &str,
) -> io::Result<(Vec<PathBuf>, Option<Vec<PathBuf>>)> {
    let mut changed_parent_paths = None;
    let mut new_child_paths = Vec::new();

    match get_instance_path_variant(parent_paths) {
        InstancePathVariant::Dir(dir_path) => {
            new_child_paths = create_instance_in_dir(dir_path, class_name, name).await?;
        }
        InstancePathVariant::File(file_path) => {
            let (new_parent_dir, new_parent_init) =
                transform_file_to_dir_with_init(file_path).await?;
            new_child_paths = create_instance_in_dir(&new_parent_dir, class_name, name).await?;
            changed_parent_paths = Some(vec![new_parent_dir, new_parent_init]);
        }
        InstancePathVariant::None => {}
    }

    Ok((new_child_paths, changed_parent_paths))
}

pub async fn delete_instance(instance_paths: &InstanceMetadataPaths) -> io::Result<()> {
    if let Some(meta_path) = instance_paths.file_meta.as_deref() {
        remove_file(meta_path).await?;
    }
    match get_instance_path_variant(instance_paths) {
        InstancePathVariant::Dir(dir_path) => remove_dir_all(dir_path).await,
        InstancePathVariant::File(file_path) => remove_file(file_path).await,
        InstancePathVariant::None => Ok(()),
    }
}
