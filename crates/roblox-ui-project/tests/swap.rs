/*!
    Live-swapping the root project file (e.g. `default.project.json` ->
    `build.project.json`). The shared, typically-additive portion must reconcile
    in place with stable `Ref`s; only the differing parts emit deltas.
*/

use std::time::Duration;

use roblox_ui_project::Delta;

mod support;

use support::*;

/**
    `default` syncs only ReplicatedStorage; `build` is a superset adding two more
    services. The shared subtree is byte-identical between them.
*/
fn fixture_with_split() -> Fixture {
    let fx = fixture();
    fx.project_named(
        "default.project.json",
        r#"{ "name": "Game", "tree": { "$className": "DataModel",
             "ReplicatedStorage": { "$path": "shared" } } }"#,
    )
    .project_named(
        "build.project.json",
        r#"{ "name": "Game", "tree": { "$className": "DataModel",
             "ReplicatedStorage": { "$path": "shared" },
             "ServerScriptService": { "$path": "server" },
             "Workspace": { "$path": "assets" } } }"#,
    )
    .file("shared/Common.luau", "return 1\n")
    .file("server/Main.server.luau", "print()\n")
    .file("assets/Thing.model.json", r#"{ "className": "Part" }"#);
    fx
}

#[test]
fn swap_is_incremental_and_ref_stable() {
    let fx = fixture_with_split();
    // Open against the lightweight default file.
    let project = fx.open();

    let (root0, rs0, common0) = run(async {
        let dom = project.read().await;
        let root = root(&dom);
        let rs = child(&dom, root, "ReplicatedStorage");
        let common = child(&dom, rs, "Common");
        // Sanity: the heavy services are absent under `default`.
        assert!(try_child(&dom, root, "Workspace").is_none());
        assert!(try_child(&dom, root, "ServerScriptService").is_none());
        (root, rs, common)
    });

    // Swap to the build file (a relative path, resolved next to the current one).
    let deltas = run(project.set_project_file("build.project.json"));

    // Only additive deltas: the new services Added, nothing Removed.
    let added: Vec<String> = deltas
        .iter()
        .filter_map(|d| match d {
            Delta::Added { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect();
    assert!(
        added.iter().any(|n| n == "ServerScriptService"),
        "added SSS: {added:?}"
    );
    assert!(
        added.iter().any(|n| n == "Workspace"),
        "added Workspace: {added:?}"
    );
    assert!(
        !deltas.iter().any(|d| matches!(d, Delta::Removed { .. })),
        "no removals on an additive swap: {deltas:?}"
    );

    run(async {
        let dom = project.read().await;
        // Root + the shared subtree kept their exact Refs.
        assert_eq!(root(&dom), root0, "root Ref stable");
        assert_eq!(
            child(&dom, root0, "ReplicatedStorage"),
            rs0,
            "RS Ref stable"
        );
        assert_eq!(child(&dom, rs0, "Common"), common0, "Common Ref stable");
        // The heavy services are now present.
        assert_eq!(class_of(&dom, child(&dom, root0, "Workspace")), "Workspace");
        assert_eq!(
            class_of(&dom, child(&dom, root0, "ServerScriptService")),
            "ServerScriptService"
        );
        // The active project file is reported.
        assert!(project.project_file().await.ends_with("build.project.json"));
    });

    // Swap back: the extras are Removed, the shared subtree stays stable.
    let back = run(project.set_project_file("default.project.json"));
    let removed = back
        .iter()
        .filter(|d| matches!(d, Delta::Removed { .. }))
        .count();
    assert!(
        removed >= 2,
        "swapping back removes the extra services: {back:?}"
    );

    run(async {
        let dom = project.read().await;
        assert!(try_child(&dom, root0, "Workspace").is_none());
        assert!(try_child(&dom, root0, "ServerScriptService").is_none());
        assert_eq!(
            child(&dom, rs0, "Common"),
            common0,
            "Common Ref still stable"
        );
    });
}

#[test]
fn swap_under_watcher_observes_new_area() {
    let fx = fixture_with_split();
    // Open watched on the default file (assets/ is not part of the tree yet).
    let project = fx.open_watched();
    run(project.set_project_file("build.project.json"));

    run(async {
        let rx = project.subscribe().await;
        // Let the restarted watcher register.
        async_io::Timer::after(Duration::from_millis(600)).await;

        // Externally edit a file that only exists in the tree under `build`.
        let thing = fx.path("assets/Thing.model.json");
        std::process::Command::new("sh")
            .arg("-c")
            .arg(format!(
                "printf '{{\"className\":\"WedgePart\"}}' > '{}'",
                thing.display()
            ))
            .status()
            .unwrap();

        // The reclass surfaces transparently after the swap.
        let deltas = collect_until(&rx, Duration::from_secs(10), |d| {
            matches!(d, Delta::Reclassed { .. } | Delta::Added { .. })
        });
        assert!(
            !deltas.is_empty(),
            "an external change under a build-only area should surface after the swap"
        );
    });
}
