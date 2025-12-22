//! Config related items
//!
//! The primary config file is located at <code>[crate::CONFIG_PATH]/config.toml</code>
//!
//! Layouts for the osk are located at <code>[crate::CONFIG_PATH]/layouts.json</code>
pub mod bar;
pub mod launcher;

pub mod layouts;

use serde::{Deserialize, Serialize};

/// Toml format of `config.toml`
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Config {
    /// Config options relating to the bar component
    pub bar: bar::BarConfig,
    /// Config options relating to the launcher component
    pub launcher: launcher::LauncherConfig,
}
