use std::process::Command;

use crate::mode::MenuMode;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};

pub struct SearchMode {}

impl SearchMode {
    pub fn new() -> Self {
        Self {}
    }
}

impl MenuMode for SearchMode {
    fn search(&self, query: &str) -> Vec<String> {
        vec![query.to_string()]
    }

    fn finish(&self, query: &str, _index: usize) {
        let _ = Command::new("xdg-open")
            .arg(format!(
                "https://duck.com?q={}",
                utf8_percent_encode(&self.search(query)[0], NON_ALPHANUMERIC)
            ))
            .spawn();
    }
}
