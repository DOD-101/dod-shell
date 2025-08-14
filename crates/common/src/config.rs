use std::{
    fs::{self, create_dir_all},
    process::exit,
    sync::LazyLock,
};

pub mod bar;
pub mod launcher;

use serde::{Deserialize, Serialize};

use crate::CONFIG_PATH;

pub static APP_CONFIG: LazyLock<Config> = LazyLock::new(load_config);

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Config {
    pub bar: bar::BarConfig,
    pub launcher: launcher::LauncherConfig,
}

/// Load the [Config] from file
///
/// ## Panics
///
/// If either the file cannot be read or cannot be parsed.
pub fn load_config() -> Config {
    let path = CONFIG_PATH.join("config.toml");

    // Ensure the config dir exists
    let _ = create_dir_all(&*CONFIG_PATH);

    if !path.try_exists().is_ok_and(|v| v) {
        let default_config = Config::default();

        if let Err(e) = fs::write(&path, toml::to_string(&default_config).unwrap()) {
            log::warn!("Failed to write the default config to file: {e}");
        } else {
            log::info!("Created default config at: {}", path.display());
        }

        return default_config;
    }

    match fs::read_to_string(path).map(|s| toml::from_str::<Config>(&s)) {
        Ok(Ok(c)) => c,
        Ok(Err(e)) => {
            log::error!("Failed to parse config: {e}");

            exit(1);
        }
        Err(e) => {
            log::error!("Failed to read config file. Using default. Io-Error: {e}");

            exit(1)
        }
    }
}
