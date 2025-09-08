//! This module contains items relating to the config
//!
//! "Config" in this context means anything located in [``common::CONFIG_PATH``].
//!
//! The main type is [``Config``].
use tokio::fs;
use zbus::{interface, zvariant};

use common::CONFIG_PATH;

/// Contains all config found within the config directory located at [``common::CONFIG_PATH``]
///
/// ## Why is [``Self::toml``] not serialized?
///
/// While it would be much more comfortable to just serialize the toml and then pass around
/// [``common::Config``]s this doesn't work since zbus has issues handling [``Option``]s due to
/// them not being a concept within the dbus protocol.
///
/// See <https://dbus2.github.io/zbus/faq.html#how-do-i-use-optiont-with-zbus>
///
/// ## Dbus
///
/// This struct implements [``zbus::object_server::Interface``], which means it acts as a dbus
/// interface. For available zbus methods and properties see [``ConfigProxy``]
///
/// ## Confusion with [``common::Config``]
///
/// Although they have the same name these two serve very different functions.
///
/// [``common::Config``], as the docs state, related to the concrete format of `config.toml`
///
/// whereas
///
/// [``Config``] refers to all types of config found within the config directory
///
/// Make sure you know which one you need.
#[derive(Debug, Clone, zvariant::Value, zvariant::OwnedValue, zvariant::Type)]
pub struct Config {
    /// The config from `config.toml`
    pub toml: String,
    /// The css from `style.scss`
    pub css: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            toml: toml::to_string(&common::Config::default())
                .expect("Should never fail to serialize our toml fomat to toml."),
            css: String::default(),
        }
    }
}

#[interface(
    name = "dod.shell.Daemon.Config",
    proxy(
        gen_blocking = false,
        default_path = "/dod/shell/Daemon",
        default_service = "dod.shell.Daemon"
    )
)]
impl Config {
    /// Dbus property for the toml config
    #[zbus(property)]
    fn config(&self) -> String {
        self.toml.clone()
    }

    /// Dbus property for the css
    #[zbus(property)]
    fn css(&self) -> String {
        self.css.clone()
    }

    /// Dbus property for both the toml config and css
    #[zbus(property)]
    fn all_config(&self) -> Config {
        self.clone()
    }
}

/// Returned by [`Config::update`] to signal which configs have changed
///
/// Field [`Self::0`] signals if [`Config::toml`] has changed
/// Field [`Self::1`] signals if [`field@Config::css`] has changed
#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConfigValuesChanged(pub bool, pub bool);

impl ConfigValuesChanged {
    #[must_use]
    pub fn changes(&self) -> bool {
        self.0 || self.1
    }
}

impl Config {
    pub async fn update(&mut self) -> ConfigValuesChanged {
        let mut changes = ConfigValuesChanged::default();
        let toml_path = CONFIG_PATH.join("config.toml");
        let scss_path = CONFIG_PATH.join("style.scss");

        // Attempt to read the toml config
        match fs::read_to_string(&toml_path).await {
            // If we can read it and it's changed make sure it is valid
            Ok(s) if self.toml != s => match toml::from_str::<common::Config>(&s) {
                Ok(_) => {
                    self.toml = s;

                    changes.0 = true;
                }
                Err(e) => {
                    log::error!("Failed to parse config: {e}");
                }
            },
            // If the config hasn't changed don't do anything
            Ok(_) => (),
            Err(e) => {
                log::error!("Failed to read config at {}: {e}", toml_path.display());
            }
        }

        match grass::from_path(scss_path, &grass::Options::default()) {
            Ok(s) if self.css != s => {
                self.css = s;

                changes.1 = true;
            }
            // If the css hasn't changed don't do anything
            Ok(_) => (),
            Err(e) => {
                log::error!("Failed to parse scss: {e}");
            }
        }

        changes
    }

    // async fn get_toml(&mut self) -> String {}
    // async fn get_css(&mut self) -> String {}
}
