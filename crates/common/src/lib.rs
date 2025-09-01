use std::{path::PathBuf, sync::LazyLock};

pub mod config;
pub mod types;

pub use config::Config;

pub static CONFIG_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    if cfg!(debug_assertions) {
        return PathBuf::from("test");
    }

    dirs::config_dir()
        .expect("Failed to get config dir.")
        .join("dod-shell")
});
