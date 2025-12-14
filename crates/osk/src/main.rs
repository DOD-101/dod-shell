use common::Layouts;
use osk::{App, AppInit};
use relm4::{RelmApp, tokio};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    simple_logger::SimpleLogger::new().env().init().unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();

    let (osk_proxy, layouts) = rt.block_on(async {
        let connection = zbus::Connection::session().await.unwrap();

        (
            daemon::osk::OskProxy::new(&connection).await.unwrap(),
            daemon::config::ConfigProxy::new(&connection)
                .await
                .unwrap()
                .layouts()
                .await
                .unwrap(),
        )
    });

    let app = RelmApp::new("dod-shell.osk");

    let layouts = serde_json::from_str::<Layouts>(&layouts)?;

    let layout = layouts.get_layout_by_name("German De").unwrap();

    app.run_async::<App>(AppInit::new(layout.clone(), osk_proxy));

    Ok(())
}
