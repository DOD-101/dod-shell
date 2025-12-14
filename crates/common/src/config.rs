//! Config related items
//!
//! The primary config file is located at [``crate::CONFIG_PATH``]`/config.toml`
//!
//! Layouts for the osk are located at [``crate::CONFIG_PATH``]`/layouts.json`
pub mod bar;
pub mod launcher;

pub mod layouts;

use serde::{Deserialize, Serialize};

/// Toml format of `config.toml`
#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Config {
    /// Config options relating to the bar component
    pub bar: bar::BarConfig,
    /// Config options relating to the launcher component
    pub launcher: launcher::LauncherConfig,
}
