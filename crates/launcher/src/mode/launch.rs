//! Launch mode for the launcher
//!
//! This mode allows the user to launch different applications.
//!
//! The applications are stored in JSON format in the [config directory](common::CONFIG_PATH) at `data/launch.json`.
use std::process::Command;

use crate::mode::LauncherMode;
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};

use common::{APP_CONFIG, config::launcher::LaunchApp};

/// See module level documentation
#[derive(Default)]
pub struct LaunchMode {
    /// The fuzzy matcher used to filter results
    matcher: SkimMatcherV2,
}

impl LauncherMode for LaunchMode {
    fn search(&self, query: &str) -> Vec<String> {
        if query.is_empty() {
            return Vec::new();
        }

        let mut options: Vec<LaunchApp> = APP_CONFIG.launcher.launch_mode.apps.clone();

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
            let cmd = APP_CONFIG
                .launcher
                .launch_mode
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
