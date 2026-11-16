//! This module contains items relating to getting information about the current active playback
//!
//! This works, by using the mpris2 API under the hood. Sometimes the data returned from this API
//! can be out-of-date or missing entirely. It is a best effort approach.
//!
//! See: <https://specifications.freedesktop.org/mpris/latest/>
use futures_util::{StreamExt, stream};

use mpris2_zbus::{media_player, player::Player};
use strum::{Display, EnumIs};
use zbus::{connection::Connection, interface, zvariant};

/// State relating to the active media playback
///
/// Under the hood this data relies on the mpris2 API.
///
/// If a value isn't available from the underlying API, the default value is used.
///
/// ## Dbus
///
/// This struct implements [``zbus::object_server::Interface``], which means it acts as a dbus
/// interface. For available zbus methods and properties see [``PlaybackProxy``]
///
/// This only holds the *published* data. Gathering is done by [``PlaybackCollector``] so that this
/// interface never holds a lock while slow work is ongoing.
#[derive(Default)]
pub struct Playback {
    /// Total length of the playback
    length: f64,
    /// Current position in the playback
    ///
    /// While logically this value should always be between 0 and [`Self::length`], this isn't
    /// assured.
    position: f64,
    /// Artists of the current playback
    artists: Vec<String>,
    /// The title of the current playback
    title: String,
    /// The status of the playback
    status: PlaybackStatus,
}

impl Playback {
    /// Create a new [`Self`] from a [Player]
    async fn from_player(player: Player) -> Self {
        let Ok(metadata) = player.metadata().await else {
            log::debug!("Failed to get player metadata");
            return Self::default();
        };

        let length = metadata
            .length()
            .map(|l| l.as_secs_f64())
            .unwrap_or_default();

        let artists = metadata.artists().unwrap_or_default();

        let title = metadata.title().unwrap_or_default();

        let position = player
            .position()
            .await
            .inspect_err(|e| log::debug!("Failed to get player position: {e}"))
            .ok()
            .flatten()
            .unwrap_or_default()
            .as_secs_f64();

        let status = player
            .playback_status()
            .await
            .inspect_err(|e| log::debug!("Failed to get player playback status: {e}"))
            .map(PlaybackStatus::from)
            .unwrap_or_default();

        Self {
            length,
            position,
            artists,
            title,
            status,
        }
    }
}

#[interface(
    name = "dod.shell.Daemon.Playback",
    proxy(
        gen_blocking = false,
        default_path = "/dod/shell/Daemon",
        default_service = "dod.shell.Daemon"
    )
)]
impl Playback {
    /// Total length of the playback
    #[zbus(property)]
    const fn length(&self) -> f64 {
        self.length
    }

    /// Artists of the current playback
    #[zbus(property)]
    fn artists(&self) -> Vec<String> {
        self.artists.clone()
    }

    /// Current position in the playback
    #[zbus(property)]
    const fn position(&self) -> f64 {
        self.position
    }

    /// Current title of the playback
    #[zbus(property)]
    fn title(&self) -> String {
        self.title.clone()
    }

    /// Current status of the playback
    #[zbus(property)]
    const fn status(&self) -> PlaybackStatus {
        self.status
    }

    /// Current position of the playback as a percentage from 0.0 to 1.0
    #[zbus(property)]
    fn progress(&self) -> f64 {
        if self.length == 0.0 {
            return 0.0;
        }
        self.position / self.length
    }
}

/// Gathers the current playback state without holding any dbus interface locks
///
/// Reuses the passed session [`Connection`] across calls to [``Self::collect``] rather than
/// re-connecting every tick.
pub struct PlaybackCollector {
    /// Connection to the session bus, reused across [``Self::collect``] calls.
    session_connection: Connection,
}

impl PlaybackCollector {
    /// Creates a new [`PlaybackCollector`].
    #[must_use]
    pub const fn new(session_connection: Connection) -> Self {
        Self { session_connection }
    }

    /// Gather the current playback state.
    ///
    /// ## MPRIS
    ///
    /// It is a best-effort approach: when no data is available the default (empty) state is
    /// returned.
    pub async fn collect(&mut self) -> Playback {
        let Ok(media_players) = media_player::MediaPlayer::new_all(&self.session_connection).await
        else {
            log::error!("Failed to find media players.");
            return Playback::default();
        };

        let players: Vec<Player> = stream::iter(media_players)
            .filter_map(async |v| (v.player().await).ok())
            .collect()
            .await;

        match find_active_player(players).await {
            Ok(Some(p)) => Playback::from_player(p).await,
            Err(err) => {
                log::error!("Failed to find the active player: {err}");
                Playback::default()
            }
            Ok(None) => {
                log::debug!("No active player found.");
                Playback::default()
            }
        }
    }
}

/// Helper to diff an existing [`Playback`] with a new one and emit the appropriate property
/// changed signals.
pub async fn publish_playback(
    iface: &zbus::object_server::InterfaceRef<Playback>,
    old: &Playback,
    new: &Playback,
) {
    #![allow(
        clippy::useless_let_if_seq,
        reason = "The second if statement relies on this."
    )]
    let ctxt = iface.signal_emitter();
    let mut progress_changed = false;

    if (old.length - new.length).abs() > 0.1 {
        let _ = new.length_changed(ctxt).await;
        progress_changed = true;
    }

    if (old.position - new.position).abs() > 0.1 {
        let _ = new.position_changed(ctxt).await;
        progress_changed = true;
    }

    if progress_changed {
        let _ = new.progress_changed(ctxt).await;
    }

    if old.artists != new.artists {
        let _ = new.artists_changed(ctxt).await;
    }

    if old.title != new.title {
        let _ = new.title_changed(ctxt).await;
    }

    if old.status != new.status {
        let _ = new.status_changed(ctxt).await;
    }
}

/// Helper function to find the current active player providing playback
///
/// ## License
///
/// This function was taken from <https://github.com/Mange/mpris-rs/blob/master/src/find.rs>.
/// [mrpis-rs](https://github.com/Mange/mpris-rs/) is licensed under the Apache License 2.0 License.
async fn find_active_player(players: Vec<Player>) -> mpris2_zbus::error::Result<Option<Player>> {
    if players.is_empty() {
        return Ok(None);
    }

    let mut first_paused: Option<Player> = None;
    let mut first_with_track: Option<Player> = None;
    let mut first_found: Option<Player> = None;

    for player in players {
        let player_status: PlaybackStatus = player.playback_status().await?.into();

        if player_status.is_playing() {
            return Ok(Some(player));
        }

        if first_paused.is_none() && player_status.is_paused() {
            first_paused.replace(player);
        } else if first_with_track.is_none() && !player.metadata().await?.is_empty() {
            first_with_track.replace(player);
        } else if first_found.is_none() {
            first_found.replace(player);
        }
    }

    Ok(first_paused.or(first_with_track).or(first_found))
}

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Default,
    Display,
    EnumIs,
    zvariant::Value,
    zvariant::OwnedValue,
    zvariant::Type,
)]
/// Playback status
///
/// See: [`mpris2_zbus::player::PlaybackStatus`]
pub enum PlaybackStatus {
    /// A track is currently playing.
    Playing,
    /// A track is currently paused.
    Paused,
    /// There is no track currently playing.
    #[default]
    Stopped,
}

impl From<mpris2_zbus::player::PlaybackStatus> for PlaybackStatus {
    fn from(value: mpris2_zbus::player::PlaybackStatus) -> Self {
        match value {
            mpris2_zbus::player::PlaybackStatus::Playing => Self::Playing,
            mpris2_zbus::player::PlaybackStatus::Paused => Self::Paused,
            mpris2_zbus::player::PlaybackStatus::Stopped => Self::Stopped,
        }
    }
}
