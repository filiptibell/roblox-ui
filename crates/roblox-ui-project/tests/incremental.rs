/*!
    Stable-`Ref` incremental deltas, driven deterministically via `sync_path`
    (no background watcher) so exact delta counts are assertable.
*/

use roblox_ui_project::{Command, Delta};

mod support;

use support::*;

fn game_project() -> &'static str {
    r#"{ "name": "Inc", "tree": { "$className": "DataModel",
         "ReplicatedStorage": { "$path": "src" } } }"#
}

#[test]
fn property_edit_keeps_stable_ref() {
    let fx = fixture();
    fx.project(game_project())
        .file("src/Hello.luau", "return 1\n")
        .file("src/Sibling.luau", "return 0\n");
    let project = fx.open();

    run(async {
        let (hello, sibling) = {
            let dom = project.read().await;
            let rs = descend(&dom, &["ReplicatedStorage"]);
            (child(&dom, rs, "Hello"), child(&dom, rs, "Sibling"))
        };
        let rx = project.subscribe().await;

        fx.file("src/Hello.luau", "return 2\n");
        project.sync_path(&fx.path("src/Hello.luau")).await;

        let deltas = drain(&rx);
        let prop_changes: Vec<_> = deltas
            .iter()
            .filter(|d| matches!(d, Delta::PropertiesChanged { .. }))
            .collect();
        assert_eq!(prop_changes.len(), 1);
        assert_eq!(prop_changes[0].id(), hello, "same Ref");
        assert!(!deltas.iter().any(|d| matches!(d, Delta::Removed { .. })));

        // The sibling never churned.
        let dom = project.read().await;
        assert!(dom.get_instance(sibling).is_some());
        assert_eq!(
            prop(&dom, hello, "Source"),
            Some(rbx_dom_weak::types::Variant::String(
                "return 2\n".to_string()
            ))
        );
    });
}

#[test]
fn deep_edit_touches_only_subtree() {
    let fx = fixture();
    fx.project(game_project())
        .file("src/deep/nest/Leaf.luau", "return 1\n")
        .file("src/Other.luau", "return 0\n");
    let project = fx.open();
    run(async {
        let (leaf, other) = {
            let dom = project.read().await;
            (
                descend(&dom, &["ReplicatedStorage", "deep", "nest", "Leaf"]),
                child(&dom, descend(&dom, &["ReplicatedStorage"]), "Other"),
            )
        };
        let rx = project.subscribe().await;

        fx.file("src/deep/nest/Leaf.luau", "return 99\n");
        project.sync_path(&fx.path("src/deep/nest/Leaf.luau")).await;

        let deltas = drain(&rx);
        assert!(deltas
            .iter()
            .all(|d| matches!(d, Delta::PropertiesChanged { id, .. } if *id == leaf)));
        let dom = project.read().await;
        assert!(dom.get_instance(other).is_some(), "Other stable");
    });
}

#[test]
fn edit_directly_under_service_root_is_stable() {
    let fx = fixture();
    fx.project(game_project())
        .file("src/Top.luau", "return 1\n");
    let project = fx.open();
    run(async {
        let top = {
            let dom = project.read().await;
            child(&dom, descend(&dom, &["ReplicatedStorage"]), "Top")
        };
        let rx = project.subscribe().await;

        fx.file("src/Top.luau", "return 2\n");
        project.sync_path(&fx.path("src/Top.luau")).await;

        let deltas = drain(&rx);
        let props: Vec<_> = deltas
            .iter()
            .filter(|d| matches!(d, Delta::PropertiesChanged { .. }))
            .collect();
        assert_eq!(props.len(), 1);
        assert_eq!(
            props[0].id(),
            top,
            "Ref stable even directly under a service"
        );
    });
}

#[test]
fn add_file_yields_single_added() {
    let fx = fixture();
    fx.project(game_project())
        .file("src/deep/Existing.luau", "return 1\n");
    let project = fx.open();
    run(async {
        let existing = {
            let dom = project.read().await;
            descend(&dom, &["ReplicatedStorage", "deep", "Existing"])
        };
        let rx = project.subscribe().await;

        fx.file("src/deep/Fresh.luau", "return 2\n");
        project.sync_path(&fx.path("src/deep/Fresh.luau")).await;

        let deltas = drain(&rx);
        let added: Vec<String> = deltas
            .iter()
            .filter_map(|d| match d {
                Delta::Added { name, .. } => Some(name.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(added, vec!["Fresh".to_string()]);
        assert!(!deltas.iter().any(|d| matches!(d, Delta::Removed { .. })));
        let dom = project.read().await;
        assert!(dom.get_instance(existing).is_some(), "Existing stable");
    });
}

#[test]
fn delete_via_filesystem_yields_single_removed() {
    let fx = fixture();
    fx.project(game_project())
        .file("src/deep/Doomed.luau", "return 1\n")
        .file("src/deep/Keep.luau", "return 2\n");
    let project = fx.open();
    run(async {
        let (doomed, keep) = {
            let dom = project.read().await;
            (
                descend(&dom, &["ReplicatedStorage", "deep", "Doomed"]),
                descend(&dom, &["ReplicatedStorage", "deep", "Keep"]),
            )
        };
        let rx = project.subscribe().await;

        let doomed_path = fx.path("src/deep/Doomed.luau");
        fx.remove("src/deep/Doomed.luau");
        project.sync_path(&doomed_path).await;

        let deltas = drain(&rx);
        let removed: Vec<_> = deltas
            .iter()
            .filter_map(|d| match d {
                Delta::Removed { id, .. } => Some(*id),
                _ => None,
            })
            .collect();
        assert_eq!(removed, vec![doomed], "only the deleted file");
        let dom = project.read().await;
        assert!(dom.get_instance(keep).is_some(), "Keep stable");
    });
}

#[test]
fn delete_via_command() {
    let fx = fixture();
    fx.project(game_project())
        .file("src/Gone.luau", "return 1\n");
    let project = fx.open();
    run(async {
        let gone = {
            let dom = project.read().await;
            child(&dom, descend(&dom, &["ReplicatedStorage"]), "Gone")
        };
        let deltas = project.apply(Command::Delete { id: gone }).await.unwrap();
        assert!(deltas
            .iter()
            .any(|d| matches!(d, Delta::Removed { id, .. } if *id == gone)));
        let dom = project.read().await;
        assert!(dom.get_instance(gone).is_none());
    });
}
