//! The launcher component of the shell
//!
//! The launcher functionality is based on the idea of different [LauncherMode]s. (Modes for short)
//!
//! ## Modes
//!
//! Each mode represents a different functionality of the launcher.
//!
//! Modes are chosen based on a prefix of the search query.
//!
//! WIP: Or in the future by command line arguments.
//!
//! The default mode is [AllMode], which is less so a mode of it's own, but more so a mode to allow
//! the selection of other modes via the prefixes.

use core::str;
use gtk::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell}; // Import the additional types
use relm4::{
    RelmApp,
    actions::{AccelsPlus, RelmAction, RelmActionGroup},
    prelude::*,
};
use std::env;

use common::{Config, classes, config::launcher::LauncherConfig, css::Class};
use daemon::config::ConfigProxy;

mod mode;
mod results;

pub use mode::{AllMode, LauncherMode};
use results::LauncherResults;

/// The main [``relm4::Component``] for the launcher
///
/// For more information see module level docs
#[derive(Default)]
pub struct App {
    /// The results of the search
    results: LauncherResults,
    /// The instance of [AllMode] for the launcher
    mode: AllMode,
    /// Config options
    config: LauncherConfig,
    /// If the viewer is invisible
    invisible: bool,
}

impl App {
    /// Create a new [``Config``] with an already initialized config
    fn new_with_config(config: LauncherConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }
}

relm4::new_action_group!(LauncherActionGroup, "launcher");
relm4::new_stateless_action!(ExitAction, LauncherActionGroup, "exit");
relm4::new_stateless_action!(ResultsMoveUpAction, LauncherActionGroup, "up");
relm4::new_stateless_action!(ResultsMoveDownAction, LauncherActionGroup, "down");

/// Input messages for [App]
#[derive(Debug)]
pub enum AppMsg {
    /// Sent when the search query is updated
    SearchUpdate(String),
    /// Sent when the search query is accepted
    SearchFinish(String),
    /// Move the selected result up by one
    ResultsMoveUp,
    /// Move the selected result down by one
    ResultsMoveDown,
    /// Quit the application
    Quit,
}

/// Widget associated with the [App] component
///
/// Generated with [macro@relm4::component].
#[relm4::component(pub)]
impl Component for App {
    type Init = (
        Option<String>,
        common::config::launcher::LauncherConfig,
        String,
    );
    type Input = AppMsg;
    type Output = ();
    type CommandOutput = ();

    view! {
        #[name(launcher_main_window)]
        gtk::Window {
            init_layer_shell: (), // Do gtk4_layer_shell stuff here
            set_layer: Layer::Overlay,
            #[watch]
            set_visible: !model.invisible,
            auto_exclusive_zone_enable: (),
            set_focusable: true,
            set_keyboard_mode: KeyboardMode::OnDemand,
            set_anchor: (Edge::Top, false),
            set_anchor: (Edge::Left, false),
            set_title: Some("Launcher"),
            set_namespace: Some("dod-shell-launcher"),
            set_default_size: (300, 100),
            set_css_classes: &classes!(MainWindow, LauncherMainWindow),

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 5,
                set_margin_all: 5,
                add_css_class: Class::OuterBox.as_ref(),

                gtk::Box {
                    #[name(main_entry)]
                    gtk::Entry {
                        set_placeholder_text: Some("Enter text..."),
                        connect_changed[sender] => move |this| { sender.input(AppMsg::SearchUpdate(this.text().to_string())); },
                        connect_activate[sender] => move |this| { sender.input(AppMsg::SearchFinish(this.text().to_string())); },
                        add_css_class: Class::MainEntry.as_ref(),
                        set_hexpand: true,
                    },
                    #[name(mode_name)]
                    gtk::Label {
                        add_css_class: Class::ModeName.as_ref(),
                    }
                },

                #[local_ref]
                results_box -> gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    add_css_class: Class::ResultsBox.as_ref(),
                },
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        relm4::set_global_css(&init.2);
        let model = App::new_with_config(init.1);

        let results_box = model.results.results_widget();
        let widgets = view_output!();

        // Make launcher exit on pressing Escape
        let app = relm4::main_application();

        app.set_accelerators_for_action::<ExitAction>(&["Escape"]);
        app.set_accelerators_for_action::<ResultsMoveUpAction>(&["Up"]);
        app.set_accelerators_for_action::<ResultsMoveDownAction>(&["Down"]);

        let exit_action: RelmAction<ExitAction> = RelmAction::new_stateless(move |_| {
            app.quit();
        });

        let mut action_group: RelmActionGroup<LauncherActionGroup> = RelmActionGroup::new();
        action_group.add_action(exit_action);

        // Move up or down in the results when pressing the arrow keys
        let up_sender = sender.clone();
        let up_action: RelmAction<ResultsMoveUpAction> = RelmAction::new_stateless(move |_| {
            let _ = up_sender.input_sender().send(AppMsg::ResultsMoveUp);
        });

        let down_sender = sender.clone();
        let down_action: RelmAction<ResultsMoveDownAction> = RelmAction::new_stateless(move |_| {
            let _ = down_sender.input_sender().send(AppMsg::ResultsMoveDown);
        });

        action_group.add_action(up_action);
        action_group.add_action(down_action);

        action_group.register_for_widget(&widgets.launcher_main_window);

        let mut entry_search = String::default();
        if let Some(initial_search) = init.0 {
            widgets.main_entry.set_text(&initial_search);

            let len = initial_search.len() as i32;
            widgets.main_entry.connect_has_focus_notify(move |e| {
                e.set_position(len);
            });

            entry_search = initial_search;
        }

        sender
            .input_sender()
            .emit(AppMsg::SearchUpdate(entry_search));

        widgets.main_entry.grab_focus();

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            AppMsg::SearchUpdate(text) => {
                self.results
                    .set_results(self.mode.search(&text, &self.config));

                widgets.mode_name.set_text(&self.mode.current_name());
            }
            AppMsg::SearchFinish(text) => {
                self.mode
                    .finish(&text, &self.config, self.results.get_selected_index());
                self.invisible = true;

                let sender = sender.clone();
                tokio::task::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    sender.input(AppMsg::Quit);
                });
            }
            AppMsg::ResultsMoveUp => {
                self.results.decrease_active();
            }
            AppMsg::ResultsMoveDown => {
                self.results.increase_active();
            }
            AppMsg::Quit => {
                relm4::main_application().quit();
            }
        }
        self.update_view(widgets, sender);
    }
}

pub fn launch() -> zbus::Result<()> {
    let handle = std::thread::spawn(|| {
        let rt =
            tokio::runtime::Runtime::new().expect("Should never fail to create tokio runtime.");
        rt.block_on(get_all_config())
    });

    let search_term = env::args().nth(1);
    let app = RelmApp::new("dod-shell.launcher");

    let (config, css) = handle
        .join()
        .expect("Should never fail to join thread here")?;
    // Running using `with_args` to stop gtk errors caused by trying to parse the command-line
    // arguments itself
    //
    // See: https://relm4.org/book/stable/cli.html
    app.with_args(Vec::new())
        .run::<App>((search_term, config, css));

    Ok(())
}

async fn get_all_config() -> zbus::Result<(common::config::launcher::LauncherConfig, String)> {
    let connection = zbus::Connection::session().await?;

    let config_proxy = ConfigProxy::new(&connection).await?;

    let config: Config = toml::from_str(&config_proxy.config().await?)
        .expect("Config string returned by daemon should always be valid.");

    let css = config_proxy.css().await?;

    Ok((config.launcher, css))
}
