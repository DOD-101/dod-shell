use std::{fs, path::PathBuf, sync::LazyLock};

pub mod config;

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
    match fs::read_to_string(CONFIG_PATH.join("style.scss")) {
        Ok(scss) => grass::from_string(scss, &grass::Options::default()).unwrap_or_else(|e| {
            log::error!("Failed to parse scss. Not applying any css. SassError: {e}");
            String::new()
        }),
        Err(e) => {
            log::error!("Failed to read style.scss. Not applying any css. IoError: {e}");
            String::new()
        }
    }
}
