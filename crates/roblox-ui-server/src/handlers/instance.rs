use anyhow::{Context, Result};
use rbx_dom_weak::types::Ref;
use serde::Deserialize;
use ustr::Ustr;

use roblox_ui_project::{Command, Delta, Project};

use crate::rpc::RpcMessage;

use super::util::ResponseInstance;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct InsertRequest {
    parent_id: Ref,
    class_name: Ustr,
    name: String,
}

impl InsertRequest {
    pub async fn respond_to(self, msg: RpcMessage, project: &Project) -> Result<RpcMessage> {
        let deltas = project
            .apply(Command::Insert {
                parent: self.parent_id,
                class: self.class_name,
                name: self.name,
            })
            .await
            .unwrap_or_default();

        // The inserted instance shows up as the first Added under the parent.
        let inserted = deltas.iter().find_map(|d| match d {
            Delta::Added { id, parent, .. } if *parent == self.parent_id => Some(*id),
            _ => None,
        });

        let dom = project.read().await;
        let instance = inserted
            .and_then(|id| dom.get_instance(id))
            .map(ResponseInstance::from_dom_instance)
            .map(|inst| inst.with_dom_metadata(&dom));
        msg.respond()
            .with_data(instance)
            .context("failed to serialize response")
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RenameRequest {
    id: Ref,
    name: String,
}

impl RenameRequest {
    pub async fn respond_to(self, msg: RpcMessage, project: &Project) -> Result<RpcMessage> {
        let deltas = project
            .apply(Command::Rename {
                id: self.id,
                name: self.name,
            })
            .await
            .unwrap_or_default();
        let was_renamed = !deltas.is_empty();
        msg.respond()
            .with_data(was_renamed)
            .context("failed to serialize response")
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DeleteRequest {
    id: Ref,
}

impl DeleteRequest {
    pub async fn respond_to(self, msg: RpcMessage, project: &Project) -> Result<RpcMessage> {
        let deltas = project
            .apply(Command::Delete { id: self.id })
            .await
            .unwrap_or_default();
        let was_deleted = deltas
            .iter()
            .any(|d| matches!(d, Delta::Removed { id, .. } if *id == self.id));
        msg.respond()
            .with_data(was_deleted)
            .context("failed to serialize response")
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct MoveRequest {
    id: Ref,
    parent_id: Ref,
}

impl MoveRequest {
    pub async fn respond_to(self, msg: RpcMessage, _project: &Project) -> Result<RpcMessage> {
        // Moving across parents changes a node's provenance key; not yet
        // supported by the filesystem mutation layer.
        let _ = (self.id, self.parent_id);
        msg.respond()
            .with_data(false)
            .context("failed to serialize response")
    }
}
