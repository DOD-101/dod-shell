//! A single entry in the list of results
//!
//! Each result can be selected, at which point it will be returned to the [`crate::LauncherMode`].
use std::{collections::HashMap, rc::Rc};

/// An individual result
#[derive(Debug, Clone, Default)]
pub struct ResultEntry {
    /// The label of the result (aka. what the user sees)
    pub label: String,
    /// Icon for the entry
    #[allow(dead_code, reason = "Needed in the next feat update")]
    pub icon: Option<&'static str>,
    /// Category for this result
    pub category: Option<Rc<ResultCategory>>,
    /// Additional data associated with a result entry
    ///
    /// This is arbitrary, and set by each mode individually to be used in the
    /// [`crate::mode::LauncherMode::finish`] function.
    pub data: HashMap<String, String>,
}

impl ResultEntry {
    /// Creates a new [`ResultEntry`].
    pub(crate) fn new(
        label: String,
        icon: Option<&'static str>,
        category: Option<Rc<ResultCategory>>,
    ) -> Self {
        Self {
            label,
            icon,
            category,
            data: HashMap::default(),
        }
    }
}

/// A category grouping multiple [`ResultEntry`]s for display in the launcher UI.
#[derive(Debug, Clone, Default)]
pub struct ResultCategory {
    /// Display name shown to the user
    pub name: String,
    /// Icon for the category
    #[allow(dead_code, reason = "Needed in the next feat update")]
    pub icon: Option<String>,
}

impl ResultCategory {
    /// Creates a new [`ResultCategory`].
    pub fn new<S: Into<String>>(name: S, icon: Option<String>) -> Self {
        Self {
            name: name.into(),
            icon,
        }
    }
}
