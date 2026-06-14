//! Items relating to the top-level bar widget
//!
//! The bar is implemented using a type-state pattern.
//!
//! We distinguish between two types of bars:
//!
//! ## Primary
//!
//! > implementation found in [`primary`]
//!
//! This is the bar that is first created. It has additional responsibilities over all other bars.
//! Primary among these it sets up the [`StateBroker`] and starts its thread.
//!
//! It is also the bar that creates the secondary bars.
//!
//! <div class="note"> The primary bar does not hold direct references to the secondary bars. The other bars
//! are created and then their runtimes are detached from the primary bar. See
//! [`relm4::prelude::AsyncComponentController::detach_runtime`] </div>
//!
//! ## Secondary
//!
//! > implementation found in [`secondary`]
//!
//! These bars are created for every other display other than the primary one.
//!
//! Visually they are identical to the primary and in day to day use this is not something that
//! should ever come up. If there are any differences between the two types of bars this is will in
//! most cases be a bug.
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use hyprland::shared::HyprData;
use relm4::{
    gtk::{
        gdk::Monitor,
        prelude::{ButtonExt, OrientableExt, WidgetExt},
    },
    prelude::*,
};
use std::{marker::PhantomData, sync::Arc};

pub mod primary;
pub mod secondary;
mod state;

use {secondary::Secondary, state::StateBroker};

#[cfg(debug_assertions)]
use gtk4_layer_shell::KeyboardMode;

use common::{classes, config::bar::BarConfig, css::Class};
use daemon::{
    osk::state::StateProxy,
    system_state::{BatteryStatus, ConnectionData, SystemStateData},
};

use crate::{
    icon,
    label_icon::LabelIcon,
    time_playing::{TimePlaying, TimePlayingInput},
    workspaces::Workspaces,
};

/// The main [``relm4::Component``] for the bar
///
/// For more information see module level docs
#[derive(Debug)]
pub struct App<I: Init + 'static> {
    /// The [`Workspaces`] widget
    workspaces: Controller<Workspaces>,
    /// [`TimePlaying`] widget
    time_playing: AsyncController<TimePlaying>,

    /// The system state received from the daemon
    system_state: Arc<SystemStateData>,
    /// The current config received from the daemon
    config: Arc<BarConfig>,

    /// If the osk is currently visible
    osk_active: bool,
    /// If the osk active state is currently locked
    osk_locked: bool,
    /// Proxy for communication with the daemon
    osk_state_proxy: StateProxy<'static>,

    /// Marker to distinguish primary and secondary bars
    _init: PhantomData<I>,
}

/// Types which can be used in initializing and [`App`]
///
/// This is used in the type-state pattern of [`App`]
pub trait Init
where
    Self: Sized + 'static,
{
    /// Initialization specific to the type of bar
    async fn init(&self, sender: AsyncComponentSender<App<Self>>);
}

/// Init Data for [`AppWidgets`]
#[derive(Debug)]
pub struct AppInit<Data>
where
    Data: Init + 'static,
{
    /// Monitor to display the bar on
    monitor: Monitor,
    /// Id of the monitor the bar is on
    monitor_id: i128,
    /// Data specific to the type of bar being initialized
    data: Data,
}

/// Input messages for [App]
#[derive(Debug, Clone)]
pub enum AppMsg {
    /// Received when the [``SystemStateData``] has changed
    UpdatedSystemState(Arc<SystemStateData>),
    /// Received when the [``common::Config``] has changed
    ConfigUpdated(Arc<BarConfig>),
    /// Sent when pressing the osk button
    ToggleOsk,
    /// Received from the daemon when the active state of the osk has changed
    OskActive(bool),
    /// Received from the daemon when the lock state of the osk has changed
    OskLocked(bool),
}

/// Auto-generated widget for [`App`]
// NOTE: Should we allow users to config the icons?
#[allow(
    clippy::float_cmp,
    reason = "Float comparison shouldn't lead to issues in this case"
)]
#[relm4::component(async, pub)]
impl<I: Init + 'static> SimpleAsyncComponent for App<I> {
    type Init = AppInit<I>;
    type Input = AppMsg;
    type Output = ();

    view! {
        /// Main window of the bar
        #[name(bar_main_window)]
        gtk::Window {
            init_layer_shell: (),
            set_layer: Layer::Top,
            set_anchor: (Edge::Top, true),
            set_anchor: (Edge::Right, true),
            set_anchor: (Edge::Left, true),
            set_monitor: Some(&init.monitor),
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
                            #[watch]
                            set_label: &model.set_drive_label(),
                            set_icon: icon::HARDDISK,
                        },
                    },

                    #[local_ref]
                    workspaces_widget -> gtk::Box {}
                },

                #[wrap(Some)]
                set_center_widget = &gtk::Box {
                    add_css_class: Class::Center.as_ref(),
                    #[local_ref]
                    time_playing_widget -> gtk::Box {}
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
                        set_class_active: (Class::Disabled.as_ref(), model.osk_locked),
                        #[watch]
                        set_visible: model.config.show_osk_button,
                        set_icon_name: icon::KEYBOARD_FILLED,
                        connect_clicked => AppMsg::ToggleOsk,
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
                        set_visible: model.config.show_capslock,
                        #[watch]
                        set_class_active: (Class::Active.as_ref(), model.system_state.capslock),
                        set_icon_name: Some(icon::KEYBOARD_CAPS_LOCK),
                    },

                    #[name(numlock_icon)]
                    gtk::Image {
                        set_css_classes: &classes!(Icon, NumLockIcon),
                        #[watch]
                        set_visible: model.config.show_numlock,
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
                            (*model.system_state.battery).as_ref().map_or(icon::BATTERY_MISSING, |battery| if battery.status == BatteryStatus::Charging {
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
                                }),
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
        // We first do all the init that is independent of the type of bar
        let workspaces = hyprland::data::Workspaces::get()
            .unwrap()
            .iter()
            .filter_map(|w| {
                // TODO: Create upstream method to check for special workspaces
                if w.monitor_id.is_some_and(|v| v == init.monitor_id)
                    && !w.name.contains("special:")
                {
                    return Some(w.id);
                }
                None
            })
            .collect();

        let connection = zbus::Connection::session().await.unwrap();

        let model = Self {
            workspaces: Workspaces::builder().launch(workspaces).detach(),
            time_playing: TimePlaying::builder().launch(()).detach(),
            system_state: SystemStateData::default().into(),
            config: common::config::bar::BarConfig::default().into(),
            osk_active: bool::default(),
            osk_locked: bool::default(),
            osk_state_proxy: StateProxy::new(&connection).await.unwrap(),

            _init: PhantomData,
        };

        let workspaces_widget = model.workspaces.widget();
        let time_playing_widget = model.time_playing.widget();
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

        // Then run the init for the concrete type of bar
        init.data.init(sender).await;

        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, msg: Self::Input, _sender: AsyncComponentSender<Self>) {
        match msg {
            AppMsg::UpdatedSystemState(data) => {
                self.system_state = data;

                self.workspaces
                    .sender()
                    .send(crate::workspaces::WorkspacesMsg::UpdateActiveWorkspace(
                        self.system_state.workspace,
                    ))
                    .expect("Failed to send WorkspaceMsg to component.");
            }
            AppMsg::ConfigUpdated(config) => {
                self.config = Arc::clone(&config);

                if self
                    .time_playing
                    .sender()
                    .send(TimePlayingInput::ConfigUpdated(config))
                    .is_err()
                {
                    log::error!("Failed to send config update to TimePlaying component");
                }
            }
            AppMsg::ToggleOsk => match self.osk_state_proxy.active().await {
                Ok(active) => {
                    if let Err(e) = self.osk_state_proxy.set_active(!active).await {
                        log::error!("Failed to set osk active property: {e}");
                    }
                }
                Err(e) => log::error!("Failed to get osk active property: {e}"),
            },
            AppMsg::OskActive(val) => self.osk_active = val,
            AppMsg::OskLocked(val) => self.osk_locked = val,
        }
    }
}

impl<I: Init + 'static> App<I> {
    /// Helper function to set the [`AppWidgets::drive`] label
    fn set_drive_label(&self) -> String {
        self.system_state
            .disks
            .iter()
            .find(|d| d.name == *self.config.disk)
            .map_or_else(
                || {
                    log::error!("Failed to find disk: {}", self.config.disk);
                    log::info!("Available disks: ");

                    for (pos, disk) in self.system_state.disks.iter().enumerate() {
                        log::info!("{}. {}", pos + 1, disk.name);
                    }

                    "Err".to_string()
                },
                |disk| disk.used.to_string(),
            )
    }
}
