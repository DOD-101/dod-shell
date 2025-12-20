/// Items relating to the `layouts.json` file
///
/// This file (and the corresponding types) are used by the osk component for determining its
/// layout.
///
/// ## Why a separate file?
///
/// Because the layout is quite large and it makes it easier to manage separately from the rest of
/// the config. We also couldn't to `config.toml` since arrays in toml aren't sorted which is a
/// deal-breaker. So it was either use a separate file or switch everything to json.
use std::fmt::Display;

use crate::css::Class;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum::Display;

/// Json format of `layouts.json`
#[derive(Serialize, Deserialize, Debug, Default, JsonSchema, Clone)]
pub struct Layouts {
    /// Version of the layout (reserved for future use)
    version: u8,

    /// Different layouts
    layouts: Vec<Layout>,

    /// The default layout to use
    default_layout: String,
}

impl Layouts {
    #[must_use]
    pub fn get_layout_by_name(&self, name: &str) -> Option<&Layout> {
        self.layouts.iter().find(|&layout| layout.name == name)
    }

    #[must_use]
    pub fn get_default_layout(&self) -> Option<&Layout> {
        self.get_layout_by_name(&self.default_layout)
    }

    #[must_use]
    pub fn get_default_layout_index(&self) -> Option<usize> {
        self.layouts
            .iter()
            .position(|l| l.name() == self.default_layout)
    }

    #[must_use]
    pub fn layouts(&self) -> &[Layout] {
        &self.layouts
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

/// All types of keys that can come up in a [`Layout`]
#[derive(Serialize, Deserialize, Debug, JsonSchema, Clone)]
pub enum Key {
    /// A modifier key
    Mod(ModKey),
    /// A key which sends a String when pressed
    Utf {
        /// The string sent without modifiers
        label: String,
        /// The string sent when the shift modifier is active
        shift_label: String,
        /// The string sent when the alt modifier is active
        alt_label: String,
    },
    /// A key which sends a key-code
    Code { code: u32 },
    /// An arrow key
    Arrow {
        /// The direction of the arrow
        direction: ArrowDirection,
    },
    /// A function key
    Fn {
        /// The number of the function key
        num: u8,
    },
    /// Enter
    Enter,
    /// Delete
    Del,
    /// Backspace
    Backspace,
    /// Space bar
    Space,
    /// Emtpy space in the keyboard
    Spacer,
    /// Escape
    Escape,
    /// Key used to change to a different layout
    LayoutSwitcher,
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

impl From<ModKey> for Class {
    fn from(value: ModKey) -> Self {
        match value {
            ModKey::Shift => Self::OskShift,
            ModKey::Ctrl => Self::OskCtrl,
            ModKey::Alt => Self::OskAlt,
            ModKey::Super => Self::OskSuper,
            ModKey::AltGr => Self::OskAltGr,
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
