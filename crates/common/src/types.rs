//! Custom types used throughout the shell
use std::{fmt::Display, ops::Deref};

/// Type representing a percentage
///
/// Internally values are stored in the format `1% = 0.01`
#[derive(Debug, Default, PartialEq, PartialOrd, Clone, Copy)]
pub struct Percentage {
    value: f32,
}

impl Percentage {
    /// Create a new [``Percentage``]
    ///
    /// Assumes the [``f32``] given is in the format:
    ///
    /// `0.1 = 10%`
    ///
    /// `1.0 = 100%`
    #[must_use]
    pub fn new(value: f32) -> Self {
        Self { value }
    }

    /// Return the internal [``f32``] value
    #[must_use]
    pub fn get_value(&self) -> f32 {
        self.value
    }
}

impl Display for Percentage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}%", (self.value * 100.0).round())
    }
}

impl Deref for Percentage {
    type Target = f32;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl From<f32> for Percentage {
    fn from(value: f32) -> Self {
        Self::new(value)
    }
}

impl From<u8> for Percentage {
    fn from(value: u8) -> Self {
        Self::new(f32::from(value) / 100.0)
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
