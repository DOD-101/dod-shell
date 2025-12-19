use futures_util::StreamExt;
use gtk4_layer_shell::{Layer, LayerShell};
use relm4::{
    component::{AsyncComponentParts, AsyncComponentSender},
    gtk::{self, prelude::*},
    prelude::*,
    tokio,
};
use std::error::Error;
use strum::EnumIs;

use common::{Layouts, config::layouts::Layout, css::Class};
use daemon::{
    config::ConfigProxy,
    osk::{Mod, OskProxy, state::StateProxy},
};

use crate::key::{GenericKey, OskKeyInputMsg, OskRow};

#[allow(dead_code)]
mod icon {
    include!(concat!(env!("OUT_DIR"), "/icon_names.rs"));

    #[allow(unused_imports)]
    pub use self::custom::*;
    pub use self::shipped::*;
}

mod key;

#[derive(Debug)]
pub struct App {
    osk_rows: FactoryVecDeque<OskRow>,
    osk_proxy: OskProxy<'static>,
    osk_state_proxy: StateProxy<'static>,

    active: bool,
    active_locked: bool,
    active_mods: u32,
    shift_state: ShiftState,
}

impl App {
    #[must_use]
    pub fn new(init: AppInit<'static>, sender: &relm4::AsyncComponentSender<Self>) -> Self {
        let mut all_osk_rows = FactoryVecDeque::builder()
            .launch_default()
            .forward(sender.input_sender(), AppMsg::KeyPressed);

        {
            let mut all_osk_rows_guard = all_osk_rows.guard();

            for row in init.layout.keys() {
                let mut osk_row: FactoryVecDeque<GenericKey> = FactoryVecDeque::builder()
                    .launch_default()
                    .forward(sender.input_sender(), AppMsg::KeyPressed);

                {
                    let mut row_guard = osk_row.guard();

                    for key in row {
                        row_guard.push_back(key.clone().into());
                    }
                }

                all_osk_rows_guard.push_back(osk_row);
            }
        }

        Self {
            osk_rows: all_osk_rows,
            osk_proxy: init.osk_proxy,
            osk_state_proxy: init.osk_state_proxy,
            active: bool::default(),
            active_locked: bool::default(),
            active_mods: u32::default(),
            shift_state: ShiftState::default(),
        }
    }

    fn send_to_all_keys(&self, message: OskKeyInputMsg) {
        let max_index = self.osk_rows.len();

        for i in 0..max_index {
            self.osk_rows.send(i, message);
        }
    }
}

#[derive(Debug)]
pub enum AppMsg {
    /// Sent by a [`key::GenericKey`] when pressed
    KeyPressed(key::OskKeyOutputMsg),
    /// The css has changed
    CssUpdated(String),
    /// Set [`App::active`]
    Active(bool),
    /// Set [`App::active_locked`]
    ActiveLocked(bool),
    /// Close the osk
    ///
    /// There is no guarantee that sending this actually close the osk. If [`App::active_locked`]
    /// is `true` this won't override it.
    Close,
    /// Toggle [`App::active_locked`]
    Lock,
}

pub struct AppInit<'a> {
    layout: Layout,
    osk_proxy: OskProxy<'a>,
    osk_state_proxy: StateProxy<'a>,
}

impl<'a> AppInit<'a> {
    #[must_use]
    pub fn new(layout: Layout, osk_proxy: OskProxy<'a>, osk_state_proxy: StateProxy<'a>) -> Self {
        Self {
            layout,
            osk_proxy,
            osk_state_proxy,
        }
    }
}

#[relm4::component(pub, async)]
impl SimpleAsyncComponent for App {
    type Init = AppInit<'static>;
    type Input = AppMsg;
    type Output = ();

    view! {
        #[name(osk_main_window)]
        gtk::Window {
            init_layer_shell: (),
            #[watch]
            set_visible: model.active,
            add_css_class: Class::OskMainWindow.as_ref(),
            set_hexpand: true,
            set_anchor:  (gtk4_layer_shell::Edge::Bottom, true),
            set_layer: Layer::Overlay,

            gtk::Box {
                set_height_request: 100,
                set_hexpand: true,
                set_orientation: gtk::Orientation::Vertical,
                set_width_request: 5,

                gtk::Box {
                    set_halign: gtk::Align::End,
                    gtk::Button {
                        add_css_class: Class::OskLockButton.as_ref(),
                        #[watch]
                        set_class_active: (Class::Active.as_ref(), model.active_locked),
                        #[watch]
                        set_icon_name: if model.active_locked { icon::LOCK_SMALL } else { icon::LOCK_SMALL_OPEN },
                        connect_clicked => AppMsg::Lock,
                    },
                    gtk::Button {
                        add_css_class: Class::OskCloseButton.as_ref(),
                        #[watch]
                        set_class_active: (Class::Disabled.as_ref(), model.active_locked),
                        #[watch]
                        set_sensitive: !model.active_locked,
                        set_icon_name: icon::CROSS_SMALL,
                        connect_clicked => AppMsg::Close,
                    },
                },

                #[local_ref]
                row -> gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                }
            }
        }
    }

    async fn init(
        init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let model = App::new(init, &sender);

        let row = model.osk_rows.widget();

        let widgets = view_output!();

        let update_sender = sender.input_sender().clone();

        relm4::spawn(async move {
            // HACK: Fix init of App to avoid needing to create a new dbus connection here
            let connection = zbus::Connection::session().await?;

            let config_proxy = ConfigProxy::new(&connection).await?;
            let osk_state_proxy = StateProxy::new(&connection).await?;

            let mut css_stream = config_proxy.receive_css_changed().await.fuse();
            let mut active_stream = osk_state_proxy.receive_active_changed().await.fuse();
            let mut active_locked_stream =
                osk_state_proxy.receive_active_locked_changed().await.fuse();

            loop {
                futures_util::select! {
                    css = css_stream.select_next_some() => {
                        if update_sender.send(AppMsg::CssUpdated(css.get().await?)).is_err() {
                            log::error!("Failed to update css.");
                        }
                    }
                    active = active_stream.select_next_some() => {
                        if update_sender.send(AppMsg::Active(active.get().await?)).is_err() {
                            log::error!("Failed to send updated `active` state.");
                        }
                    }
                    active_locked = active_locked_stream.select_next_some() => {
                        if update_sender.send(AppMsg::ActiveLocked(active_locked.get().await?)).is_err() {
                            log::error!("Failed to send updated `active_locked` state.");
                        }
                    }
                }
            }

            #[allow(unreachable_code)]
            Ok::<(), zbus::Error>(())
        });

        let monitor = relm4::gtk::gdk::Display::default()
            .and_then(|d| d.monitors().item(0).and_downcast::<gtk::gdk::Monitor>())
            .expect("Failed to get primary Monitor.");

        widgets.osk_main_window.set_monitor(Some(&monitor));

        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, msg: Self::Input, _sender: relm4::AsyncComponentSender<Self>) {
        match msg {
            AppMsg::KeyPressed(k) => match k {
                key::OskKeyOutputMsg::Utf(v) => {
                    self.osk_proxy.type_string(v).await.unwrap();

                    if self.shift_state.is_on() {
                        self.active_mods = Mod::Shift.remove_from(self.active_mods);
                        self.shift_state = self.shift_state.prev();
                        self.send_to_all_keys(OskKeyInputMsg::ActiveMods(
                            self.active_mods,
                            self.shift_state,
                        ));
                    }
                }
                key::OskKeyOutputMsg::Code(v) => {
                    self.osk_proxy
                        .press_key_with_mask(v - 8, self.active_mods)
                        .await
                        .unwrap();

                    if self.shift_state.is_on() {
                        self.active_mods = Mod::Shift.remove_from(self.active_mods);
                        self.shift_state = self.shift_state.prev();
                        self.send_to_all_keys(OskKeyInputMsg::ActiveMods(
                            self.active_mods,
                            self.shift_state,
                        ));
                    }
                }
                key::OskKeyOutputMsg::Mod(v) => {
                    if v.contained_in(self.active_mods) {
                        self.active_mods = v.remove_from(self.active_mods);
                    } else {
                        self.active_mods = v.add_to(self.active_mods);
                    }

                    self.send_to_all_keys(OskKeyInputMsg::ActiveMods(
                        self.active_mods,
                        self.shift_state,
                    ));
                }
                key::OskKeyOutputMsg::Shift => {
                    self.shift_state = self.shift_state.next();

                    if self.shift_state.into() {
                        self.active_mods = Mod::Shift.add_to(self.active_mods);
                    } else {
                        self.active_mods = Mod::Shift.remove_from(self.active_mods);
                    }

                    self.send_to_all_keys(OskKeyInputMsg::ActiveMods(
                        self.active_mods,
                        self.shift_state,
                    ));
                }
            },
            AppMsg::CssUpdated(css) => relm4::set_global_css(&css),
            AppMsg::Active(active) => self.active = active,
            AppMsg::ActiveLocked(active_locked) => self.active_locked = active_locked,
            AppMsg::Close => {
                if self.osk_state_proxy.set_active(false).await.is_err() {
                    log::error!("Failed to send updated `active` to daemon");
                }
            }
            AppMsg::Lock => {
                if self
                    .osk_state_proxy
                    .set_active_locked(!self.active_locked)
                    .await
                    .is_err()
                {
                    log::error!("Failed to send updated `active_locked` to daemon");
                }
            }
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, EnumIs)]
pub enum ShiftState {
    #[default]
    Off,
    On,
    Locked,
}

impl ShiftState {
    fn next(self) -> ShiftState {
        match self {
            Self::Off => Self::On,
            Self::On => Self::Locked,
            Self::Locked => Self::Off,
        }
    }

    #[allow(dead_code)]
    fn prev(self) -> ShiftState {
        match self {
            ShiftState::Off => Self::Locked,
            ShiftState::On => Self::Off,
            ShiftState::Locked => Self::On,
        }
    }
}

impl From<ShiftState> for bool {
    fn from(value: ShiftState) -> Self {
        match value {
            ShiftState::Off => false,
            ShiftState::On | ShiftState::Locked => true,
        }
    }
}

pub fn launch() -> Result<(), Box<dyn Error>> {
    simple_logger::SimpleLogger::new().env().init().unwrap();

    let rt = tokio::runtime::Runtime::new()?;

    let (osk_proxy, osk_state_proxy, layouts) = rt.block_on(async {
        let connection = zbus::Connection::session().await.unwrap();

        (
            daemon::osk::OskProxy::new(&connection).await.unwrap(),
            daemon::osk::state::StateProxy::new(&connection)
                .await
                .unwrap(),
            daemon::config::ConfigProxy::new(&connection)
                .await
                .unwrap()
                .layouts()
                .await
                .unwrap(),
        )
    });

    let app = RelmApp::new("dod-shell.osk");

    let layouts = serde_json::from_str::<Layouts>(&layouts)?;

    let layout = layouts.get_layout_by_name("German De").unwrap();

    relm4_icons::initialize_icons(icon::GRESOURCE_BYTES, icon::RESOURCE_PREFIX);

    app.run_async::<App>(AppInit::new(layout.clone(), osk_proxy, osk_state_proxy));

    Ok(())
}
