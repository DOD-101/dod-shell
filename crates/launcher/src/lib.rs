use core::str;
use gtk::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell}; // Import the additional types
use relm4::{
    actions::{AccelsPlus, RelmAction, RelmActionGroup},
    prelude::*,
};

mod mode;
mod results;

use mode::{AllMode, MenuMode};
use results::LauncherResults;

pub struct App {
    results: LauncherResults,
    mode: AllMode,
}

relm4::new_action_group!(LauncherActionGroup, "launcher");
relm4::new_stateless_action!(ExitAction, LauncherActionGroup, "exit");
relm4::new_stateless_action!(ResultsMoveUpAction, LauncherActionGroup, "up");

relm4::new_stateless_action!(ResultsMoveDownAction, LauncherActionGroup, "down");

#[derive(Debug)]
pub enum AppMsg {
    SearchUpdate(String),
    SearchFinish(String),
    ResultsMoveUp,
    ResultsMoveDown,
}

#[relm4::component(pub)]
impl SimpleComponent for App {
    type Init = ();
    type Input = AppMsg;
    type Output = ();

    view! {
        #[name(main_window)]
        gtk::Window {
            init_layer_shell: (), // Do gtk4_layer_shell stuff here
            set_layer: Layer::Overlay,
            auto_exclusive_zone_enable: (),
            set_focusable: true,
            set_keyboard_mode: KeyboardMode::OnDemand,
            // set_margin: (Edge::Left, 40),
            set_anchor: (Edge::Top, false),
            set_anchor: (Edge::Left, false),
            set_title: Some("Launcher"),
            set_default_size: (300, 100),

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 5,
                set_margin_all: 5,
                set_css_classes: &["outer-box"],

                #[name(main_entry)]
                gtk::Entry {
                    set_placeholder_text: Some("Enter text..."),
                    connect_changed[sender] => move |this| { sender.input(AppMsg::SearchUpdate(this.text().to_string())); },
                    connect_activate[sender] => move |this| { sender.input(AppMsg::SearchFinish(this.text().to_string())); },
                    set_css_classes: &["main-entry"],
                },

                #[local_ref]
                results_box -> gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_css_classes: &["results-box"],
                },
            }
        }
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = App {
            results: LauncherResults::default(),
            mode: AllMode::new(),
        };

        let results_box = model.results.results.widget();
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

        action_group.register_for_widget(&widgets.main_window);

        relm4::set_global_css(&common::get_css());
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            AppMsg::SearchUpdate(text) => {
                {
                    let mut results = self.results.results.guard();
                    results.clear();
                    self.mode.search(&text).into_iter().for_each(|o| {
                        results.push_back(o);
                    });
                }

                self.results.reset_and_set();
            }
            AppMsg::SearchFinish(text) => {
                self.mode.finish(&text, self.results.get_selected_index());
                relm4::main_application().quit();
            }
            AppMsg::ResultsMoveUp => {
                self.results.decrease_and_set();
            }
            AppMsg::ResultsMoveDown => {
                self.results.increase_and_set();
            }
        }
    }
}
