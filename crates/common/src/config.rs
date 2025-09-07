//! Toml config related items
//!
//! The config file is located at [``crate::CONFIG_PATH``]`/config.toml`
pub mod bar;
pub mod launcher;

use serde::{Deserialize, Serialize};

/// The toml format which is (de-)serialized by serde
#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Config {
    pub bar: bar::BarConfig,
    pub launcher: launcher::LauncherConfig,
}
