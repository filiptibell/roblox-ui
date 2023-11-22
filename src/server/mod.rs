use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::{
    sync::{mpsc::unbounded_channel, Mutex as AsyncMutex},
    task::JoinSet,
};

mod config;
mod dom;
mod handlers;
mod notify;
mod provider;
mod rpc;
mod tasks;

pub use config::*;

pub struct Server {
    config: Config,
}

impl Server {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub async fn serve(self) -> Result<()> {
        let (file_event_tx, file_event_rx) = unbounded_channel();

        let instance_dom = dom::Dom::new(self.config.clone());
        let instance_dom = Arc::new(AsyncMutex::new(instance_dom));

        let instance_provider = provider::InstanceProvider::new(self.config.clone());
        let instance_provider = Arc::new(AsyncMutex::new(instance_provider));

        // Spawn all of our tasks: watch files -> provide instances -> serve instances
        let mut set = JoinSet::new();
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
}
