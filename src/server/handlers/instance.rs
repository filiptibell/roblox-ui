use anyhow::{Context, Result};
use rbx_dom_weak::types::Ref;
use serde::Deserialize;

use crate::server::{dom::Dom, rpc::RpcMessage};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct InsertRequest {
    parent_id: Ref,
    class_name: String,
    name: String,
}

impl InsertRequest {
    pub async fn respond_to(self, msg: RpcMessage, dom: &mut Dom) -> Result<RpcMessage> {
        let inserted_instance_id_opt = dom
            .insert_instance(self.parent_id, self.class_name, self.name)
            .await;
        msg.respond()
            .with_data(inserted_instance_id_opt)
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
    pub async fn respond_to(self, msg: RpcMessage, dom: &mut Dom) -> Result<RpcMessage> {
        let was_renamed = dom.rename_instance(self.id, self.name).await;
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
    pub async fn respond_to(self, msg: RpcMessage, dom: &mut Dom) -> Result<RpcMessage> {
        let was_deleted = dom.delete_instance(self.id).await;
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
    pub async fn respond_to(self, msg: RpcMessage, dom: &mut Dom) -> Result<RpcMessage> {
        let was_deleted = dom.move_instance(self.id, self.parent_id).await;
        msg.respond()
            .with_data(was_deleted)
            .context("failed to serialize response")
    }
}
