//! Config options relating to the launcher component of the shell
use serde::{Deserialize, Serialize};

/// See module level documentation
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct LauncherConfig {
    /// See [``LaunchModeConfig``]
    pub launch_mode: LaunchModeConfig,
}

/// Config relating to the `Launch` mode of the launcher
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct LaunchModeConfig {
    pub apps: Vec<LaunchApp>,
}

/// Format for each app, that can be launched
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct LaunchApp {
    /// Name of the app
    ///
    /// This will be displayed for the user and is the main way to search for
    /// an app (subject to change in the future)
    pub name: String,
    /// Command run to launch the app, if selected
    pub cmd: String,

    /// A longer description of the app
    ///
    /// Not currently used
    pub description: Option<String>,
}
