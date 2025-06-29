use gtk4_layer_shell::{Edge, Layer, LayerShell};
use hyprland::shared::HyprData;
use relm4::{
    gtk::{
        gdk::{Monitor, prelude::DisplayExt},
        gio::prelude::{ListModelExt, ListModelExtManual},
        glib::object::CastNone,
        prelude::{GtkApplicationExt, OrientableExt, WidgetExt},
    },
    prelude::*,
};
use time::{OffsetDateTime, macros::format_description};

#[cfg(debug_assertions)]
use gtk4_layer_shell::KeyboardMode;

mod workspaces;
use workspaces::Workspaces;

#[derive(Debug)]
pub struct App {
    workspaces: Controller<Workspaces>,
    time: String,
}

#[derive(Debug)]
pub enum AppMsg {
    UpdateTime(String),
}

#[relm4::component(pub)]
impl SimpleComponent for App {
    type Init = (Monitor, usize, bool);
    type Input = AppMsg;
    type Output = ();

    view! {

        #[name(bar_main_window)]
        gtk::Window {
            init_layer_shell: (),
            set_layer: Layer::Top,
            set_anchor: (Edge::Top, true),
            set_anchor: (Edge::Right, true),
            set_anchor: (Edge::Left, true),
            set_monitor: Some(&init.0),
            set_visible: true,
            set_css_classes: &["bar-main-window"],
            auto_exclusive_zone_enable: (),

            gtk::CenterBox {
                set_orientation: gtk::Orientation::Horizontal,
                // #[wrap(Some)]
                // set_start_widget = &gtk::Box {
                //     gtk::Label {
                //         set_label: &format!("Hello from Monitor: {monitor_id}"),
                //     }
                // },
                set_start_widget: Some(model.workspaces.widget()),

                #[wrap(Some)]
                set_center_widget = &gtk::Box {
                    gtk::Label {
                        #[watch]
                        set_label: &model.time
                    }
                },
                #[wrap(Some)]
                set_end_widget = &gtk::Box {
                    gtk::Label {
                        set_label: "Hello"
                    }
                },

            }
        }

    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let workspaces = hyprland::data::Workspaces::get()
            .unwrap()
            .iter()
            .filter_map(|w| {
                // HACK: The check if the id is greater than 0 is a hack, because hyprland-rs
                // doesn't have a way to check if a workspace is special. This only works because
                // of my convention to have special workspaces be less than 0. !! UPSTREAM PR NEEDED !!
                if w.monitor_id.is_some_and(|w| w == init.1 as i128) && w.id > 0 {
                    return Some(w.id);
                }
                None
            })
            .collect();

        let model = Self {
            workspaces: Workspaces::builder().launch(workspaces).detach(),
            time: String::new(),
        };

        let widgets = view_output!();

        #[cfg(debug_assertions)]
        {
            widgets.bar_main_window.set_focusable(true);
            widgets
                .bar_main_window
                .set_keyboard_mode(KeyboardMode::OnDemand);
        }

        if init.2 {
            let monitor_list = relm4::gtk::gdk::Display::default()
                .expect("Failed to get display")
                .monitors();
            let mut monitors = monitor_list.iter::<Monitor>().flatten().enumerate();

            monitors.next(); // Discard the first monitor, since that is what the main window is on

            let app = relm4::main_application();
            for monitor in monitors {
                let builder = Self::builder();
                app.add_window(&builder.root);

                builder
                    .launch((monitor.1, monitor.0, false))
                    .detach_runtime();
            }
        }

        let time_sender = sender.input_sender().clone();
        relm4::spawn_blocking(move || {
            let format = format_description!("[hour]:[minute]:[second] | [year]-[month]-[day]");
            loop {
                let now = OffsetDateTime::now_local().unwrap();
                let formatted = now.format(&format).unwrap();

                time_sender
                    .send(AppMsg::UpdateTime(formatted))
                    .expect("Failed to send updated time to Bar.");

                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        });

        relm4::set_global_css(&common::get_css());
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            AppMsg::UpdateTime(t) => self.time = t,
        }
    }
}

/// Launches the Bar on all monitors
///
/// ## Panics
///
/// If either the main relm4 application panics or if it cannot get the primary (aka the first)
/// monitor to display the bar on.
pub fn launch_on_all_monitors() {
    let app = RelmApp::new("dod-shell.bar");
    let monitor = relm4::gtk::gdk::Display::default()
        .and_then(|d| d.monitors().item(0).and_downcast::<Monitor>())
        .expect("Failed to get primary Monitor.");

    app.run::<App>((monitor, 0, true));
}
