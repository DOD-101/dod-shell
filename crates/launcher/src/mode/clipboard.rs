//! Clipboard mode for the launcher
//!
//! This mode allows users to go through their clipboard history and pick a previous clipboard item
//! to bring back into the active clipboard.
use std::process::Command;

use crate::mode::LauncherMode;

use common::APP_CONFIG;
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};

/// See module level documentation
pub struct ClipboardMode {
    /// Clipboard history
    data: Vec<String>,
    /// The fuzzy matcher used to filter results
    matcher: SkimMatcherV2,
}

impl Default for ClipboardMode {
    fn default() -> Self {
        let cmd_output = &Command::new("cliphist")
            .arg("list")
            .output()
            .expect("cliphist should be installed")
            .stdout;

        let output = String::from_utf8_lossy(cmd_output);

        let data = output
            .lines()
            .map(|s| s.split_once("\t").unwrap().1)
            .map(String::from)
            .collect();

        let matcher = SkimMatcherV2::default();

        Self { data, matcher }
    }
}

impl LauncherMode for ClipboardMode {
    fn search(&self, query: &str) -> Vec<String> {
        if query.is_empty() {
            return self.data[..(self.data.len().min(APP_CONFIG.launcher.max_results))].into();
        }

        let mut options: Vec<(i64, String)> = self
            .data
            .iter()
            .filter_map(|o| {
                let score = self.matcher.fuzzy_match(o, query).unwrap_or_default();

                if score == 0 {
                    return None;
                }

                Some((score, o.clone()))
            })
            .collect();

        options.sort_by_key(|o| o.0);

        options.truncate(APP_CONFIG.launcher.max_results);

        options.into_iter().map(|o| o.1).collect()
    }

    fn finish(&self, query: &str, index: usize) {
        let val = &self.search(query)[index];

        let _ = Command::new("wl-copy").arg(val).spawn();
    }
}
