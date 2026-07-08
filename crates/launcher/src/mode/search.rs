//! Search mode for the launcher
//!
//! This mode allows the user to search for something on the web using [DuckDuckGo](https://duckduckgo.com/).
use std::process::Command;

use crate::mode::{LauncherMode, NamedMode};
use common::config::launcher::LauncherConfig;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};

/// See module level documentation
#[derive(Default)]
pub struct SearchMode {}

impl LauncherMode for SearchMode {
    fn search(&self, query: &str, _config: &LauncherConfig) -> Vec<String> {
        vec![query.to_string()]
    }

    fn finish(&self, query: &str, config: &LauncherConfig, _index: usize) {
        let _ = Command::new("xdg-open")
            .arg(format!(
                "https://duck.com?q={}",
                utf8_percent_encode(&self.search(query, config)[0], NON_ALPHANUMERIC)
            ))
            .spawn();
    }
}

impl NamedMode for SearchMode {
    fn name(&self) -> &str {
        "web"
    }
}
