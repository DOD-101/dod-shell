//! Different modes of the launcher
//!
//! See [AllMode] for more information.
mod clipboard;
mod launch;
mod math;
mod search;

use std::{cell::LazyCell, sync::Mutex};

pub use clipboard::ClipboardMode;
use common::config::launcher::LauncherConfig;
pub use launch::LaunchMode;
pub use math::MathMode;
pub use search::SearchMode;

/// Trait representing a mode of the launcher
///
/// See the module level documentation for more information.
pub trait LauncherMode {
    /// Function called each time the search query is updated
    ///
    /// Returns a list of results.
    fn search(&self, query: &str, config: &LauncherConfig) -> Vec<String>;
    /// Function called when the users accepts the result
    ///
    /// `index`: The index of the result in the results list.
    fn finish(&self, query: &str, config: &LauncherConfig, index: usize);
}

/// Trait representing a mode that has a name
trait NamedMode: LauncherMode {
    /// The Display name of the mode
    fn name(&self) -> &str;
}

/// The default mode of the Launcher
///
/// By itself this mode doesn't do anything, but allows the selection of other modes via prefixes.
///
/// ## Performance
///
/// Since the creation of this creates instances of all other modes, it should only be called once in
/// the lifetime of the application. Dropping the returned [AllMode] will therefore also loose
/// all state of the other modes.
#[derive(Default)]
pub struct AllMode {
    launch: LazyCell<LaunchMode>,
    math: LazyCell<MathMode>,
    search: LazyCell<SearchMode>,
    clipboard: LazyCell<ClipboardMode>,
    /// Name of the this mode for [function@name]
    ///
    /// Since AllMode is not really a mode in itself, we set this value to whatever the name of the
    /// last used mode in via the search method.
    name: Mutex<String>,
}

impl AllMode {
    /// Helper function to pick the correct mode based on the prefix of the search query
    fn pick_mode<'a>(&self, query: &'a str) -> (&dyn NamedMode, &'a str) {
        let query = query.trim();

        match query.chars().next() {
            Some('=') => (&*self.math, query.strip_prefix('=').unwrap()),
            Some('?') => (&*self.search, query.strip_prefix('?').unwrap()),
            Some('&') => (&*self.clipboard, query.strip_prefix('&').unwrap()),
            _ => (&*self.launch, query),
        }
    }

    pub fn current_name(&self) -> String {
        self.name.lock().unwrap().clone()
    }
}

impl LauncherMode for AllMode {
    fn search(&self, query: &str, config: &LauncherConfig) -> Vec<String> {
        let (mode, query) = self.pick_mode(query);

        *self.name.lock().unwrap() = mode.name().to_string();

        mode.search(query, config)
    }
    fn finish(&self, query: &str, config: &LauncherConfig, index: usize) {
        let (mode, query) = self.pick_mode(query);

        mode.finish(query, config, index);
    }
}
