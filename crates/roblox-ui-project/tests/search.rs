/*!
    The Roblox Studio explorer search, per
    <https://create.roblox.com/docs/studio/explorer#search-methods>: name
    substring, `is:`/`tag:`, property comparisons (with sub-fields, quoted units,
    enum-by-name, default fallback), dotted ancestry with `*`/`**`, and `and`/`or`.
*/

use rbx_dom_weak::{
    types::{CFrame, Color3, Matrix3, Tags, Vector3},
    InstanceBuilder, WeakDom,
};

use roblox_ui_project::Project;

mod support;

use support::*;

fn tags(list: &[&str]) -> Tags {
    let mut t = Tags::new();
    for s in list {
        t.push(s);
    }
    t
}

/**
    A `Workspace.Cart.Wheel.Trim` model (rich properties) plus scripts and a
    shared folder, exercising every search facet.
*/
fn searchable() -> Project {
    let fx = fixture();
    fx.project(
        r#"{ "name": "Game", "tree": { "$className": "DataModel",
             "Workspace": { "$path": "wsp" },
             "ReplicatedStorage": { "$path": "shared" } } }"#,
    );

    let trim = InstanceBuilder::new("Part")
        .with_name("Trim")
        .with_property("Anchored", false)
        .with_property("Transparency", 1.0f32)
        .with_property("Tags", tags(&["Light Source"]));
    let wheel = InstanceBuilder::new("Part")
        .with_name("Wheel")
        .with_property("Anchored", true)
        .with_property("Transparency", 0.5f32)
        .with_property("Size", Vector3::new(20.0, 5.0, 20.0))
        // Canonical stored props (Position/Color are aliases of these).
        .with_property(
            "CFrame",
            CFrame::new(Vector3::new(1.0, 2.0, 3.0), Matrix3::identity()),
        )
        .with_property("Color", Color3::new(1.0, 0.0, 0.0))
        .with_property("Tags", tags(&["Spinning"]))
        .with_child(trim);
    let cart = InstanceBuilder::new("Model")
        .with_name("Cart")
        .with_child(wheel);
    fx.rbxmx("wsp/Cart.rbxmx", &WeakDom::new(cart));

    fx.file("wsp/MainScript.server.luau", "print()\n");
    fx.file("shared/Stuff/Helper.luau", "return {}\n");

    // Keep the fixture dir alive for the duration of the returned Project by
    // leaking it — tests are short-lived processes.
    let project = fx.open();
    std::mem::forget(fx);
    project
}

/**
    Sorted names of the instances a query matches.
*/
fn found(project: &Project, query: &str) -> Vec<String> {
    run(async {
        let dom = project.read().await;
        let mut names: Vec<String> = dom
            .search(query, None)
            .iter()
            .filter_map(|id| dom.get_instance(*id).map(|i| i.name.clone()))
            .collect();
        names.sort();
        names
    })
}

#[test]
fn name_substring_case_insensitive() {
    let p = searchable();
    assert_eq!(found(&p, "wheel"), vec!["Wheel"]);
    assert_eq!(found(&p, "WHEEL"), vec!["Wheel"]);
    assert_eq!(found(&p, "script"), vec!["MainScript"]);
}

#[test]
fn is_class_and_subclass() {
    let p = searchable();
    assert_eq!(found(&p, "is:Part"), vec!["Trim", "Wheel"]);
    assert_eq!(found(&p, "is:BasePart"), vec!["Trim", "Wheel"]);
    // Workspace is itself a subclass of Model in the Roblox class hierarchy.
    assert_eq!(found(&p, "is:Model"), vec!["Cart", "Workspace"]);
    assert_eq!(
        found(&p, "is:LuaSourceContainer"),
        vec!["Helper", "MainScript"]
    );
}

#[test]
fn tag_search_with_spaces() {
    let p = searchable();
    assert_eq!(found(&p, "tag:Spinning"), vec!["Wheel"]);
    assert_eq!(found(&p, r#"tag:"Light Source""#), vec!["Trim"]);
}

#[test]
fn property_bool_and_number() {
    let p = searchable();
    assert_eq!(found(&p, "Anchored=true"), vec!["Wheel"]);
    assert_eq!(found(&p, "Anchored=false"), vec!["Trim"]); // default-fallback resolves it
    assert_eq!(found(&p, "Transparency > 0.4"), vec!["Trim", "Wheel"]);
    assert_eq!(found(&p, "Transparency = 0.5"), vec!["Wheel"]);
    assert_eq!(found(&p, "Transparency ~= 0.5"), vec!["Trim"]);
    assert_eq!(found(&p, "Transparency >= 1"), vec!["Trim"]);
}

#[test]
fn property_string_partial_and_classname() {
    let p = searchable();
    // Default Material is Plastic for parts; partial, case-insensitive.
    assert_eq!(found(&p, "Material == plas"), vec!["Trim", "Wheel"]);
    assert_eq!(found(&p, "ClassName = Part"), vec!["Trim", "Wheel"]);
    assert!(found(&p, "ClassName = Decal").is_empty());
    // `classname:` partial filter form.
    assert_eq!(found(&p, "classname:Model"), vec!["Cart"]);
}

#[test]
fn property_subfields_and_units() {
    let p = searchable();
    assert_eq!(found(&p, "Position.X = 1"), vec!["Wheel"]);
    // 0-255 scale; >200 excludes Trim's default mid-gray (~163).
    assert_eq!(found(&p, "Color.R > 200"), vec!["Wheel"]);
    assert_eq!(found(&p, r#"Size > "10, 1, 10""#), vec!["Wheel"]);
}

#[test]
fn ancestry_paths() {
    let p = searchable();
    assert_eq!(found(&p, "Workspace.Cart"), vec!["Cart"]);
    assert_eq!(found(&p, "Cart.Wheel"), vec!["Wheel"]);
    assert_eq!(found(&p, "Cart.*.Trim"), vec!["Trim"]); // grandchild
    assert_eq!(found(&p, "Cart.**"), vec!["Trim", "Wheel"]); // all descendants
}

#[test]
fn boolean_combinations() {
    let p = searchable();
    assert_eq!(found(&p, "Anchored=true and is:Part"), vec!["Wheel"]);
    assert_eq!(found(&p, "Wheel or Trim"), vec!["Trim", "Wheel"]);
    assert_eq!(
        found(&p, "(Transparency=1) or (Transparency=0.5)"),
        vec!["Trim", "Wheel"]
    );
    // Implicit-AND of an ancestry filter and a property comparison.
    assert_eq!(found(&p, "Cart.** Transparency=1"), vec!["Trim"]);
}

#[test]
fn limit_caps_results() {
    let p = searchable();
    let count = run(async { p.read().await.search("is:BasePart", Some(1)).len() });
    assert_eq!(count, 1);
}
