use std::{fmt::Display, rc::Rc};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum::Display;

use crate::key::{GenericKey, OskKeyOutputMsg, code_resolve, symbol::SymbolMap};

const SUPPORTED_LAYOUT_FORMAT_VERSION: u8 = 1;

/// Format for the entire file
#[derive(Serialize, Deserialize, Debug, Default, JsonSchema)]
pub struct Layouts {
    version: u8,

    layouts: Vec<Layout>,
}

impl Layouts {
    #[must_use]
    pub fn get_layout_by_name(&self, name: &str) -> Option<&Layout> {
        self.layouts.iter().find(|&layout| layout.name == name)
    }
}

type Vertical<T> = Vec<T>;
type Horizontal<T> = Vec<T>;

/// Format for an individual layout
#[derive(Serialize, Deserialize, Debug, Default, JsonSchema, Clone)]
pub struct Layout {
    /// Name of the layout
    name: String,
    /// Shorter representation of the layout
    ///
    /// e.g: "en-us", "de-de"
    name_short: String,
    /// The actual keys of the layout in the format:
    ///
    /// Vertical<Horizontal<Key>>
    keys: Vertical<Horizontal<Key>>,
}

impl Layout {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn name_short(&self) -> &str {
        &self.name_short
    }

    #[must_use]
    pub fn keys(&self) -> &[Vec<Key>] {
        &self.keys
    }
}

#[derive(Serialize, Deserialize, Debug, JsonSchema, Clone)]
pub enum Key {
    Mod(ModKey),
    Utf {
        label: String,
        shift_label: String,
        alt_label: String,
    },
    Code {
        code: u32,
    },
    Arrow {
        direction: ArrowDirection,
    },
    Fn {
        num: u8,
    },
    Enter,
    Del,
    Backspace,
    Space,
    /// Emtpy space in the keyboard
    Spacer,
    Escape,
}

#[derive(
    Serialize, Deserialize, Debug, JsonSchema, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Display,
)]
pub enum ModKey {
    Shift,
    Ctrl,
    Alt,
    AltGr,
    #[strum(to_string = "")]
    Super,
}
impl TryFrom<ModKey> for daemon::osk::Mod {
    type Error = ModKey;
    fn try_from(value: ModKey) -> Result<Self, Self::Error> {
        match value {
            ModKey::Ctrl => Ok(Self::Ctrl),
            ModKey::Alt => Ok(Self::Alt),
            ModKey::AltGr => Ok(Self::AltGr),
            ModKey::Super => Ok(Self::Super),

            ModKey::Shift => Err(value),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, JsonSchema, Clone, Copy)]
pub enum ArrowDirection {
    Up,
    Down,
    Left,
    Right,
}

impl Display for ArrowDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ArrowDirection::Up => "↑",
                ArrowDirection::Down => "↓",
                ArrowDirection::Left => "←",
                ArrowDirection::Right => "→",
            }
        )
    }
}

impl From<Key> for GenericKey {
    fn from(value: Key) -> Self {
        match value {
            Key::Mod(mod_key) => {
                let msg: OskKeyOutputMsg = match mod_key.try_into() {
                    Ok(v) => OskKeyOutputMsg::Mod(v),
                    Err(_) => OskKeyOutputMsg::Shift,
                };
                GenericKey::new(
                    SymbolMap::new_single_symbol(mod_key.to_string()),
                    Rc::new(move |_| Some(msg.clone())),
                )
            }
            Key::Utf {
                label,
                shift_label,
                alt_label,
            } => GenericKey::new(
                SymbolMap::new(label, shift_label, alt_label),
                Rc::new(|_| None),
            ),
            Key::Code { code } => {
                let mut chars = code_resolve::to_chars(code).into_iter().map(|v| {
                    let a = v.unwrap_or_default().to_string();
                    if &a == "\0" {
                        return String::default();
                    }
                    a
                });
                GenericKey::new(
                    SymbolMap::new(
                        chars.next().unwrap(),
                        chars.next().unwrap(),
                        chars.next().unwrap(),
                    ),
                    Rc::new(move |_| Some(OskKeyOutputMsg::Code(code))),
                )
            }
            Key::Arrow { .. } => todo!(),
            Key::Fn { .. } => todo!(),
            Key::Enter => GenericKey::new(
                SymbolMap::new_single_symbol("Enter".to_string()),
                Rc::new(|_| Some(OskKeyOutputMsg::Code(36))),
            ),
            Key::Del => todo!(),
            Key::Backspace => GenericKey::new(
                SymbolMap::new_single_symbol("󰭜".to_string()),
                Rc::new(|_| Some(OskKeyOutputMsg::Code(22))),
            ),
            Key::Space => GenericKey::new(
                SymbolMap::new_single_symbol(" ".to_string()),
                Rc::new(|_| Some(OskKeyOutputMsg::Code(65))),
            ),
            Key::Spacer => GenericKey::new(SymbolMap::default(), Rc::new(|_| None)),
            Key::Escape => GenericKey::new(
                SymbolMap::new_single_symbol("Esc".to_string()),
                Rc::new(|_| Some(OskKeyOutputMsg::Code(9))),
            ),
        }
    }
}
