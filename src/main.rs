mod classes;
mod cli;
mod icons;
mod reflection;
mod util;
mod watcher;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    cli::Cli::new().run().await
}
