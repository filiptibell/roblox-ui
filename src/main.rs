mod classes;
mod cli;
mod icons;
mod reflection;
mod server;
mod util;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    cli::Cli::new().run().await
}
