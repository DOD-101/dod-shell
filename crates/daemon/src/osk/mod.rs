//! This module contains items relating to creating and using an on-screen-keyboard (osk).
//!
//! This is primarily for use by dod-shell-osk.
//!
//! The main type is [``Osk``]
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::{
    Mutex, RwLock,
    mpsc::{Receiver, error::TryRecvError},
};
use wayland_client::{Connection, EventQueue};
use zbus::{
    fdo, interface,
    object_server::{InterfaceRef, SignalEmitter},
    zvariant,
};

pub mod state;
mod wayland;

use wayland::{WaylandInterface, WaylandStateMsg};

/// Overarching type containing data needed for communication with the Wayland server for getting
/// information and performing the tasks of an Osk.
///
/// ## Dbus
///
/// This struct implements [``zbus::object_server::Interface``], which means it acts as a dbus
/// interface. For available zbus methods and properties see [``OskProxy``]
#[derive(Debug)]
pub struct Osk {
    /// Connection to the Wayland server (compositor)
    connection: Connection,
    /// See [`WaylandInterface`]
    wayland_interface: RwLock<WaylandInterface>,
    /// Event Queue for the [`WaylandInterface`]
    event_queue: Mutex<EventQueue<WaylandInterface>>,
    /// Receiver for state changes emitted by [`Self::wayland_interface`]
    state_receiver: Mutex<Receiver<WaylandStateMsg>>,
}

impl Osk {
    /// Create a new Osk.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    ///
    /// 1. We fail to connect to the Wayland server (compositor).
    ///
    /// 2. There is an issue with internal [initialization](`common::types::DeferedInit`).
    pub fn new() -> Result<Self> {
        let connection = Connection::connect_to_env()?;
        let (mut wayland_interface, state_receiver) = WaylandInterface::new();

        let event_queue = wayland_interface.init(&connection)?;

        Ok(Self {
            connection,
            event_queue: event_queue.into(),
            wayland_interface: wayland_interface.into(),

            state_receiver: state_receiver.into(),
        })
    }

    /// Handles events coming from the Wayland server (compositor).
    ///
    /// # Errors
    ///
    /// This function will return an error if there is an error in the communication with the
    /// Wayland server.
    // NOTE: If something about the Wayland connection breaks look here. Even though the docs make
    // me believe I need to be calling dispatch and using ReadLock this seems to be working
    pub async fn handle_wayland_events(
        &self,
        ctxt: &SignalEmitter<'_>,
        state_iface: &InterfaceRef<state::State>,
    ) -> Result<()> {
        let mut event_queue = self.event_queue.lock().await;

        event_queue.roundtrip(&mut *self.wayland_interface.write().await)?;

        loop {
            match self.state_receiver.lock().await.try_recv() {
                Ok(msg) => {
                    state_iface
                        .get_mut()
                        .await
                        .update_from_msg(ctxt, msg)
                        .await?;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => unreachable!(
                    "Should never be able to have the sender be closed, since it is *indirectly* owned by self."
                ),
            }
        }

        Ok(())
    }

    /// Internal helper function used to flush the [`Connection`] to the Wayland server from within zbus
    /// methods.
    ///
    /// # Errors
    ///
    /// This function will return an error if there is an issue with flushing the Wayland
    /// [`Connection`].
    fn flush_wayland_connection(&self) -> fdo::Result<()> {
        self.connection
            .flush()
            .inspect_err(|e| {
                log::error!("Failed to flush Wayland connection in zbus method call: {e}");
            })
            .map_err(|e| fdo::Error::Failed(e.to_string()))
    }
}

#[interface(
    name = "dod.shell.Daemon.Osk",
    spawn = false,
    proxy(
        gen_blocking = false,
        default_path = "/dod/shell/Daemon",
        default_service = "dod.shell.Daemon"
    )
)]
impl Osk {
    /// Type a single char
    async fn type_char(&self, char: char) -> fdo::Result<()> {
        self.wayland_interface.read().await.type_char(char);

        self.flush_wayland_connection()
    }

    /// Type an entire string
    async fn type_string(&self, string: String) -> fdo::Result<()> {
        self.wayland_interface.read().await.type_string(string);

        self.flush_wayland_connection()
    }

    /// Press a single key with a combination of [`Mod`]s
    ///
    /// This method should only be used if you need to
    ///
    /// 1. press a non-charcter key (eg. `Escape`)
    ///
    /// 2. need to use a modifiers with the key
    ///
    /// Otherwise using either [`Self::type_string`] or [`Self::type_char`] will be simpler.
    async fn press_key(&self, key: u32, mods: Vec<Mod>) -> fdo::Result<()> {
        self.wayland_interface
            .read()
            .await
            .press_key_code(key, &mods);

        self.flush_wayland_connection()
    }

    /// Press a single key with a combination of [`Mod`]s.
    ///
    /// In comparison to [`Self::press_key`] this method takes a mask of all modifiers rather than
    /// a list of them.
    ///
    /// This method should only be used if you need to
    ///
    /// 1. press a non-charcter key (eg. `Escape`)
    ///
    /// 2. need to use a modifiers with the key
    ///
    /// Otherwise using either [`Self::type_string`] or [`Self::type_char`] will be simpler.
    async fn press_key_with_mask(&self, key: u32, mod_mask: u32) -> fdo::Result<()> {
        self.wayland_interface
            .read()
            .await
            .press_key_code_with_mask(key, mod_mask);

        self.flush_wayland_connection()
    }
}

/// Keyboard modifiers for use with [`OskProxy::press_key`]
///
/// For more information see: <https://xkbcommon.org/doc/current/keymap-text-format-v1-v2.html#modifiers-encoding> and [`xkbcommon`]
#[derive(
    Debug,
    Clone,
    Copy,
    zvariant::Value,
    zvariant::OwnedValue,
    zvariant::Type,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
)]
pub enum Mod {
    Shift = 0x1,
    Ctrl = 0x4,
    Alt = 0x8,
    Super = 0x40,
    AltGr = 0x80,
}

impl Mod {
    /// Join a series of [`Mod`]s into a single bit mask
    #[must_use]
    pub fn join_mods(mods: &[Self]) -> u32 {
        mods.iter().fold(0_u32, |acc, val| acc | val.add_to(acc))
    }

    /// Check if `self` is in `others`
    #[must_use]
    pub fn contained_in(self, others: u32) -> bool {
        (self as u32 & others) != 0
    }

    /// Removes `self` from `others`
    ///
    /// Will return `others` unchanged if it doesn't contain `self`
    #[must_use]
    pub fn remove_from(self, others: u32) -> u32 {
        if self.contained_in(others) {
            return others ^ self as u32;
        }

        others
    }

    /// Adds `self` to `others`
    #[must_use]
    pub fn add_to(self, others: u32) -> u32 {
        others | self as u32
    }
}

#[cfg(test)]
mod test {
    use super::Mod;

    #[test]
    fn mod_join() {
        let a = Mod::Shift;
        let b = Mod::Ctrl;

        let joined = Mod::join_mods(&[a, b]);

        assert_eq!(joined, 5);
    }

    #[test]
    fn mod_contain() {
        let a = Mod::Shift;
        let b = Mod::Ctrl;
        let c = Mod::Super;

        let joined = Mod::join_mods(&[a, b]);

        assert!(b.contained_in(joined));

        assert!(!c.contained_in(joined));
    }

    #[test]
    fn mod_remove() {
        let a = Mod::Shift;
        let b = Mod::Ctrl;
        let c = Mod::Super;

        let abc = Mod::join_mods(&[a, b, c]);

        let ac = Mod::join_mods(&[a, c]);

        assert_eq!(b.remove_from(abc), ac);

        assert_eq!(b.remove_from(ac), ac);
    }

    #[test]
    fn mod_add() {
        let a = Mod::Shift;
        let b = Mod::Ctrl;
        let c = Mod::Super;

        let abc = Mod::join_mods(&[a, b, c]);
        let ac = Mod::join_mods(&[a, c]);

        assert_eq!(b.add_to(ac), abc);
    }
}
