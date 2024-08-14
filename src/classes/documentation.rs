use std::collections::BTreeMap;
use std::str::FromStr;

use anyhow::{Context, Result};
use serde::Deserialize;
use url::Url;

use super::constants::*;
use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum ApiDocKey {
    RobloxGlobal(String),
    RobloxEnum(String),
    RobloxEnumItem(String, String),
    LuauGlobal(String),
}

impl FromStr for ApiDocKey {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = s.split('/').collect::<Vec<_>>();

        let mut it = parts.into_iter();
        let scope = it.next().ok_or_else(|| anyhow::anyhow!("missing scope"))?;
        let kind = it.next().ok_or_else(|| anyhow::anyhow!("missing kind"))?;
        let name = it.next().ok_or_else(|| anyhow::anyhow!("missing name"))?;

        match scope.trim_start_matches('@').to_ascii_lowercase().as_str() {
            "luau" => Ok(Self::LuauGlobal(name.to_string())),
            "roblox" => match kind.trim_start_matches('@').to_ascii_lowercase().as_str() {
                "global" | "globaltype" => Ok(Self::RobloxGlobal(name.to_string())),
                "enum" => match name.split_once('.') {
                    Some((enum_name, item_name)) => Ok(Self::RobloxEnumItem(
                        enum_name.to_string(),
                        item_name.to_string(),
                    )),
                    None => Ok(Self::RobloxEnum(name.to_string())),
                },
                _ => Err(anyhow::anyhow!("unknown Roblox kind")),
            },
            _ => Err(anyhow::anyhow!("unknown scope")),
        }
    }
}

impl<'de> serde::Deserialize<'de> for ApiDocKey {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        ApiDocKey::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Deserialize)]
struct ApiDocItem {
    documentation: Option<String>,
    learn_more_link: Option<String>,
    #[serde(default)]
    keys: BTreeMap<String, String>,
}

pub async fn insert_documentation(classes: &mut Classes) -> Result<()> {
    let bytes = reqwest::get(API_DOCS_URL)
        .await
        .context("failed to fetch api docs json (1)")?
        .bytes()
        .await
        .context("failed to fetch api docs json (2)")?;

    let docs = serde_json::from_slice::<BTreeMap<ApiDocKey, ApiDocItem>>(&bytes)
        .context("failed to deserialize api docs json")?;

    for (doc_key, doc_item) in docs {
        let class_name = match &doc_key {
            ApiDocKey::RobloxGlobal(c) => c,
            _ => continue,
        };
        let class_data = match classes.class_datas.get_mut(class_name) {
            Some(d) => d,
            None => continue,
        };
        if let Some(desc) = doc_item.documentation {
            if !desc.trim().is_empty() {
                class_data.description = Some(desc);
            }
        }
        if let Some(url) = doc_item.learn_more_link {
            if !url.trim().is_empty() {
                let parsed = Url::parse(&url).with_context(|| {
                    format!("failed to parse url '{url}' for api doc item '{class_name}'")
                })?;
                class_data.documentation_url = Some(parsed);
            }
        }
    }

    Ok(())
}
