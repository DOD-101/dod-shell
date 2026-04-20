//! Items relating to updating the bars state
//!
//! The goal here is to allow for a variable amount of bar instances, without adding significant
//! overhead by having each one fetch updates from the daemon independently.
//!
//! See: [`StateBroker`]
use crate::app::AppMsg;
use daemon::{config::ConfigProxy, osk::state::StateProxy, system_state::SystemStateProxy};
use futures_util::StreamExt;
use std::{
    marker::PhantomData,
    sync::{Arc, RwLock},
};
use zbus::Connection;

// NOTE: We should probably generalize this to all type 1 components and move it to common
// See comment above.

/// Default state of [`StateBroker`]
///
/// This is the default state. Here components can be added, by adding them with
/// [`StateBroker::subscribe`]. Once all components have been added the state is changed to
/// [`SendingUpdates`] using [`StateBroker::finish`].
pub struct GatheringSubscribers;

/// Second state of [`StateBroker`]
///
/// In this state the [`StateBroker`] no longer accepts new components and will start sending
/// updates to all subscribers previously added.
pub struct SendingUpdates;

/// Manages updates from the daemon for all instances of the [`super::App`]
pub struct StateBroker<State = GatheringSubscribers> {
    /// The zbus connection used to talk to the deamon
    connection: Connection,
    /// Senders of components updates will be sent to
    subscribers: RwLock<Vec<relm4::Sender<AppMsg>>>,

    /// Marker to support type-state pattern
    _marker: PhantomData<State>,
}

impl StateBroker {
    /// Create a new instance of [`Self`]
    pub async fn new() -> Self {
        Self {
            connection: zbus::Connection::session().await.unwrap(),
            subscribers: RwLock::new(Vec::new()),

            _marker: PhantomData,
        }
    }

    /// Add a subscriber, which will receive state updates
    pub fn subscribe(&self, subscriber: relm4::Sender<AppMsg>) {
        let mut guard = self.subscribers.write().expect("Should never poison.");

        guard.push(subscriber);
    }

    /// Finish allowing components to be added.
    pub fn finish(self) -> StateBroker<SendingUpdates> {
        StateBroker::<SendingUpdates> {
            connection: self.connection,
            subscribers: self.subscribers,
            _marker: PhantomData,
        }
    }
}

impl StateBroker<SendingUpdates> {
    /// Send an update to all subscribers
    fn send_update(&self, msg: &AppMsg) {
        let guard = self.subscribers.read().expect("Should never poison.");

        for subscriber in &*guard {
            if subscriber.send(msg.clone()).is_err() {
                log::error!("Failed to send update to bar (subscriber).");
            }
        }
        log::trace!("Sent update to all bars.");
    }

    /// Loops getting updates from the daemon and sending them to all components
    ///
    /// This function will only exit if it encounters an error.
    pub async fn start_updating(&self) -> zbus::Result<()> {
        let config_proxy = ConfigProxy::new(&self.connection).await?;
        let state_proxy = SystemStateProxy::new(&self.connection).await?;
        let osk_state_proxy = StateProxy::new(&self.connection).await?;

        let mut state_stream = state_proxy.receive_state_data_changed().await.fuse();
        let mut config_stream = config_proxy.receive_config_changed().await.fuse();
        let mut css_stream = config_proxy.receive_css_changed().await.fuse();
        let mut osk_active_stream = osk_state_proxy.receive_active_changed().await.fuse();
        let mut osk_active_locked_stream =
            osk_state_proxy.receive_active_locked_changed().await.fuse();

        loop {
            futures_util::select! {
                c = config_stream.select_next_some() => {
                    let config = toml::from_str::<common::config::Config>(&c.get().await?)
                        .expect("Config string returned by daemon should always be valid.");

                    self.send_update(&AppMsg::ConfigUpdated(Arc::new(config.bar)));
                }
                css = css_stream.select_next_some() => {
                    relm4::set_global_css(&css.get().await?);
                }
                s = state_stream.select_next_some() => {
                    self.send_update(&AppMsg::UpdatedSystemState(Arc::new(s.get().await?)));
                }
                active = osk_active_stream.select_next_some() => {
                    self.send_update(&AppMsg::OskActive(active.get().await?));
                }
                locked = osk_active_locked_stream.select_next_some() => {
                    self.send_update(&AppMsg::OskLocked(locked.get().await?));
                }
            }
        }
    }
}
