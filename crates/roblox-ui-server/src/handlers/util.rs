use rbx_dom_weak::{types::Ref, Instance};
use serde::Serialize;
use ustr::Ustr;

use roblox_ui_project::{Dom, InstanceMetadata};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ResponseInstance {
    id: Ref,
    #[serde(skip_serializing_if = "Ref::is_none")]
    parent_id: Ref,
    class_name: Ustr,
    name: String,
    children: Vec<Ref>,
    metadata: Option<InstanceMetadata>,
}

impl ResponseInstance {
    pub fn from_dom_instance(inst: &Instance) -> Self {
        Self {
            id: inst.referent(),
            parent_id: inst.parent(),
            class_name: inst.class,
            name: inst.name.to_owned(),
            children: inst.children().to_vec(),
            metadata: None,
        }
    }

    pub fn with_dom_metadata(mut self, dom: &Dom) -> Self {
        self.metadata = dom.get_metadata(self.id).cloned();
        self
    }
}
