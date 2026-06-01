/*!
    File -> sync-rule classification, mirroring Rojo 7's `snapshot_middleware`
    dispatch order. Pure functions, no I/O.
*/

/**
    The sync rule a single file maps to.
*/
#[derive(Debug, Clone, Copy)]
pub(crate) enum FileRule {
    /// A script of the given class name, with `Source` read from the file text.
    Script(&'static str),
    Rbxmx,
    Rbxm,
    ModelJson,
    /// A nested `*.project.json` file.
    Project,
    /// A `*.meta.json` overlay — never an instance on its own.
    Meta,
    /// `.json` / `.jsonc` / `.toml` / `.yaml` → `ModuleScript`.
    Module,
    LocalizationTable,
    StringValue,
    /// A standalone `init.*` file — consumed by its directory.
    Init,
}

/**
    Whether a filename is a reserved `init.*` file that promotes its directory.
*/
pub(crate) fn is_init_filename(fname: &str) -> bool {
    matches!(
        fname,
        "init.luau"
            | "init.lua"
            | "init.server.luau"
            | "init.server.lua"
            | "init.client.luau"
            | "init.client.lua"
            | "init.plugin.luau"
            | "init.plugin.lua"
            | "init.csv"
    )
}

/**
    Classify a filename into a sync rule, returning the instance name (the file
    stem with the matched suffix removed) and the rule. Order matters: compound
    suffixes are matched before their shorter counterparts.
*/
pub(crate) fn classify_file(fname: &str) -> Option<(&str, FileRule)> {
    if is_init_filename(fname) {
        return Some(("", FileRule::Init));
    }

    const RULES: &[(&str, FileRule)] = &[
        (".server.luau", FileRule::Script("Script")),
        (".server.lua", FileRule::Script("Script")),
        (".client.luau", FileRule::Script("LocalScript")),
        (".client.lua", FileRule::Script("LocalScript")),
        (".plugin.luau", FileRule::Script("Script")),
        (".plugin.lua", FileRule::Script("Script")),
        (".luau", FileRule::Script("ModuleScript")),
        (".lua", FileRule::Script("ModuleScript")),
        (".rbxmx", FileRule::Rbxmx),
        (".rbxm", FileRule::Rbxm),
        (".model.json", FileRule::ModelJson),
        (".model.jsonc", FileRule::ModelJson),
        (".project.json", FileRule::Project),
        (".project.jsonc", FileRule::Project),
        (".meta.json", FileRule::Meta),
        (".meta.jsonc", FileRule::Meta),
        (".json", FileRule::Module),
        (".jsonc", FileRule::Module),
        (".toml", FileRule::Module),
        (".csv", FileRule::LocalizationTable),
        (".txt", FileRule::StringValue),
        (".yaml", FileRule::Module),
        (".yml", FileRule::Module),
    ];

    for (suffix, rule) in RULES {
        if let Some(name) = fname.strip_suffix(suffix) {
            return Some((name, *rule));
        }
    }
    None
}

/**
    Whether a filename's properties come from a parsed model, and so must not be
    overlaid by a sibling `.meta.json`'s `properties` (Rojo forbids this).
*/
pub(crate) fn is_model_path(fname: &str) -> bool {
    fname.ends_with(".rbxmx")
        || fname.ends_with(".rbxm")
        || fname.ends_with(".model.json")
        || fname.ends_with(".model.jsonc")
}

/**
    The class a directory becomes for a given `init.*` filename, and whether it
    is a script whose `Source` should be read.
*/
pub(crate) fn init_dir_class(fname: &str) -> Option<(&'static str, bool)> {
    Some(match fname {
        "init.luau" | "init.lua" => ("ModuleScript", true),
        "init.server.luau" | "init.server.lua" => ("Script", true),
        "init.client.luau" | "init.client.lua" => ("LocalScript", true),
        "init.plugin.luau" | "init.plugin.lua" => ("Script", true),
        "init.csv" => ("LocalizationTable", false),
        _ => return None,
    })
}

/**
    Rojo's directory `init` priority — the first present wins.
*/
pub(crate) const INIT_PRIORITY: &[&str] = &[
    "init.luau",
    "init.lua",
    "init.server.luau",
    "init.server.lua",
    "init.client.luau",
    "init.client.lua",
    "init.plugin.luau",
    "init.plugin.lua",
    "init.csv",
];
