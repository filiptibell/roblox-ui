mod classes;
mod cli;
mod icons;
mod reflection;
mod server;
mod util;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build_global()
        .expect("failed to create thread pool");
    cli::Cli::new().run().await
}
