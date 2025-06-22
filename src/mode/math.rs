use std::f64::consts::{E, PI};
use std::process::Command;

use evalexpr::{HashMapContext, context_map};

use crate::mode::MenuMode;

pub struct MathMode {
    context: HashMapContext,
}

impl MathMode {
    pub fn new() -> Self {
        Self {
            context: context_map! {
                "e" => float E,
                "pi" => float PI,
            }
            .unwrap(),
        }
    }
}

impl MenuMode for MathMode {
    fn search(&self, query: &str) -> Vec<String> {
        let query = query.trim().replacen("=", "", 1);

        if query.is_empty() {
            return vec![String::from("0")];
        }

        match evalexpr::eval_with_context(&query, &self.context) {
            Ok(val) => vec![val.to_string()],
            Err(e) => vec![e.to_string()],
        }
    }

    fn finish(&self, query: &str) {
        let result = &self.search(query)[0];

        let _ = Command::new("wl-copy").arg(result).spawn();
    }
}
