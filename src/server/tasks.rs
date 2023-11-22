use std::{path::PathBuf, sync::Arc};

use anyhow::Result;
use serde_json::Value as JsonValue;
use tokio::{
    io::BufReader,
    sync::{
        mpsc::{UnboundedReceiver, UnboundedSender},
        Mutex as AsyncMutex,
    },
};
use tracing::{debug, error};

use super::{
    config::Config,
    dom::Dom,
    handlers::handle_rpc_message,
    notify::{AsyncFileCache, AsyncFileEvent, AsyncFileWatcher},
    provider::InstanceProvider,
    rpc::RpcMessage,
};

type FileEvent = (AsyncFileEvent, PathBuf, Option<String>);

/**
    Receives instances from an instance provider (receiver) and serves them over stdio.
*/
pub async fn serve_instances(
    _config: Config,
    instance_dom: Arc<AsyncMutex<Dom>>,
    instances: Arc<AsyncMutex<InstanceProvider>>,
) -> Result<()> {
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();

    // Spawn a task to listen for requests over stdin
    let stdin_dom = Arc::clone(&instance_dom);
    let stdin_handle = tokio::spawn(async move {
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
    let mut instance_receiver = {
        let mut instances = instances.lock().await;
        instances.take_instance_receiver().unwrap()
    };

    // Emit an initial 'null' (meaning no instance data) to
    // let the consumer know instance watching has started
    RpcMessage::new_request("dom/notification")
        .with_data(JsonValue::Null)?
        .write_to(&mut stdout)
        .await?;

    // Watch for further changes received from instance provider(s)
    while let Some(root_node_opt) = instance_receiver.recv().await {
        let mut dom = instance_dom.lock().await;
        for notification in dom.apply_new_root(root_node_opt) {
            RpcMessage::new_request("dom/notification")
                .with_data(notification)?
                .write_to(&mut stdout)
                .await?;
        }
    }

    // Since our stdin task was spawned in the background
    // we must also manually abort it when we are done
    stdin_handle.abort();

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
    mut file_event_rx: UnboundedReceiver<FileEvent>,
) -> Result<()> {
    while let Some((event, file_path, file_contents)) = file_event_rx.recv().await {
        // TODO: Make the dom aware of this file event, to add it to root metadata (for rojo, wally, ...)

        let res = if config.is_sourcemap_path(&file_path) {
            let mut instances = instances.lock().await;
            instances.update_file(file_contents.as_deref()).await
        } else if config.is_rojo_project_path(&file_path) {
            let mut instances = instances.lock().await;
            instances.update_rojo(file_contents.as_deref()).await
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
pub async fn watch_files(config: Config, file_event_tx: UnboundedSender<FileEvent>) -> Result<()> {
    let paths = config.paths_to_watch();
    let paths = paths.iter().map(|p| p.to_path_buf()).collect::<Vec<_>>();

    // Update all paths once initially
    let mut cache = AsyncFileCache::new();
    for path in &paths {
        if let Some(event) = cache.read_file_at(path).await? {
            file_event_tx.send((
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
            file_event_tx.send((
                event,
                path.to_path_buf(),
                cache.get_file(&path).map(|f| f.to_string()),
            ))?;
        }
    }

    Ok(())
}
