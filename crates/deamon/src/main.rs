use futures_util::stream::StreamExt;
use zbus::{Connection, Result, conn::Builder, interface, proxy::PropertyChanged};

use deamon::system_state::{SystemState, SystemStateData, SystemStateDataProxy};

mod system_state;

// Although we use `tokio` here, you can use any async runtime of choice.
#[tokio::main]
async fn main() -> Result<()> {
    let system_state = SystemStateData::default();

    let connection = Builder::session()?
        .name("dod.shell.Deamon")?
        .serve_at("/dod/shell/Deamon", system_state)?
        .build()
        .await?;

    let obj_server = connection.object_server();

    let state_iface = obj_server
        .interface::<_, SystemStateData>("/dod/shell/Deamon")
        .await
        .unwrap();

    let iface2 = state_iface.clone().get_mut().await;

    let proxy = SystemStateDataProxy::builder(&connection).build().await?;

    let _ = dbg!(proxy.get_state_data().await);

    loop {
        std::future::pending::<()>().await;
    }
}
