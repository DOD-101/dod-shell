//! Binary for the bar. See lib for more information

use bar::launch_on_all_monitors;
use common::logger;

fn main() {
    logger!();

    launch_on_all_monitors();
}
