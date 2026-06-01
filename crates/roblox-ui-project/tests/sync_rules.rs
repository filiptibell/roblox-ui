/*!
    Per-rule fidelity: every Rojo 7 file → instance sync rule, built from a fresh
    fixture on disk with **no `rojo` binary involved**.
*/

use rbx_dom_weak::{
    types::{Variant, Vector3},
    InstanceBuilder, WeakDom,
};

mod support;

use support::*;

/**
    A standard DataModel project whose ReplicatedStorage maps to `src/`.
*/
fn game_project() -> &'static str {
    r#"{
        "name": "Game",
        "tree": {
            "$className": "DataModel",
            "ReplicatedStorage": { "$path": "src" }
        }
    }"#
}

#[test]
fn script_kinds() {
    let fx = fixture();
    fx.project(game_project())
        .file("src/Module.luau", "return 1\n")
        .file("src/ModuleOld.lua", "return 2\n")
        .file("src/Server.server.luau", "print('s')\n")
        .file("src/Client.client.luau", "print('c')\n")
        .file("src/Plugin.plugin.luau", "print('p')\n");
    let project = fx.open();
    run(async {
        let dom = project.read().await;
        let rs = descend(&dom, &["ReplicatedStorage"]);

        assert_eq!(class_of(&dom, child(&dom, rs, "Module")), "ModuleScript");
        assert_eq!(class_of(&dom, child(&dom, rs, "ModuleOld")), "ModuleScript");
        assert_eq!(class_of(&dom, child(&dom, rs, "Server")), "Script");
        assert_eq!(class_of(&dom, child(&dom, rs, "Client")), "LocalScript");
        // `.plugin` scripts are Scripts.
        assert_eq!(class_of(&dom, child(&dom, rs, "Plugin")), "Script");

        // Source comes from the file text.
        assert_eq!(
            prop(&dom, child(&dom, rs, "Module"), "Source"),
            Some(Variant::String("return 1\n".to_string()))
        );
    });
}

#[test]
fn directory_becomes_folder() {
    let fx = fixture();
    fx.project(game_project()).dir("src/Empty");
    let project = fx.open();
    run(async {
        let dom = project.read().await;
        let rs = descend(&dom, &["ReplicatedStorage"]);
        assert_eq!(class_of(&dom, child(&dom, rs, "Empty")), "Folder");
    });
}

#[test]
fn init_files_promote_directory() {
    let fx = fixture();
    fx.project(game_project())
        .file("src/AsModule/init.luau", "return {}\n")
        .file("src/AsModule/child.luau", "return 1\n")
        .file("src/AsServer/init.server.luau", "print()\n")
        .file("src/AsClient/init.client.luau", "print()\n");
    let project = fx.open();
    run(async {
        let dom = project.read().await;
        let rs = descend(&dom, &["ReplicatedStorage"]);

        let as_module = child(&dom, rs, "AsModule");
        assert_eq!(class_of(&dom, as_module), "ModuleScript");
        // The init's Source promotes onto the directory instance.
        assert_eq!(
            prop(&dom, as_module, "Source"),
            Some(Variant::String("return {}\n".to_string()))
        );
        // Non-init children still appear under the promoted directory.
        assert_eq!(
            class_of(&dom, child(&dom, as_module, "child")),
            "ModuleScript"
        );

        assert_eq!(class_of(&dom, child(&dom, rs, "AsServer")), "Script");
        assert_eq!(class_of(&dom, child(&dom, rs, "AsClient")), "LocalScript");
    });
}

#[test]
fn model_json_implicit_and_nested() {
    let fx = fixture();
    fx.project(game_project()).file(
        "src/Thing.model.json",
        r#"{
            "className": "Part",
            "properties": { "Size": [4, 1, 2], "Anchored": true },
            "children": [
                { "className": "IntValue", "name": "Count", "properties": { "Value": 7 } }
            ]
        }"#,
    );
    let project = fx.open();
    run(async {
        let dom = project.read().await;
        let part = child(&dom, descend(&dom, &["ReplicatedStorage"]), "Thing");
        assert_eq!(class_of(&dom, part), "Part");
        assert_eq!(
            prop(&dom, part, "Size"),
            Some(Variant::Vector3(Vector3::new(4.0, 1.0, 2.0)))
        );
        assert_eq!(prop(&dom, part, "Anchored"), Some(Variant::Bool(true)));

        let count = child(&dom, part, "Count");
        assert_eq!(class_of(&dom, count), "IntValue");
        assert_eq!(prop(&dom, count, "Value"), Some(Variant::Int64(7)));
    });
}

#[test]
fn model_jsonc_with_comments_and_explicit_values() {
    let fx = fixture();
    fx.project(game_project()).file(
        "src/Commented.model.jsonc",
        r#"{
            // a frame with an explicit (tagged) UDim2 size
            "className": "Frame",
            "properties": {
                "Size": { "UDim2": [[0, 100], [0, 50]] } /* explicit */
            }
        }"#,
    );
    let project = fx.open();
    run(async {
        let dom = project.read().await;
        let frame = child(&dom, descend(&dom, &["ReplicatedStorage"]), "Commented");
        assert_eq!(class_of(&dom, frame), "Frame");
        match prop(&dom, frame, "Size") {
            Some(Variant::UDim2(_)) => {}
            other => panic!("expected UDim2, got {other:?}"),
        }
    });
}

#[test]
fn model_json_attributes() {
    let fx = fixture();
    fx.project(game_project()).file(
        "src/WithAttrs.model.json",
        r#"{
            "className": "Folder",
            "attributes": { "Speed": 16, "Enabled": true }
        }"#,
    );
    let project = fx.open();
    run(async {
        let dom = project.read().await;
        let folder = child(&dom, descend(&dom, &["ReplicatedStorage"]), "WithAttrs");
        match prop(&dom, folder, "Attributes") {
            Some(Variant::Attributes(attrs)) => {
                assert!(attrs.get("Speed").is_some());
                assert_eq!(attrs.get("Enabled"), Some(&Variant::Bool(true)));
            }
            other => panic!("expected Attributes, got {other:?}"),
        }
    });
}

#[test]
fn rbxmx_and_rbxm_models() {
    let fx = fixture();
    // Build a model with full properties via rbx_dom_weak, write as both formats.
    let model = || {
        WeakDom::new(
            InstanceBuilder::new("Model")
                .with_child(InstanceBuilder::new("Part").with_property("Anchored", true)),
        )
    };
    fx.project(game_project())
        .rbxmx("src/Xml.rbxmx", &model())
        .rbxm("src/Bin.rbxm", &model());
    let project = fx.open();
    run(async {
        let dom = project.read().await;
        let rs = descend(&dom, &["ReplicatedStorage"]);
        for name in ["Xml", "Bin"] {
            let m = child(&dom, rs, name);
            assert_eq!(class_of(&dom, m), "Model", "{name} class");
            let part = child(&dom, m, "Part");
            assert_eq!(prop(&dom, part, "Anchored"), Some(Variant::Bool(true)));
        }
    });
}

#[test]
fn meta_json_sibling_and_init_overlays() {
    let fx = fixture();
    fx.project(game_project())
        // Sibling overlay: set Disabled on a Script.
        .file("src/Job.server.luau", "return true\n")
        .file(
            "src/Job.meta.json",
            r#"{ "properties": { "Disabled": true } }"#,
        )
        // init.meta.json changes a directory's className.
        .file(
            "src/Special/init.meta.json",
            r#"{ "className": "Configuration" }"#,
        )
        .file("src/Special/value.luau", "return 1\n");
    let project = fx.open();
    run(async {
        let dom = project.read().await;
        let rs = descend(&dom, &["ReplicatedStorage"]);

        let job = child(&dom, rs, "Job");
        assert_eq!(prop(&dom, job, "Disabled"), Some(Variant::Bool(true)));

        let special = child(&dom, rs, "Special");
        assert_eq!(class_of(&dom, special), "Configuration");
        assert_eq!(
            class_of(&dom, child(&dom, special, "value")),
            "ModuleScript"
        );
    });
}

#[test]
fn text_csv_json_toml_rules() {
    let fx = fixture();
    fx.project(game_project())
        .file("src/Notes.txt", "hello world")
        .file("src/Strings.csv", "Key,Source\nGREETING,Hi\n")
        .file("src/Data.json", r#"{ "a": 1 }"#)
        .file("src/Conf.toml", "a = 1\n");
    let project = fx.open();
    run(async {
        let dom = project.read().await;
        let rs = descend(&dom, &["ReplicatedStorage"]);

        let notes = child(&dom, rs, "Notes");
        assert_eq!(class_of(&dom, notes), "StringValue");
        assert_eq!(
            prop(&dom, notes, "Value"),
            Some(Variant::String("hello world".to_string()))
        );
        assert_eq!(
            class_of(&dom, child(&dom, rs, "Strings")),
            "LocalizationTable"
        );
        assert_eq!(class_of(&dom, child(&dom, rs, "Data")), "ModuleScript");
        assert_eq!(class_of(&dom, child(&dom, rs, "Conf")), "ModuleScript");
    });
}

#[test]
fn nested_project_files() {
    let fx = fixture();
    // A child .project.json file.
    fx.project(game_project())
        .project_named(
            "src/Sub.project.json",
            r#"{ "name": "Sub", "tree": { "$className": "Folder",
                 "Inner": { "$className": "BoolValue" } } }"#,
        )
        // A directory containing default.project.json (package-style).
        .file(
            "src/Pkg/default.project.json",
            r#"{ "name": "Pkg", "tree": { "$className": "ModuleScript" } }"#,
        );
    let project = fx.open();
    run(async {
        let dom = project.read().await;
        let rs = descend(&dom, &["ReplicatedStorage"]);

        let sub = child(&dom, rs, "Sub");
        assert_eq!(class_of(&dom, sub), "Folder");
        assert_eq!(class_of(&dom, child(&dom, sub, "Inner")), "BoolValue");

        // The directory `Pkg` is named after the directory but classed by its project.
        let pkg = child(&dom, rs, "Pkg");
        assert_eq!(class_of(&dom, pkg), "ModuleScript");
    });
}

#[test]
fn datamodel_services_get_correct_classes() {
    let fx = fixture();
    fx.project(
        r#"{
            "name": "Services",
            "tree": {
                "$className": "DataModel",
                "ReplicatedStorage": { "$path": "rs" },
                "ServerScriptService": { "$path": "sss" },
                "Workspace": { "$path": "wsp" }
            }
        }"#,
    )
    .dir("rs")
    .dir("sss")
    .dir("wsp");
    let project = fx.open();
    run(async {
        let dom = project.read().await;
        let root = root(&dom);
        assert_eq!(
            class_of(&dom, child(&dom, root, "ReplicatedStorage")),
            "ReplicatedStorage"
        );
        assert_eq!(
            class_of(&dom, child(&dom, root, "ServerScriptService")),
            "ServerScriptService"
        );
        assert_eq!(class_of(&dom, child(&dom, root, "Workspace")), "Workspace");
    });
}

#[test]
fn project_properties_and_path_plus_children_merge() {
    let fx = fixture();
    fx.project(
        r#"{
            "name": "Merge",
            "tree": {
                "$className": "DataModel",
                "Lighting": {
                    "$properties": { "ClockTime": 14 },
                    "Extra": { "$className": "Folder" }
                },
                "ReplicatedStorage": {
                    "$path": "src",
                    "Injected": { "$className": "BoolValue" }
                }
            }
        }"#,
    )
    .file("src/FromDisk.luau", "return 1\n");
    let project = fx.open();
    run(async {
        let dom = project.read().await;
        let root = root(&dom);

        // $properties applied to a class-only node (numeric type per reflection).
        let lighting = child(&dom, root, "Lighting");
        let clock = prop(&dom, lighting, "ClockTime");
        let clock_ok = matches!(clock, Some(Variant::Float32(v)) if (v - 14.0).abs() < 1e-3)
            || matches!(clock, Some(Variant::Float64(v)) if (v - 14.0).abs() < 1e-3);
        assert!(clock_ok, "ClockTime should resolve to ~14, got {clock:?}");
        assert!(try_child(&dom, lighting, "Extra").is_some());

        // $path children + inline children are merged under one instance.
        let rs = child(&dom, root, "ReplicatedStorage");
        assert!(try_child(&dom, rs, "FromDisk").is_some(), "from disk");
        assert!(try_child(&dom, rs, "Injected").is_some(), "inline child");
    });
}

#[test]
fn path_outside_project_directory() {
    // The project file lives in `app/`, but pulls shared code from `../shared`.
    let fx = fixture();
    fx.file("shared/Util.luau", "return {}\n").project_named(
        "app/default.project.json",
        r#"{ "name": "App", "tree": { "$className": "DataModel",
             "ReplicatedStorage": { "$path": "../shared" } } }"#,
    );
    let config = roblox_ui_project::Config {
        project_file: fx.path("app/default.project.json"),
        ignore_globs: Vec::new(),
    };
    let project = run(roblox_ui_project::Project::open_unwatched(config)).unwrap();
    run(async {
        let dom = project.read().await;
        let rs = descend(&dom, &["ReplicatedStorage"]);
        assert_eq!(class_of(&dom, child(&dom, rs, "Util")), "ModuleScript");
    });
}
