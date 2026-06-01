/*!
    Optional fidelity gate: compare our instance tree to **real `rojo`** output.

    If a `rojo` binary is on `PATH`, build a fixture and assert our tree
    (class + name + child structure) matches `rojo sourcemap`. If `rojo` is not
    installed the test self-skips — `rojo` is never a runtime dependency, only a
    dev-time oracle.
*/

use std::path::PathBuf;
use std::process::Command as OsCommand;

use rbx_dom_weak::types::Ref;
use serde::Deserialize;

use roblox_ui_project::Dom;

mod support;

use support::*;

#[derive(Debug, Deserialize)]
struct SourcemapNode {
    name: String,
    #[serde(rename = "className")]
    class_name: String,
    #[serde(default)]
    children: Vec<SourcemapNode>,
}

/**
    Resolve a *real* Rojo binary (one that actually runs `--version`): an explicit
    `ROJO_BIN` override, else `rojo` on `PATH`, else an official binary from a
    rokit tool store. Returns `None` (→ self-skip) if none works.
*/
fn resolve_rojo() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(explicit) = std::env::var("ROJO_BIN") {
        candidates.push(PathBuf::from(explicit));
    }
    candidates.push(PathBuf::from("rojo"));
    if let Ok(home) = std::env::var("HOME") {
        let store = PathBuf::from(home).join(".rokit/tool-storage/rojo-rbx/rojo");
        if let Ok(entries) = std::fs::read_dir(&store) {
            let mut versions: Vec<PathBuf> = entries
                .flatten()
                .map(|e| e.path().join("rojo"))
                .filter(|p| p.is_file())
                .collect();
            // Prefer stable versions (no pre-release dash in the dir name).
            versions.sort();
            versions.reverse();
            versions.sort_by_key(|p| {
                p.parent()
                    .and_then(|d| d.file_name())
                    .and_then(|n| n.to_str())
                    .map(|n| n.contains('-'))
                    .unwrap_or(true)
            });
            candidates.extend(versions);
        }
    }

    candidates.into_iter().find(|bin| {
        OsCommand::new(bin)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    })
}

/**
    A `(name, class, sorted-children)` projection for order-insensitive compare.
*/
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Node {
    name: String,
    class: String,
    children: Vec<Node>,
}

fn from_sourcemap(node: &SourcemapNode) -> Node {
    let mut children: Vec<Node> = node.children.iter().map(from_sourcemap).collect();
    children.sort();
    Node {
        name: node.name.clone(),
        class: node.class_name.clone(),
        children,
    }
}

fn from_dom(dom: &Dom, id: Ref) -> Node {
    let inst = dom.get_instance(id).unwrap();
    let mut children: Vec<Node> = dom.children(id).iter().map(|c| from_dom(dom, *c)).collect();
    children.sort();
    Node {
        name: inst.name.clone(),
        class: inst.class.to_string(),
        children,
    }
}

#[test]
fn matches_real_rojo_sourcemap() {
    let Some(rojo) = resolve_rojo() else {
        eprintln!("skipping rojo_oracle: no runnable `rojo` binary found");
        return;
    };
    eprintln!("rojo_oracle: using {}", rojo.display());

    // A fixture using only universally-agreed rules.
    let fx = fixture();
    fx.project(
        r#"{
            "name": "Oracle",
            "tree": {
                "$className": "DataModel",
                "ReplicatedStorage": { "$path": "shared" },
                "ServerScriptService": { "$path": "server" }
            }
        }"#,
    )
    .file("shared/Module.luau", "return {}\n")
    .file("shared/Nested/init.luau", "return {}\n")
    .file("shared/Nested/Child.luau", "return 1\n")
    .file("shared/Server.server.luau", "print()\n")
    .file(
        "shared/Model.model.json",
        r#"{ "className": "Folder", "children": [ { "className": "BoolValue", "name": "Flag" } ] }"#,
    )
    .file("server/Main.server.luau", "print('main')\n");

    let output = OsCommand::new(&rojo)
        .current_dir(fx.root_path())
        .args(["sourcemap", "default.project.json", "--include-non-scripts"])
        .output()
        .expect("run rojo sourcemap");
    assert!(
        output.status.success(),
        "rojo sourcemap failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let rojo_root: SourcemapNode =
        serde_json::from_slice(&output.stdout).expect("parse rojo sourcemap json");

    let project = fx.open();
    let ours = run(async {
        let dom = project.read().await;
        from_dom(&dom, root(&dom))
    });

    let theirs = from_sourcemap(&rojo_root);
    assert_eq!(ours, theirs, "our tree must match rojo's sourcemap");
}
