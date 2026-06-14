//! Time-Playing Component
//!
//! This [`relm4::Component`] shows the current time and, if there is any playback the title,
//! artists and progress of that playback.
//!
//! The formatting of the time (and the playback information) can be adjusted via
//! [`common::config::bar::BarConfig::date_time_format`] and [`common::config::bar::BarConfig::date_time_playing_format`].
//!
//! The data relating to the playback is received from the daemon.
//!
//! See: [`daemon::playback::PlaybackProxy`]
use std::{sync::Arc, time::Duration};

use common::{config::bar::BarConfig, css::Class};
use daemon::playback::PlaybackProxy;
use futures_util::{FutureExt, StreamExt};
use relm4::{gtk::prelude::*, prelude::*, tokio};
use time::{
    OffsetDateTime, UtcOffset,
    format_description::{self, OwnedFormatItem},
};

/// See module level documentation
#[derive(Debug)]
pub struct TimePlaying {
    /// Config from the daemon
    ///
    /// See: [`TimePlayingInput::ConfigUpdated`]
    config: Arc<BarConfig>,
    /// Cached format description
    ///
    /// The bool indicates the value of [`Self::show_playing`] at the time the value was set.
    ///
    /// See: [`Self::current_format_description`]
    current_format_description: Option<(OwnedFormatItem, bool)>,
    /// Dbus connection
    ///
    /// Used for communication with the mpris2 API
    connection: zbus::Connection,
    /// If there is currently any playback
    ///
    /// If true the playback information will be show and [`common::config::bar::BarConfig::date_time_playing_format`] will be used.
    show_playing: bool,
    /// Artists of the current playback
    artists: Vec<String>,
    /// Title of the current playback
    title: String,
}

/// Input messages sent to the [`TimePlaying`] component from other parts of the application
#[derive(Debug)]
pub enum TimePlayingInput {
    /// Sent by the main App component when it receives a config change
    ConfigUpdated(Arc<BarConfig>),
}

/// Output messages from commands spawned by the [`TimePlaying`] component
///
/// There are two commands that run alongside the component
///
/// ## Time Updates
///
/// One command handles the updating of the current time. It sends an update every second with the
/// current time.
///
/// ## Playback Updates
///
/// The other command gets updates from the daemon with changes about the information about the
/// current playback.
#[derive(Debug)]
pub enum TPCommandOutput {
    /// Current time (as Unix time stamp)
    TimeUpdate(i64),
    /// Progress of the playback
    ///
    /// Should be a value from 0.0 to 1.0
    Progress(f64),
    /// Title of the playback
    Title(String),
    /// Artists of the playback
    Artists(Vec<String>),
    /// If there is currently any playback
    Playing(bool),
}

/// Auto-generated widget for [`TimePlaying`]
#[relm4::component(pub, async)]
impl AsyncComponent for TimePlaying {
    type Input = TimePlayingInput;
    type CommandOutput = TPCommandOutput;
    type Output = ();
    type Init = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            /// Main label for displaying the current time & date and playback information
            #[name(date_time)]
            gtk::Label {
                add_css_class: Class::TimePlayingLabel.as_ref(),
            },

            append: progress_bar = &gtk::ProgressBar {
                add_css_class: Class::TimePlayingProgressbar.as_ref(),
                #[watch]
                set_visible: model.show_playing,
                set_fraction: 0.3,
            }
        }
    }

    async fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let model = Self {
            config: BarConfig::default().into(),
            current_format_description: None,
            connection: zbus::Connection::session().await.unwrap(),
            show_playing: false,
            artists: Vec::default(),
            title: String::default(),
        };

        let playback_proxy = PlaybackProxy::new(&model.connection).await.unwrap();

        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    let mut interval = tokio::time::interval(Duration::from_secs(1));
                    loop {
                        interval.tick().await;
                        let _ = out.send(TPCommandOutput::TimeUpdate(
                            time::UtcDateTime::now().unix_timestamp(),
                        ));
                    }
                })
                .drop_on_shutdown()
                .boxed()
        });

        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    let mut progress_tx = playback_proxy
                        .receive_progress_changed()
                        .await
                        .fuse();
                    let mut title_tx = playback_proxy
                        .receive_title_changed()
                        .await
                        .fuse();
                    let mut artists_tx = playback_proxy
                        .receive_artists_changed()
                        .await
                        .fuse();
                    let mut status_tx = playback_proxy
                        .receive_status_changed()
                        .await
                        .fuse();

                    loop {
                        futures_util::select! {
                            progress = progress_tx.select_next_some() => {
                                out.emit(TPCommandOutput::Progress(progress.get().await.unwrap_or_default()));
                            }
                            title = title_tx.select_next_some() => {
                                out.emit(TPCommandOutput::Title(title.get().await.unwrap_or_default()));
                            }
                            artists = artists_tx.select_next_some() => {
                                out.emit(TPCommandOutput::Artists(artists.get().await.unwrap_or_default()));
                            }
                            status = status_tx.select_next_some() => {
                                out.emit(TPCommandOutput::Playing(!status.get().await.unwrap_or_default().is_stopped()));
                            }
                        }
                    }
                })
                .drop_on_shutdown()
                .boxed()
        });

        let widgets = view_output!();

        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        message: Self::Input,
        _sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            TimePlayingInput::ConfigUpdated(config) => {
                self.config = config;
                self.current_format_description = None;
            }
        }
    }

    async fn update_cmd_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::CommandOutput,
        sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            TPCommandOutput::TimeUpdate(time) => {
                widgets
                    .date_time
                    .set_label(&self.update_date_time_label(time));
            }
            TPCommandOutput::Progress(progress) => {
                widgets.progress_bar.set_fraction(progress);
            }
            TPCommandOutput::Title(title) => {
                self.title = title;
            }
            TPCommandOutput::Artists(artists) => {
                self.artists = artists;
            }
            TPCommandOutput::Playing(playing) => {
                self.show_playing = playing;
            }
        }

        self.update_view(widgets, sender);
    }
}

impl TimePlaying {
    /// Helper function to set the [`AppWidgets::date_time`] label
    fn update_date_time_label(&mut self, time: i64) -> String {
        OffsetDateTime::from_unix_timestamp(time)
            .expect("Unix timestamp from daemon should always be valid")
            .to_offset(
                UtcOffset::current_local_offset()
                    .inspect_err(|e| log::error!("Failed to get local offset: {e}"))
                    .unwrap_or(UtcOffset::UTC),
            )
            .format(&self.current_format_description())
            .unwrap()
            .replace(
                "{artists}",
                &self
                    .artists
                    .iter()
                    .fold(String::new(), |acc, a| acc + " " + a),
            )
            .replace("{title}", &self.title)
    }

    /// Gets the current format description to use
    ///
    /// If the cached value can be used that is returned if not the new one is computed, cached and
    /// then returned.
    fn current_format_description(&mut self) -> OwnedFormatItem {
        if let Some((ref fmt_desc, playing)) = self.current_format_description
            && playing == self.show_playing
        {
            return fmt_desc.clone();
        }

        let fmt_desc = if self.show_playing {
            format_description::parse_owned::<2>(&self.config.date_time_playing_format)
                .unwrap_or_else(|_| default_playing_format_description())
        } else {
            format_description::parse_owned::<2>(&self.config.date_time_format)
                .unwrap_or_else(|_| default_format_description())
        };

        self.current_format_description = Some((fmt_desc.clone(), self.show_playing));

        fmt_desc
    }
}

/// Get [`common::config::bar::date_time_default`] as an [`OwnedFormatItem`]
fn default_format_description() -> OwnedFormatItem {
    format_description::parse_owned::<2>(&common::config::bar::date_time_default())
        .expect("Parsing default format description should never fail.")
}

/// Get [`common::config::bar::date_time_playing_default`] as an [`OwnedFormatItem`]
fn default_playing_format_description() -> OwnedFormatItem {
    format_description::parse_owned::<2>(&common::config::bar::date_time_playing_default())
        .expect("Parsing default format description should never fail.")
}
