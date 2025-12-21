use futures_util::StreamExt;
use gtk4_layer_shell::{Layer, LayerShell};
use relm4::{
    component::{AsyncComponentParts, AsyncComponentSender},
    gtk::{self, prelude::*},
    prelude::*,
};
use std::process::exit;
use strum::EnumIs;

use common::{Layouts, css::Class};
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

    layouts: Layouts,
    layout_index: Option<usize>,

    active: bool,
    active_locked: bool,
    active_mods: u32,
    shift_state: ShiftState,
}

trait AppErrExt<T> {
    fn abort_on_err(self) -> T;
}

impl<T, E: std::error::Error> AppErrExt<T> for Result<T, E> {
    fn abort_on_err(self) -> T {
        match self {
            Ok(v) => v,
            Err(e) => {
                log::error!("Failed to init app: {e}");

                exit(1)
            }
        }
    }
}

impl App {
    fn send_to_all_keys(&self, message: OskKeyInputMsg) {
        let max_index = self.osk_rows.len();

        for i in 0..max_index {
            self.osk_rows.send(i, message);
        }
    }

    fn update_layout(&mut self, sender: &relm4::AsyncComponentSender<Self>) {
        let Some(layout_index) = self.layout_index else {
            return;
        };

        let layout = &self.layouts.layouts()[layout_index];

        let all_rows = &mut self.osk_rows;

        {
            let mut all_osk_rows_guard = all_rows.guard();

            all_osk_rows_guard.clear();

            for row in layout.keys() {
                let mut osk_row: FactoryVecDeque<GenericKey> = FactoryVecDeque::builder()
                    .launch_default()
                    .forward(sender.input_sender(), AppMsg::KeyPressed);

                {
                    let mut row_guard = osk_row.guard();

                    row_guard.clear();

                    for key in row {
                        row_guard.push_back(key.clone().into());
                    }
                }

                all_osk_rows_guard.push_back(osk_row);
            }
        }
    }
}

#[derive(Debug)]
pub enum AppMsg {
    /// Sent by a [`key::GenericKey`] when pressed
    KeyPressed(key::OskKeyOutputMsg),
    /// The css has changed
    CssUpdated(String),
    /// The layouts have changed
    LayoutsUpdated(Layouts),
    /// Set [`App::active`]
    Active(bool),
    /// Set [`App::active_locked`]
    ActiveLocked(bool),
    /// Close the osk
    ///
    /// There is no guarantee that sending this actually closes the osk. If [`App::active_locked`]
    /// is `true` this won't override it.
    Close,
    /// Toggle [`App::active_locked`]
    Lock,
}

#[relm4::component(pub, async)]
impl SimpleAsyncComponent for App {
    type Init = ();
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
                    add_css_class: Class::OskMainBox.as_ref(),
                    set_orientation: gtk::Orientation::Vertical,
                }
            }
        }
    }

    async fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let connection = zbus::Connection::session().await.abort_on_err();

        let config_proxy = ConfigProxy::new(&connection).await.abort_on_err();
        let osk_proxy = OskProxy::new(&connection).await.abort_on_err();
        let osk_state_proxy = StateProxy::new(&connection).await.abort_on_err();

        let mut layouts_stream = config_proxy.receive_layouts_changed().await.fuse();
        let mut css_stream = config_proxy.receive_css_changed().await.fuse();
        let mut active_stream = osk_state_proxy.receive_active_changed().await.fuse();
        let mut active_locked_stream = osk_state_proxy.receive_active_locked_changed().await.fuse();

        let model = {
            let all_osk_rows = FactoryVecDeque::builder()
                .launch_default()
                .forward(sender.input_sender(), AppMsg::KeyPressed);

            let layouts: Layouts = serde_json::from_str(
                &layouts_stream
                    .select_next_some()
                    .await
                    .get()
                    .await
                    .abort_on_err(),
            )
            .abort_on_err();

            let mut app = Self {
                osk_rows: all_osk_rows,
                osk_proxy,
                osk_state_proxy,
                layout_index: layouts.get_default_layout_index(),
                layouts,
                active: bool::default(),
                active_locked: bool::default(),
                active_mods: u32::default(),
                shift_state: ShiftState::default(),
            };

            app.update_layout(&sender);

            app
        };

        let row = model.osk_rows.widget();

        let widgets = view_output!();

        let update_sender = sender.input_sender().clone();

        relm4::spawn(async move {
            loop {
                if futures_util::select! {
                    css = css_stream.select_next_some() => {
                        update_sender.send(AppMsg::CssUpdated(css.get().await?))
                    }
                    active = active_stream.select_next_some() => {
                        update_sender.send(AppMsg::Active(active.get().await?))
                    }
                    active_locked = active_locked_stream.select_next_some() => {
                        update_sender.send(AppMsg::ActiveLocked(active_locked.get().await?))
                    }
                    layouts = layouts_stream.select_next_some() => {
                        update_sender.send(AppMsg::LayoutsUpdated(serde_json::from_str(&layouts.get().await?)
                            .expect("Should never fail to parse layout")))
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

        let monitor = relm4::gtk::gdk::Display::default()
            .and_then(|d| d.monitors().item(0).and_downcast::<gtk::gdk::Monitor>())
            .expect("Failed to get primary Monitor.");

        widgets.osk_main_window.set_monitor(Some(&monitor));

        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, msg: Self::Input, sender: relm4::AsyncComponentSender<Self>) {
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
                key::OskKeyOutputMsg::SwitchLayout => {
                    if let Some(layout_index) = self.layout_index.as_mut() {
                        *layout_index += 1;

                        if *layout_index >= self.layouts.layouts().len() {
                            *layout_index = 0;
                        }

                        self.update_layout(&sender);
                    }
                }
            },
            AppMsg::CssUpdated(css) => relm4::set_global_css(&css),
            AppMsg::LayoutsUpdated(layouts) => {
                self.layouts = layouts;

                if let Some(layout_index) = self.layout_index.as_mut()
                    && *layout_index >= self.layouts.layouts().len()
                {
                    *layout_index = 0;
                }

                self.update_layout(&sender);
            }
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

/// Main entry point for launching the osk
///
/// ## Errors
///
/// This function errors if there is are any problems with:
///
/// 1. Creating a tokio runtime
///
/// 2. Getting the needed state from the daemon
#[allow(clippy::missing_panics_doc)]
pub fn launch() {
    simple_logger::SimpleLogger::new()
        .env()
        .init()
        .expect("Should never fail to init logger.");

    let app = RelmApp::new("dod-shell.osk");
    relm4_icons::initialize_icons(icon::GRESOURCE_BYTES, icon::RESOURCE_PREFIX);

    app.run_async::<App>(());
}
