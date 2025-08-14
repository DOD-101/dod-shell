//! Different modes of the launcher
//!
//! See [AllMode] for more information.
mod launch;
mod math;
mod search;

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
    fn search(&self, query: &str) -> Vec<String>;
    /// Function called when the users accepts the result
    ///
    /// `index`: The index of the result in the results list.
    fn finish(&self, query: &str, index: usize);
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
    launch: LaunchMode,
    math: MathMode,
    search: SearchMode,
}

impl AllMode {
    /// Helper function to pick the correct mode based on the prefix of the search query
    fn pick_mode<'a>(&self, query: &'a str) -> (&dyn LauncherMode, &'a str) {
        let query = query.trim();

        match query.chars().next() {
            Some('=') => (&self.math, query.strip_prefix('=').unwrap()),
            Some('?') => (&self.search, query.strip_prefix('?').unwrap()),
            _ => (&self.launch, query),
        }
    }
}

impl LauncherMode for AllMode {
    fn search(&self, query: &str) -> Vec<String> {
        let (mode, query) = self.pick_mode(query);

        mode.search(query)
    }
    fn finish(&self, query: &str, index: usize) {
        let (mode, query) = self.pick_mode(query);

        mode.finish(query, index);
    }
}
