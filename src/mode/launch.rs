use std::{fs, path::Path, process::Command};

use crate::mode::MenuMode;
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use serde::{Deserialize, Serialize};
use serde_json::Result;

pub struct LaunchMode {
    matcher: SkimMatcherV2,
    data: LaunchData,
}

impl LaunchMode {
    pub fn new() -> Result<Self> {
        Ok(LaunchMode {
            matcher: SkimMatcherV2::default(),
            data: LaunchMode::load_data()?,
        })
    }

    fn load_data() -> Result<LaunchData> {
        #[cfg(debug_assertions)]
        let path = Path::new("test/data/launch.json").to_path_buf();

        #[cfg(not(debug_assertions))]
        let path = dirs::config_dir()
            .unwrap()
            .join(Path::new("dod-shell/data/launch.json"));

        let data = fs::read_to_string(path).unwrap();

        return serde_json::from_str(&data);
    }
}

impl MenuMode for LaunchMode {
    fn search(&self, query: &str) -> Vec<String> {
        let mut options: Vec<LaunchApp> = self.data.apps.clone();

        options.sort_by(|a, b| {
            self.matcher
                .fuzzy_match(&b.name, &query)
                .unwrap_or_default()
                .cmp(
                    &self
                        .matcher
                        .fuzzy_match(&a.name, &query)
                        .unwrap_or_default(),
                )
        });

        return options.into_iter().map(|o| o.name).collect();
    }

    fn finish(&self, query: &str) {
        self.search(query);

        let cmd = self
            .data
            .apps
            .get(0)
            .map_or(String::new(), |o| o.cmd.clone());

        let mut cmd_iter = cmd.split_whitespace();

        let cmd = Command::new(cmd_iter.next().unwrap())
            .args(cmd_iter.collect::<Vec<&str>>())
            .spawn();

        print!("{:#?}", cmd)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct LaunchData {
    version: u8,
    apps: Vec<LaunchApp>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct LaunchApp {
    name: String,
    cmd: String,
    description: Option<String>,
}
