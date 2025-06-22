mod launch;

use std::rc::Rc;

pub use launch::LaunchMode;

pub trait MenuMode {
    fn search(&self, query: &str) -> Vec<String>;
    fn finish(&self, query: &str);
}

pub struct ModePicker {
    launch: Rc<LaunchMode>,
}

impl ModePicker {
    pub fn new() -> Self {
        Self {
            launch: Rc::new(LaunchMode::new().unwrap()),
        }
    }

    pub fn pick_mode(&self, query: &str) -> Rc<dyn MenuMode> {
        return self.launch.clone();
    }
}
