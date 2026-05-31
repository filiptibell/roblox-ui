use std::{
    env::current_dir,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use path_clean::PathClean;

static CWD: LazyLock<PathBuf> = LazyLock::new(|| current_dir().expect("failed to get current dir"));

pub fn make_absolute_and_clean(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    if path.is_relative() {
        CWD.join(path).clean()
    } else {
        path.clean()
    }
}
