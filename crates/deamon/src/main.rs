use zbus::{Result, conn::Builder};

use deamon::system_state::SystemState;

#[tokio::main]
async fn main() -> Result<()> {
    simple_logger::SimpleLogger::new().env().init().unwrap();
    let connection = Builder::session()?
        .name("dod.shell.Deamon")?
        .serve_at("/dod/shell/Deamon", SystemState::default())?
        .build()
        .await?;

    let obj_server = connection.object_server();

    let state_iface = obj_server
        .interface::<_, SystemState>("/dod/shell/Deamon")
        .await
        .unwrap();

    loop {
        {
            let mut state = state_iface.get_mut().await;

            let start = tokio::time::Instant::now();

            state.update().await;
            state
                .get_state_data_changed(state_iface.signal_emitter())
                .await?;

            let end = tokio::time::Instant::now();

            let delta = (end - start).as_millis();

            log::trace!("State updated. Took {delta}ms");
        }

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
