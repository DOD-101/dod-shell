//! Module containing items around the actual keys of the osk
pub mod code_resolve;
pub mod symbol;

mod osk_key;
mod osk_row;

use strum::EnumIs;

use crate::ShiftState;

pub use {osk_key::GenericKey, osk_row::OskRow};

/// Output messages for [`osk_key::GenericKeyWidgets`]
///
/// These are sent when the key is pressed.
#[derive(Debug, Clone, PartialEq, Eq, EnumIs)]
pub enum OskKeyOutputMsg {
    /// A string to be types
    Utf(String),
    /// A key-code to be pressed
    Code(u32),
    /// A pressed modifier
    Mod(daemon::osk::Mod),
    /// Changing of the [`crate::ShiftState`]
    Shift,
    /// Change the current layout of the osk
    SwitchLayout,
}

/// Input messages for [`osk_key::GenericKeyWidgets`]
///
/// These are used to update the [`GenericKey`]s of changes to their state
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum OskKeyInputMsg {
    /// [`crate::App::active_mods`] or [`crate::App::shift_state`] have changed
    ActiveMods(u32, ShiftState),
    /// Sent by a key to itself when it has been clicked
    Clicked,
}
