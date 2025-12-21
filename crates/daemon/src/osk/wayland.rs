//! Items relating to the communication with the Wayland server (compositor) for the Osk.
//!
//! The main type is [`WaylandInterface`]
use std::{io::Write, os::fd::AsFd, thread};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tempfile::tempfile;
use tokio::sync::mpsc::{Receiver, Sender, channel};
use wayland_client::{
    Connection, Dispatch, EventQueue, WEnum, delegate_noop,
    protocol::{
        wl_keyboard, wl_registry,
        wl_seat::{self, WlSeat},
    },
};
use wayland_protocols::wp::text_input::zv3::client::zwp_text_input_v3::ContentPurpose;
use wayland_protocols_misc::{
    zwp_input_method_v2::client::{
        zwp_input_method_manager_v2,
        zwp_input_method_v2::{self, ZwpInputMethodV2},
    },
    zwp_virtual_keyboard_v1::client::{
        zwp_virtual_keyboard_manager_v1,
        zwp_virtual_keyboard_v1::{self, ZwpVirtualKeyboardV1},
    },
};
use xkbcommon::xkb;
use zbus::zvariant;

use crate::osk::Mod;
use common::types::DeferedInit;

/// State object for [`wayland_client::Dispatch`] methods.
///
/// Also used for sending the actual input of the osk back to the server.
#[derive(Debug)]
pub struct WaylandInterface {
    /// Protocol for the creation of other protocols.
    wl_seat: DeferedInit<WlSeat>,
    /// Protocol for sending characters to the server for input and getting information on the
    /// current input field.
    input_method: DeferedInit<ZwpInputMethodV2>,
    /// Protocol for sending individual key codes and modifiers to the server.
    keyboard: DeferedInit<ZwpVirtualKeyboardV1>,

    /// Sends updates to the Osk-related state
    ///
    /// See [`crate::osk::Osk::state_receiver`]
    state_sender: Sender<WaylandStateMsg>,
}

impl WaylandInterface {
    /// Create a new [`WaylandInterface`]
    ///
    /// ## Note: Before using any methods you must call [`Self::init`].
    pub fn new() -> (Self, Receiver<WaylandStateMsg>) {
        // NOTE: Channel buffer size is rather arbitrary here
        let (rx, tx) = channel(50);

        (
            Self {
                wl_seat: DeferedInit::default(),
                input_method: DeferedInit::default(),
                keyboard: DeferedInit::default(),
                state_sender: rx,
            },
            tx,
        )
    }

    /// Attempts to create all of the Wayland protocols
    pub fn init(&mut self, conn: &Connection) -> Result<EventQueue<Self>> {
        let mut event_queue = conn.new_event_queue();

        let qh = event_queue.handle();

        let display = conn.display();

        display.get_registry(&qh, ());

        event_queue.roundtrip(self)?;

        if !(self.wl_seat.is_set() && self.input_method.is_set() && self.keyboard.is_set()) {
            return Err(
                common::err::Error::WaylandInterfaceFailedInit(format!("{self:#?}")).into(),
            );
        }

        Ok(event_queue)
    }
}

impl WaylandInterface {
    /// Send a [`String`] to the server as user input
    ///
    /// aka. sending "Hello" would be as if the user had typed "H", "e", "l", "l", "o"
    pub fn type_string(&self, utf: String) {
        self.input_method.commit_string(utf);

        self.input_method.commit(1);
    }

    /// Send a single [`char`] to the server as user input
    pub fn type_char(&self, char: char) {
        self.type_string(char.to_string());
    }

    /// Press a key based off of it's key code with a list of mods.
    pub fn press_key_code(&self, key: u32, mods: &[Mod]) {
        let mod_mask = Mod::join_mods(mods);

        self.press_key_code_with_mask(key, mod_mask);
    }

    /// Press a key based off of it's key code with a mod mask.
    pub fn press_key_code_with_mask(&self, key: u32, mod_mask: u32) {
        self.keyboard.modifiers(mod_mask, 0, 0, 0);
        self.keyboard.key(0, key, 1);
        self.keyboard.key(0, key, 0);
        self.keyboard.modifiers(0, 0, 0, 0);
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for WaylandInterface {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: <wl_registry::WlRegistry as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        qh: &wayland_client::QueueHandle<Self>,
    ) {
        const INIT_MSG: &str = "There should be no way for init to be called more than once here.";
        if let wl_registry::Event::Global {
            name, interface, ..
        } = event
        {
            match &interface[..] {
                "wl_seat" => {
                    state
                        .wl_seat
                        .init(registry.bind::<WlSeat, _, _>(name, 1, qh, ()))
                        .expect(INIT_MSG);
                }
                "zwp_input_method_manager_v2" => {
                    let compositor = registry
                        .bind::<zwp_input_method_manager_v2::ZwpInputMethodManagerV2, _, _>(
                            name,
                            1,
                            qh,
                            (),
                        );

                    state
                        .input_method
                        .init(compositor.get_input_method(&state.wl_seat, qh, ()))
                        .expect(INIT_MSG);
                }
                "zwp_virtual_keyboard_manager_v1" => {
                    let compositor = registry
                        .bind::<zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1, _, _>(
                        name,
                        1,
                        qh,
                        (),
                    );

                    let keyboard = compositor.create_virtual_keyboard(&state.wl_seat, qh, ());
                    let keymap: xkb::Keymap = xkb::Keymap::new_from_names(
                        &xkb::Context::new(xkb::CONTEXT_NO_FLAGS),
                        "",
                        "",
                        "de",
                        "",
                        None,
                        xkb::COMPILE_NO_FLAGS,
                    )
                    .expect("Keymap creation should never fail.");

                    let keymap_str = keymap.get_as_string(xkb::KEYMAP_FORMAT_TEXT_V1);

                    let mut tmp = tempfile().expect("Temporary file creation should never fail");
                    tmp.write_all(keymap_str.as_bytes()).unwrap();
                    tmp.flush().unwrap();

                    let size = u32::try_from(keymap_str.len())
                        .expect("Keymap len should never be larger than a u32.");
                    let fd = tmp.as_fd();

                    keyboard.keymap(wl_keyboard::KeymapFormat::XkbV1.into(), fd, size);

                    state.keyboard.init(keyboard).expect(INIT_MSG);
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<zwp_input_method_v2::ZwpInputMethodV2, ()> for WaylandInterface {
    fn event(
        state: &mut Self,
        _proxy: &zwp_input_method_v2::ZwpInputMethodV2,
        event: <zwp_input_method_v2::ZwpInputMethodV2 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        type Event = zwp_input_method_v2::Event;

        let msg = match event {
            Event::Activate => Some(WaylandStateMsg::Active(true)),
            Event::Deactivate => Some(WaylandStateMsg::Active(false)),
            Event::SurroundingText {
                text,
                cursor,
                anchor,
            } => Some(WaylandStateMsg::SurroundingText {
                text,
                cursor,
                anchor,
            }),
            Event::ContentType { hint, purpose } => Some(WaylandStateMsg::ContentType {
                content_hint_bits: hint.into(),
                content_purpose: if let WEnum::Value(value) = purpose {
                    value.into()
                } else {
                    ContentPurposeWrapper::Normal
                },
            }),
            Event::TextChangeCause { .. } | Event::Done => None,
            Event::Unavailable => {
                log::error!(
                    "Wayland Input Method has become unavailable. There is currently no handling of this."
                );
                None
            }
            _ => unimplemented!(),
        };

        // NOTE: Not sure if this the best way to get around blocking on the main tokio thread
        let sender = state.state_sender.clone();
        thread::spawn(move || {
            if let Some(msg) = msg
                && let Err(e) = sender.blocking_send(msg)
            {
                log::error!("Failed to send Wayland state message to Osk interface: {e}");
            }
        });
    }
}

delegate_noop!(WaylandInterface: ignore wl_seat::WlSeat);
delegate_noop!(WaylandInterface: zwp_input_method_manager_v2::ZwpInputMethodManagerV2);
delegate_noop!(WaylandInterface: zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1);
delegate_noop!(WaylandInterface: zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1);

/// Messages sent when there is a change in the Osk / input method state received by
/// [`ZwpInputMethodV2`]
pub enum WaylandStateMsg {
    /// If the Osk is active
    Active(bool),
    /// Information relating to the current text in the input field
    SurroundingText {
        /// The text in the input field
        text: String,
        /// Where the cursor is in that text
        cursor: u32,
        /// Where the text selection begins
        ///
        /// If no text is selected this is the same as `cursor`.
        anchor: u32,
    },
    /// What type of input field the Osk is in
    ContentType {
        /// Bit mask of all content hints current active
        ///
        /// See [`wayland_protocols::wp::text_input::zv3::client::zwp_text_input_v3::ContentHint`]
        content_hint_bits: u32,
        /// The type of content. See [`ContentPurpose`]
        content_purpose: ContentPurposeWrapper,
    },
}

/// Creates a new enum with all the fields of the external enum
macro_rules! wrap_external_enum {
    (
        // name of wrapper enum you want to generate
        $wrapper:ident,
        // external enum you want to wrap
        $external:path,
        // list of variants shared by both
        $( $variant:ident ),* $(,)?
    ) => {

        #[doc = concat!(
            "Wrapper enum automatically generated from `",
            stringify!($external),
            "`."
        )]
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            Serialize,
            Deserialize,
            zvariant::Value,
            zvariant::OwnedValue,
            zvariant::Type,
        )]
        pub enum $wrapper {
            $( $variant ),*
        }

        impl From<$external> for $wrapper {
            fn from(value: $external) -> Self {
                match value {
                    $( <$external>::$variant => Self::$variant, )*
                    _ => unimplemented!(),
                }
            }
        }

        impl From<$wrapper> for $external {
            fn from(value: $wrapper ) -> Self {
                match value {
                    $( <$wrapper>::$variant => Self::$variant, )*
                }
            }
        }
    };
}

wrap_external_enum!(
    ContentPurposeWrapper,
    ContentPurpose,
    Normal,
    Alpha,
    Digits,
    Number,
    Phone,
    Url,
    Email,
    Name,
    Password,
    Pin,
    Date,
    Time,
    Datetime,
    Terminal,
);
