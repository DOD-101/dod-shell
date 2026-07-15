//! Clipboard mode for the launcher
//!
//! This mode allows users to go through their clipboard history and pick a previous clipboard item
//! to bring back into the active clipboard.
use std::process::{Command, Stdio};

use crate::{
    mode::{LauncherMode, NamedMode},
    results::{ResultCategory, ResultEntry},
};
use std::iter::Iterator;

use fuzzy_matcher::{
    FuzzyMatcher,
    skim::{SkimMatcherV2, SkimScoreConfig},
};

/// See module level documentation
pub struct ClipboardMode {
    /// Clipboard history
    ///
    /// String: the display name / content of the entry. What is shown to the user
    /// u32: the index of the entry used to retrieve it later from cliphist
    ///
    /// Both these values are gotten by parsing the output of `cliphist list`.
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
            .map(|s| s.split_once('\t').unwrap())
            .map(|v| (v.1.to_string(), v.0.parse().unwrap()))
            .collect();

        let config = SkimScoreConfig {
            bonus_consecutive: 10,
            bonus_head: 10,
            ..Default::default()
        };

        let matcher = SkimMatcherV2::default().score_config(config);

        Self { data, matcher }
    }
}

impl LauncherMode for ClipboardMode {
    fn search(&self, query: &str) -> Vec<ResultCategory> {
        if query.is_empty() {
            return vec![
                self.data
                    .iter()
                    .cloned()
                    .map(|d| {
                        let mut entry = ResultEntry::new(d.0, None);

                        entry.data.insert("id".to_string(), d.1.to_string());

                        entry
                    })
                    .collect::<ResultCategory>(),
            ];
        }

        let keys = self.data.iter().map(|v| &v.0);

        let mut options: Vec<(i64, String)> = keys
            .enumerate()
            .filter_map(|(i, o)| {
                let score = self.matcher.fuzzy_match(o, query).unwrap_or_default();

                if score == 0 {
                    return None;
                }

                // multiply by the position to take into account the historical order
                #[allow(
                    clippy::cast_possible_wrap,
                    reason = "The index will never be i64::MAX."
                )]
                Some((score * i as i64, o.clone()))
            })
            .collect();

        options.sort_by_key(|o| o.0);

        vec![
            options
                .into_iter()
                .map(|d| ResultEntry::new(d.1, None))
                .collect::<ResultCategory>(),
        ]
    }

    fn finish(&self, _query: &str, result: &ResultEntry) {
        let id = result.data.get("id").unwrap();

        let mut child1 = Command::new("cliphist")
            .arg("decode")
            .arg(id)
            .stdout(Stdio::piped())
            .spawn()
            .expect("failed to spawn cliphist");

        let mut child2 = Command::new("wl-copy")
            .stdin(child1.stdout.take().unwrap())
            .spawn()
            .expect("failed to spawn wl-copy");

        if !(child1.wait().is_ok_and(|v| v.success()) && child2.wait().is_ok_and(|v| v.success())) {
            print!("ERROR: Failed to copy to clipboard.");
        }
        // if the result is none we just exit, the assumption being there were no valid results
    }
}

impl NamedMode for ClipboardMode {
    fn name(&self) -> &'static str {
        "clipboard"
    }
}
