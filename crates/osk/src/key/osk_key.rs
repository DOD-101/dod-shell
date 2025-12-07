use super::{OskKeyInputMsg, OskKeyOutputMsg, symbol::SymbolMap};
use std::rc::Rc;

use gtk::prelude::*;
use relm4::{factory::CloneableFactoryComponent, gtk, prelude::*};

type OnUp = Rc<dyn Fn(&GenericKey) -> Option<OskKeyOutputMsg>>;

#[derive(Clone)]
pub struct GenericKey {
    symbol_map: SymbolMap,

    on_up: OnUp,
}

impl std::fmt::Debug for GenericKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GenericKey")
            .field("symbol_map", &self.symbol_map)
            .finish_non_exhaustive()
    }
}

impl GenericKey {
    pub fn new(symbol_map: SymbolMap, on_up: OnUp) -> Self {
        Self { symbol_map, on_up }
    }

    fn up(&self) -> Option<OskKeyOutputMsg> {
        (self.on_up)(self)
    }
}

impl CloneableFactoryComponent for GenericKey {
    fn get_init(&self) -> Self::Init {
        self.clone()
    }
}

#[relm4::factory(pub)]
impl FactoryComponent for GenericKey {
    type Init = GenericKey;
    type Input = OskKeyInputMsg;
    type Output = OskKeyOutputMsg;
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        #[root]
        gtk::Button {
            #[watch]
            set_label: self.symbol_map.active_symb(),
            connect_clicked=> Self::Input::Clicked,
        }
    }

    fn init_model(
        init: Self::Init,
        _index: &Self::Index,
        _sender: relm4::FactorySender<Self>,
    ) -> Self {
        init
    }

    fn update(&mut self, message: Self::Input, sender: FactorySender<Self>) {
        match message {
            Self::Input::ActiveSymbol(active_symbol) => {
                self.symbol_map.set_active(active_symbol);
            }
            Self::Input::Clicked => {
                if let Some(msg) = self.up() {
                    sender.output_sender().send(msg).unwrap();
                }
            }
        }
    }
}
