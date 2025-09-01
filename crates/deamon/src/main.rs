use futures_util::stream::StreamExt;
use zbus::{Connection, Result, conn::Builder, interface, proxy::PropertyChanged};

use deamon::system_state::{SystemState, SystemStateData, SystemStateDataProxy};

// Although we use `tokio` here, you can use any async runtime of choice.
#[tokio::main]
async fn main() -> Result<()> {
    let system_state = SystemStateData::default();

    let connection = Builder::session()?
        .name("dod.shell.Deamon")?
        .serve_at("/dod/shell/Deamon", system_state)?
        .build()
        .await?;

    let proxy = SystemStateDataProxy::builder(&connection).build().await?;

    let _ = dbg!(proxy.get_state_data().await);

    loop {
        std::future::pending::<()>().await;
    }
}
