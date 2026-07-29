//! Config options relating to the launcher component of the shell
use serde::{Deserialize, Serialize};

/// See module level documentation
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct LauncherConfig {
    /// Max height for the list of results
    #[serde(default = "results_height_default")]
    pub results_height: i32,
    /// If results (and category headers) should be centered
    #[serde(default)]
    pub center_results: bool,
    #[serde(default)]
    /// See [``LaunchModeConfig``]
    pub launch_mode: LaunchModeConfig,
}

/// Config relating to the `Launch` mode of the launcher
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct LaunchModeConfig {
    /// All apps the launcher will show
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

/// Default for [`LauncherConfig::results_height`]
const fn results_height_default() -> i32 {
    400
}
