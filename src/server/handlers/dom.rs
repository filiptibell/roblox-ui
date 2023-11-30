use std::path::PathBuf;

use anyhow::{Context, Result};
use rbx_dom_weak::types::Ref;
use serde::Deserialize;

use crate::server::{
    dom::{Dom, DomQueryParams},
    rpc::RpcMessage,
};

use super::util::ResponseInstance;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RootRequest {}

impl RootRequest {
    pub async fn respond_to(self, msg: RpcMessage, dom: &mut Dom) -> Result<RpcMessage> {
        let instance = dom
            .get_root_id()
            .and_then(|id| dom.get_instance(id))
            .map(ResponseInstance::from_dom_instance)
            .map(|inst| inst.with_dom_metadata(dom));
        msg.respond()
            .with_data(instance)
            .context("failed to serialize response")
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct GetRequest {
    id: Ref,
}

impl GetRequest {
    pub async fn respond_to(self, msg: RpcMessage, dom: &mut Dom) -> Result<RpcMessage> {
        let instance = dom
            .get_instance(self.id)
            .map(ResponseInstance::from_dom_instance)
            .map(|inst| inst.with_dom_metadata(dom));
        msg.respond()
            .with_data(instance)
            .context("failed to serialize response")
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ChildrenRequest {
    id: Ref,
}

impl ChildrenRequest {
    pub async fn respond_to(self, msg: RpcMessage, dom: &mut Dom) -> Result<RpcMessage> {
        let child_ids = dom
            .get_instance(self.id)
            .map(|inst| inst.children())
            .unwrap_or_default();
        let instances = child_ids
            .iter()
            .filter_map(|id| dom.get_instance(*id))
            .map(ResponseInstance::from_dom_instance)
            .map(|inst| inst.with_dom_metadata(dom))
            .collect::<Vec<_>>();
        msg.respond()
            .with_data(instances)
            .context("failed to serialize response")
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AncestorsRequest {
    id: Ref,
}

impl AncestorsRequest {
    pub async fn respond_to(self, msg: RpcMessage, dom: &mut Dom) -> Result<RpcMessage> {
        let mut current = Some(self.id);
        let mut ancestor_ids = Vec::new();
        while let Some(current_id) = current.take() {
            ancestor_ids.push(current_id);
            let current_instance = match dom.get_instance(current_id) {
                Some(inst) => inst,
                None => continue,
            };
            let parent = current_instance.parent();
            if parent.is_some() {
                current.replace(parent);
            }
        }

        ancestor_ids.reverse(); // Sort top level ancestor first, target instance last

        let instances = ancestor_ids
            .iter()
            .filter_map(|id| dom.get_instance(*id))
            .map(ResponseInstance::from_dom_instance)
            .map(|inst| inst.with_dom_metadata(dom))
            .collect::<Vec<_>>();
        msg.respond()
            .with_data(instances)
            .context("failed to serialize response")
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct FindByPathRequest {
    path: PathBuf,
}

impl FindByPathRequest {
    pub async fn respond_to(self, msg: RpcMessage, dom: &mut Dom) -> Result<RpcMessage> {
        let instance = dom
            .find_by_path(self.path)
            .and_then(|id| dom.get_instance(id))
            .map(ResponseInstance::from_dom_instance)
            .map(|inst| inst.with_dom_metadata(dom));
        msg.respond()
            .with_data(instance)
            .context("failed to serialize response")
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct FindByQueryRequest {
    query: String,
    limit: Option<usize>,
}

impl FindByQueryRequest {
    pub async fn respond_to(self, msg: RpcMessage, dom: &mut Dom) -> Result<RpcMessage> {
        let mut params = DomQueryParams::from_str(&self.query);
        params.limit = self.limit;

        let instances = dom
            .find_by_query(params)
            .iter()
            .filter_map(|id| dom.get_instance(*id))
            .map(ResponseInstance::from_dom_instance)
            .map(|inst| inst.with_dom_metadata(dom))
            .collect::<Vec<_>>();

        msg.respond()
            .with_data(instances)
            .context("failed to serialize response")
    }
}
