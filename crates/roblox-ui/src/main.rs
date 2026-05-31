mod cli;

fn main() -> anyhow::Result<()> {
    async_global_executor::block_on(cli::Cli::new().run())
}
