use std::error::Error;

use osk::{App, AppInit, layouts::Layouts};
use relm4::{RelmApp, tokio};
use schemars::schema_for;

fn main() -> Result<(), Box<dyn Error>> {
    simple_logger::SimpleLogger::new().env().init().unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();

    let osk_proxy = rt.block_on(async {
        let connection = zbus::Connection::session().await.unwrap();

        daemon::osk::OskProxy::new(&connection).await.unwrap()
    });

    let app = RelmApp::new("dod-shell.osk");

    let schema = schema_for!(Layouts);

    print!("{}", serde_json::to_string(&schema).unwrap());

    let layouts = serde_json::from_str::<Layouts>(include_str!("./layout.json"))?;

    let layout = layouts.get_layout_by_name("German De").unwrap();

    app.run_async::<App>(AppInit::new(layout.clone(), osk_proxy));

    Ok(())
}
