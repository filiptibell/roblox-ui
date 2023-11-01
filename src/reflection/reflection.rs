use std::collections::BTreeMap;
use std::io::{Cursor, Read};

use anyhow::{Context, Result};
use serde::Serialize;

use super::constants::*;
use super::reflection_node::*;
use super::reflection_value::*;

#[derive(Debug, Clone, Serialize)]
pub struct ReflectionMetadata {
    pub classes: BTreeMap<String, ReflectionMetadataClass>,
    pub enums: BTreeMap<String, ReflectionMetadataEnum>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReflectionMetadataClass {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, Value>,
    // FUTURE: Include properties, methods, events?
}

#[derive(Debug, Clone, Serialize)]
pub struct ReflectionMetadataEnum {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<ReflectionMetadataEnumItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReflectionMetadataEnumItem {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, Value>,
}

pub async fn download_latest_studio() -> Result<Vec<u8>> {
    let client = reqwest::Client::new();

    let version_bytes = client
        .get(URL_VERSION)
        .send()
        .await
        .context("failed to send version string request")?
        .bytes()
        .await
        .context("failed to get version string bytes")?;
    let version_string = String::from_utf8(version_bytes.to_vec())
        .context("failed to parse version string as utf-8")?;

    let studio_url = URL_STUDIO.replace(URL_VERSION_MARKER, &version_string);
    let studio_bytes = client
        .get(studio_url)
        .send()
        .await
        .context("failed to send studio request")?
        .bytes()
        .await
        .context("failed to get studio bytes")?;

    Ok(studio_bytes.to_vec())
}

pub fn extract_reflection_metadata_from_zip(studio_bytes: &[u8]) -> Result<Vec<u8>> {
    let mut reader = Cursor::new(studio_bytes);
    let mut archive =
        zip::ZipArchive::new(&mut reader).context("failed to read classic icon pack zip file")?;

    let file_idx = (0..archive.len())
        .find(|i| {
            if let Ok(file) = archive.by_index(*i) {
                file.is_file()
                    && matches!(
                        file.enclosed_name()
                            .and_then(|p| p.file_name())
                            .and_then(|f| f.to_str()),
                        Some("ReflectionMetadata.xml")
                    )
            } else {
                false
            }
        })
        .context("failed to find reflection metadata in studio zip")?;

    let mut file = archive.by_index(file_idx).unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    Ok(buffer)
}

pub fn parse_reflection_metadata(reflection_bytes: &[u8]) -> Result<ReflectionMetadata> {
    let reflection_tree = parse_reflection_tree(reflection_bytes)?;

    let classes = reflection_tree
        .find_child(|child| matches!(child.name(), Some("Classes")))
        .context("missing classes in reflection metadata")?
        .children()
        .iter()
        .filter_map(|child| child.split_properties())
        .collect::<Vec<_>>();
    let enums = reflection_tree
        .find_child(|child| matches!(child.name(), Some("Enums")))
        .context("missing enums in reflection metadata")?
        .children()
        .iter()
        .filter_map(|child| child.split_properties())
        .collect::<Vec<_>>();

    let mut reflection_data = ReflectionMetadata {
        classes: BTreeMap::new(),
        enums: BTreeMap::new(),
    };

    for (props, _rest) in classes {
        let name = props.extract_name_node_string()?;
        let mut values = props.extract_non_name_values()?;
        let summary = values
            .remove("summary")
            .and_then(|s| s.as_string().map(|s| s.to_string()));

        reflection_data.classes.insert(
            name.clone(),
            ReflectionMetadataClass {
                name,
                summary,
                values,
            },
        );
    }

    for (props, rest) in enums {
        let name = props.extract_name_node_string()?;
        let mut values = props.extract_non_name_values()?;
        let summary = values
            .remove("summary")
            .and_then(|s| s.as_string().map(|s| s.to_string()));

        let mut items = Vec::new();
        for item in rest {
            if matches!(item, Node::Item { name, .. } if name == "EnumItem") {
                let (item_props, _) = item
                    .split_properties()
                    .context("EnumItem is missing Properties")?;
                let item_name = item_props
                    .extract_name_node_string()
                    .context("EnumItem is missing Name property")?;
                let mut item_values = item_props.extract_non_name_values()?;
                let item_summary = item_values
                    .remove("summary")
                    .and_then(|s| s.as_string().map(|s| s.to_string()));
                items.push(ReflectionMetadataEnumItem {
                    name: item_name,
                    summary: item_summary,
                    values: item_values,
                })
            }
        }

        reflection_data.enums.insert(
            name.clone(),
            ReflectionMetadataEnum {
                name,
                summary,
                values,
                items,
            },
        );
    }

    Ok(reflection_data)
}
