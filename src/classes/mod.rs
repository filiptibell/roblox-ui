use std::collections::BTreeMap;
use std::ops::Not; // Skip serializing 'false' bools

use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use rbx_reflection::{ClassTag, ReflectionDatabase};
use serde::Serialize;

static CLASS_DATABASE: Lazy<&ReflectionDatabase> = Lazy::new(rbx_reflection_database::get);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Classes {
    pub class_count: usize,
    pub class_datas: BTreeMap<String, ClassData>,
}

impl Classes {
    pub fn from_database() -> Result<Self> {
        let mut class_datas = BTreeMap::new();

        for class_name in CLASS_DATABASE.classes.keys() {
            let class_data = ClassData::from_class_name(class_name.as_ref())?;
            class_datas.insert(class_name.clone().into_owned(), class_data);
        }

        Ok(Self {
            class_count: class_datas.len(),
            class_datas,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassData {
    pub name: String,
    #[serde(skip_serializing_if = "Not::not")]
    pub is_service: bool,
    #[serde(skip_serializing_if = "Not::not")]
    pub is_deprecated: bool,
    #[serde(skip_serializing_if = "Not::not")]
    pub not_browsable: bool,
    #[serde(skip_serializing_if = "Not::not")]
    pub not_creatable: bool,
}

impl ClassData {
    pub fn from_class_name<S>(name: S) -> Result<Self>
    where
        S: Into<String>,
    {
        let name = name.into();
        let desc = CLASS_DATABASE
            .classes
            .get(name.as_str())
            .with_context(|| format!("no class '{name}' was found in reflection database"))?;

        Ok(Self {
            name,
            is_service: desc.tags.contains(&ClassTag::Service),
            is_deprecated: desc.tags.contains(&ClassTag::Deprecated),
            not_browsable: desc.tags.contains(&ClassTag::NotBrowsable),
            not_creatable: desc.tags.contains(&ClassTag::NotCreatable),
        })
    }
}
