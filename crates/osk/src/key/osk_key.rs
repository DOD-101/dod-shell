use crate::ShiftState;

use super::{
    OskKeyInputMsg, OskKeyOutputMsg,
    symbol::{ActiveSymbol, SymbolMap},
};
use std::rc::Rc;

use common::css::{Class, ClassList};
use gtk::prelude::*;
use relm4::{factory::CloneableFactoryComponent, gtk, prelude::*};

type OnUp = Rc<dyn Fn(&GenericKey) -> Option<OskKeyOutputMsg>>;
// NOTE: Currently the OutputMsg isn't used here, but in the future it will be
type OnDown = Rc<dyn Fn(&mut GenericKey) -> Option<OskKeyOutputMsg>>;

#[derive(Clone)]
pub struct GenericKey {
    symbol_map: SymbolMap,

    on_down: OnDown,
    on_up: OnUp,

    css_classes: ClassList,
}

impl std::fmt::Debug for GenericKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GenericKey")
            .field("symbol_map", &self.symbol_map)
            .field("css_classes", &self.css_classes)
            .finish_non_exhaustive()
    }
}

impl GenericKey {
    pub fn new(
        symbol_map: SymbolMap,
        on_up: OnUp,
        on_down: OnDown,
        css_classes: impl Into<ClassList>,
    ) -> Self {
        let mut class_list = css_classes.into();
        class_list.insert(Class::OskKey);
        Self {
            symbol_map,
            on_down,
            on_up,
            css_classes: class_list,
        }
    }

    pub fn new_without_down(
        symbol_map: SymbolMap,
        on_up: OnUp,
        css_classes: impl Into<ClassList>,
    ) -> Self {
        Self::new(symbol_map, on_up, Rc::new(|_| None), css_classes.into())
    }

    fn up(&self) -> Option<OskKeyOutputMsg> {
        (self.on_up)(self)
    }

    fn down(&mut self) -> Option<OskKeyOutputMsg> {
        let callback = self.on_down.clone();
        callback(self)
    }

    pub fn css_classes_mut(&mut self) -> &mut ClassList {
        &mut self.css_classes
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
            #[watch]
            set_css_classes: &Vec::from(&self.css_classes),
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
            Self::Input::ActiveMods(mods, shift_state) => {
                if daemon::osk::Mod::Alt.contained_in(mods)
                    || daemon::osk::Mod::AltGr.contained_in(mods)
                {
                    self.symbol_map.set_active(ActiveSymbol::Alt);
                } else if daemon::osk::Mod::Shift.contained_in(mods) {
                    self.symbol_map.set_active(ActiveSymbol::Shift);
                } else {
                    self.symbol_map.set_active(ActiveSymbol::Default);
                }

                if self.css_classes.contains(&Class::OskShift) {
                    match shift_state {
                        ShiftState::Off => {
                            self.css_classes.remove(&Class::OskKeyActive);
                            self.css_classes.remove(&Class::OskShiftLock);
                        }
                        ShiftState::On => {
                            self.css_classes.insert(Class::OskKeyActive);
                        }
                        ShiftState::Locked => {
                            self.css_classes.insert(Class::OskShiftLock);
                        }
                    }
                }
            }
            Self::Input::Clicked => {
                if let Some(msg) = self.down() {
                    sender.output_sender().send(msg).unwrap();
                }

                if let Some(msg) = self.up() {
                    sender.output_sender().send(msg).unwrap();
                }
            }
        }
    }
}
