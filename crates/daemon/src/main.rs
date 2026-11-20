//! Binary for the daemon. See lib for more information

#![deny(clippy::arbitrary_source_item_ordering)]

use std::time::Duration;

use anyhow::{Context, Result, bail};
use futures_util::StreamExt;
use tokio::{sync::mpsc, task::JoinSet, time::interval};
use zbus::{conn::Builder, connection::Connection, object_server::InterfaceRef};

use common::{logger, types::Timer};
use daemon::{
    config::{Config, ConfigValuesChanged},
    osk::{Osk, state::State as OskState},
    playback::{Playback, PlaybackCollector},
    system::{self, System},
    system_state::{SystemCollector, SystemState},
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

/// How often to collect & publish system state / playback info, and pump Wayland events.
const POLL_INTERVAL: Duration = Duration::from_secs(1);

#[tokio::main]
async fn main() -> Result<()> {
    logger!();

    let connection = Builder::session()?
        .name("dod.shell.Daemon")?
        .serve_at(DBUS_PATH, Config::default())?
        .serve_at(DBUS_PATH, Osk::new()?)?
        .serve_at(DBUS_PATH, OskState::default())?
        .serve_at(DBUS_PATH, SystemState::default())?
        .serve_at(DBUS_PATH, Playback::default())?
        .build()
        .await?;

    let obj_server = connection.object_server();

    let (state_iface, system_iface, config_iface, osk_iface, osk_state_iface, playback_iface) = create_ifaces!(
        obj_server,
        DBUS_PATH,
        SystemState,
        System,
        Config,
        Osk,
        OskState,
        Playback
    );

    let system_connection = Connection::system()
        .await
        .context("Failed to connect to the system bus.")?;

    // Channel used to hand the freshly-parsed config from the config watcher over to the state
    // poller, so the two don't reach into one another's interfaces.
    let (config_tx, config_rx) = mpsc::channel::<common::Config>(8);

    let mut set: JoinSet<Result<()>> = JoinSet::new();

    // Config watcher: purely event-driven (inotify). Runs independently.
    set.spawn(config_task(config_iface.clone(), config_tx));

    // Osk Wayland event pump: event-driven, entirely separate from polling.
    set.spawn(osk_task(osk_iface.clone(), osk_state_iface.clone()));

    system::system_task(system_iface, config_rx);

    // System-state / playback poller: ticks on a fixed interval.
    set.spawn(poll_task(
        state_iface.clone(),
        playback_iface.clone(),
        system_connection,
    ));

    // Supervisor: any worker exiting is fatal, so wait for the first one to finish.
    if let Some(res) = set.join_next().await {
        res??;
        bail!("Daemon worker exited unexpectedly.");
    }

    unreachable!("Daemon worker should never exit.")
}

/// Worker that watches the config directory for changes (inotify) and, on change, updates the
/// [`Config`] interface and forwards the freshly-parsed [`common::Config`] to the state poller.
///
/// Falls back to polling the config directory if it can't be watched.
async fn config_task(
    config_iface: InterfaceRef<Config>,
    config_tx: mpsc::Sender<common::Config>,
) -> Result<()> {
    // Load the config once at startup so clients get the on-disk values immediately.
    apply_config_update(&config_iface, &config_tx).await?;

    match daemon::config::create_config_update_stream() {
        Ok(mut config_update_stream) => {
            while config_update_stream.next().await.is_some() {
                apply_config_update(&config_iface, &config_tx).await?;
            }

            unreachable!("Config update stream should never end.")
        }
        Err(err) => {
            log::error!("Failed to watch config directory, polling instead: {err}");

            let mut interval = interval(POLL_INTERVAL);
            loop {
                interval.tick().await;
                apply_config_update(&config_iface, &config_tx).await?;
            }
        }
    }
}

/// Wrapper around [`Config::update`] that emits the appropriate signals and forwards the parsed
/// config to the state poller when the toml has changed.
async fn apply_config_update(
    config_iface: &InterfaceRef<Config>,
    config_tx: &mpsc::Sender<common::Config>,
) -> Result<()> {
    let mut iface = config_iface.get_mut().await;

    let changes: ConfigValuesChanged = iface.update().await;

    if changes.toml_changed() {
        iface.config_changed(config_iface.signal_emitter()).await?;
    }

    if changes.css_changed() {
        iface.css_changed(config_iface.signal_emitter()).await?;
    }

    if changes.layouts_changed() {
        iface.layouts_changed(config_iface.signal_emitter()).await?;
    }

    if changes.any_changes() {
        iface
            .all_config_changed(config_iface.signal_emitter())
            .await?;
    }

    let toml_changed = changes.toml_changed();
    let toml = iface.toml.clone();
    drop(iface);

    if toml_changed {
        let config = toml::from_str::<common::Config>(&toml)
            .expect("Config string returned by daemon should always be valid.");
        if config_tx.send(config).await.is_err() {
            bail!("State config receiver dropped. This should not be possible.")
        }
    }

    Ok(())
}

/// Helper method to collect and publish [`SystemState`] + [`Playback`].
///
/// Runs on a fixed interval. All collection happens *outside* the interface locks; the locks are
/// only held long enough to write the freshly-collected data back and emit the changed signals.
async fn poll_task(
    state_iface: InterfaceRef<SystemState>,
    playback_iface: InterfaceRef<Playback>,
    system_connection: Connection,
) -> Result<()> {
    let mut collector = SystemCollector::new(system_connection);
    let mut playback_collector =
        PlaybackCollector::new(playback_iface.signal_emitter().connection().clone());

    let mut interval = interval(POLL_INTERVAL);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                update_state(&state_iface, &mut collector).await?;
                update_playback(&playback_iface, &mut playback_collector).await;
            }
        }
    }
}

/// Helper method to update the values of [`SystemState`]
async fn update_state(
    iface: &InterfaceRef<SystemState>,
    collector: &mut SystemCollector,
) -> Result<()> {
    // Leave the target at 10 to avoid a warning on first run
    let _timer = Timer::new("SystemState updated", Some(Duration::from_millis(10)));

    // Collect *outside* the interface lock.
    let data = collector.collect().await;

    // Short, bounded lock: write + emit, then release.
    let mut state = iface.get_mut().await;
    state.set_data(data);
    state.state_data_changed(iface.signal_emitter()).await?;

    Ok(())
}

/// Helper method to update the values of [`Playback`]
async fn update_playback(iface: &InterfaceRef<Playback>, collector: &mut PlaybackCollector) {
    let _timer = Timer::new("Playback data updated", Some(Duration::from_millis(10)));

    let new = collector.collect().await;

    // Diff against the currently-published state and emit signals *without* the lock.
    let old = iface.get().await;
    daemon::playback::publish_playback(iface, &old, &new).await;
    drop(old);

    // Short, bounded lock: write back the new state.
    let mut playback = iface.get_mut().await;
    *playback = new;
}

/// Worker that pumps Wayland events for the Osk and forwards state changes to the [`OskState`]
/// interface.
///
/// Note: the underlying Wayland [`EventQueue::roundtrip`] is blocking, which is why this runs in
/// its own task and never shares a lock with the other workers.
async fn osk_task(
    osk_iface: InterfaceRef<Osk>,
    osk_state_iface: InterfaceRef<OskState>,
) -> Result<()> {
    let osk = osk_iface.get().await;
    let mut interval = interval(POLL_INTERVAL);

    loop {
        interval.tick().await;

        if let Err(err) = osk
            .handle_wayland_events(osk_iface.signal_emitter(), &osk_state_iface)
            .await
        {
            log::error!("Failed to handle wayland events: {err}");
        }
    }
}
