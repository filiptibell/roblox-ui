/*!
    Transparent watching: changes made by a **separate OS process** must surface
    as deltas automatically, with the consumer only calling `subscribe()`. This
    is the core "just works for external changes" guarantee.
*/

use std::process::Command as OsCommand;
use std::thread::sleep;
use std::time::Duration;

use roblox_ui_project::Delta;

mod support;

use support::*;

fn game_project() -> &'static str {
    r#"{ "name": "Watch", "tree": { "$className": "DataModel",
         "ReplicatedStorage": { "$path": "src" } } }"#
}

/**
    Run a shell command that mutates the filesystem from *outside* the process.
*/
fn external(cmd: &str) {
    let status = OsCommand::new("sh")
        .arg("-c")
        .arg(cmd)
        .status()
        .expect("spawn shell");
    assert!(status.success(), "external command failed: {cmd}");
}

const TIMEOUT: Duration = Duration::from_secs(10);

/**
    Give the background watcher a moment to register with the OS before we poke
    the filesystem (notify sets up its backend on an executor thread).
*/
const WATCH_WARMUP: Duration = Duration::from_millis(600);

#[test]
fn external_modify_is_observed() {
    let fx = fixture();
    fx.project(game_project())
        .file("src/Live.luau", "return 1\n");
    let project = fx.open_watched();

    run(async {
        let live = {
            let dom = project.read().await;
            child(&dom, descend(&dom, &["ReplicatedStorage"]), "Live")
        };
        let rx = project.subscribe().await;
        sleep(WATCH_WARMUP);

        // Edit the file from a completely separate process — no sync_path call.
        let path = fx.path("src/Live.luau");
        external(&format!("printf 'return 42\\n' > '{}'", path.display()));

        let deltas = collect_until(
            &rx,
            TIMEOUT,
            |d| matches!(d, Delta::PropertiesChanged { id, .. } if *id == live),
        );
        assert!(
            deltas
                .iter()
                .any(|d| matches!(d, Delta::PropertiesChanged { id, .. } if *id == live)),
            "external modify should surface a PropertiesChanged on the same Ref; got {deltas:?}"
        );

        let dom = project.read().await;
        assert_eq!(
            prop(&dom, live, "Source"),
            Some(rbx_dom_weak::types::Variant::String(
                "return 42\n".to_string()
            ))
        );
    });
}

#[test]
fn external_create_is_observed() {
    let fx = fixture();
    fx.project(game_project())
        .file("src/Existing.luau", "return 1\n");
    let project = fx.open_watched();

    run(async {
        let rx = project.subscribe().await;
        sleep(WATCH_WARMUP);

        let path = fx.path("src/Created.luau");
        external(&format!("printf 'return 2\\n' > '{}'", path.display()));

        let deltas = collect_until(
            &rx,
            TIMEOUT,
            |d| matches!(d, Delta::Added { name, .. } if name == "Created"),
        );
        assert!(
            deltas
                .iter()
                .any(|d| matches!(d, Delta::Added { name, .. } if name == "Created")),
            "external create should surface an Added; got {deltas:?}"
        );
    });
}

#[test]
fn external_delete_is_observed() {
    let fx = fixture();
    fx.project(game_project())
        .file("src/Doomed.luau", "return 1\n")
        .file("src/Keep.luau", "return 2\n");
    let project = fx.open_watched();

    run(async {
        let doomed = {
            let dom = project.read().await;
            child(&dom, descend(&dom, &["ReplicatedStorage"]), "Doomed")
        };
        let rx = project.subscribe().await;
        sleep(WATCH_WARMUP);

        let path = fx.path("src/Doomed.luau");
        external(&format!("rm -f '{}'", path.display()));

        let deltas = collect_until(
            &rx,
            TIMEOUT,
            |d| matches!(d, Delta::Removed { id, .. } if *id == doomed),
        );
        assert!(
            deltas
                .iter()
                .any(|d| matches!(d, Delta::Removed { id, .. } if *id == doomed)),
            "external delete should surface a Removed; got {deltas:?}"
        );
    });
}
