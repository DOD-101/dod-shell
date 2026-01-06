//! Items relating to the `layouts.json` file
//!
//! This file (and the corresponding types) are used by the osk component for determining its
//! layout.
//!
//! ## Why a separate file?
//!
//! Because the layout is quite large and it makes it easier to manage separately from the rest of
//! the config. We also couldn't to `config.toml` since arrays in toml aren't sorted which is a
//! deal-breaker. So it was either use a separate file or switch everything to json.
use crate::css::Class;
use serde::{Deserialize, Serialize};
use strum::Display;

/// Json format of `layouts.json`
#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Layouts {
    /// Version of the layout (reserved for future use)
    version: u8,

    /// Different layouts
    #[allow(
        clippy::struct_field_names,
        reason = "Any other naming would be less clear here."
    )]
    layouts: Vec<Layout>,

    /// The default layout to use
    default_layout: String,
}

impl Default for Layouts {
    fn default() -> Self {
        Self {
            version: 1,
            layouts: Vec::default(),
            default_layout: String::default(),
        }
    }
}

impl Layouts {
    /// Attempts to get a layout by it's [name](`Layout::name`)
    #[must_use]
    pub fn get_layout_by_name(&self, name: &str) -> Option<&Layout> {
        self.layouts.iter().find(|&layout| layout.name == name)
    }

    /// Attempts to get the default layout
    ///
    /// If this method fails that means that `default_layout` is pointing to an
    /// non-existent layout.
    #[must_use]
    pub fn get_default_layout(&self) -> Option<&Layout> {
        self.get_layout_by_name(&self.default_layout)
    }

    /// Attempts to get the index of the default layout
    ///
    /// If this method fails that means that `default_layout` is pointing to an
    /// non-existent layout.
    #[must_use]
    pub fn get_default_layout_index(&self) -> Option<usize> {
        self.layouts
            .iter()
            .position(|l| l.name() == self.default_layout)
    }

    /// Returns a reference to the layouts of this [`Layouts`].
    #[must_use]
    pub fn layouts(&self) -> &[Layout] {
        &self.layouts
    }
}

/// Format for an individual layout
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Layout {
    /// Name of the layout
    name: String,
    /// Shorter representation of the layout
    ///
    /// e.g: "en-us", "de-de"
    name_short: String,
    /// The actual keys of the layout in the format:
    ///
    /// `Vertical<Horizontal<Key>>`
    keys: Vec<Vec<Key>>,
}

impl Layout {
    /// Returns a reference to the name of this [`Layout`].
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns a reference to the name short of this [`Layout`].
    #[must_use]
    pub fn name_short(&self) -> &str {
        &self.name_short
    }

    /// Returns a reference to the keys of this [`Layout`].
    #[must_use]
    pub fn keys(&self) -> &[Vec<Key>] {
        &self.keys
    }
}

/// All types of keys that can come up in a [`Layout`]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
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
    Code {
        /// The key code
        code: u32,
    },
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
    /// Escape
    Escape,
    /// Key used to change to a different layout
    LayoutSwitcher,
}

/// Different types of Modifiers
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Display)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum ModKey {
    /// Shift
    Shift,
    /// Control / Ctrl
    Ctrl,
    /// Alt
    Alt,
    /// This is is the right alt key on German QWERTZ keyboards.
    /// Officially called `ISO_Level3_Shift`.
    AltGr,
    /// Super / "windows-key"
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

/// Different directions an arrow key can point in
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Display)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[allow(missing_docs, reason = "No docs needed.")]
pub enum ArrowDirection {
    Up,
    Down,
    Left,
    Right,
}

impl ArrowDirection {
    /// Returns the symbol (aka. arrow icon) for the direction
    #[must_use]
    pub const fn as_symbol(&self) -> &str {
        match self {
            Self::Up => "↑",
            Self::Down => "↓",
            Self::Left => "←",
            Self::Right => "→",
        }
    }
}
