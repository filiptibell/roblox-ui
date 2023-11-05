use std::{
    env::current_dir,
    path::{Path, PathBuf},
    str::FromStr,
};

use once_cell::sync::Lazy;
use path_clean::PathClean;
use serde::Deserialize;

/**
    Settings for instance watching.

    Note that all fields are optional for deserializing or parsing
    from a string, but have some defaults that may be surprising:

    - `autogenerate` defaults to `true`
    - `include_non_scripts` defaults to `true`
    - `rojo_project_file` defaults to `default.project.json` in the current directory
    - `sourcemap_file` defaults to `sourcemap.json` in the current directory
*/
#[derive(Debug, Clone)]
pub struct Settings {
    // TODO: Implement include_non_scripts and ignore_globs where relevant
    pub autogenerate: bool,
    pub include_non_scripts: bool,
    pub ignore_globs: Vec<String>,
    pub rojo_project_file: PathBuf,
    pub sourcemap_file: PathBuf,
}

impl Settings {
    pub fn is_sourcemap_path(&self, path: &Path) -> bool {
        let abs_path = make_absolute_and_clean(path);
        abs_path == self.sourcemap_file
    }

    pub fn is_rojo_project_path(&self, path: &Path) -> bool {
        let abs_path = make_absolute_and_clean(path);
        abs_path == self.rojo_project_file
    }

    pub fn paths_to_watch(&self) -> Vec<&Path> {
        if self.autogenerate {
            vec![
                self.sourcemap_file.as_ref(),
                self.rojo_project_file.as_ref(),
            ]
        } else {
            vec![self.sourcemap_file.as_ref()]
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        SettingsDeserializable::default().into()
    }
}

impl From<SettingsDeserializable> for Settings {
    fn from(value: SettingsDeserializable) -> Self {
        Self {
            autogenerate: value.autogenerate,
            include_non_scripts: value.include_non_scripts,
            ignore_globs: value.ignore_globs,
            rojo_project_file: value.rojo_project_file.expect("missing rojo_project_file"),
            sourcemap_file: value.sourcemap_file.expect("missing sourcemap_file"),
        }
    }
}

impl FromStr for Settings {
    type Err = serde_json::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut this = serde_json::from_str::<SettingsDeserializable>(s)?;
        this.apply_path_defaults_and_clean();
        Ok(this.into())
    }
}

/**
    Proxy struct for parsing and/or deserializing a `Settings` struct.

    All fields are optional and have defaults, check [`Settings`] for additional details.
*/
#[derive(Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct SettingsDeserializable {
    autogenerate: bool,
    include_non_scripts: bool,
    ignore_globs: Vec<String>,
    rojo_project_file: Option<PathBuf>,
    sourcemap_file: Option<PathBuf>,
}

impl SettingsDeserializable {
    fn apply_path_defaults_and_clean(&mut self) {
        self.rojo_project_file
            .replace(match &self.rojo_project_file {
                Some(proj) => make_absolute_and_clean(proj),
                None => DEFAULT_ROJO_PROJECT_PATH.to_path_buf(),
            });
        self.sourcemap_file.replace(match &self.sourcemap_file {
            Some(smap) => make_absolute_and_clean(smap),
            None => DEFAULT_SOURCEMAP_PATH.to_path_buf(),
        });
    }
}

impl Default for SettingsDeserializable {
    fn default() -> Self {
        let mut this = Self {
            autogenerate: true,
            include_non_scripts: true,
            ignore_globs: vec![],
            rojo_project_file: None,
            sourcemap_file: None,
        };
        this.apply_path_defaults_and_clean();
        this
    }
}

static DEFAULT_SOURCEMAP_PATH: Lazy<PathBuf> = Lazy::new(|| {
    let path = PathBuf::from("sourcemap.json");
    make_absolute_and_clean(&path)
});

static DEFAULT_ROJO_PROJECT_PATH: Lazy<PathBuf> = Lazy::new(|| {
    let path = PathBuf::from("default.project.json");
    make_absolute_and_clean(&path)
});

fn make_absolute_and_clean(path: &Path) -> PathBuf {
    if path.is_relative() {
        current_dir()
            .expect("failed to get current dir")
            .join(path)
            .clean()
    } else {
        path.clean()
    }
}
