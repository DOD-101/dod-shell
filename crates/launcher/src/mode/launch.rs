//! Launch mode for the launcher
//!
//! This mode allows the user to launch different applications.
//!
//! The applications are stored in the dod-shell config file.
use std::{
    collections::HashSet,
    os::unix::process::CommandExt,
    process::{Command, Stdio},
};

use crate::{
    mode::{LauncherMode, NamedMode},
    results::{ResultCategory, ResultEntry},
};
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};

use common::config::launcher::{LaunchApp, LauncherConfig};

/// See module level documentation
pub struct LaunchMode {
    /// The fuzzy matcher used to filter results
    matcher: SkimMatcherV2,

    apps: Vec<LaunchApp>,

    executables: HashSet<String>,
}

impl LaunchMode {
    pub fn new(config: &LauncherConfig) -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
            apps: config.launch_mode.apps.clone(),

            executables: path_lookup::get_executables(),
        }
    }

    fn filter_apps(&self, query: &str) -> (i64, ResultCategory) {
        let mut options: Vec<(i64, &LaunchApp)> = self
            .apps
            .iter()
            .filter_map(|o| {
                let score = self.matcher.fuzzy_match(&o.name, query).unwrap_or_default();

                if score == 0 && !query.is_empty() {
                    return None;
                }

                Some((score, o))
            })
            .collect();

        options.sort_by_key(|o| o.0);

        let max_score = options.iter().map(|o| o.0).max().unwrap_or_default();

        let mut category = ResultCategory {
            name: String::from("Apps"),
            ..Default::default()
        };

        {
            let mut guard = category.entries.guard();

            for a in options.iter().map(|v| v.1.clone()) {
                let mut entry = ResultEntry::new(a.name, None);
                entry.data.insert("cmd".to_string(), a.cmd);

                guard.push_back(entry);
            }
        }

        (max_score, category)
    }

    fn filter_executables(&self, query: &str) -> (i64, ResultCategory) {
        let mut options: Vec<(i64, &String)> = self
            .executables
            .iter()
            .filter_map(|o| {
                let score = self.matcher.fuzzy_match(o, query).unwrap_or_default();

                if score == 0 && !query.is_empty() {
                    return None;
                }

                Some((score, o))
            })
            .collect();

        options.sort_by_key(|o| o.0);

        let max_score = options.iter().map(|o| o.0).max().unwrap_or_default();

        let mut category = ResultCategory {
            name: String::from("Executables"),
            ..Default::default()
        };

        {
            let mut guard = category.entries.guard();

            for a in options.iter().map(|v| v.1.clone()) {
                let mut entry = ResultEntry::new(a.clone(), None);
                entry.data.insert("cmd".to_string(), a);

                guard.push_back(entry);
            }
        }

        (max_score, category)
    }
}

impl LauncherMode for LaunchMode {
    fn search(&self, query: &str) -> Vec<ResultCategory> {
        let mut categories = vec![self.filter_apps(query), self.filter_executables(query)];

        categories.sort_by_key(|v| v.0);

        categories.into_iter().map(|v| v.1).collect()
    }

    fn finish(&self, _query: &str, result: &ResultEntry) {
        let mut cmd_iter = result.data.get("cmd").unwrap().split_whitespace();

        let _ = Command::new(cmd_iter.next().unwrap())
            .args(cmd_iter.collect::<Vec<&str>>())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn();
    }
}

impl NamedMode for LaunchMode {
    fn name(&self) -> &str {
        "launch"
    }
}
