//! Binary for the daemon. See lib for more information

#![deny(clippy::arbitrary_source_item_ordering)]

use std::time::Duration;

use common::types::Timer;
use zbus::{conn::Builder, object_server::InterfaceRef};

use anyhow::{Ok, Result};

use daemon::{
    config::{Config, ConfigProxy},
    osk::{Osk, state::State as OskState},
    system_state::SystemState,
};

/// Macro used to create [`zbus::object_server::InterfaceRef`]s
macro_rules! create_ifaces {
    ($obj_server:ident, $path:expr, $($iface:ident),+ ) => {
        (
        $(
            $obj_server
                .interface::<_, $iface>($path)
                .await
                .expect(&format!("Interface '{}' should be have been registered at path '{}'", stringify!($iface), $path)),
        )+
        )
    };
}

/// Path to all dbus served [`zbus::object_server::Interface`]s
const DBUS_PATH: &str = "/dod/shell/Daemon";

#[tokio::main]
async fn main() -> Result<()> {
    simple_logger::SimpleLogger::new().env().init().unwrap();

    let connection = Builder::session()?
        .name("dod.shell.Daemon")?
        .serve_at(DBUS_PATH, Config::default())?
        .serve_at(DBUS_PATH, Osk::new()?)?
        .serve_at(DBUS_PATH, OskState::default())?
        .serve_at(DBUS_PATH, SystemState::default())?
        .build()
        .await?;

    let obj_server = connection.object_server();

    let (state_iface, config_iface, osk_iface, osk_state_iface) =
        create_ifaces!(obj_server, DBUS_PATH, SystemState, Config, Osk, OskState);

    let config_proxy = ConfigProxy::new(&connection).await?;

    loop {
        if update_config(&config_iface).await? {
            let config = toml::from_str::<common::Config>(&config_proxy.config().await?)
                .expect("Config string returned by daemon should always be valid.");

            let mut state = state_iface.get_mut().await;

            state.set_config(config);
        }

        update_state(&state_iface).await?;

        {
            let osk = osk_iface.get().await;

            let _ = osk
                .handle_wayland_events(osk_iface.signal_emitter(), &osk_state_iface)
                .await;
        }

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

/// Helper method to update the values of [`SystemState`]
async fn update_state(iface: &InterfaceRef<SystemState>) -> Result<()> {
    // Leave the target at 10 to avoid a warning on first run
    let _timer = Timer::new("SystemState updated", Some(Duration::from_millis(10)));

    let mut state = iface.get_mut().await;

    state.update().await;
    state.state_data_changed(iface.signal_emitter()).await?;

    Ok(())
}

/// Helper method to update the values of [`Config`]
async fn update_config(iface: &InterfaceRef<Config>) -> Result<bool> {
    let _timer = Timer::new("Config updated", Some(Duration::from_millis(5)));

    let mut state = iface.get_mut().await;

    let changes = state.update().await;

    if changes.toml_changed() {
        state.config_changed(iface.signal_emitter()).await?;
    }

    if changes.css_changed() {
        state.css_changed(iface.signal_emitter()).await?;
    }

    if changes.layouts_changed() {
        state.layouts_changed(iface.signal_emitter()).await?;
    }

    if changes.any_changes() {
        state.all_config_changed(iface.signal_emitter()).await?;
    }

    Ok(changes.toml_changed())
}
