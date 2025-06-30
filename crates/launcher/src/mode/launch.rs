//! Launch mode for the launcher
//!
//! This mode allows the user to launch different applications.
//!
//! The applications are stored in JSON format in the [config directory](common::CONFIG_PATH) at `data/launch.json`.
use std::{fs, process::Command};

use crate::mode::LauncherMode;
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use serde::{Deserialize, Serialize};

/// See module level documentation
pub struct LaunchMode {
    /// The fuzzy matcher used to filter results
    matcher: SkimMatcherV2,
    /// The data from the config file
    data: LaunchData,
}

impl LaunchMode {
    pub fn new() -> Self {
        LaunchMode {
            matcher: SkimMatcherV2::default(),
            data: LaunchMode::load_data().unwrap_or_default(),
        }
    }

    /// Helper function to load the data from the config file
    fn load_data() -> Result<LaunchData> {
        let path = common::CONFIG_PATH.join("data/launch.json");

        match fs::read_to_string(&path) {
            Ok(json) => serde_json::from_str(&json).map_err(|err| {
                log::error!(
                    "Failed to parse LaunchMode config at: {}. Serde Err: {}",
                    path.display(),
                    err
                );
                err.into()
            }),
            Err(e) => {
                log::error!(
                    "Failed to read LaunchMode config at: {}. Os Err: {}",
                    path.display(),
                    e
                );

                Err(e.into())
            }
        }
    }
}

impl LauncherMode for LaunchMode {
    fn search(&self, query: &str) -> Vec<String> {
        if query.is_empty() {
            return Vec::new();
        }

        let mut options: Vec<LaunchApp> = self.data.apps.clone();

        options.sort_by(|a, b| {
            self.matcher
                .fuzzy_match(&b.name, query)
                .unwrap_or_default()
                .cmp(&self.matcher.fuzzy_match(&a.name, query).unwrap_or_default())
        });

        options.into_iter().map(|o| o.name).collect()
    }

    fn finish(&self, query: &str, index: usize) {
        if let Some(result) = self.search(query).get(index) {
            let cmd = self
                .data
                .apps
                .iter()
                .find_map(|a| {
                    if a.name == *result {
                        Some(&a.cmd)
                    } else {
                        None
                    }
                })
                .expect("Result has to be in self.data.apps.");

            let mut cmd_iter = cmd.split_whitespace();

            let _ = Command::new(cmd_iter.next().unwrap())
                .args(cmd_iter.collect::<Vec<&str>>())
                .spawn();
        }
    }
}

/// Format for the config file
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
struct LaunchData {
    version: u8,
    apps: Vec<LaunchApp>,
}

/// Format for each app, that can be launched, in the config file
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
struct LaunchApp {
    name: String,
    cmd: String,
    description: Option<String>,
}

type Result<T> = std::result::Result<T, LaunchError>;

/// Errors that can occur when loading the config file
#[derive(Debug)]
pub enum LaunchError {
    #[allow(dead_code)]
    SerdeErr(serde_json::Error),
    #[allow(dead_code)]
    IoErr(std::io::Error),
}

impl From<serde_json::Error> for LaunchError {
    fn from(value: serde_json::Error) -> Self {
        Self::SerdeErr(value)
    }
}

impl From<std::io::Error> for LaunchError {
    fn from(value: std::io::Error) -> Self {
        Self::IoErr(value)
    }
}
