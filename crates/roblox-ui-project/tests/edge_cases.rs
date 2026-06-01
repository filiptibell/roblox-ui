/*!
    Known-problematic / edge-case fixtures, regenerated per run.
*/

use rbx_dom_weak::{types::Variant, InstanceBuilder, WeakDom};

mod support;

use support::*;

fn game_project() -> &'static str {
    r#"{ "name": "Edge", "tree": { "$className": "DataModel",
         "ReplicatedStorage": { "$path": "src" } } }"#
}

#[test]
fn deeply_nested_directories() {
    let fx = fixture();
    fx.project(game_project())
        .file("src/a/b/c/d/e/Leaf.luau", "return 1\n");
    let project = fx.open();
    run(async {
        let dom = project.read().await;
        let leaf = descend(
            &dom,
            &["ReplicatedStorage", "a", "b", "c", "d", "e", "Leaf"],
        );
        assert_eq!(class_of(&dom, leaf), "ModuleScript");
    });
}

#[test]
fn duplicate_name_inline_and_disk() {
    // An inline project child and a file produce two same-named children.
    let fx = fixture();
    fx.project(
        r#"{ "name": "Dup", "tree": { "$className": "DataModel",
             "ReplicatedStorage": { "$path": "src", "Thing": { "$className": "Folder" } } } }"#,
    )
    .file("src/Thing.luau", "return 1\n");
    let project = fx.open();
    run(async {
        let dom = project.read().await;
        let rs = descend(&dom, &["ReplicatedStorage"]);
        let things = dom
            .children(rs)
            .iter()
            .filter(|id| dom.get_instance(**id).map(|i| i.name.as_str()) == Some("Thing"))
            .count();
        // Both the inline Folder and the disk ModuleScript survive (distinct keys).
        assert_eq!(things, 2);
    });
}

#[test]
fn rbxmx_with_multiple_top_level_takes_first() {
    let fx = fixture();
    // A synthetic container with two top-level instances written as roots.
    let mut dom = WeakDom::new(InstanceBuilder::new("DataModel"));
    let root = dom.root_ref();
    dom.insert(root, InstanceBuilder::new("Part"));
    dom.insert(root, InstanceBuilder::new("WedgePart"));
    let ids = dom.root().children().to_vec();
    let path = fx.path("src/Multi.rbxmx");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut bytes = Vec::new();
    rbx_xml::to_writer_default(&mut bytes, &dom, &ids).unwrap();
    std::fs::write(&path, bytes).unwrap();
    fx.project(game_project());

    let project = fx.open();
    run(async {
        let dom = project.read().await;
        let multi = child(&dom, descend(&dom, &["ReplicatedStorage"]), "Multi");
        // We take the first top-level instance (Rojo would error on multi-root).
        assert_eq!(class_of(&dom, multi), "Part");
    });
}

#[test]
fn default_valued_properties_are_omitted() {
    let fx = fixture();
    fx.project(game_project()).file(
        "src/P.model.json",
        r#"{ "className": "Part", "properties": { "Anchored": false, "CanCollide": false } }"#,
    );
    let project = fx.open();
    run(async {
        let dom = project.read().await;
        let part = child(&dom, descend(&dom, &["ReplicatedStorage"]), "P");
        // Anchored default is false → omitted from the override set.
        assert_eq!(prop(&dom, part, "Anchored"), None, "default omitted");
        // CanCollide default is true → an explicit false IS a real override.
        assert_eq!(prop(&dom, part, "CanCollide"), Some(Variant::Bool(false)));
    });
}

#[test]
fn enum_by_name_and_number() {
    let fx = fixture();
    fx.project(game_project())
        .file(
            "src/ByName.model.json",
            r#"{ "className": "Part", "properties": { "Shape": "Ball" } }"#,
        )
        .file(
            "src/ByNum.model.json",
            r#"{ "className": "Part", "properties": { "Shape": 2 } }"#,
        );
    let project = fx.open();
    run(async {
        let dom = project.read().await;
        let rs = descend(&dom, &["ReplicatedStorage"]);
        assert!(
            matches!(
                prop(&dom, child(&dom, rs, "ByName"), "Shape"),
                Some(Variant::Enum(_))
            ),
            "enum by name resolves"
        );
        assert!(
            matches!(
                prop(&dom, child(&dom, rs, "ByNum"), "Shape"),
                Some(Variant::Enum(_))
            ),
            "enum by number resolves"
        );
    });
}

#[test]
fn children_are_name_sorted() {
    let fx = fixture();
    fx.project(game_project())
        .file("src/Charlie.luau", "")
        .file("src/Alpha.luau", "")
        .file("src/Bravo.luau", "");
    let project = fx.open();
    run(async {
        let dom = project.read().await;
        let rs = descend(&dom, &["ReplicatedStorage"]);
        let names: Vec<String> = dom
            .children(rs)
            .iter()
            .filter_map(|id| dom.get_instance(*id).map(|i| i.name.clone()))
            .collect();
        assert_eq!(names, vec!["Alpha", "Bravo", "Charlie"]);
    });
}

#[test]
fn unusual_filenames() {
    let fx = fixture();
    fx.project(game_project())
        .file("src/My Script.luau", "return 1\n")
        .file("src/ünïcödé.luau", "return 2\n");
    let project = fx.open();
    run(async {
        let dom = project.read().await;
        let rs = descend(&dom, &["ReplicatedStorage"]);
        assert!(try_child(&dom, rs, "My Script").is_some());
        assert!(try_child(&dom, rs, "ünïcödé").is_some());
    });
}

#[test]
fn lone_meta_file_is_ignored() {
    let fx = fixture();
    fx.project(game_project())
        .file("src/Orphan.meta.json", r#"{ "className": "Folder" }"#)
        .file("src/Real.luau", "return 1\n");
    let project = fx.open();
    run(async {
        let dom = project.read().await;
        let rs = descend(&dom, &["ReplicatedStorage"]);
        // The orphan meta produces no instance; the real script still appears.
        assert!(try_child(&dom, rs, "Orphan").is_none());
        assert!(try_child(&dom, rs, "Real").is_some());
    });
}

#[test]
fn missing_path_is_graceful() {
    let fx = fixture();
    fx.project(
        r#"{ "name": "Missing", "tree": { "$className": "DataModel",
             "Gone": { "$className": "Folder", "$path": "does/not/exist" } } }"#,
    );
    let project = fx.open();
    run(async {
        let dom = project.read().await;
        // No crash; the node still exists from its $className.
        let gone = child(&dom, root(&dom), "Gone");
        assert_eq!(class_of(&dom, gone), "Folder");
    });
}
