use daemon::{config::ConfigProxy, osk::OskProxy};
use futures_util::StreamExt;
use gtk4_layer_shell::{Layer, LayerShell};
use relm4::{
    component::{AsyncComponentParts, AsyncComponentSender},
    gtk::{self, prelude::*},
    prelude::*,
};

use crate::{
    key::{GenericKey, OskKeyInputMsg, OskRow, symbol::ActiveSymbol},
    layouts::Layout,
};

mod key;
pub mod layouts;

#[derive(Debug)]
pub struct App {
    osk_rows: FactoryVecDeque<OskRow>,
    osk_proxy: OskProxy<'static>,

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

    fn calculate_active_symbol(&self) -> ActiveSymbol {
        let mods = self.active_mods;

        if daemon::osk::Mod::Alt.contained_in(mods) || daemon::osk::Mod::AltGr.contained_in(mods) {
            return ActiveSymbol::Alt;
        }

        if daemon::osk::Mod::Shift.contained_in(mods) {
            return ActiveSymbol::Shift;
        }

        ActiveSymbol::Default
    }
}

#[derive(Debug)]
pub enum AppMsg {
    KeyPressed(key::OskKeyOutputMsg),
    /// Sent when the css has been changed
    CssUpdated(String),
}

pub struct AppInit<'a> {
    layout: Layout,
    osk_proxy: OskProxy<'a>,
}

impl<'a> AppInit<'a> {
    #[must_use]
    pub fn new(layout: Layout, osk_proxy: OskProxy<'a>) -> Self {
        Self { layout, osk_proxy }
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
            add_css_class: "osk-main-window",
            set_hexpand: true,
            set_anchor:  (gtk4_layer_shell::Edge::Bottom, true),
            set_layer: Layer::Overlay,

            gtk::Box {
                set_height_request: 100,
                set_hexpand: true,
                set_orientation: gtk::Orientation::Vertical,
                set_width_request: 5,

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
            let connection = zbus::Connection::session().await?;

            let config_proxy = ConfigProxy::new(&connection).await?;

            let mut css_stream = config_proxy.receive_css_changed().await;

            loop {
                if let Some(item) = css_stream.next().await
                    && update_sender
                        .send(AppMsg::CssUpdated(item.get().await?))
                        .is_err()
                {
                    log::error!("Failed to update css.");
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
                }
                key::OskKeyOutputMsg::Code(v) => self
                    .osk_proxy
                    .press_key_with_mask(v - 8, self.active_mods)
                    .await
                    .unwrap(),
                key::OskKeyOutputMsg::Mod(v) => {
                    if v.contained_in(self.active_mods) {
                        self.active_mods = v.remove_from(self.active_mods);
                    } else {
                        self.active_mods = v.add_to(self.active_mods);
                    }

                    self.send_to_all_keys(OskKeyInputMsg::ActiveSymbol(
                        self.calculate_active_symbol(),
                    ));
                }
                key::OskKeyOutputMsg::Shift => {
                    self.shift_state = self.shift_state.next();

                    if self.shift_state.into() {
                        self.active_mods = daemon::osk::Mod::Shift.add_to(self.active_mods);
                    } else {
                        self.active_mods = daemon::osk::Mod::Shift.remove_from(self.active_mods);
                    }

                    self.send_to_all_keys(OskKeyInputMsg::ActiveSymbol(
                        self.calculate_active_symbol(),
                    ));
                }
            },
            AppMsg::CssUpdated(css) => relm4::set_global_css(&css),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd)]
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
