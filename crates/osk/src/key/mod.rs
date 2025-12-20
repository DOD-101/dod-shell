pub mod code_resolve;
pub mod symbol;

mod osk_key;
mod osk_row;

use strum::EnumIs;

use crate::ShiftState;

pub use {osk_key::GenericKey, osk_row::OskRow};

#[derive(Debug, Clone, PartialEq, Eq, EnumIs)]
pub enum OskKeyOutputMsg {
    Utf(String),
    Code(u32),
    Mod(daemon::osk::Mod),
    Shift,
    SwitchLayout,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum OskKeyInputMsg {
    ActiveMods(u32, ShiftState),
    Clicked,
}
