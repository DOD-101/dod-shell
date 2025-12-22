//! Implementation of the secondary bar
//!
//! For more information see: [`super`]
use super::{App, AppInit, Init, StateBroker};
use relm4::{AsyncComponentSender, gtk::gdk::Monitor};
use std::sync::Arc;

/// State for secondary bar
///
/// The [`StateBroker`] is subscribed to in the [`Self::init`] method.
pub struct Secondary(Arc<StateBroker>);

impl Init for Secondary {
    async fn init(&self, sender: AsyncComponentSender<App<Self>>) {
        self.0.subscribe(sender.input_sender().clone());
    }
}

impl AppInit<Secondary> {
    /// Create a new [`Self`]
    pub const fn new(monitor: Monitor, monitor_id: i128, broker: Arc<StateBroker>) -> Self {
        Self {
            monitor,
            monitor_id,

            data: Secondary(broker),
        }
    }
}
