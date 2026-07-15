//! Binary for the launcher. See lib for more information

use common::logger;
use launcher::launch;

fn main() -> zbus::Result<()> {
    logger!();

    launch()
}
