/*!
    Scale benchmark: load a large generated project (~1M instances) and measure
    that a single-file content edit produces its delta within a sane budget.

    Run with: `cargo run -p roblox-ui-project --release --example bench_scale`
    Override the instance count with `COUNT=1000000`.

    The bulk of the instances are produced from a handful of large `.model.json`
    files (each holding a wide subtree), so total instance count is high while
    the watched file count stays modest. The *measured edit* targets a lone
    script file, which the incremental fast path rebuilds in O(subtree).
*/

use std::fs;
use std::time::Instant;

use futures_lite::future::block_on;

use roblox_ui_project::{Config, Delta, Project};

fn main() {
    let total: usize = std::env::var("COUNT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1_000_000);
    let per_file: usize = 5_000;
    let files = total.div_ceil(per_file);

    let dir = std::env::temp_dir().join("roblox-ui-bench-scale");
    let _ = fs::remove_dir_all(&dir);
    let src = dir.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        dir.join("default.project.json"),
        r#"{ "name": "Bench", "tree": { "$className": "DataModel",
            "ReplicatedStorage": { "$path": "src" } } }"#,
    )
    .unwrap();

    // Generate `files` model.json files, each a Folder with `per_file` children.
    let gen_start = Instant::now();
    for f in 0..files {
        let mut json = String::with_capacity(per_file * 40);
        json.push_str(r#"{"className":"Folder","children":["#);
        for i in 0..per_file {
            if i > 0 {
                json.push(',');
            }
            json.push_str(r#"{"className":"Folder","name":"n"#);
            json.push_str(&i.to_string());
            json.push_str(r#""}"#);
        }
        json.push_str("]}");
        fs::write(src.join(format!("bulk{f}.model.json")), json).unwrap();
    }
    // A lone script — the target of the measured edit.
    let leaf = src.join("Leaf.luau");
    fs::write(&leaf, "return 1\n").unwrap();
    println!(
        "generated {files} files x {per_file} (+overhead) in {:?}",
        gen_start.elapsed()
    );

    block_on(async {
        // Initial full sync of ~1M instances.
        let load_start = Instant::now();
        let project = Project::open(Config {
            project_file: dir.join("default.project.json"),
            ignore_globs: Vec::new(),
        })
        .await
        .expect("open");
        let load = load_start.elapsed();

        let count = project.read().await.len();
        println!("initial sync: {count} instances in {load:?}");

        let deltas = project.subscribe().await;

        // Single-file content edit on the lone script.
        fs::write(&leaf, "return 2\n").unwrap();
        let edit_start = Instant::now();
        project.sync_path(&leaf).await;
        let edit = edit_start.elapsed();

        let mut prop_changes = 0;
        while let Ok(batch) = deltas.try_recv() {
            for d in batch {
                if matches!(d, Delta::PropertiesChanged { .. }) {
                    prop_changes += 1;
                }
            }
        }
        println!("single-file edit: {prop_changes} PropertiesChanged delta in {edit:?}");
        assert_eq!(
            prop_changes, 1,
            "edit must yield exactly one property delta"
        );
    });
}
