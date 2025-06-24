mod launch;
mod math;
mod search;

pub use launch::LaunchMode;
pub use math::MathMode;
pub use search::SearchMode;

pub trait MenuMode {
    fn search(&self, query: &str) -> Vec<String>;
    fn finish(&self, query: &str);
}

pub struct AllMode {
    launch: LaunchMode,
    math: MathMode,
    search: SearchMode,
}

impl AllMode {
    pub fn new() -> Self {
        Self {
            launch: LaunchMode::new(),
            math: MathMode::new(),
            search: SearchMode::new(),
        }
    }

    fn pick_mode<'a>(&self, query: &'a str) -> (&dyn MenuMode, &'a str) {
        let query = query.trim();

        match query.chars().next() {
            Some('=') => (&self.math, query.strip_prefix('=').unwrap()),
            Some('?') => (&self.search, query.strip_prefix('?').unwrap()),
            _ => (&self.launch, query),
        }
    }
}

impl MenuMode for AllMode {
    fn search(&self, query: &str) -> Vec<String> {
        let (mode, query) = self.pick_mode(query);

        mode.search(query)
    }
    fn finish(&self, query: &str) {
        let (mode, query) = self.pick_mode(query);

        mode.finish(query);
    }
}
