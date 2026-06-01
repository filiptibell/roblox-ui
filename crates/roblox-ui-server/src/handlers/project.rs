use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Deserialize;

use roblox_ui_project::Project;

use crate::rpc::RpcMessage;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SetFileRequest {
    /**
        Path to the project file to switch to (absolute, or relative to the
        current project file's directory — e.g. `"build.project.json"`).
    */
    path: PathBuf,
}

impl SetFileRequest {
    pub async fn respond_to(self, msg: RpcMessage, project: &Project) -> Result<RpcMessage> {
        // The reconcile is incremental and Ref-stable; its deltas are broadcast
        // to subscribers, so the explorer updates via the normal notification
        // stream. We just report the now-active project file back to the caller.
        project.set_project_file(self.path).await;
        let current = project.project_file().await;
        msg.respond()
            .with_data(current.to_string_lossy().to_string())
            .context("failed to serialize response")
    }
}
