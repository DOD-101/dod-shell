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
    /// If [`Self::active`] is locked (aka. can't be changed)
    active_locked: bool,
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
            active_locked: bool::default(),
            text: String::default(),
            cursor: u32::default(),
            anchor: u32::default(),
            content_hint_bits: u32::default(),
            content_purpose: ContentPurposeWrapper::Normal,
        }
    }
}

impl State {
    /// Update [`Self`] from a [`WaylandStateMsg`]
    ///
    /// # Errors
    ///
    /// Errors if there is an issue sending the zbus signals.
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
                if self.active != active && self.set_active_inner(active) {
                    self.active_changed(ctxt).await?;
                    changed = true;
                }
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

    /// Helper function to make sure setting [`Self::active`] always respects [`Self::active_locked`]
    fn set_active_inner(&mut self, active: bool) -> bool {
        if self.active_locked {
            false
        } else {
            self.active = active;

            true
        }
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
    /// Returns a snapshot of the entire current state.
    ///
    /// This is exposed as a D-Bus property for convenience, allowing clients
    /// to retrieve all state fields atomically rather than querying each
    /// property individually.
    #[zbus(property)]
    fn all(&self) -> State {
        self.clone()
    }

    /// Indicates whether the on-screen keyboard (OSK) is currently active.
    ///
    /// When `true`, the OSK should be considered visible and ready to accept
    /// input. When `false`, the OSK is inactive.
    #[zbus(property)]
    fn active(&self) -> bool {
        self.active
    }

    /// Requests a change to the active state of the OSK.
    ///
    /// If [`Self::active_locked`] is set to `true`, this request is ignored and
    /// the active state remains unchanged.
    #[zbus(property)]
    fn set_active(&mut self, active: bool) {
        self.set_active_inner(active);
    }

    /// Indicates whether changes to the active state are locked.
    ///
    /// When `true`, attempts to modify [`Self::active`] via D-Bus will have
    /// no effect.
    #[zbus(property)]
    fn active_locked(&self) -> bool {
        self.active_locked
    }

    /// Sets whether the active state of the OSK is locked.
    ///
    /// When locking is enabled, the current active state is preserved until
    /// the lock is released.
    #[zbus(property)]
    fn set_active_locked(&mut self, active_locked: bool) {
        self.active_locked = active_locked;
    }

    /// Returns the current surrounding text of the focused input field.
    ///
    /// This typically represents the full text content known to the text
    /// input protocol, not just the portion currently visible.
    #[zbus(property)]
    fn text(&self) -> String {
        self.text.clone()
    }

    /// Returns the current cursor position within the surrounding text.
    ///
    /// The value is expressed as a byte or character offset, as defined by
    /// the underlying Wayland text-input protocol.
    #[zbus(property)]
    fn cursor(&self) -> u32 {
        self.cursor
    }

    /// Returns the anchor position of the current text selection.
    ///
    /// If no text is selected, this value is equal to [`Self::cursor`].
    #[zbus(property)]
    fn anchor(&self) -> u32 {
        self.anchor
    }

    /// Returns the bitmask of active content hints.
    ///
    /// These hints describe properties of the expected input, such as
    /// auto-completion or spell-check preferences, as defined by the
    /// Wayland text-input protocol.
    #[zbus(property)]
    fn content_hint_bits(&self) -> u32 {
        self.content_hint_bits
    }

    /// Returns the current content purpose.
    ///
    /// The content purpose provides semantic information about the expected
    /// input (for example, normal text, password, or email), allowing the OSK
    /// to adapt its layout or behavior accordingly.
    #[zbus(property)]
    fn content_purpose(&self) -> ContentPurposeWrapper {
        self.content_purpose
    }
}
