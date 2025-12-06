#[derive(Default, Debug, Clone)]
pub struct SymbolMap {
    default: String,
    shift: String,
    alt: String,

    active: ActiveSymbol,
}

#[derive(Default, Debug, Clone, Copy)]
pub enum ActiveSymbol {
    #[default]
    Default,
    Shift,
    Alt,
}

impl SymbolMap {
    pub fn new(default: String, shift: String, alt: String) -> Self {
        Self {
            default,
            shift,
            alt,

            active: ActiveSymbol::default(),
        }
    }

    pub fn new_single_symbol(symbol: String) -> Self {
        Self::new(symbol.clone(), symbol.clone(), symbol)
    }

    pub fn active_symb(&self) -> &str {
        match self.active {
            ActiveSymbol::Default => self.default_symb(),
            ActiveSymbol::Shift => self.shift_symb(),
            ActiveSymbol::Alt => self.alt_symb(),
        }
    }

    pub fn default_symb(&self) -> &str {
        &self.default
    }

    pub fn shift_symb(&self) -> &str {
        &self.shift
    }

    pub fn alt_symb(&self) -> &str {
        &self.alt
    }

    pub fn active(&self) -> &ActiveSymbol {
        &self.active
    }

    pub fn set_active(&mut self, active: ActiveSymbol) {
        self.active = active;
    }
}
