/*!
    In-process, library-only usage with **transparent** file watching — no stdio,
    no RPC, and no manual re-sync.

    Run with: `cargo run -p roblox-ui-project --example library_usage`

    It builds a tiny project on disk, opens it as a [`Project`] (which starts
    watching in the background automatically), subscribes to the delta stream,
    then edits a file **from a separate OS process**. The change surfaces as a
    `PropertiesChanged` delta on the same stable `Ref` — exactly as it would if an
    editor or build tool elsewhere on the machine touched the file.
*/

use std::process::Command;
use std::time::Duration;

use async_io::Timer;
use futures_lite::future::{block_on, or};

use roblox_ui_project::{Config, Delta, Project};

fn main() {
    let dir = std::env::temp_dir().join("roblox-ui-example-library");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(
        dir.join("default.project.json"),
        r#"{ "name": "Example", "tree": { "$className": "DataModel",
            "ReplicatedStorage": { "$path": "src" } } }"#,
    )
    .unwrap();
    let script = dir.join("src/Greeting.luau");
    std::fs::write(&script, "return 'hello'\n").unwrap();

    block_on(async {
        // 1. Open the backend. The file watcher is now running in the background.
        let project = Project::open(Config {
            project_file: dir.join("default.project.json"),
            ignore_globs: Vec::new(),
        })
        .await
        .expect("open project");

        // 2. Subscribe to the self-contained delta stream.
        let deltas = project.subscribe().await;

        // 3. Read the initial tree + a typed property.
        let greeting = {
            let dom = project.read().await;
            let root = dom.get_root_id().unwrap();
            let rs = dom
                .children(root)
                .iter()
                .copied()
                .find(|id| {
                    dom.get_instance(*id).map(|i| i.name.as_str()) == Some("ReplicatedStorage")
                })
                .unwrap();
            let greeting = dom
                .children(rs)
                .iter()
                .copied()
                .find(|id| dom.get_instance(*id).map(|i| i.name.as_str()) == Some("Greeting"))
                .unwrap();
            println!(
                "loaded Greeting, Source = {:?}",
                dom.get_properties(greeting).unwrap().get(&"Source".into())
            );
            greeting
        };

        // Give the watcher a moment to register with the OS.
        Timer::after(Duration::from_millis(500)).await;

        // 4. Edit the file from a SEPARATE PROCESS — we never touch the Project.
        println!("editing src/Greeting.luau from an external `sh` process...");
        Command::new("sh")
            .arg("-c")
            .arg(format!(
                "printf \"return 'goodbye'\\n\" > '{}'",
                script.display()
            ))
            .status()
            .unwrap();

        // 5. The change surfaces automatically as a delta on the same Ref.
        let deadline = Duration::from_secs(5);
        loop {
            let batch = or(async { deltas.recv().await.ok() }, async {
                Timer::after(deadline).await;
                None
            })
            .await;
            let Some(batch) = batch else {
                println!("(timed out waiting for a delta)");
                break;
            };
            let hit = batch
                .iter()
                .any(|d| matches!(d, Delta::PropertiesChanged { id, .. } if *id == greeting));
            for delta in &batch {
                if let Delta::PropertiesChanged { id, changed } = delta {
                    println!(
                        "transparent delta: PropertiesChanged on {:?} (same ref = {}): {:?}",
                        id,
                        *id == greeting,
                        changed
                    );
                }
            }
            if hit {
                break;
            }
        }
    });
}
