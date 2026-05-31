use std::sync::Arc;

use anyhow::Result;
use async_channel::unbounded;
use async_lock::Mutex as AsyncMutex;
use futures_lite::future::try_zip;

use roblox_ui_project::{Dom, InstanceProvider};

pub use roblox_ui_project::Config;

mod handlers;
mod output;
mod rpc;
mod tasks;

pub struct Server {
    config: Config,
}

impl Server {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub async fn serve_instances(self) -> Result<()> {
        let (file_event_tx, file_event_rx) = unbounded();

        let instance_dom = Dom::new();
        let instance_dom = Arc::new(AsyncMutex::new(instance_dom));

        let instance_provider = InstanceProvider::new(self.config.clone());
        let instance_provider = Arc::new(AsyncMutex::new(instance_provider));

        // Run all of our tasks concurrently: watch files -> provide instances
        // -> serve instances -> emit notifications. They depend on each other
        // and pass messages upstream. Whenever a task errors fatally we bubble
        // that up, which drops (and thus cancels) all of the other tasks too.
        try_zip(
            try_zip(
                tasks::emit_notifications_dom(self.config.clone(), Arc::clone(&instance_dom)),
                tasks::serve_instances(
                    self.config.clone(),
                    Arc::clone(&instance_dom),
                    Arc::clone(&instance_provider),
                ),
            ),
            try_zip(
                tasks::provide_instances(
                    self.config.clone(),
                    Arc::clone(&instance_dom),
                    Arc::clone(&instance_provider),
                    file_event_rx,
                ),
                tasks::watch_files(self.config.clone(), file_event_tx),
            ),
        )
        .await?;

        Ok(())
    }

    pub async fn serve_output(self) -> Result<()> {
        let output_processor = output::OutputProcessor::new(self.config.clone());
        let output_processor = Arc::new(AsyncMutex::new(output_processor));

        // Run our tasks concurrently: start server for plugin -> emit
        // notifications. A fatal error in either cancels the other.
        try_zip(
            tasks::emit_notifications_output(self.config.clone(), Arc::clone(&output_processor)),
            tasks::connect_notifications_plugin(self.config.clone(), Arc::clone(&output_processor)),
        )
        .await?;

        Ok(())
    }
}
