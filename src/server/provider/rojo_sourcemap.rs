use std::{process::Stdio, time::Duration};

use anyhow::{bail, Context, Result};
use command_group::{AsyncCommandGroup, AsyncGroupChild};
use once_cell::sync::Lazy;
use semver::{Version, VersionReq};
use tracing::{debug, error, trace};

use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, BufReader},
    process::{ChildStderr, ChildStdout, Command},
    sync::mpsc::UnboundedSender,
    task::{self},
    time::sleep,
};

use super::{
    super::config::Config, rojo_stub::generate_project_file_instance_tree, InstanceNode,
    RojoProjectFile,
};

const SPAWN_TIMEOUT: Duration = Duration::from_secs(5);
static REQUIRED_VERSION: Lazy<VersionReq> = Lazy::new(|| VersionReq::parse("7.3.0").unwrap());

/**
    An instance provider that uses a rojo project
    file and `rojo sourcemap --watch` to emit diffs.
*/
#[derive(Debug)]
pub struct RojoSourcemapProvider {
    config: Config,
    sender: UnboundedSender<Option<InstanceNode>>,
    version: Option<Version>,
    child: Option<AsyncGroupChild>,
}

impl RojoSourcemapProvider {
    pub fn new(config: Config, sender: UnboundedSender<Option<InstanceNode>>) -> Self {
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
        let version = tokio::select! {
            v = get_rojo_version() => v?,
            _ = sleep(SPAWN_TIMEOUT) => bail!("rojo --version timed out"),
        };
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
            self.sender.send(tree.clone()).ok();
            tree
        } else {
            self.sender.send(None).ok();
            None
        };

        // Grab the output streams to process sourcemaps, and store the
        // child process in our struct so it doesn't drop and get killed
        let stdout = child.inner().stdout.take().unwrap();
        let stderr = child.inner().stderr.take().unwrap();
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
            child.kill().await?;
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
        .group_spawn()
        .context("failed to execute rojo --version")?;

    let output = child
        .wait_with_output()
        .await
        .context("failed to wait on rojo --version")?;

    let version_string = String::from_utf8(output.stdout)
        .context("failed to parse rojo --version output into string")?;

    version_string
        .split_whitespace()
        .find_map(|word| Version::parse(word).ok())
        .context("failed to parse rojo --version output")
}

fn spawn_rojo_sourcemap(config: &Config) -> Result<AsyncGroupChild> {
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
        .group_spawn()
        .context("failed to spawn rojo sourcemap --watch")
}

fn handle_rojo_streams(
    stdout: ChildStdout,
    stderr: ChildStderr,
    sender: UnboundedSender<Option<InstanceNode>>,
    tree_stub: Option<InstanceNode>,
) {
    // Note that we don't really need to care about the join handles
    // for our tasks here, they will exit when the rojo process dies

    task::spawn(async move {
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
                    sender.send(Some(smap)).ok();
                }
            }
            buffer.clear();
        }
    });

    task::spawn(async move {
        let mut reader = BufReader::new(stderr);
        let mut buffer = String::new();
        while reader.read_to_string(&mut buffer).await.unwrap() > 0 {
            error!("rojo error: {buffer}");
            buffer.clear();
        }
    });
}
