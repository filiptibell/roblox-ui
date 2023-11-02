use anyhow::Result;

mod transport;

pub use transport::*;

#[derive(Debug, Clone)]
pub struct WatcherArguments {
    pub transport: Transport,
}

pub struct Watcher {
    args: WatcherArguments,
}

impl Watcher {
    pub fn new(args: WatcherArguments) -> Self {
        Self { args }
    }

    pub async fn watch(self) -> Result<()> {
        match self.args.transport {
            Transport::Socket(port) => {
                let (_read, _write) = Transport::create_socket(port).await;
                // TODO: Start watching
            }
            Transport::Stdio => {
                let (_stdin, _stdout) = Transport::create_stdio();
                // TODO: Start watching
            }
        }

        Ok(())
    }
}
