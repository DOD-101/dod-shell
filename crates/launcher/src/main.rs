use relm4::RelmApp;
use std::env;

use deamon::config::ConfigProxy;
use launcher::App;

fn main() -> zbus::Result<()> {
    simple_logger::SimpleLogger::new().env().init().unwrap();

    let search_term = env::args().nth(1);
    let app = RelmApp::new("dod-shell.launcher");

    let rt = tokio::runtime::Runtime::new().unwrap();
    let all_config = rt.block_on(get_all_config())?;

    // Running using `with_args` to stop gtk errors caused by trying to parse the command-line
    // arguments itself
    //
    // See: https://relm4.org/book/stable/cli.html
    app.with_args(Vec::new())
        .run::<App>((search_term, all_config));

    Ok(())
}

async fn get_all_config() -> zbus::Result<deamon::config::Config> {
    let connection = zbus::Connection::session().await?;

    let config_proxy = ConfigProxy::new(&connection).await?;
    let all_config = config_proxy.all_config().await?;

    Ok(all_config)
}
