use std::path::Path;

const FILE_PATH_SUFFIXES: &[&str] = &[".luau", ".lua", ".rbxmx", ".rbxm", ".txt", ".csv", ".json"];

const OPENABLE_PATH_SUFFIXES: &[&str] = &[".luau", ".lua", ".csv"];

pub fn path_file_name(path: &Path) -> Option<&str> {
    path.file_name().and_then(|fname| fname.to_str())
}

pub fn is_known_file(file_name: &str) -> bool {
    FILE_PATH_SUFFIXES
        .iter()
        .any(|suffix| file_name.ends_with(suffix))
}

pub fn is_openable_file(file_name: &str) -> bool {
    OPENABLE_PATH_SUFFIXES
        .iter()
        .any(|suffix| file_name.ends_with(suffix))
}

pub fn is_false(v: &bool) -> bool {
    v == &false
}
