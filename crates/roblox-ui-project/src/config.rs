/*!
    Configuration for the in-house project backend - which `*.project.json`
    to load and which globs to ignore. Accepts (and ignores) the legacy
    sourcemap/Rojo settings keys for backwards compatibility.
*/

use std::{path::PathBuf, str::FromStr, sync::LazyLock};

use serde::Deserialize;

use roblox_ui_util::path::make_absolute_and_clean;

/**
    Configuration for the in-house project backend.

    All fields are optional when parsing; the project file defaults to
    `default.project.json` in the current directory. The legacy `sourcemapFile`
    and `autogenerate` keys are still accepted (and ignored) for compatibility
    with existing extension settings — we no longer use a sourcemap or Rojo.
*/
#[derive(Debug, Clone)]
pub struct Config {
    /// Path to the `*.project.json` to load.
    pub project_file: PathBuf,
    /// Extra glob patterns to ignore while watching/walking.
    pub ignore_globs: Vec<String>,
}

impl Config {
    /**
        The compiled ignore globs, skipping any that fail to parse.
    */
    pub fn ignore_patterns(&self) -> Vec<glob::Pattern> {
        self.ignore_globs
            .iter()
            .filter_map(|g| glob::Pattern::new(g).ok())
            .collect()
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
            project_file: value
                .project_file
                .or(value.rojo_project_file)
                .map(make_absolute_and_clean)
                .unwrap_or_else(|| DEFAULT_PROJECT_PATH.clone()),
            ignore_globs: value.ignore_globs,
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
            let this = serde_json::from_str::<ConfigDeserializable>(trimmed)?;
            Ok(this.into())
        }
    }
}

#[test]
fn parse_config() {
    let full_conf = r#"
    {
        "autogenerate": true,
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
    let parsed = full_conf.parse::<Config>().unwrap();
    assert!(parsed.project_file.ends_with("default.project.json"));
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct ConfigDeserializable {
    project_file: Option<PathBuf>,
    /// Legacy alias for `project_file`.
    rojo_project_file: Option<PathBuf>,
    #[serde(default)]
    ignore_globs: Vec<String>,
}

static DEFAULT_PROJECT_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| make_absolute_and_clean(PathBuf::from("default.project.json")));
