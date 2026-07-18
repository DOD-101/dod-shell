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
use freedesktop_desktop_entry::{DesktopEntry, desktop_entries, get_languages_from_env};
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};

use common::config::launcher::{LaunchApp, LauncherConfig};

/// See module level documentation
pub struct LaunchMode {
    /// The fuzzy matcher used to filter results
    matcher: SkimMatcherV2,
    /// Apps configured through [`LauncherConfig`]
    apps: Vec<LaunchApp>,
    /// Executables found in the system `$PATH`
    executables: HashSet<String>,
    /// Desktop entries found on the system
    desktop_entries: Vec<DesktopEntry>,
}

impl LaunchMode {
    /// Create a new [`LaunchMode`]
    pub fn new(config: &LauncherConfig) -> Self {
        let locales = get_languages_from_env();
        let desktop_entries = desktop_entries(&locales);

        Self {
            matcher: SkimMatcherV2::default(),
            apps: config.launch_mode.apps.clone(),

            executables: path_lookup::get_executables(),
            desktop_entries,
        }
    }

    /// Generic helper method to filter results and sort them based of their fuzzy match to `query`
    fn filter_results<Items>(
        &self,
        query: &str,
        items: Items,
        mut category: ResultCategory,
    ) -> (i64, ResultCategory)
    where
        Items: Iterator<Item = ResultEntry>,
    {
        let mut options: Vec<(i64, ResultEntry)> = items
            .filter_map(|o| {
                let score = self
                    .matcher
                    .fuzzy_match(&o.label, query)
                    .unwrap_or_default();

                if score == 0 && !query.is_empty() {
                    return None;
                }

                Some((score, o))
            })
            .collect();

        options.sort_by_key(|o| o.0);

        let max_score = options.iter().map(|o| o.0).max().unwrap_or_default();

        {
            let mut guard = category.entries.guard();

            for o in options.into_iter().map(|v| v.1) {
                guard.push_back(o);
            }
        }

        (max_score, category)
    }

    /// Helper Method to filter through [`Self::apps`] returning a [`ResultCategory`]
    ///
    /// See: [`Self::filter_results`]
    fn filter_apps(&self, query: &str) -> (i64, ResultCategory) {
        let category = ResultCategory {
            name: String::from("Apps"),
            ..Default::default()
        };

        self.filter_results(
            query,
            self.apps.iter().map(|app| {
                let mut entry = ResultEntry::new(app.name.clone(), None);

                entry.data.insert("cmd".to_string(), app.cmd.clone());

                entry
            }),
            category,
        )
    }

    /// Helper method to filter through [`Self::executables`] returning a [`ResultCategory`]
    ///
    /// See: [`Self::filter_results`]
    fn filter_executables(&self, query: &str) -> (i64, ResultCategory) {
        let category = ResultCategory {
            name: String::from("Executables"),
            ..Default::default()
        };

        self.filter_results(
            query,
            self.executables.iter().map(|exe| {
                let mut entry = ResultEntry::new(exe.clone(), None);

                entry.data.insert("cmd".to_string(), exe.clone());

                entry
            }),
            category,
        )
    }

    /// Helper method to filter through [`Self::desktop_entries`] returning a [`ResultCategory`]
    ///
    /// See: [`Self::filter_results`]
    fn filter_desktop_entries(&self, query: &str) -> (i64, ResultCategory) {
        let locales = get_languages_from_env();
        let category = ResultCategory {
            name: String::from("Desktop Entries"),
            ..Default::default()
        };

        self.filter_results(
            query,
            self.desktop_entries.iter().filter_map(|de| {
                if de.no_display() || de.hidden() {
                    return None;
                }

                let name = de.name(&locales)?.into_owned();
                let exec = de.exec()?.to_owned();

                let mut entry = ResultEntry::new(name, None);
                entry.data.insert("cmd".to_string(), exec);

                Some(entry)
            }),
            category,
        )
    }
}

impl LauncherMode for LaunchMode {
    fn search(&self, query: &str) -> Vec<ResultCategory> {
        let mut categories = vec![
            self.filter_apps(query),
            self.filter_desktop_entries(query),
            self.filter_executables(query),
        ];

        categories.sort_by_key(|v| v.0);

        categories.into_iter().map(|v| v.1).collect()
    }

    fn finish(&self, _query: &str, result: &ResultEntry) {
        let mut cmd_iter = result.data.get("cmd").unwrap().split_whitespace();

        let _ = Command::new(cmd_iter.next().unwrap())
            .args(cmd_iter)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn();
    }
}

impl NamedMode for LaunchMode {
    fn name(&self) -> &'static str {
        "launch"
    }
}
