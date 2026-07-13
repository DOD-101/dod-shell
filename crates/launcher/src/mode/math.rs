//! Math mode for the launcher
//!
//! This mode allows the user to do math with the help of [evalexpr](https://docs.rs/evalexpr/latest/evalexpr/).
use std::f64::consts::{E, PI};
use std::process::Command;

use evalexpr::{HashMapContext, context_map};

use crate::mode::{LauncherMode, NamedMode};
use crate::results::{ResultCategory, ResultEntry};

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
    fn search(&self, query: &str) -> Vec<ResultCategory> {
        if query.is_empty() {
            return vec![ResultEntry::new(String::from("0"), None).into_category()];
        }

        let res = match evalexpr::eval_with_context(query, &self.context) {
            Ok(val) => val.to_string(),
            Err(e) => e.to_string(),
        };

        vec![ResultEntry::new(res, None).into_category()]
    }

    fn finish(&self, _query: &str, result: &ResultEntry) {
        let _ = Command::new("wl-copy").arg(result.label.clone()).spawn();
    }
}

impl NamedMode for MathMode {
    fn name(&self) -> &str {
        "math"
    }
}
