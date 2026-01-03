use launcher::launch;

fn main() -> zbus::Result<()> {
    simple_logger::SimpleLogger::new().env().init().unwrap();
    launch()
}
