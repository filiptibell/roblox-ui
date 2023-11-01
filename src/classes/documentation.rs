use std::collections::HashMap;

use anyhow::{Context, Result};
use serde::Deserialize;
use url::Url;

use super::constants::*;
use super::*;

#[derive(Debug, Clone, Deserialize)]
struct ApiDocItem {
    documentation: Option<String>,
    learn_more_link: Option<String>,
}

fn parse_class_name(path: &str) -> Option<&str> {
    let parts = path.split('/').collect::<Vec<_>>();
    let last = parts.last()?;
    if !last.contains('.') {
        Some(last)
    } else {
        None
    }
}

pub async fn insert_documentation(classes: &mut Classes) -> Result<()> {
    let bytes = reqwest::get(API_DOCS_URL)
        .await
        .context("failed to fetch api docs json (1)")?
        .bytes()
        .await
        .context("failed to fetch api docs json (2)")?;

    let docs = serde_json::from_slice::<HashMap<String, ApiDocItem>>(&bytes)
        .context("failed to deserialize api docs json")?;

    for (path, doc_item) in docs {
        let class_name = match parse_class_name(&path) {
            Some(c) => c,
            None => continue,
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
