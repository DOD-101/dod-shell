//! Common functionality shared among all components of the shell
use std::{io::Write, path::PathBuf, sync::LazyLock};

use env_logger::fmt::style::{AnsiColor, Style};

pub mod config;
pub mod css;
pub mod err;
pub mod types;

pub use config::{Config, layouts::Layouts};

/// The path to the config dir
///
/// Changes depending on weather the build is for release or debug.
pub static CONFIG_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    if cfg!(debug_assertions) {
        return PathBuf::from("test");
    }

    dirs::config_dir()
        .expect("Failed to get config dir.")
        .join("dod-shell")
});

/// Calls [`function@logger`] with `CARGO_PKG_NAME`
#[macro_export]
macro_rules! logger {
    () => {
        $crate::logger(env!("CARGO_PKG_NAME"));
    };
}

/// Init the logger for a crate
///
/// Used to ensure consistent formatting across the project
///
/// Do not use this function directly use [`macro@logger`].
pub fn logger(target: &'static str) {
    env_logger::Builder::new()
        .filter_level(if cfg!(debug_assertions) {
            log::LevelFilter::Trace
        } else {
            log::LevelFilter::Info
        })
        .parse_env("DOD_SHELL_LOG")
        .default_format()
        .format(move |formatter, record| {
            let time = formatter.timestamp();
            let level = record.level();

            let style = Style::new().fg_color(match level {
                log::Level::Error => Some(AnsiColor::Red.into()),
                log::Level::Warn => Some(AnsiColor::Yellow.into()),
                log::Level::Info => Some(AnsiColor::Cyan.into()),
                log::Level::Debug | log::Level::Trace => None,
            });

            writeln!(
                formatter,
                "{time} [{target}] {style}{level}{style:#} {}",
                record.args()
            )
        })
        .write_style(env_logger::WriteStyle::Always)
        .init();
}
