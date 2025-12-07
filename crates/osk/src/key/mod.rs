pub mod code_resolve;
pub mod symbol;

mod osk_key;
mod osk_row;

pub use {osk_key::GenericKey, osk_row::OskRow};

#[derive(Debug, Clone)]
pub enum OskKeyOutputMsg {
    Utf(String),
    Code(u32),
    Mod(daemon::osk::Mod),
    Shift,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum OskKeyInputMsg {
    ActiveSymbol(symbol::ActiveSymbol),
    Clicked,
}
