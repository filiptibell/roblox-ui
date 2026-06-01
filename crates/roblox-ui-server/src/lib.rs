use std::sync::Arc;

use anyhow::Result;
use async_lock::Mutex as AsyncMutex;
use futures_lite::future::try_zip;

use roblox_ui_project::Project;

pub use roblox_ui_project::Config;

mod handlers;
mod notification;
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
        // Open the project; this performs the initial sync AND starts the
        // background file watcher transparently. We subscribe right after so the
        // emit task receives every subsequent transaction's deltas.
        let project = Arc::new(Project::open(self.config.clone()).await?);
        let delta_rx = project.subscribe().await;

        // Emit notifications + serve stdin requests concurrently. The project's
        // watcher runs in the background (owned by `project`); a fatal error in
        // either task bubbles up and cancels the other.
        try_zip(
            tasks::emit_notifications_dom(Arc::clone(&project), delta_rx),
            tasks::serve_instances(Arc::clone(&project)),
        )
        .await?;

        Ok(())
    }

    pub async fn serve_output(self) -> Result<()> {
        let output_processor = output::OutputProcessor::new(self.config.clone());
        let output_processor = Arc::new(AsyncMutex::new(output_processor));

        try_zip(
            tasks::emit_notifications_output(self.config.clone(), Arc::clone(&output_processor)),
            tasks::connect_notifications_plugin(self.config.clone(), Arc::clone(&output_processor)),
        )
        .await?;

        Ok(())
    }
}
