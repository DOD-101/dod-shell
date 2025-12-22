//! Implementation of the primary bar
//!
//! For more information see: [`super`]
use super::{App, AppInit, Init, Secondary, StateBroker, state::SendingUpdates};
use relm4::AsyncComponentSender;
use relm4::{gtk::gdk::Monitor, gtk::prelude::*, prelude::*};
use std::sync::Arc;

/// State for primary bar
///
/// For more information see: [`super`]
pub struct Primary;

impl Init for Primary {
    async fn init(&self, sender: AsyncComponentSender<super::App<Self>>) {
        let mut state_broker = Arc::new(StateBroker::new().await);

        let monitor_list = relm4::gtk::gdk::Display::default()
            .expect("Failed to get display")
            .monitors();
        let mut monitors = monitor_list.iter::<Monitor>().flatten().enumerate();

        monitors.next(); // Discard the first monitor, since that is what the primary bar is on

        let app = relm4::main_application();
        for monitor in monitors {
            let builder = App::<Secondary>::builder();
            app.add_window(&builder.root);

            let init =
                AppInit::<Secondary>::new(monitor.1, monitor.0 as i128, state_broker.clone());

            builder.launch(init).detach_runtime();
        }

        state_broker.subscribe(sender.input_sender().clone());

        relm4::spawn(async move {
            let unwraped: StateBroker<SendingUpdates>;
            loop {
                match Arc::try_unwrap(state_broker) {
                    Ok(broker) => {
                        unwraped = broker.finish();
                        break;
                    }
                    Err(arc) => {
                        log::info!(
                            "Waiting for init to be complete before sending update messages."
                        );
                        relm4::tokio::time::sleep(relm4::tokio::time::Duration::from_millis(50))
                            .await;
                        state_broker = arc;
                    }
                }
            }

            if let Err(e) = unwraped.start_updating().await {
                log::error!("Getting updates from the daemon has failed: {e}");
                log::error!("No more updates will be received.");
            }
        });
    }
}

impl AppInit<Primary> {
    /// Create a new [`Self`]
    pub const fn new(monitor: Monitor, monitor_id: i128) -> Self {
        Self {
            monitor,
            monitor_id,

            data: Primary,
        }
    }
}
