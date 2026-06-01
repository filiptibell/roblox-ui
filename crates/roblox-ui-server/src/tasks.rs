use std::sync::Arc;

use anyhow::Result;
use async_channel::Receiver;
use async_lock::Mutex as AsyncMutex;
use futures_lite::io::BufReader;
use serde_json::Value as JsonValue;
use tracing::{debug, error};

use roblox_ui_project::{Config, Delta, Project};

use super::{
    handlers::handle_rpc_message, notification::delta_to_notification, output::OutputProcessor,
    rpc::RpcMessage,
};

/**
    Emits DOM notifications over stdio as deltas arrive from the project.

    The delta receiver must be obtained *before* the project's initial sync so
    no transactions are missed; we additionally synthesize an initial `Added`
    for the already-present root so a freshly-connected consumer can fetch it.
*/
pub async fn emit_notifications_dom(
    project: Arc<Project>,
    delta_rx: Receiver<Vec<Delta>>,
) -> Result<()> {
    let mut stdout = blocking::Unblock::new(std::io::stdout());

    // Initial 'null' tells the consumer notifications have started.
    RpcMessage::new_request("dom/notification")
        .with_data(JsonValue::Null)?
        .write_to(&mut stdout)
        .await?;

    // Replay the current root (if any) as an Added so the consumer can fetch.
    if let Some(root) = project.read().await.get_root_id() {
        let notification = roblox_ui_project_added_root(root);
        RpcMessage::new_request("dom/notification")
            .with_data(notification)?
            .write_to(&mut stdout)
            .await?;
    }

    while let Ok(deltas) = delta_rx.recv().await {
        for delta in &deltas {
            let notification = delta_to_notification(delta);
            RpcMessage::new_request("dom/notification")
                .with_data(notification)?
                .write_to(&mut stdout)
                .await?;
        }
    }

    Ok(())
}

fn roblox_ui_project_added_root(
    root: rbx_dom_weak::types::Ref,
) -> super::notification::DomNotification {
    super::notification::DomNotification::Added {
        parent_id: None,
        child_id: root,
    }
}

/**
    Serves requests received over stdin against the project.
*/
pub async fn serve_instances(project: Arc<Project>) -> Result<()> {
    let stdin = blocking::Unblock::new(std::io::stdin());
    let mut reader = BufReader::new(stdin);
    while let Some(res) = RpcMessage::read_from(&mut reader).await {
        match res {
            Err(e) => error!("error reading stdio message: {e:?}"),
            Ok(msg) => {
                debug!("got stdio message: {msg:?}");
                if let Err(e) = handle_rpc_message(msg, &project).await {
                    error!("failed to respond to message: {e:?}")
                }
            }
        }
    }
    Ok(())
}

/**
    Emits notifications from an output processor over stdio when they become available.
*/
pub async fn emit_notifications_output(
    _config: Config,
    output_processor: Arc<AsyncMutex<OutputProcessor>>,
) -> Result<()> {
    let mut stdout = blocking::Unblock::new(std::io::stdout());

    RpcMessage::new_request("output/notification")
        .with_data(JsonValue::Null)?
        .write_to(&mut stdout)
        .await?;

    let notification_receiver = {
        let mut output_processor = output_processor.lock().await;
        output_processor.take_notification_receiver().unwrap()
    };

    while let Ok(notification) = notification_receiver.recv().await {
        RpcMessage::new_request("output/notification")
            .with_data(notification)?
            .write_to(&mut stdout)
            .await?;
    }

    Ok(())
}

/**
    Starts the plugin server for the output processor.
*/
pub async fn connect_notifications_plugin(
    _config: Config,
    _output_processor: Arc<AsyncMutex<OutputProcessor>>,
) -> Result<()> {
    // TODO: Implement this
    Ok(())
}
