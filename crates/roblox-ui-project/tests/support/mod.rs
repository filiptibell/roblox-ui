/*!
    Shared test scaffolding: a `tempfile`-backed fixture builder plus helpers for
    opening a `Project`, navigating the DOM, reading typed properties, and
    draining the delta stream. Every test regenerates its fixtures from scratch
    in a throwaway temp dir, mirroring Rojo's `rojo-test` approach.
*/
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use async_channel::Receiver;
use async_io::Timer;
use futures_lite::future::{block_on, or};
use rbx_dom_weak::{
    types::{Ref, Variant},
    Ustr, WeakDom,
};
use tempfile::TempDir;

use roblox_ui_project::{Config, Delta, Dom, Project};

/**
    A throwaway project on disk. Files are written eagerly; the temp dir is
    removed when the `Fixture` is dropped at the end of the test.
*/
pub struct Fixture {
    tmp: TempDir,
}

pub fn fixture() -> Fixture {
    Fixture {
        tmp: TempDir::new().expect("create temp dir"),
    }
}

impl Fixture {
    /**
        Write the `default.project.json` for this fixture.
    */
    pub fn project(&self, json: &str) -> &Self {
        self.file("default.project.json", json)
    }

    /**
        Write a project file with a custom filename (for nested-project tests).
    */
    pub fn project_named(&self, rel: &str, json: &str) -> &Self {
        self.file(rel, json)
    }

    /**
        Write a text file at `rel`, creating parent directories.
    */
    pub fn file(&self, rel: &str, contents: &str) -> &Self {
        let path = self.path(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, contents).unwrap();
        self
    }

    /**
        Serialize a `WeakDom` to an `.rbxmx` at `rel` (round-trip fidelity).
    */
    pub fn rbxmx(&self, rel: &str, dom: &WeakDom) -> &Self {
        let path = self.path(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut bytes = Vec::new();
        rbx_xml::to_writer_default(&mut bytes, dom, &[dom.root_ref()]).unwrap();
        fs::write(&path, bytes).unwrap();
        self
    }

    /**
        Serialize a `WeakDom` to an `.rbxm` (binary) at `rel`.
    */
    pub fn rbxm(&self, rel: &str, dom: &WeakDom) -> &Self {
        let path = self.path(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut bytes = Vec::new();
        rbx_binary::to_writer(&mut bytes, dom, &[dom.root_ref()]).unwrap();
        fs::write(&path, bytes).unwrap();
        self
    }

    /**
        Create an (empty) directory at `rel`.
    */
    pub fn dir(&self, rel: &str) -> &Self {
        fs::create_dir_all(self.path(rel)).unwrap();
        self
    }

    /**
        Remove a file or directory at `rel`.
    */
    pub fn remove(&self, rel: &str) -> &Self {
        let path = self.path(rel);
        if path.is_dir() {
            fs::remove_dir_all(&path).unwrap();
        } else {
            let _ = fs::remove_file(&path);
        }
        self
    }

    pub fn root_path(&self) -> &Path {
        self.tmp.path()
    }

    pub fn path(&self, rel: &str) -> PathBuf {
        self.tmp.path().join(rel)
    }

    pub fn project_file(&self) -> PathBuf {
        self.path("default.project.json")
    }

    fn config(&self) -> Config {
        Config {
            project_file: self.project_file(),
            ignore_globs: Vec::new(),
        }
    }

    /**
        Open without the background watcher (deterministic; drive `sync_path`).
    */
    pub fn open(&self) -> Project {
        block_on(Project::open_unwatched(self.config())).expect("open project")
    }

    /**
        Open *with* the transparent background watcher.
    */
    pub fn open_watched(&self) -> Project {
        block_on(Project::open(self.config())).expect("open watched project")
    }
}

// ---- async helpers --------------------------------------------------------

pub fn run<F: std::future::Future>(fut: F) -> F::Output {
    block_on(fut)
}

/**
    Drain all immediately-available delta batches, flattened.
*/
pub fn drain(rx: &Receiver<Vec<Delta>>) -> Vec<Delta> {
    let mut all = Vec::new();
    while let Ok(batch) = rx.try_recv() {
        all.extend(batch);
    }
    all
}

/**
    Await one delta batch, up to `dur`. `None` on timeout.
*/
pub fn recv_timeout(rx: &Receiver<Vec<Delta>>, dur: Duration) -> Option<Vec<Delta>> {
    block_on(or(async { rx.recv().await.ok() }, async {
        Timer::after(dur).await;
        None
    }))
}

/**
    Collect deltas until `pred` matches one or the overall deadline elapses.
*/
pub fn collect_until(
    rx: &Receiver<Vec<Delta>>,
    timeout: Duration,
    mut pred: impl FnMut(&Delta) -> bool,
) -> Vec<Delta> {
    let deadline = std::time::Instant::now() + timeout;
    let mut all = Vec::new();
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match recv_timeout(rx, remaining) {
            Some(batch) => {
                let hit = batch.iter().any(&mut pred);
                all.extend(batch);
                if hit {
                    break;
                }
            }
            None => break,
        }
    }
    all
}

// ---- DOM navigation -------------------------------------------------------

pub fn root(dom: &Dom) -> Ref {
    dom.get_root_id().expect("project has a root")
}

pub fn try_child(dom: &Dom, parent: Ref, name: &str) -> Option<Ref> {
    dom.children(parent)
        .iter()
        .copied()
        .find(|id| dom.get_instance(*id).map(|i| i.name.as_str()) == Some(name))
}

pub fn child(dom: &Dom, parent: Ref, name: &str) -> Ref {
    try_child(dom, parent, name).unwrap_or_else(|| panic!("missing child {name:?}"))
}

/**
    Descend from the root following a path of child names.
*/
pub fn descend(dom: &Dom, names: &[&str]) -> Ref {
    let mut current = root(dom);
    for name in names {
        current = child(dom, current, name);
    }
    current
}

pub fn class_of(dom: &Dom, id: Ref) -> String {
    dom.get_instance(id).unwrap().class.to_string()
}

pub fn prop(dom: &Dom, id: Ref, name: &str) -> Option<Variant> {
    dom.get_properties(id)
        .and_then(|p| p.get(&Ustr::from(name)).cloned())
}
