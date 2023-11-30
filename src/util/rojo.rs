use std::path::Path;

/**
    File extension -> class name conversions as defined by the Rojo spec:

    https://rojo.space/docs/v7/sync-details/

    Note that we intentionally mark any model / meta files as plain instances
    here, since we would have to read their contents to get proper class name
*/
pub const CLASS_NAME_SUFFIXES: &[(&str, &str)] = &[
    (".server.luau", "Script"),
    (".server.lua", "Script"),
    (".client.luau", "LocalScript"),
    (".client.lua", "LocalScript"),
    (".luau", "ModuleScript"),
    (".lua", "ModuleScript"),
    (".rbxmx", "Instance"),
    (".rbxm", "Instance"),
    (".txt", "StringValue"),
    (".csv", "LocalizationTable"),
    (".model.json", "Instance"),
    (".project.json", "Instance"),
    (".meta.json", "Instance"),
    (".json", "ModuleScript"),
];

pub fn file_name_str(path: &Path) -> Option<&str> {
    path.file_name().and_then(|f| f.to_str())
}

pub fn parse_name_and_class_name(path: &Path) -> Option<(&str, &'static str)> {
    let file_name = file_name_str(path)?;
    for (suffix, class_name) in CLASS_NAME_SUFFIXES {
        if file_name.ends_with(suffix) {
            let name = file_name.strip_suffix(suffix).unwrap();
            return Some((name, class_name));
        }
    }
    None
}
