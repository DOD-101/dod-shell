//! Search mode for the launcher
//!
//! This mode allows the user to search for something on the web using [DuckDuckGo](https://duckduckgo.com/).
use std::process::Command;

use crate::{
    mode::{LauncherMode, NamedMode},
    results::ResultEntry,
};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};

/// See module level documentation
#[derive(Default)]
pub struct SearchMode {}

impl LauncherMode for SearchMode {
    fn search(&self, query: &str) -> Vec<ResultEntry> {
        vec![ResultEntry::new(query.to_string(), None, None)]
    }

    fn finish(&self, query: &str, _result: ResultEntry) {
        let _ = Command::new("xdg-open")
            .arg(format!(
                "https://duck.com?q={}",
                utf8_percent_encode(query, NON_ALPHANUMERIC)
            ))
            .spawn();
    }
}

impl NamedMode for SearchMode {
    fn name(&self) -> &'static str {
        "web"
    }
}
