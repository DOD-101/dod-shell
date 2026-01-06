use common::logger;
use launcher::launch;

fn main() -> zbus::Result<()> {
    logger!();

    launch()
}
