//! Clipboard mode for the launcher
//!
//! This mode allows users to go through their clipboard history and pick a previous clipboard item
//! to bring back into the active clipboard.
use std::process::{Command, Stdio};

use crate::mode::LauncherMode;

use common::config::launcher::LauncherConfig;
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};

/// See module level documentation
pub struct ClipboardMode {
    /// Clipboard history
    data: Vec<(String, u32)>,
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
            .map(|s| s.split_once("\t").unwrap())
            .map(|v| (v.1.to_string(), v.0.parse().unwrap()))
            .collect();

        let matcher = SkimMatcherV2::default();

        Self { data, matcher }
    }
}

impl LauncherMode for ClipboardMode {
    fn search(&self, query: &str, config: &LauncherConfig) -> Vec<String> {
        if query.is_empty() {
            return self
                .data
                .iter()
                .take(config.max_results)
                .map(|v| &v.0)
                .cloned()
                .collect();
        }

        let keys = self.data.iter().map(|v| &v.0);

        let mut options: Vec<(i64, String)> = keys
            .filter_map(|o| {
                let score = self.matcher.fuzzy_match(o, query).unwrap_or_default();

                if score == 0 {
                    return None;
                }

                Some((score, o.clone()))
            })
            .collect();

        options.sort_by_key(|o| o.0);

        options.truncate(config.max_results);

        options.into_iter().map(|o| o.1).collect()
    }

    fn finish(&self, query: &str, config: &LauncherConfig, index: usize) {
        let results = &self.search(query, config);
        let val = results.get(index);

        if let Some(val) = val {
            let code = self
                .data
                .iter()
                .find_map(|v| if v.0 == *val { Some(v.1) } else { None })
                .expect("Value should be valid since it was just returned by search.");

            let mut child1 = Command::new("cliphist")
                .arg("decode")
                .arg(code.to_string())
                .stdout(Stdio::piped())
                .spawn()
                .expect("failed to spawn cliphist");

            let mut child2 = Command::new("wl-copy")
                .stdin(child1.stdout.take().unwrap())
                .spawn()
                .expect("failed to spawn wl-copy");

            if !(child1.wait().is_ok_and(|v| v.success())
                && child2.wait().is_ok_and(|v| v.success()))
            {
                print!("ERROR: Failed to copy to clipboard.")
            }
        }
        // if the result is none we just exit, the assumption being there were no valid results
    }
}
