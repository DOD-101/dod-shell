pub mod code_resolve;
pub mod symbol;

use strum::EnumIs;
use symbol::SymbolMap;

use gtk::prelude::*;
use relm4::{factory::CloneableFactoryComponent, gtk, prelude::*};

use crate::key::symbol::ActiveSymbol;

#[derive(Debug, Clone, Default)]
pub struct GenericKey {
    symbol_map: SymbolMap,

    key_type: GenericKeyType,
}

impl CloneableFactoryComponent for GenericKey {
    fn get_init(&self) -> Self::Init {
        self.clone()
    }
}

impl GenericKey {
    pub fn builder() -> GenericKeyBuilder {
        GenericKeyBuilder {
            symbol_map: SymbolMap::default(),
            key_type: GenericKeyType::default(),
        }
    }
}

pub struct GenericKeyBuilder {
    symbol_map: SymbolMap,

    key_type: GenericKeyType,
}

#[derive(Default, Debug, Clone, EnumIs)]
pub enum GenericKeyType {
    #[default]
    Utf,
    Code(u32),
    Mod(daemon::osk::Mod),
    /// Bool indicates shift-lock
    Shift,
    NonKey,
}

impl GenericKeyBuilder {
    pub fn key_type(mut self, key_type: GenericKeyType) -> Self {
        self.key_type = key_type;

        self
    }

    pub fn symbol_map(mut self, symbol_map: SymbolMap) -> Self {
        self.symbol_map = symbol_map;

        self
    }

    pub fn build(self) -> GenericKey {
        GenericKey {
            symbol_map: self.symbol_map,

            key_type: self.key_type,
        }
    }
}

#[derive(Debug)]
pub enum GenericKeyMsg {
    Utf(String),
    Code(u32),
    Mod(daemon::osk::Mod),
    Shift,
}

#[derive(Debug, Clone)]
pub struct OskRow(FactoryVecDeque<GenericKey>);

#[relm4::factory(pub)]
impl FactoryComponent for OskRow {
    type Init = FactoryVecDeque<GenericKey>;
    type Input = GenericKeyInputMsg;
    type Output = GenericKeyMsg;
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        #[root]
        gtk::Box {
            #[local_ref]
            row -> gtk::Box {
                set_align: gtk::Align::Center,
                add_css_class: "osk-row",
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

#[derive(Debug, Copy, Clone)]
pub enum GenericKeyInputMsg {
    ActiveSymbol(ActiveSymbol),
    Clicked,
}

#[relm4::factory(pub)]
impl FactoryComponent for GenericKey {
    type Init = GenericKey;
    type Input = GenericKeyInputMsg;
    type Output = GenericKeyMsg;
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
                let output_msg = match self.key_type {
                    GenericKeyType::Utf => Some(GenericKeyMsg::Utf(
                        self.symbol_map.active_symb().to_string(),
                    )),
                    GenericKeyType::Code(v) => Some(GenericKeyMsg::Code(v)),
                    GenericKeyType::Mod(v) => Some(GenericKeyMsg::Mod(v)),
                    GenericKeyType::Shift => Some(GenericKeyMsg::Shift),
                    GenericKeyType::NonKey => None,
                };

                if let Some(msg) = output_msg {
                    sender.output_sender().send(msg).unwrap();
                }
            }
        }
    }
}
