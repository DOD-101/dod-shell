use relm4::{gtk::prelude::*, prelude::*};

use super::entry::{ResultEntry, ResultEntryInput};

#[derive(Debug, Clone)]
pub(crate) struct ResultCategory {
    pub name: String,
    #[allow(dead_code, reason = "Needed in the next feat update")]
    pub icon: Option<String>,
    pub entries: FactoryVecDeque<ResultEntry>,
}

impl Default for ResultCategory {
    fn default() -> Self {
        Self {
            name: String::default(),
            icon: Option::default(),
            entries: FactoryVecDeque::builder().launch_default().detach(),
        }
    }
}

impl ResultCategory {
    pub(crate) fn from_entries(entries: Vec<ResultEntry>) -> Self {
        Self {
            entries: FactoryVecDeque::from_iter(entries, gtk::Box::default()),
            ..Default::default()
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.entries.len() == 0
    }
}

#[derive(Debug)]
pub(crate) enum ResultCategoryInput {
    EntryMessage(usize, ResultEntryInput),
}

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
