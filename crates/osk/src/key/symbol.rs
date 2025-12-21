//! See [`SymbolMap`]
use strum::EnumIs;

/// Different [`String`]s a key can display as
#[derive(Default, Debug, Clone)]
pub struct SymbolMap {
    /// If no modifiers are active
    default: String,
    /// If the shift modifier is active
    shift: String,
    /// If the alt modifier is active
    alt: String,

    /// The currently active symbol
    active: ActiveSymbol,
}

/// Which symbol is currently active
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, EnumIs)]
pub enum ActiveSymbol {
    /// Corresponds to [`field@SymbolMap::default`]
    #[default]
    Default,
    /// Corresponds to [`SymbolMap::shift`]
    Shift,
    /// Corresponds to [`SymbolMap::alt`]
    Alt,
}

impl SymbolMap {
    /// Create a new [`Self`]
    ///
    /// Active will be set to [`ActiveSymbol::default`]
    pub fn new(default: String, shift: String, alt: String) -> Self {
        Self {
            default,
            shift,
            alt,

            active: ActiveSymbol::default(),
        }
    }

    /// Crates a new [`Self`] setting all symbols to the same [`String`]
    pub fn new_single_symbol(symbol: String) -> Self {
        Self::new(symbol.clone(), symbol.clone(), symbol)
    }

    /// Returns the active symbol
    pub fn active_symb(&self) -> &str {
        match self.active {
            ActiveSymbol::Default => self.default_symb(),
            ActiveSymbol::Shift => self.shift_symb(),
            ActiveSymbol::Alt => self.alt_symb(),
        }
    }

    /// Returns [`field@Self::default`]
    pub fn default_symb(&self) -> &str {
        &self.default
    }

    /// Returns [`Self::shift`]
    pub fn shift_symb(&self) -> &str {
        &self.shift
    }

    /// Returns [`Self::alt`]
    pub fn alt_symb(&self) -> &str {
        &self.alt
    }

    /// Set the current active symbol
    pub const fn set_active(&mut self, active: ActiveSymbol) {
        self.active = active;
    }
}
