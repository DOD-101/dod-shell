//! Launch mode for the launcher
//!
//! This mode allows the user to launch different applications.
//!
//! The applications are stored in the dod-shell config file.
use std::{
    collections::HashSet,
    process::{Command, Stdio},
    rc::Rc,
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
    apps: Box<[LaunchApp]>,
    /// Executables found in the system `$PATH`
    executables: HashSet<String>,
    /// Desktop entries found on the system
    desktop_entries: Box<[DesktopEntry]>,
}

impl LaunchMode {
    /// Create a new [`LaunchMode`]
    pub fn new(config: &LauncherConfig) -> Self {
        let locales = get_languages_from_env();
        let desktop_entries = desktop_entries(&locales).into_boxed_slice();

        Self {
            matcher: SkimMatcherV2::default(),
            apps: config.launch_mode.apps.clone().into_boxed_slice(),

            executables: path_lookup::get_executables(),
            desktop_entries,
        }
    }

    /// Generic helper method to filter results and sort them based of their fuzzy match to `query`
    fn filter_results<Items>(&self, query: &str, items: Items) -> Vec<ResultEntry>
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

        options.sort_unstable_by_key(|o| std::cmp::Reverse(o.0));

        options.into_iter().map(|o| o.1).collect()
    }

    /// Helper method to filter through [`Self::apps`]
    ///
    /// See: [`Self::filter_results`]
    fn filter_apps(&self, query: &str) -> Vec<ResultEntry> {
        let category = Rc::new(ResultCategory::new("Apps", None));

        self.filter_results(
            query,
            self.apps.iter().map(|app| {
                let mut entry = ResultEntry::new(app.name.clone(), None, Some(category.clone()));

                entry.data.insert("cmd".to_string(), app.cmd.clone());

                entry
            }),
        )
    }

    /// Helper method to filter through [`Self::executables`]
    ///
    /// See: [`Self::filter_results`]
    fn filter_executables(&self, query: &str) -> Vec<ResultEntry> {
        let category = Rc::new(ResultCategory::new("Executables", None));

        self.filter_results(
            query,
            self.executables.iter().map(|exe| {
                let mut entry = ResultEntry::new(exe.clone(), None, Some(category.clone()));

                entry.data.insert("cmd".to_string(), exe.clone());

                entry
            }),
        )
    }

    /// Helper method to filter through [`Self::desktop_entries`]
    ///
    /// See: [`Self::filter_results`]
    fn filter_desktop_entries(&self, query: &str) -> Vec<ResultEntry> {
        let locales = get_languages_from_env();
        let category = Rc::new(ResultCategory::new("Desktop Entries", None));

        self.filter_results(
            query,
            self.desktop_entries.iter().filter_map(|de| {
                if de.no_display() || de.hidden() {
                    return None;
                }

                let name = de.name(&locales)?.into_owned();
                let exec = de.exec()?.to_owned();

                let mut entry = ResultEntry::new(name, None, Some(category.clone()));
                entry.data.insert("cmd".to_string(), exec);

                Some(entry)
            }),
        )
    }
}

impl LauncherMode for LaunchMode {
    fn search(&self, query: &str) -> Vec<ResultEntry> {
        vec![
            self.filter_apps(query),
            self.filter_desktop_entries(query),
            self.filter_executables(query),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    fn finish(&self, _query: &str, result: ResultEntry) {
        let mut cmd_iter = result.data.get("cmd").unwrap().split_whitespace();
        let _ = Command::new("systemd-run")
            .args(["--user", "--scope", "--collect", "--quiet"])
            .arg(cmd_iter.next().unwrap())
            .args(cmd_iter)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
}

impl NamedMode for LaunchMode {
    fn name(&self) -> &'static str {
        "launch"
    }
}
