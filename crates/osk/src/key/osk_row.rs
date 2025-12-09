use super::{GenericKey, OskKeyInputMsg, OskKeyOutputMsg};
use gtk::prelude::*;
use relm4::{gtk, prelude::*};

use common::css::Class;

#[derive(Debug, Clone)]
pub struct OskRow(FactoryVecDeque<GenericKey>);

#[relm4::factory(pub)]
impl FactoryComponent for OskRow {
    type Init = FactoryVecDeque<GenericKey>;
    type Input = OskKeyInputMsg;
    type Output = OskKeyOutputMsg;
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        #[root]
        gtk::Box {
            #[local_ref]
            row -> gtk::Box {
                set_align: gtk::Align::Center,
                add_css_class: Class::OskRow.as_ref(),
                set_hexpand: true,
            }
        }
    }

    fn init_widgets(
        &mut self,
        _index: &Self::Index,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as relm4::factory::FactoryView>::ReturnedWidget,
        _sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let row = self.0.widget();

        let widgets = view_output!();

        widgets
    }

    fn init_model(init: Self::Init, _index: &Self::Index, _sender: FactorySender<Self>) -> Self {
        Self(init)
    }

    fn update(&mut self, message: Self::Input, _sender: FactorySender<Self>) {
        let max_index = self.0.len();

        for i in 0..max_index {
            self.0.send(i, message);
        }
    }
}
