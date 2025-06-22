mod launch;
mod math;

use std::rc::Rc;

pub use launch::LaunchMode;
pub use math::MathMode;

pub trait MenuMode {
    fn search(&self, query: &str) -> Vec<String>;
    fn finish(&self, query: &str);
}

pub struct ModePicker {
    launch: Rc<LaunchMode>,
    math: Rc<MathMode>,
}

impl ModePicker {
    pub fn new() -> Self {
        Self {
            launch: Rc::new(LaunchMode::new().unwrap()),
            math: Rc::new(MathMode::new()),
        }
    }

    pub fn pick_mode(&self, query: &str) -> Rc<dyn MenuMode> {
        match query.chars().next() {
            // HACK: I dislike the disconnect between deciding
            // the mode here via the first char, but then removing
            // that first char in the Mode itself instead of in the picker
            Some(c) if c == '=' => self.math.clone(),
            _ => self.launch.clone(),
        }
    }
}
