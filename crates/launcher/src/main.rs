use std::env;

use launcher::App;
use relm4::RelmApp;

fn main() {
    simple_logger::SimpleLogger::new().env().init().unwrap();

    let search_term = env::args().nth(1);
    let app = RelmApp::new("dod-shell.launcher");

    // Running using `with_args` to stop gtk errors caused by trying to parse the command-line
    // arguments itself
    //
    // See: https://relm4.org/book/stable/cli.html
    app.with_args(Vec::new()).run::<App>(search_term);
}
