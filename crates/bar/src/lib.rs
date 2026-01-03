//! The bar component of the shell
//!
//! The bar is useful for displaying general information on the top of the screen.
use relm4::{
    gtk::{
        gdk::{Monitor, prelude::DisplayExt},
        gio::prelude::ListModelExt,
        glib::object::CastNone,
    },
    prelude::*,
};

#[allow(
    clippy::doc_markdown,
    reason = "Upstream issue. Already fixed. Remove this when relm4-icons 10.0.1 is released."
)]
#[allow(
    clippy::missing_docs_in_private_items,
    reason = "Upstream missing docs."
)]
mod icon {
    //! Auto generated icons module
    //!
    //! See `build.rs` for more information.
    include!(concat!(env!("OUT_DIR"), "/icon_names.rs"));

    pub use self::custom::*;
    pub use self::shipped::*;
}

mod app;
mod label_icon;
mod workspaces;

/// Launches the Bar on all monitors
///
/// ## Panics
///
/// If either the main relm4 application panics or if it cannot get the primary (the first)
/// monitor to display the bar on.
pub fn launch_on_all_monitors() {
    let app = RelmApp::new("dod-shell.bar");
    let monitor = relm4::gtk::gdk::Display::default()
        .and_then(|d| d.monitors().item(0).and_downcast::<Monitor>())
        .expect("Failed to get primary Monitor.");

    relm4_icons::initialize_icons(icon::GRESOURCE_BYTES, icon::RESOURCE_PREFIX);

    app.run_async::<app::App<app::primary::Primary>>(app::AppInit::<app::primary::Primary>::new(
        monitor, 0,
    ));
}
