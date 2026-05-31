use std::{path::PathBuf, sync::Arc};

use anyhow::Result;
use async_channel::{Receiver, Sender};
use async_lock::Mutex as AsyncMutex;
use futures_lite::io::BufReader;
use serde_json::Value as JsonValue;
use tracing::{debug, error};

use roblox_ui_project::{
    AsyncFileCache, AsyncFileEvent, AsyncFileWatcher, Config, Dom, InstanceProvider,
};

use super::{handlers::handle_rpc_message, output::OutputProcessor, rpc::RpcMessage};

type FileEvent = (AsyncFileEvent, PathBuf, Option<String>);

/**
    Emits notifications from an instance dom over stdio when they become available.
*/
pub async fn emit_notifications_dom(
    _config: Config,
    instance_dom: Arc<AsyncMutex<Dom>>,
) -> Result<()> {
    let mut stdout = blocking::Unblock::new(std::io::stdout());

    // Emit an initial 'null' (meaning no instance data) to
    // let the consumer know instance notifications have started
    RpcMessage::new_request("dom/notification")
        .with_data(JsonValue::Null)?
        .write_to(&mut stdout)
        .await?;

    // Take out the notification receiver from the dom
    let notification_receiver = {
        let mut dom = instance_dom.lock().await;
        dom.take_notification_receiver().unwrap()
    };

    // Emit rest of notifications while they keep coming in
    while let Ok(notification) = notification_receiver.recv().await {
        RpcMessage::new_request("dom/notification")
            .with_data(notification)?
            .write_to(&mut stdout)
            .await?;
    }

    Ok(())
}

/**
    Receives instances from an instance provider (receiver) and serves them over stdio.
*/
pub async fn serve_instances(
    _config: Config,
    instance_dom: Arc<AsyncMutex<Dom>>,
    instances: Arc<AsyncMutex<InstanceProvider>>,
) -> Result<()> {
    let stdin = blocking::Unblock::new(std::io::stdin());

    // Spawn a task to listen for requests over stdin
    let stdin_dom = Arc::clone(&instance_dom);
    let stdin_handle = async_global_executor::spawn(async move {
        let mut reader = BufReader::new(stdin);
        while let Some(res) = RpcMessage::read_from(&mut reader).await {
            match res {
                Err(e) => error!("error reading stdio message: {e:?}"),
                Ok(msg) => {
                    debug!("got stdio message: {msg:?}");
                    let mut dom = stdin_dom.lock().await;
                    if let Err(e) = handle_rpc_message(msg, &mut dom).await {
                        error!("failed to respond to message: {e:?}")
                    }
                }
            }
        }
    });

    // Take out the instance receiver from the provider
    let instance_receiver = {
        let mut instances = instances.lock().await;
        instances.take_instance_receiver().unwrap()
    };

    // Watch for further changes received from instance provider(s)
    while let Ok(root_node_opt) = instance_receiver.recv().await {
        let mut dom = instance_dom.lock().await;
        dom.apply_new_root(root_node_opt);
    }

    // Since our stdin task was spawned in the background
    // we must also manually abort it when we are done
    drop(stdin_handle);

    Ok(())
}

/**
    Provides instances based on file events from the given receiver.

    Will process the file events, start/update/stop relevant instance providers,
    use instance provider(s) to process files and subprocesses, and then
    send changes using the sender in the given [`InstanceProvider`].
*/
pub async fn provide_instances(
    config: Config,
    _instance_dom: Arc<AsyncMutex<Dom>>,
    instances: Arc<AsyncMutex<InstanceProvider>>,
    file_event_rx: Receiver<FileEvent>,
) -> Result<()> {
    while let Ok((event, file_path, file_contents)) = file_event_rx.recv().await {
        // TODO: Make the dom aware of this file event, to add it to root metadata (for rojo, wally, ...)

        let res = if config.is_sourcemap_path(&file_path) {
            let mut instances = instances.lock().await;
            instances
                .update_file(file_path.as_path(), file_contents.as_deref())
                .await
        } else if config.is_rojo_project_path(&file_path) {
            let mut instances = instances.lock().await;
            instances
                .update_rojo(file_path.as_path(), file_contents.as_deref())
                .await
        } else {
            Ok(())
        };

        match res {
            Err(e) => error!("{:?} -> {} -> {e:?}", event, file_path.display()),
            Ok(_) => debug!("{:?} -> {}", event, file_path.display()),
        }
    }

    Ok(())
}

/**
    Watches for file changes to files in the given config and emits them using the given sender.
*/
pub async fn watch_files(config: Config, file_event_tx: Sender<FileEvent>) -> Result<()> {
    let paths = config.paths_to_watch();
    let paths = paths.iter().map(|p| p.to_path_buf()).collect::<Vec<_>>();

    // Update all paths once initially
    let mut cache = AsyncFileCache::new();
    for path in &paths {
        if let Some(event) = cache.read_file_at(path).await? {
            file_event_tx.try_send((
                event,
                path.to_path_buf(),
                cache.get_file(path).map(|f| f.to_string()),
            ))?;
        }
    }

    // Watch for further changes to the paths
    let mut watcher = AsyncFileWatcher::new(paths)?;
    while let Some(path) = watcher.recv().await {
        if let Some(event) = cache.read_file_at(&path).await? {
            file_event_tx.try_send((
                event,
                path.to_path_buf(),
                cache.get_file(&path).map(|f| f.to_string()),
            ))?;
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

    // Emit an initial 'null' (meaning no instance data) to
    // let the consumer know instance notifications have started
    RpcMessage::new_request("output/notification")
        .with_data(JsonValue::Null)?
        .write_to(&mut stdout)
        .await?;

    // Take out the notification receiver from the processor
    let notification_receiver = {
        let mut output_processor = output_processor.lock().await;
        output_processor.take_notification_receiver().unwrap()
    };

    // Emit rest of notifications while they keep coming in
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
    output_processor: Arc<AsyncMutex<OutputProcessor>>,
) -> Result<()> {
    // TODO: Implement this

    Ok(())
}
