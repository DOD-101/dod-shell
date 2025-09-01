//! Search mode for the launcher
//!
//! This mode allows the user to search for something on the web using [DuckDuckGo](https://duckduckgo.com/).
use std::process::Command;

use crate::mode::LauncherMode;
use common::Config;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};

/// See module level documentation
#[derive(Default)]
pub struct SearchMode {}

impl LauncherMode for SearchMode {
    fn search(&self, query: &str, _config: &Config) -> Vec<String> {
        vec![query.to_string()]
    }

    fn finish(&self, query: &str, config: &Config, _index: usize) {
        let _ = Command::new("xdg-open")
            .arg(format!(
                "https://duck.com?q={}",
                utf8_percent_encode(&self.search(query, config)[0], NON_ALPHANUMERIC)
            ))
            .spawn();
    }
}
