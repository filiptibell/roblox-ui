use std::collections::BTreeMap;

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value as JsonValue;

use super::constants::*;

mod item;
mod key;
mod link;

pub use item::*;
pub use key::*;
pub use link::*;

#[derive(Debug, Clone, Serialize)]
pub struct ApiDocTree {
    #[serde(flatten)]
    inner: BTreeMap<ApiDocKey, ApiDocItem>,
}

impl ApiDocTree {
    pub async fn download() -> Result<Self> {
        let bytes = reqwest::get(API_DOCS_URL)
            .await
            .context("failed to fetch api docs json (1)")?
            .bytes()
            .await
            .context("failed to fetch api docs json (2)")?;

        let json = serde_json::from_slice::<BTreeMap<String, JsonValue>>(&bytes)
            .context("failed to deserialize api docs json")?;

        let mut inner = BTreeMap::new();
        for (string_key, json_value) in json {
            let key = ApiDocKey::parse(&string_key)
                .with_context(|| format!("failed to parse api docs key '{string_key}'"))?;
            let item = ApiDocItem::parse(&json_value)
                .with_context(|| format!("failed to parse api docs item '{string_key}'"))?;
            inner.insert(key, item);
        }

        Ok(Self { inner })
    }
}
