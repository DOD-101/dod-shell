//! The bar component of the shell
//!
//! The bar is useful for displaying general information on the top of the screen.
use futures_util::StreamExt;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use hyprland::shared::HyprData;
use relm4::{
    gtk::{
        gdk::{Monitor, prelude::DisplayExt},
        gio::prelude::{ListModelExt, ListModelExtManual},
        glib::object::CastNone,
        prelude::{ButtonExt, GtkApplicationExt, OrientableExt, WidgetExt},
    },
    prelude::*,
};
use time::{OffsetDateTime, UtcOffset, macros::format_description};

#[cfg(debug_assertions)]
use gtk4_layer_shell::KeyboardMode;

use common::{Config, classes, css::Class};
use daemon::{
    config::ConfigProxy,
    osk::state::StateProxy,
    system_state::{BatteryStatus, ConnectionData, SystemStateData, SystemStateProxy},
};

mod icon {
    include!(concat!(env!("OUT_DIR"), "/icon_names.rs"));
    pub use self::custom::*;
    pub use self::shipped::*;
}

mod label_icon;
mod workspaces;

use label_icon::LabelIcon;
use workspaces::Workspaces;

// TODO: Users should be able to adjust this format
const DATE_TIME_FORMAT: &[time::format_description::BorrowedFormatItem<'_>] =
    format_description!("[hour]:[minute]:[second] | [year]-[month]-[day]");

/// The main [``relm4::Component``] for the bar
///
/// For more information see module level docs
#[derive(Debug)]
pub struct App {
    workspaces: Controller<Workspaces>,
    system_state: SystemStateData,
    config: Config,

    osk_active: bool,
    osk_state_proxy: StateProxy<'static>,
}

/// Input messages for [App]
#[derive(Debug)]
pub enum AppMsg {
    /// Sent when the [``SystemStateData``] has been changed
    UpdatedSystemState(SystemStateData),
    /// Sent when the [``Config``] has been changed
    // NOTE: Should we generalize this updating of the config for all components of type 1
    // We could use another enum and then have a function wich takes a type T (aka an AppMsg enum)
    // wich impls From<GeneralConfigUpdateEnum>?
    ConfigUpdated(Config),
    /// Sent when the css has been changed
    CssUpdated(String),

    LaunchOsk,
    OskActive(bool),
}

// NOTE: Should we allow users to config the icons?
#[allow(clippy::float_cmp)]
#[relm4::component(pub, async)]
impl SimpleAsyncComponent for App {
    /// (The monitor to display the bar on, the monitor id the bar is on, if this is the main bar)
    // NOTE: We should add a BarInit (or AppInit) struct or type alias
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
            set_css_classes: &classes!(MainWindow, BarMainWindow),
            auto_exclusive_zone_enable: (),

            gtk::CenterBox {
                set_orientation: gtk::Orientation::Horizontal,
                add_css_class: Class::BarCenterbox.as_ref(),

                #[wrap(Some)]
                set_start_widget = &gtk::Box {
                    add_css_class: Class::Left.as_ref(),

                    gtk::Box {
                        add_css_class: Class::HardwareInfo.as_ref(),

                        #[name(cpu)]
                        LabelIcon {
                            add_css_class: Class::Cpu.as_ref(),
                            #[watch]
                            set_label: &model.system_state.cpu_usage.to_string(),
                            set_icon: icon::PROCESSOR,
                        },

                        #[name(ram)]
                        LabelIcon {
                            add_css_class: Class::Ram.as_ref(),
                            #[watch]
                            set_label: &model.system_state.mem_usage.to_string(),
                            set_icon: icon::RAM_FILLED,
                        },

                        #[name(drive)]
                        LabelIcon {
                            // TODO: There should be a way for the user to know which disks are available
                            #[watch]
                            set_label: &model
                                        .system_state
                                        .disks
                                        .iter()
                                        .find(|d| d.name == *model.config.bar.disk)
                                        .map_or("Err".to_string(), |d| d.used.to_string())
                                        ,
                            set_icon: icon::HARDDISK,
                        },
                    },

                    #[local_ref]
                    workspaces_widget -> gtk::Box {}
                },

                #[wrap(Some)]
                set_center_widget = &gtk::Box {
                    add_css_class: Class::Center.as_ref(),
                    #[name(date_time)]
                    gtk::Label {
                        #[watch]
                        set_label: &OffsetDateTime::from_unix_timestamp(model.system_state.time)
                                        .expect("Unix timestamp from daemon should always be valid")
                                        .to_offset(
                                            UtcOffset::current_local_offset()
                                            .inspect_err(|e| log::error!("Failed to get local offset: {e}"))
                                            .unwrap_or(UtcOffset::UTC))
                                        .format(&DATE_TIME_FORMAT).unwrap()
                    }
                },

                #[wrap(Some)]
                #[name(end_box)]
                set_end_widget = &gtk::Box {
                    add_css_class: Class::Right.as_ref(),
                    set_orientation: gtk::Orientation::Horizontal,

                    gtk::Button {
                        set_css_classes: &classes!(OskButton, Icon),
                        #[watch]
                        set_class_active: (Class::Active.as_ref(), model.osk_active),
                        #[watch]
                        set_visible: model.config.bar.show_osk_button,
                        set_icon_name: icon::KEYBOARD_FILLED,
                        connect_clicked => AppMsg::LaunchOsk,
                    },

                    #[name(internet_revealer)]
                    gtk::Revealer {
                        add_css_class: Class::InternetNameRevealer.as_ref(),
                        set_transition_type: gtk::RevealerTransitionType::SlideRight,
                        gtk::Label {
                            #[watch]
                            set_label: if let ConnectionData::Wireless { ssid, .. } = &model.system_state.network {
                                        &ssid } else { "" }

                        }
                    },
                    #[name(internet_icon)]
                    gtk::Image {
                        set_css_classes: &classes!(Icon, InternetIcon),
                        #[watch]
                        set_class_active: (Class::Active.as_ref(), model.system_state.network != ConnectionData::None),
                        #[watch]
                        set_icon_name: Some(match model.system_state.network {
                                        ConnectionData::Wired => icon::LAN,
                                        ConnectionData::Wireless {signal, ..} => match *signal {
                                            0.75.. => icon::RADIOWAVES_1,
                                            0.50.. => icon::RADIOWAVES_2,
                                            0.35.. => icon::RADIOWAVES_3,
                                            _ => icon::RADIOWAVES_4,
                                        },
                                        ConnectionData::None =>  icon::RADIOWAVES_5,
                        }),
                    },

                    #[name(bluetooth_icon)]
                    gtk::Image {
                        set_css_classes: &classes!(Icon, BluetoothIcon),
                        #[watch]
                        set_class_active: (Class::Active.as_ref(), model.system_state.bluetooth),
                        set_icon_name: Some(icon::BLUETOOTH),
                    },

                    #[name(capslock_icon)]
                    gtk::Image {
                        set_css_classes: &classes!(Icon, CapsLockIcon),
                        #[watch]
                        set_visible: model.config.bar.show_capslock,
                        #[watch]
                        set_class_active: (Class::Active.as_ref(), model.system_state.capslock),
                        set_icon_name: Some(icon::KEYBOARD_CAPS_LOCK),
                    },

                    #[name(numlock_icon)]
                    gtk::Image {
                        set_css_classes: &classes!(Icon, NumLockIcon),
                        #[watch]
                        set_visible: model.config.bar.show_numlock,
                        #[watch]
                        set_class_active: (Class::Active.as_ref(), model.system_state.numlock),
                        set_icon_name: Some(icon::DOCUMENT_PAGE_NUMBER_FILLED_SYMBOLIC),
                    },

                    #[name(volume_label_icon)]
                    LabelIcon {
                        #[watch]
                        set_label: &(if *model.system_state.volume > 0.0 { model.system_state.volume.to_string() } else { String::new() }),
                        #[watch]
                        set_class_active: (Class::Muted.as_ref(), *model.system_state.volume == -1.0),
                        #[watch]
                        set_icon: match *model.system_state.volume {
                                    -1.0 => icon::SPEAKER_OFF_FILLED,
                                    _ if model.system_state.bluetooth => icon::SPEAKER_BLUETOOTH_FILLED_SYMBOLIC,
                                    0.0 => icon::SPEAKER_MUTE_FILLED,
                                    0.66.. => icon::SPEAKER_2_FILLED,
                                    0.33.. => icon::SPEAKER_1_FILLED,
                                    0.0.. => icon::SPEAKER_0_FILLED,
                                    _ => unreachable!(),
                                  }
                    },
                    #[name(battery_label_icon)]
                    LabelIcon {
                        #[watch]
                        set_label:
                            &model
                                .system_state
                                .battery
                                .as_ref()
                                .map(|v| v.charge.to_string())
                                .unwrap_or_default(),
                        #[watch]
                        set_visible: model.system_state.battery.is_some(),
                        #[watch]
                        set_class_active: (Class::BatteryLow.as_ref(), model.system_state.battery.as_ref().is_some_and(|b| *b.charge <= 0.3)),
                        #[watch]
                        set_icon:
                            if let Some(battery) = &*model.system_state.battery {
                                if battery.status == BatteryStatus::Charging {
                                    if *battery.charge == 1.0 {
                                        icon::BATTERY_LEVEL_100_CHARGED
                                    } else {
                                        icon::BATTERY_LEVEL_0_CHARGING
                                    }
                                } else {
                                    match *battery.charge {
                                        1.0.. => icon::BATTERY_LEVEL_100,
                                        0.9.. => icon::BATTERY_LEVEL_90,
                                        0.8.. => icon::BATTERY_LEVEL_80,
                                        0.7.. => icon::BATTERY_LEVEL_70,
                                        0.6.. => icon::BATTERY_LEVEL_60,
                                        0.5.. => icon::BATTERY_LEVEL_50,
                                        0.4.. => icon::BATTERY_LEVEL_40,
                                        0.3.. => icon::BATTERY_LEVEL_30,
                                        0.0.. => icon::BATTERY_LOW,
                                        _ => unreachable!("Battery value should never be negative"),
                                    }
                                }
                            } else {
                                icon::BATTERY_MISSING
                            },
                    },
                }
            }
        }
    }

    async fn init(
        init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let workspaces = hyprland::data::Workspaces::get()
            .unwrap()
            .iter()
            .filter_map(|w| {
                // INFO: Could create upstream method to check for special workspaces
                if w.monitor_id == init.1 as i128 && !w.name.contains("special:") {
                    return Some(w.id);
                }
                None
            })
            .collect();

        let connection = zbus::Connection::session().await.unwrap();

        let model = Self {
            workspaces: Workspaces::builder().launch(workspaces).detach(),
            system_state: SystemStateData::default(),
            config: Config::default(),
            osk_active: bool::default(),
            osk_state_proxy: StateProxy::new(&connection).await.unwrap(),
        };

        // NOTE: We should probably generalize this to all type 1 components and move it to common
        // See comment above.
        //
        // NOTE: It might also be good to not spawn this for all bars but have one thread for all
        // of them and then send it to all bars
        let update_sender = sender.input_sender().clone();
        relm4::spawn(async move {
            let config_proxy = ConfigProxy::new(&connection).await?;
            let state_proxy = SystemStateProxy::new(&connection).await?;
            let osk_state_proxy = StateProxy::new(&connection).await?;

            let mut state_stream = state_proxy.receive_state_data_changed().await.fuse();
            let mut config_stream = config_proxy.receive_config_changed().await.fuse();
            let mut css_stream = config_proxy.receive_css_changed().await.fuse();
            let mut osk_active_stream = osk_state_proxy.receive_active_changed().await.fuse();

            loop {
                if futures_util::select! {
                    c = config_stream.select_next_some() => {
                        let config = toml::from_str(&c.get().await?)
                            .expect("Config string returned by daemon should always be valid.");

                        update_sender.send(AppMsg::ConfigUpdated(config))
                    }
                    c = css_stream.select_next_some() => {
                        update_sender.send(AppMsg::CssUpdated(c.get().await?))
                    }
                    s = state_stream.select_next_some() => {
                        update_sender.send(AppMsg::UpdatedSystemState(s.get().await?))
                    }
                    active = osk_active_stream.select_next_some() => {
                        update_sender.send(AppMsg::OskActive(active.get().await?))
                    }
                }
                .is_err()
                {
                    log::error!("Failed processing update from daemon");
                }
            }

            #[allow(unreachable_code)]
            Ok::<(), zbus::Error>(())
        });

        let workspaces_widget = model.workspaces.widget();
        let widgets = view_output!();

        let internet_controller = gtk::EventControllerMotion::new();
        let ir1 = widgets.internet_revealer.clone();
        internet_controller.connect_enter(move |_, _, _| {
            ir1.set_reveal_child(true);
        });
        let ir2 = widgets.internet_revealer.clone();
        internet_controller.connect_leave(move |_| {
            ir2.set_reveal_child(false);
        });
        widgets.internet_icon.add_controller(internet_controller);

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

        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, msg: Self::Input, _sender: AsyncComponentSender<Self>) {
        match msg {
            AppMsg::UpdatedSystemState(data) => {
                self.system_state = data;

                self.workspaces
                    .sender()
                    .send(workspaces::WorkspacesMsg::UpdateActiveWorkspace(
                        self.system_state.workspace,
                    ))
                    .expect("Failed to send WorkspaceMsg to component.");
            }
            AppMsg::ConfigUpdated(config) => self.config = config,
            AppMsg::CssUpdated(css) => relm4::set_global_css(&css),
            AppMsg::LaunchOsk => match self.osk_state_proxy.active().await {
                Ok(active) => {
                    if let Err(e) = self.osk_state_proxy.set_active(!active).await {
                        log::error!("Failed to set osk active property: {e}");
                    }
                }
                Err(e) => log::error!("Failed to get osk active property: {e}"),
            },
            AppMsg::OskActive(val) => self.osk_active = val,
        }
    }
}

/// Launches the [Bar](``App``) on all monitors
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

    app.run_async::<App>((monitor, 0, true));
}
