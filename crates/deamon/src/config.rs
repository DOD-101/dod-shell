use tokio::fs;
use zbus::{interface, zvariant};

use common::CONFIG_PATH;

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
    name = "dod.shell.Deamon.Config",
    proxy(
        gen_blocking = false,
        default_path = "/dod/shell/Deamon",
        default_service = "dod.shell.Deamon"
    )
)]
impl Config {
    #[zbus(property)]
    fn config(&self) -> String {
        self.toml.clone()
    }

    #[zbus(property)]
    fn css(&self) -> String {
        self.css.clone()
    }

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
