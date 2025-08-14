use std::{path::PathBuf, sync::LazyLock};

pub mod config;
pub mod types;

pub use config::load_config;

pub static CONFIG_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    if cfg!(debug_assertions) {
        return PathBuf::from("test");
    }

    dirs::config_dir()
        .expect("Failed to get config dir.")
        .join("dod-shell")
});

pub fn get_css() -> String {
    grass::from_path(CONFIG_PATH.join("style.scss"), &grass::Options::default()).unwrap_or_else(
        |e| {
            log::error!("Failed to parse scss. Not applying any css. SassError: {e}");
            String::new()
        },
    )
}
