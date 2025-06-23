use std::{fs, path::PathBuf, sync::LazyLock};

use gtk::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell}; // Import the additional types
use relm4::prelude::*;

static CONFIG_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    if cfg!(debug_assertions) {
        return PathBuf::from("test");
    }

    return dirs::config_dir()
        .expect("Failed to get config dir.")
        .join("dod-shell");
});

mod mode;

use mode::{AllMode, MenuMode};

struct App {
    options: FactoryVecDeque<LaunchOption>,
    mode: AllMode,
}

#[derive(Debug)]
struct LaunchOption {
    label: String,
}

// #[derive(Debug)]
// struct LaunchMsg {}
//
//

#[relm4::factory]
impl FactoryComponent for LaunchOption {
    type Init = String;
    type Input = ();
    type Output = ();
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        #[name(launch_option_label)]
        gtk::Label {
            set_label: &self.label,
        }
    }

    fn init_model(label: Self::Init, _index: &Self::Index, _sender: FactorySender<Self>) -> Self {
        Self { label }
    }
}

#[derive(Debug)]
enum AppMsg {
    SearchUpdate(String),
    SearchFinish(String),
}

#[relm4::component]
impl SimpleComponent for App {
    type Init = ();
    type Input = AppMsg;
    type Output = ();

    view! {
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
                set_css_classes: &["outer_box"],

                #[name(main_entry)]
                gtk::Entry {
                    set_placeholder_text: Some("Enter text..."),
                    connect_changed[sender] => move |this| { sender.input(AppMsg::SearchUpdate(this.text().to_string())); },
                    connect_activate[sender] => move |this| { sender.input(AppMsg::SearchFinish(this.text().to_string())); },
                    set_css_classes: &["main-entry"],
                },

                #[local_ref]
                options_box -> gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_css_classes: &["options_box"],
                },
            }
        }
    }

    fn init(
        _created_options: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let options = FactoryVecDeque::builder()
            .launch(gtk::Box::default())
            .forward(sender.input_sender(), |output| match output {
                _ => todo!(),
            });

        let model = App {
            options,
            mode: AllMode::new(),
        };

        let options_box = model.options.widget();
        // Insert the code generation of the view! macro here
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            AppMsg::SearchUpdate(text) => {
                let mut options = self.options.guard();
                options.clear();
                self.mode.search(&text).into_iter().for_each(|o| {
                    options.push_back(o);
                });
            }
            AppMsg::SearchFinish(text) => {
                self.mode.finish(&text);
                relm4::main_application().quit();
            }
        }
    }
}

fn get_css() -> String {
    match fs::read_to_string(CONFIG_PATH.join("style.scss")) {
        Ok(scss) => grass::from_string(scss, &grass::Options::default()).unwrap_or_else(|e| {
            log::error!(
                "Failed to parse scss. Not applying any css. SassError: {}",
                *e
            );
            String::new()
        }),
        Err(e) => {
            log::error!(
                "Failed to read style.scss. Not applying any css. IoError: {}",
                e
            );
            String::new()
        }
    }
}

fn main() {
    simple_logger::SimpleLogger::new().env().init().unwrap();
    let app = RelmApp::new("dod-shell.launcher");
    relm4::set_global_css(&get_css());
    app.run::<App>(());
}
