use gtk4_layer_shell::{Edge, Layer, LayerShell};
use relm4::{
    gtk::{
        gdk::{Monitor, prelude::DisplayExt},
        gio::prelude::ListModelExt,
        gio::prelude::ListModelExtManual,
        glib::object::CastNone,
        prelude::{GtkApplicationExt, OrientableExt, WidgetExt},
    },
    prelude::*,
};

#[derive(Debug)]
pub struct App {}

#[derive(Debug)]
pub enum AppMsg {}

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
            set_focusable: false,
            set_anchor: (Edge::Top, true),
            set_anchor: (Edge::Right, true),
            set_anchor: (Edge::Left, true),
            set_monitor: Some(&init.0),
            set_visible: true,
            set_css_classes: &["bar-main-window"],
            auto_exclusive_zone_enable: (),

            gtk::CenterBox {
                set_orientation: gtk::Orientation::Horizontal,
                #[wrap(Some)]
                set_start_widget = &gtk::Box {
                    gtk::Label {
                        set_label: &format!("Hello from Monitor: {monitor_id}"),
                    }
                },
                #[wrap(Some)]
                set_center_widget = &gtk::Box {
                    gtk::Label {
                        set_label: "Hello"
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
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let monitor_id = init.1;

        let model = App {};
        let widgets = view_output!();

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

        relm4::set_global_css(&common::get_css());
        ComponentParts { model, widgets }
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
