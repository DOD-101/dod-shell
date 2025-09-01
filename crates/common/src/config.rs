pub mod bar;
pub mod launcher;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Config {
    pub bar: bar::BarConfig,
    pub launcher: launcher::LauncherConfig,
}
