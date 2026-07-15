//! Different modes of the launcher
//!
//! See [`AllMode`] for more information.
mod clipboard;
mod launch;
mod math;
mod search;

use std::{
    cell::{LazyCell, OnceCell},
    sync::Mutex,
};

pub use clipboard::ClipboardMode;
use common::config::launcher::LauncherConfig;
pub use launch::LaunchMode;
pub use math::MathMode;
pub use search::SearchMode;

use crate::results::{ResultCategory, ResultEntry};

/// Trait representing a mode of the launcher
///
/// See the module level documentation for more information.
pub trait LauncherMode {
    /// Function called each time the search query is updated
    ///
    /// Returns a list of results.
    fn search(&self, query: &str) -> Vec<ResultCategory>;
    /// Function called when the users accepts the result
    ///
    /// `index`: The index of the result in the results list.
    fn finish(&self, query: &str, result: &ResultEntry);
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
/// Since this mode holds the state of all other modes for the application it should only be created
/// once during the life cycle of the application. Creation however doesn't take long, since the
/// creation of the individual modes is done lazily on fist use.
pub struct AllMode {
    /// See: [`LaunchMode`]
    launch: OnceCell<LaunchMode>,
    /// See: [`MathMode`]
    math: LazyCell<MathMode>,
    /// See: [`SearchMode`]
    search: LazyCell<SearchMode>,
    /// See: [`ClipboardMode`]
    clipboard: LazyCell<ClipboardMode>,
    /// Name of the this mode for [function@name]
    ///
    /// Since `AllMode` is not really a mode in itself, we set this value to whatever the name of the
    /// last used mode in via the search method.
    name: Mutex<String>,

    /// Config passed on creation of [`Self::launch`]
    config: LauncherConfig,
}

impl AllMode {
    /// Creates a new [`AllMode`]
    pub fn new(config: LauncherConfig) -> Self {
        Self {
            launch: OnceCell::default(),
            math: LazyCell::default(),
            search: LazyCell::default(),
            clipboard: LazyCell::default(),
            name: Mutex::default(),
            config,
        }
    }

    /// Helper function to pick the correct mode based on the prefix of the search query
    fn pick_mode<'a>(&self, query: &'a str) -> (&dyn NamedMode, &'a str) {
        let query = query.trim();

        match query.chars().next() {
            Some('=') => (&*self.math, query.strip_prefix('=').unwrap()),
            Some('?') => (&*self.search, query.strip_prefix('?').unwrap()),
            Some('&') => (&*self.clipboard, query.strip_prefix('&').unwrap()),
            _ => (
                self.launch
                    .get_or_init(move || LaunchMode::new(&self.config)),
                query,
            ),
        }
    }

    /// Returns the name of the current mode
    pub fn current_name(&self) -> String {
        self.name.lock().unwrap().clone()
    }
}

impl LauncherMode for AllMode {
    fn search(&self, query: &str) -> Vec<ResultCategory> {
        let (mode, query) = self.pick_mode(query);

        *self.name.lock().unwrap() = mode.name().to_string();

        mode.search(query)
    }
    fn finish(&self, query: &str, result: &ResultEntry) {
        let (mode, query) = self.pick_mode(query);

        mode.finish(query, result);
    }
}
