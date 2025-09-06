use zbus::{Result, conn::Builder, object_server::InterfaceRef};

use deamon::{
    config::{Config, ConfigProxy, ConfigValuesChanged},
    system_state::SystemState,
};

#[tokio::main]
async fn main() -> Result<()> {
    simple_logger::SimpleLogger::new().env().init().unwrap();
    let connection = Builder::session()?
        .name("dod.shell.Deamon")?
        .serve_at(
            "/dod/shell/Deamon",
            SystemState::new(common::Config::default()),
        )?
        .serve_at("/dod/shell/Deamon", Config::default())?
        .build()
        .await?;

    let obj_server = connection.object_server();

    let state_iface = obj_server
        .interface::<_, SystemState>("/dod/shell/Deamon")
        .await
        .unwrap();

    let config_iface = obj_server
        .interface::<_, Config>("/dod/shell/Deamon")
        .await
        .unwrap();

    let config_proxy = ConfigProxy::new(&connection).await?;

    loop {
        if update_config(&config_iface).await? {
            let config = toml::from_str::<common::Config>(&config_proxy.config().await?)
                .expect("Config string returned by deamon should always be valid.");

            let mut state = state_iface.get_mut().await;

            state.set_config(config);
        }

        update_state(&state_iface).await?;

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

async fn update_state(iface: &InterfaceRef<SystemState>) -> Result<()> {
    let mut state = iface.get_mut().await;

    let start = tokio::time::Instant::now();

    state.update().await;
    state.state_data_changed(iface.signal_emitter()).await?;

    let delta = start.elapsed().as_millis();

    log::trace!("State updated. Took {delta}ms");

    Ok(())
}

async fn update_config(iface: &InterfaceRef<Config>) -> Result<bool> {
    let mut state = iface.get_mut().await;

    let start = tokio::time::Instant::now();

    let changes = state.update().await;

    if let ConfigValuesChanged(true, _) = changes {
        state.config_changed(iface.signal_emitter()).await?;
    }

    if let ConfigValuesChanged(_, true) = changes {
        state.css_changed(iface.signal_emitter()).await?;
    }

    if changes.changes() {
        state.all_config_changed(iface.signal_emitter()).await?;
    }

    let delta = start.elapsed().as_millis();

    log::trace!("Config updated. Took {delta}ms");

    Ok(changes.0)
}
