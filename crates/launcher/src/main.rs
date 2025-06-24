use launcher::App;
use relm4::RelmApp;

fn main() {
    simple_logger::SimpleLogger::new().env().init().unwrap();
    let app = RelmApp::new("dod-shell.launcher");
    app.run::<App>(());
}
