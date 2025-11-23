//! Wayland state relating to the Osk
//!
//! The main type is [`State`]
use crate::osk::wayland::{ContentPurposeWrapper, WaylandStateMsg};

use serde::{Deserialize, Serialize};
use zbus::{interface, zvariant};

/// Wayland state relating to the Osk
///
/// ## Dbus
///
/// This struct implements [``zbus::object_server::Interface``], which means it acts as a dbus
/// interface. For available zbus methods and properties see [``StateProxy``]
#[derive(
    Debug,
    Clone,
    zvariant::Value,
    zvariant::OwnedValue,
    zvariant::Type,
    Serialize,
    Deserialize,
    PartialEq,
)]
pub struct State {
    /// If the Osk is active
    active: bool,
    /// The text in the input field
    text: String,
    /// Where the cursor is in the input field
    cursor: u32,
    /// Where the current text selection begins
    ///
    /// If no text is selected this is the same as [`field@Self::cursor`].
    anchor: u32,
    /// Bit mask of all content hints current active
    ///
    /// See [`wayland_protocols::wp::text_input::zv3::client::zwp_text_input_v3::ContentHint`]
    content_hint_bits: u32,
    /// The type of content
    ///
    /// See [`wayland_protocols::wp::text_input::zv3::client::zwp_text_input_v3::ContentPurpose`]
    content_purpose: ContentPurposeWrapper,
}

impl Default for State {
    fn default() -> Self {
        Self {
            active: bool::default(),
            text: String::default(),
            cursor: u32::default(),
            anchor: u32::default(),
            content_hint_bits: u32::default(),
            content_purpose: ContentPurposeWrapper::Normal,
        }
    }
}

impl State {
    pub async fn update_from_msg(
        &mut self,
        ctxt: &zbus::object_server::SignalEmitter<'_>,
        msg: WaylandStateMsg,
    ) -> zbus::Result<()> {
        let mut changed = false;

        macro_rules! changed_attrs {
            ( $( $attr:ident ),* $(,)? ) => {
                $(
                paste::paste! {
                    if self.$attr != $attr {
                        self.$attr = $attr;
                        self.[<$attr _changed>](ctxt).await?;
                        changed = true;
                    }
                }
                )*
            };
        }

        match msg {
            WaylandStateMsg::Active(active) => {
                changed_attrs!(active);
            }
            WaylandStateMsg::SurroundingText {
                text,
                cursor,
                anchor,
            } => {
                changed_attrs!(text, cursor, anchor);
            }
            WaylandStateMsg::ContentType {
                content_hint_bits,
                content_purpose,
            } => {
                changed_attrs!(content_hint_bits, content_purpose);
            }
        }

        if changed {
            self.all_changed(ctxt).await?;
        }

        Ok(())
    }
}

#[interface(
    name = "dod.shell.Daemon.Osk.State",
    proxy(
        gen_blocking = false,
        default_path = "/dod/shell/Daemon",
        default_service = "dod.shell.Daemon"
    )
)]
impl State {
    #[zbus(property)]
    fn all(&self) -> State {
        self.clone()
    }

    #[zbus(property)]
    fn active(&self) -> bool {
        self.active
    }

    #[zbus(property)]
    fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    #[zbus(property)]
    fn text(&self) -> String {
        self.text.clone()
    }

    #[zbus(property)]
    fn cursor(&self) -> u32 {
        self.cursor
    }

    #[zbus(property)]
    fn anchor(&self) -> u32 {
        self.anchor
    }

    #[zbus(property)]
    fn content_hint_bits(&self) -> u32 {
        self.content_hint_bits
    }

    #[zbus(property)]
    fn content_purpose(&self) -> ContentPurposeWrapper {
        self.content_purpose
    }
}
