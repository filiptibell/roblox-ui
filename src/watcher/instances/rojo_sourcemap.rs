use std::{process::Stdio, time::Duration};

use anyhow::{bail, Context, Result};
use once_cell::sync::Lazy;
use semver::{Version, VersionReq};
use tracing::{debug, error, trace};

use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, BufReader},
    process::{Child, ChildStderr, ChildStdout, Command},
    task::{self},
    time::sleep,
};

use super::*;

const SPAWN_TIMEOUT: Duration = Duration::from_secs(5);
static REQUIRED_VERSION: Lazy<VersionReq> = Lazy::new(|| VersionReq::parse("7.3.0").unwrap());

/**
    An instance provider that uses a rojo project
    file and `rojo sourcemap --watch` to emit diffs.
*/
#[derive(Debug, Default)]
pub struct RojoSourcemapProvider {
    settings: Settings,
    version: Option<Version>,
    child: Option<Child>,
}

impl RojoSourcemapProvider {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings,
            version: None,
            child: None,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
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
        let mut child = spawn_rojo_sourcemap(&self.settings)?;

        // Emit an initial single "null" to let any consumer know watching started
        println!("null");

        // Grab the output streams to process sourcemaps, and store the
        // child process in our struct so it doesn't drop and get killed
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        handle_rojo_streams(stdout, stderr);
        self.child.replace(child);

        Ok(())
    }

    pub async fn update(&mut self) -> Result<()> {
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
    let version_bytes = Command::new("rojo")
        .arg("--version")
        .kill_on_drop(true)
        .output()
        .await
        .context("failed to spawn rojo --version")?
        .stdout;

    let version_string = String::from_utf8(version_bytes)
        .context("failed to parse rojo --version output into string")?;

    version_string
        .split_whitespace()
        .find_map(|word| Version::parse(word).ok())
        .context("failed to parse rojo --version output")
}

fn spawn_rojo_sourcemap(settings: &Settings) -> Result<Child> {
    assert!(
        settings.autogenerate,
        "autogenerate must be enabled to spawn rojo sourcemap --watch"
    );

    let mut command = Command::new("rojo");
    command
        .arg("sourcemap")
        .arg(&settings.rojo_project_file)
        .arg("--watch")
        .kill_on_drop(true)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if settings.include_non_scripts {
        command.arg("--include-non-scripts");
    }

    command
        .spawn()
        .context("failed to spawn rojo sourcemap --watch")
}

fn handle_rojo_streams(stdout: ChildStdout, stderr: ChildStderr) {
    // Note that we don' really need to care about the join handles
    // for our tasks here, they will exit when the rojo process dies

    task::spawn(async move {
        let mut current = None::<InstanceNode>;
        let mut reader = BufReader::new(stdout);
        let mut buffer = String::new();
        while reader.read_line(&mut buffer).await.unwrap() > 0 {
            trace!("got sourcemap with {} characters", buffer.len());
            match InstanceNode::from_json(&buffer) {
                Err(e) => error!("failed to deserialize rojo sourcemap: {e}"),
                Ok(smap) => {
                    match current.take() {
                        None => println!("{}", smap.diff_full()),
                        Some(old) => println!("{}", old.diff_with(&smap)),
                    }
                    current.replace(smap);
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
