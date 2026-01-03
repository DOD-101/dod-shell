//! See [`GenericKey`]
use crate::{ShiftState, key::code_resolve::get_key_code};

use super::{
    OskKeyInputMsg, OskKeyOutputMsg,
    symbol::{ActiveSymbol, SymbolMap},
};
use std::rc::Rc;

use common::{
    config::layouts::{Key, ModKey},
    css::{Class, ClassList},
};
use daemon::osk::Mod;
use gtk::prelude::*;
use relm4::{factory::CloneableFactoryComponent, gtk, prelude::*};

/// Closure for when the key is pressed down
// NOTE: Currently the OutputMsg isn't used here, but in the future it will be
type OnDown = Rc<dyn Fn(&mut GenericKey) -> Option<OskKeyOutputMsg>>;
/// Closure for when the key is released
type OnUp = Rc<dyn Fn(&GenericKey) -> Option<OskKeyOutputMsg>>;

/// A key for the osk
///
/// This is what actually gets displayed and can be pressed in the form of [`GenericKeyWidgets`]
///
/// ## Why "generic"?
///
/// Because all keys, no matter what they do need to be brought into this form the be used.
///
/// See [`Self::from::<key>()`] to see how this is done.
#[derive(Clone)]
pub struct GenericKey {
    /// See [`SymbolMap`]
    symbol_map: SymbolMap,

    /// Called when the key is pressed down
    on_down: OnDown,
    /// Called when the key is released
    on_up: OnUp,

    /// The keys css classes
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
    /// Create a new [`Self`] setting all values
    ///
    /// `css_classes` will have [`Class::OskKey`] added automatically.
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

    /// Create a new [`Self`] without [`Self::on_down`]
    pub fn new_without_down(
        symbol_map: SymbolMap,
        on_up: OnUp,
        css_classes: impl Into<ClassList>,
    ) -> Self {
        Self::new(symbol_map, on_up, Rc::new(|_| None), css_classes.into())
    }

    /// Call [`Self::on_up`]
    fn up(&self) -> Option<OskKeyOutputMsg> {
        (self.on_up)(self)
    }

    /// Call [`Self::on_down`]
    fn down(&mut self) -> Option<OskKeyOutputMsg> {
        let callback = self.on_down.clone();
        callback(self)
    }

    /// Returns a mutable reference to the css classes of this [`GenericKey`].
    pub const fn css_classes_mut(&mut self) -> &mut ClassList {
        &mut self.css_classes
    }
}

impl CloneableFactoryComponent for GenericKey {
    fn get_init(&self) -> Self::Init {
        self.clone()
    }
}

impl From<Key> for GenericKey {
    #[allow(
        clippy::too_many_lines,
        reason = "Might split this up in the future. But for now doing that would only hurt readability."
    )]
    fn from(value: Key) -> Self {
        match value {
            Key::Mod(mod_key) => {
                let msg = match mod_key {
                    ModKey::Ctrl => OskKeyOutputMsg::Mod(Mod::Ctrl),
                    ModKey::Alt => OskKeyOutputMsg::Mod(Mod::Alt),
                    ModKey::AltGr => OskKeyOutputMsg::Mod(Mod::AltGr),
                    ModKey::Super => OskKeyOutputMsg::Mod(Mod::Super),

                    ModKey::Shift => OskKeyOutputMsg::Shift,
                };

                let class = Class::from(mod_key);

                let on_down = move |key: &mut Self| -> Option<OskKeyOutputMsg> {
                    let css_classes = key.css_classes_mut();

                    if !class.is_osk_shift() {
                        if css_classes.contains(&Class::OskKeyActive) {
                            css_classes.remove(&Class::OskKeyActive);
                        } else {
                            css_classes.insert(Class::OskKeyActive);
                        }
                    }

                    None
                };

                Self::new(
                    SymbolMap::new_single_symbol(mod_key.to_string()),
                    Rc::new(move |_| Some(msg.clone())),
                    Rc::new(on_down),
                    &[Class::OskMod, class],
                )
            }
            Key::Utf {
                label,
                shift_label,
                alt_label,
            } => Self::new_without_down(
                SymbolMap::new(label, shift_label, alt_label),
                Rc::new(|key| {
                    Some(OskKeyOutputMsg::Utf(
                        key.symbol_map.active_symb().to_string(),
                    ))
                }),
                &[Class::OskUtf, Class::OskNormal],
            ),
            Key::Code { code } => {
                const NEXT_EXPECT_MSG: &str =
                    "Should never fail since we know the iterator to be 3 long";
                let mut chars = crate::key::code_resolve::to_chars(code)
                    .into_iter()
                    .map(|v| {
                        let a = v.unwrap_or_default().to_string();
                        if &a == "\0" {
                            return String::default();
                        }
                        a
                    });
                Self::new_without_down(
                    SymbolMap::new(
                        chars.next().expect(NEXT_EXPECT_MSG),
                        chars.next().expect(NEXT_EXPECT_MSG),
                        chars.next().expect(NEXT_EXPECT_MSG),
                    ),
                    Rc::new(move |_| Some(OskKeyOutputMsg::Code(code))),
                    &[Class::OskCode, Class::OskNormal],
                )
            }
            Key::Arrow { direction } => Self::new_without_down(
                SymbolMap::new_single_symbol(direction.as_symbol().to_string()),
                Rc::new(move |_| {
                    Some(OskKeyOutputMsg::Code(
                        get_key_code(&format!("XK_{direction}"))
                            .expect("Should never fail to resolve key code."),
                    ))
                }),
                &[Class::OskArrow],
            ),
            Key::Fn { num } => Self::new_without_down(
                SymbolMap::new_single_symbol(format!("F{num}")),
                // TODO: Make the format of Fn key so that you can't even pass an invalid number
                Rc::new(move |_| {
                    get_key_code(&format!("XK_F{num}")).map_or_else(
                        || {
                            log::error!("Function key number is invalid. It won't do anything.");
                            None
                        },
                        |val| Some(OskKeyOutputMsg::Code(val)),
                    )
                }),
                &[Class::OskFn],
            ),
            Key::Enter => Self::new_without_down(
                SymbolMap::new_single_symbol("Enter".to_string()),
                Rc::new(|_| Some(OskKeyOutputMsg::Code(36))),
                &[Class::OskEnter],
            ),
            Key::Del => Self::new_without_down(
                SymbolMap::new_single_symbol("Del".to_string()),
                Rc::new(|_| Some(OskKeyOutputMsg::Code(119))),
                &[Class::OskDel],
            ),
            Key::Backspace => Self::new_without_down(
                SymbolMap::new_single_symbol("󰭜".to_string()),
                Rc::new(|_| Some(OskKeyOutputMsg::Code(22))),
                &[Class::OskBackspace],
            ),
            Key::Space => Self::new_without_down(
                SymbolMap::new_single_symbol(" ".to_string()),
                Rc::new(|_| Some(OskKeyOutputMsg::Code(65))),
                &[Class::OskSpace],
            ),
            Key::Escape => Self::new_without_down(
                SymbolMap::new_single_symbol("Esc".to_string()),
                Rc::new(|_| Some(OskKeyOutputMsg::Code(9))),
                &[Class::OskEscape],
            ),
            Key::LayoutSwitcher => Self::new_without_down(
                SymbolMap::new_single_symbol("".to_string()),
                Rc::new(|_| Some(OskKeyOutputMsg::SwitchLayout)),
                &[Class::OskLayoutSwitcher],
            ),
        }
    }
}

#[relm4::factory(pub)]
impl FactoryComponent for GenericKey {
    type Init = Self;
    type Input = OskKeyInputMsg;
    type Output = OskKeyOutputMsg;
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        #[root]
        /// Button for the key
        gtk::Button {
            #[watch]
            set_label: self.symbol_map.active_symb(),
            #[watch]
            set_css_classes: &Vec::from(&self.css_classes),
            connect_clicked => Self::Input::Clicked,
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
