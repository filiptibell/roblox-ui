use std::{
    env::current_dir,
    path::{Path, PathBuf},
    str::FromStr,
};

use once_cell::sync::Lazy;
use path_clean::PathClean;
use serde::Deserialize;

/**
    Configuration for the instance server.

    Note that all fields are optional for deserializing or parsing
    from a string, but have some defaults that may be surprising:

    - `autogenerate` defaults to `true`
    - `include_non_scripts` defaults to `true`
    - `rojo_project_file` defaults to `default.project.json` in the current directory
    - `sourcemap_file` defaults to `sourcemap.json` in the current directory
*/
#[derive(Debug, Clone)]
pub struct Config {
    pub autogenerate: bool,
    pub rojo_project_file: PathBuf,
    pub sourcemap_file: PathBuf,
}

impl Config {
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
                /*
                    NOTE: Order here is important! We should put the project
                    file first, since during initialization these paths and their
                    corresponding instance providers are started & checked in order.

                    The rojo project provider will take precedence and ensure we never
                    use the sourcemap, and emit an initial massive instance tree diff.
                */
                self.rojo_project_file.as_ref(),
                self.sourcemap_file.as_ref(),
            ]
        } else {
            vec![self.sourcemap_file.as_ref()]
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        ConfigDeserializable::default().into()
    }
}

impl From<ConfigDeserializable> for Config {
    fn from(value: ConfigDeserializable) -> Self {
        Self {
            autogenerate: value.autogenerate,
            rojo_project_file: value.rojo_project_file.expect("missing rojo_project_file"),
            sourcemap_file: value.sourcemap_file.expect("missing sourcemap_file"),
        }
    }
}

impl FromStr for Config {
    type Err = serde_json::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        let trimmed = if (trimmed.starts_with('\'') && trimmed.ends_with('\''))
            || (trimmed.starts_with('\"') && trimmed.ends_with('\"'))
        {
            &trimmed[1..trimmed.len() - 1]
        } else {
            trimmed
        };
        if trimmed.is_empty() || trimmed == "null" {
            Ok(Self::default())
        } else {
            let mut this = serde_json::from_str::<ConfigDeserializable>(trimmed)?;
            this.apply_path_defaults_and_clean();
            Ok(this.into())
        }
    }
}

#[test]
fn parse_config() {
    let full_conf = r#"
    {
        "autogenerate": true,
        "ignoreNonScripts": false,
        "rojoProjectFile": "default.project.json",
        "sourcemapFile": "sourcemap.json"
    }
    "#;
    assert!("".parse::<Config>().is_ok());
    assert!("''".parse::<Config>().is_ok());
    assert!("null".parse::<Config>().is_ok());
    assert!("{}".parse::<Config>().is_ok());
    assert!("'{}'".parse::<Config>().is_ok());
    assert!(full_conf.parse::<Config>().is_ok());
}

/**
    Proxy struct for parsing and/or deserializing a `Config` struct.

    All fields are optional and have defaults, check [`Config`] for additional details.
*/
#[derive(Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ConfigDeserializable {
    autogenerate: bool,
    rojo_project_file: Option<PathBuf>,
    sourcemap_file: Option<PathBuf>,
}

impl ConfigDeserializable {
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

impl Default for ConfigDeserializable {
    fn default() -> Self {
        let mut this = Self {
            autogenerate: true,
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
