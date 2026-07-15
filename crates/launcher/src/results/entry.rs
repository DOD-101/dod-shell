//! A single entry in the list of results
//!
//! Each result can be selected, at which point it will be returned to the [`crate::LauncherMode`].
use std::collections::HashMap;

use common::css::Class;

use relm4::{factory::CloneableFactoryComponent, prelude::*};

use crate::results::ResultCategory;

/// An individual result
#[derive(Debug, Clone, Default)]
pub struct ResultEntry {
    /// The label of the result (aka. what the user sees)
    pub label: String,
    /// Icon for the entry
    #[allow(dead_code, reason = "Needed in the next feat update")]
    pub icon: Option<&'static str>,
    /// Whether the result is currently selected
    active: bool,

    /// Additional data associated with a result entry
    ///
    /// This is arbitrary, and set by each mode individually to be used in the
    /// [`crate::mode::LauncherMode::finish`] function.
    pub data: HashMap<String, String>,
}

impl ResultEntry {
    /// Creates a new [`ResultEntry`].
    pub(crate) fn new(label: String, icon: Option<&'static str>) -> Self {
        Self {
            label,
            icon,
            active: false,

            data: HashMap::default(),
        }
    }

    /// Turn this entry into a category with only this entry
    pub(crate) fn into_category(self) -> ResultCategory {
        let mut category = ResultCategory::default();

        {
            let mut guard = category.entries.guard();

            guard.push_back(self);
        }

        category
    }
}

/// Input messages for [`ResultEntry`]
#[derive(Debug)]
pub enum ResultEntryInput {
    /// Set the [`ResultEntry`] as (in)active
    SetActive(bool),
}

/// Widget associated with the [`ResultEntry`] component
///
/// Generated with [`macro@relm4::component`].
#[relm4::factory(pub)]
impl FactoryComponent for ResultEntry {
    type Init = Self;
    type Input = ResultEntryInput;
    type Output = ();
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        /// Label for the name of the entry
        #[name(launch_option_label)]
        gtk::Label {
            set_label: &self.label,
            #[watch]
            set_class_active: (Class::Active.as_ref(), self.active),
        }
    }

    fn init_model(init: Self::Init, _index: &Self::Index, _sender: FactorySender<Self>) -> Self {
        init
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        match msg {
            ResultEntryInput::SetActive(active) => self.active = active,
        }
    }
}

impl CloneableFactoryComponent for ResultEntry {
    fn get_init(&self) -> Self::Init {
        self.clone()
    }
}
