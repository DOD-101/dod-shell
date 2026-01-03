//! Config options relating to the bar component of the shell
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// See module level documentation
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct BarConfig {
    /// Path to main disk you want information displayed about
    ///
    /// This should be a path in the format: `/dev/sda1`
    pub disk: String,
    /// Path to the device battery
    ///
    /// If this option is not set no information about the device's battery will be shown
    pub battery: Option<PathBuf>,
    /// Show if caps lock is enabled or not
    #[serde(default)]
    pub show_capslock: bool,
    /// Show if num lock is enabled or not
    #[serde(default)]
    pub show_numlock: bool,
    /// Show osk button
    #[serde(default)]
    pub show_osk_button: bool,
    /// Format for displaying the date and time
    ///
    /// See: <https://time-rs.github.io/book/api/format-description.html>
    #[serde(default = "date_time_default")]
    pub date_time_format: String,
}

/// Default for [`BarConfig::date_time_format`]
#[must_use]
pub fn date_time_default() -> String {
    "[hour]:[minute]:[second] | [year]-[month]-[day]".to_string()
}
