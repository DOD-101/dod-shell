use std::{fs, process::Command};

use crate::mode::MenuMode;
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use serde::{Deserialize, Serialize};

pub struct LaunchMode {
    matcher: SkimMatcherV2,
    data: LaunchData,
}

impl LaunchMode {
    pub fn new() -> Self {
        LaunchMode {
            matcher: SkimMatcherV2::default(),
            data: LaunchMode::load_data().unwrap_or_default(),
        }
    }

    fn load_data() -> Result<LaunchData> {
        let path = crate::CONFIG_PATH.join("data/launch.json");

        match fs::read_to_string(&path) {
            Ok(json) => serde_json::from_str(&json).map_err(|err| {
                log::error!(
                    "Failed to parse LaunchMode config at: {}. Serde Err: {}",
                    path.display(),
                    err
                );
                err.into()
            }),
            Err(e) => {
                log::error!(
                    "Failed to read LaunchMode config at: {}. Os Err: {}",
                    path.display(),
                    e
                );

                Err(e.into())
            }
        }
    }
}

impl MenuMode for LaunchMode {
    fn search(&self, query: &str) -> Vec<String> {
        if query.is_empty() {
            return Vec::new();
        }

        let mut options: Vec<LaunchApp> = self.data.apps.clone();

        options.sort_by(|a, b| {
            self.matcher
                .fuzzy_match(&b.name, query)
                .unwrap_or_default()
                .cmp(&self.matcher.fuzzy_match(&a.name, query).unwrap_or_default())
        });

        options.into_iter().map(|o| o.name).collect()
    }

    fn finish(&self, query: &str) {
        if let Some(result) = self.search(query).first() {
            let cmd = self
                .data
                .apps
                .iter()
                .find_map(|a| {
                    if a.name == *result {
                        Some(&a.cmd)
                    } else {
                        None
                    }
                })
                .expect("Result has to be in self.data.apps.");

            let mut cmd_iter = cmd.split_whitespace();

            let _ = Command::new(cmd_iter.next().unwrap())
                .args(cmd_iter.collect::<Vec<&str>>())
                .spawn();
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
struct LaunchData {
    version: u8,
    apps: Vec<LaunchApp>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
struct LaunchApp {
    name: String,
    cmd: String,
    description: Option<String>,
}

type Result<T> = std::result::Result<T, LaunchError>;

#[derive(Debug)]
pub enum LaunchError {
    #[allow(dead_code)]
    SerdeErr(serde_json::Error),
    #[allow(dead_code)]
    IoErr(std::io::Error),
}

impl From<serde_json::Error> for LaunchError {
    fn from(value: serde_json::Error) -> Self {
        Self::SerdeErr(value)
    }
}

impl From<std::io::Error> for LaunchError {
    fn from(value: std::io::Error) -> Self {
        Self::IoErr(value)
    }
}
