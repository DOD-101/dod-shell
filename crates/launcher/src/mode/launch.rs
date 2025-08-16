//! Launch mode for the launcher
//!
//! This mode allows the user to launch different applications.
//!
//! The applications are stored in the dod-shell config file.
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
        let apps = &APP_CONFIG.launcher.launch_mode.apps;
        if query.is_empty() {
            return apps[..(apps.len().min(APP_CONFIG.launcher.max_results))]
                .iter()
                .map(|o| o.name.clone())
                .collect();
        }

        let mut options: Vec<(i64, &LaunchApp)> = apps
            .iter()
            .filter_map(|o| {
                let score = self.matcher.fuzzy_match(&o.name, query).unwrap_or_default();

                if score == 0 {
                    return None;
                }

                Some((score, o))
            })
            .collect();

        options.sort_by_key(|o| o.0);

        options.truncate(APP_CONFIG.launcher.max_results);

        options.into_iter().map(|o| o.1.name.clone()).collect()
    }

    fn finish(&self, query: &str, index: usize) {
        let result = &self.search(query)[index];

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
