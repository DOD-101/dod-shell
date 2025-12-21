//! Binary for the bar. See lib for more information

use bar::launch_on_all_monitors;

fn main() {
    simple_logger::SimpleLogger::new().env().init().unwrap();
    launch_on_all_monitors();
}
