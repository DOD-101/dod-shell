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
/// [``common::Config``], as the docs state, relates to the concrete format of `config.toml`
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
    /// The osk layouts from `layouts.json`
    pub layouts: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            toml: toml::to_string(&common::Config::default())
                .expect("Should never fail to serialize our format to toml."),
            css: String::default(),
            layouts: serde_json::to_string(&common::Layouts::default())
                .expect("Should never fail to serialize our format to json"),
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

    /// Dbus property for the osk layouts
    #[zbus(property)]
    fn layouts(&self) -> String {
        self.layouts.clone()
    }

    /// Dbus property for both the toml config and css
    #[zbus(property)]
    #[allow(
        clippy::use_self,
        reason = "Can't use Self here because of zbus macro."
    )]
    fn all_config(&self) -> Config {
        self.clone()
    }
}

/// Returned by [`Config::update`] to signal which config values have changed
#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConfigValuesChanged {
    /// Corresponds to [`Config::toml`]
    toml: bool,
    /// Corresponds to [`field@Config::css`]
    css: bool,
    /// Corresponds to [`field@Config::layouts`]
    layouts: bool,
}

impl ConfigValuesChanged {
    /// If any config value has changed
    #[must_use]
    pub const fn any_changes(&self) -> bool {
        self.toml || self.css || self.layouts
    }

    /// If the toml value changed
    #[must_use]
    pub const fn toml_changed(&self) -> bool {
        self.toml
    }

    /// If the css value changed
    #[must_use]
    pub const fn css_changed(&self) -> bool {
        self.css
    }

    /// If the layouts value changed
    #[must_use]
    pub const fn layouts_changed(&self) -> bool {
        self.layouts
    }
}

impl Config {
    /// Update the config values from disk
    pub async fn update(&mut self) -> ConfigValuesChanged {
        let mut changes = ConfigValuesChanged::default();
        let toml_path = CONFIG_PATH.join("config.toml");
        let scss_path = CONFIG_PATH.join("style.scss");
        let layouts_path = CONFIG_PATH.join("layouts.json");

        // -- toml --
        match fs::read_to_string(&toml_path).await {
            // If we can read it and it's changed make sure it is valid
            Ok(s) if self.toml != s => match toml::from_str::<common::Config>(&s) {
                Ok(_) => {
                    self.toml = s;

                    changes.toml = true;
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

        // -- css --
        match grass::from_path(scss_path, &grass::Options::default()) {
            Ok(s) if self.css != s => {
                self.css = s;

                changes.css = true;
            }
            // If the css hasn't changed don't do anything
            Ok(_) => (),
            Err(e) => {
                log::error!("Failed to parse scss: {e}");
            }
        }

        // -- layouts --
        match fs::read_to_string(&layouts_path).await {
            Ok(s) if self.layouts != s => {
                match serde_json::from_str::<common::config::layouts::Layouts>(&s) {
                    Ok(_) => {
                        self.layouts = s;

                        changes.layouts = true;
                    }
                    Err(e) => {
                        log::error!("Failed to parse layouts: {e}");
                    }
                }
            }
            // If the layouts haven't changed don't do anything
            Ok(_) => (),
            Err(e) => {
                log::error!("Failed to read config at {}: {e}", layouts_path.display());
            }
        }

        changes
    }
}
