//! Math mode for the launcher
//!
//! This mode allows the user to do math with the help of [evalexpr](https://docs.rs/evalexpr/latest/evalexpr/).
use std::f64::consts::{E, PI};
use std::process::Command;

use evalexpr::{HashMapContext, context_map};

use crate::mode::LauncherMode;

/// See crate level documentation
pub struct MathMode {
    context: HashMapContext,
}

impl Default for MathMode {
    fn default() -> Self {
        Self {
            context: context_map! {
                "e" => float E,
                "pi" => float PI,
            }
            .unwrap(),
        }
    }
}

impl LauncherMode for MathMode {
    fn search(&self, query: &str) -> Vec<String> {
        if query.is_empty() {
            return vec![String::from("0")];
        }

        match evalexpr::eval_with_context(query, &self.context) {
            Ok(val) => vec![val.to_string()],
            Err(e) => vec![e.to_string()],
        }
    }

    fn finish(&self, query: &str, _index: usize) {
        let result = &self.search(query)[0];

        let _ = Command::new("wl-copy").arg(result).spawn();
    }
}
