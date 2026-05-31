use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::{
    sync::{mpsc::unbounded_channel, Mutex as AsyncMutex},
    task::JoinSet,
};

mod handlers;
mod output;
mod rpc;
mod tasks;

use roblox_ui_project::{Dom, InstanceProvider};

pub use roblox_ui_project::Config;

pub struct Server {
    config: Config,
}

impl Server {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub async fn serve_instances(self) -> Result<()> {
        let (file_event_tx, file_event_rx) = unbounded_channel();

        let instance_dom = Dom::new();
        let instance_dom = Arc::new(AsyncMutex::new(instance_dom));

        let instance_provider = InstanceProvider::new(self.config.clone());
        let instance_provider = Arc::new(AsyncMutex::new(instance_provider));

        // Spawn all of our tasks: watch files -> provide instances -> serve instances -> emit notifications
        // These all depend on each other and pass messages upstream, so we spawn them in reverse order
        let mut set = JoinSet::new();
        set.spawn(tasks::emit_notifications_dom(
            self.config.clone(),
            Arc::clone(&instance_dom),
        ));
        set.spawn(tasks::serve_instances(
            self.config.clone(),
            Arc::clone(&instance_dom),
            Arc::clone(&instance_provider),
        ));
        set.spawn(tasks::provide_instances(
            self.config.clone(),
            Arc::clone(&instance_dom),
            Arc::clone(&instance_provider),
            file_event_rx,
        ));
        set.spawn(tasks::watch_files(self.config.clone(), file_event_tx));

        // Whenever a task errors fatally, we should bubble that up, which
        // will drop our JoinSet and cancel all of our other tasks as well
        while let Some(res) = set.join_next().await {
            res.context("failed to join task")?
                .context("task errored")?;
        }

        Ok(())
    }

    pub async fn serve_output(self) -> Result<()> {
        let output_processor = output::OutputProcessor::new(self.config.clone());
        let output_processor = Arc::new(AsyncMutex::new(output_processor));

        // Spawn all of our tasks: start server for plugin -> emit notifications
        // These all depend on each other and pass messages upstream, so we spawn them in reverse order
        let mut set = JoinSet::new();
        set.spawn(tasks::emit_notifications_output(
            self.config.clone(),
            Arc::clone(&output_processor),
        ));
        set.spawn(tasks::connect_notifications_plugin(
            self.config.clone(),
            Arc::clone(&output_processor),
        ));

        // Whenever a task errors fatally, we should bubble that up, which
        // will drop our JoinSet and cancel all of our other tasks as well
        while let Some(res) = set.join_next().await {
            res.context("failed to join task")?
                .context("task errored")?;
        }

        Ok(())
    }
}
