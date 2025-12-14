//! Common functionality shared among all components of the shell
use std::{path::PathBuf, sync::LazyLock};

pub mod config;
pub mod css;
pub mod err;
pub mod types;

pub use config::{Config, layouts::Layouts};

/// The path to the config dir
///
/// Changes depending on weather the build is for release or debug.
pub static CONFIG_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    if cfg!(debug_assertions) {
        return PathBuf::from("test");
    }

    dirs::config_dir()
        .expect("Failed to get config dir.")
        .join("dod-shell")
});
