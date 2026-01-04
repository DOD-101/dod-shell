//! Custom types used throughout the shell
use std::time::{Duration, Instant};
use std::{cell::UnsafeCell, fmt::Debug, fmt::Display, ops::Deref, sync::Once};

use log::Level;

/// Type representing a percentage
///
/// Internally values are stored in the format `1% = 0.01`
#[derive(
    Debug,
    Default,
    PartialEq,
    PartialOrd,
    Clone,
    Copy,
    zvariant::Value,
    zvariant::OwnedValue,
    zvariant::Type,
)]
pub struct Percentage {
    /// The value
    value: f64,
}

impl Percentage {
    /// Create a new [``Percentage``]
    ///
    /// Assumes the [``f64``] given is in the format:
    ///
    /// `0.1 = 10%`
    ///
    /// `1.0 = 100%`
    #[must_use]
    pub const fn new(value: f64) -> Self {
        Self { value }
    }

    /// Return the internal [``f64``] value
    #[must_use]
    pub const fn get_value(&self) -> f64 {
        self.value
    }
}

impl Display for Percentage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}%", (self.value * 100.0).round())
    }
}

impl Deref for Percentage {
    type Target = f64;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl From<f64> for Percentage {
    fn from(value: f64) -> Self {
        Self::new(value)
    }
}

impl From<u8> for Percentage {
    fn from(value: u8) -> Self {
        Self::new(f64::from(value) / 100.0)
    }
}

// TODO: Look into Maybeuninit for this

/// A type to be initialized at a later time.
///
/// Using this type necessitates having a 2 Step creation process.
///
/// ## Step 1
///
/// Create the actual variable
///
/// ```
/// # use common::types::DeferedInit;
///
/// let a: DeferedInit<String> = DeferedInit::default();
///
/// // ! Using `a.get_value()` or dereferencing `a` here will panic !
///
/// assert!(a.is_set() == false);
///
/// ```
///
/// ## Step 2
///
/// Actually populate wit with data (initialize it)
///
/// ```
/// # use common::types::DeferedInit;
///
/// # let a: DeferedInit<String> = DeferedInit::default();
///
/// a.init("Hello World".to_string());
///
/// assert_eq!(a.get_value(), &"Hello World".to_string());
///
/// ```
///
/// ## When to use this
///
/// Usage of this type should be avoided in *most* cases.
/// It should only be used when you *must* create the actual data in a later step and other options
/// such as a Typestate pattern wouldn't work.
///
/// For an example usage see `crates/daemon/src/osk/wayland.rs`.
pub struct DeferedInit<T> {
    /// The data to be initialized
    data: UnsafeCell<Option<T>>,
    /// A lock to ensure initialization only happens once
    once: Once,
}

unsafe impl<T: Sync> Sync for DeferedInit<T> {}

impl<T> Default for DeferedInit<T> {
    fn default() -> Self {
        Self {
            data: UnsafeCell::new(None),
            once: Once::new(),
        }
    }
}

impl<T: Debug> Debug for DeferedInit<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            f.debug_struct("DeferedInit")
                .field("data", self.data.get().as_ref().expect(Self::POINTER_MSG))
                .finish_non_exhaustive()
        }
    }
}

impl<T> Deref for DeferedInit<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get_value()
    }
}

impl<T> DeferedInit<T> {
    /// Error message for [`Result::expect`] when dealing with pointers
    const POINTER_MSG: &str = "Value pointer should never be null. This is a bug.";

    /// Inner helper function for [`Self::init`]
    ///
    /// # Panics
    ///
    /// Panics if called with the internal value already set.
    fn init_inner(&self, val: T) {
        unsafe {
            let value = self.data.get().as_mut().expect(Self::POINTER_MSG);

            assert!(value.is_none());

            *value = Some(val);
        }
    }

    /// Sets the value of this [`DeferedInit<T>`].
    ///
    /// # Errors
    ///
    /// This function will return an error containing the passed in value if it
    /// has already been called.
    pub fn init(&self, val: T) -> Result<(), T> {
        if self.once.is_completed() {
            return Err(val);
        }

        self.once.call_once(|| self.init_inner(val));

        Ok(())
    }

    /// Returns a reference to the get value of this [`DeferedInit<T>`].
    ///
    /// # Panics
    ///
    /// Panics if the internal value hasn't been initialized (aka. [`Self::init`] hasn't been
    /// called).
    pub const fn get_value(&self) -> &T {
        unsafe {
            let value = self.data.get().as_ref().expect(Self::POINTER_MSG);

            value.as_ref().expect("Value was never initialized")
        }
    }

    /// If the value has been set.
    ///
    /// If this function returns true it is safe to call [`Self::get_value`].
    #[allow(
        clippy::missing_panics_doc,
        reason = "As stated by POINTER_MSG, any panic here would be a bug."
    )]
    pub const fn is_set(&self) -> bool {
        unsafe {
            let value = self.data.get().as_ref().expect(Self::POINTER_MSG);

            value.is_some()
        }
    }
}

/// A Timer implemented using RAII
///
/// The timer is started once it is created and finished when dropped, where it will log the time
/// taken using the log crate.
pub struct Timer<'a> {
    /// When the timer was created / started
    start: Instant,
    /// Name of the timer
    ///
    /// Used for logging when finished
    name: &'a str,
    /// At what level to log when the timer is finished / dropped
    level: Level,
    /// An optional target for how long the timer should take
    ///
    /// If this target is not met the log level will be set to [`Level::Warn`]
    target: Option<Duration>,
}

impl<'a> Timer<'a> {
    /// Creates a new timer
    ///
    /// By default the log level is [`log::Level::Trace`]
    #[must_use]
    pub fn new(name: &'a str, target: Option<Duration>) -> Self {
        Self::new_with_level(name, Level::Trace, target)
    }

    /// Creates a new timer with a custom log level
    #[must_use]
    pub fn new_with_level(name: &'a str, level: Level, target: Option<Duration>) -> Self {
        Self {
            start: Instant::now(),
            name,
            level,
            target,
        }
    }
}

impl Drop for Timer<'_> {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();

        let time = if elapsed.as_millis() > 0 {
            format!("{}ms", elapsed.as_millis())
        } else {
            format!("{}μs", elapsed.as_micros())
        };

        let target_missed = self.target.is_some_and(|duration| duration < elapsed);

        let level = if target_missed {
            Level::Warn
        } else {
            self.level
        };

        log::log!(
            level,
            "{} (took {}{})",
            self.name,
            time,
            if target_missed {
                format!(
                    " (Target: {}μs)",
                    self.target
                        .expect("Should never fail since met_target checked that this is some.")
                        .as_micros()
                )
            } else {
                String::new()
            }
        );
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[allow(
        clippy::float_cmp,
        reason = "The float comparison is tested to work here without problem."
    )]
    fn new_percentages() {
        assert_eq!(Percentage::new(0.1).get_value(), 0.1);

        assert_eq!(
            std::convert::Into::<Percentage>::into(1_u8).get_value(),
            0.01
        );

        assert_eq!(*Percentage::new(0.1), 0.1);
    }

    #[test]
    fn defered_init() {
        let defered_init: DeferedInit<&'static str> = DeferedInit::default();

        assert!(!defered_init.is_set());

        defered_init
            .init("Hello")
            .expect("First call to init. Shouldn't fail.");

        assert!(defered_init.is_set());

        assert_eq!(defered_init.get_value(), &"Hello");
    }
}
