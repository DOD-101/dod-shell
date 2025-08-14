//! Config options relating to the bar component of the shell
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// See module level documentation
#[derive(Serialize, Deserialize, Default, Debug)]
pub struct BarConfig {
    /// Path to main disk you want information displayed about
    ///
    /// This should be a path in the format: `/dev/sda1`
    pub disk: PathBuf,
    /// Path to the device battery
    ///
    /// If this option is not set no information about the device's battery will be shown
    pub battery: Option<PathBuf>,
    /// Show if caps lock is enabled or not
    pub show_capslock: bool,
    /// Show if num lock is enabled or not
    pub show_numlock: bool,
}
