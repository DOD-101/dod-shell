//! Custom types used throughout the shell
use std::{fmt::Display, ops::Deref};

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
    pub fn new(value: f64) -> Self {
        Self { value }
    }

    /// Return the internal [``f64``] value
    #[must_use]
    pub fn get_value(&self) -> f64 {
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[allow(clippy::float_cmp)]
    fn new_percentages() {
        assert_eq!(Percentage::new(0.1).get_value(), 0.1);

        assert_eq!(
            std::convert::Into::<Percentage>::into(1_u8).get_value(),
            0.01
        );

        assert_eq!(*Percentage::new(0.1), 0.1);
    }
}
