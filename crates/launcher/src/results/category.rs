//! A single category within the launcher results list.
//!
//! See: [`ResultCategory`]

use relm4::{gtk::prelude::*, prelude::*};

use super::entry::{ResultEntry, ResultEntryInput};

/// A category grouping multiple [`ResultEntry`]s for display in the launcher UI.
#[derive(Debug, Clone)]
pub struct ResultCategory {
    /// Display name shown to the user
    pub name: String,
    /// Icon for the category
    #[allow(dead_code, reason = "Needed in the next feat update")]
    pub icon: Option<String>,
    /// Entries in this category
    pub entries: FactoryVecDeque<ResultEntry>,
}

impl Default for ResultCategory {
    /// Creates an empty category with default values (no name, no icon, no entries).
    fn default() -> Self {
        Self {
            name: String::default(),
            icon: Option::default(),
            entries: FactoryVecDeque::builder().launch_default().detach(),
        }
    }
}

impl FromIterator<ResultEntry> for ResultCategory {
    fn from_iter<T: IntoIterator<Item = ResultEntry>>(iter: T) -> Self {
        Self {
            entries: FactoryVecDeque::from_iter(iter, gtk::Box::default()),
            ..Default::default()
        }
    }
}

impl ResultCategory {
    /// Returns the number of entries in this category.
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if this category contains no entries.
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.len() == 0
    }
}

/// Input messages for [`ResultCategory`].
#[derive(Debug)]
pub enum ResultCategoryInput {
    /// Forwards a message to a specific entry within this category.
    EntryMessage(usize, ResultEntryInput),
}

/// Widget associated with the [`ResultCategory`] component.
#[relm4::factory(pub)]
impl FactoryComponent for ResultCategory {
    type Init = Self;
    type Input = ResultCategoryInput;
    type Output = ();
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            gtk::Label {
                set_visible: !self.name.is_empty(),
                set_label: &self.name,
            },

            /// Entries for this category
            #[local_ref]
            entries -> gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
            }
        }
    }

    fn init_model(init: Self::Init, _index: &Self::Index, _sender: FactorySender<Self>) -> Self {
        init
    }

    fn init_widgets(
        &mut self,
        _index: &Self::Index,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as relm4::factory::FactoryView>::ReturnedWidget,
        _sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let entries = self.entries.widget();

        let widgets = view_output!();

        widgets
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        match msg {
            ResultCategoryInput::EntryMessage(index, entry_msg) => {
                self.entries.send(index, entry_msg);
            }
        }
    }
}
