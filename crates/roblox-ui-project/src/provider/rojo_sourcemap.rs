use std::{process::Stdio, sync::LazyLock, time::Duration};

use anyhow::{bail, Context, Result};
use async_channel::Sender;
use async_io::Timer;
use async_process::{Child, ChildStderr, ChildStdout, Command};
use futures_lite::{
    future,
    io::{AsyncBufReadExt, AsyncReadExt, BufReader},
};
use semver::{Version, VersionReq};
use tracing::{debug, error, trace};

use super::{
    super::config::Config, rojo_stub::generate_project_file_instance_tree, InstanceNode,
    RojoProjectFile,
};

const SPAWN_TIMEOUT: Duration = Duration::from_secs(5);
static REQUIRED_VERSION: LazyLock<VersionReq> =
    LazyLock::new(|| VersionReq::parse("7.3.0").unwrap());

/**
    An instance provider that uses a rojo project
    file and `rojo sourcemap --watch` to emit diffs.
*/
#[derive(Debug)]
pub struct RojoSourcemapProvider {
    config: Config,
    sender: Sender<Option<InstanceNode>>,
    version: Option<Version>,
    child: Option<Child>,
}

impl RojoSourcemapProvider {
    pub fn new(config: Config, sender: Sender<Option<InstanceNode>>) -> Self {
        Self {
            config,
            sender,
            version: None,
            child: None,
        }
    }

    pub async fn start(&mut self, project_file: Option<&RojoProjectFile>) -> Result<()> {
        trace!("starting rojo provider");

        // Spawn rojo to figure out what version
        // it has and if it meets our requirement
        let version = future::or(get_rojo_version(), async {
            Timer::after(SPAWN_TIMEOUT).await;
            bail!("rojo --version timed out")
        })
        .await?;
        debug!("found rojo version: {}", version);
        // HACK: Get rid of prerelease for version req,
        // having a prerelease makes it not match :-(
        let version = Version::new(version.major, version.minor, version.patch);
        if !REQUIRED_VERSION.matches(&version) {
            bail!(
                "installed rojo version does not meet the {} requirement",
                *REQUIRED_VERSION
            );
        }
        self.version.replace(version);

        // Spawn the sourcemap watching command, which
        // should not fail if our version check was correct
        let mut child = spawn_rojo_sourcemap(&self.config)?;

        // Emit an initial instance tree to let any consumer know watching started,
        // we will try our best to construct a top-level tree stub here using only
        // the rojo project file and parsing its 'tree' field, but this may fail
        let tree_stub = if let Some(project_file) = project_file {
            let tree = generate_project_file_instance_tree(project_file).await;
            self.sender.try_send(tree.clone()).ok();
            tree
        } else {
            self.sender.try_send(None).ok();
            None
        };

        // Grab the output streams to process sourcemaps, and store the
        // child process in our struct so it doesn't drop and get killed
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        handle_rojo_streams(stdout, stderr, self.sender.clone(), tree_stub);
        self.child.replace(child);

        Ok(())
    }

    pub async fn update(&mut self, _project_file: Option<&RojoProjectFile>) -> Result<()> {
        trace!("updating rojo provider");
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        trace!("stopping rojo provider");
        self.version.take();
        if let Some(mut child) = self.child.take() {
            child.kill()?;
        }
        Ok(())
    }
}

async fn get_rojo_version() -> Result<Version> {
    let child = Command::new("rojo")
        .arg("--version")
        .kill_on_drop(true)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to execute rojo --version")?;

    let output = child
        .output()
        .await
        .context("failed to wait on rojo --version")?;

    let version_string = String::from_utf8(output.stdout)
        .context("failed to parse rojo --version output into string")?;

    version_string
        .split_whitespace()
        .find_map(|word| Version::parse(word).ok())
        .context("failed to parse rojo --version output")
}

fn spawn_rojo_sourcemap(config: &Config) -> Result<Child> {
    assert!(
        config.autogenerate,
        "autogenerate must be enabled to spawn rojo sourcemap --watch"
    );

    Command::new("rojo")
        .arg("sourcemap")
        .arg(&config.rojo_project_file)
        .arg("--watch")
        .arg("--include-non-scripts")
        .kill_on_drop(true)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to spawn rojo sourcemap --watch")
}

fn handle_rojo_streams(
    stdout: ChildStdout,
    stderr: ChildStderr,
    sender: Sender<Option<InstanceNode>>,
    tree_stub: Option<InstanceNode>,
) {
    // Note that we don't really need to care about the task handles here,
    // they will exit when the rojo process dies and its streams close

    async_global_executor::spawn(async move {
        let mut reader = BufReader::new(stdout);
        let mut buffer = String::new();
        while reader.read_line(&mut buffer).await.unwrap() > 0 {
            trace!("got sourcemap with {} characters", buffer.len());
            match InstanceNode::from_json(&buffer) {
                Err(e) => error!("failed to deserialize rojo sourcemap: {e}"),
                Ok(mut smap) => {
                    if let Some(stub) = &tree_stub {
                        smap.merge_stub(stub);
                    }
                    sender.try_send(Some(smap)).ok();
                }
            }
            buffer.clear();
        }
    })
    .detach();

    async_global_executor::spawn(async move {
        let mut reader = BufReader::new(stderr);
        let mut buffer = String::new();
        while reader.read_to_string(&mut buffer).await.unwrap() > 0 {
            error!("rojo error: {buffer}");
            buffer.clear();
        }
    })
    .detach();
}
